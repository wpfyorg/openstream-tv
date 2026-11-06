//! VZY routing/caching layer. Only the free tier is exposed: the anonymous
//! Kaltura session can read `EntitlementType == "Free"` channels, resolve
//! their clear HLS/DASH manifests and fetch EPG. Premium and login-required
//! assets are filtered out before they can reach the catalogue or playlist.

use crate::keyed_locks::KeyedLocks;
use crate::television::{Channel, LiveUrlOutput, Mpd};
use crate::vzy::{
    channel_id, midnight_utc, parse_channel_id, to_television_channel, Asset, Client, Device,
    Session,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};

const CATALOGUE_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const CATALOGUE_RETRY_BACKOFF: Duration = Duration::from_secs(15);
const PLAYBACK_TTL: Duration = Duration::from_secs(5 * 60);

pub struct VzyState {
    config_enabled: bool,
    client: RwLock<Option<Arc<Client>>>,
    bootstrapped: std::sync::atomic::AtomicBool,
    catalogue: RwLock<Vec<Asset>>,
    catalogue_fetched_at: RwLock<Option<Instant>>,
    catalogue_failed_at: RwLock<Option<Instant>>,
    playback: RwLock<HashMap<String, (LiveUrlOutput, Instant)>>,
    generation: std::sync::atomic::AtomicU64,
    commit: Mutex<()>,
    locks: KeyedLocks,
}

impl VzyState {
    pub fn new(config_enabled: bool) -> VzyState {
        VzyState {
            config_enabled,
            client: RwLock::new(None),
            bootstrapped: std::sync::atomic::AtomicBool::new(false),
            catalogue: RwLock::new(Vec::new()),
            catalogue_fetched_at: RwLock::new(None),
            catalogue_failed_at: RwLock::new(None),
            playback: RwLock::new(HashMap::new()),
            generation: std::sync::atomic::AtomicU64::new(0),
            commit: Mutex::new(()),
            locks: KeyedLocks::default(),
        }
    }

    pub fn enabled(&self) -> bool {
        self.config_enabled
    }

    /// Anonymous sessions are created lazily, so "connected" means the client
    /// exists and the feature is on.
    pub fn connected(&self) -> bool {
        self.config_enabled && self.client.read().unwrap().is_some()
    }

    pub fn init(&self, http: &reqwest::Client, store: &crate::store::Store) {
        if !self.config_enabled {
            return;
        }
        let Ok(device) = Device::load_or_create(store) else {
            return;
        };
        let client = Client::new(http.clone(), device);
        if let Some(session) = Session::load(store) {
            *client.session.write().unwrap() = Some(session);
        }
        *self.client.write().unwrap() = Some(Arc::new(client));
    }

