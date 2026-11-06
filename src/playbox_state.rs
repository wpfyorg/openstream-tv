//! PlayboxTV routing/caching layer: catalogue, live playback, entitlement and
//! the playback headers the media/license proxy must forward.
//!
//! Protocol details live in `src/playbox.rs`. Everything here is local state:
//! TTLs, per-account generation guards and cache invalidation. No endpoint or
//! wire shape is defined in this file.

use crate::keyed_locks::KeyedLocks;
use crate::playbox::{
    content_id, ActivePacks, Client, Credentials, Device, Item, ResolvedPlayback,
};
use crate::television::{Channel, LiveUrlOutput, Mpd};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

const CATALOGUE_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const PLAYBACK_TTL: Duration = Duration::from_secs(5 * 60);
const ENTITLEMENT_TTL: Duration = Duration::from_secs(10 * 60);

pub struct PlayboxState {
    config_enabled: bool,
    client: RwLock<Option<Arc<Client>>>,
    catalogue: RwLock<Vec<Item>>,
    catalogue_fetched_at: RwLock<Option<Instant>>,
    resolved: RwLock<HashMap<String, (ResolvedPlayback, Instant)>>,
    entitlement: RwLock<Option<(bool, Instant)>>,
    /// Bumped under `commit` whenever account-scoped caches are cleared. A
    /// fetch captures it first and commits only if it is unchanged.
    generation: std::sync::atomic::AtomicU64,
    commit: Mutex<()>,
    locks: KeyedLocks,
    #[cfg(test)]
    test_base: RwLock<Option<String>>,
}

impl PlayboxState {
    pub fn new(config_enabled: bool) -> PlayboxState {
        PlayboxState {
            config_enabled,
            client: RwLock::new(None),
            catalogue: RwLock::new(Vec::new()),
            catalogue_fetched_at: RwLock::new(None),
            resolved: RwLock::new(HashMap::new()),
            entitlement: RwLock::new(None),
            generation: std::sync::atomic::AtomicU64::new(0),
            commit: Mutex::new(()),
            locks: KeyedLocks::default(),
            #[cfg(test)]
            test_base: RwLock::new(None),
        }
    }

    #[cfg(test)]
    pub fn set_base_for_test(&self, base: &str) {
        *self.test_base.write().unwrap() = Some(base.to_string());
    }

    fn client_for_http(
        &self,
        http: &reqwest::Client,
        store: &crate::store::Store,
    ) -> anyhow::Result<Client> {
        let device = Device::load_or_create(store)?;
        #[allow(unused_mut)] // `mut` is only needed by the cfg(test) base override.
        let mut client = Client::new(http.clone(), device);
        #[cfg(test)]
        if let Some(base) = self.test_base.read().unwrap().as_deref() {
            client.set_bases_for_test(base);
        }
        Ok(client)
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
        let Ok(device) = Device::load_or_create(store) else {
            return;
        };
        if let Some(creds) = Credentials::load(store) {
            let client = Client {
                http: http.clone(),
                login_v2: crate::playbox::BASE_LOGIN_V2.to_string(),
                player_v2: crate::playbox::BASE_PLAYER_V2.to_string(),
                user_v1: crate::playbox::BASE_USER_V1.to_string(),
                catalogue: crate::playbox::BASE_CATALOGUE.to_string(),
                consume: crate::playbox::BASE_CONSUME.to_string(),
                device,
                creds: RwLock::new(creds),
            };
            *self.client.write().unwrap() = Some(Arc::new(client));
        }
    }

    #[cfg(test)]
    pub fn set_client_for_test(&self, client: Arc<Client>) {
        *self.client.write().unwrap() = Some(client);
    }

    fn generation(&self) -> u64 {
        self.generation.load(std::sync::atomic::Ordering::SeqCst)
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
        let _guard = self.commit.lock().unwrap();
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        *self.catalogue.write().unwrap() = Vec::new();
        *self.catalogue_fetched_at.write().unwrap() = None;
        self.resolved.write().unwrap().clear();
        *self.entitlement.write().unwrap() = None;
    }

    pub fn logout(&self, store: &crate::store::Store) -> anyhow::Result<()> {
        self.clear();
        *self.client.write().unwrap() = None;
        crate::playbox::delete_credentials(store)?;
        Ok(())
    }

    /// Sends an OTP before a session exists, so it builds a client from the
    /// persisted device exactly like `verify_otp` does.
    pub async fn send_otp(
        &self,
        http: &reqwest::Client,
        store: &crate::store::Store,
        number: &str,
    ) -> anyhow::Result<bool> {
        let client = self.client_for_http(http, store)?;
        let envelope = client.send_otp(number).await?;
        Ok(envelope.status_code == 200)
    }

