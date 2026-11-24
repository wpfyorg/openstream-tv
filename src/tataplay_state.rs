//! Tata Play routing/caching layer: catalogue, decryption keys, live playback
//! and the headers the media/license proxy must forward.
//!
//! Protocol details live in `src/tataplay.rs`. Everything here is local state:
//! TTLs, per-account generation guards and cache invalidation.

use crate::keyed_locks::KeyedLocks;
use crate::tataplay::{
    document_id, AppKeys, Client, GuideChannel, GuideProgramme, ResolvedPlayback,
};
use crate::television::{Channel, LiveUrlOutput, Mpd};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

const CATALOGUE_TTL: Duration = Duration::from_secs(30 * 60);
const CATALOGUE_RETRY_BACKOFF: Duration = Duration::from_secs(15);
const KEYS_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const PLAYBACK_TTL: Duration = Duration::from_secs(5 * 60);
const EPG_TTL: Duration = Duration::from_secs(15 * 60);
/// Stop beating once no media or license request has touched the playback for
/// this long. Matches the playback TTL so an idle session is never resumed from
/// a cache entry that has already expired.
#[cfg(not(test))]
const HEARTBEAT_IDLE_TIMEOUT: Duration = PLAYBACK_TTL;
#[cfg(test)]
const HEARTBEAT_IDLE_TIMEOUT: Duration = Duration::from_millis(150);
/// Fallback cadence when the access-control response omits `heartbeatInterval`.
/// A 30 s beat stays well inside the 5 minute playback TTL.
const DEFAULT_HEARTBEAT_SECS: u64 = 30;

/// A running concurrency session: the upstream frees the slot when the beat
/// stops, so playback needs one POST at start and a `DELETE` at stop.
struct Heartbeat {
    document: String,
    session_id: String,
    client: Arc<Client>,
    task: tokio::task::JoinHandle<()>,
}

/// One license endpoint and the credentials that authorise it. Both values
/// always come from the same `ResolvedPlayback`: the URL is signed for the
/// playback session the token belongs to, so mixing a URL from one resolution
/// with a token from another is refused upstream (`130001`/`130601`).
#[derive(Clone, Debug)]
pub struct LicenseSession {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub session_id: String,
}

impl LicenseSession {
    fn from_playback(resolved: &ResolvedPlayback) -> Option<LicenseSession> {
        if resolved.license_url.is_empty() {
            return None;
        }
        let mut headers = resolved.license_headers.clone();
        // Irdeto denies the Widevine challenge as `[__anonymous]` without the
        // playback token from `tokenInfo`.
        headers.retain(|(name, _)| !name.eq_ignore_ascii_case("authorization"));
        if !resolved.license_token.is_empty() {
            headers.push((
                "Authorization".to_string(),
                format!("Bearer {}", resolved.license_token),
            ));
        }
        Some(LicenseSession {
            url: resolved.license_url.clone(),
            headers,
            session_id: resolved.session_id.clone(),
        })
    }
}

#[derive(Clone, Debug)]
struct PlaybackIntent {
    document: String,
    playback_id: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlaybackMode {
    /// A user selection; may close the previous channel's session.
    Tune,
    /// A manifest/licence/media refresh; may only touch the current tune.
    Refresh,
}

pub struct TataplayState {
    config_enabled: bool,
    client: RwLock<Option<Arc<Client>>>,
    catalogue: RwLock<Vec<Channel>>,
    catalogue_fetched_at: RwLock<Option<Instant>>,
    catalogue_failed_at: RwLock<Option<Instant>>,
    keys: RwLock<Option<(AppKeys, Instant)>>,
    resolved: RwLock<HashMap<String, (ResolvedPlayback, Instant)>>,
    /// The channel the user last asked to play, and the id of that tune.
    /// Refreshes may only reuse or re-resolve this channel; a late manifest
    /// for a channel that has been left must not take the slot back.
    intent: Mutex<Option<PlaybackIntent>>,
    /// Serializes deliberate tunes and session handoffs for the single account.
    tune: tokio::sync::Mutex<()>,
    /// Guide days keyed by IST midnight. Shared by XMLTV generation and the
    /// per-channel web guide so one day is fetched once.
    epg_cache: RwLock<HashMap<i64, (Vec<GuideChannel>, Instant)>>,
    /// Last media/license access, so the heartbeat can stop when playback ends.
    last_activity: Arc<Mutex<Instant>>,
    /// When a session close was attempted, so a 130402 retry can be limited to
    /// a local channel transition instead of another device's stream.
    last_close: Mutex<Option<Instant>>,
    heartbeat: Mutex<Option<Heartbeat>>,
    /// Bumped under `commit` whenever account-scoped caches are cleared. A
    /// fetch captures it first and commits only if it is unchanged.
    generation: AtomicU64,
    commit: Mutex<()>,
    account_transition: tokio::sync::Mutex<()>,
    locks: KeyedLocks,
}

impl TataplayState {
    pub fn new(config_enabled: bool) -> TataplayState {
        TataplayState {
            config_enabled,
            client: RwLock::new(None),
            catalogue: RwLock::new(Vec::new()),
            catalogue_fetched_at: RwLock::new(None),
            catalogue_failed_at: RwLock::new(None),
            keys: RwLock::new(None),
            resolved: RwLock::new(HashMap::new()),
            intent: Mutex::new(None),
            tune: tokio::sync::Mutex::new(()),
            epg_cache: RwLock::new(HashMap::new()),
            last_activity: Arc::new(Mutex::new(Instant::now())),
            last_close: Mutex::new(None),
            heartbeat: Mutex::new(None),
            generation: AtomicU64::new(0),
            commit: Mutex::new(()),
            account_transition: tokio::sync::Mutex::new(()),
            locks: KeyedLocks::default(),
        }
    }

    fn client_for_http(
        &self,
        http: &reqwest::Client,
        store: &crate::store::Store,
    ) -> anyhow::Result<Client> {
        Client::from_store(http.clone(), store)
    }

    pub fn enabled(&self) -> bool {
        self.config_enabled
    }

    pub fn connected(&self) -> bool {
        self.client.read().unwrap().is_some()
    }

    pub fn client(&self) -> Option<Arc<Client>> {
        self.client.read().unwrap().clone()
    }

    pub fn init(&self, http: &reqwest::Client, store: &crate::store::Store) {
        if !self.config_enabled {
            return;
        }
        if !crate::tataplay::is_logged_in(store) {
            return;
        }
        match self.client_for_http(http, store) {
            Ok(client) => *self.client.write().unwrap() = Some(Arc::new(client)),
            Err(_) => *self.client.write().unwrap() = None,
        }
    }

    #[cfg(test)]
    pub fn set_client_for_test(&self, client: Arc<Client>) {
        *self.client.write().unwrap() = Some(client);
    }

    #[cfg(test)]
    pub fn expire_catalogue_for_test(&self) {
        *self.catalogue_fetched_at.write().unwrap() =
            Some(Instant::now() - CATALOGUE_TTL - Duration::from_secs(1));
        *self.catalogue_failed_at.write().unwrap() = None;
    }

    fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    fn commit_if_current<R>(&self, generation: u64, commit: impl FnOnce() -> R) -> Option<R> {
        let _guard = self.commit.lock().unwrap();
        if self.generation() != generation {
            return None;
        }
        Some(commit())
    }

    /// Drops every account-scoped cache at one serialized boundary. Credentials
    /// are left in place; `logout` removes them separately.
    pub fn clear(&self) {
        self.stop_heartbeat();
        let _guard = self.commit.lock().unwrap();
        self.generation.fetch_add(1, Ordering::SeqCst);
        *self.catalogue.write().unwrap() = Vec::new();
        *self.catalogue_fetched_at.write().unwrap() = None;
        *self.catalogue_failed_at.write().unwrap() = None;
        *self.keys.write().unwrap() = None;
        self.resolved.write().unwrap().clear();
        self.epg_cache.write().unwrap().clear();
    }

    pub async fn logout(&self, store: &crate::store::Store) -> anyhow::Result<()> {
        let _transition = self.account_transition.lock().await;
        // Drop the client before the generation bump so an in-flight fetch can
        // never publish one account's data after the next account is active.
        *self.client.write().unwrap() = None;
        self.clear();
        crate::tataplay::logout(store).await
    }

    /// Sends an OTP before a session exists, so it builds a client from the
    /// persisted device exactly like `verify_otp` does.
    pub async fn send_otp(
        &self,
        http: &reqwest::Client,
        store: &crate::store::Store,
        rmn: &str,
        sid: &str,
    ) -> anyhow::Result<Vec<crate::tataplay::LoginRef>> {
        let client = self.client_for_http(http, store)?;
        client.send_otp(store, rmn, sid).await
    }

