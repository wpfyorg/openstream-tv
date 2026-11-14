//! EPG (Electronic Program Guide) generation and serving. Mirrors
//! `pkg/epg` and the EPG handlers in `internal/handlers/epg.go`. The
//! background regeneration loop (`main::epg_task_loop`) approximates the Go
//! version's "run again ~24h out at a random off-peak time" without
//! reproducing its exact hour arithmetic; see the README's parity notes.

use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path as FsPath;
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const EPG_URL_FMT: &str =
    "https://jiotv.data.cdn.jio.com/apis/v1.3/getepg/get?offset={offset}&channel_id={id}";
pub const EPG_POSTER_URL: &str = "https://jiotv.catchup.cdn.jio.com/dare_images/shows/";
const CACHE_METADATA_VERSION: u32 = 1;
const CACHE_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
pub const STORE_KEY_ENABLED: &str = "epg_enabled";
pub const COMPANION_APP_ENV: &str = "OPENSTREAM_COMPANION_APP";

pub fn enabled_with_override(
    config_enabled: bool,
    stored: Option<&str>,
    force_enabled: bool,
) -> bool {
    if force_enabled {
        return true;
    }
    match stored {
        Some("true") => true,
        Some("false") => false,
        _ => config_enabled,
    }
}

#[derive(Default)]
pub struct EpgState {
    current_fingerprint: RwLock<Option<String>>,
    context_lock: Mutex<()>,
    generation_lock: tokio::sync::Mutex<()>,
}

impl EpgState {
    pub fn invalidate(&self) {
        *self.current_fingerprint.write().unwrap() = None;
    }

    pub fn current_fingerprint(&self) -> Option<String> {
        self.current_fingerprint.read().unwrap().clone()
    }

    fn set_current_fingerprint(&self, fingerprint: String) {
        *self.current_fingerprint.write().unwrap() = Some(fingerprint);
    }

    pub(crate) fn context_guard(&self) -> MutexGuard<'_, ()> {
        self.context_lock.lock().unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CacheMetadata {
    version: u32,
    fingerprint: String,
}

struct ContextSnapshot {
    fingerprint: String,
    product: crate::state::ActiveProduct,
    channels: Vec<crate::television::Channel>,
}

fn cache_path(state: &AppState) -> String {
    format!("{}epg.xml.gz", state.path_prefix)
}

fn metadata_path(path: &str) -> String {
    format!("{path}.context.json")
}

fn hash_field(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn snapshot_fingerprint(
    state: &AppState,
    product: crate::state::ActiveProduct,
    channels: &[crate::television::Channel],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"jiotv-xmltv-context-v1");
    hash_field(&mut hasher, product.as_str());

    match product {
        crate::state::ActiveProduct::Tv => {
            hash_field(&mut hasher, &state.tv.account_fingerprint());
            hash_field(&mut hasher, "tv-linear-entitlement:unknown");
        }
        crate::state::ActiveProduct::Extras => {
            hash_field(&mut hasher, &state.extras.account_fingerprint());
            hash_field(&mut hasher, &state.extras.entitlement_fingerprint());
        }
        crate::state::ActiveProduct::Fastway => {
            hash_field(&mut hasher, &state.fastway.account_fingerprint());
        }
        #[cfg(feature = "beta")]
        crate::state::ActiveProduct::Playbox => {
            hash_field(&mut hasher, &state.playbox.account_fingerprint());
            hash_field(&mut hasher, "playbox-epg:unavailable");
        }
        #[cfg(feature = "beta")]
        crate::state::ActiveProduct::Vzy => {
            hash_field(&mut hasher, &state.vzy.account_fingerprint());
            hash_field(&mut hasher, "vzy-free-tier");
        }
        #[cfg(feature = "beta")]
        crate::state::ActiveProduct::Airtel => {
            hash_field(&mut hasher, &state.airtel.account_fingerprint());
            hash_field(&mut hasher, "airtel-live-epg");
        }
        #[cfg(feature = "beta")]
        crate::state::ActiveProduct::TataPlay => {
            hash_field(&mut hasher, &state.tataplay.account_fingerprint());
            hash_field(&mut hasher, "tataplay-epg:channels-only");
        }
    }

    // XMLTV currently contains only upstream channels. Sorting makes this a
    // stable catalogue-generation identifier across process restarts.
    let mut catalogue: Vec<_> = channels
        .iter()
        .map(|ch| (ch.id.clone(), ch.name.clone(), ch.category))
        .collect();
    catalogue.sort();
    hasher.update((catalogue.len() as u64).to_le_bytes());
    for (id, name, category) in catalogue {
        hash_field(&mut hasher, &id);
        hash_field(&mut hasher, &name);
        hasher.update(category.to_le_bytes());
    }
    hex::encode(hasher.finalize())
}

async fn context_snapshot(state: &AppState) -> anyhow::Result<ContextSnapshot> {
    let product = state.active_product();
    let channels = state.effective_upstream_channels().await?;
    let fingerprint = snapshot_fingerprint(state, product, &channels);
    Ok(ContextSnapshot {
        fingerprint,
        product,
        channels,
    })
}

fn read_cache_metadata(path: &str) -> Option<CacheMetadata> {
    let raw = std::fs::read(metadata_path(path)).ok()?;
    serde_json::from_slice(&raw).ok()
}

fn cache_matches(path: &str, fingerprint: &str) -> bool {
    if !FsPath::new(path).is_file() {
        return false;
    }
    matches!(
        read_cache_metadata(path),
        Some(meta) if meta.version == CACHE_METADATA_VERSION && meta.fingerprint == fingerprint
    )
}

fn cache_is_fresh(path: &str) -> bool {
    std::fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|modified| modified.elapsed().ok())
        .map(|age| age <= CACHE_MAX_AGE)
        .unwrap_or(false)
}

fn invalidate_cache_files(path: &str) {
    for candidate in [path.to_string(), metadata_path(path)] {
        if let Err(e) = std::fs::remove_file(&candidate) {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("cannot invalidate stale EPG cache {candidate}: {e}");
            }
        }
    }
}

/// Clears the current fingerprint only if no context change happened since
/// `epoch` was captured. The check runs under the context guard, which every
/// context change also holds while it rotates the epoch, so a failed or
/// obsolete validation cannot erase the fingerprint a newer context installed.
pub(crate) fn invalidate_if_current(state: &AppState, epoch: u64) {
    let _guard = state.epg_state.context_guard();
    if state.context_epoch() == epoch {
        state.epg_state.invalidate();
    }
}

