//! DASH/Widevine proxying: `/live/mpd/:id`, `/live/key/:id`, `/render.mpd`,
//! `/render.dash/*`, `/drm` and `/dashtime`. Mirrors `LiveManifestMpdHandler`
//! / `LiveManifestKeyHandler` / `MpdHandler` / `DashHandler` /
//! `DRMKeyHandler` / `DASHTimeHandler` in `internal/handlers/drm.go`, with
//! the same reduced fidelity noted in `stream.rs` (no singleflight, and here
//! also: no extras-specific CDN user-agent switching, and Set-Cookie from the
//! CDN is not forwarded to the client — the embedded `/hdnea/<enc>` segment
//! in the rewritten `BaseURL` carries the token instead, which is enough for
//! same-process segment proxying but not for a client using cookies to talk
//! to the CDN directly).

use crate::state::AppState;
use crate::television::{self, LiveUrlOutput};
use axum::body::{Body, Bytes};
use axum::extract::{Query, State};
use axum::http::{header, Method, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
#[cfg(feature = "beta")]
use base64::Engine as _;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DRM_MPD_CACHE_TTL: Duration = Duration::from_secs(30);
const DASH_ROUTE_BINDING_PREFIX: &str = "openstream-dash-v1:";

#[derive(serde::Serialize, serde::Deserialize)]
struct DashRouteBinding {
    channel_id: String,
    value: String,
}

pub(crate) fn encrypt_dash_route_value(
    secure: &crate::secureurl::SecureUrl,
    channel_id: &str,
    value: &str,
) -> String {
    let payload = DashRouteBinding {
        channel_id: channel_id.to_string(),
        value: value.to_string(),
    };
    let serialized = serde_json::to_string(&payload).expect("dash route binding serializes");
    secure.encrypt_deterministic(&format!("{DASH_ROUTE_BINDING_PREFIX}{serialized}"))
}

fn decrypt_dash_route_value(
    secure: &crate::secureurl::SecureUrl,
    channel_id: &str,
    encrypted: &str,
) -> Option<String> {
    let decoded = secure.decrypt(encrypted).ok()?;
    let serialized = decoded.strip_prefix(DASH_ROUTE_BINDING_PREFIX)?;
    let payload: DashRouteBinding = serde_json::from_str(serialized).ok()?;
    (payload.channel_id == channel_id).then_some(payload.value)
}

#[derive(Clone, Default)]
pub struct DrmMpdOutput {
    // Computed for parity with the Go struct (`buildDrmMpdOutput`) and kept
    // in case a future in-app DRM player page needs them directly; nothing
    // reads them today since /live/mpd/:id redirects straight to play_url.
    #[allow(dead_code)]
    pub is_drm: bool,
    pub play_url: String,
    pub license_url: String,
    #[allow(dead_code)]
    pub tv_url_host: String,
    #[allow(dead_code)]
    pub tv_url_path: String,
}

#[derive(Default)]
pub struct DashState {
    drm_mpd_cache: RwLock<std::collections::HashMap<String, (DrmMpdOutput, Instant, u64)>>,
    live_cache: RwLock<std::collections::HashMap<String, (LiveUrlOutput, Instant, u64)>>,
    /// Merged Widevine PSSH per Tata Play channel, so one EME session can ask
    /// for every rendition's key in a single challenge.
    #[cfg(feature = "beta")]
    tataplay_pssh: RwLock<std::collections::HashMap<String, (String, Instant)>>,
    /// Coalesces concurrent live-cache misses for the same channel.
    live_locks: crate::keyed_locks::KeyedLocks,
    cdn_clock: Mutex<Option<(SystemTime, Instant)>>,
}

#[cfg(feature = "beta")]
const TATAPLAY_PSSH_TTL: Duration = Duration::from_secs(60);
#[cfg(feature = "beta")]
const WIDEVINE_SYSTEM_ID: [u8; 16] = [
    0xed, 0xef, 0x8b, 0xa9, 0x79, 0xd6, 0x4a, 0xce, 0xa3, 0xc8, 0x27, 0xdc, 0xd5, 0x1d, 0x21, 0xed,
];

impl DashState {
    /// Served only in the context epoch it was built in, so a hit racing an
    /// account switch is a miss rather than the previous account's URLs.
    fn get_cached(&self, key: &str, epoch: u64) -> Option<DrmMpdOutput> {
        let map = self.drm_mpd_cache.read().unwrap();
        let (out, at, entry_epoch) = map.get(key)?;
        (*entry_epoch == epoch && at.elapsed() <= DRM_MPD_CACHE_TTL).then(|| out.clone())
    }

    /// Stores `out`, built under `epoch`, only if `current_epoch()` still
    /// equals it, compared under the write lock that `clear` also takes after
    /// the epoch rotates. Returns whether it was stored.
    fn set_cached(
        &self,
        key: &str,
        out: DrmMpdOutput,
        epoch: u64,
        current_epoch: impl FnOnce() -> u64,
    ) -> bool {
        let mut map = self.drm_mpd_cache.write().unwrap();
        if current_epoch() != epoch {
            return false;
        }
        map.insert(key.to_string(), (out, Instant::now(), epoch));
        true
    }

    /// A cached entry is only served in the context epoch it was fetched in,
    /// so a hit racing an account switch (epoch rotated, cache not yet
    /// cleared) is a miss rather than the previous account's data.
    fn get_live(&self, channel_id: &str, epoch: u64) -> Option<LiveUrlOutput> {
        let map = self.live_cache.read().unwrap();
        let (out, at, entry_epoch) = map.get(channel_id)?;
        (*entry_epoch == epoch && at.elapsed() <= DRM_MPD_CACHE_TTL).then(|| out.clone())
    }

    /// Stores `out`, fetched under `epoch`, only if `current_epoch()` still
    /// equals it. The comparison runs under the cache's write lock, which
    /// `clear` also takes after the epoch rotates, so a fetch started before an
    /// account switch cannot repopulate the cleared cache. Returns whether the
    /// entry was stored.
    fn set_live(
        &self,
        channel_id: &str,
        out: LiveUrlOutput,
        epoch: u64,
        current_epoch: impl FnOnce() -> u64,
    ) -> bool {
        let mut map = self.live_cache.write().unwrap();
        if current_epoch() != epoch {
            return false;
        }
        map.insert(channel_id.to_string(), (out, Instant::now(), epoch));
        true
    }

    fn record_publish_time(&self, t: SystemTime) {
        *self.cdn_clock.lock().unwrap() = Some((t, Instant::now()));
    }

    /// The CDN's extrapolated current time, if an MPD fetch has observed one.
    pub fn cdn_now(&self) -> Option<SystemTime> {
        let guard = self.cdn_clock.lock().unwrap();
        let (t, at) = (*guard)?;
        Some(t + at.elapsed())
    }

    #[cfg(feature = "beta")]
    pub fn invalidate_channel(&self, channel_id: &str) {
        self.live_cache.write().unwrap().remove(channel_id);
        let prefix = format!("{channel_id}_");
        self.drm_mpd_cache
            .write()
            .unwrap()
            .retain(|key, _| !key.starts_with(&prefix));
        #[cfg(feature = "beta")]
        self.tataplay_pssh.write().unwrap().remove(channel_id);
    }

    #[cfg(feature = "beta")]
    fn get_tataplay_pssh(&self, channel_id: &str) -> Option<String> {
        let map = self.tataplay_pssh.read().unwrap();
        let (value, at) = map.get(channel_id)?;
        (at.elapsed() <= TATAPLAY_PSSH_TTL).then(|| value.clone())
    }

    #[cfg(feature = "beta")]
    fn set_tataplay_pssh(
        &self,
        channel_id: &str,
        value: String,
        epoch: u64,
        current_epoch: impl FnOnce() -> u64,
    ) -> bool {
        let mut map = self.tataplay_pssh.write().unwrap();
        if current_epoch() != epoch {
            return false;
        }
        map.insert(channel_id.to_string(), (value, Instant::now()));
        true
    }

    pub fn clear(&self) {
        self.drm_mpd_cache.write().unwrap().clear();
        self.live_cache.write().unwrap().clear();
        #[cfg(feature = "beta")]
        self.tataplay_pssh.write().unwrap().clear();
        *self.cdn_clock.lock().unwrap() = None;
    }
}

/// Splits a CDN manifest URL into its origin (scheme, host and port), its host
/// (for cookie-domain rewriting), and the directory its segments live in (the
/// URL with its filename dropped, trailing slash kept).
/// Mirrors Go's `buildDrmMpdOutput`/`MpdHandler`:
/// `parsed := url.Parse(tvURL); dir := strings.Join(strings.Split(parsed.Path, "/")[:n-1], "/") + "/"`.
///
/// Operates only on `Url::path()`, never on the raw URL string, so a query
/// string containing its own `/` characters (a real shape for JioTV/extras's
/// `hdnea` token, e.g. `...~acl=/*~...`) can never leak into the split: the
/// `url` crate has already separated path from query at the first
/// unescaped `?` by the time `.path()` returns anything, regardless of what
/// the query itself contains. The one thing worth guarding explicitly is a
/// path that ends in `/` (or has `//` from an upstream quirk) producing a
/// trailing empty split segment that would otherwise get treated as the
/// "filename" to drop, silently keeping the real last directory component
/// and reproducing exactly the one-extra-path-segment symptom this was
/// written to fix.
fn cdn_host_and_dir(url_str: &str) -> Option<(String, String, String)> {
    let parsed = url::Url::parse(url_str).ok()?;
    let host = parsed.host_str()?.to_string();
    let mut origin = format!("{}://{host}", parsed.scheme());
    if let Some(port) = parsed.port() {
        origin.push(':');
        origin.push_str(&port.to_string());
    }
    let mut path = parsed.path();
    while path.len() > 1 && path.ends_with('/') {
        path = &path[..path.len() - 1];
    }
    let dir = match path.rfind('/') {
        Some(idx) => path[..=idx].to_string(),
        None => "/".to_string(),
    };
    Some((origin, host, dir))
}

/// The full playback response for the in-app player, cached for the same
/// short window as the DASH output so reloads and concurrent viewers do not
/// each hit the playback API. Returns the context epoch the response belongs
/// to; callers that encrypt URLs from it must confirm the epoch is unchanged
/// afterwards, or the previous account's data would be re-encrypted under the
/// new epoch.
pub(crate) async fn get_live_cached(
    state: &AppState,
    channel_id: &str,
) -> anyhow::Result<(LiveUrlOutput, u64)> {
    get_live_cached_selected(state, channel_id, None).await
}

/// `get_live_cached` for a caller that already registered a Tata Play
/// selection; a cache miss resolves only while that selection still owns the
/// channel.
pub(crate) async fn get_live_cached_selected(
    state: &AppState,
    channel_id: &str,
    selected: Option<&str>,
) -> anyhow::Result<(LiveUrlOutput, u64)> {
    let epoch = state.secure.current_epoch();
    if let Some(cached) = state.dash_state.get_live(channel_id, epoch) {
        return Ok((cached, epoch));
    }
    let _guard = state.dash_state.live_locks.lock(channel_id).await;
    // Retry once if an account/product switch lands mid-fetch.
    for _ in 0..2 {
        let epoch = state.secure.current_epoch();
        // Another request may have filled the cache while this one waited.
        if let Some(cached) = state.dash_state.get_live(channel_id, epoch) {
            return Ok((cached, epoch));
        }
        let live = crate::stream::fetch_live_selected(state, channel_id, selected).await?;
        if state
            .dash_state
            .set_live(channel_id, live.clone(), epoch, || {
                state.secure.current_epoch()
            })
        {
            return Ok((live, epoch));
        }
    }
    anyhow::bail!("the active account changed while resolving channel {channel_id}; retry")
}

pub(crate) async fn get_drm_mpd(
    state: &AppState,
    channel_id: &str,
    quality: &str,
) -> anyhow::Result<DrmMpdOutput> {
    let epoch = state.secure.current_epoch();
    let cache_key = format!("{channel_id}_{quality}");
    if let Some(cached) = state.dash_state.get_cached(&cache_key, epoch) {
        return Ok(cached);
    }
    let live = crate::stream::fetch_live(state, channel_id).await?;
    if state.secure.current_epoch() == epoch {
        state.observe_playback(channel_id, &live);
    }
    let out = build_drm_mpd_output(state, &live, channel_id, quality)?;
    // If an account switch landed mid-fetch, `out` carries the old account's
    // data under new-epoch URLs: neither cache it nor return it.
    if !state
        .dash_state
        .set_cached(&cache_key, out.clone(), epoch, || {
            state.secure.current_epoch()
        })
    {
        anyhow::bail!("the active account changed while resolving channel {channel_id}; retry");
    }
    Ok(out)
}

pub(crate) fn build_drm_mpd_output(
    state: &AppState,
    live: &LiveUrlOutput,
    channel_id: &str,
    quality: &str,
) -> anyhow::Result<DrmMpdOutput> {
    let bitrates = live.mpd.resolved_bitrates();
    let mut tv_url = television::select_quality(
        quality,
        &bitrates.auto,
        &bitrates.high,
        &bitrates.medium,
        &bitrates.low,
    )
    .to_string();
    if tv_url.is_empty() {
        tv_url = [
            &bitrates.high,
            &bitrates.auto,
            &bitrates.medium,
            &bitrates.low,
        ]
        .into_iter()
        .find(|s| !s.is_empty())
        .cloned()
        .unwrap_or_default();
    }
    if tv_url.is_empty() {
        tv_url = live.mpd.result.clone();
    }
    if tv_url.is_empty() {
        return Ok(DrmMpdOutput {
            is_drm: live.is_drm,
            ..Default::default()
        });
    }

    let channel_enc_url = encrypt_dash_route_value(&state.secure, channel_id, &tv_url);
    let pid = if live.playback_id.is_empty() {
        String::new()
    } else {
        let enc = encrypt_dash_route_value(&state.secure, channel_id, &live.playback_id);
        format!("&pid={enc}")
    };
    let license_url = if !live.resolved_license_url().is_empty() {
        let enc_key =
            encrypt_dash_route_value(&state.secure, channel_id, live.resolved_license_url());
        format!("/drm?auth={enc_key}&channel_id={channel_id}&channel={channel_enc_url}{pid}")
    } else {
        String::new()
    };

    if live.algo_name == "timesplay" {
        return Ok(DrmMpdOutput {
            is_drm: live.is_drm,
            play_url: tv_url,
            license_url,
            ..Default::default()
        });
    }

    let (origin, _host, dir_path) =
        cdn_host_and_dir(&tv_url).ok_or_else(|| anyhow::anyhow!("invalid upstream URL"))?;
    let tv_url_path = state.secure.encrypt_deterministic(&dir_path);
    let tv_url_host = state.secure.encrypt_deterministic(&origin);

    Ok(DrmMpdOutput {
        is_drm: live.is_drm,
        play_url: format!(
            "/render.mpd?auth={channel_enc_url}&channel_id={channel_id}&q={quality}{pid}"
        ),
        license_url,
        tv_url_host,
        tv_url_path,
    })
}

#[derive(serde::Deserialize)]
pub struct QualityQuery {
    q: Option<String>,
}

/// `/live/mpd/:channelID` — the IPTV-facing MPD route: redirects straight to
/// the manifest, no player page (there is no Go-template/HTML player in this
/// rewrite; see the README's UI parity gap).
pub async fn live_mpd_handler(
    axum::extract::Path(channel_id): axum::extract::Path<String>,
    Query(q): Query<QualityQuery>,
    State(state): State<Arc<AppState>>,
    prefix: Option<axum::Extension<crate::api::KeyPrefix>>,
) -> Response {
    let epoch = state.secure.current_epoch();
    let response = live_mpd_inner(channel_id, q, state.clone(), prefix).await;
    state.stable_since(epoch, response)
}

async fn live_mpd_inner(
    channel_id: String,
    q: QualityQuery,
    state: Arc<AppState>,
    prefix: Option<axum::Extension<crate::api::KeyPrefix>>,
) -> Response {
    let quality = q.q.unwrap_or_else(|| "auto".to_string());
    tracing::debug!(channel = %channel_id, quality = %quality, "live mpd request");

    if !state.channel_allowed(&channel_id).await {
        return (
            StatusCode::NOT_FOUND,
            format!("Channel {channel_id} is not available for the active account"),
        )
            .into_response();
    }

    if let Some(ch) = state.custom_channels.get(&channel_id) {
        return Redirect::to(&ch.url).into_response();
    }

    #[cfg(feature = "beta")]
    if crate::tataplay::is_tataplay_channel(&channel_id) {
        state.tataplay.tune_if_changed(&channel_id);
        state.dash_state.invalidate_channel(&channel_id);
    }

    // get_drm_mpd -> fetch_live already refreshes the right credentials
    // (JioTV's or extras's, depending on how the channel routes).
    let drm = get_drm_mpd(&state, &channel_id, &quality).await;
    match drm {
        Ok(out) if !out.play_url.is_empty() => Redirect::to(&out.play_url).into_response(),
        _ => {
            // No DRM/DASH stream available; fall back to this server's own
            // HLS route rather than the Go version's HTML fallback player.
            let prefix_str = prefix.as_ref().map(|p| p.0 .0.clone()).unwrap_or_default();
            let hls_path = format!(
                "{prefix_str}{}",
                crate::extras_state::ExtrasState::live_hls_path(&channel_id, &quality)
            );
            Redirect::to(&hls_path).into_response()
        }
    }
}

/// `/live/key/:channelID` — proxies the Widevine license the same way
/// `/drm` does, after resolving it via `getDrmMpd`.
pub async fn live_key_handler(
    axum::extract::Path(channel_id): axum::extract::Path<String>,
    Query(q): Query<QualityQuery>,
    State(state): State<Arc<AppState>>,
    method: Method,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response {
    let quality = q.q.unwrap_or_else(|| "auto".to_string());
    let cookie_names = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(';')
                .filter_map(|part| part.split('=').next())
                .map(str::trim)
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();
    tracing::debug!(
        channel = %channel_id,
        %method,
        challenge_bytes = body.len(),
        authorization = headers.contains_key(header::AUTHORIZATION),
        cookies = %cookie_names,
        user_agent = headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .unwrap_or(""),
        "live license request"
    );
    if !state.channel_allowed(&channel_id).await {
        return (
            StatusCode::NOT_FOUND,
            format!("Channel {channel_id} is not available for the active account"),
        )
            .into_response();
    }
    #[cfg(feature = "beta")]
    if crate::tataplay::is_tataplay_channel(&channel_id)
        && state.tataplay.active_document().is_some()
        && !state.tataplay.is_active_channel(&channel_id)
    {
        // A delayed key request for the channel the user left must not tune
        // it back before `license_session` gets a chance to refuse it.
        return (StatusCode::CONFLICT, "not the active playback").into_response();
    }
    let drm = match get_drm_mpd(&state, &channel_id, &quality).await {
        Ok(d) => d,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    if drm.license_url.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            format!("No License URL found for channel {channel_id}"),
        )
            .into_response();
    }
    let Some((_, query)) = drm.license_url.split_once('?') else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "malformed license URL").into_response();
    };
    let params: std::collections::HashMap<String, String> =
        url::form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect();
    drm_license_impl(state, params, method, headers, body).await
}