    pub async fn verify_otp(
        &self,
        http: &reqwest::Client,
        store: &crate::store::Store,
        otp: &str,
        ref_id: &str,
        ref_id_mode: bool,
        on_installed: impl FnOnce(),
    ) -> anyhow::Result<bool> {
        let _transition = self.account_transition.lock().await;
        let client = self.client_for_http(http, store)?;
        let credentials = client.verify_otp(store, otp, ref_id, ref_id_mode).await?;
        // Install the new client before rotating the cache context: a fetch
        // that starts in between sees the old generation and its commit is
        // dropped, while a fetch that sees the new generation also sees the
        // new client.
        self.init(http, store);
        // Rotate the context at the exact moment the new account replaces the
        // old one, before any account-scoped cache can be reused.
        on_installed();
        self.clear();
        let _ = credentials;
        Ok(true)
    }

    /// Refreshes the retained client's tokens and replaces it in place so the
    /// media, license and catalogue paths stop using the expired token.
    pub async fn refresh(&self, store: &crate::store::Store) -> anyhow::Result<()> {
        let _transition = self.account_transition.lock().await;
        let client = self
            .client()
            .ok_or_else(|| anyhow::anyhow!("tataplay: not connected"))?;
        let credentials = client.refresh(store).await?;
        *self.client.write().unwrap() = Some(Arc::new(client.with_credentials(credentials)));
        self.clear();
        Ok(())
    }

    /// Returns whether a fresh catalogue is available. A failed fetch leaves
    /// the previous generation dropped rather than serving another account's
    /// list, and the caller can report the upstream failure.
    pub async fn ensure_catalogue(&self) -> bool {
        if !self.connected() {
            return false;
        }
        if self
            .catalogue_fetched_at
            .read()
            .unwrap()
            .is_some_and(|at| at.elapsed() < CATALOGUE_TTL)
        {
            return true;
        }
        if self
            .catalogue_failed_at
            .read()
            .unwrap()
            .is_some_and(|at| at.elapsed() < CATALOGUE_RETRY_BACKOFF)
        {
            return false;
        }
        let _guard = self.locks.lock("catalogue").await;
        if self
            .catalogue_fetched_at
            .read()
            .unwrap()
            .is_some_and(|at| at.elapsed() < CATALOGUE_TTL)
        {
            return true;
        }
        if self
            .catalogue_failed_at
            .read()
            .unwrap()
            .is_some_and(|at| at.elapsed() < CATALOGUE_RETRY_BACKOFF)
        {
            return false;
        }
        // Read both after the lock so they describe one account. A transition
        // that swaps the client before the bump leaves this commit stale and
        // the bump discards it.
        let generation = self.generation();
        let Some(client) = self.client() else {
            return false;
        };
        match client.catalogue().await {
            Ok(channels) => {
                let now = Instant::now();
                self.commit_if_current(generation, || {
                    let mut catalogue = self.catalogue.write().unwrap();
                    let changed = catalogue.len() != channels.len()
                        || catalogue
                            .iter()
                            .zip(channels.iter())
                            .any(|(old, new)| old.id != new.id);
                    *catalogue = channels;
                    if changed {
                        // A pack change invalidates every cached guide day:
                        // the new lineup can have different programme rows.
                        self.epg_cache.write().unwrap().clear();
                    }
                    *self.catalogue_fetched_at.write().unwrap() = Some(now);
                    *self.catalogue_failed_at.write().unwrap() = None;
                })
                .is_some()
            }
            Err(_) => {
                let _ = self.commit_if_current(generation, || {
                    *self.catalogue_failed_at.write().unwrap() = Some(Instant::now());
                });
                false
            }
        }
    }

    pub fn channels(&self) -> Vec<Channel> {
        self.catalogue.read().unwrap().clone()
    }

    pub fn contains(&self, channel_id: &str) -> bool {
        self.catalogue
            .read()
            .unwrap()
            .iter()
            .any(|channel| channel.id == channel_id)
    }

    /// One IST day of the guide, cached so XMLTV and the web guide share the
    /// paged fetches that the BFF requires.
    pub async fn epg_day(&self, day_start: i64) -> anyhow::Result<Vec<GuideChannel>> {
        if let Some((channels, at)) = self.epg_cache.read().unwrap().get(&day_start) {
            if at.elapsed() < EPG_TTL {
                return Ok(channels.clone());
            }
        }
        let generation = self.generation();
        let client = self
            .client()
            .ok_or_else(|| anyhow::anyhow!("tataplay: not connected"))?;
        let _guard = self.locks.lock(&format!("epg:{day_start}")).await;
        if let Some((channels, at)) = self.epg_cache.read().unwrap().get(&day_start) {
            if at.elapsed() < EPG_TTL {
                return Ok(channels.clone());
            }
        }
        let channels = client.guide_day(day_start).await?;
        let now = Instant::now();
        self.commit_if_current(generation, || {
            let mut cache = self.epg_cache.write().unwrap();
            cache.retain(|_, (_, at)| at.elapsed() < EPG_TTL);
            cache.insert(day_start, (channels.clone(), now));
        });
        Ok(channels)
    }

    /// Channels and the next two IST days' programmes, for
    /// `epg::generate_xml_gz_from_sources`.
    pub async fn epg_source(
        &self,
    ) -> anyhow::Result<(Vec<(String, String)>, Vec<crate::epg::XmlProgramme>)> {
        self.ensure_catalogue().await;
        let channels = self.channels();
        let today = crate::tataplay::tataplay_day_start();
        let mut guide: HashMap<String, Vec<GuideProgramme>> = HashMap::new();
        let mut categories: HashMap<String, String> = HashMap::new();
        for day in [today, today.saturating_add(86_400_000)] {
            for channel in self.epg_day(day).await? {
                categories
                    .entry(channel.id.clone())
                    .or_insert_with(|| channel.primary_genre.clone());
                guide
                    .entry(channel.id)
                    .or_default()
                    .extend(channel.programmes);
            }
        }
        let mut sources = Vec::with_capacity(channels.len());
        let mut programmes = Vec::new();
        for channel in &channels {
            sources.push((channel.id.clone(), channel.name.clone()));
            let Some(document) = document_id(&channel.id) else {
                continue;
            };
            let Some(rows) = guide.get(document) else {
                continue;
            };
            let category = categories
                .get(document)
                .map(String::as_str)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| crate::television::category_name(channel.category));
            let mut seen = std::collections::HashSet::new();
            for row in rows {
                if !seen.insert((row.start_ms, row.end_ms)) {
                    continue;
                }
                programmes.push(crate::epg::XmlProgramme {
                    channel: channel.id.clone(),
                    start: crate::epg::format_xmltv_time(row.start_ms),
                    stop: crate::epg::format_xmltv_time(row.end_ms),
                    title: row.title.clone(),
                    desc: row.desc.clone(),
                    category: category.to_string(),
                    icon: row.icon.clone(),
                });
            }
        }
        Ok((sources, programmes))
    }

    /// One channel's programmes for a day offset, in the shared web-guide
    /// shape.
    pub async fn web_epg(
        &self,
        channel_id: &str,
        offset_days: i64,
    ) -> anyhow::Result<Vec<GuideProgramme>> {
        let document = document_id(channel_id)
            .ok_or_else(|| anyhow::anyhow!("tataplay: not a tataplay channel"))?;
        self.ensure_catalogue().await;
        // A direct EPG link must respect the account's current lineup. A
        // cached guide day may still contain a channel removed by a pack
        // change; catalogue membership is the gate.
        if !self.contains(channel_id) {
            return Ok(Vec::new());
        }
        let day = crate::tataplay::tataplay_day_start()
            .saturating_add(offset_days.clamp(0, 6).saturating_mul(86_400_000));
        let guide = self.epg_day(day).await?;
        Ok(guide
            .into_iter()
            .find(|channel| channel.id == document)
            .map(|channel| channel.programmes)
            .unwrap_or_default())
    }

    async fn app_keys(&self) -> Option<AppKeys> {
        if let Some((keys, at)) = self.keys.read().unwrap().as_ref() {
            if at.elapsed() < KEYS_TTL {
                return Some(keys.clone());
            }
        }
        let client = self.client()?;
        let keys = match client.app_config().await {
            Ok(keys) => keys,
            Err(error) => {
                tracing::warn!(%error, "tataplay: cannot fetch the decryption keys");
                return None;
            }
        };
        *self.keys.write().unwrap() = Some((keys.clone(), Instant::now()));
        Some(keys)
    }