/// Establishes the current stable XMLTV context before the HTTP server can
/// expose a cached guide. A legacy/missing sidecar or a mismatched context
/// invalidates the artifact regardless of mtime.
pub async fn prepare_cache_for_state(state: &Arc<AppState>) -> anyhow::Result<bool> {
    let epoch = state.context_epoch();
    let snapshot = context_snapshot(state).await?;
    let _guard = state.epg_state.context_guard();
    if state.context_epoch() != epoch {
        anyhow::bail!("account context changed while validating EPG cache");
    }
    state
        .epg_state
        .set_current_fingerprint(snapshot.fingerprint.clone());
    let path = cache_path(state);
    let matches = cache_matches(&path, &snapshot.fingerprint);
    if !matches {
        invalidate_cache_files(&path);
    }
    Ok(matches)
}

#[derive(Deserialize, Default)]
struct EpgObject {
    #[serde(rename = "startEpoch", default)]
    start_epoch: i64,
    #[serde(rename = "endEpoch", default)]
    end_epoch: i64,
    #[serde(rename = "showCategory", default)]
    show_category: String,
    #[serde(default)]
    description: String,
    #[serde(rename = "showname", default)]
    title: String,
    #[serde(rename = "episodePoster", default)]
    poster: String,
}

#[derive(Deserialize, Default)]
struct EpgResponse {
    #[serde(default)]
    epg: Vec<EpgObject>,
}

/// One `<programme>` entry, public so `extras::to_xmltv` can hand in extra
/// entries from the extras-only channels (mirrors `epg.Programme` in the Go
/// tree, used the same way by `ExtrasEPGSource`).
pub struct XmlProgramme {
    pub channel: String,
    pub start: String,
    pub stop: String,
    pub title: String,
    pub desc: String,
    pub category: String,
    pub icon: String,
}

/// Guide entries fetched at once while the XMLTV document is written. Each
/// channel's entries are serialized and dropped as soon as its fetch
/// returns, so peak memory follows this window rather than the catalogue
/// size.
const EPG_FETCH_CONCURRENCY: usize = 20;

const XMLTV_HEADER: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\t<!DOCTYPE tv SYSTEM \"http://www.w3.org/2006/05/tv\">";

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Formats a Unix-millisecond epoch as XMLTV's `"20060102150405 -0700"`
/// (always `+0000`/UTC here, since that's what this rewrite computes with).
pub fn format_xmltv_time(epoch_ms: i64) -> String {
    let secs = (epoch_ms / 1000).max(0) as u64;
    let (y, mo, d, hh, mm, ss) = crate::television::civil_from_unix(secs);
    format!("{y:04}{mo:02}{d:02}{hh:02}{mm:02}{ss:02} +0000")
}

async fn fetch_epg_for_channel(client: &reqwest::Client, channel_id: i64) -> Vec<EpgObject> {
    let mut out = Vec::new();
    for offset in 0..2 {
        let url = EPG_URL_FMT
            .replace("{offset}", &offset.to_string())
            .replace("{id}", &channel_id.to_string());
        let Ok(resp) = client
            .get(&url)
            .header(header::USER_AGENT, "okhttp/4.12.0")
            .send()
            .await
        else {
            continue;
        };
        if !resp.status().is_success() {
            continue;
        }
        if let Ok(parsed) = resp.json::<EpgResponse>().await {
            out.extend(parsed.epg);
        }
    }
    out
}

fn gzip_writer() -> std::io::BufWriter<flate2::write::GzEncoder<Vec<u8>>> {
    std::io::BufWriter::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::default(),
    ))
}

fn finish_gzip(
    out: std::io::BufWriter<flate2::write::GzEncoder<Vec<u8>>>,
) -> anyhow::Result<Vec<u8>> {
    Ok(out.into_inner().map_err(|e| e.into_error())?.finish()?)
}

fn write_channel<W: Write>(out: &mut W, id: &str, display: &str) -> std::io::Result<()> {
    write!(
        out,
        "<channel id=\"{}\"><display-name>{}</display-name></channel>",
        xml_escape(id),
        xml_escape(display)
    )
}

fn write_programme<W: Write>(out: &mut W, p: &XmlProgramme) -> std::io::Result<()> {
    write!(
        out,
        "<programme channel=\"{}\" start=\"{}\" stop=\"{}\"><title lang=\"en\">{}</title><desc lang=\"en\">{}</desc><category lang=\"en\">{}</category><icon src=\"{}\"></icon></programme>",
        p.channel,
        p.start,
        p.stop,
        xml_escape(&p.title),
        xml_escape(&p.desc),
        xml_escape(&p.category),
        xml_escape(&p.icon)
    )
}

fn write_fetched_programme<W: Write>(
    out: &mut W,
    channel: &str,
    e: &EpgObject,
) -> std::io::Result<()> {
    write!(
        out,
        "<programme channel=\"{}\" start=\"{}\" stop=\"{}\"><title lang=\"en\">{}</title><desc lang=\"en\">{}</desc><category lang=\"en\">{}</category><icon src=\"{}{}\"></icon></programme>",
        channel,
        format_xmltv_time(e.start_epoch),
        format_xmltv_time(e.end_epoch),
        xml_escape(&e.title),
        xml_escape(&e.description),
        xml_escape(&e.show_category),
        EPG_POSTER_URL,
        xml_escape(&e.poster)
    )
}

async fn write_finished_channel<W: Write>(
    out: &mut W,
    task: tokio::task::JoinHandle<(i64, Vec<EpgObject>)>,
) -> anyhow::Result<usize> {
    let Ok((channel, entries)) = task.await else {
        return Ok(0);
    };
    let channel = channel.to_string();
    for entry in &entries {
        write_fetched_programme(out, &channel, entry)?;
    }
    Ok(entries.len())
}

fn finish_guide(
    out: std::io::BufWriter<flate2::write::GzEncoder<Vec<u8>>>,
    programmes: usize,
) -> anyhow::Result<Vec<u8>> {
    if programmes == 0 {
        anyhow::bail!("no EPG programmes were fetched");
    }
    let mut out = out;
    out.write_all(b"</tv>")?;
    finish_gzip(out)
}

