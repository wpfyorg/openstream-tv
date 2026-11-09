//! Tata Play routing/caching layer: catalogue, decryption keys, live playback
//! and the headers the media/license proxy must forward.
//!
//! Protocol details live in `src/tataplay.rs`. Everything here is local state:
//! TTLs, per-account generation guards and cache invalidation.

use crate::keyed_locks::KeyedLocks;
use crate::tataplay::{document_id, AppKeys, Client, ResolvedPlayback};
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

pub struct TataplayState {
    config_enabled: bool,
    client: RwLock<Option<Arc<Client>>>,
    catalogue: RwLock<Vec<Channel>>,
    catalogue_fetched_at: RwLock<Option<Instant>>,
    catalogue_failed_at: RwLock<Option<Instant>>,
    keys: RwLock<Option<(AppKeys, Instant)>>,
    resolved: RwLock<HashMap<String, (ResolvedPlayback, Instant)>>,
    /// Last media/license access, so the heartbeat can stop when playback ends.
    last_activity: Arc<Mutex<Instant>>,
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
            last_activity: Arc::new(Mutex::new(Instant::now())),
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
                    *self.catalogue.write().unwrap() = channels;
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

    async fn app_keys(&self) -> Option<AppKeys> {
        if let Some((keys, at)) = self.keys.read().unwrap().as_ref() {
            if at.elapsed() < KEYS_TTL {
                return Some(keys.clone());
            }
        }
        let client = self.client()?;
        let keys = client.app_config().await.ok()?;
        *self.keys.write().unwrap() = Some((keys.clone(), Instant::now()));
        Some(keys)
    }

    async fn ensure_playback(&self, document: &str) -> Option<ResolvedPlayback> {
        *self.last_activity.lock().unwrap() = Instant::now();
        if let Some((resolved, at)) = self.resolved.read().unwrap().get(document) {
            if at.elapsed() < PLAYBACK_TTL {
                return Some(resolved.clone());
            }
        }
        let generation = self.generation();
        let client = self.client()?;
        let _guard = self.locks.lock(&format!("playback:{document}")).await;
        if self.generation() != generation {
            return None;
        }
        if let Some((resolved, at)) = self.resolved.read().unwrap().get(document) {
            if at.elapsed() < PLAYBACK_TTL {
                return Some(resolved.clone());
            }
        }
        let keys = self.app_keys().await?;
        let resolved = client.play(document, &keys).await.ok()?;
        let now = Instant::now();
        let clone = resolved.clone();
        self.commit_if_current(generation, || {
            self.resolved
                .write()
                .unwrap()
                .insert(document.to_string(), (resolved, now));
        })?;
        self.start_heartbeat(&client, document, &clone);
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
        let document = document_id(channel_id)
            .ok_or_else(|| anyhow::anyhow!("tataplay: not a tataplay channel"))?;
        self.ensure_catalogue().await;
        let resolved = self
            .ensure_playback(document)
            .await
            .ok_or_else(|| anyhow::anyhow!("tataplay: playback could not be resolved"))?;
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
        Ok(out)
    }

    /// Headers the media proxy must add for a resolved Tata Play channel.
    pub async fn media_headers(&self, channel_id: &str) -> Vec<(String, String)> {
        let Some(document) = document_id(channel_id) else {
            return Vec::new();
        };
        match self.ensure_playback(document).await {
            Some(resolved) => resolved.media_headers,
            None => Vec::new(),
        }
    }

    /// Headers the license proxy must add for a resolved Tata Play channel.
    pub async fn license_headers(&self, channel_id: &str) -> Vec<(String, String)> {
        let Some(document) = document_id(channel_id) else {
            return Vec::new();
        };
        match self.ensure_playback(document).await {
            Some(resolved) => resolved.license_headers,
            None => Vec::new(),
        }
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
    async fn catalogue_flattens_rails_and_dedupes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/homescreen/pub/api/v1/lit/hierarchy/LIT_LIVETV"))
            .and(header("appVersion", crate::tataplay::VERSION_NAME))
            .and(query_param("locale", "eng"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {"railId": 1, "contentList": [
                        {"id": 1918, "channelName": "Star Plus", "logo": "a.png", "hdChannel": true},
                        {"id": 1914, "channelName": "Zee TV"}
                    ]},
                    {"railId": 2, "contentList": [
                        {"id": 1918, "channelName": "Star Plus"},
                        {"id": 40, "channelName": "Sony SAB"}
                    ]}
                ]
            })))
            .mount(&server)
            .await;
        let state = TataplayState::new(true);
        state.set_client_for_test(client_for(&server));
        assert!(state.ensure_catalogue().await);
        let channels = state.channels();
        assert_eq!(channels.len(), 3);
        assert!(state.contains(&channel_id("1918")));
        assert!(state.contains("tp_40"));
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