    /// Whether the heartbeat slot is held by this document's session.
    ///
    /// A cached `ResolvedPlayback` is only usable while its concurrency
    /// session is the one the account is beating. Switching away closes that
    /// session, so a cache hit after switching back must resolve a fresh one
    /// instead of replaying a session the upstream has already freed.
    fn session_is_active(&self, document: &str, resolved: &ResolvedPlayback) -> bool {
        if resolved.session_id.is_empty() {
            return true;
        }
        let slot = self.heartbeat.lock().unwrap();
        slot.as_ref().is_some_and(|running| {
            running.document == document && running.session_id == resolved.session_id
        })
    }

    /// The document whose session currently owns the account's single
    /// concurrency slot.
    pub fn active_document(&self) -> Option<String> {
        let slot = self.heartbeat.lock().unwrap();
        slot.as_ref().map(|running| running.document.clone())
    }

    pub fn is_active_channel(&self, channel_id: &str) -> bool {
        let Some(document) = document_id(channel_id) else {
            return false;
        };
        self.active_document().as_deref() == Some(document)
    }

    /// Whether a session close was attempted recently, so a `130402` licence
    /// refusal belongs to a local switch rather than another device.
    pub fn recently_closed(&self, within: Duration) -> bool {
        self.last_close
            .lock()
            .unwrap()
            .is_some_and(|at| at.elapsed() <= within)
    }

    #[cfg(test)]
    pub fn mark_closed_for_test(&self) {
        *self.last_close.lock().unwrap() = Some(Instant::now());
    }

    /// Start a deliberate tune. The new id makes every manifest and licence
    /// URL minted for the previous tune stale, including a return to the same
    /// channel later.
    pub fn tune(&self, channel_id: &str) -> Option<String> {
        let document = document_id(channel_id)?;
        let playback_id = Self::new_playback_id();
        // Serialized with license publication so a session can never be
        // installed after this selection replaced its intent.
        let _guard = self.commit.lock().unwrap();
        *self.intent.lock().unwrap() = Some(PlaybackIntent {
            document: document.to_string(),
            playback_id: playback_id.clone(),
        });
        Some(playback_id)
    }

    fn new_playback_id() -> String {
        format!("{:016x}", rand::random::<u64>())
    }

    /// Start a tune only when the channel is not already the live intent.
    /// Players that re-request `/live/mpd` during a manifest refresh must not
    /// rotate their own route id.
    pub fn tune_if_changed(&self, channel_id: &str) -> Option<String> {
        let document = document_id(channel_id)?;
        if self.intent_for(document).is_some()
            && self.active_document().as_deref() == Some(document)
        {
            return self.current_playback_id(channel_id);
        }
        self.tune(channel_id)
    }

    /// A selection outside Tata Play (for example a custom channel) makes any
    /// pending Tata Play resolve stale before it can start a session, and
    /// releases the account's single slot instead of leaving it to expire.
    pub fn cancel_tune(&self) {
        {
            let _guard = self.commit.lock().unwrap();
            *self.intent.lock().unwrap() = None;
        }
        self.stop_heartbeat();
    }

    /// Whether a resolved request still matches the user's current selection.
    pub fn selection_matches(&self, channel_id: &str, playback_id: &str) -> bool {
        document_id(channel_id).is_some_and(|document| {
            self.intent_for(document)
                .is_some_and(|intent| intent.playback_id == playback_id)
        })
    }

    fn intent_for(&self, document: &str) -> Option<PlaybackIntent> {
        self.intent
            .lock()
            .unwrap()
            .as_ref()
            .filter(|intent| intent.document == document)
            .cloned()
    }

    /// The intent for a tune, creating one when the caller did not go through
    /// [`Self::tune`] (tests and direct provider use).
    fn begin_tune(&self, document: &str) -> PlaybackIntent {
        let _guard = self.commit.lock().unwrap();
        let mut intent = self.intent.lock().unwrap();
        if let Some(current) = intent.as_ref() {
            if current.document == document {
                return current.clone();
            }
        }
        let next = PlaybackIntent {
            document: document.to_string(),
            playback_id: Self::new_playback_id(),
        };
        *intent = Some(next.clone());
        next
    }

    /// The playback id a route must present. Empty when no route is live.
    pub fn current_playback_id(&self, channel_id: &str) -> Option<String> {
        let document = document_id(channel_id)?;
        if self.active_document().as_deref() == Some(document) {
            if let Some((resolved, _)) = self.resolved.read().unwrap().get(document) {
                if !resolved.playback_id.is_empty() {
                    return Some(resolved.playback_id.clone());
                }
            }
        }
        self.intent_for(document).map(|intent| intent.playback_id)
    }

    /// Whether a route's playback id belongs to the tune that currently owns
    /// the slot. An id from a previous tune of the same channel is stale.
    pub fn playback_matches(&self, channel_id: &str, playback_id: &str) -> bool {
        if playback_id.is_empty() {
            return true;
        }
        let Some(document) = document_id(channel_id) else {
            return false;
        };
        if self.active_document().as_deref() != Some(document) {
            return false;
        }
        self.resolved
            .read()
            .unwrap()
            .get(document)
            .map(|(resolved, _)| resolved.playback_id == playback_id)
            .unwrap_or(false)
    }

    async fn ensure_playback(
        &self,
        document: &str,
        mode: PlaybackMode,
        selected: Option<&str>,
    ) -> Option<ResolvedPlayback> {
        *self.last_activity.lock().unwrap() = Instant::now();
        // A tune must keep the intent it entered with. Creating it only after
        // the mutex would let a queued request replace a newer selection.
        // A request that came through `/api/live/play` carries the id it
        // selected; when a custom channel has since replaced it, that request
        // must stop here instead of starting a session for a channel the user
        // already left.
        let tune_intent = match (mode, selected) {
            (PlaybackMode::Tune, Some(selected)) => {
                let intent = self.intent_for(document)?;
                if intent.playback_id != selected {
                    return None;
                }
                Some(intent)
            }
            (PlaybackMode::Tune, None) => Some(self.begin_tune(document)),
            (PlaybackMode::Refresh, _) => None,
        };
        if mode == PlaybackMode::Refresh {
            self.intent_for(document)?;
            if let Some(active) = self.active_document() {
                if active != document {
                    return None;
                }
            }
        }
        let cached = self.resolved.read().unwrap().get(document).cloned();
        if let Some((resolved, at)) = cached {
            if at.elapsed() < PLAYBACK_TTL && self.session_is_active(document, &resolved) {
                let mut resolved = resolved;
                if let Some(intent) = &tune_intent {
                    if self.intent_for(document).map(|i| i.playback_id)
                        != Some(intent.playback_id.clone())
                    {
                        return None;
                    }
                    if resolved.playback_id != intent.playback_id {
                        resolved.playback_id = intent.playback_id.clone();
                        if let Some((entry, _)) = self.resolved.write().unwrap().get_mut(document) {
                            entry.playback_id = intent.playback_id.clone();
                        }
                    }
                }
                return Some(resolved);
            }
        }
        let generation = self.generation();
        let Some(client) = self.client() else {
            tracing::warn!("tataplay: playback resolve has no client");
            return None;
        };
        let _tune = self.tune.lock().await;
        if self.generation() != generation {
            tracing::warn!("tataplay: playback resolve lost its account generation");
            return None;
        }
        // Another resolve may have filled the cache while this one waited.
        let cached = self.resolved.read().unwrap().get(document).cloned();
        if let Some((resolved, at)) = cached {
            if at.elapsed() < PLAYBACK_TTL && self.session_is_active(document, &resolved) {
                let mut resolved = resolved;
                if let Some(intent) = &tune_intent {
                    if self.intent_for(document).map(|i| i.playback_id)
                        != Some(intent.playback_id.clone())
                    {
                        return None;
                    }
                    if resolved.playback_id != intent.playback_id {
                        resolved.playback_id = intent.playback_id.clone();
                        if let Some((entry, _)) = self.resolved.write().unwrap().get_mut(document) {
                            entry.playback_id = intent.playback_id.clone();
                        }
                    }
                }
                return Some(resolved);
            }
        }
        let intent = match mode {
            PlaybackMode::Tune => tune_intent?,
            PlaybackMode::Refresh => match self.intent_for(document) {
                Some(intent) => intent,
                // A tune for another channel landed while this refresh waited.
                None => return None,
            },
        };
        // A queued tune keeps its original identity even if another selection
        // arrived while it waited for the mutex.
        if self.intent_for(document).map(|i| i.playback_id) != Some(intent.playback_id.clone()) {
            return None;
        }
        if mode == PlaybackMode::Refresh {
            if let Some(active) = self.active_document() {
                if active != document {
                    return None;
                }
            }
        }
        let Some(keys) = self.app_keys().await else {
            tracing::warn!("tataplay: playback resolve has no decryption keys");
            return None;
        };
        // The account's concurrency slot is one. Release whatever session owns
        // it before asking access control for the next one.
        self.stop_heartbeat_and_wait().await;
        let mut resolved = match client.play(document, &keys).await {
            Ok(resolved) => resolved,
            Err(error) => {
                tracing::warn!(%error, "tataplay: playback resolve failed");
                return None;
            }
        };
        resolved.playback_id = intent.playback_id.clone();
        // A newer tune may have landed during the access-control call. Do not
        // install or beat a session for a channel the user has already left.
        if self.intent_for(document).map(|current| current.playback_id)
            != Some(intent.playback_id.clone())
        {
            if let Err(error) = client.end_session(&resolved.session_id).await {
                tracing::debug!(%error, "tataplay stale resolve session end failed");
            }
            return None;
        }
        let clone = resolved.clone();
        self.commit_if_current(generation, || {
            self.resolved
                .write()
                .unwrap()
                .insert(document.to_string(), (resolved, Instant::now()));
            self.start_heartbeat(&client, document, &clone);
        })?;
        Some(clone)
    }