/// Writes the guide into the gzip stream as channel fetches finish, so the
/// document, the programme list, and a second XML copy never coexist in
/// memory. Programme order matches the catalogue order of `tv_sources`.
async fn generate_xml_gz_from_sources(
    client: &reqwest::Client,
    tv_sources: Vec<(String, String)>,
    extra_sources: Vec<(String, String)>,
    extra_programmes: Vec<XmlProgramme>,
) -> anyhow::Result<Vec<u8>> {
    let mut out = gzip_writer();
    out.write_all(XMLTV_HEADER.as_bytes())?;
    out.write_all(br#"<tv version="" encoding="">"#)?;
    for (id, display) in tv_sources.iter().chain(extra_sources.iter()) {
        write_channel(&mut out, id, display)?;
    }

    let mut programmes = 0usize;
    let mut window: std::collections::VecDeque<tokio::task::JoinHandle<(i64, Vec<EpgObject>)>> =
        std::collections::VecDeque::with_capacity(EPG_FETCH_CONCURRENCY);
    for (channel_id, _) in &tv_sources {
        let Ok(id) = channel_id.parse::<i64>() else {
            continue;
        };
        let client = client.clone();
        window.push_back(tokio::spawn(async move {
            (id, fetch_epg_for_channel(&client, id).await)
        }));
        if window.len() >= EPG_FETCH_CONCURRENCY {
            let task = window.pop_front().expect("window is non-empty");
            programmes += write_finished_channel(&mut out, task).await?;
        }
    }
    while let Some(task) = window.pop_front() {
        programmes += write_finished_channel(&mut out, task).await?;
    }

    for p in &extra_programmes {
        write_programme(&mut out, p)?;
        programmes += 1;
    }

    finish_guide(out, programmes)
}

/// XMLTV with channel entries but no programmes, used for a product whose
/// real programme timing is not understood.
#[cfg_attr(not(feature = "beta"), allow(dead_code))]
fn generate_channels_only_xml_gz(channels: Vec<(String, String)>) -> anyhow::Result<Vec<u8>> {
    let mut out = gzip_writer();
    out.write_all(XMLTV_HEADER.as_bytes())?;
    out.write_all(br#"<tv version="" encoding="">"#)?;
    for (id, display) in &channels {
        write_channel(&mut out, id, display)?;
    }
    out.write_all(b"</tv>")?;
    finish_gzip(out)
}

#[cfg(test)]
fn gzip_xml(xml: &str) -> anyhow::Result<Vec<u8>> {
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(XMLTV_HEADER.as_bytes())?;
    gz.write_all(xml.as_bytes())?;
    Ok(gz.finish()?)
}

fn atomic_write(path: &str, bytes: &[u8]) -> anyhow::Result<()> {
    let target = FsPath::new(path);
    let parent = target.parent().unwrap_or_else(|| FsPath::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file_mut().sync_all()?;
    temp.persist(target).map_err(|e| e.error)?;
    Ok(())
}

fn publish_cache_if_current(
    state: &AppState,
    epoch: u64,
    fingerprint: &str,
    bytes: &[u8],
) -> anyhow::Result<bool> {
    let _guard = state.epg_state.context_guard();
    if state.context_epoch() != epoch
        || state.epg_state.current_fingerprint().as_deref() != Some(fingerprint)
    {
        return Ok(false);
    }

    let path = cache_path(state);
    let metadata = serde_json::to_vec(&CacheMetadata {
        version: CACHE_METADATA_VERSION,
        fingerprint: fingerprint.to_string(),
    })?;
    atomic_write(&path, bytes)?;
    atomic_write(&metadata_path(&path), &metadata)?;
    Ok(true)
}

/// Rebuilds XMLTV from the same active product catalogue used by the UI and
/// playlist. Local custom channels have no upstream EPG source and are not
/// added here.
pub async fn regenerate_for_state(state: &Arc<AppState>) -> anyhow::Result<()> {
    let _generation = state.epg_state.generation_lock.lock().await;
    let epoch = state.context_epoch();
    let snapshot = context_snapshot(state).await?;
    {
        let _guard = state.epg_state.context_guard();
        if state.context_epoch() != epoch {
            anyhow::bail!("account context changed before EPG generation started");
        }
        state
            .epg_state
            .set_current_fingerprint(snapshot.fingerprint.clone());
        let path = cache_path(state);
        if !cache_matches(&path, &snapshot.fingerprint) {
            invalidate_cache_files(&path);
        }
    }

    let bytes = match snapshot.product {
        crate::state::ActiveProduct::Tv => {
            let tv_sources = snapshot
                .channels
                .iter()
                .map(|ch| (ch.id.clone(), ch.name.clone()))
                .collect();
            generate_xml_gz_from_sources(&state.http, tv_sources, Vec::new(), Vec::new()).await?
        }
        crate::state::ActiveProduct::Extras => {
            let (extra_sources, extra_programmes) = state.extras.epg_source(&state.tv).await?;
            generate_xml_gz_from_sources(&state.http, Vec::new(), extra_sources, extra_programmes)
                .await?
        }
        crate::state::ActiveProduct::Fastway => {
            let (sources, programmes) = state.fastway.epg_source(&state.store).await?;
            generate_xml_gz_from_sources(&state.http, Vec::new(), sources, programmes).await?
        }
        // PlayboxTV's EPG response has no programme stop time, so only the
        // channel list is published; no programme is invented.
        #[cfg(feature = "beta")]
        crate::state::ActiveProduct::Playbox => {
            let channels = snapshot
                .channels
                .iter()
                .map(|ch| (ch.id.clone(), ch.name.clone()))
                .collect();
            generate_channels_only_xml_gz(channels)?
        }
        #[cfg(feature = "beta")]
        crate::state::ActiveProduct::Vzy => {
            let (sources, programmes) = state.vzy.epg_source(&state.store).await?;
            if programmes.is_empty() {
                generate_channels_only_xml_gz(sources)?
            } else {
                generate_xml_gz_from_sources(&state.http, Vec::new(), sources, programmes).await?
            }
        }
        #[cfg(feature = "beta")]
        crate::state::ActiveProduct::Airtel => {
            let (sources, programmes) = state.airtel.epg_source(&state.store).await?;
            if programmes.is_empty() {
                generate_channels_only_xml_gz(sources)?
            } else {
                generate_xml_gz_from_sources(&state.http, Vec::new(), sources, programmes).await?
            }
        }
        // Tata Play's guide endpoint needs epoch-millisecond start/end bounds
        // whose runtime encoding is not yet proven, so only the channel list is
        // published; no programme is invented.
        #[cfg(feature = "beta")]
        crate::state::ActiveProduct::TataPlay => {
            let channels = snapshot
                .channels
                .iter()
                .map(|ch| (ch.id.clone(), ch.name.clone()))
                .collect();
            generate_channels_only_xml_gz(channels)?
        }
    };

    // Entitlement/catalogue state can change while network EPG requests are
    // in flight even without an explicit account switch. Only a generation
    // whose complete effective context still matches may become current.
    let confirmed = context_snapshot(state).await?;
    if state.context_epoch() != epoch || confirmed.fingerprint != snapshot.fingerprint {
        anyhow::bail!("discarded EPG generated for an obsolete account context");
    }
    if !publish_cache_if_current(state, epoch, &snapshot.fingerprint, &bytes)? {
        anyhow::bail!("discarded EPG generated for an obsolete account epoch");
    }
    Ok(())
}

pub async fn ensure_current_cache(state: &Arc<AppState>) -> anyhow::Result<bool> {
    let matches = prepare_cache_for_state(state).await?;
    let path = cache_path(state);
    if matches && cache_is_fresh(&path) {
        return Ok(true);
    }
    regenerate_for_state(state).await?;
    Ok(true)
}

pub fn trigger_regeneration(state: &Arc<AppState>) {
    if !state.config.epg {
        return;
    }
    let state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = ensure_current_cache(&state).await {
            tracing::warn!("EPG regeneration after account change failed: {e}");
        }
    });
}

/// `GET /epg.xml.gz` only serves a cache whose sidecar matches the active
/// account/mode fingerprint. Context switches serialize with this read, so
/// there is no window where the catalogue has switched but old XMLTV can be
/// returned.
pub async fn epg_handler(State(state): State<Arc<AppState>>) -> Response {
    let _guard = state.epg_state.context_guard();
    let Some(fingerprint) = state.epg_state.current_fingerprint() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "EPG is being prepared for the active account",
        )
            .into_response();
    };
    let path = cache_path(&state);
    if !cache_matches(&path, &fingerprint) {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "EPG is not ready for the active account",
        )
            .into_response();
    }
    match std::fs::read(&path) {
        Ok(bytes) => Response::builder()
            .header(header::CONTENT_TYPE, "application/gzip")
            .header(header::CONTENT_ENCODING, "gzip")
            .body(Body::from(bytes))
            .unwrap(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "EPG is not ready for the active account",
        )
            .into_response(),
    }
}