/// `/drm?auth=...&channel=...&channel_id=...` license proxy.
pub async fn drm_license_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    method: Method,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response {
    drm_license_impl(state, params, method, headers, body).await
}

async fn drm_license_impl(
    state: Arc<AppState>,
    params: std::collections::HashMap<String, String>,
    method: Method,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response {
    let Some(auth) = params.get("auth") else {
        return (StatusCode::BAD_REQUEST, "auth is required").into_response();
    };
    let Some(channel_id) = params
        .get("channel_id")
        .filter(|id| !id.is_empty())
        .cloned()
    else {
        return (StatusCode::BAD_REQUEST, "channel_id is required").into_response();
    };
    let decoded_url = match decrypt_dash_route_value(&state.secure, &channel_id, auth) {
        Some(u) => u,
        // A 400, like `/render.mpd`: 403 on a media request must mean the
        // provider refused, which the player UI reports as a denial.
        None => return (StatusCode::BAD_REQUEST, "invalid auth parameter").into_response(),
    };

    // A Fastway channel's license is a PallyCon token exchange: the provider
    // issues a short-lived token for the channel's DRM content ID, and that
    // token is what authorises the Widevine challenge at PallyCon.
    if let Some(content_id) = state.fastway.route(&channel_id) {
        let Some(license_url) = state.fastway.license_url() else {
            return (StatusCode::NOT_FOUND, "fastway is not connected").into_response();
        };
        let (token, meta) = match state.fastway.license_token(&content_id, &state.store).await {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        };
        let req = state
            .http
            .post(&license_url)
            .header(header::USER_AGENT, crate::fastway::LICENSE_USER_AGENT)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header("pallycon-customdata-v2", token)
            .header("pallycon-client-meta", meta);
        return send_license_request(req, body).await;
    }

    // PlayboxTV's license request carries the playback token in the same
    // multi-DRM headers the app sets (`buildCustomDrmSessionManager`).
    #[cfg(feature = "beta")]
    if crate::playbox::is_playbox_channel(&channel_id) {
        let mut req = state
            .http
            .request(method, &decoded_url)
            .header(header::CONTENT_TYPE, "application/octet-stream");
        for (k, v) in state.playbox.license_headers(&channel_id).await {
            req = req.header(k.as_str(), v.as_str());
        }
        return send_license_request(req, body).await;
    }

    // Tata Play's license request carries the subscriber token and device
    // identity that authorised the playback session. The URL and the bearer
    // token must come from the same resolved session: the route's URL was
    // minted when the manifest was served and may belong to an older session
    // by the time the player sends its challenge.
    #[cfg(feature = "beta")]
    if crate::tataplay::is_tataplay_channel(&channel_id) {
        // An absent route id is the legacy shape and stays accepted. A route
        // id that is present but cannot be decrypted belongs to an older
        // account context; treating it as absent would let a stale challenge
        // ride the current session's token.
        let mut playback_id = None;
        if let Some(enc) = params.get("pid") {
            let Some(pid) = decrypt_dash_route_value(&state.secure, &channel_id, enc) else {
                tracing::debug!(channel = %channel_id, "invalid tataplay playback id");
                return (StatusCode::CONFLICT, "stale playback").into_response();
            };
            if !state.tataplay.playback_matches(&channel_id, &pid) {
                tracing::debug!(channel = %channel_id, "stale tataplay license request");
                return (StatusCode::CONFLICT, "stale playback").into_response();
            }
            playback_id = Some(pid);
        }
        let session = match state.tataplay.license_session(&channel_id).await {
            Ok(session) => session,
            Err(error) => {
                tracing::debug!(
                    channel = %channel_id,
                    %error,
                    "tataplay license session unavailable"
                );
                return (StatusCode::CONFLICT, error.to_string()).into_response();
            }
        };
        tracing::debug!(channel = %channel_id, "tataplay license upstream");
        let build = |url: &str, headers: &[(String, String)]| {
            let mut req = state
                .http
                .request(method.clone(), url)
                .header(header::CONTENT_TYPE, "application/octet-stream");
            for (k, v) in headers {
                req = req.header(k.as_str(), v.as_str());
            }
            req
        };
        let send = |url: &str, headers: &[(String, String)], body: Bytes| {
            send_license_parts(build(url, headers), body)
        };
        let mut first = send(&session.url, &session.headers, body.clone()).await;
        // Closing the previous channel's session frees the account's single
        // slot asynchronously. A switch can therefore see `130402` for a
        // moment; resend the same challenge (same URL, token and body) a few
        // times before treating it as a real stream limit.
        let switch_race = state.tataplay.recently_closed(Duration::from_secs(10));
        let mut attempt = 0u32;
        while switch_race
            && !first.0.is_success()
            && license_error_code(&first.1) == Some(130402)
            && attempt < 3
        {
            attempt += 1;
            tokio::time::sleep(Duration::from_millis(400 * u64::from(attempt))).await;
            first = send(&session.url, &session.headers, body.clone()).await;
        }
        if !first.0.is_success() && license_error_code(&first.1) == Some(130402) {
            return tataplay_stream_limit_response();
        }
        if !first.0.is_success() && license_error_code(&first.1) == Some(130601) {
            // Irdeto consumes the playback token with the first challenge.
            // Shaka and ExoPlayer request audio and video keys separately, so
            // the second challenge is refused `130601`. Resolve a fresh
            // session and retry that challenge once.
            if let Ok(fresh) = state
                .tataplay
                .refresh_license(&channel_id, &session.session_id, playback_id.as_deref())
                .await
            {
                tracing::debug!(
                    channel = %channel_id,
                    "retrying tataplay license with a fresh token"
                );
                let retried = send(&fresh.url, &fresh.headers, body).await;
                if !retried.0.is_success() && license_error_code(&retried.1) == Some(130402) {
                    return tataplay_stream_limit_response();
                }
                return license_response(retried);
            }
        }
        return license_response(first);
    }

    // A extras channel's license is authorised by the token already in the
    // license URL; send the headers the extras app sends and skip the
    // cookie-harvesting dance below entirely, mirroring `DRMKeyHandler`'s
    // `extrasLicenseHeaders` branch.
    let is_custom = state.custom_channels.contains(&channel_id);
    if let Some(content_id) = state
        .extras
        .route(&channel_id, state.tv.logged_in(), is_custom)
    {
        let mut req = state
            .http
            .request(method, &decoded_url)
            .header(header::USER_AGENT, crate::extras::PLAYER_USER_AGENT)
            .header(header::CONTENT_TYPE, "application/octet-stream");
        for (k, v) in state.extras.license_headers(&content_id, "") {
            req = req.header(k, v);
        }
        return send_license_request(req, body).await;
    }

    // Airtel Xstream's Widevine endpoint authenticates with the headers the
    // playback response shipped, and speaks the app's JSON envelope unless
    // `licensePayloadKey` is `default`.
    #[cfg(feature = "beta")]
    if crate::airtel::is_airtel_channel(&channel_id) {
        let Some(license) = state.airtel.license(&state.store, &channel_id).await else {
            return (
                StatusCode::NOT_FOUND,
                "airtel: the channel has no license endpoint",
            )
                .into_response();
        };
        let mut req = state
            .http
            .request(method, &decoded_url)
            .header(header::USER_AGENT, crate::airtel::PLAYER_USER_AGENT);
        for (k, v) in &license.headers {
            req = req.header(k.as_str(), v.as_str());
        }
        return send_airtel_license_request(req, body, &license).await;
    }

    // A HEAD request to the (still-encrypted-in-the-URL) channel manifest
    // harvests any auth cookie the CDN wants forwarded onto the license
    // request, mirroring DRMKeyHandler's cookie relay.
    let mut cookie_header = None;
    if let Some(channel_enc) = params.get("channel") {
        if let Some(channel_url) = decrypt_dash_route_value(&state.secure, &channel_id, channel_enc)
        {
            if let Ok(resp) = state.http.head(&channel_url).send().await {
                let cookies: Vec<String> = resp
                    .headers()
                    .get_all(header::SET_COOKIE)
                    .iter()
                    .filter_map(|v| v.to_str().ok())
                    .filter_map(|sc| sc.split(';').next().map(str::trim).map(str::to_string))
                    .filter(|c| c.contains('='))
                    .collect();
                if !cookies.is_empty() {
                    cookie_header = Some(cookies.join("; "));
                }
            }
        }
    }

    let creds = state.tv.creds.read().unwrap().clone().unwrap_or_default();
    let mut req = state
        .http
        .request(method, &decoded_url)
        .header("accesstoken", &creds.access_token)
        .header("Connection", "keep-alive")
        .header("os", "android")
        .header("appName", "RJIL_JioTV")
        .header("subscriberId", &creds.crm)
        .header(header::USER_AGENT, television::PLAYER_USER_AGENT)
        .header("ssotoken", &creds.sso_token)
        .header("x-platform", "android")
        .header("srno", generate_date_time())
        .header("crmid", &creds.crm)
        .header("channelid", &channel_id)
        .header("uniqueId", &creds.unique_id)
        .header("versionCode", "422")
        .header("usergroup", "tvYR7NSNn7rymo3F")
        .header("devicetype", "phone")
        .header("Accept-Encoding", "gzip, deflate")
        .header("osVersion", "13")
        .header("deviceId", &state.tv.device_id)
        .header(header::CONTENT_TYPE, "application/octet-stream");
    if let Some(c) = cookie_header {
        req = req.header(header::COOKIE, c);
    }
    // The player's own Accept/Origin headers aren't forwarded (mirrors the
    // Go handler explicitly deleting them before proxying).
    let _ = &headers;

    send_license_request(req, body).await
}

/// Sends an Airtel Widevine challenge in whichever envelope the provider
/// declared (`PlayerDRMCallback.c`/`.b`), and unwraps the base64 answer back
/// to raw license bytes.
#[cfg(feature = "beta")]
async fn send_airtel_license_request(
    req: reqwest::RequestBuilder,
    body: Bytes,
    license: &crate::airtel::License,
) -> Response {
    let wrapped = license.payload_key != "default" && !license.payload_key.is_empty();
    let body = crate::airtel::license_request_body(license, &body);
    let req = req.header(
        header::CONTENT_TYPE,
        if wrapped {
            "application/json"
        } else {
            "application/octet-stream"
        },
    );
    match req.body(body).send().await {
        Ok(resp) => {
            let status =
                StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let bytes = resp.bytes().await.map(|b| b.to_vec()).unwrap_or_default();
            if !status.is_success() {
                return (status, "airtel: the license server refused the challenge")
                    .into_response();
            }
            let decoded = crate::airtel::license_response_body(license, bytes);
            Response::builder()
                .status(status)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .body(Body::from(decoded))
                .unwrap()
        }
        Err(_) => (StatusCode::BAD_GATEWAY, "airtel: license request failed").into_response(),
    }
}

/// The Irdeto business code from a refused license challenge, if the body is
/// its JSON envelope (`{"code":130601,...}`). `130601` means the playback
/// token that signed the challenge was already spent; `130402` means the
/// account is at its concurrency limit and must not be retried with another
/// session.
#[cfg(feature = "beta")]
fn license_error_code(body: &[u8]) -> Option<i64> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    match value.get("code")? {
        serde_json::Value::Number(number) => number.as_i64(),
        serde_json::Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

#[cfg(feature = "beta")]
fn tataplay_stream_limit_response() -> Response {
    (
        StatusCode::CONFLICT,
        axum::Json(serde_json::json!({
            "code": 130402,
            "message": "Waiting for the previous stream to close"
        })),
    )
        .into_response()
}

async fn send_license_parts(
    req: reqwest::RequestBuilder,
    body: Bytes,
) -> (StatusCode, Bytes, Option<String>) {
    tracing::debug!(challenge_bytes = body.len(), "license request");
    match req.body(body).send().await {
        Ok(resp) => {
            let status =
                StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let ct = resp
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
            let bytes = resp.bytes().await.unwrap_or_default();
            tracing::debug!(
                status = status.as_u16(),
                license_bytes = bytes.len(),
                content_type = ct.as_deref().unwrap_or(""),
                "license response"
            );
            if !status.is_success() {
                let preview = String::from_utf8_lossy(&bytes[..bytes.len().min(300)]);
                tracing::debug!(status = status.as_u16(), body = %preview, "license upstream rejected");
            }
            (status, bytes, ct)
        }
        Err(error) => {
            let kind = if error.is_timeout() {
                "timeout"
            } else if error.is_connect() {
                "connect"
            } else {
                "request"
            };
            tracing::debug!(error = kind, "license upstream request failed");
            (
                StatusCode::BAD_GATEWAY,
                Bytes::from_static(b"license upstream request failed"),
                Some("text/plain".to_string()),
            )
        }
    }
}

fn license_response((status, bytes, ct): (StatusCode, Bytes, Option<String>)) -> Response {
    let mut builder = Response::builder().status(status);
    if let Some(ct) = ct {
        builder = builder.header(header::CONTENT_TYPE, ct);
    }
    builder.body(Body::from(bytes)).unwrap()
}

async fn send_license_request(req: reqwest::RequestBuilder, body: Bytes) -> Response {
    license_response(send_license_parts(req, body).await)
}

fn generate_date_time() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let millis = now.subsec_millis();
    let (y, m, d, hh, mm, _ss) = crate::television::civil_from_unix(now.as_secs());
    format!(
        "{:02}{:02}{:02}{:02}{:02}{:03}",
        y % 100,
        m,
        d,
        hh,
        mm,
        millis
    )
}

#[derive(serde::Deserialize)]
pub struct RenderMpdQuery {
    auth: String,
    channel_id: Option<String>,
    q: Option<String>,
    hdnea: Option<String>,
    #[cfg_attr(not(feature = "beta"), allow(dead_code))]
    pid: Option<String>,
}

/// Tata Play's DASH manifest advertises Widevine without any `cenc:pssh`, so
/// each player derives its init data from an individual track's init segment.
/// Every rendition uses its own key id, and Irdeto's playback token authorises
/// exactly one challenge: a player that asks per track is refused `130601` on
/// the second one.
///
/// Collect every rendition's key id from the init segments, then publish one
/// Widevine PSSH listing all of them. A player that trusts the manifest asks
/// for every key in a single challenge, which is how the provider's own app
/// streams the channel.
#[cfg(feature = "beta")]
async fn tataplay_mpd_pssh(
    state: &Arc<AppState>,
    channel_id: &str,
    mpd_url: &str,
    body: &str,
    epoch: u64,
) -> Option<String> {
    if body.contains("<cenc:pssh") {
        return None;
    }
    if let Some(cached) = state.dash_state.get_tataplay_pssh(channel_id) {
        return Some(cached);
    }
    let init_urls = tataplay_init_urls(mpd_url, body);
    if init_urls.is_empty() {
        return None;
    }
    // The samples themselves can name a different key than the init segment's
    // `tenc`/PSSH (`seig` sample groups). That key is the one the CDM actually
    // needs, so probe one small media segment for its key id too.
    let media_urls = tataplay_media_probe_urls(mpd_url, body);
    let init_expected = init_urls.len();
    let media_expected = media_urls.len();
    let mut urls = init_urls;
    urls.extend(media_urls.iter().cloned());
    let cookie = crate::stream::extract_hdnea_from_url(mpd_url).map(|token| hdnea_cookie(&token));
    let mut fetches = tokio::task::JoinSet::new();
    for url in urls {
        let http = state.http.clone();
        let cookie = cookie.clone();
        let task_url = url.clone();
        fetches.spawn(async move {
            let mut request = http.get(&url).timeout(Duration::from_secs(8));
            if let Some(cookie) = &cookie {
                request = request.header(header::COOKIE, cookie);
            }
            let response = request.send().await.ok()?;
            response.bytes().await.ok().map(|bytes| (task_url, bytes))
        });
    }
    let mut kids: Vec<[u8; 16]> = Vec::new();
    let mut seig_found = false;
    let mut init_ok = 0usize;
    let mut media_ok = 0usize;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        if left.is_zero() {
            fetches.abort_all();
            break;
        }
        match tokio::time::timeout(left, fetches.join_next()).await {
            Ok(Some(Ok(Some((url, bytes))))) => {
                if media_urls.contains(&url) {
                    media_ok += 1;
                    seig_found |= seig_key_ids(&bytes, &mut kids);
                } else {
                    init_ok += 1;
                    pssh_key_ids(&bytes, &mut kids);
                }
            }
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(_) => {
                fetches.abort_all();
                break;
            }
        }
    }
    if init_ok < init_expected || media_ok < media_expected || kids.is_empty() || !seig_found {
        // Without the sample group's key the merged PSSH would ask for keys
        // that do not decrypt the media; fall back to the in-band PSSH instead.
        return None;
    }
    let pssh = merged_widevine_pssh(&kids);
    if !state
        .dash_state
        .set_tataplay_pssh(channel_id, pssh.clone(), epoch, || {
            state.secure.current_epoch()
        })
    {
        return None;
    }
    Some(pssh)
}