    pub async fn verify_otp(
        &self,
        http: &reqwest::Client,
        store: &crate::store::Store,
        number: &str,
        otp: &str,
        on_installed: impl FnOnce(),
    ) -> anyhow::Result<bool> {
        let client = self.client_for_http(http, store)?;
        let envelope = client.verify_otp(number, otp).await?;
        if envelope.status_code != 200 || envelope.session().is_none() {
            return Ok(false);
        }
        // Rotate the context at the exact moment the new account replaces the
        // old one, before any account-scoped cache can be reused.
        on_installed();
        if !client.install_session(number, &envelope, store)? {
            return Ok(false);
        }
        // Reload through `init` so the state holds the same client shape as a
        // fresh start.
        self.init(http, store);
        Ok(true)
    }

    /// Returns whether a fresh catalogue is available. A failed fetch leaves
    /// the previous generation dropped rather than serving another account's
    /// list, and the caller can report the upstream failure.
    pub async fn ensure_catalogue(&self) -> bool {
        let Some(client) = self.client() else {
            return false;
        };
        let fresh = self
            .catalogue_fetched_at
            .read()
            .unwrap()
            .is_some_and(|at| at.elapsed() < CATALOGUE_TTL);
        if fresh {
            return true;
        }
        let _guard = self.locks.lock("catalogue").await;
        let fresh = self
            .catalogue_fetched_at
            .read()
            .unwrap()
            .is_some_and(|at| at.elapsed() < CATALOGUE_TTL);
        if fresh {
            return true;
        }
        let generation = self.generation();
        match client.channels().await {
            Ok(items) => {
                let now = Instant::now();
                self.commit_if_current(generation, || {
                    *self.catalogue.write().unwrap() = items;
                    *self.catalogue_fetched_at.write().unwrap() = Some(now);
                })
                .is_some()
            }
            Err(_) => false,
        }
    }

    pub fn channels(&self) -> Vec<Channel> {
        self.catalogue
            .read()
            .unwrap()
            .iter()
            .map(crate::playbox::to_television_channel)
            .collect()
    }

    pub fn contains(&self, channel_id: &str) -> bool {
        let Some(content) = content_id(channel_id) else {
            return false;
        };
        self.catalogue
            .read()
            .unwrap()
            .iter()
            .any(|item| item.content_id() == content)
    }

    async fn ensure_playback(&self, content: &str) -> Option<ResolvedPlayback> {
        if let Some((resolved, at)) = self.resolved.read().unwrap().get(content) {
            if at.elapsed() < PLAYBACK_TTL {
                return Some(resolved.clone());
            }
        }
        let client = self.client()?;
        let _guard = self.locks.lock(&format!("playback:{content}")).await;
        if let Some((resolved, at)) = self.resolved.read().unwrap().get(content) {
            if at.elapsed() < PLAYBACK_TTL {
                return Some(resolved.clone());
            }
        }
        let generation = self.generation();
        let item = self
            .catalogue
            .read()
            .unwrap()
            .iter()
            .find(|item| item.content_id() == content)
            .cloned();
        let type_id = item.as_ref().map(Item::playback_type_id).unwrap_or(0);
        let resolved = client.resolve(content, type_id).await.ok()?;
        let now = Instant::now();
        let clone = resolved.clone();
        self.commit_if_current(generation, || {
            self.resolved
                .write()
                .unwrap()
                .insert(content.to_string(), (resolved, now));
        })?;
        Some(clone)
    }

    /// Resolves a `pb_` channel to the shared playback shape. The 60-second
    /// live cache follows the same convention as JioTV+.
    pub async fn live(&self, channel_id: &str) -> anyhow::Result<LiveUrlOutput> {
        let content = content_id(channel_id)
            .ok_or_else(|| anyhow::anyhow!("playbox: not a playbox channel"))?;
        self.ensure_catalogue().await;
        let resolved = self
            .ensure_playback(content)
            .await
            .ok_or_else(|| anyhow::anyhow!("playbox: playback could not be resolved"))?;
        let mut out = LiveUrlOutput {
            result: "OK".to_string(),
            message: String::new(),
            ..Default::default()
        };
        if resolved.is_dash {
            out.mpd = Mpd {
                auto: resolved.playable.clone(),
                key: resolved.license_url.clone(),
                ..Default::default()
            };
        } else {
            out.bitrates.auto = resolved.playable.clone();
        }
        out.key_url = resolved.license_url.clone();
        out.is_drm = !resolved.license_url.is_empty();
        Ok(out)
    }

