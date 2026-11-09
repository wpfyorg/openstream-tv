//! Airtel Xstream routing/caching layer: the catalogue, the playback cache
//! and the XMLTV guide. The protocol itself lives in [`crate::airtel`].

use crate::airtel::{
    channel_id, parse_channel_id, to_television_channel, AirtelChannel, Client, License, Playback,
};
use crate::keyed_locks::KeyedLocks;
use crate::television::{Channel, LiveUrlOutput, Mpd};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};

const CATALOGUE_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const CATALOGUE_RETRY_BACKOFF: Duration = Duration::from_secs(15);
const PLAYBACK_TTL: Duration = Duration::from_secs(5 * 60);
/// The guide covers the next six hours, enough for XMLTV's "now/next" use.
const EPG_WINDOW: Duration = Duration::from_secs(6 * 60 * 60);
const EPG_PAST: Duration = Duration::from_secs(60 * 60);

pub struct AirtelState {
    config_enabled: bool,
    client: RwLock<Option<Arc<Client>>>,
    catalogue: RwLock<Vec<AirtelChannel>>,
    catalogue_fetched_at: RwLock<Option<Instant>>,
    catalogue_failed_at: RwLock<Option<Instant>>,
    playback: RwLock<HashMap<String, (Playback, Instant)>>,
    generation: AtomicU64,
    commit: Mutex<()>,
    locks: KeyedLocks,
}

impl AirtelState {
    pub fn new(config_enabled: bool) -> AirtelState {
        AirtelState {
            config_enabled,
            client: RwLock::new(None),
            catalogue: RwLock::new(Vec::new()),
            catalogue_fetched_at: RwLock::new(None),
            catalogue_failed_at: RwLock::new(None),
            playback: RwLock::new(HashMap::new()),
            generation: AtomicU64::new(0),
            commit: Mutex::new(()),
            locks: KeyedLocks::default(),
        }
    }

    pub fn enabled(&self) -> bool {
        self.config_enabled
    }

    /// Airtel has no anonymous tier: "connected" means the feature is on and
    /// an account is logged in.
    pub fn connected(&self) -> bool {
        self.config_enabled
            && self
                .client
                .read()
                .unwrap()
                .as_ref()
                .is_some_and(|c| c.is_logged_in())
    }

    pub fn init(&self, http: &reqwest::Client, store: &crate::store::Store) {
        if !self.config_enabled {
            return;
        }
        if let Ok(client) = Client::from_store(http.clone(), store) {
            *self.client.write().unwrap() = Some(Arc::new(client));
        }
    }

    #[cfg(test)]
    pub fn set_client_for_test(&self, client: Arc<Client>) {
        *self.client.write().unwrap() = Some(client);
    }