/// Absolute URLs for every `Representation`'s init segment in the origin
/// manifest. Tata Play uses one `BaseURL` for the MPD and a `SegmentTemplate`
/// per adaptation set; the MPD's `hdnea` query is reused for the fetches.
#[cfg(feature = "beta")]
fn tataplay_init_urls(mpd_url: &str, body: &str) -> Vec<String> {
    let Ok(base) = url::Url::parse(mpd_url) else {
        return Vec::new();
    };
    let base_url = xml_tag_text(body, "BaseURL").unwrap_or("");
    let query = base.query().map(str::to_string);
    let mut urls = Vec::new();
    for (tag, block) in child_blocks(body, "AdaptationSet") {
        if !(tag.contains("contentType=\"audio\"") || tag.contains("contentType=\"video\"")) {
            continue;
        }
        let Some(template) = xml_tag(block, "SegmentTemplate") else {
            continue;
        };
        let Some(init) = attr_value(template, "initialization") else {
            continue;
        };
        for rep in xml_tags(block, "Representation") {
            let Some(id) = attr_value(rep, "id") else {
                continue;
            };
            let init = init.replace("$RepresentationID$", id);
            let relative = if base_url.is_empty() {
                init
            } else {
                format!("{base_url}{init}")
            };
            let Ok(mut url) = base.join(&relative) else {
                continue;
            };
            if url.query().is_none() {
                if let Some(query) = &query {
                    url.set_query(Some(query));
                }
            }
            urls.push(url.to_string());
        }
    }
    urls
}

/// The opening tag of the first `name` element, including its `>`.
#[cfg(feature = "beta")]
fn xml_tag<'a>(body: &'a str, name: &str) -> Option<&'a str> {
    let start = body.find(&format!("<{name}"))?;
    let end = start + body[start..].find('>')?;
    Some(&body[start..=end])
}

/// The first `name` element's text content, or "" for a self-closing tag.
#[cfg(feature = "beta")]
fn xml_tag_text<'a>(body: &'a str, name: &str) -> Option<&'a str> {
    let open = xml_tag(body, name)?;
    if open.ends_with("/>") {
        return Some("");
    }
    let start = body.find(open)? + open.len();
    let close = format!("</{name}>");
    let end = start + body[start..].find(&close)?;
    Some(&body[start..end])
}

#[cfg(feature = "beta")]
fn xml_tags<'a>(body: &'a str, name: &str) -> Vec<&'a str> {
    let needle = format!("<{name}");
    let mut tags = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find(&needle) {
        let Some(end) = rest[start..].find('>') else {
            break;
        };
        tags.push(&rest[start..=start + end]);
        rest = &rest[start + end + 1..];
    }
    tags
}

/// Every `<name ...>...</name>` child as (opening tag, inner body), using the
/// first matching close tag.
#[cfg(feature = "beta")]
fn child_blocks<'a>(body: &'a str, name: &str) -> Vec<(&'a str, &'a str)> {
    let needle = format!("<{name}");
    let close = format!("</{name}>");
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find(&needle) {
        let Some(open_end) = rest[start..].find('>') else {
            break;
        };
        let after = start + open_end + 1;
        let Some(close_rel) = rest[after..].find(&close) else {
            break;
        };
        out.push((
            &rest[start..=start + open_end],
            &rest[after..after + close_rel],
        ));
        rest = &rest[after + close_rel + close.len()..];
    }
    out
}

#[cfg(feature = "beta")]
fn attr_value<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = start + tag[start..].find('"')?;
    Some(&tag[start..end])
}

/// Widevine key ids from every `pssh` box in an init segment.
#[cfg(feature = "beta")]
fn pssh_key_ids(data: &[u8], kids: &mut Vec<[u8; 16]>) {
    let mut at = 0;
    while at + 32 <= data.len() {
        let Some(pos) = data[at..].windows(4).position(|w| w == b"pssh") else {
            break;
        };
        let marker = at + pos;
        if marker < 4 {
            at = marker + 4;
            continue;
        }
        let start = marker - 4;
        let size = u32::from_be_bytes(data[start..start + 4].try_into().unwrap()) as usize;
        if size < 32 || start + size > data.len() {
            at = marker + 4;
            continue;
        }
        let box_bytes = &data[start..start + size];
        if box_bytes[12..28] != WIDEVINE_SYSTEM_ID {
            at = start + size;
            continue;
        }
        // Version 1 stores a KID count and KIDs before the system data; a
        // version 0 box stores a protobuf WidevinePsshData from byte 32.
        if box_bytes[8] == 1 {
            if let Some(count_bytes) = box_bytes.get(28..32) {
                let count = u32::from_be_bytes(count_bytes.try_into().unwrap()) as usize;
                for index in 0..count {
                    let kid_start = 32 + index * 16;
                    let Some(value) = box_bytes.get(kid_start..kid_start + 16) else {
                        break;
                    };
                    let mut kid = [0u8; 16];
                    kid.copy_from_slice(value);
                    if !kids.contains(&kid) {
                        kids.push(kid);
                    }
                }
            }
            at = start + size;
            continue;
        }
        let payload = &box_bytes[32..];
        let mut i = 0;
        while i < payload.len() {
            let tag = payload[i];
            i += 1;
            let field = tag >> 3;
            match tag & 7 {
                0 => {
                    while i < payload.len() && payload[i] & 0x80 != 0 {
                        i += 1;
                    }
                    i += 1;
                }
                2 => {
                    let Some(&len) = payload.get(i) else { break };
                    i += 1;
                    let len = len as usize;
                    let Some(value) = payload.get(i..i + len) else {
                        break;
                    };
                    if field == 2 && len == 16 {
                        let mut kid = [0u8; 16];
                        kid.copy_from_slice(value);
                        if !kids.contains(&kid) {
                            kids.push(kid);
                        }
                    }
                    i += len;
                }
                5 => i += 4,
                1 => i += 8,
                _ => break,
            }
        }
        at = start + size;
    }
}