    /// Registers the playback session and keeps beating while it is live.
    /// `NetworkApi.X` sends no body, so the id and device name are the only
    /// inputs.
    fn start_heartbeat(&self, client: &Arc<Client>, document: &str, resolved: &ResolvedPlayback) {
        if resolved.session_id.is_empty() {
            return;
        }
        let mut slot = self.heartbeat.lock().unwrap();
        if slot.as_ref().is_some_and(|running| {
            running.session_id == resolved.session_id && !running.task.is_finished()
        }) {
            return;
        }
        if let Some(previous) = slot.take() {
            previous.task.abort();
            // Aborting the task only stops the beat. Close the replaced session
            // explicitly with the client that owns it so the upstream
            // concurrency slot is not held until it times out.
            tokio::spawn(async move {
                if let Err(error) = previous.client.end_session(&previous.session_id).await {
                    tracing::debug!(%error, "tataplay replaced session end failed");
                }
            });
        }
        let seconds = resolved
            .heartbeat_interval
            .unwrap_or(DEFAULT_HEARTBEAT_SECS);
        let session_id = resolved.session_id.clone();
        let task_client = (**client).clone();
        let task_session = session_id.clone();
        let last_activity = self.last_activity.clone();
        let task = tokio::spawn(async move {
            if let Err(error) = task_client.start_session(&task_session).await {
                tracing::debug!(%error, "tataplay session start failed");
            }
            let mut ticker = tokio::time::interval(Duration::from_secs(seconds));
            ticker.tick().await;
            loop {
                ticker.tick().await;
                if last_activity.lock().unwrap().elapsed() >= HEARTBEAT_IDLE_TIMEOUT {
                    if let Err(error) = task_client.end_session(&task_session).await {
                        tracing::debug!(%error, "tataplay idle session end failed");
                    }
                    break;
                }
                if let Err(error) = task_client.start_session(&task_session).await {
                    tracing::debug!(%error, "tataplay session heartbeat failed");
                }
            }
        });
        *slot = Some(Heartbeat {
            document: document.to_string(),
            session_id,
            client: client.clone(),
            task,
        });
    }

    /// Stops the beat and waits for the upstream DELETE, freeing the single
    /// concurrency slot before a new access-control call.
    async fn stop_heartbeat_and_wait(&self) {
        let running = self.heartbeat.lock().unwrap().take();
        let Some(running) = running else {
            return;
        };
        let document = running.document.clone();
        *self.last_close.lock().unwrap() = Some(Instant::now());
        running.task.abort();
        match running.client.end_session(&running.session_id).await {
            Ok(()) => tracing::debug!(document = %document, "tataplay session closed"),
            Err(error) => {
                tracing::debug!(%error, document = %document, "tataplay session end failed")
            }
        }
    }

    /// Stops the beat and closes the session, freeing the concurrency slot.
    fn stop_heartbeat(&self) {
        let Some(running) = self.heartbeat.lock().unwrap().take() else {
            return;
        };
        running.task.abort();
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        // Use the client that owns the session: logout clears the active client
        // and verify_otp replaces it before this cleanup runs.
        tokio::spawn(async move {
            if let Err(error) = running.client.end_session(&running.session_id).await {
                tracing::debug!(%error, "tataplay session end failed");
            }
        });
    }