/// `GET /epg/:channelID/:offset` — proxies a single channel/day's schedule,
/// mirroring `WebEPGHandler`'s day-offset correction (skipped here: this
/// rewrite passes the upstream response straight through — see README).
pub(crate) const IST_OFFSET_MS: i64 = 5 * 60 * 60 * 1000 + 30 * 60 * 1000;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

fn epg_day_offset_from_epoch(epoch_ms: i64) -> i64 {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let epg_day = (epoch_ms + IST_OFFSET_MS) / DAY_MS;
    let today = (now_ms + IST_OFFSET_MS) / DAY_MS;
    (today - epg_day).max(0)
}

/// How many days the getepg API's "today" lags the real date, derived from
/// the `serverDate` it stamps on its response (both compared in IST day
/// boundaries, matching the API's own). 0 means nothing to correct. Mirrors
/// `webEPGDayOffset`.
fn web_epg_day_offset(body: &[u8]) -> i64 {
    #[derive(Deserialize)]
    struct Entry {
        #[serde(rename = "serverDate", default)]
        server_date: String,
    }
    #[derive(Deserialize)]
    struct Resp {
        #[serde(default)]
        epg: Vec<Entry>,
    }
    let Ok(resp) = serde_json::from_slice::<Resp>(body) else {
        return 0;
    };
    let Some(first) = resp.epg.first() else {
        return 0;
    };
    if first.server_date.is_empty() {
        return 0;
    }
    let Some(server_day_start) = crate::dash::parse_rfc3339(&first.server_date) else {
        return 0;
    };
    let server_ms = server_day_start
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    epg_day_offset_from_epoch(server_ms)
}

async fn fetch_web_epg(
    state: &AppState,
    channel_id: &str,
    offset: i64,
) -> Option<(StatusCode, Vec<u8>)> {
    let url = EPG_URL_FMT
        .replace("{offset}", &offset.to_string())
        .replace("{id}", channel_id);
    let resp = state
        .http
        .get(&url)
        .header(header::USER_AGENT, "okhttp/4.12.0")
        .send()
        .await
        .ok()?;
    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let bytes = resp.bytes().await.ok()?.to_vec();
    Some((status, bytes))
}