/// A single small media segment URL (prefer audio) so the caller can read the
/// sample-group key id (`seig`) that the encrypted samples actually use.
#[cfg(feature = "beta")]
fn tataplay_media_probe_urls(mpd_url: &str, body: &str) -> Vec<String> {
    let Ok(base) = url::Url::parse(mpd_url) else {
        return Vec::new();
    };
    let base_url = xml_tag_text(body, "BaseURL").unwrap_or("");
    let query = base.query().map(str::to_string);
    let mut audio = None;
    let mut video = None;
    for (tag, block) in child_blocks(body, "AdaptationSet") {
        let is_audio = tag.contains("contentType=\"audio\"");
        let is_video = tag.contains("contentType=\"video\"");
        if !is_audio && !is_video {
            continue;
        }
        let Some(template) = xml_tag(block, "SegmentTemplate") else {
            continue;
        };
        let Some(media) = attr_value(template, "media") else {
            continue;
        };
        if !media.contains("$Number$") {
            continue;
        }
        let start = attr_value(template, "startNumber")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(1);
        // Probe the newest segment in the timeline: the first one may still be
        // encrypted with a rotation key from hours ago.
        let mut unbounded = false;
        let count = xml_tag_text(block, "SegmentTimeline")
            .map(|timeline| {
                xml_tags(timeline, "S")
                    .into_iter()
                    .map(|entry| {
                        let repeat = attr_value(entry, "r");
                        if repeat == Some("-1") {
                            unbounded = true;
                            return 1;
                        }
                        repeat
                            .and_then(|value| value.parse::<u64>().ok())
                            .map(|repeat| repeat + 1)
                            .unwrap_or(1)
                    })
                    .sum::<u64>()
            })
            .filter(|count| *count > 0)
            .unwrap_or(1);
        if unbounded {
            // The timeline is open-ended; the newest segment cannot be
            // identified without the Period boundary.
            return Vec::new();
        }
        let number = start.saturating_add(count.saturating_sub(1));
        let Some(rep) = xml_tags(block, "Representation")
            .into_iter()
            .min_by_key(|rep| {
                attr_value(rep, "bandwidth")
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(u64::MAX)
            })
        else {
            continue;
        };
        let Some(id) = attr_value(rep, "id") else {
            continue;
        };
        let mut path = media
            .replace("$RepresentationID$", id)
            .replace("$Number$", &number.to_string());
        if let Some(bandwidth) = attr_value(rep, "bandwidth") {
            path = path.replace("$Bandwidth$", bandwidth);
        }
        if path.contains('$') {
            continue;
        }
        let relative = if base_url.is_empty() {
            path
        } else {
            format!("{base_url}{path}")
        };
        let Ok(mut url) = base.join(&relative) else {
            continue;
        };
        if url.query().is_none() {
            if let Some(query) = &query {
                url.set_query(Some(query));
            }
        }
        if is_audio && audio.is_none() {
            audio = Some(url.to_string());
        } else if is_video && video.is_none() {
            video = Some(url.to_string());
        }
    }
    audio.into_iter().chain(video).take(2).collect()
}

/// Widevine key ids from CENC `seig` sample group descriptions. These
/// override the init segment's default key for the samples that reference
/// them, and must be requested in the license challenge.
#[cfg(feature = "beta")]
fn seig_key_ids(data: &[u8], kids: &mut Vec<[u8; 16]>) -> bool {
    let mut found = false;
    let mut at = 0;
    while at + 24 <= data.len() {
        let Some(pos) = data[at..].windows(4).position(|w| w == b"sgpd") else {
            break;
        };
        let marker = at + pos;
        if marker < 4 {
            at = marker + 4;
            continue;
        }
        let start = marker - 4;
        let size = u32::from_be_bytes(data[start..start + 4].try_into().unwrap()) as usize;
        if size < 24 || start + size > data.len() {
            at = marker + 4;
            continue;
        }
        let box_bytes = &data[start..start + size];
        let version = box_bytes[8];
        if &box_bytes[12..16] != b"seig" {
            at = start + size;
            continue;
        }
        let default_len = u32::from_be_bytes(box_bytes[16..20].try_into().unwrap()) as usize;
        let count = u32::from_be_bytes(box_bytes[20..24].try_into().unwrap()) as usize;
        let mut off = 24;
        for _ in 0..count {
            let len = if default_len == 0 {
                let Some(value) = box_bytes.get(off..off + 4) else {
                    break;
                };
                off += 4;
                u32::from_be_bytes(value.try_into().unwrap()) as usize
            } else {
                default_len
            };
            let Some(entry) = box_bytes.get(off..off + len) else {
                break;
            };
            off += len;
            let kid_at = if version == 1 && entry.len() >= 20 {
                Some(4)
            } else if entry.len() >= 18 {
                Some(2)
            } else {
                None
            };
            if let Some(kid_at) = kid_at {
                let mut kid = [0u8; 16];
                kid.copy_from_slice(&entry[kid_at..kid_at + 16]);
                found = true;
                if !kids.contains(&kid) {
                    kids.push(kid);
                }
            }
        }
        at = start + size;
    }
    found
}

/// One Widevine PSSH v0 box whose `WidevinePsshData` lists every key id.
#[cfg(feature = "beta")]
fn merged_widevine_pssh(kids: &[[u8; 16]]) -> String {
    let mut payload = Vec::with_capacity(kids.len() * 18);
    for kid in kids {
        payload.extend_from_slice(&[0x12, 0x10]);
        payload.extend_from_slice(kid);
    }
    let size = 32 + payload.len();
    let mut pssh = Vec::with_capacity(size);
    pssh.extend_from_slice(&(size as u32).to_be_bytes());
    pssh.extend_from_slice(b"pssh");
    pssh.extend_from_slice(&[0, 0, 0, 0]);
    pssh.extend_from_slice(&WIDEVINE_SYSTEM_ID);
    pssh.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    pssh.extend_from_slice(&payload);
    base64::engine::general_purpose::STANDARD.encode(pssh)
}

/// Replaces each Widevine `ContentProtection` with one that carries the merged
/// PSSH, and declares the `cenc` namespace the child element needs.
#[cfg(feature = "beta")]
fn inject_widevine_pssh(body: &str, pssh: &str) -> String {
    let mut out = String::with_capacity(body.len() + pssh.len() * 4 + 64);
    let mut rest = body;
    while let Some(start) = rest.find("<ContentProtection") {
        let Some(open_end) = rest[start..].find('>') else {
            break;
        };
        let end = start + open_end;
        let tag = &rest[start..=end];
        out.push_str(&rest[..start]);
        if tag.contains("urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed") {
            if let Some(open) = tag.strip_suffix("/>") {
                out.push_str(open);
                out.push_str(&format!(
                    "><cenc:pssh>{pssh}</cenc:pssh></ContentProtection>"
                ));
            } else {
                out.push_str(tag);
                out.push_str(&format!("<cenc:pssh>{pssh}</cenc:pssh>"));
            }
        } else {
            out.push_str(tag);
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    if !out.contains("xmlns:cenc=") {
        if let Some(start) = out.find("<MPD") {
            if let Some(end) = out[start..].find('>') {
                let mut with_ns = String::with_capacity(out.len() + 40);
                with_ns.push_str(&out[..start + end]);
                with_ns.push_str(" xmlns:cenc=\"urn:mpeg:cenc:2013\"");
                with_ns.push_str(&out[start + end..]);
                return with_ns;
            }
        }
    }
    out
}

/// `/render.mpd` — fetches the MPD, rewrites `BaseURL` to route segments
/// through `/render.dash/...`, injects a `UTCTiming` element pointing at
/// `/dashtime`, and records the CDN's own clock from `publishTime`.
pub async fn render_mpd_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<RenderMpdQuery>,
) -> Response {
    let epoch = state.secure.current_epoch();
    let response = render_mpd_inner(state.clone(), q).await;
    state.stable_since(epoch, response)
}

/// Apply a successful live refresh to the manifest URL, discarding it when
/// the account changed during the fetch.
fn apply_live_refresh(
    state: &AppState,
    channel_id: &str,
    live: &LiveUrlOutput,
    quality: &str,
    epoch: u64,
    decrypted: &mut String,
) {
    if state.secure.current_epoch() != epoch {
        return;
    }
    state.observe_playback(channel_id, live);
    let fresh = television::select_best_live_mpd_url(live, quality);
    if !fresh.is_empty() {
        *decrypted = fresh;
    }
}

async fn render_mpd_inner(state: Arc<AppState>, q: RenderMpdQuery) -> Response {
    let Some(channel_id) = q.channel_id.as_deref().filter(|id| !id.is_empty()) else {
        return (StatusCode::BAD_REQUEST, "channel_id is required").into_response();
    };
    tracing::debug!(channel = %channel_id, "render mpd request");
    let mut decrypted = match decrypt_dash_route_value(&state.secure, channel_id, &q.auth) {
        Some(u) => u,
        None => return (StatusCode::BAD_REQUEST, "invalid auth parameter").into_response(),
    };
    let quality = q.q.clone().unwrap_or_default();
    let epoch = state.secure.current_epoch();

    #[cfg(feature = "beta")]
    {
        let tata = crate::tataplay::is_tataplay_channel(channel_id);
        if tata {
            if let Some(enc) = q.pid.as_deref() {
                let Some(pid) = decrypt_dash_route_value(&state.secure, channel_id, enc) else {
                    return (StatusCode::CONFLICT, "stale playback").into_response();
                };
                if !state.tataplay.playback_matches(channel_id, &pid) {
                    return (StatusCode::CONFLICT, "stale playback").into_response();
                }
            }
        }
        if tata {
            // Refresh only the tune that owns the slot. A late manifest for a
            // channel the user left must not reopen it or close the new one.
            if let Ok(live) = state.tataplay.refresh_live(channel_id).await {
                apply_live_refresh(&state, channel_id, &live, &quality, epoch, &mut decrypted);
            }
        } else if let Ok(live) = crate::stream::fetch_live(&state, channel_id).await {
            apply_live_refresh(&state, channel_id, &live, &quality, epoch, &mut decrypted);
        }
    }
    #[cfg(not(feature = "beta"))]
    if let Ok(live) = crate::stream::fetch_live(&state, channel_id).await {
        apply_live_refresh(&state, channel_id, &live, &quality, epoch, &mut decrypted);
    }

    let Some((proxy_origin, proxy_host, base_path)) = cdn_host_and_dir(&decrypted) else {
        return (StatusCode::BAD_REQUEST, "invalid upstream URL").into_response();
    };
    let enc_host = encrypt_dash_route_value(&state.secure, channel_id, &proxy_origin);
    let enc_path = encrypt_dash_route_value(&state.secure, channel_id, &base_path);

    let encoded_channel = urlencoding::encode(channel_id);
    // The token can arrive as an explicit query parameter (players that pass
    // it through) or only inside the upstream manifest URL (Tata Play). Either
    // way the segment route must carry it.
    let cached_hdnea = q
        .hdnea
        .clone()
        .or_else(|| crate::stream::extract_hdnea_from_url(&decrypted));
    let mut dash_base =
        format!("/render.dash/channel/{encoded_channel}/host/{enc_host}/path/{enc_path}");
    if let Some(h) = &cached_hdnea {
        if !h.is_empty() {
            let enc =
                encrypt_dash_route_value(&state.secure, channel_id, &format!("__hdnea__={h}"));
            dash_base = format!("/render.dash/channel/{encoded_channel}/host/{enc_host}/path/{enc_path}/hdnea/{enc}");
        }
    }

    let (status, body, set_cookies) = proxy_mpd(&state, channel_id, &decrypted).await;
    let mut status = status;
    let mut body = body;
    let mut set_cookies = set_cookies;
    if matches!(status, 401 | 403) {
        let stripped = crate::stream::strip_hdnea_from_url(&decrypted);
        let (s, b, c) = proxy_mpd(&state, channel_id, &stripped).await;
        status = s;
        body = b;
        set_cookies = c;
    }
    #[cfg(feature = "beta")]
    if matches!(status, 401 | 403 | 404) && crate::playbox::is_playbox_channel(channel_id) {
        state.playbox.invalidate_playback(channel_id).await;
    }

    let upstream_hdnea = set_cookies.iter().find_map(|sc| {
        sc.split(';')
            .map(str::trim)
            .find_map(|p| p.strip_prefix("__hdnea__="))
    });
    if let Some(h) = upstream_hdnea {
        let enc = encrypt_dash_route_value(&state.secure, channel_id, &format!("__hdnea__={h}"));
        dash_base = format!(
            "/render.dash/channel/{encoded_channel}/host/{enc_host}/path/{enc_path}/hdnea/{enc}"
        );
    }

    let mut body_str = String::from_utf8_lossy(&body).to_string();

    if let Some(pt) = extract_publish_time(&body_str) {
        state.dash_state.record_publish_time(pt);
    }

    if let Some(root_end) = body_str
        .find("<MPD")
        .and_then(|i| body_str[i..].find('>').map(|j| i + j))
    {
        // The origin publishes its SegmentTimeline a couple of segments behind
        // its own clock. Without a presentation delay Shaka asks for the
        // not-yet-published edge, gets 404 and loops on the manifest.
        let mut inserted = 0;
        if !body_str[..root_end].contains("suggestedPresentationDelay") {
            let attr = " suggestedPresentationDelay=\"PT24S\"";
            body_str.insert_str(root_end, attr);
            inserted = attr.len();
        }
        if !body_str.contains("<UTCTiming") {
            let utc_timing = "<UTCTiming schemeIdUri=\"urn:mpeg:dash:utc:http-xsdate:2014\" value=\"/dashtime\"/>";
            body_str.insert_str(root_end + inserted + 1, utc_timing);
        }
    }

    #[cfg(feature = "beta")]
    if crate::tataplay::is_tataplay_channel(channel_id) {
        if let Some(pssh) =
            tataplay_mpd_pssh(&state, channel_id, &decrypted, &body_str, epoch).await
        {
            body_str = inject_widevine_pssh(&body_str, &pssh);
        }
    }

    body_str = rewrite_base_url(&body_str, &dash_base);

    let status_code = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut builder = Response::builder()
        .status(status_code)
        .header(header::CONTENT_TYPE, "application/dash+xml");
    // Forward the CDN's own cookies to the client (Shaka and other
    // same-origin players use them for the segment requests that follow),
    // with Domain stripped and Path rewritten to this server's own
    // /render.dash prefix — mirrors MpdHandler's Set-Cookie rewriting.
    for raw in &set_cookies {
        let domain_needle = format!("Domain={proxy_host};");
        let rewritten = raw
            .replace(&domain_needle, "")
            .replacen("path=/", "path=/render.dash", 1);
        builder = builder.header(header::SET_COOKIE, rewritten);
    }
    builder.body(Body::from(body_str)).unwrap()
}

/// Points segment requests at `/render.dash`, like Go's MpdHandler: every
/// existing `<BaseURL>` becomes `<dash_base>/dash/`; without one, a
/// `<BaseURL><dash_base>/</BaseURL>` goes after every `<Period ...>` tag.
fn rewrite_base_url(body: &str, dash_base: &str) -> String {
    if body.contains("<BaseURL>") {
        let mut out = String::with_capacity(body.len());
        let mut rest = body;
        while let Some(start) = rest.find("<BaseURL>") {
            let Some(end_rel) = rest[start..].find("</BaseURL>") else {
                break;
            };
            out.push_str(&rest[..start]);
            out.push_str(&format!("<BaseURL>{dash_base}/dash/</BaseURL>"));
            rest = &rest[start + end_rel + "</BaseURL>".len()..];
        }
        out.push_str(rest);
        return out;
    }
    let mut out = String::with_capacity(body.len() + 128);
    let mut rest = body;
    while let Some(start) = rest.find("<Period") {
        // Skip tags that only start with "<Period", such as <PeriodX>.
        let after = rest[start + "<Period".len()..].chars().next();
        let Some(tag_end_rel) = rest[start..].find('>') else {
            break;
        };
        let insert_at = start + tag_end_rel + 1;
        out.push_str(&rest[..insert_at]);
        if matches!(after, Some(c) if c.is_whitespace() || c == '>' || c == '/') {
            out.push_str(&format!("\n<BaseURL>{dash_base}/</BaseURL>"));
        }
        rest = &rest[insert_at..];
    }
    out.push_str(rest);
    out
}

fn extract_publish_time(body: &str) -> Option<SystemTime> {
    let idx = body.find("publishTime=\"")? + "publishTime=\"".len();
    let end = body[idx..].find('"')?;
    let value = &body[idx..idx + end];
    parse_rfc3339(value)
}

/// A minimal RFC3339 parser (`2024-01-02T03:04:05.678Z` or with a numeric
/// offset), just enough for MPD `publishTime` values, avoiding a datetime
/// dependency.
pub(crate) fn parse_rfc3339(s: &str) -> Option<SystemTime> {
    let (date, rest) = s.split_once('T')?;
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    let rest = rest.trim_end_matches('Z');
    let (time_part, offset) = rest
        .split_once('+')
        .map(|(a, b)| (a, Some(("+", b))))
        .unwrap_or_else(|| {
            // A "-" offset only appears after the seconds field, so look past
            // index 5 (HH:MM) to avoid the date's own dashes not being present here.
            if let Some(idx) = rest.rfind('-') {
                if idx > 5 {
                    return (&rest[..idx], Some(("-", &rest[idx + 1..])));
                }
            }
            (rest, None)
        });
    let mut tparts = time_part.split(':');
    let hh: i64 = tparts.next()?.parse().ok()?;
    let mm: i64 = tparts.next()?.parse().ok()?;
    let ss_frac = tparts.next()?;
    let ss: i64 = ss_frac.split('.').next()?.parse().ok()?;

    let days = days_from_civil(y, m, d);
    let mut total_secs = days * 86400 + hh * 3600 + mm * 60 + ss;
    if let Some((sign, off)) = offset {
        let mut op = off.split(':');
        let oh: i64 = op.next()?.parse().ok()?;
        let om: i64 = op.next().unwrap_or("0").parse().unwrap_or(0);
        let off_secs = oh * 3600 + om * 60;
        total_secs -= if sign == "+" { off_secs } else { -off_secs };
    }
    if total_secs < 0 {
        return None;
    }
    Some(UNIX_EPOCH + Duration::from_secs(total_secs as u64))
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// The extras CDN refuses DASH manifests/segments unless the User-Agent starts
/// with `JioTV.Plus/` (see `pkg/extras`'s `PlayerUserAgent` doc comment);
/// Fastway's CDN expects the native ExoPlayer agent. `AppState` tracks which
/// hosts need which.
fn player_user_agent_for(state: &AppState, url: &str) -> &'static str {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| state.player_user_agent_for(h)))
        .unwrap_or(television::PLAYER_USER_AGENT)
}

async fn proxy_mpd(state: &AppState, channel_id: &str, url: &str) -> (u16, Vec<u8>, Vec<String>) {
    #[cfg(not(feature = "beta"))]
    let _ = channel_id;
    let mut req = state.http.get(url);
    #[cfg(feature = "beta")]
    if crate::playbox::is_playbox_channel(channel_id) {
        let mut has_user_agent = false;
        for (name, value) in state.playbox.media_headers(channel_id).await {
            if name.eq_ignore_ascii_case("user-agent") {
                has_user_agent = true;
            }
            req = req.header(name.as_str(), value.as_str());
        }
        if !has_user_agent {
            req = req.header(header::USER_AGENT, television::PLAYER_USER_AGENT);
        }
    } else if crate::airtel::is_airtel_channel(channel_id) {
        for (name, value) in state.airtel.media_headers(&state.store, channel_id).await {
            req = req.header(name.as_str(), value.as_str());
        }
    } else {
        req = req.header(header::USER_AGENT, player_user_agent_for(state, url));
    }
    #[cfg(not(feature = "beta"))]
    {
        req = req.header(header::USER_AGENT, player_user_agent_for(state, url));
    }
    match req.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let cookies: Vec<String> = resp
                .headers()
                .get_all(header::SET_COOKIE)
                .iter()
                .filter_map(|v| v.to_str().ok())
                .map(str::to_string)
                .collect();
            let body = resp.bytes().await.map(|b| b.to_vec()).unwrap_or_default();
            (status, body, cookies)
        }
        Err(_) => (502, Vec::new(), Vec::new()),
    }
}