    fn stop_heartbeat_for(&self, document: &str) {
        let matches = self
            .heartbeat
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|running| running.document == document);
        if matches {
            self.stop_heartbeat();
        }
    }

    /// Drop one cached playback entry after the CDN rejects its media URL.
    pub async fn invalidate_playback(&self, channel_id: &str) {
        let Some(document) = document_id(channel_id) else {
            return;
        };
        let _guard = self.locks.lock(&format!("playback:{document}")).await;
        self.resolved.write().unwrap().remove(document);
        self.stop_heartbeat_for(document);
    }

    /// Resolves a `tp_` channel to the shared playback shape.
    pub async fn live(&self, channel_id: &str) -> anyhow::Result<LiveUrlOutput> {
        self.live_selected(channel_id, None).await
    }

    /// Resolves a channel only while `selected` is still the user's choice.
    /// `/api/live/play` passes the id it registered, so a request that a
    /// custom-channel selection superseded cannot open a session.
    pub async fn live_selected(
        &self,
        channel_id: &str,
        selected: Option<&str>,
    ) -> anyhow::Result<LiveUrlOutput> {
        let document = document_id(channel_id)
            .ok_or_else(|| anyhow::anyhow!("tataplay: not a tataplay channel"))?;
        self.ensure_catalogue().await;
        let resolved = self
            .ensure_playback(document, PlaybackMode::Tune, selected)
            .await
            .ok_or_else(|| anyhow::anyhow!("tataplay: playback could not be resolved"))?;
        Ok(self.live_output(&resolved))
    }

    /// Refresh the manifest for the tune that already owns the slot. A
    /// different channel's late manifest gets no resolve and cannot take the
    /// slot back.
    pub async fn refresh_live(&self, channel_id: &str) -> anyhow::Result<LiveUrlOutput> {
        let document = document_id(channel_id)
            .ok_or_else(|| anyhow::anyhow!("tataplay: not a tataplay channel"))?;
        self.ensure_catalogue().await;
        let resolved = self
            .ensure_playback(document, PlaybackMode::Refresh, None)
            .await
            .ok_or_else(|| anyhow::anyhow!("tataplay: playback could not be refreshed"))?;
        Ok(self.live_output(&resolved))
    }

    fn live_output(&self, resolved: &ResolvedPlayback) -> LiveUrlOutput {
        let mut out = LiveUrlOutput {
            result: "OK".to_string(),
            ..Default::default()
        };
        out.mpd = Mpd {
            auto: resolved.playable.clone(),
            key: resolved.license_url.clone(),
            ..Default::default()
        };
        out.key_url = resolved.license_url.clone();
        out.is_drm = !resolved.license_url.is_empty();
        out.playback_id = resolved.playback_id.clone();
        out
    }

    /// Headers the media proxy must add for a resolved Tata Play channel.
    pub async fn media_headers(&self, channel_id: &str) -> Vec<(String, String)> {
        let Some(document) = document_id(channel_id) else {
            return Vec::new();
        };
        match self
            .ensure_playback(document, PlaybackMode::Refresh, None)
            .await
        {
            Some(resolved) => resolved.media_headers,
            None => Vec::new(),
        }
    }

    /// Headers the license proxy must add for a resolved Tata Play channel.
    /// Resolves a fresh access-control session and returns its license URL and
    /// playback token. Used when Irdeto refuses a challenge with `130601`
    /// after the first licence, because the token is consumed by that one
    /// challenge.
    ///
    /// `stale_session_id` is the session the refused challenge used. If
    /// another license request already rotated this channel to a newer
    /// session while this one waited for the lock, that newer session is
    /// returned as-is: rotating again would close the session the other
    /// request just handed to its player.
    pub async fn refresh_license(
        &self,
        channel_id: &str,
        stale_session_id: &str,
        expected_playback_id: Option<&str>,
    ) -> anyhow::Result<LicenseSession> {
        let document = document_id(channel_id)
            .ok_or_else(|| anyhow::anyhow!("tataplay: not a tataplay channel"))?;
        let _guard = self.locks.lock(&format!("license:{document}")).await;

        if let Some((resolved, _)) = self.resolved.read().unwrap().get(document) {
            if !resolved.session_id.is_empty() && resolved.session_id != stale_session_id {
                // A replacement tune owns this channel now. Do not hand its
                // token to the old challenge.
                if expected_playback_id.is_some_and(|expected| expected != resolved.playback_id) {
                    anyhow::bail!("tataplay: playback changed during the license refresh");
                }
                return LicenseSession::from_playback(resolved)
                    .ok_or_else(|| anyhow::anyhow!("tataplay: playback carries no license URL"));
            }
        }

        // A late retry for a channel that no longer owns the account's single
        // slot must not close the session the account is using now.
        let _tune = self.tune.lock().await;
        let Some(intent) = self.intent_for(document) else {
            anyhow::bail!("tataplay: channel {channel_id} is no longer the intended playback");
        };
        if let Some(active) = self.active_document() {
            if active != document {
                anyhow::bail!("tataplay: channel {channel_id} is not the active playback");
            }
        }
        if expected_playback_id.is_some_and(|expected| expected != intent.playback_id) {
            anyhow::bail!("tataplay: playback changed during the license refresh");
        }
        let playback_id = self
            .resolved
            .read()
            .unwrap()
            .get(document)
            .map(|(resolved, _)| resolved.playback_id.clone())
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| intent.playback_id.clone());
        let generation = self.generation();

        let client = self
            .client()
            .ok_or_else(|| anyhow::anyhow!("tataplay: not connected"))?;
        let keys = self
            .app_keys()
            .await
            .ok_or_else(|| anyhow::anyhow!("tataplay: decryption keys are unavailable"))?;
        // The account allows one live session. Close the one this playback
        // already registered before asking for another, or the license
        // endpoint answers `130402 Too many concurrent streams`.
        self.stop_heartbeat_and_wait().await;

        // The upstream can still see the session just closed for a moment.
        // Retry the access-control call briefly rather than failing the
        // player's only retry on that race.
        let mut resolved = None;
        let mut last_error = None;
        for attempt in 0..3u32 {
            match client.play(document, &keys).await {
                Ok(playback) => {
                    resolved = Some(playback);
                    break;
                }
                Err(error) => {
                    let message = error.to_string();
                    let slot_busy = message.contains("130402")
                        || message.to_ascii_lowercase().contains("concurrent");
                    last_error = Some(error);
                    if !slot_busy || attempt == 2 {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(300 * u64::from(attempt + 1))).await;
                }
            }
        }
        let mut resolved = resolved.ok_or_else(|| {
            last_error
                .unwrap_or_else(|| anyhow::anyhow!("tataplay: playback could not be resolved"))
        })?;
        resolved.playback_id = playback_id.clone();
        let committed = self.commit_if_current(generation, || {
            // Atomic with `tune`: a selection that replaced this playback id
            // while `client.play` waited must win, even though the account
            // generation is unchanged.
            if self.intent_for(document).map(|current| current.playback_id)
                != Some(playback_id.clone())
            {
                return false;
            }
            self.resolved
                .write()
                .unwrap()
                .insert(document.to_string(), (resolved.clone(), Instant::now()));
            self.start_heartbeat(&client, document, &resolved);
            true
        });
        if committed != Some(true) {
            if let Err(error) = client.end_session(&resolved.session_id).await {
                tracing::debug!(%error, "tataplay stale refresh session end failed");
            }
            if committed == Some(false) {
                anyhow::bail!("tataplay: playback changed during the license refresh");
            }
            anyhow::bail!("tataplay: account changed during the license refresh");
        }
        LicenseSession::from_playback(&resolved)
            .ok_or_else(|| anyhow::anyhow!("tataplay: playback carries no license URL"))
    }

    /// The active license URL and the credentials that authorise it, taken
    /// from one `ResolvedPlayback` snapshot. A license request for a channel
    /// that no longer owns the playback slot is refused instead of borrowing
    /// the active channel's session.
    pub async fn license_session(&self, channel_id: &str) -> anyhow::Result<LicenseSession> {
        let document = document_id(channel_id)
            .ok_or_else(|| anyhow::anyhow!("tataplay: not a tataplay channel"))?;
        let _guard = self.locks.lock(&format!("license:{document}")).await;
        if let Some(active) = self.active_document() {
            if active != document {
                anyhow::bail!("tataplay: channel {channel_id} is not the active playback");
            }
        }
        let resolved = self
            .ensure_playback(document, PlaybackMode::Refresh, None)
            .await
            .ok_or_else(|| anyhow::anyhow!("tataplay: playback could not be resolved"))?;
        LicenseSession::from_playback(&resolved)
            .ok_or_else(|| anyhow::anyhow!("tataplay: playback carries no license URL"))
    }

    pub fn playlist_ready(&self, channel_id: &str) -> bool {
        self.contains(channel_id)
    }

    pub fn account_fingerprint(&self) -> String {
        let client = self.client.read().unwrap();
        let Some(client) = client.as_ref() else {
            return String::new();
        };
        let Ok(creds) = client.credentials() else {
            return String::new();
        };
        let mut hasher = Sha256::new();
        hasher.update(b"tataplay-account-v1");
        hasher.update(creds.subscriber_id.as_bytes());
        hasher.update(b"\0");
        hasher.update(creds.access_token.as_bytes());
        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tataplay::{channel_id, Credentials};
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn guide_row(id: u32, number: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "title": format!("Channel {id}"),
            "channelNumber": number
        })
    }

    fn guide_row_with_epg(id: u32, number: &str) -> serde_json::Value {
        let mut row = guide_row(id, number);
        row["epg"] = serde_json::json!([{
            "startTime": 1795293000000i64,
            "endTime": 1795296600000i64,
            "title": "Test programme"
        }]);
        row
    }

    fn client_for(server: &MockServer) -> Arc<Client> {
        Arc::new(Client::for_test(
            &server.uri(),
            Credentials {
                access_token: "token".into(),
                refresh_token: "refresh".into(),
                subscriber_id: "sub-1".into(),
                sid: "sid-1".into(),
            },
        ))
    }

    #[tokio::test]
    async fn catalogue_uses_the_subscribed_guide_and_sorts_channels() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/lit-mobile-common-bff/api/v2/channels/guide"))
            .and(header("sid", "sid-1"))
            .and(query_param("limit", "600"))
            .and(query_param("subscribedFilter", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"total": 3, "channelList": [
                    {"id": 1918, "title": "Star Plus", "channelNumber": "115",
                     "hdChannel": true, "boxCoverImage": "a.png"},
                    {"id": 1914, "title": "Zee TV", "channelNumber": "120"},
                    {"id": 40, "title": "Sony SAB", "channelNumber": "121",
                     "restartTv": true}
                ]}
            })))
            .mount(&server)
            .await;
        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        assert!(state.ensure_catalogue().await);
        let channels = state.channels();
        // The guide's subscribed page is the lineup; there is no second pack
        // filter to drop a row the provider returned.
        assert_eq!(channels.len(), 3);
        assert!(state.contains(&channel_id("1918")));
        assert!(state.contains(&channel_id("1914")));
        assert!(state.contains("tp_40"));
        assert_eq!(channels[0].id, "tp_1918");
        assert_eq!(channels[0].logo_url, "a.png");
        assert_eq!(channels[1].id, "tp_1914");
        assert!(channels[2].is_catchup_available);
    }

    #[tokio::test]
    async fn catalogue_change_clears_cached_guide_days() {
        let server = MockServer::start().await;
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let calls_for_response = calls.clone();
        Mock::given(method("GET"))
            .and(path("/lit-mobile-common-bff/api/v2/channels/guide"))
            .and(query_param("limit", "600"))
            .and(query_param("subscribedFilter", "true"))
            .respond_with(move |_request: &wiremock::Request| {
                let call = calls_for_response.fetch_add(1, Ordering::SeqCst);
                let rows = match call {
                    0 => vec![guide_row(1918, "115")],
                    1 => vec![guide_row(1918, "115"), guide_row(1928, "116")],
                    _ => vec![guide_row(1918, "115")],
                };
                let total = rows.len();
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "data": {"total": total, "channelList": rows}
                }))
            })
            .expect(3)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/lit-mobile-common-bff/api/v2/channels/guide"))
            .and(query_param("limit", "200"))
            .and(query_param("subscribedFilter", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"total": 1, "channelList": [guide_row_with_epg(1918, "115")]}
            })))
            .mount(&server)
            .await;

        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        assert!(state.ensure_catalogue().await);
        assert!(state.contains("tp_1918"));
        assert!(!state.contains("tp_1928"));
        let today = crate::tataplay::tataplay_day_start();
        assert!(!state.epg_day(today).await.unwrap().is_empty());
        assert!(!state.epg_cache.read().unwrap().is_empty());

        state.expire_catalogue_for_test();
        assert!(state.ensure_catalogue().await);
        assert!(
            state.contains("tp_1928"),
            "an added pack appears after refresh"
        );
        assert!(
            state.epg_cache.read().unwrap().is_empty(),
            "a changed lineup must not reuse the old guide day"
        );

        state.expire_catalogue_for_test();
        assert!(state.ensure_catalogue().await);
        assert!(
            !state.contains("tp_1928"),
            "a removed pack leaves the lineup"
        );
        assert!(state.epg_cache.read().unwrap().is_empty());
        server.verify().await;
    }

    #[tokio::test]
    async fn direct_web_epg_requires_catalogue_membership() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/lit-mobile-common-bff/api/v2/channels/guide"))
            .and(query_param("limit", "600"))
            .and(query_param("subscribedFilter", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"total": 1, "channelList": [guide_row(1918, "115")]}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/lit-mobile-common-bff/api/v2/channels/guide"))
            .and(query_param("limit", "200"))
            .and(query_param("subscribedFilter", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"total": 1, "channelList": [guide_row_with_epg(1918, "115")]}
            })))
            .mount(&server)
            .await;

        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        assert!(
            state.web_epg("tp_1928", 0).await.unwrap().is_empty(),
            "a direct link to a channel outside the lineup has no rows"
        );
        assert_eq!(state.web_epg("tp_1918", 0).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn catalogue_failure_keeps_the_previous_lineup() {
        let server = MockServer::start().await;
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let calls_for_response = calls.clone();
        Mock::given(method("GET"))
            .and(path("/lit-mobile-common-bff/api/v2/channels/guide"))
            .and(query_param("limit", "600"))
            .and(query_param("subscribedFilter", "true"))
            .respond_with(move |_request: &wiremock::Request| {
                if calls_for_response.fetch_add(1, Ordering::SeqCst) == 0 {
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({
                        "data": {"total": 1, "channelList": [guide_row(1918, "115")]}
                    }))
                } else {
                    ResponseTemplate::new(500)
                }
            })
            .expect(2)
            .mount(&server)
            .await;

        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        assert!(state.ensure_catalogue().await);
        state.expire_catalogue_for_test();
        assert!(!state.ensure_catalogue().await);
        assert!(state.contains("tp_1918"));
        assert_eq!(state.channels().len(), 1);
        server.verify().await;
    }

    #[tokio::test]
    async fn playback_failure_does_not_remove_a_catalogue_channel() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/lit-mobile-common-bff/api/v2/channels/guide"))
            .and(query_param("limit", "600"))
            .and(query_param("subscribedFilter", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"total": 1, "channelList": [guide_row(1918, "115")]}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/lit-ls-config/pub/api/v1/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"app": {"dd": {"v1": "ZG5INSxTMSVDbSN1cFclZQ==", "v2": "YWVzRW5jcnlwdGlvbktleQ=="}}}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/pas/api/v1/access/control/LIVE/1918"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "code": 130402,
                "message": "Too many concurrent streams for device type."
            })))
            .mount(&server)
            .await;

        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        assert!(state.ensure_catalogue().await);
        assert!(state.live("tp_1918").await.is_err());
        assert!(
            state.contains("tp_1918"),
            "a tune-time refusal must not change the lineup"
        );
    }

    #[tokio::test]
    async fn xmltv_programmes_only_reference_catalogue_channels() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/lit-mobile-common-bff/api/v2/channels/guide"))
            .and(query_param("limit", "600"))
            .and(query_param("subscribedFilter", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"total": 1, "channelList": [guide_row(1918, "115")]}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/lit-mobile-common-bff/api/v2/channels/guide"))
            .and(query_param("limit", "200"))
            .and(query_param("subscribedFilter", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"total": 2, "channelList": [
                    guide_row_with_epg(1918, "115"),
                    guide_row_with_epg(1928, "116")
                ]}
            })))
            .mount(&server)
            .await;

        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        let (sources, programmes) = state.epg_source().await.unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].0, "tp_1918");
        assert!(!programmes.is_empty());
        assert!(
            programmes
                .iter()
                .all(|programme| programme.channel == "tp_1918"),
            "a programme must never reference a channel outside the XMLTV set"
        );
    }

    #[tokio::test]
    async fn stale_guide_fetch_cannot_repopulate_after_clear() {
        use tokio::sync::Notify;

        let server = MockServer::start().await;
        let started = Arc::new(Notify::new());
        let started_for_response = started.clone();
        Mock::given(method("GET"))
            .and(path("/lit-mobile-common-bff/api/v2/channels/guide"))
            .and(query_param("limit", "200"))
            .and(query_param("subscribedFilter", "true"))
            .respond_with(move |_request: &wiremock::Request| {
                started_for_response.notify_one();
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(150))
                    .set_body_json(serde_json::json!({
                        "data": {"total": 1, "channelList": [guide_row_with_epg(1918, "115")]}
                    }))
            })
            .mount(&server)
            .await;

        let state = Arc::new(TataplayState::new(true));
        state.set_client_for_test(client_for(&server));
        let today = crate::tataplay::tataplay_day_start();
        let fetch = {
            let state = state.clone();
            tokio::spawn(async move { state.epg_day(today).await })
        };
        started.notified().await;
        state.clear();
        let _ = fetch.await.unwrap();
        assert!(
            state.epg_cache.read().unwrap().is_empty(),
            "an old guide response must not repopulate the new context"
        );
    }

    #[tokio::test]
    async fn playback_decrypts_urls_with_published_keys() {
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
                    "sessionId": "sess-1",
                    "tokenInfo": {"token": "playback-token", "expiresIn": 1795285971000i64},
                    "playbackUrls": {
                        "dashWidewinePlayUrl": "not-encrypted",
                        "dashWidewineLicenseUrl": "not-encrypted"
                    }
                }
            })))
            .mount(&server)
            .await;
        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        let live = state.live("tp_1918").await.unwrap();
        assert_eq!(live.result, "OK");
        assert_eq!(live.mpd.auto, "not-encrypted");
        assert!(live.is_drm);
        let session = state.license_session("tp_1918").await.unwrap();
        assert_eq!(session.url, "not-encrypted");
        assert_eq!(session.session_id, "sess-1");
        assert!(
            session
                .headers
                .iter()
                .any(|(name, value)| name == "Authorization" && value == "Bearer playback-token"),
            "the Irdeto challenge must carry the playback token: {:?}",
            session.headers
        );
    }

    /// Refreshing a spent license token must hand the caller the new URL and
    /// the new token together, and a later stale retry must reuse that
    /// session instead of rotating it again.
    #[tokio::test]
    async fn refresh_returns_one_session_url_and_token_and_reuses_it() {
        use std::sync::atomic::AtomicUsize;

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
        let base_for_response = base.clone();
        Mock::given(method("GET"))
            .and(path("/pas/api/v1/access/control/LIVE/1918"))
            .respond_with(move |_request: &wiremock::Request| {
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
                            "dashWidewineLicenseUrl": format!("{base_for_response}/license/{session}")
                        }
                    }
                }))
            })
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/smgs/api/v1/session"))
            .and(header("x-session-id", "sess-a"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .expect(1)
            .mount(&server)
            .await;

        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        state.live("tp_1918").await.unwrap();

        let first = state.license_session("tp_1918").await.unwrap();
        assert_eq!(first.url, format!("{base}/license/sess-a"));
        assert!(first
            .headers
            .iter()
            .any(|(name, value)| name == "Authorization" && value == "Bearer token-a"));

        let fresh = state
            .refresh_license("tp_1918", &first.session_id, None)
            .await
            .unwrap();
        assert_eq!(fresh.session_id, "sess-b");
        assert_eq!(
            fresh.url,
            format!("{base}/license/sess-b"),
            "the refreshed URL must not be paired with the spent token"
        );
        assert!(
            fresh
                .headers
                .iter()
                .any(|(name, value)| name == "Authorization" && value == "Bearer token-b"),
            "the refreshed session must carry its own token: {:?}",
            fresh.headers
        );

        // A second, delayed 130601 retry that still names session A must reuse
        // B rather than closing it and opening another session.
        let reused = state
            .refresh_license("tp_1918", "sess-a", None)
            .await
            .unwrap();
        assert_eq!(reused.session_id, "sess-b");
        assert_eq!(reused.url, fresh.url);
        assert_eq!(plays.load(Ordering::SeqCst), 2);
        server.verify().await;
    }

    /// A deliberate tune must close the old session before asking access
    /// control for the new channel.
    #[tokio::test]
    async fn tune_closes_the_old_session_before_resolving_the_next_channel() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/lit-ls-config/pub/api/v1/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"app": {"dd": {"v1": "ZG5INSxTMSVDbSN1cFclZQ==", "v2": "YWVzRW5jcnlwdGlvbktleQ=="}}}
            })))
            .mount(&server)
            .await;
        for (document, session, token) in
            [("1918", "sess-a", "token-a"), ("523", "sess-b", "token-b")]
        {
            Mock::given(method("GET"))
                .and(path(format!("/pas/api/v1/access/control/LIVE/{document}")))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "data": {
                        "sessionId": session,
                        "heartbeatInterval": 3600,
                        "tokenInfo": {"token": token},
                        "playbackUrls": {
                            "dashWidewinePlayUrl": "not-encrypted",
                            "dashWidewineLicenseUrl": format!("{}/{session}", server.uri())
                        }
                    }
                })))
                .mount(&server)
                .await;
        }
        Mock::given(method("POST"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;

        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        state.live("tp_1918").await.unwrap();
        wait_for(&server, "POST", "/smgs/api/v1/session").await;

        state.tune("tp_523");
        state.live("tp_523").await.unwrap();

        let requests = server.received_requests().await.unwrap();
        let deleted_a = requests
            .iter()
            .position(|request| {
                request.method.as_str() == "DELETE"
                    && request
                        .headers
                        .get("x-session-id")
                        .is_some_and(|value| value == "sess-a")
            })
            .expect("old session DELETE");
        let get_b = requests
            .iter()
            .position(|request| {
                request.method.as_str() == "GET" && request.url.path().ends_with("/LIVE/523")
            })
            .expect("access control for the new channel");
        assert!(
            deleted_a < get_b,
            "DELETE A must precede access control for B"
        );
        assert_eq!(state.active_document().as_deref(), Some("523"));
    }

    /// A late refresh for the channel the user left must not reopen it or
    /// close the channel that now owns the slot.
    #[tokio::test]
    async fn stale_refresh_does_not_take_the_slot_back() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/lit-ls-config/pub/api/v1/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"app": {"dd": {"v1": "ZG5INSxTMSVDbSN1cFclZQ==", "v2": "YWVzRW5jcnlwdGlvbktleQ=="}}}
            })))
            .mount(&server)
            .await;
        for (document, session) in [("1918", "sess-a"), ("523", "sess-b")] {
            Mock::given(method("GET"))
                .and(path(format!("/pas/api/v1/access/control/LIVE/{document}")))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "data": {
                        "sessionId": session,
                        "heartbeatInterval": 3600,
                        "playbackUrls": {
                            "dashWidewinePlayUrl": "not-encrypted",
                            "dashWidewineLicenseUrl": "not-encrypted"
                        }
                    }
                })))
                .mount(&server)
                .await;
        }
        Mock::given(method("POST"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;

        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        state.live("tp_1918").await.unwrap();
        state.tune("tp_523");
        state.live("tp_523").await.unwrap();
        let mark = server.received_requests().await.unwrap().len();

        assert!(state.refresh_live("tp_1918").await.is_err());
        assert_eq!(state.active_document().as_deref(), Some("523"));

        let requests = server.received_requests().await.unwrap();
        assert!(
            !requests[mark..].iter().any(|request| {
                request.method.as_str() == "GET" && request.url.path().ends_with("/LIVE/1918")
            }),
            "a stale refresh must not resolve the old channel"
        );
        assert!(
            !requests[mark..].iter().any(|request| {
                request.method.as_str() == "DELETE"
                    && request
                        .headers
                        .get("x-session-id")
                        .is_some_and(|value| value == "sess-b")
            }),
            "a stale refresh must not close the active channel"
        );
    }

    #[tokio::test]
    async fn returning_to_a_channel_rotates_its_playback_id() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/lit-ls-config/pub/api/v1/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"app": {"dd": {"v1": "ZG5INSxTMSVDbSN1cFclZQ==", "v2": "YWVzRW5jcnlwdGlvbktleQ=="}}}
            })))
            .mount(&server)
            .await;
        for (document, session) in [("1918", "sess-a"), ("523", "sess-b")] {
            Mock::given(method("GET"))
                .and(path(format!("/pas/api/v1/access/control/LIVE/{document}")))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "data": {
                        "sessionId": session,
                        "heartbeatInterval": 3600,
                        "playbackUrls": {
                            "dashWidewinePlayUrl": "not-encrypted",
                            "dashWidewineLicenseUrl": "not-encrypted"
                        }
                    }
                })))
                .mount(&server)
                .await;
        }
        Mock::given(method("POST"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;

        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        let first = state.tune("tp_1918").unwrap();
        state.live("tp_1918").await.unwrap();
        assert!(state.playback_matches("tp_1918", &first));

        state.tune("tp_523");
        state.live("tp_523").await.unwrap();
        let second = state.tune("tp_1918").unwrap();
        state.live("tp_1918").await.unwrap();

        assert_ne!(first, second);
        assert!(!state.playback_matches("tp_1918", &first));
        assert!(state.playback_matches("tp_1918", &second));
    }

    /// A licence refresh keeps the tune id so the player's existing manifest
    /// and licence routes stay valid.
    #[tokio::test]
    async fn licence_refresh_preserves_the_tune_id() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/lit-ls-config/pub/api/v1/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"app": {"dd": {"v1": "ZG5INSxTMSVDbSN1cFclZQ==", "v2": "YWVzRW5jcnlwdGlvbktleQ=="}}}
            })))
            .mount(&server)
            .await;
        let plays = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let plays_for_response = plays.clone();
        Mock::given(method("GET"))
            .and(path("/pas/api/v1/access/control/LIVE/1918"))
            .respond_with(move |_request: &wiremock::Request| {
                let call = plays_for_response.fetch_add(1, Ordering::SeqCst);
                let session = if call == 0 { "sess-a" } else { "sess-b" };
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "data": {
                        "sessionId": session,
                        "heartbeatInterval": 3600,
                        "tokenInfo": {"token": format!("token-{session}")},
                        "playbackUrls": {
                            "dashWidewinePlayUrl": "not-encrypted",
                            "dashWidewineLicenseUrl": "not-encrypted"
                        }
                    }
                }))
            })
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;

        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        let tune = state.tune("tp_1918").unwrap();
        state.live("tp_1918").await.unwrap();
        let before = state.current_playback_id("tp_1918").unwrap();
        assert_eq!(before, tune);

        let first_session = state.license_session("tp_1918").await.unwrap();
        state
            .refresh_license("tp_1918", &first_session.session_id, Some(&tune))
            .await
            .unwrap();

        assert_eq!(
            state.current_playback_id("tp_1918").as_deref(),
            Some(tune.as_str()),
            "a licence refresh must not rotate the route id"
        );
        assert_eq!(plays.load(Ordering::SeqCst), 2);
    }

    /// A refresh that lands after the user picked another channel must end the
    /// session it just opened instead of publishing it for the old channel.
    #[tokio::test]
    async fn licence_refresh_rejects_a_superseded_tune() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/lit-ls-config/pub/api/v1/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"app": {"dd": {"v1": "ZG5INSxTMSVDbSN1cFclZQ==", "v2": "YWVzRW5jcnlwdGlvbktleQ=="}}}
            })))
            .mount(&server)
            .await;
        let plays = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let plays_for_response = plays.clone();
        Mock::given(method("GET"))
            .and(path("/pas/api/v1/access/control/LIVE/1918"))
            .respond_with(move |_request: &wiremock::Request| {
                let call = plays_for_response.fetch_add(1, Ordering::SeqCst);
                let session = if call == 0 { "sess-a" } else { "sess-b" };
                let mut response = ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "data": {
                        "sessionId": session,
                        "heartbeatInterval": 3600,
                        "tokenInfo": {"token": format!("token-{session}")},
                        "playbackUrls": {
                            "dashWidewinePlayUrl": "not-encrypted",
                            "dashWidewineLicenseUrl": "not-encrypted"
                        }
                    }
                }));
                if call > 0 {
                    // Keep the refresh in flight while another tune lands.
                    response = response.set_delay(Duration::from_millis(200));
                }
                response
            })
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;

        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        state.live("tp_1918").await.unwrap();
        let playback_id = state.current_playback_id("tp_1918").unwrap();
        let first = state.license_session("tp_1918").await.unwrap();

        let supersede = async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            state.tune("tp_523");
        };
        let (refresh, ()) = tokio::join!(
            state.refresh_license("tp_1918", &first.session_id, Some(&playback_id)),
            supersede
        );
        let error = refresh.unwrap_err().to_string();
        assert!(error.contains("playback changed"), "{error}");
        assert_ne!(state.active_document().as_deref(), Some("1918"));
        assert!(
            state
                .resolved
                .read()
                .unwrap()
                .get("1918")
                .is_some_and(|(resolved, _)| resolved.session_id == "sess-a"),
            "the superseded session must not be published"
        );
        assert!(
            server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .any(|request| request.method.as_str() == "DELETE"
                    && request
                        .headers
                        .get("x-session-id")
                        .is_some_and(|value| value == "sess-b")),
            "the late session must be ended"
        );
    }

    /// A request that was queued behind a custom-channel selection must not
    /// open a session with a recreated intent.
    #[tokio::test]
    async fn cancel_tune_stops_a_queued_resolve_before_access_control() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/pas/api/v1/access/control/LIVE/1918"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": {}})))
            .expect(0)
            .mount(&server)
            .await;

        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        let selected = state.tune("tp_1918").unwrap();
        assert!(state.selection_matches("tp_1918", &selected));
        state.cancel_tune();
        assert!(!state.selection_matches("tp_1918", &selected));
        assert!(state
            .ensure_playback("1918", PlaybackMode::Tune, Some(&selected))
            .await
            .is_none());
        server.verify().await;
    }

    /// A 130402 on a deliberate tune gives the provider a moment, but a
    /// failed DELETE must not stop the new channel from resolving.
    #[tokio::test]
    async fn failed_old_session_delete_does_not_block_the_new_channel() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/lit-ls-config/pub/api/v1/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"app": {"dd": {"v1": "ZG5INSxTMSVDbSN1cFclZQ==", "v2": "YWVzRW5jcnlwdGlvbktleQ=="}}}
            })))
            .mount(&server)
            .await;
        for (document, session) in [("1918", "sess-a"), ("523", "sess-b")] {
            Mock::given(method("GET"))
                .and(path(format!("/pas/api/v1/access/control/LIVE/{document}")))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "data": {
                        "sessionId": session,
                        "heartbeatInterval": 3600,
                        "playbackUrls": {
                            "dashWidewinePlayUrl": "not-encrypted",
                            "dashWidewineLicenseUrl": "not-encrypted"
                        }
                    }
                })))
                .mount(&server)
                .await;
        }
        Mock::given(method("POST"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        state.live("tp_1918").await.unwrap();
        state.tune("tp_523");
        state.live("tp_523").await.unwrap();

        assert_eq!(state.active_document().as_deref(), Some("523"));
        assert!(state.playback_matches("tp_523", &state.current_playback_id("tp_523").unwrap()));
    }

    /// A license request for the channel that was playing before a switch
    /// must not take the slot or close the channel that owns it now.
    #[tokio::test]
    async fn stale_license_request_cannot_steal_the_active_channel() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;

        let state = TataplayState::new(true);
        let client = client_for(&server);
        state.set_client_for_test(client.clone());
        let active = ResolvedPlayback {
            session_id: "sess-live".into(),
            heartbeat_interval: Some(3600),
            ..ResolvedPlayback::default()
        };
        state.start_heartbeat(&client, "tp_1", &active);
        wait_for(&server, "POST", "/smgs/api/v1/session").await;

        let error = state.license_session("tp_2").await.unwrap_err().to_string();
        assert!(
            error.contains("not the active playback")
                || error.contains("no longer the intended playback"),
            "{error}"
        );
        let error = state
            .refresh_license("tp_2", "sess-stale", None)
            .await
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("not the active playback")
                || error.contains("no longer the intended playback"),
            "{error}"
        );

        assert_eq!(state.active_document().as_deref(), Some("tp_1"));
        assert_eq!(
            server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .filter(|request| request.method.as_str() == "DELETE")
                .count(),
            0,
            "a stale license retry must not end the active session"
        );
    }

    async fn wait_for(server: &MockServer, method: &str, path: &str) {
        for _ in 0..100 {
            let seen = server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .filter(|request| request.method.as_str() == method && request.url.path() == path)
                .count();
            if seen > 0 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("no {method} {path} request arrived");
    }

    #[tokio::test]
    async fn refresh_replaces_the_retained_client() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/rest-api/pub/api/v3/refreshToken"))
            .and(header("authorization", "bearer token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "accessToken": "new-access",
                    "refreshToken": "new-refresh",
                    "subscriberId": "sub-1",
                    "userDetails": {"sid": "sid-1"}
                }
            })))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        store
            .set(
                crate::tataplay::STORE_KEY_CREDENTIALS,
                &serde_json::to_string(&Credentials {
                    access_token: "token".into(),
                    refresh_token: "refresh".into(),
                    subscriber_id: "sub-1".into(),
                    sid: "sid-1".into(),
                })
                .unwrap(),
            )
            .unwrap();
        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));

        state.refresh(&store).await.unwrap();

        assert_eq!(
            state.client().unwrap().credentials().unwrap().access_token,
            "new-access"
        );
        assert_eq!(
            Credentials::load(&store).unwrap().access_token,
            "new-access"
        );
        server.verify().await;
    }

    #[tokio::test]
    async fn heartbeat_stops_when_playback_goes_idle() {
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
                    "sessionId": "sess-idle",
                    "heartbeatInterval": 1,
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
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .expect(1)
            .mount(&server)
            .await;
        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        state.live("tp_1918").await.unwrap();
        wait_for(&server, "POST", "/smgs/api/v1/session").await;

        wait_for(&server, "DELETE", "/smgs/api/v1/session").await;
        tokio::time::sleep(Duration::from_millis(1200)).await;
        let posts = server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|request| request.method.as_str() == "POST")
            .count();
        assert_eq!(posts, 1, "the stopped heartbeat must not keep beating");
        server.verify().await;
    }

    #[tokio::test]
    async fn playback_registers_and_closes_the_upstream_session() {
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
                    "sessionId": "sess-1",
                    "heartbeatInterval": 3600,
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
        Mock::given(method("DELETE"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;
        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        state.live("tp_1918").await.unwrap();
        wait_for(&server, "POST", "/smgs/api/v1/session").await;
        let post = server
            .received_requests()
            .await
            .unwrap()
            .into_iter()
            .find(|request| request.method.as_str() == "POST")
            .unwrap();
        assert_eq!(
            post.headers.get("x-session-id").unwrap().to_str().unwrap(),
            "sess-1"
        );
        state.invalidate_playback("tp_1918").await;
        wait_for(&server, "DELETE", "/smgs/api/v1/session").await;
    }

    #[tokio::test]
    async fn stopping_after_a_client_change_ends_the_original_session() {
        let original = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&original)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/smgs/api/v1/session"))
            .and(header("x-session-id", "sess-original"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .expect(1)
            .mount(&original)
            .await;
        let replacement = MockServer::start().await;

        let state = TataplayState::new(true);
        let original_client = client_for(&original);
        state.set_client_for_test(original_client.clone());
        let resolved = ResolvedPlayback {
            session_id: "sess-original".into(),
            heartbeat_interval: Some(3600),
            ..ResolvedPlayback::default()
        };
        state.start_heartbeat(&original_client, "tp_1", &resolved);
        wait_for(&original, "POST", "/smgs/api/v1/session").await;

        // Logout and verify_otp replace the active client before clear() stops
        // the heartbeat; cleanup must still reach the original session.
        state.set_client_for_test(client_for(&replacement));
        state.clear();

        wait_for(&original, "DELETE", "/smgs/api/v1/session").await;
        assert!(
            replacement.received_requests().await.unwrap().is_empty(),
            "cleanup must not use the replacement client"
        );
        original.verify().await;
    }

    #[tokio::test]
    async fn replacing_a_heartbeat_ends_the_previous_session() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/smgs/api/v1/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/smgs/api/v1/session"))
            .and(header("x-session-id", "sess-old"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 0})))
            .expect(1)
            .mount(&server)
            .await;
        let state = TataplayState::new(true);
        let client = client_for(&server);
        let first = ResolvedPlayback {
            session_id: "sess-old".into(),
            heartbeat_interval: Some(3600),
            ..ResolvedPlayback::default()
        };
        state.start_heartbeat(&client, "tp_1", &first);
        wait_for(&server, "POST", "/smgs/api/v1/session").await;

        let second = ResolvedPlayback {
            session_id: "sess-new".into(),
            heartbeat_interval: Some(3600),
            ..ResolvedPlayback::default()
        };
        state.start_heartbeat(&client, "tp_2", &second);

        wait_for(&server, "DELETE", "/smgs/api/v1/session").await;
        server.verify().await;
    }
}