/// `GET /epg/:channelID/:offset` — proxies a single channel/day's schedule.
/// If the response's `serverDate` shows the API's "today" lagging real
/// time, re-fetches with the corrected offset so the schedule covers the
/// present moment, falling back to the original response if that retry
/// fails. Mirrors `WebEPGHandler`/`webEPGWithCorrectedDay`.
pub async fn web_epg_handler(
    Path((channel_id, offset)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    // A constant answer, checked before the account-scoped catalogue so the
    // gap is reported even when PlayboxTV is not the active product.
    #[cfg(feature = "beta")]
    if crate::playbox::is_playbox_channel(&channel_id) {
        return (
            StatusCode::NOT_IMPLEMENTED,
            "playbox EPG is unavailable: the client's weekly_epg response has no \
             programme end time and its startTime is display-only",
        )
            .into_response();
    }
    if !state.channel_allowed(&channel_id).await {
        return (
            StatusCode::NOT_FOUND,
            "channel is not available for the active account",
        )
            .into_response();
    }
    if let Some(content_id) = crate::fastway::content_id(&channel_id) {
        return fastway_web_epg(&state, content_id, &offset).await;
    }
    #[cfg(feature = "beta")]
    if crate::vzy::is_vzy_channel(&channel_id) {
        return vzy_web_epg(&state, &channel_id, &offset).await;
    }
    #[cfg(feature = "beta")]
    if crate::airtel::is_airtel_channel(&channel_id) {
        return airtel_web_epg(&state, &channel_id, &offset).await;
    }
    if let Some(content_id) = crate::extras::content_id::content_id(&channel_id) {
        return extras_web_epg(&state, content_id, &offset).await;
    }
    let channel_id_trimmed = channel_id.strip_prefix("sl").unwrap_or(&channel_id);
    if channel_id_trimmed.parse::<i64>().is_err() {
        return (StatusCode::BAD_REQUEST, "Invalid channel ID").into_response();
    }
    let Ok(offset_num) = offset.parse::<i64>() else {
        return (StatusCode::BAD_REQUEST, "Invalid offset").into_response();
    };

    let Some((status, body)) = fetch_web_epg(&state, channel_id_trimmed, offset_num).await else {
        return (StatusCode::BAD_GATEWAY, "upstream EPG request failed").into_response();
    };

    let (status, body) = if status == StatusCode::OK {
        let day_offset = web_epg_day_offset(&body);
        if day_offset > 0 {
            match fetch_web_epg(&state, channel_id_trimmed, offset_num + day_offset).await {
                Some((StatusCode::OK, adjusted)) => (StatusCode::OK, adjusted),
                _ => (status, body),
            }
        } else {
            (status, body)
        }
    } else {
        (status, body)
    };

    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap()
}

/// The web guide for a Fastway channel, in the same shape as JioTV's. Fastway
/// EPG epochs are seconds; the UI expects milliseconds.
async fn fastway_web_epg(state: &AppState, content_id: &str, offset: &str) -> Response {
    let offset = match offset.parse::<i64>() {
        Ok(o) if o >= 0 => o,
        _ => return (StatusCode::BAD_REQUEST, "Invalid offset").into_response(),
    };
    let programmes = match state
        .fastway
        .web_epg(content_id, offset, &state.store)
        .await
    {
        Ok(p) => p,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let entries: Vec<serde_json::Value> = programmes
        .iter()
        .map(|p| {
            serde_json::json!({
                "showname": p.title,
                "description": p.description,
                "startEpoch": p.start_epoch.saturating_mul(1000),
                "endEpoch": p.end_epoch.saturating_mul(1000),
                "episodePoster": "",
            })
        })
        .collect();
    axum::Json(serde_json::json!({ "epg": entries })).into_response()
}

/// Airtel Xstream guide normalized to the same shape as JioTV/Fastway. The
/// middleware returns epoch millis already.
#[cfg(feature = "beta")]
async fn airtel_web_epg(state: &AppState, channel_id: &str, offset: &str) -> Response {
    if crate::airtel::parse_channel_id(channel_id).is_none() {
        return (StatusCode::BAD_REQUEST, "Invalid channel ID").into_response();
    }
    let offset = match offset.parse::<i64>() {
        Ok(o) if o >= 0 => o,
        _ => return (StatusCode::BAD_REQUEST, "Invalid offset").into_response(),
    };
    let programmes = match state.airtel.web_epg(&state.store, channel_id, offset).await {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    };
    let entries: Vec<serde_json::Value> = programmes
        .iter()
        .map(|p| {
            serde_json::json!({
                "showname": p.title,
                "description": p.subtitle,
                "startEpoch": p.start_millis(),
                "endEpoch": p.end_millis(),
                "episodePoster": "",
            })
        })
        .collect();
    axum::Json(serde_json::json!({ "epg": entries })).into_response()
}

/// VZY browser guide normalized to the same shape as JioTV/Fastway.
#[cfg(feature = "beta")]
async fn vzy_web_epg(state: &AppState, channel_id: &str, offset: &str) -> Response {
    let Some(content_id) = crate::vzy::parse_channel_id(channel_id) else {
        return (StatusCode::BAD_REQUEST, "Invalid channel ID").into_response();
    };
    let offset = match offset.parse::<i64>() {
        Ok(o) if o >= 0 => o,
        _ => return (StatusCode::BAD_REQUEST, "Invalid offset").into_response(),
    };
    if !state.vzy.ensure_catalogue(&state.store).await {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "vzy catalogue is unavailable",
        )
            .into_response();
    }
    let Some(client) = state.vzy.client() else {
        return (StatusCode::NOT_FOUND, "vzy is not enabled").into_response();
    };
    let date =
        crate::vzy::midnight_utc(SystemTime::now()).saturating_add(offset.saturating_mul(86_400));
    let guide = match client
        .epg(
            &state.store,
            &[content_id],
            date,
            crate::vzy::local_offset_minutes(),
        )
        .await
    {
        Ok(guide) => guide,
        Err(e) => return (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    };
    let entries: Vec<serde_json::Value> = guide
        .get(&content_id.to_string())
        .map(|programmes| {
            programmes
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "showname": p.name,
                        "description": p.description,
                        "startEpoch": if p.start_date.abs() < 10_000_000_000 { p.start_date.saturating_mul(1000) } else { p.start_date },
                        "endEpoch": if p.end_date.abs() < 10_000_000_000 { p.end_date.saturating_mul(1000) } else { p.end_date },
                        "episodePoster": "",
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    axum::Json(serde_json::json!({ "epg": entries })).into_response()
}

/// The web guide for a extras channel, in the same shape as JioTV's.
/// Mirrors `extrasWebEPG`.
async fn extras_web_epg(state: &AppState, content_id: &str, offset: &str) -> Response {
    let Some(client) = state.extras.client_for_vod() else {
        return (StatusCode::NOT_FOUND, "extras is not connected").into_response();
    };
    let offset = match offset.parse::<i64>() {
        Ok(o) if o >= 0 => o,
        _ => return (StatusCode::BAD_REQUEST, "Invalid offset").into_response(),
    };
    let ids = [content_id.to_string()];
    let mut guide = match client.epg(&ids, &[offset]).await {
        Ok(g) => g,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let day_offset = guide
        .get(content_id)
        .and_then(|progs| progs.first())
        .map(|program| epg_day_offset_from_epoch(program.start_epoch))
        .unwrap_or(0);
    if day_offset > 0 {
        if let Ok(adjusted) = client.epg(&ids, &[offset + day_offset]).await {
            if adjusted
                .get(content_id)
                .map(|progs| !progs.is_empty())
                .unwrap_or(false)
            {
                guide = adjusted;
            }
        }
    }
    let entries: Vec<serde_json::Value> = guide
        .get(content_id)
        .map(|progs| {
            progs
                .iter()
                .map(|p| {
                    let poster = if p.thumbnail.ends_with('/') {
                        ""
                    } else {
                        p.thumbnail.as_str()
                    };
                    serde_json::json!({
                        "showname": p.title,
                        "description": p.description,
                        "startEpoch": p.start_epoch,
                        "endEpoch": p.end_epoch,
                        "episodePoster": poster,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    axum::Json(serde_json::json!({ "epg": entries })).into_response()
}

/// `GET /jtvposter/:date/:file`
pub async fn poster_handler(
    Path((date, file)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let url = format!("{EPG_POSTER_URL}{date}/{file}");
    match state.http.get(&url).send().await {
        Ok(resp) => {
            let status =
                StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let ct = resp.headers().get(header::CONTENT_TYPE).cloned();
            let bytes = resp.bytes().await.unwrap_or_default();
            let mut builder = Response::builder().status(status);
            if let Some(ct) = ct {
                builder = builder.header(header::CONTENT_TYPE, ct);
            }
            builder.body(Body::from(bytes)).unwrap()
        }
        Err(_) => (StatusCode::BAD_GATEWAY, "upstream poster request failed").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "beta")]
    #[tokio::test]
    async fn playbox_web_epg_reports_the_missing_stop_time() {
        let dir = tempfile::tempdir().unwrap();
        let store =
            std::sync::Arc::new(crate::store::Store::open(dir.path().to_str().unwrap()).unwrap());
        std::mem::forget(dir);
        let state = std::sync::Arc::new(AppState {
            config: crate::config::Config::default(),
            path_prefix: String::new(),
            access: std::sync::Arc::new(crate::access::Access::new(store.clone())),
            store,
            tv: std::sync::Arc::new(crate::television::Television::new(reqwest::Client::new())),
            secure: std::sync::Arc::new(crate::secureurl::SecureUrl::new(false)),
            http: reqwest::Client::new(),
            drm_channels: Default::default(),
            custom_channels: std::sync::Arc::new(crate::custom_channels::CustomChannels::new()),
            render_caches: Default::default(),
            dash_state: Default::default(),
            epg_state: Default::default(),
            extras: std::sync::Arc::new(crate::extras_state::ExtrasState::new(false, None)),
            fastway: std::sync::Arc::new(crate::fastway_state::FastwayState::new(false)),
            playbox: std::sync::Arc::new(crate::playbox_state::PlayboxState::new(false)),
            tataplay: Arc::new(crate::tataplay_state::TataplayState::new(false)),
            vzy: std::sync::Arc::new(crate::vzy_state::VzyState::new(false)),
            airtel: std::sync::Arc::new(crate::airtel_state::AirtelState::new(false)),
            vod_state: Default::default(),
            public_ip: std::sync::Arc::new(crate::unlock::PublicIp::new(reqwest::Client::new())),
            unlock_limiter: std::sync::Arc::new(crate::unlock::AttemptLimiter::default()),
            listen: Default::default(),
            tunnel_origin: Default::default(),
        });
        let resp = web_epg_handler(Path(("pb_ch1".into(), "0".into())), State(state)).await;
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[cfg(feature = "beta")]
    #[tokio::test]
    async fn vzy_web_epg_uses_shared_browser_route_shape() {
        use wiremock::matchers::{header as match_header, method, path, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/ottuser/action/anonymousLogin"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"ks": "anon-ks", "expiry": 9999999999u64}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/asset/action/list"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"totalCount": 1, "objects": [
                    {"id": 1, "name": "Free One", "metas": {"EntitlementType": {"value": "Free"}}}
                ]}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path_regex(
                r"^/api_v3/service/epg/action/get/date/\d+/slots/all$",
            ))
            .and(match_header("Authorization", "Bearer anon-ks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "epgChunk": {
                    "1": [{
                        "id": 9,
                        "name": "Show",
                        "description": "Desc",
                        "startDate": 1793923200,
                        "endDate": 1793926800,
                        "linearAssetId": 1
                    }]
                }
            })))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store =
            std::sync::Arc::new(crate::store::Store::open(dir.path().to_str().unwrap()).unwrap());
        let http = reqwest::Client::new();
        let mut client = crate::vzy::Client::new(http.clone(), crate::vzy::Device::new());
        client.set_bases_for_test(&server.uri(), &format!("{}/api_v3", server.uri()));
        let vzy = std::sync::Arc::new(crate::vzy_state::VzyState::new(true));
        vzy.set_client_for_test(std::sync::Arc::new(client));
        let state = std::sync::Arc::new(AppState {
            config: crate::config::Config::default(),
            path_prefix: String::new(),
            access: std::sync::Arc::new(crate::access::Access::new(store.clone())),
            store,
            tv: std::sync::Arc::new(crate::television::Television::new(http.clone())),
            secure: std::sync::Arc::new(crate::secureurl::SecureUrl::new(false)),
            http: http.clone(),
            drm_channels: Default::default(),
            custom_channels: std::sync::Arc::new(crate::custom_channels::CustomChannels::new()),
            render_caches: Default::default(),
            dash_state: Default::default(),
            epg_state: Default::default(),
            extras: std::sync::Arc::new(crate::extras_state::ExtrasState::new(false, None)),
            fastway: std::sync::Arc::new(crate::fastway_state::FastwayState::new(false)),
            playbox: std::sync::Arc::new(crate::playbox_state::PlayboxState::new(false)),
            tataplay: Arc::new(crate::tataplay_state::TataplayState::new(false)),
            vzy,
            airtel: std::sync::Arc::new(crate::airtel_state::AirtelState::new(false)),
            vod_state: Default::default(),
            public_ip: std::sync::Arc::new(crate::unlock::PublicIp::new(http)),
            unlock_limiter: std::sync::Arc::new(crate::unlock::AttemptLimiter::default()),
            listen: Default::default(),
            tunnel_origin: Default::default(),
        });

        let resp = web_epg_handler(Path(("vzy_1".into(), "0".into())), State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["epg"][0]["showname"], "Show");
        assert_eq!(json["epg"][0]["description"], "Desc");
        assert_eq!(json["epg"][0]["startEpoch"], 1793923200000i64);
        assert_eq!(json["epg"][0]["endEpoch"], 1793926800000i64);
    }
    use http_body_util::BodyExt;
    use std::collections::HashMap;

    #[test]
    fn companion_app_forces_epg_on_regardless_of_config_or_persisted_override() {
        assert!(enabled_with_override(false, Some("false"), true));
        assert!(enabled_with_override(false, Some("invalid"), true));
        assert!(enabled_with_override(false, None, true));
    }

    #[test]
    fn stored_override_takes_precedence_when_epg_is_not_forced() {
        assert!(!enabled_with_override(true, Some("false"), false));
        assert!(enabled_with_override(false, Some("true"), false));
        assert!(enabled_with_override(true, Some("invalid"), false));
        assert!(enabled_with_override(true, None, false));
        assert!(!enabled_with_override(false, Some("false"), false));
        assert!(!enabled_with_override(false, Some("invalid"), false));
        assert!(!enabled_with_override(false, None, false));
    }

    fn tv_channel(id: &str, name: &str) -> crate::television::Channel {
        crate::television::Channel {
            id: id.into(),
            name: name.into(),
            category: 5,
            ..Default::default()
        }
    }

    fn tv_credentials(account: &str) -> crate::television::Credentials {
        crate::television::Credentials {
            sso_token: format!("sso-{account}"),
            crm: format!("crm-{account}"),
            unique_id: format!("uid-{account}"),
            access_token: format!("access-{account}"),
            refresh_token: format!("refresh-{account}"),
        }
    }

    fn extras_channel(id: &str, name: &str) -> crate::extras::LiveChannel {
        crate::extras::LiveChannel {
            content_id: id.into(),
            name: name.into(),
            sub_provider: "TestProvider".into(),
            playback_type: "dash".into(),
            ..Default::default()
        }
    }

    fn extras_credentials(account: &str) -> crate::extras::Credentials {
        crate::extras::Credentials {
            number: "0000000000".into(),
            sso_token: format!("sso-{account}"),
            subscriber_id: format!("sub-{account}"),
            unique: format!("unique-{account}"),
            user_id: format!("user-{account}"),
            auth_token: format!("auth-{account}"),
            refresh_token: format!("refresh-{account}"),
        }
    }

    fn test_state(
        dir: &tempfile::TempDir,
        tv_account: &str,
        tv_channels: Vec<crate::television::Channel>,
        extras: Option<(&str, Vec<crate::extras::LiveChannel>)>,
    ) -> Arc<AppState> {
        let prefix = format!("{}/", dir.path().display());
        let store = Arc::new(crate::store::Store::open(&prefix).unwrap());
        let http = reqwest::Client::new();
        let tv = Arc::new(crate::television::Television::new(http.clone()));
        tv.set_credentials(tv_credentials(tv_account));
        tv.set_channels_for_test(tv_channels);

        let extras_state = Arc::new(crate::extras_state::ExtrasState::new(
            extras.is_some(),
            None,
        ));
        extras_state.init(&http, &store);
        if let Some((account, channels)) = extras {
            extras_state.prime_for_test(
                extras_credentials(account),
                channels,
                Some(HashMap::from([("TestProvider".to_string(), true)])),
            );
        }

        Arc::new(AppState {
            config: crate::config::Config::default(),
            path_prefix: prefix,
            access: Arc::new(crate::access::Access::new(store.clone())),
            store,
            tv,
            secure: Arc::new(crate::secureurl::SecureUrl::new(false)),
            http: http.clone(),
            drm_channels: Default::default(),
            custom_channels: Arc::new(crate::custom_channels::CustomChannels::new()),
            render_caches: Default::default(),
            dash_state: Default::default(),
            epg_state: Default::default(),
            extras: extras_state,
            fastway: Arc::new(crate::fastway_state::FastwayState::new(false)),
            playbox: Arc::new(crate::playbox_state::PlayboxState::new(false)),
            tataplay: Arc::new(crate::tataplay_state::TataplayState::new(false)),
            vzy: Arc::new(crate::vzy_state::VzyState::new(false)),
            airtel: Arc::new(crate::airtel_state::AirtelState::new(false)),
            vod_state: Default::default(),
            public_ip: Arc::new(crate::unlock::PublicIp::new(http)),
            unlock_limiter: Arc::new(crate::unlock::AttemptLimiter::default()),
            listen: Default::default(),
            tunnel_origin: Default::default(),
        })
    }

    async fn publish_test_guide(state: &Arc<AppState>, marker: &str) -> Vec<u8> {
        let epoch = state.context_epoch();
        let snapshot = context_snapshot(state).await.unwrap();
        {
            let _guard = state.epg_state.context_guard();
            state
                .epg_state
                .set_current_fingerprint(snapshot.fingerprint.clone());
        }
        let xml = format!(
            "<tv><channel id=\"{marker}\"><display-name>{marker}</display-name></channel></tv>"
        );
        let bytes = gzip_xml(&xml).unwrap();
        assert!(publish_cache_if_current(state, epoch, &snapshot.fingerprint, &bytes).unwrap());
        bytes
    }

    async fn response_bytes(response: Response) -> Vec<u8> {
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec()
    }

    async fn channel_ids(state: Arc<AppState>) -> Vec<String> {
        let response = crate::api::channels(State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["channels"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|row| row["id"].as_str().map(str::to_string))
            .collect()
    }

    #[test]
    fn escapes_xml_special_characters() {
        assert_eq!(
            xml_escape("Tom & Jerry <Show>"),
            "Tom &amp; Jerry &lt;Show&gt;"
        );
    }

    #[test]
    fn formats_xmltv_time() {
        // 2024-01-02T03:04:05Z in ms
        assert_eq!(format_xmltv_time(1704164645000), "20240102030405 +0000");
    }

    fn decode_guide(bytes: &[u8]) -> String {
        use std::io::Read;
        let mut xml = String::new();
        flate2::read::GzDecoder::new(bytes)
            .read_to_string(&mut xml)
            .unwrap();
        xml
    }

    #[test]
    fn streamed_guide_matches_the_previous_document_layout() {
        let mut out = gzip_writer();
        out.write_all(XMLTV_HEADER.as_bytes()).unwrap();
        out.write_all(br#"<tv version="" encoding="">"#).unwrap();
        write_channel(&mut out, "101", "Tom & Jerry").unwrap();
        write_programme(
            &mut out,
            &XmlProgramme {
                channel: "101".into(),
                start: "20240102030405 +0000".into(),
                stop: "20240102040405 +0000".into(),
                title: "Tom & Jerry".into(),
                desc: "A <cat>".into(),
                category: "Kids".into(),
                icon: format!("{EPG_POSTER_URL}x.jpg"),
            },
        )
        .unwrap();
        let bytes = finish_guide(out, 1).unwrap();

        assert_eq!(
            decode_guide(&bytes),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\t<!DOCTYPE tv SYSTEM \"http://www.w3.org/2006/05/tv\">",
                "<tv version=\"\" encoding=\"\">",
                "<channel id=\"101\"><display-name>Tom &amp; Jerry</display-name></channel>",
                "<programme channel=\"101\" start=\"20240102030405 +0000\" stop=\"20240102040405 +0000\">",
                "<title lang=\"en\">Tom &amp; Jerry</title><desc lang=\"en\">A &lt;cat&gt;</desc>",
                "<category lang=\"en\">Kids</category>",
                "<icon src=\"https://jiotv.catchup.cdn.jio.com/dare_images/shows/x.jpg\"></icon></programme>",
                "</tv>"
            )
        );
    }

    #[test]
    fn fetched_programme_formats_xmltv_time_and_escapes_the_poster() {
        let mut xml = Vec::new();
        write_fetched_programme(
            &mut xml,
            "101",
            &EpgObject {
                start_epoch: 1704164645000,
                end_epoch: 1704168245000,
                show_category: "Kids".into(),
                description: "D".into(),
                title: "T".into(),
                poster: "a&b.jpg".into(),
            },
        )
        .unwrap();
        let xml = String::from_utf8(xml).unwrap();
        assert!(xml.starts_with(
            "<programme channel=\"101\" start=\"20240102030405 +0000\" stop=\"20240102040405 +0000\">"
        ));
        assert!(xml.contains(
            "<icon src=\"https://jiotv.catchup.cdn.jio.com/dare_images/shows/a&amp;b.jpg\"></icon>"
        ));
    }

    #[test]
    fn guide_without_programmes_is_rejected() {
        let out = gzip_writer();
        assert!(finish_guide(out, 0).is_err());
    }

    #[test]
    fn channels_only_guide_has_channels_and_no_programmes() {
        let bytes = generate_channels_only_xml_gz(vec![
            ("pb_1".into(), "Playbox & One".into()),
            ("tp_2".into(), "Tata Two".into()),
        ])
        .unwrap();
        let xml = decode_guide(&bytes);
        assert!(xml.starts_with(XMLTV_HEADER));
        assert!(xml.contains(
            "<channel id=\"pb_1\"><display-name>Playbox &amp; One</display-name></channel>"
        ));
        assert!(
            xml.contains("<channel id=\"tp_2\"><display-name>Tata Two</display-name></channel>")
        );
        assert!(!xml.contains("<programme"));
        assert!(xml.ends_with("</tv>"));
    }

    #[test]
    fn day_offset_is_zero_for_current_server_date() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let now_str = {
            let (y, m, d, hh, mm, ss) = crate::television::civil_from_unix((now / 1000) as u64);
            format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.000Z")
        };
        let body = format!(r#"{{"epg":[{{"serverDate":"{now_str}"}}]}}"#);
        assert_eq!(web_epg_day_offset(body.as_bytes()), 0);
    }

    #[test]
    fn day_offset_is_positive_when_server_date_is_stale() {
        let stale = "2020-01-01T00:00:00.000Z";
        let body = format!(r#"{{"epg":[{{"serverDate":"{stale}"}}]}}"#);
        assert!(web_epg_day_offset(body.as_bytes()) > 1000);
    }

    #[test]
    fn extras_epg_day_offset_uses_programme_epoch() {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        assert_eq!(epg_day_offset_from_epoch(now_ms - 3 * DAY_MS), 3);
        assert_eq!(epg_day_offset_from_epoch(now_ms + DAY_MS), 0);
    }

    #[test]
    fn day_offset_is_zero_without_a_server_date() {
        assert_eq!(web_epg_day_offset(br#"{"epg":[]}"#), 0);
        assert_eq!(web_epg_day_offset(b"not json"), 0);
    }

    #[tokio::test]
    async fn fresh_tv_cache_is_rejected_on_restart_into_extras() {
        let dir = tempfile::tempdir().unwrap();
        let tv = test_state(&dir, "tv-a", vec![tv_channel("101", "TV A")], None);
        let old = publish_test_guide(&tv, "tv-a-guide").await;
        assert!(cache_is_fresh(&cache_path(&tv)));

        let extras = test_state(
            &dir,
            "tv-a",
            vec![tv_channel("101", "TV A")],
            Some(("extras-a", vec![extras_channel("201", "Extras A")])),
        );
        assert!(!prepare_cache_for_state(&extras).await.unwrap());
        let ids = channel_ids(extras.clone()).await;
        assert_eq!(ids, vec!["ex_201"]);

        let unavailable = epg_handler(State(extras.clone())).await;
        assert_eq!(unavailable.status(), StatusCode::SERVICE_UNAVAILABLE);
        let fresh = publish_test_guide(&extras, "extras-a-guide").await;
        let served = epg_handler(State(extras)).await;
        assert_eq!(served.status(), StatusCode::OK);
        assert_eq!(response_bytes(served).await, fresh);
        assert_ne!(fresh, old);
    }

    #[tokio::test]
    async fn fresh_extras_cache_is_rejected_on_restart_into_tv() {
        let dir = tempfile::tempdir().unwrap();
        let extras = test_state(
            &dir,
            "tv-a",
            vec![tv_channel("101", "TV A")],
            Some(("extras-a", vec![extras_channel("201", "Extras A")])),
        );
        let old = publish_test_guide(&extras, "extras-a-guide").await;

        let tv = test_state(&dir, "tv-a", vec![tv_channel("101", "TV A")], None);
        assert!(!prepare_cache_for_state(&tv).await.unwrap());
        assert_eq!(channel_ids(tv.clone()).await, vec!["101"]);
        assert_eq!(
            epg_handler(State(tv.clone())).await.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );

        let fresh = publish_test_guide(&tv, "tv-a-guide").await;
        assert_eq!(response_bytes(epg_handler(State(tv)).await).await, fresh);
        assert_ne!(fresh, old);
    }

    #[tokio::test]
    async fn same_context_restart_reuses_fresh_cache() {
        let dir = tempfile::tempdir().unwrap();
        let first = test_state(&dir, "tv-a", vec![tv_channel("101", "TV A")], None);
        let expected = publish_test_guide(&first, "same-context").await;

        let restarted = test_state(&dir, "tv-a", vec![tv_channel("101", "TV A")], None);
        assert!(prepare_cache_for_state(&restarted).await.unwrap());
        let response = epg_handler(State(restarted)).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_bytes(response).await, expected);
    }

    #[tokio::test]
    async fn fresh_cache_from_account_a_is_rejected_for_account_b() {
        let dir = tempfile::tempdir().unwrap();
        let account_a = test_state(&dir, "account-a", vec![tv_channel("101", "TV")], None);
        publish_test_guide(&account_a, "account-a-guide").await;

        let account_b = test_state(&dir, "account-b", vec![tv_channel("101", "TV")], None);
        assert!(!prepare_cache_for_state(&account_b).await.unwrap());
        assert_eq!(
            epg_handler(State(account_b)).await.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[tokio::test]
    async fn stale_startup_validation_cannot_clear_a_newer_fingerprint() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(&dir, "account-a", vec![tv_channel("101", "TV")], None);
        let started = state.context_epoch();

        // The context changes while the background validation is running, and
        // the new context installs its own fingerprint.
        state.invalidate_context();
        state.epg_state.set_current_fingerprint("newer".into());
        invalidate_if_current(&state, started);
        assert_eq!(
            state.epg_state.current_fingerprint().as_deref(),
            Some("newer"),
            "an obsolete validation erased the newer context's fingerprint"
        );

        // A validation that still owns the current context may invalidate it.
        invalidate_if_current(&state, state.context_epoch());
        assert!(state.epg_state.current_fingerprint().is_none());
    }

    #[tokio::test]
    async fn obsolete_generation_cannot_overwrite_current_epoch() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(&dir, "account-a", vec![tv_channel("101", "TV")], None);
        let old_epoch = state.context_epoch();
        let old_snapshot = context_snapshot(&state).await.unwrap();
        let old_bytes = gzip_xml("<tv><channel id=\"old\"></channel></tv>").unwrap();

        state.invalidate_context();
        state.tv.set_credentials(tv_credentials("account-b"));
        state
            .tv
            .set_channels_for_test(vec![tv_channel("101", "TV")]);
        assert!(!prepare_cache_for_state(&state).await.unwrap());
        let current = publish_test_guide(&state, "account-b-guide").await;

        assert!(!publish_cache_if_current(
            &state,
            old_epoch,
            &old_snapshot.fingerprint,
            &old_bytes
        )
        .unwrap());
        let response = epg_handler(State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_bytes(response).await, current);
    }
}