/// `/render.dash/channel/<id>/host/<enc>/path/<enc>[/hdnea/<enc>]/<segment-path>` —
/// proxies one DASH segment or the manifest's own directory listing to the
/// real CDN. The whole remainder of the path is taken as a single wildcard
/// and parsed by hand (mirrors `DashHandler`'s own manual parsing, which
/// exists for the same reason: the segment path's own depth is unbounded).
/// Parses `/render.dash/channel/<id>/host/<enc>/path/<enc>[/hdnea/<enc>]/<segment-path>`
/// into `(channel_id, enc_host, enc_path, enc_hdnea, segment_path)`, where
/// `segment_path` starts with `/`. A pure function (no decryption, no I/O)
/// so it's directly testable against the exact path shapes a real client
/// resolves a relative `SegmentTemplate` reference into.
fn split_dash_path(path: &str) -> Option<(String, &str, &str, Option<&str>, String)> {
    let rest = path.strip_prefix("/render.dash/channel/")?;
    let (encoded_channel, rest) = rest.split_once("/host/")?;
    let channel_id = urlencoding::decode(encoded_channel).ok()?.into_owned();
    let (enc_host, remainder) = rest.split_once("/path/")?;
    if let Some((before, after)) = remainder.split_once("/hdnea/") {
        let (enc_hdnea, seg) = after.split_once('/').unwrap_or((after, ""));
        Some((
            channel_id,
            enc_host,
            before,
            Some(enc_hdnea),
            format!("/{seg}"),
        ))
    } else {
        let (p, seg) = remainder.split_once('/').unwrap_or((remainder, ""));
        Some((channel_id, enc_host, p, None, format!("/{seg}")))
    }
}

/// Builds the final upstream CDN URL from a decrypted origin (scheme, host and
/// port), the segment's own relative path, and the query the client's resolved
/// segment request carried (its own `?m=...` cache-buster, say) — kept pure
/// and separate from `cdn_host_and_dir` (which goes the other way, from an
/// upstream manifest URL down to origin+dir) so both directions of this round
/// trip are independently testable.
fn build_dash_proxy_url(origin: &str, base_path: &str, segment_path: &str, query: &str) -> String {
    let base_path = base_path.trim_end_matches('/');
    let mut url = format!("{origin}{base_path}{segment_path}");
    if !query.is_empty() {
        url.push('?');
        url.push_str(query);
    }
    url
}

pub async fn render_dash_handler(
    State(state): State<Arc<AppState>>,
    uri: axum::http::Uri,
) -> Response {
    let (channel_id, enc_host, enc_path, enc_hdnea, segment_path) =
        match split_dash_path(uri.path()) {
            Some(v) => v,
            None => return (StatusCode::BAD_REQUEST, "malformed dash path").into_response(),
        };

    let hdnea_token = match enc_hdnea {
        Some(enc) => match decrypt_dash_route_value(&state.secure, &channel_id, enc)
            .and_then(|s| s.strip_prefix("__hdnea__=").map(str::to_string))
        {
            Some(token) => Some(token),
            None => return (StatusCode::BAD_REQUEST, "invalid hdnea parameter").into_response(),
        },
        None => None,
    };

    let origin = match decrypt_dash_route_value(&state.secure, &channel_id, enc_host) {
        Some(h) => h,
        None => return (StatusCode::BAD_REQUEST, "invalid host parameter").into_response(),
    };
    let base_path = match decrypt_dash_route_value(&state.secure, &channel_id, enc_path) {
        Some(p) => p,
        None => return (StatusCode::BAD_REQUEST, "invalid path parameter").into_response(),
    };

    let proxy_url = build_dash_proxy_url(
        &origin,
        &base_path,
        &segment_path,
        uri.query().unwrap_or(""),
    );

    let (status, body, ct) =
        proxy_dash_segment(&state, &channel_id, &proxy_url, hdnea_token.as_deref()).await;
    let mut status = status;
    let mut body = body;
    let mut ct = ct;
    if matches!(status, 401 | 403) {
        #[cfg(feature = "beta")]
        if crate::playbox::is_playbox_channel(&channel_id) {
            state.playbox.invalidate_playback(&channel_id).await;
            if crate::stream::fetch_live(&state, &channel_id).await.is_ok() {
                let (s, b, c) =
                    proxy_dash_segment(&state, &channel_id, &proxy_url, hdnea_token.as_deref())
                        .await;
                status = s;
                body = b;
                ct = c;
            }
        } else {
            let (s, b, c) = proxy_dash_segment(&state, &channel_id, &proxy_url, None).await;
            status = s;
            body = b;
            ct = c;
        }
        #[cfg(not(feature = "beta"))]
        {
            let (s, b, c) = proxy_dash_segment(&state, &channel_id, &proxy_url, None).await;
            status = s;
            body = b;
            ct = c;
        }
    }

    let mut builder =
        Response::builder().status(StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY));
    if let Some(ct) = ct {
        builder = builder.header(header::CONTENT_TYPE, ct);
    }
    builder.body(body).unwrap()
}

/// Akamai's token filter reads `hdnea` as a cookie. The `__hdnea__` name is
/// what the JioTV/Fastway CDNs read; sending both keeps those paths working
/// while Tata Play's `bpk-tv` origin accepts the request.
fn hdnea_cookie(token: &str) -> String {
    format!("hdnea={token}; __hdnea__={token}")
}

async fn proxy_dash_segment(
    state: &AppState,
    channel_id: &str,
    url: &str,
    hdnea: Option<&str>,
) -> (u16, Body, Option<String>) {
    #[cfg(not(feature = "beta"))]
    let _ = channel_id;
    let mut req = state.http.get(url);
    #[cfg(feature = "beta")]
    if crate::playbox::is_playbox_channel(channel_id) {
        let mut has_user_agent = false;
        for (name, value) in state.playbox.media_headers(channel_id).await {
            if name.eq_ignore_ascii_case("user-agent") {
                has_user_agent = true;
            }
            req = req.header(name.as_str(), value.as_str());
        }
        if !has_user_agent {
            req = req.header(header::USER_AGENT, television::PLAYER_USER_AGENT);
        }
    } else if crate::airtel::is_airtel_channel(channel_id) {
        for (name, value) in state.airtel.media_headers(&state.store, channel_id).await {
            req = req.header(name.as_str(), value.as_str());
        }
        if let Some(t) = hdnea {
            req = req.header(header::COOKIE, hdnea_cookie(t));
        }
    } else {
        req = req.header(header::USER_AGENT, player_user_agent_for(state, url));
        if let Some(t) = hdnea {
            req = req.header(header::COOKIE, hdnea_cookie(t));
        }
    }
    #[cfg(not(feature = "beta"))]
    {
        req = req.header(header::USER_AGENT, player_user_agent_for(state, url));
        if let Some(t) = hdnea {
            req = req.header(header::COOKIE, hdnea_cookie(t));
        }
    }
    match req.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let ct = resp
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            let body = crate::stream::passthrough_body(resp);
            (status, body, ct)
        }
        Err(_) => (502, Body::empty(), None),
    }
}

/// `/dashtime` — a UTCTiming clock source; the CDN's own extrapolated clock
/// once one has been observed, else the machine clock.
pub async fn dashtime_handler(State(state): State<Arc<AppState>>) -> Response {
    let now = state.dash_state.cdn_now().unwrap_or_else(SystemTime::now);
    let secs = now.duration_since(UNIX_EPOCH).unwrap();
    let (y, m, d, hh, mm, ss) = crate::television::civil_from_unix(secs.as_secs());
    let millis = secs.subsec_millis();
    let body = format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{millis:03}Z");
    Response::builder()
        .header(header::DATE, httpdate::fmt_http_date(now))
        .body(Body::from(body))
        .unwrap()
}