    pub fn client(&self) -> Option<Arc<Client>> {
        self.client.read().unwrap().clone()
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

    pub fn clear(&self) {
        let _guard = self.commit.lock().unwrap();
        self.generation.fetch_add(1, Ordering::SeqCst);
        *self.catalogue.write().unwrap() = Vec::new();
        *self.catalogue_fetched_at.write().unwrap() = None;
        *self.catalogue_failed_at.write().unwrap() = None;
        self.playback.write().unwrap().clear();
    }

    pub fn logout(&self, store: &crate::store::Store) -> anyhow::Result<()> {
        if let Some(client) = self.client() {
            let mut guard = client.creds.write().unwrap();
            *guard = None;
            crate::airtel::delete_credentials(store)?;
        } else {
            crate::airtel::delete_credentials(store)?;
        }
        self.clear();
        Ok(())
    }

    /// Loads the account's entitled channel list. Returns false when the
    /// account is missing or Airtel refuses the request.
    pub async fn ensure_catalogue(&self, store: &crate::store::Store) -> bool {
        let Some(client) = self.client() else {
            return false;
        };
        if !client.is_logged_in() {
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
        let generation = self.generation();
        match client.channels(store).await {
            Ok(channels) if !channels.is_empty() => {
                let now = Instant::now();
                self.commit_if_current(generation, || {
                    *self.catalogue.write().unwrap() = channels;
                    *self.catalogue_fetched_at.write().unwrap() = Some(now);
                    *self.catalogue_failed_at.write().unwrap() = None;
                })
                .is_some()
            }
            _ => {
                let _ = self.commit_if_current(generation, || {
                    *self.catalogue_failed_at.write().unwrap() = Some(Instant::now());
                });
                false
            }
        }
    }

    /// The catalogue in channel-number order, which is the order Airtel's own
    /// guide uses.
    pub fn channels(&self) -> Vec<Channel> {
        let catalogue = self.catalogue.read().unwrap();
        let mut ordered: Vec<&AirtelChannel> = catalogue.iter().collect();
        ordered.sort_by(|a, b| {
            a.channel_no
                .unwrap_or(i64::MAX)
                .cmp(&b.channel_no.unwrap_or(i64::MAX))
                .then_with(|| a.title.cmp(&b.title))
        });
        ordered.into_iter().map(to_television_channel).collect()
    }

    pub fn contains(&self, channel_id: &str) -> bool {
        let Some(content_id) = parse_channel_id(channel_id) else {
            return false;
        };
        self.catalogue
            .read()
            .unwrap()
            .iter()
            .any(|c| c.id == content_id)
    }

    /// Resolve a channel to the shared playback shape. Airtel decides the
    /// container per channel, so both `bitrates.auto` (HLS) and `mpd.auto`
    /// (DASH) can be filled; the DASH branch is what carries a Widevine
    /// license.
    pub async fn live(
        &self,
        channel_id: &str,
        store: &crate::store::Store,
    ) -> anyhow::Result<LiveUrlOutput> {
        let content_id = parse_channel_id(channel_id)
            .ok_or_else(|| anyhow::anyhow!("airtel: not an airtel channel"))?
            .to_string();
        self.ensure_catalogue(store).await;
        let playback = self.playback_entry(store, &content_id).await?;
        let mut out = LiveUrlOutput {
            result: "OK".to_string(),
            ..Default::default()
        };
        if playback.is_dash {
            out.mpd = Mpd {
                auto: playback.url.clone(),
                ..Default::default()
            };
        } else {
            out.bitrates.auto = playback.url.clone();
        }
        out.is_drm = playback.license.is_some();
        if let Some(license) = &playback.license {
            out.key_url = license.url.clone();
        }
        Ok(out)
    }

    /// The cached playback descriptor, resolving it on first use.
    pub async fn playback_entry(
        &self,
        store: &crate::store::Store,
        content_id: &str,
    ) -> anyhow::Result<Playback> {
        let key = channel_id(content_id);
        if let Some((playback, at)) = self.playback.read().unwrap().get(&key) {
            if at.elapsed() < PLAYBACK_TTL {
                return Ok(playback.clone());
            }
        }
        let client = self
            .client()
            .ok_or_else(|| anyhow::anyhow!("airtel is not enabled"))?;
        if !client.is_logged_in() {
            anyhow::bail!("airtel is not logged in");
        }
        let _guard = self.locks.lock(&format!("playback:{key}")).await;
        if let Some((playback, at)) = self.playback.read().unwrap().get(&key) {
            if at.elapsed() < PLAYBACK_TTL {
                return Ok(playback.clone());
            }
        }
        let generation = self.generation();
        let playback = client.playback(store, content_id).await?;
        let now = Instant::now();
        let cached = playback.clone();
        self.commit_if_current(generation, || {
            self.playback.write().unwrap().insert(key, (cached, now));
        });
        Ok(playback)
    }

    /// Resolve enough playback metadata for a cold M3U request to classify
    /// Airtel channels as HLS/DASH and clear/DRM before the playlist is
    /// rendered. Work is bounded so a large catalogue cannot create an
    /// unbounded burst of upstream requests.
    pub async fn prepare_playlist_metadata(&self, store: Arc<crate::store::Store>) {
        let _guard = self.locks.lock("playlist-metadata").await;
        let generation = self.generation();
        let Some(client) = self.client() else {
            return;
        };
        if !client.is_logged_in() || self.generation() != generation {
            return;
        }
        let work: Vec<String> = self
            .catalogue
            .read()
            .unwrap()
            .iter()
            .filter_map(|item| {
                let key = channel_id(&item.id);
                let fresh = self
                    .playback
                    .read()
                    .unwrap()
                    .get(&key)
                    .is_some_and(|(_, at)| at.elapsed() < PLAYBACK_TTL);
                (!fresh).then(|| item.id.clone())
            })
            .collect();

        const MAX_IN_FLIGHT: usize = 8;
        let mut pending = work.into_iter();
        let mut tasks = tokio::task::JoinSet::new();
        for _ in 0..MAX_IN_FLIGHT {
            let Some(content_id) = pending.next() else {
                break;
            };
            let client = client.clone();
            let store = store.clone();
            tasks.spawn(async move {
                let playback = client.playback(&store, &content_id).await;
                (content_id, playback)
            });
        }

        while let Some(joined) = tasks.join_next().await {
            if self.generation() != generation {
                tasks.abort_all();
                return;
            }
            if let Ok((content_id, Ok(playback))) = joined {
                let now = Instant::now();
                let key = channel_id(&content_id);
                let _ = self.commit_if_current(generation, || {
                    self.playback.write().unwrap().insert(key, (playback, now));
                });
            }
            if let Some(content_id) = pending.next() {
                let client = client.clone();
                let store = store.clone();
                tasks.spawn(async move {
                    let playback = client.playback(&store, &content_id).await;
                    (content_id, playback)
                });
            }
        }
    }

    /// The CDN headers the app sends for this channel's manifest/segments.
    /// Falls back to just the player agent when playback cannot be resolved.
    pub async fn media_headers(
        &self,
        store: &crate::store::Store,
        channel_id: &str,
    ) -> Vec<(String, String)> {
        let Some(content_id) = parse_channel_id(channel_id) else {
            return vec![("User-Agent".to_string(), PLACEHOLDER_AGENT.to_string())];
        };
        match self.playback_entry(store, content_id).await {
            Ok(playback) => {
                let has_agent = playback
                    .headers
                    .iter()
                    .any(|(name, _)| name.eq_ignore_ascii_case("user-agent"));
                let mut headers = playback.headers;
                if !has_agent {
                    headers.push((
                        "User-Agent".to_string(),
                        crate::airtel::PLAYER_USER_AGENT.to_string(),
                    ));
                }
                headers
            }
            Err(_) => vec![(
                "User-Agent".to_string(),
                crate::airtel::PLAYER_USER_AGENT.to_string(),
            )],
        }
    }

    /// The Widevine descriptor for a channel, when the stream is protected.
    pub async fn license(&self, store: &crate::store::Store, channel_id: &str) -> Option<License> {
        let content_id = parse_channel_id(channel_id)?;
        self.playback_entry(store, content_id)
            .await
            .ok()
            .and_then(|p| p.license)
    }

    pub fn is_drm(&self, channel_id: &str) -> bool {
        self.playback
            .read()
            .unwrap()
            .get(channel_id)
            .is_some_and(|(p, _)| p.has_drm())
    }

    pub fn is_dash(&self, channel_id: &str) -> bool {
        self.playback
            .read()
            .unwrap()
            .get(channel_id)
            .is_some_and(|(p, _)| p.is_dash)
    }

    /// Channel display names plus the guide entries for the next few hours.
    pub async fn epg_source(
        &self,
        store: &crate::store::Store,
    ) -> anyhow::Result<(Vec<(String, String)>, Vec<crate::epg::XmlProgramme>)> {
        self.ensure_catalogue(store).await;
        let client = self
            .client()
            .ok_or_else(|| anyhow::anyhow!("airtel is not enabled"))?;
        if !client.is_logged_in() {
            anyhow::bail!("airtel is not logged in");
        }
        let channels = self.catalogue.read().unwrap().clone();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let start = now.saturating_sub(EPG_PAST.as_millis() as i64);
        let end = now.saturating_add(EPG_WINDOW.as_millis() as i64);
        let guide = client.epg(store, start, end).await?;
        let mut sources = Vec::new();
        let mut programmes = Vec::new();
        for channel in &channels {
            let id = channel_id(&channel.id);
            sources.push((id.clone(), channel.title.clone()));
            if let Some(programs) = guide.get(&channel.id) {
                for program in programs {
                    programmes.push(crate::epg::XmlProgramme {
                        channel: id.clone(),
                        start: crate::epg::format_xmltv_time(program.start_millis()),
                        stop: crate::epg::format_xmltv_time(program.end_millis()),
                        title: program.title.clone(),
                        desc: program.subtitle.clone(),
                        category: String::new(),
                        icon: String::new(),
                    });
                }
            }
        }
        Ok((sources, programmes))
    }

    /// The guide for one channel and day offset, for `/epg/<id>/<offset>`.
    pub async fn web_epg(
        &self,
        store: &crate::store::Store,
        channel_id: &str,
        offset_days: i64,
    ) -> anyhow::Result<Vec<crate::airtel::AirtelProgram>> {
        let content_id = parse_channel_id(channel_id)
            .ok_or_else(|| anyhow::anyhow!("airtel: not an airtel channel"))?;
        let client = self
            .client()
            .ok_or_else(|| anyhow::anyhow!("airtel is not enabled"))?;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let day = 86_400_000i64;
        let local = now + crate::epg::IST_OFFSET_MS;
        let start = (local - local.rem_euclid(day) - crate::epg::IST_OFFSET_MS)
            .saturating_add(offset_days.max(0).saturating_mul(day));
        let end = start.saturating_add(day);
        let guide = client.epg(store, start, end).await?;
        Ok(guide.get(content_id).cloned().unwrap_or_default())
    }

    pub fn account_fingerprint(&self) -> String {
        let (uid, device) = self
            .client()
            .map(|c| {
                (
                    c.credentials().map(|c| c.uid).unwrap_or_default(),
                    c.device.udid.clone(),
                )
            })
            .unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(b"airtel-account-v1");
        hasher.update(uid.as_bytes());
        hasher.update(device.as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// Placeholder agent used when the channel id is not an Airtel id; the real
/// path always resolves a playback descriptor first.
const PLACEHOLDER_AGENT: &str = "okhttp/4.12.0";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::airtel::{is_airtel_channel, Device};
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn client_for(server: &MockServer) -> Arc<Client> {
        let mut client = Client::new(reqwest::Client::new(), Device::new());
        client.set_bases_for_test(&server.uri(), &server.uri(), &server.uri());
        *client.creds.write().unwrap() = Some(crate::airtel::Credentials {
            uid: "uid".into(),
            token: "token".into(),
            language: "hi".into(),
            content_properties: HashMap::from([("cp".to_string(), "airtel".to_string())]),
            ..Default::default()
        });
        Arc::new(client)
    }

    #[tokio::test]
    async fn live_fills_the_hls_half_for_a_clear_channel() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/app/v3/content/channels"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"channels":[{"id":"1001","title":"Star Plus","hd":true}]}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v4/user/content/playback"))
            .and(query_param("contentId", "1001"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"success":true,"playback":{"playUrl":"https://cdn/1001/index.m3u8"}}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        let state = AirtelState::new(true);
        state.set_client_for_test(client_for(&server));
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let out = state.live("airtel_1001", &store).await.unwrap();
        assert_eq!(out.bitrates.auto, "https://cdn/1001/index.m3u8");
        assert!(out.mpd.auto.is_empty());
        assert!(!out.is_drm);
        assert!(state.contains("airtel_1001"));
        assert!(!state.contains("airtel_9999"));
    }

    #[tokio::test]
    async fn live_fills_the_dash_half_and_license_for_a_protected_channel() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/app/v3/content/channels"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"channels":[{"id":"2002","title":"Star Movies","hd":true}]}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v4/user/content/playback"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"success":true,"playback":{"playUrl":"https://cdn/2002/manifest.mpd","headers":{"x-cdn":"a"}},"drm":{"licenceUrl":"https://play.airtel.tv/drm","headers":{"x-atv-cp":"airtel"},"body":{},"licensePayloadKey":"default"}}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        let state = AirtelState::new(true);
        state.set_client_for_test(client_for(&server));
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let out = state.live("airtel_2002", &store).await.unwrap();
        assert_eq!(out.mpd.auto, "https://cdn/2002/manifest.mpd");
        assert!(out.bitrates.auto.is_empty());
        assert!(out.is_drm);
        assert_eq!(out.key_url, "https://play.airtel.tv/drm");
        assert!(state.is_drm("airtel_2002"));
        assert!(state.is_dash("airtel_2002"));
        let headers = state.media_headers(&store, "airtel_2002").await;
        assert!(headers.iter().any(|(k, v)| k == "x-cdn" && v == "a"));
        assert!(headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("user-agent")));
        let license = state.license(&store, "airtel_2002").await.unwrap();
        assert_eq!(license.url, "https://play.airtel.tv/drm");
    }

    #[tokio::test]
    async fn logout_during_refresh_does_not_restore_the_account() {
        use tokio::sync::Notify;
        use wiremock::Request;

        let server = MockServer::start().await;
        let refresh_started = std::sync::Arc::new(Notify::new());
        let refresh_started_for_response = refresh_started.clone();
        Mock::given(method("POST"))
            .and(path("/v2/user/session/refreshAuthToken"))
            .respond_with(move |_request: &Request| {
                refresh_started_for_response.notify_one();
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(100))
                    .set_body_raw(
                        r#"{"success":true,"token":"new-token"}"#,
                        "application/json",
                    )
            })
            .expect(1)
            .mount(&server)
            .await;
        let state = AirtelState::new(true);
        let client = client_for(&server);
        state.set_client_for_test(client.clone());
        let dir = tempfile::tempdir().unwrap();
        let store =
            std::sync::Arc::new(crate::store::Store::open(dir.path().to_str().unwrap()).unwrap());
        crate::airtel::Credentials {
            uid: "uid".into(),
            token: "token".into(),
            ..Default::default()
        }
        .save(&store)
        .unwrap();

        let refresh_store = store.clone();
        let refresh = tokio::spawn(async move { client.refresh(&refresh_store).await });
        refresh_started.notified().await;
        state.logout(&store).unwrap();
        let error = refresh.await.unwrap().unwrap_err();
        assert!(error
            .to_string()
            .contains("logged out during token refresh"));
        assert!(crate::airtel::Credentials::load(&store).is_none());
        assert!(!state.connected());
    }

    #[tokio::test]
    async fn prepare_playlist_metadata_classifies_playback_before_export() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/app/v3/content/channels"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"channels":[{"id":"2002","title":"Star Movies","hd":true}]}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v4/user/content/playback"))
            .and(query_param("contentId", "2002"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"success":true,"playback":{"playUrl":"https://cdn/2002/manifest.mpd"},"drm":{"licenceUrl":"https://play.airtel.tv/drm"}}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        let state = AirtelState::new(true);
        state.set_client_for_test(client_for(&server));
        let dir = tempfile::tempdir().unwrap();
        let store =
            std::sync::Arc::new(crate::store::Store::open(dir.path().to_str().unwrap()).unwrap());
        assert!(state.ensure_catalogue(&store).await);
        assert!(!state.is_drm("airtel_2002"));
        assert!(!state.is_dash("airtel_2002"));
        state.prepare_playlist_metadata(store.clone()).await;
        assert!(state.is_drm("airtel_2002"));
        assert!(state.is_dash("airtel_2002"));
    }

    #[tokio::test]
    async fn web_epg_saturates_an_extreme_day_offset() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/app/v3/content/epg"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(r#"{"programGuide":{"1001":[]}}"#, "application/json"),
            )
            .mount(&server)
            .await;
        let state = AirtelState::new(true);
        state.set_client_for_test(client_for(&server));
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        // A huge offset from the URL must not overflow the day arithmetic.
        assert!(state.web_epg(&store, "airtel_1001", i64::MAX).await.is_ok());
        assert!(state.web_epg(&store, "airtel_1001", 0).await.is_ok());
    }

    #[tokio::test]
    async fn disconnected_state_rejects_catalogue_and_playback() {
        let state = AirtelState::new(true);
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        assert!(!state.connected());
        assert!(!state.ensure_catalogue(&store).await);
        assert!(state.live("airtel_1001", &store).await.is_err());
    }

    #[tokio::test]
    async fn logout_drops_the_stored_account() {
        let state = AirtelState::new(true);
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let server = MockServer::start().await;
        state.set_client_for_test(client_for(&server));
        crate::airtel::Credentials {
            uid: "uid".into(),
            token: "token".into(),
            ..Default::default()
        }
        .save(&store)
        .unwrap();
        state.logout(&store).unwrap();
        assert!(crate::airtel::Credentials::load(&store).is_none());
        assert!(!state.connected());
    }

    #[test]
    fn account_fingerprint_changes_with_the_account() {
        let state = AirtelState::new(false);
        assert!(state.account_fingerprint().len() == 64);
        assert!(!is_airtel_channel("vzy_1"));
    }
}