    #[cfg(test)]
    pub fn set_client_for_test(&self, client: Arc<Client>) {
        *self.client.write().unwrap() = Some(client);
        self.bootstrapped
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn client(&self) -> Option<Arc<Client>> {
        self.client.read().unwrap().clone()
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

    pub fn clear(&self) {
        let _guard = self.commit.lock().unwrap();
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        *self.catalogue.write().unwrap() = Vec::new();
        *self.catalogue_fetched_at.write().unwrap() = None;
        *self.catalogue_failed_at.write().unwrap() = None;
        self.playback.write().unwrap().clear();
    }

    pub fn logout(&self, store: &crate::store::Store) -> anyhow::Result<()> {
        crate::vzy::delete_session(store)?;
        if let Some(client) = self.client() {
            *client.session.write().unwrap() = None;
        }
        self.bootstrapped
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.clear();
        Ok(())
    }

    async fn ensure_bootstrap(&self, client: &Client, store: &crate::store::Store) -> bool {
        if self.bootstrapped.load(std::sync::atomic::Ordering::SeqCst) {
            return true;
        }
        let _guard = self.locks.lock("bootstrap").await;
        if self.bootstrapped.load(std::sync::atomic::Ordering::SeqCst) {
            return true;
        }
        if client.bootstrap(store).await.is_ok() {
            self.bootstrapped
                .store(true, std::sync::atomic::Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Loads every page of the free-tier catalogue. Returns false when the
    /// upstream session or catalogue is unavailable.
    pub async fn ensure_catalogue(&self, store: &crate::store::Store) -> bool {
        let Some(client) = self.client() else {
            return false;
        };
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
        let generation = self.generation();
        if !self.ensure_bootstrap(&client, store).await {
            let _ = self.commit_if_current(generation, || {
                *self.catalogue_failed_at.write().unwrap() = Some(Instant::now());
            });
            return false;
        }
        let mut all = Vec::new();
        let mut fetched_total = 0i64;
        let mut page = 1;
        loop {
            match client.channels_page(store, page, 200).await {
                Ok((items, total)) => {
                    let fetched = items.len() as i64;
                    fetched_total += fetched;
                    all.extend(items.into_iter().filter(Asset::is_free));
                    if fetched == 0 || fetched_total >= total || page >= 20 {
                        break;
                    }
                    page += 1;
                }
                Err(_) => {
                    let _ = self.commit_if_current(generation, || {
                        *self.catalogue_failed_at.write().unwrap() = Some(Instant::now());
                    });
                    return false;
                }
            }
        }
        let now = Instant::now();
        self.commit_if_current(generation, || {
            *self.catalogue.write().unwrap() = all;
            *self.catalogue_fetched_at.write().unwrap() = Some(now);
            *self.catalogue_failed_at.write().unwrap() = None;
        })
        .is_some()
    }

    pub fn channels(&self) -> Vec<Channel> {
        self.catalogue
            .read()
            .unwrap()
            .iter()
            .map(to_television_channel)
            .collect()
    }

    pub fn contains(&self, channel_id: &str) -> bool {
        let Some(id) = parse_channel_id(channel_id) else {
            return false;
        };
        self.catalogue.read().unwrap().iter().any(|a| a.id == id)
    }

    /// Resolve a free channel to the shared playback shape. HLS is preferred
    /// because every free-tier asset verified live exposes a clear HLS
    /// rendition; DASH is used only when no HLS is present.
    pub async fn live(
        &self,
        channel_id: &str,
        store: &crate::store::Store,
    ) -> anyhow::Result<LiveUrlOutput> {
        let id = parse_channel_id(channel_id)
            .ok_or_else(|| anyhow::anyhow!("vzy: not a vzy channel"))?;
        self.ensure_catalogue(store).await;
        if let Some((out, at)) = self.playback.read().unwrap().get(channel_id) {
            if at.elapsed() < PLAYBACK_TTL {
                return Ok(out.clone());
            }
        }
        let client = self
            .client()
            .ok_or_else(|| anyhow::anyhow!("vzy is not enabled"))?;
        let _guard = self.locks.lock(&format!("playback:{id}")).await;
        if let Some((out, at)) = self.playback.read().unwrap().get(channel_id) {
            if at.elapsed() < PLAYBACK_TTL {
                return Ok(out.clone());
            }
        }
        let playback = client.playback(store, id).await?;
        let mut out = LiveUrlOutput {
            result: "OK".to_string(),
            ..Default::default()
        };
        if !playback.hls.is_empty() {
            out.bitrates.auto = playback.hls;
        } else {
            out.mpd = Mpd {
                auto: playback.dash,
                ..Default::default()
            };
        }
        let now = Instant::now();
        let generation = self.generation();
        let cached = out.clone();
        self.commit_if_current(generation, || {
            self.playback
                .write()
                .unwrap()
                .insert(channel_id.to_string(), (cached, now));
        });
        Ok(out)
    }

    pub async fn epg_source(
        &self,
        store: &crate::store::Store,
    ) -> anyhow::Result<(Vec<(String, String)>, Vec<crate::epg::XmlProgramme>)> {
        self.ensure_catalogue(store).await;
        let client = self
            .client()
            .ok_or_else(|| anyhow::anyhow!("vzy is not enabled"))?;
        let channels = self.catalogue.read().unwrap().clone();
        let ids: Vec<i64> = channels.iter().map(|a| a.id).collect();
        let guide = client
            .epg(
                store,
                &ids,
                midnight_utc(SystemTime::now()),
                crate::vzy::local_offset_minutes(),
            )
            .await?;
        let mut sources = Vec::new();
        let mut programmes = Vec::new();
        for asset in &channels {
            let ch_id = channel_id(&asset.id.to_string());
            sources.push((ch_id.clone(), asset.name.clone()));
            if let Some(progs) = guide.get(&asset.id.to_string()) {
                for p in progs {
                    programmes.push(crate::epg::XmlProgramme {
                        channel: ch_id.clone(),
                        start: crate::epg::format_xmltv_time(seconds_to_millis(p.start_date)),
                        stop: crate::epg::format_xmltv_time(seconds_to_millis(p.end_date)),
                        title: p.name.clone(),
                        desc: p.description.clone(),
                        category: String::new(),
                        icon: String::new(),
                    });
                }
            }
        }
        Ok((sources, programmes))
    }

    pub fn account_fingerprint(&self) -> String {
        let udid = self
            .client()
            .map(|c| c.device.udid.clone())
            .unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(b"vzy-device-v1");
        hasher.update(udid.as_bytes());
        hex::encode(hasher.finalize())
    }
}

fn seconds_to_millis(value: i64) -> i64 {
    if value.abs() < 10_000_000_000 {
        value * 1000
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn client_for(server: &MockServer) -> Arc<Client> {
        let mut client = Client::new(reqwest::Client::new(), Device::new());
        client.set_bases_for_test(&server.uri(), &format!("{}/api_v3", server.uri()));
        client.epg_base = format!("{}/api_v3/", server.uri());
        Arc::new(client)
    }

    async fn catalogue_server() -> MockServer {
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
            .and(body_string_contains("KalturaChannelFilter"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"totalCount": 3, "objects": [
                    {"id": 1, "name": "Free One", "metas": {"EntitlementType": {"value": "Free"},
                        "ChannelNumber": {"value": 1.0}}, "images": [{"imageTypeName": "16x9", "url": "https://img/1.png"}]},
                    {"id": 2, "name": "Paid Two", "metas": {"EntitlementType": {"value": "Premium"}}},
                    {"id": 3, "name": "Free Three", "metas": {"EntitlementType": {"value": "Free"}}}
                ]}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/asset/action/get"))
            .and(body_string_contains("\"id\":1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"id": 1, "name": "Free One",
                    "metas": {"EntitlementType": {"value": "Free"}},
                    "mediaFiles": [{"url": "https://cdn/free.m3u8"}]}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/asset/action/get"))
            .and(body_string_contains("\"id\":2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"id": 2, "name": "Paid Two",
                    "metas": {"EntitlementType": {"value": "Premium"}},
                    "mediaFiles": [{"url": "https://cdn/paid.m3u8"}]}
            })))
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn only_free_channels_enter_the_catalogue_and_play() {
        let server = catalogue_server().await;
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let state = VzyState::new(true);
        state.set_client_for_test(client_for(&server));

        assert!(state.ensure_catalogue(&store).await);
        let channels = state.channels();
        assert_eq!(channels.len(), 2);
        assert!(state.contains("vzy_1"));
        assert!(state.contains("vzy_3"));
        assert!(!state.contains("vzy_2"), "premium must not be advertised");

        let live = state.live("vzy_1", &store).await.unwrap();
        assert_eq!(live.bitrates.auto, "https://cdn/free.m3u8");
        assert_eq!(
            state.live("vzy_2", &store).await.unwrap_err().to_string(),
            "vzy: asset 2 is not in the free tier"
        );
    }

    #[tokio::test]
    async fn catalogue_failure_uses_short_backoff() {
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
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let state = VzyState::new(true);
        state.set_client_for_test(client_for(&server));

        assert!(!state.ensure_catalogue(&store).await);
        assert!(!state.ensure_catalogue(&store).await);
        assert!(state.catalogue_failed_at.read().unwrap().is_some());
    }

    #[tokio::test]
    async fn clear_drops_catalogue_and_playback() {
        let server = catalogue_server().await;
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let state = VzyState::new(true);
        state.set_client_for_test(client_for(&server));
        state.ensure_catalogue(&store).await;
        state.live("vzy_1", &store).await.unwrap();
        state.clear();
        assert!(!state.contains("vzy_1"));
        assert!(state.playback.read().unwrap().is_empty());
    }

    #[tokio::test]
    async fn epg_maps_free_channels_to_xmltv() {
        let server = catalogue_server().await;
        Mock::given(method("GET"))
            .and(wiremock::matchers::header(
                "Authorization",
                "Bearer anon-ks",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "epgChunk": {
                    "1": [{"id": 9, "name": "Show", "description": "Desc",
                           "startDate": 1793923200, "endDate": 1793926800, "linearAssetId": 1}]
                }
            })))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let state = VzyState::new(true);
        state.set_client_for_test(client_for(&server));
        let (sources, programmes) = state.epg_source(&store).await.unwrap();
        assert_eq!(sources.len(), 2);
        let show = programmes.iter().find(|p| p.channel == "vzy_1").unwrap();
        assert_eq!(show.title, "Show");
        assert_eq!(show.start, "20261106000000 +0000");
    }

    #[test]
    fn milliseconds_are_not_double_scaled() {
        assert_eq!(seconds_to_millis(1793923200), 1793923200000);
        assert_eq!(seconds_to_millis(1793923200000), 1793923200000);
    }
}