mod httpdate {
    use std::time::SystemTime;

    /// A minimal RFC 7231 `Date` header formatter (no chrono dependency).
    pub fn fmt_http_date(t: SystemTime) -> String {
        let secs = t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let (y, m, d, hh, mm, ss) = crate::television::civil_from_unix(secs);
        let days_since_epoch = (secs / 86400) as i64;
        // 1970-01-01 was a Thursday (weekday index 4 in a Mon=0 scheme).
        let weekday = ((days_since_epoch % 7) + 4).rem_euclid(7);
        const WD: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        const MO: [&str; 13] = [
            "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        format!(
            "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
            WD[weekday as usize], d, MO[m as usize], y, hh, mm, ss
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdnea_cookie_carries_both_akamai_and_legacy_names() {
        assert_eq!(hdnea_cookie("token"), "hdnea=token; __hdnea__=token");
    }

    fn fastway_app_state(
        store: Arc<crate::store::Store>,
        fastway: Arc<crate::fastway_state::FastwayState>,
    ) -> Arc<AppState> {
        let http = reqwest::Client::new();
        Arc::new(AppState {
            config: crate::config::Config::default(),
            path_prefix: String::new(),
            access: Arc::new(crate::access::Access::new(store.clone())),
            store,
            tv: Arc::new(television::Television::new(http.clone())),
            secure: Arc::new(crate::secureurl::SecureUrl::new(false)),
            http: http.clone(),
            drm_channels: Default::default(),
            custom_channels: Arc::new(crate::custom_channels::CustomChannels::new()),
            render_caches: Default::default(),
            dash_state: Default::default(),
            epg_state: Default::default(),
            extras: Arc::new(crate::extras_state::ExtrasState::new(false, None)),
            fastway,
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

    #[tokio::test]
    async fn fastway_license_proxy_sends_the_pallycon_token_and_challenge() {
        use crate::fastway::{Credentials, Device, Endpoints, FastwayChannel};
        use wiremock::matchers::{body_string_contains, header, method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(crate::store::Store::open(dir.path().to_str().unwrap()).unwrap());
        Credentials {
            username: "redacted-user".into(),
            password: "redacted-pass".into(),
            access_token: "redacted-token".into(),
        }
        .save(&store)
        .unwrap();

        let api = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/apiv2/pallycon/TokenIssueV3"))
            .and(query_param("contentId", "CID_7"))
            .and(header("auth", "auth=redacted-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "error_code": 1,
                "response_object": [{"base64Token": "pallycon-token"}]
            })))
            .mount(&api)
            .await;

        let expected_meta =
            crate::fastway::Client::new(reqwest::Client::new(), Device::new()).client_meta();
        let pallycon = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/ri/licenseManager.do"))
            .and(header(
                header::USER_AGENT.as_str(),
                crate::fastway::LICENSE_USER_AGENT,
            ))
            .and(header(
                header::CONTENT_TYPE.as_str(),
                "application/octet-stream",
            ))
            .and(header("pallycon-customdata-v2", "pallycon-token"))
            .and(header("pallycon-client-meta", expected_meta))
            .and(body_string_contains("widevine-challenge"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/octet-stream")
                    .set_body_bytes(b"license-response".to_vec()),
            )
            .mount(&pallycon)
            .await;

        let http = reqwest::Client::new();
        let fastway = Arc::new(crate::fastway_state::FastwayState::new(true));
        fastway.init(&http, &store);
        fastway.prime_for_test(vec![FastwayChannel {
            id: 7,
            channel_number: 554,
            title: "ANANDPUR LIVE".into(),
            stream_url: "http://cdn.example:8081/live/index.mpd".into(),
            encryption_url: "CID_7".into(),
            channel_mode: "live".into(),
            ..Default::default()
        }]);
        fastway.set_endpoints_for_test(Endpoints {
            api: api.uri(),
            license: format!("{}/ri/licenseManager.do", pallycon.uri()),
        });

        let state = fastway_app_state(store, fastway);
        let params = std::collections::HashMap::from([
            (
                "auth".to_string(),
                encrypt_dash_route_value(&state.secure, "fw_7", "http://example/license"),
            ),
            ("channel_id".to_string(), "fw_7".to_string()),
        ]);
        let response = drm_license_handler(
            State(state),
            Query(params),
            Method::POST,
            axum::http::HeaderMap::new(),
            Bytes::from_static(b"widevine-challenge"),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"license-response");
    }

    /// A spent Irdeto token (`130601`) must be retried with the license URL
    /// and bearer token of the *replacement* playback session, while the
    /// player's original challenge body is forwarded unchanged. The route's
    /// own license URL belongs to the older session and must not be reused.
    #[cfg(feature = "beta")]
    #[tokio::test]
    async fn tataplay_license_retries_a_spent_token_as_one_session_pair() {
        use crate::tataplay::{Client as TpClient, Credentials};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use wiremock::matchers::{body_string, header, method, path};
        use wiremock::{Mock, MockServer, Request, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/lit-ls-config/pub/api/v1/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"app": {"dd": {"v1": "ZG5INSxTMSVDbSN1cFclZQ==", "v2": "YWVzRW5jcnlwdGlvbktleQ=="}}}
            })))
            .mount(&server)
            .await;
        let plays = Arc::new(AtomicUsize::new(0));
        let plays_for_response = plays.clone();
        let base = server.uri();
        Mock::given(method("GET"))
            .and(path("/pas/api/v1/access/control/LIVE/1918"))
            .respond_with(move |_request: &Request| {
                let call = plays_for_response.fetch_add(1, Ordering::SeqCst);
                let (session, token) = if call == 0 {
                    ("sess-a", "token-a")
                } else {
                    ("sess-b", "token-b")
                };
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "data": {
                        "sessionId": session,
                        "heartbeatInterval": 3600,
                        "tokenInfo": {"token": token},
                        "playbackUrls": {
                            "dashWidewinePlayUrl": "not-encrypted",
                            "dashWidewineLicenseUrl": format!("{base}/license/{session}")
                        }
                    }
                }))
            })
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/license/sess-a"))
            .and(header("authorization", "Bearer token-a"))
            .and(body_string("widevine-challenge"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "code": 130601,
                "message": "The playback token was already used."
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/license/sess-b"))
            .and(header("authorization", "Bearer token-b"))
            .and(body_string("widevine-challenge"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"fresh-license".to_vec()))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/smgs/api/v1/session"))
            .and(header("x-session-id", "sess-a"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;

        let tataplay = Arc::new(crate::tataplay_state::TataplayState::new(true));
        tataplay.tune("tp_1918");
        tataplay.set_client_for_test(Arc::new(TpClient::for_test(
            &server.uri(),
            Credentials {
                access_token: "access".into(),
                refresh_token: "refresh".into(),
                subscriber_id: "sub-1".into(),
                sid: "sid-1".into(),
            },
        )));
        let state = license_test_state_with(
            Arc::new(crate::playbox_state::PlayboxState::new(false)),
            tataplay,
        );
        // Minted for a session that no longer exists: the handler must take
        // the URL from playback state, not from this route value.
        let params = std::collections::HashMap::from([
            (
                "auth".to_string(),
                encrypt_dash_route_value(&state.secure, "tp_1918", "http://stale.example/license"),
            ),
            ("channel_id".to_string(), "tp_1918".to_string()),
        ]);
        let response = drm_license_handler(
            State(state),
            Query(params),
            Method::POST,
            axum::http::HeaderMap::new(),
            Bytes::from_static(b"widevine-challenge"),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"fresh-license");
        assert_eq!(plays.load(Ordering::SeqCst), 2);
        server.verify().await;
    }

    /// A channel switch frees the previous slot asynchronously. A `130402`
    /// refusal is resent once with the same session/challenge, not a new
    /// session.
    #[cfg(feature = "beta")]
    #[tokio::test]
    async fn tataplay_license_resends_once_after_a_concurrency_refusal() {
        use crate::tataplay::{Client as TpClient, Credentials};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use wiremock::matchers::{body_string, header, method, path};
        use wiremock::{Mock, MockServer, Request, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/lit-ls-config/pub/api/v1/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"app": {"dd": {"v1": "ZG5INSxTMSVDbSN1cFclZQ==", "v2": "YWVzRW5jcnlwdGlvbktleQ=="}}}
            })))
            .mount(&server)
            .await;
        let base = server.uri();
        Mock::given(method("GET"))
            .and(path("/pas/api/v1/access/control/LIVE/1918"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "sessionId": "sess-a",
                    "heartbeatInterval": 3600,
                    "tokenInfo": {"token": "token-a"},
                    "playbackUrls": {
                        "dashWidewinePlayUrl": "not-encrypted",
                        "dashWidewineLicenseUrl": format!("{base}/license/sess-a")
                    }
                }
            })))
            .mount(&server)
            .await;
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_response = attempts.clone();
        Mock::given(method("POST"))
            .and(path("/license/sess-a"))
            .and(header("authorization", "Bearer token-a"))
            .and(body_string("widevine-challenge"))
            .respond_with(move |_request: &Request| {
                if attempts_for_response.fetch_add(1, Ordering::SeqCst) == 0 {
                    ResponseTemplate::new(403).set_body_json(serde_json::json!({
                        "code": 130402,
                        "message": "Too many concurrent streams for device type."
                    }))
                } else {
                    ResponseTemplate::new(200).set_body_bytes(b"license-after-slot".to_vec())
                }
            })
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;

        let tataplay = Arc::new(crate::tataplay_state::TataplayState::new(true));
        tataplay.tune("tp_1918");
        tataplay.set_client_for_test(Arc::new(TpClient::for_test(
            &server.uri(),
            Credentials {
                access_token: "access".into(),
                refresh_token: "refresh".into(),
                subscriber_id: "sub-1".into(),
                sid: "sid-1".into(),
            },
        )));
        tataplay.mark_closed_for_test();
        let state = license_test_state_with(
            Arc::new(crate::playbox_state::PlayboxState::new(false)),
            tataplay,
        );
        let params = std::collections::HashMap::from([
            (
                "auth".to_string(),
                encrypt_dash_route_value(&state.secure, "tp_1918", "http://stale.example/license"),
            ),
            ("channel_id".to_string(), "tp_1918".to_string()),
        ]);
        let response = drm_license_handler(
            State(state),
            Query(params),
            Method::POST,
            axum::http::HeaderMap::new(),
            Bytes::from_static(b"widevine-challenge"),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"license-after-slot");
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        server.verify().await;
    }

    #[cfg(feature = "beta")]
    #[test]
    fn merged_widevine_pssh_lists_every_key_id() {
        let first = [0x11u8; 16];
        let second = [0x22u8; 16];
        let encoded = merged_widevine_pssh(&[first, second]);
        let raw = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .unwrap();
        assert_eq!(&raw[4..8], b"pssh");
        assert_eq!(&raw[12..28], &crate::dash::WIDEVINE_SYSTEM_ID);
        let mut kids = Vec::new();
        pssh_key_ids(&raw, &mut kids);
        assert_eq!(kids, vec![first, second]);
    }

    #[cfg(feature = "beta")]
    #[test]
    fn injection_replaces_only_the_widevine_content_protection() {
        let body = concat!(
            "<MPD xmlns=\"urn:mpeg:dash:schema:mpd:2011\"><Period>",
            "<AdaptationSet contentType=\"video\">",
            "<ContentProtection value=\"cenc\" schemeIdUri=\"urn:mpeg:dash:mp4protection:2011\"/>",
            "<ContentProtection schemeIdUri=\"urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed\" value=\"Widevine\"/>",
            "</AdaptationSet></Period></MPD>"
        );
        let out = inject_widevine_pssh(body, "AAAA");
        assert!(out.contains("xmlns:cenc=\"urn:mpeg:cenc:2013\""));
        assert_eq!(out.matches("<cenc:pssh>AAAA</cenc:pssh>").count(), 1);
        assert_eq!(out.matches("mp4protection").count(), 1);
        assert!(out.contains("</ContentProtection></AdaptationSet>"));
    }

    #[cfg(feature = "beta")]
    #[test]
    fn init_urls_resolve_audio_and_video_templates_only() {
        let body = concat!(
            "<MPD><Period><BaseURL>dash/</BaseURL>",
            "<AdaptationSet contentType=\"audio\">",
            "<SegmentTemplate initialization=\"$RepresentationID$.dash\" media=\"$Number$.m4s\"/>",
            "<Representation id=\"audio_1\"/><Representation id=\"audio_2\"/>",
            "</AdaptationSet>",
            "<AdaptationSet contentType=\"text\">",
            "<SegmentTemplate initialization=\"$RepresentationID$.dash\"/>",
            "<Representation id=\"text_1\"/>",
            "</AdaptationSet>",
            "<AdaptationSet contentType=\"video\">",
            "<SegmentTemplate initialization=\"$RepresentationID$.dash\" media=\"$Number$.m4s\"/>",
            "<Representation id=\"video_1\"/>",
            "</AdaptationSet></Period></MPD>"
        );
        let urls = tataplay_init_urls(
            "https://cdn.example/live/channel/movies.mpd?hdnea=token",
            body,
        );
        assert_eq!(
            urls,
            vec![
                "https://cdn.example/live/channel/dash/audio_1.dash?hdnea=token",
                "https://cdn.example/live/channel/dash/audio_2.dash?hdnea=token",
                "https://cdn.example/live/channel/dash/video_1.dash?hdnea=token",
            ]
        );
    }

    #[cfg(feature = "beta")]
    #[test]
    fn seig_sample_group_key_is_collected_from_a_media_segment() {
        let box_bytes = hex::decode(
            "0000002c736770640100000073656967000000140000000100000108b4e62b2f4a0f55a69d73dadd4dde2b58",
        )
        .unwrap();
        let mut kids = Vec::new();
        assert!(seig_key_ids(&box_bytes, &mut kids));
        assert_eq!(
            kids,
            vec![[
                0xb4, 0xe6, 0x2b, 0x2f, 0x4a, 0x0f, 0x55, 0xa6, 0x9d, 0x73, 0xda, 0xdd, 0x4d, 0xde,
                0x2b, 0x58,
            ]]
        );
        // A duplicated KID still proves the media probe carried a seig entry.
        assert!(seig_key_ids(&box_bytes, &mut kids));
        assert_eq!(kids.len(), 1);
    }

    #[cfg(feature = "beta")]
    #[test]
    fn version_one_pssh_reads_the_header_kid_array() {
        let kid = [
            0x58u8, 0xd6, 0xc1, 0x34, 0xfe, 0x35, 0x59, 0x80, 0x89, 0xd6, 0x91, 0x96, 0x0a, 0x18,
            0x84, 0xbb,
        ];
        let size = 32 + 16 + 4;
        let mut box_bytes = Vec::with_capacity(size);
        box_bytes.extend_from_slice(&(size as u32).to_be_bytes());
        box_bytes.extend_from_slice(b"pssh");
        box_bytes.extend_from_slice(&[1, 0, 0, 0]);
        box_bytes.extend_from_slice(&WIDEVINE_SYSTEM_ID);
        box_bytes.extend_from_slice(&1u32.to_be_bytes());
        box_bytes.extend_from_slice(&kid);
        box_bytes.extend_from_slice(&0u32.to_be_bytes());
        let mut kids = Vec::new();
        pssh_key_ids(&box_bytes, &mut kids);
        assert_eq!(kids, vec![kid]);
    }

    #[cfg(feature = "beta")]
    #[test]
    fn media_probe_prefers_a_small_audio_segment() {
        let body = concat!(
            "<MPD><Period><BaseURL>dash/</BaseURL>",
            "<AdaptationSet contentType=\"audio\">",
            "<SegmentTemplate startNumber=\"100\" media=\"$RepresentationID$-$Number$.m4s\">",
            "<SegmentTimeline><S t=\"0\" d=\"8\" r=\"4\"/></SegmentTimeline></SegmentTemplate>",
            "<Representation id=\"audio_1\" bandwidth=\"80800\"/>",
            "</AdaptationSet>",
            "<AdaptationSet contentType=\"video\">",
            "<SegmentTemplate startNumber=\"100\" media=\"$RepresentationID$-$Number$.m4s\">",
            "<SegmentTimeline><S t=\"0\" d=\"8\" r=\"4\"/></SegmentTimeline></SegmentTemplate>",
            "<Representation id=\"video_1\" bandwidth=\"4130000\"/>",
            "</AdaptationSet></Period></MPD>"
        );
        assert_eq!(
            tataplay_media_probe_urls(
                "https://cdn.example/live/channel/movies.mpd?hdnea=token",
                body
            ),
            vec![
                "https://cdn.example/live/channel/dash/audio_1-104.m4s?hdnea=token",
                "https://cdn.example/live/channel/dash/video_1-104.m4s?hdnea=token",
            ]
        );
    }

    #[cfg(feature = "beta")]
    #[test]
    fn tataplay_playback_id_is_bound_into_the_route_urls() {
        let state = license_test_state_with(
            Arc::new(crate::playbox_state::PlayboxState::new(false)),
            Arc::new(crate::tataplay_state::TataplayState::new(false)),
        );
        let mut live = LiveUrlOutput {
            result: "OK".into(),
            is_drm: true,
            playback_id: "pid-1".into(),
            ..Default::default()
        };
        live.mpd.auto = "https://cdn.example/live/ch.mpd".into();
        live.key_url = "https://lic.example/license".into();

        let out = build_drm_mpd_output(&state, &live, "tp_1", "auto").unwrap();
        for url in [&out.play_url, &out.license_url] {
            let pid = url
                .split("pid=")
                .nth(1)
                .unwrap_or_else(|| panic!("missing pid in {url}"))
                .split('&')
                .next()
                .unwrap();
            assert_eq!(
                decrypt_dash_route_value(&state.secure, "tp_1", pid).as_deref(),
                Some("pid-1"),
                "pid must decrypt for the channel in {url}"
            );
        }
    }

    /// A licence request from a previous tune of the same channel must not
    /// reach the upstream or borrow the current session.
    #[cfg(feature = "beta")]
    #[tokio::test]
    async fn tataplay_license_rejects_a_stale_playback_id() {
        use crate::tataplay::{Client as TpClient, Credentials};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/lit-ls-config/pub/api/v1/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"app": {"dd": {"v1": "ZG5INSxTMSVDbSN1cFclZQ==", "v2": "YWVzRW5jcnlwdGlvbktleQ=="}}}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/pas/api/v1/access/control/LIVE/1918"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "sessionId": "sess-a",
                    "heartbeatInterval": 3600,
                    "tokenInfo": {"token": "token-a"},
                    "playbackUrls": {
                        "dashWidewinePlayUrl": "not-encrypted",
                        "dashWidewineLicenseUrl": "not-encrypted"
                    }
                }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/license"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"must-not-be-called".to_vec()))
            .expect(0)
            .mount(&server)
            .await;

        let tataplay = Arc::new(crate::tataplay_state::TataplayState::new(true));
        tataplay.set_client_for_test(Arc::new(TpClient::for_test(
            &server.uri(),
            Credentials {
                access_token: "access".into(),
                refresh_token: "refresh".into(),
                subscriber_id: "sub-1".into(),
                sid: "sid-1".into(),
            },
        )));
        let stale = tataplay.tune("tp_1918").unwrap();
        tataplay.live("tp_1918").await.unwrap();
        tataplay.tune("tp_1918");
        tataplay.live("tp_1918").await.unwrap();

        let state = license_test_state_with(
            Arc::new(crate::playbox_state::PlayboxState::new(false)),
            tataplay,
        );
        let params = std::collections::HashMap::from([
            (
                "auth".to_string(),
                encrypt_dash_route_value(&state.secure, "tp_1918", "http://license.test/x"),
            ),
            ("channel_id".to_string(), "tp_1918".to_string()),
            (
                "pid".to_string(),
                encrypt_dash_route_value(&state.secure, "tp_1918", &stale),
            ),
        ]);
        let response = drm_license_handler(
            State(state),
            Query(params),
            Method::POST,
            axum::http::HeaderMap::new(),
            Bytes::from_static(b"widevine-challenge"),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CONFLICT);
        server.verify().await;
    }

    /// A `pid` that is present but cannot be decrypted belongs to an older
    /// account context. Treating it as absent would pair an old challenge with
    /// the current session's token.
    #[cfg(feature = "beta")]
    #[tokio::test]
    async fn tataplay_license_rejects_an_undecryptable_playback_id() {
        let state = license_test_state(Arc::new(crate::playbox_state::PlayboxState::new(false)));
        let params = std::collections::HashMap::from([
            (
                "auth".to_string(),
                encrypt_dash_route_value(&state.secure, "tp_1918", "http://license.test/x"),
            ),
            ("channel_id".to_string(), "tp_1918".to_string()),
            ("pid".to_string(), "not-a-route-value".to_string()),
        ]);
        let response = drm_license_handler(
            State(state),
            Query(params),
            Method::POST,
            axum::http::HeaderMap::new(),
            Bytes::from_static(b"widevine-challenge"),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(&body[..], b"stale playback");
    }

    /// The manifest route applies the same rule as the license route.
    #[cfg(feature = "beta")]
    #[tokio::test]
    async fn tataplay_manifest_rejects_an_undecryptable_playback_id() {
        let state = license_test_state(Arc::new(crate::playbox_state::PlayboxState::new(false)));
        let q = RenderMpdQuery {
            auth: encrypt_dash_route_value(&state.secure, "tp_1918", "http://origin.test/x.mpd"),
            channel_id: Some("tp_1918".to_string()),
            q: None,
            hdnea: None,
            pid: Some("not-a-route-value".to_string()),
        };
        let response = render_mpd_inner(state, q).await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    fn drm_output() -> DrmMpdOutput {
        DrmMpdOutput {
            play_url: "/render.mpd?x".into(),
            ..Default::default()
        }
    }

    #[cfg(feature = "beta")]
    fn license_test_state(playbox: Arc<crate::playbox_state::PlayboxState>) -> Arc<AppState> {
        license_test_state_with(
            playbox,
            Arc::new(crate::tataplay_state::TataplayState::new(false)),
        )
    }

    #[cfg(feature = "beta")]
    fn license_test_state_with(
        playbox: Arc<crate::playbox_state::PlayboxState>,
        tataplay: Arc<crate::tataplay_state::TataplayState>,
    ) -> Arc<AppState> {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(crate::store::Store::open(dir.path().to_str().unwrap()).unwrap());
        std::mem::forget(dir);
        Arc::new(AppState {
            config: crate::config::Config::default(),
            path_prefix: String::new(),
            access: Arc::new(crate::access::Access::new(store.clone())),
            store,
            tv: Arc::new(television::Television::new(reqwest::Client::new())),
            secure: Arc::new(crate::secureurl::SecureUrl::new(false)),
            http: reqwest::Client::new(),
            drm_channels: Default::default(),
            custom_channels: Arc::new(crate::custom_channels::CustomChannels::new()),
            render_caches: Default::default(),
            dash_state: Default::default(),
            epg_state: Default::default(),
            extras: Arc::new(crate::extras_state::ExtrasState::new(false, None)),
            fastway: Arc::new(crate::fastway_state::FastwayState::new(false)),
            playbox,
            tataplay,
            vzy: Arc::new(crate::vzy_state::VzyState::new(false)),
            airtel: Arc::new(crate::airtel_state::AirtelState::new(false)),
            vod_state: Default::default(),
            public_ip: Arc::new(crate::unlock::PublicIp::new(reqwest::Client::new())),
            unlock_limiter: Arc::new(crate::unlock::AttemptLimiter::default()),
            listen: Default::default(),
            tunnel_origin: Default::default(),
        })
    }

    #[cfg(feature = "beta")]
    #[tokio::test]
    async fn playbox_license_forwards_the_resolved_token_headers() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/consume/v1/playback/resolve"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": 200,
                "data": {"url": {
                    "playable": "https://cdn.example/ch1.mpd",
                    "licenceUrl": format!("{}/license", server.uri()),
                    "token": "drm-token"
                }}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/license"))
            .and(header("X-AxDRM-Message", "drm-token"))
            .and(header("Authorization", "Bearer drm-token"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"license".to_vec()))
            .mount(&server)
            .await;

        let playbox = Arc::new(crate::playbox_state::PlayboxState::new(true));
        let mut client =
            crate::playbox::Client::new(reqwest::Client::new(), crate::playbox::Device::new());
        client.set_bases_for_test(&server.uri());
        *client.creds.write().unwrap() = crate::playbox::Credentials {
            phone: "9876543210".into(),
            token: "opaque-session-token".into(),
            customer_id: "cust-1".into(),
        };
        playbox.set_client_for_test(Arc::new(client));

        let state = license_test_state(playbox);
        let license_url = format!("{}/license", server.uri());
        let params = std::collections::HashMap::from([
            (
                "auth".to_string(),
                encrypt_dash_route_value(&state.secure, "pb_ch1", &license_url),
            ),
            ("channel_id".to_string(), "pb_ch1".to_string()),
        ]);
        let resp = drm_license_handler(
            State(state),
            Query(params),
            Method::POST,
            axum::http::HeaderMap::new(),
            Bytes::from_static(b"challenge"),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[cfg(feature = "beta")]
    #[tokio::test]
    async fn playbox_license_rejects_auth_bound_to_a_different_channel() {
        let playbox = Arc::new(crate::playbox_state::PlayboxState::new(true));
        let state = license_test_state(playbox);
        let params = std::collections::HashMap::from([
            (
                "auth".to_string(),
                encrypt_dash_route_value(
                    &state.secure,
                    "pb_source",
                    "https://example.invalid/license",
                ),
            ),
            ("channel_id".to_string(), "pb_target".to_string()),
        ]);

        let response = drm_license_handler(
            State(state),
            Query(params),
            Method::POST,
            axum::http::HeaderMap::new(),
            Bytes::from_static(b"challenge"),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[cfg(feature = "beta")]
    #[tokio::test]
    async fn playbox_mpd_rejection_invalidates_cached_playback() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, Request, ResponseTemplate};

        let server = MockServer::start().await;
        let media_url = format!("{}/stale.mpd", server.uri());
        Mock::given(method("GET"))
            .and(path("/catalogue/v3/page/livetv2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "statusCode": 200,
                "livetv": [{"id": "ch1", "name": "One", "typeId": 106}]
            })))
            .expect(1)
            .mount(&server)
            .await;
        let resolve_calls = Arc::new(AtomicUsize::new(0));
        let resolve_calls_for_response = resolve_calls.clone();
        let media_for_response = media_url.clone();
        Mock::given(method("POST"))
            .and(path("/consume/v1/playback/resolve"))
            .respond_with(move |_request: &Request| {
                resolve_calls_for_response.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "code": 200,
                    "data": {"url": {"playable": media_for_response, "token": "media-token"}}
                }))
            })
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/stale.mpd"))
            .respond_with(ResponseTemplate::new(403))
            .expect(2)
            .mount(&server)
            .await;

        let playbox = Arc::new(crate::playbox_state::PlayboxState::new(true));
        let mut client =
            crate::playbox::Client::new(reqwest::Client::new(), crate::playbox::Device::new());
        client.set_bases_for_test(&server.uri());
        *client.creds.write().unwrap() = crate::playbox::Credentials {
            phone: "9876543210".into(),
            token: "opaque-session-token".into(),
            customer_id: "cust-1".into(),
        };
        playbox.set_client_for_test(Arc::new(client));
        let state = license_test_state(playbox);

        let first = state.playbox.live("pb_ch1").await.unwrap();
        assert_eq!(first.mpd.auto, media_url);
        assert!(state.playbox.playlist_ready("pb_ch1"));

        let response = render_mpd_inner(
            state.clone(),
            RenderMpdQuery {
                auth: encrypt_dash_route_value(&state.secure, "pb_ch1", &media_url),
                channel_id: Some("pb_ch1".into()),
                q: Some("auto".into()),
                hdnea: None,
                pid: None,
            },
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(!state.playbox.playlist_ready("pb_ch1"));

        let _ = state.playbox.live("pb_ch1").await.unwrap();
        assert_eq!(resolve_calls.load(Ordering::SeqCst), 2);
    }

    #[cfg(feature = "beta")]
    #[tokio::test]
    async fn playbox_dash_segment_rejection_reresolves_before_retry() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, Request, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/catalogue/v3/page/livetv2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "statusCode": 200,
                "livetv": [{"id": "ch1", "name": "One", "typeId": 106}]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let resolve_calls = Arc::new(AtomicUsize::new(0));
        let resolve_calls_for_response = resolve_calls.clone();
        Mock::given(method("POST"))
            .and(path("/consume/v1/playback/resolve"))
            .respond_with(move |_request: &Request| {
                let first = resolve_calls_for_response.fetch_add(1, Ordering::SeqCst) == 0;
                let user_id = if first { "stale-user" } else { "fresh-user" };
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "code": 200,
                    "data": {"url": {
                        "playable": "https://cdn.example/ch1.mpd",
                        "token": "media-token",
                        "headers": {"user_id": user_id}
                    }}
                }))
            })
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/dash/seg.m4s"))
            .and(header("user_id", "stale-user"))
            .respond_with(ResponseTemplate::new(403))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/dash/seg.m4s"))
            .and(header("user_id", "fresh-user"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"segment".to_vec()))
            .expect(1)
            .mount(&server)
            .await;

        let playbox = Arc::new(crate::playbox_state::PlayboxState::new(true));
        let mut client =
            crate::playbox::Client::new(reqwest::Client::new(), crate::playbox::Device::new());
        client.set_bases_for_test(&server.uri());
        *client.creds.write().unwrap() = crate::playbox::Credentials {
            phone: "9876543210".into(),
            token: "opaque-session-token".into(),
            customer_id: "cust-1".into(),
        };
        playbox.set_client_for_test(Arc::new(client));
        let state = license_test_state(playbox);
        let _ = state.playbox.live("pb_ch1").await.unwrap();

        let enc_host = encrypt_dash_route_value(&state.secure, "pb_ch1", &server.uri());
        let enc_path = encrypt_dash_route_value(&state.secure, "pb_ch1", "/");
        let uri: axum::http::Uri =
            format!("/render.dash/channel/pb_ch1/host/{enc_host}/path/{enc_path}/dash/seg.m4s")
                .parse()
                .unwrap();
        let response = render_dash_handler(State(state), uri).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"segment");
        assert_eq!(resolve_calls.load(Ordering::SeqCst), 2);
    }

    #[cfg(feature = "beta")]
    #[tokio::test]
    async fn dash_route_rejects_destination_bound_to_a_different_channel() {
        let playbox = Arc::new(crate::playbox_state::PlayboxState::new(true));
        let state = license_test_state(playbox);
        let enc_host =
            encrypt_dash_route_value(&state.secure, "pb_source", "https://example.invalid");
        let enc_path = encrypt_dash_route_value(&state.secure, "pb_source", "/media/");
        let uri: axum::http::Uri =
            format!("/render.dash/channel/pb_target/host/{enc_host}/path/{enc_path}/seg.m4s")
                .parse()
                .unwrap();

        let response = render_dash_handler(State(state), uri).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn drm_mpd_cache_is_bound_to_its_context_epoch() {
        let state = DashState::default();
        assert!(state.set_cached("154_auto", drm_output(), 1, || 1));
        assert!(state.get_cached("154_auto", 1).is_some());
        // Epoch rotated but `clear` has not run yet: a hit would be the
        // previous account's URLs.
        assert!(state.get_cached("154_auto", 2).is_none());
    }

    #[test]
    fn drm_mpd_cache_discards_builds_that_outlived_their_context() {
        let state = DashState::default();
        assert!(!state.set_cached("154_auto", drm_output(), 1, || 2));
        assert!(state.get_cached("154_auto", 1).is_none());
        assert!(state.get_cached("154_auto", 2).is_none());
    }

    #[test]
    fn live_cache_serves_recent_entries_and_clears_with_the_context() {
        let state = DashState::default();
        assert!(state.get_live("154", 1).is_none());
        assert!(state.set_live(
            "154",
            LiveUrlOutput {
                hdnea: "token".into(),
                ..Default::default()
            },
            1,
            || 1,
        ));
        assert_eq!(state.get_live("154", 1).unwrap().hdnea, "token");
        assert!(state.get_live("155", 1).is_none());
        state.clear();
        assert!(state.get_live("154", 1).is_none());
    }

    #[test]
    fn live_cache_discards_fetches_that_outlived_their_context() {
        let state = DashState::default();
        assert!(!state.set_live("154", LiveUrlOutput::default(), 1, || 2));
        assert!(state.get_live("154", 1).is_none());
        assert!(state.get_live("154", 2).is_none());
    }

    #[test]
    fn live_cache_hit_in_a_newer_epoch_is_a_miss() {
        let state = DashState::default();
        assert!(state.set_live("154", LiveUrlOutput::default(), 1, || 1));
        // The epoch rotated but `clear` has not run yet.
        assert!(state.get_live("154", 2).is_none());
    }

    #[tokio::test]
    async fn concurrent_live_cache_misses_share_one_fetch() {
        let state = Arc::new(DashState::default());
        let fetches = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let viewer = |state: Arc<DashState>, fetches: Arc<std::sync::atomic::AtomicU32>| async move {
            if state.get_live("154", 1).is_some() {
                return;
            }
            let _guard = state.live_locks.lock("154").await;
            if state.get_live("154", 1).is_some() {
                return;
            }
            fetches.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(30)).await;
            state.set_live("154", LiveUrlOutput::default(), 1, || 1);
        };
        let tasks: Vec<_> = (0..5)
            .map(|_| tokio::spawn(viewer(state.clone(), fetches.clone())))
            .collect();
        for t in tasks {
            t.await.unwrap();
        }
        assert_eq!(fetches.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn parses_rfc3339_publish_time() {
        let t = parse_rfc3339("2024-01-02T03:04:05.678Z").unwrap();
        let secs = t.duration_since(UNIX_EPOCH).unwrap().as_secs();
        // 2024-01-02T03:04:05Z
        assert_eq!(secs, 1704164645);
    }

    #[test]
    fn extracts_publish_time_attr() {
        let body = r#"<MPD publishTime="2024-01-02T03:04:05.000Z" other="x">"#;
        assert!(extract_publish_time(body).is_some());
    }

    #[test]
    fn rewrites_existing_base_url() {
        let body = "<Period><BaseURL>https://old/</BaseURL></Period>";
        let out = rewrite_base_url(body, "/render.dash/channel/ex_1/host/x/path/y");
        assert_eq!(
            out,
            "<Period><BaseURL>/render.dash/channel/ex_1/host/x/path/y/dash/</BaseURL></Period>"
        );
    }

    #[test]
    fn inserts_base_url_when_missing() {
        let body = "<Period id=\"0\"><AdaptationSet/></Period>";
        let out = rewrite_base_url(body, "/render.dash/channel/ex_1/host/x/path/y");
        assert!(out.contains(
            "<Period id=\"0\">\n<BaseURL>/render.dash/channel/ex_1/host/x/path/y/</BaseURL>"
        ));
        assert!(!out.contains("/dash/"));
    }

    #[test]
    fn rewrites_every_period_and_base_url() {
        let two = "<Period id=\"a\"><AdaptationSet/></Period><Period id=\"b\"/>";
        assert_eq!(
            rewrite_base_url(two, "/r")
                .matches("<BaseURL>/r/</BaseURL>")
                .count(),
            2
        );
        let bases = "<BaseURL>a/</BaseURL><Period><BaseURL>b/</BaseURL></Period>";
        assert_eq!(
            rewrite_base_url(bases, "/r")
                .matches("<BaseURL>/r/dash/</BaseURL>")
                .count(),
            2
        );
    }

    /// Regression test for a live-verified bug: channel 151's extras-mirrored
    /// MPD carries an `hdnea` token whose value contains a literal `/`
    /// (Akamai-style ACLs look like `exp=...~acl=/*~data=...~hmac=...`).
    /// The directory must come out exactly as many segments deep as the
    /// path alone implies, regardless of what's in the query.
    #[test]
    fn host_and_dir_ignore_slashes_inside_the_query() {
        let url = "https://cdn.example.com/bpk-tv/MoviesNow_BTS/WDVLive/index.mpd\
                   ?hdnea=exp=1830000000~acl=/*~data=hdntl~hmac=deadbeef";
        let (_, host, dir) = cdn_host_and_dir(url).unwrap();
        assert_eq!(host, "cdn.example.com");
        assert_eq!(dir, "/bpk-tv/MoviesNow_BTS/WDVLive/");
        assert_eq!(
            dir.matches('/').count(),
            4,
            "expected exactly 4 slashes: leading + 3 directories"
        );
    }

    #[test]
    fn host_and_dir_are_not_thrown_off_by_a_trailing_slash() {
        // A defensive case beyond what's been seen live: a manifest URL
        // ending in `/` (naming a directory, not a file) must not leave the
        // real last directory component in the "filename" slot.
        let (_, _, dir) = cdn_host_and_dir("https://cdn.example.com/a/b/c/index.mpd/").unwrap();
        assert_eq!(dir, "/a/b/c/");
    }

    #[test]
    fn host_and_dir_plain_case_matches_go() {
        let (_, host, dir) =
            cdn_host_and_dir("https://cdn.example.com/bpk-tv/Name_BTS/WDVLive/index.mpd").unwrap();
        assert_eq!(host, "cdn.example.com");
        assert_eq!(dir, "/bpk-tv/Name_BTS/WDVLive/");
    }

    #[test]
    fn host_and_dir_preserve_the_upstream_scheme_and_port() {
        // Fastway's demo CDN is plain HTTP on a non-default port; dropping
        // either piece would make every proxied segment unreachable.
        let (origin, host, dir) =
            cdn_host_and_dir("http://democdn.weedge.in:8081/live/index.mpd").unwrap();
        assert_eq!(origin, "http://democdn.weedge.in:8081");
        assert_eq!(host, "democdn.weedge.in");
        assert_eq!(dir, "/live/");
    }

    /// End to end (as pure functions, no network): a relative
    /// `SegmentTemplate` reference that carries its own `?m=` query, as
    /// resolved by a real client against a BaseURL this server derived from
    /// an upstream MPD URL whose own query contains `/` (Akamai-style ACLs:
    /// `...~acl=/*~...`) — the exact scenario behind the channel 151
    /// "Rust's BaseURL has one more path segment than Go's" bug. Checks the
    /// full round trip: `cdn_host_and_dir` (manifest URL -> host + dir) then
    /// `split_dash_path` + `build_dash_proxy_url` (rewritten request path ->
    /// upstream URL) must land back on exactly the original directory, with
    /// the segment's query preserved and nothing extra.
    #[test]
    fn dash_round_trip_survives_a_query_with_slashes_and_a_segment_with_its_own_query() {
        let secure = crate::secureurl::SecureUrl::new(false);
        let upstream = "https://cdn.example.com/bpk-tv/MoviesNow_BTS/WDVLive/index.mpd\
                         ?hdnea=exp=1830000000~acl=/*~hmac=deadbeef";

        let (origin, _host, dir_path) = cdn_host_and_dir(upstream).unwrap();
        assert_eq!(origin, "https://cdn.example.com");
        assert_eq!(dir_path, "/bpk-tv/MoviesNow_BTS/WDVLive/");
        let enc_host = encrypt_dash_route_value(&secure, "ex_151", &origin);
        let enc_path = encrypt_dash_route_value(&secure, "ex_151", &dir_path);

        // Shaka resolves the relative SegmentTemplate reference
        // "index_video_7_0_init.mp4?m=1773052885" against our rewritten
        // BaseURL ("/render.dash/channel/<id>/host/<enc>/path/<enc>/dash/") into this
        // request.
        let request_path = format!("/render.dash/channel/ex_151/host/{enc_host}/path/{enc_path}/dash/index_video_7_0_init.mp4");
        let (channel_id, got_enc_host, got_enc_path, got_hdnea, segment_path) =
            split_dash_path(&request_path).unwrap();
        assert_eq!(channel_id, "ex_151");
        assert_eq!(got_enc_host, enc_host);
        assert_eq!(got_enc_path, enc_path);
        assert!(got_hdnea.is_none());
        assert_eq!(segment_path, "/dash/index_video_7_0_init.mp4");

        let decrypted_host = decrypt_dash_route_value(&secure, "ex_151", got_enc_host).unwrap();
        let decrypted_dir = decrypt_dash_route_value(&secure, "ex_151", got_enc_path).unwrap();
        assert_eq!(decrypted_host, "https://cdn.example.com");
        assert_eq!(decrypted_dir, "/bpk-tv/MoviesNow_BTS/WDVLive/");

        let proxy_url = build_dash_proxy_url(
            &decrypted_host,
            &decrypted_dir,
            &segment_path,
            "m=1773052885",
        );
        assert_eq!(
            proxy_url,
            "https://cdn.example.com/bpk-tv/MoviesNow_BTS/WDVLive/dash/index_video_7_0_init.mp4?m=1773052885"
        );
    }

    #[test]
    fn split_dash_path_extracts_the_hdnea_segment_when_present() {
        let (channel, host, path, hdnea, seg) =
            split_dash_path("/render.dash/channel/ex_42/host/H/path/P/hdnea/HD/seg.m4s").unwrap();
        assert_eq!(channel, "ex_42");
        assert_eq!(
            (host, path, hdnea, seg.as_str()),
            ("H", "P", Some("HD"), "/seg.m4s")
        );
    }

    #[test]
    fn split_dash_path_rejects_a_malformed_prefix() {
        assert!(split_dash_path("/not-render-dash/x").is_none());
        assert!(split_dash_path("/render.dash/host/H/path/P/seg.m4s").is_none());
    }
}