    /// Headers the media proxy must add for a resolved Playbox channel.
    pub async fn media_headers(&self, channel_id: &str) -> Vec<(String, String)> {
        let Some(content) = content_id(channel_id) else {
            return Vec::new();
        };
        match self.ensure_playback(content).await {
            Some(resolved) => resolved.media_headers,
            None => Vec::new(),
        }
    }

    /// Headers the license proxy must add for a resolved Playbox channel.
    pub async fn license_headers(&self, channel_id: &str) -> Vec<(String, String)> {
        let Some(content) = content_id(channel_id) else {
            return Vec::new();
        };
        match self.ensure_playback(content).await {
            Some(resolved) => resolved.license_headers,
            None => Vec::new(),
        }
    }

    pub fn is_drm(&self, channel_id: &str) -> bool {
        let Some(content) = content_id(channel_id) else {
            return false;
        };
        self.resolved
            .read()
            .unwrap()
            .get(content)
            .map(|(resolved, _)| !resolved.license_url.is_empty())
            .unwrap_or(false)
    }

    pub async fn subscription(&self) -> Option<bool> {
        if let Some((entitled, at)) = *self.entitlement.read().unwrap() {
            if at.elapsed() < ENTITLEMENT_TTL {
                return Some(entitled);
            }
        }
        let client = self.client()?;
        let packs: ActivePacks = client.subscription().await.ok()?;
        let entitled = packs.entitled();
        *self.entitlement.write().unwrap() = Some((entitled, Instant::now()));
        Some(entitled)
    }

    pub fn account_fingerprint(&self) -> String {
        let client = self.client.read().unwrap();
        let Some(client) = client.as_ref() else {
            return String::new();
        };
        let creds = client.creds.read().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(b"playbox-account-v1");
        hasher.update(creds.phone.as_bytes());
        hasher.update(b"\0");
        hasher.update(creds.token.as_bytes());
        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playbox::Client;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn client_for(server: &MockServer) -> Arc<Client> {
        let mut client = Client::new(reqwest::Client::new(), Device::new());
        client.set_bases_for_test(&server.uri());
        *client.creds.write().unwrap() = Credentials {
            phone: "9876543210".into(),
            token: "opaque-session-token".into(),
            customer_id: "cust-1".into(),
        };
        Arc::new(client)
    }

    #[tokio::test]
    async fn catalogue_and_live_are_cached_and_cleared_on_logout() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/catalogue/v3/page/livetv2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "statusCode": 200,
                "livetv": [{"id": "ch1", "name": "One", "typeId": 106}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/consume/v1/playback/resolve"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": 200,
                "data": {"url": {
                    "playable": "https://cdn.example/ch1.m3u8",
                    "token": "drm-token",
                    "headers": {"user_id": "u1", "userAgent": "UA"}
                }}
            })))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let state = PlayboxState::new(true);
        state.set_client_for_test(client_for(&server));

        state.ensure_catalogue().await;
        assert!(state.contains("pb_ch1"));
        assert_eq!(state.channels()[0].name, "One");

        let live = state.live("pb_ch1").await.unwrap();
        assert_eq!(live.bitrates.auto, "https://cdn.example/ch1.m3u8");
        let headers = state.media_headers("pb_ch1").await;
        assert!(headers.contains(&("user_id".into(), "u1".into())));

        state.logout(&store).unwrap();
        assert!(!state.connected());
        assert!(!state.contains("pb_ch1"));
        assert!(state.media_headers("pb_ch1").await.is_empty());
    }

    #[tokio::test]
    async fn a_generation_change_drops_an_in_flight_catalogue() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/catalogue/v3/page/livetv2"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"livetv": [{"id": "ch1"}]}))
                    .set_delay(Duration::from_millis(200)),
            )
            .mount(&server)
            .await;
        let state = Arc::new(PlayboxState::new(true));
        state.set_client_for_test(client_for(&server));
        let s2 = state.clone();
        let handle = tokio::spawn(async move { s2.ensure_catalogue().await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        state.clear();
        handle.await.unwrap();
        assert!(!state.contains("pb_ch1"));
    }

    #[tokio::test]
    async fn account_fingerprint_changes_with_the_token() {
        let server = MockServer::start().await;
        let state = PlayboxState::new(true);
        state.set_client_for_test(client_for(&server));
        let a = state.account_fingerprint();
        {
            let client = state.client().unwrap();
            client.creds.write().unwrap().token = "different".into();
        }
        assert_ne!(a, state.account_fingerprint());
    }
}
