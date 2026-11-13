use crate::access::Access;
use crate::airtel_state::AirtelState;
use crate::config::Config;
use crate::custom_channels::CustomChannels;
use crate::dash::DashState;
use crate::extras_state::ExtrasState;
use crate::fastway_state::FastwayState;
use crate::playbox_state::PlayboxState;
use crate::secureurl::SecureUrl;
use crate::store::Store;
use crate::stream::RenderCaches;
use crate::tataplay_state::TataplayState;
use crate::television::Television;
use crate::vzy_state::VzyState;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveProduct {
    Tv,
    Extras,
    Fastway,
    Playbox,
    Vzy,
    Airtel,
    TataPlay,
}

/// Store key holding the provider the user last activated. Exactly one value
/// is kept, so the provider catalogues can never be active together.
pub const STORE_KEY_ACTIVE_PROVIDER: &str = "active_provider";

impl ActiveProduct {
    pub fn as_str(self) -> &'static str {
        match self {
            ActiveProduct::Tv => "tv",
            ActiveProduct::Extras => "extras",
            ActiveProduct::Fastway => "fastway",
            ActiveProduct::Playbox => "playbox",
            ActiveProduct::Vzy => "vzy",
            ActiveProduct::Airtel => "airtel",
            ActiveProduct::TataPlay => "tataplay",
        }
    }

    /// Provider names accepted by the UI/API. The legacy `tv`/`extras`
    /// spellings keep an existing store working.
    pub fn from_name(name: &str) -> Option<ActiveProduct> {
        match name {
            "jiotv" | "tv" => Some(ActiveProduct::Tv),
            "jiotv_plus" | "jiotv-plus" | "tv_plus" | "extras" => Some(ActiveProduct::Extras),
            "fastway" => Some(ActiveProduct::Fastway),
            "playbox" | "playbox_tv" | "playboxtv" => Some(ActiveProduct::Playbox),
            "vzy" => Some(ActiveProduct::Vzy),
            "airtel" | "airtel_xstream" | "airtel-xstream" | "xstream" => {
                Some(ActiveProduct::Airtel)
            }
            "tataplay" | "tata_play" | "tata-play" | "tataplay_smart" => {
                Some(ActiveProduct::TataPlay)
            }
            _ => None,
        }
    }

    pub fn store_name(self) -> &'static str {
        match self {
            ActiveProduct::Tv => "jiotv",
            ActiveProduct::Extras => "jiotv_plus",
            ActiveProduct::Fastway => "fastway",
            ActiveProduct::Playbox => "playbox",
            ActiveProduct::Vzy => "vzy",
            ActiveProduct::Airtel => "airtel",
            ActiveProduct::TataPlay => "tataplay",
        }
    }
}

/// Store keys for playback observations learned from successful JioTV
/// resolutions. `...FOR` records which account they belong to, so a different
/// login discards them instead of trusting another account's stream shapes.
pub const STORE_KEY_LEARNED_DASH: &str = "playbackLearnedDash";
pub const STORE_KEY_LEARNED_DRM: &str = "playbackLearnedDrm";
pub const STORE_KEY_LEARNED_FOR: &str = "playbackLearnedFor";

/// What this server has observed about JioTV playback: channels whose
/// catalogue entry has no HLS rendition, and which of those also need a
/// Widevine license (a DASH channel is not automatically protected).
#[derive(Debug, Default)]
pub struct LearnedPlayback {
    dash: HashSet<String>,
    drm: HashSet<String>,
    /// Account fingerprint the observations were made for.
    account: String,
}

/// JioTV channel IDs are bare numeric strings. Every other provider uses a
/// prefixed id space of its own, so this also keeps observations to JioTV.
pub fn is_learnable_channel_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 8 && id.bytes().all(|b| b.is_ascii_digit())
}

fn parse_learned_ids(csv: &str) -> HashSet<String> {
    csv.split(',')
        .map(str::trim)
        .filter(|id| is_learnable_channel_id(id))
        .map(str::to_string)
        .collect()
}

fn learned_ids_csv(ids: &HashSet<String>) -> String {
    let mut sorted: Vec<&str> = ids.iter().map(String::as_str).collect();
    sorted.sort_unstable();
    sorted.join(",")
}

/// Restores persisted observations, but only when they were made for the same
/// JioTV account. A missing, malformed or foreign record is simply ignored.
pub fn load_learned_playback(store: &Store, account: &str) -> LearnedPlayback {
    let mut learned = LearnedPlayback::default();
    if account.is_empty() {
        return learned;
    }
    match store.get_opt(STORE_KEY_LEARNED_FOR) {
        Some(stored) if stored == account => {}
        _ => return learned,
    }
    learned.dash = parse_learned_ids(&store.get_opt(STORE_KEY_LEARNED_DASH).unwrap_or_default());
    learned.drm = parse_learned_ids(&store.get_opt(STORE_KEY_LEARNED_DRM).unwrap_or_default());
    // Only channels known to be DASH-only can require DASH.
    learned.drm.retain(|id| learned.dash.contains(id));
    learned.account = account.to_string();
    learned
}

pub struct AppState {
    pub config: Config,
    /// The resolved data directory (`config.path_prefix`, or `~/.openstream-tv`; `/etc/openstream` on OpenWrt),
    /// always ending in `/`. Used for `epg.xml.gz` and similar data files.
    pub path_prefix: String,
    pub access: Arc<Access>,
    pub store: Arc<Store>,
    pub tv: Arc<Television>,
    pub secure: Arc<SecureUrl>,
    pub http: reqwest::Client,
    /// Playback routing learned from successful JioTV resolutions: channels
    /// that only have DASH, and which of those need a license. Seeded by the
    /// static list in `drm_channels.rs`; the field keeps its original name so
    /// existing `Default` construction sites stay valid.
    pub drm_channels: std::sync::RwLock<LearnedPlayback>,
    pub custom_channels: Arc<CustomChannels>,
    pub render_caches: RenderCaches,
    pub dash_state: DashState,
    pub epg_state: crate::epg::EpgState,
    pub extras: Arc<ExtrasState>,
    pub fastway: Arc<FastwayState>,
    pub playbox: Arc<PlayboxState>,
    pub airtel: Arc<AirtelState>,
    pub tataplay: Arc<TataplayState>,
    pub vzy: Arc<VzyState>,
    pub vod_state: crate::vod::VodState,
    /// The server's cached public IPv4, used to compute today's extras
    /// unlock code. See `unlock.rs`.
    pub public_ip: Arc<crate::unlock::PublicIp>,
    pub unlock_limiter: Arc<crate::unlock::AttemptLimiter>,
    /// The ports `serve` listens on, recorded once the listeners are chosen so
    /// the UI can offer the plain-HTTP playlist origin from an HTTPS page.
    pub listen: std::sync::OnceLock<ListenPorts>,
    /// HTTPS origin learned from the cloudflared process or saved guided setup.
    pub tunnel_origin: std::sync::OnceLock<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListenPorts {
    pub http: u16,
    pub tls: Option<u16>,
}

impl AppState {
    /// Returns `response` only if no account or product transition rotated the
    /// context epoch since `epoch` was captured at the start of the request.
    /// A handler that mints encrypted URLs from account-scoped data must be
    /// wrapped in this: if the epoch moved meanwhile, the response could carry
    /// the previous context's data under URLs encrypted for the new one.
    pub fn stable_since(
        &self,
        epoch: u64,
        response: axum::response::Response,
    ) -> axum::response::Response {
        use axum::response::IntoResponse;
        if self.secure.current_epoch() == epoch {
            response
        } else {
            (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "The active account changed while this request was running; retry",
            )
                .into_response()
        }
    }

    /// Process-local account/mode epoch shared by all encrypted playback
    /// artifacts. Rotating it makes previously issued manifest, segment,
    /// key and license URLs unusable immediately.
    pub fn context_epoch(&self) -> u64 {
        self.secure.current_epoch()
    }

    /// Invalidates every account-sensitive process cache at one serialized
    /// context boundary. EPG publishers use the same guard before committing,
    /// so a generation started in the old epoch cannot publish after this
    /// returns.
    pub fn invalidate_context(&self) {
        let _guard = self.epg_state.context_guard();
        self.epg_state.invalidate();
        self.secure.rotate();
        self.render_caches.clear();
        self.dash_state.clear();
        self.vod_state.clear();
        // Provider switching must not leak one account's catalogue into the
        // next context: drop provider caches at the same boundary.
        self.playbox.clear();
        self.vzy.clear();
        self.airtel.clear();
        self.tataplay.clear();
    }

    /// The provider the user explicitly activated, if any. A stored choice is
    /// kept even while its provider is disconnected, so reconnecting that
    /// provider makes it active again.
    pub fn preferred_product(&self) -> Option<ActiveProduct> {
        self.store
            .get_opt(STORE_KEY_ACTIVE_PROVIDER)
            .as_deref()
            .and_then(ActiveProduct::from_name)
    }

    /// Whether a provider can currently supply its catalogue.
    pub fn provider_available(&self, product: ActiveProduct) -> bool {
        match product {
            ActiveProduct::Tv => self.tv.logged_in(),
            ActiveProduct::Extras => self.extras.connected(),
            ActiveProduct::Fastway => self.fastway.connected(),
            ActiveProduct::Playbox => self.playbox.connected(),
            ActiveProduct::Vzy => self.vzy.connected(),
            ActiveProduct::Airtel => self.airtel.connected(),
            ActiveProduct::TataPlay => self.tataplay.connected(),
        }
    }

    /// Used when the stored choice is unavailable or absent: prefer an
    /// authenticated provider in the documented order, then a connected JioTV
    /// account, and only then the anonymous VZY free tier, which must never
    /// shadow an account the user signed in to.
    fn default_product(&self) -> ActiveProduct {
        if self.fastway.connected() {
            ActiveProduct::Fastway
        } else if self.extras.connected() {
            ActiveProduct::Extras
        } else if self.playbox.connected() {
            ActiveProduct::Playbox
        } else if self.airtel.connected() {
            ActiveProduct::Airtel
        } else if self.tataplay.connected() {
            ActiveProduct::TataPlay
        } else if self.tv.logged_in() {
            ActiveProduct::Tv
        } else if self.vzy.connected() {
            ActiveProduct::Vzy
        } else {
            ActiveProduct::Tv
        }
    }

    /// Product selection follows the provider the user activated, but only
    /// while that provider can supply its catalogue. Disconnecting it falls
    /// back to the next available provider without erasing the preference.
    pub fn active_product(&self) -> ActiveProduct {
        match self.preferred_product() {
            Some(product) if self.provider_available(product) => product,
            _ => self.default_product(),
        }
    }

    /// Persists the provider the user activated. A single value is stored, so
    /// activating one provider deactivates every other one.
    pub fn set_active_product(
        &self,
        product: ActiveProduct,
    ) -> Result<(), crate::store::StoreError> {
        self.store
            .set(STORE_KEY_ACTIVE_PROVIDER, product.store_name())
    }

    /// Returns only channels from the active upstream product. Custom
    /// channels are deliberately excluded here because they are local user
    /// additions rather than part of either provider catalogue.
    pub async fn effective_upstream_channels(
        &self,
    ) -> anyhow::Result<Vec<crate::television::Channel>> {
        match self.active_product() {
            ActiveProduct::Fastway => {
                self.fastway.refresh_catalogue_if_needed(&self.store).await;
                Ok(self.fastway.catalogue_channels())
            }
            ActiveProduct::Extras => {
                self.extras.refresh_catalogue_if_needed(&self.tv).await;
                Ok(self.extras.catalogue_channels())
            }
            ActiveProduct::Playbox => {
                let loaded = self.playbox.ensure_catalogue().await;
                let channels = self.playbox.channels();
                if !loaded && channels.is_empty() {
                    anyhow::bail!("playbox catalogue is unavailable");
                }
                Ok(channels)
            }
            ActiveProduct::Vzy => {
                let loaded = self.vzy.ensure_catalogue(&self.store).await;
                let channels = self.vzy.channels();
                if !loaded && channels.is_empty() {
                    anyhow::bail!("vzy catalogue is unavailable");
                }
                Ok(channels)
            }
            ActiveProduct::TataPlay => {
                let loaded = self.tataplay.ensure_catalogue().await;
                let channels = self.tataplay.channels();
                if !loaded && channels.is_empty() {
                    anyhow::bail!("tataplay catalogue is unavailable");
                }
                Ok(channels)
            }
            ActiveProduct::Tv => Ok(self.tv.channels().await?.result),
            ActiveProduct::Airtel => {
                let loaded = self.airtel.ensure_catalogue(&self.store).await;
                let channels = self.airtel.channels();
                if !loaded && channels.is_empty() {
                    anyhow::bail!("airtel catalogue is unavailable");
                }
                Ok(channels)
            }
        }
    }

    /// The one catalogue exposed to UI, playlists and channel lookups.
    /// Local custom channels remain available regardless of provider mode.
    pub async fn effective_channels(&self) -> anyhow::Result<Vec<crate::television::Channel>> {
        let mut channels = self.effective_upstream_channels().await?;
        channels.extend(self.custom_channels.all());
        Ok(channels)
    }

    pub async fn channel_allowed(&self, id: &str) -> bool {
        if self.custom_channels.contains(id) {
            return true;
        }
        match self.active_product() {
            ActiveProduct::Fastway => {
                self.fastway.refresh_catalogue_if_needed(&self.store).await;
                self.fastway.contains_catalogue_channel(id)
            }
            ActiveProduct::Extras => {
                self.extras.refresh_catalogue_if_needed(&self.tv).await;
                self.extras.contains_catalogue_channel(id)
            }
            ActiveProduct::Playbox => {
                self.playbox.ensure_catalogue().await;
                self.playbox.contains(id)
            }
            ActiveProduct::Vzy => {
                self.vzy.ensure_catalogue(&self.store).await;
                self.vzy.contains(id)
            }
            ActiveProduct::Airtel => {
                self.airtel.ensure_catalogue(&self.store).await;
                self.airtel.contains(id)
            }
            ActiveProduct::TataPlay => {
                self.tataplay.ensure_catalogue().await;
                self.tataplay.contains(id)
            }
            ActiveProduct::Tv => self
                .tv
                .channels()
                .await
                .map(|list| list.result.iter().any(|ch| ch.id == id))
                .unwrap_or(false),
        }
    }

    /// Mirrors `isDRMChannel`: DRM state comes from the provider when the
    /// channel routes through Fastway or extras, else the static JioTV DRM
    /// list. Fastway clear channels are DASH without a license and therefore
    /// report `false` here; use `is_dash_channel` for the container choice.
    /// Callers on a hot path that needs a fresh catalogue should await the
    /// provider's `refresh_*_if_needed` first (playlist/channels handlers
    /// already do).
    pub fn is_drm_channel(&self, id: &str) -> bool {
        if !self.config.drm {
            return false;
        }
        if FastwayState::is_fastway_channel(id) {
            return self.fastway.is_drm(id);
        }
        if crate::playbox::is_playbox_channel(id) {
            return self.playbox.is_drm(id);
        }
        if crate::airtel::is_airtel_channel(id) {
            return self.airtel.is_drm(id);
        }
        if crate::tataplay::is_tataplay_channel(id) {
            // Every Tata Play live channel is Widevine DASH, independent of
            // whether its playback has been resolved yet.
            return true;
        }
        match self
            .extras
            .route(id, self.tv.logged_in(), self.custom_channels.contains(id))
        {
            Some(content_id) => {
                let is_ex_id = ExtrasState::is_extras_channel(id);
                self.extras
                    .is_drm(&content_id, is_ex_id)
                    .unwrap_or_else(|| {
                        crate::drm_channels::is_drm_channel(id) || self.learned_drm(id)
                    })
            }
            None => {
                crate::drm_channels::is_drm_channel(id) || self.learned_drm(id)
            }
        }
    }

    /// Whether the channel is DASH even without a Widevine license. Fastway's
    /// whole catalogue is DASH, while Playbox learns the container from its
    /// resolved playback metadata.
    pub fn is_dash_channel(&self, id: &str) -> bool {
        if FastwayState::is_fastway_channel(id) {
            return self.fastway.is_dash(id);
        }
        if crate::playbox::is_playbox_channel(id) {
            return self.playbox.is_dash(id);
        }
        if crate::airtel::is_airtel_channel(id) {
            return self.airtel.is_dash(id);
        }
        if crate::tataplay::is_tataplay_channel(id) {
            return true;
        }
        if is_learnable_channel_id(id) {
            let learned = self.drm_channels.read().unwrap();
            if learned.dash.contains(id) {
                // A protected DASH channel still needs the DRM pathway enabled.
                return self.config.drm || !learned.drm.contains(id);
            }
        }
        false
    }

    fn learned_drm(&self, id: &str) -> bool {
        self.drm_channels.read().unwrap().drm.contains(id)
    }

    /// Whether `fetch_live` resolves this id with the JioTV provider, rather
    /// than a custom channel or another provider. A numeric id alone is not
    /// enough: extras can claim it while it is the active source.
    pub fn resolves_through_jiotv(&self, id: &str) -> bool {
        if !is_learnable_channel_id(id) || self.custom_channels.contains(id) {
            return false;
        }
        if self.fastway.route(id).is_some()
            || crate::playbox::is_playbox_channel(id)
            || crate::vzy::is_vzy_channel(id)
            || crate::airtel::is_airtel_channel(id)
            || crate::tataplay::is_tataplay_channel(id)
        {
            return false;
        }
        self.extras.route(id, self.tv.logged_in(), false).is_none()
    }

    /// Records what a successful JioTV resolution showed: a channel with an
    /// HLS rendition plays over HLS, a channel that only has DASH needs the
    /// MPD route, and a DASH channel with a license URL needs a license too.
    /// Only successful resolutions are passed in, so transient errors never
    /// change what is stored.
    pub fn observe_playback(&self, id: &str, live: &crate::television::LiveUrlOutput) {
        if !self.resolves_through_jiotv(id) {
            return;
        }
        let has_hls = !crate::television::select_best_live_hls_url(live, "auto").is_empty();
        if has_hls {
            self.forget_learned_playback(id);
        } else if crate::television::has_dash(live) {
            self.learn_playback(id, live.has_drm_stream());
        }
    }

    /// Remembers that `id` needs the MPD route, and whether it also needs a
    /// license. Persisted only when an observation actually changes.
    pub(crate) fn learn_playback(&self, id: &str, needs_license: bool) {
        let (dash_csv, drm_csv) = {
            let mut learned = self.drm_channels.write().unwrap();
            let mut changed = learned.dash.insert(id.to_string());
            if needs_license {
                changed |= learned.drm.insert(id.to_string());
            }
            if !changed {
                return;
            }
            tracing::info!(
                channel = id,
                needs_license,
                "learned that a JioTV channel needs the DASH route"
            );
            (
                learned_ids_csv(&learned.dash),
                learned_ids_csv(&learned.drm),
            )
        };
        self.persist_learned_playback(&dash_csv, &drm_csv);
    }

    /// Forgets a channel that has an HLS rendition again.
    fn forget_learned_playback(&self, id: &str) {
        let (dash_csv, drm_csv) = {
            let mut learned = self.drm_channels.write().unwrap();
            let removed_dash = learned.dash.remove(id);
            let removed_drm = learned.drm.remove(id);
            if !removed_dash && !removed_drm {
                return;
            }
            tracing::info!(channel = id, "JioTV channel has HLS again");
            (
                learned_ids_csv(&learned.dash),
                learned_ids_csv(&learned.drm),
            )
        };
        self.persist_learned_playback(&dash_csv, &drm_csv);
    }

    fn persist_learned_playback(&self, dash_csv: &str, drm_csv: &str) {
        if let Err(e) = self.store.set(STORE_KEY_LEARNED_DASH, dash_csv) {
            tracing::warn!("cannot save learned DASH channels: {e}");
        }
        if let Err(e) = self.store.set(STORE_KEY_LEARNED_DRM, drm_csv) {
            tracing::warn!("cannot save learned DRM channels: {e}");
        }
    }

    /// Adopts `account` as the login the observations belong to. A different
    /// account invalidates whatever was learned for the previous one.
    pub fn set_learned_playback_account(&self, account: &str) {
        let (dash_csv, drm_csv) = {
            let mut learned = self.drm_channels.write().unwrap();
            if learned.account == account {
                return;
            }
            learned.account = account.to_string();
            learned.dash.clear();
            learned.drm.clear();
            (String::new(), String::new())
        };
        if let Err(e) = self.store.set(STORE_KEY_LEARNED_FOR, account) {
            tracing::warn!("cannot save the account for learned playback: {e}");
        }
        self.persist_learned_playback(&dash_csv, &drm_csv);
    }

    /// The user agent a provider's CDN expects for manifests and segments.
    pub fn player_user_agent_for(&self, host: &str) -> &'static str {
        if self.fastway.is_cdn_host(host) {
            crate::fastway::PLAYER_USER_AGENT
        } else if crate::airtel::looks_like_airtel_cdn(host) {
            crate::airtel::PLAYER_USER_AGENT
        } else {
            self.extras.player_user_agent_for(host)
        }
    }

    /// Whether a channel from the already-filtered effective catalogue can
    /// be started with the active account.
    pub fn is_playable(&self, id: &str) -> bool {
        if self.custom_channels.contains(id) {
            return true;
        }
        match self.active_product() {
            ActiveProduct::Fastway => self.fastway.connected(),
            ActiveProduct::Extras => self.extras.connected(),
            ActiveProduct::Playbox => self.playbox.connected(),
            ActiveProduct::Vzy => self.vzy.connected(),
            ActiveProduct::Airtel => self.airtel.connected(),
            ActiveProduct::TataPlay => self.tataplay.connected(),
            ActiveProduct::Tv => self.tv.logged_in(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    fn state() -> Arc<AppState> {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(dir.path().to_str().unwrap()).unwrap());
        std::mem::forget(dir);
        Arc::new(AppState {
            config: Config::default(),
            path_prefix: String::new(),
            access: Arc::new(Access::new(store.clone())),
            store,
            tv: Arc::new(Television::new(reqwest::Client::new())),
            secure: Arc::new(SecureUrl::new(false)),
            http: reqwest::Client::new(),
            drm_channels: Default::default(),
            custom_channels: Arc::new(CustomChannels::new()),
            render_caches: Default::default(),
            dash_state: Default::default(),
            epg_state: Default::default(),
            extras: Arc::new(ExtrasState::new(false, None)),
            fastway: Arc::new(FastwayState::new(false)),
            playbox: Arc::new(PlayboxState::new(false)),
            tataplay: Arc::new(TataplayState::new(false)),
            vzy: Arc::new(VzyState::new(false)),
            airtel: Arc::new(AirtelState::new(false)),
            vod_state: Default::default(),
            public_ip: Arc::new(crate::unlock::PublicIp::new(reqwest::Client::new())),
            unlock_limiter: Arc::new(crate::unlock::AttemptLimiter::default()),
            listen: Default::default(),
            tunnel_origin: Default::default(),
        })
    }

    fn state_with_providers(
        store: Arc<Store>,
        extras: Arc<ExtrasState>,
        playbox: Arc<PlayboxState>,
    ) -> Arc<AppState> {
        Arc::new(AppState {
            config: Config::default(),
            path_prefix: String::new(),
            access: Arc::new(Access::new(store.clone())),
            store,
            tv: Arc::new(Television::new(reqwest::Client::new())),
            secure: Arc::new(SecureUrl::new(false)),
            http: reqwest::Client::new(),
            drm_channels: Default::default(),
            custom_channels: Arc::new(CustomChannels::new()),
            render_caches: Default::default(),
            dash_state: Default::default(),
            epg_state: Default::default(),
            extras,
            fastway: Arc::new(FastwayState::new(false)),
            playbox,
            tataplay: Arc::new(TataplayState::new(false)),
            vzy: Arc::new(VzyState::new(false)),
            airtel: Arc::new(AirtelState::new(false)),
            vod_state: Default::default(),
            public_ip: Arc::new(crate::unlock::PublicIp::new(reqwest::Client::new())),
            unlock_limiter: Arc::new(crate::unlock::AttemptLimiter::default()),
            listen: Default::default(),
            tunnel_origin: Default::default(),
        })
    }

    fn playbox_state_with_mock(server: &wiremock::MockServer) -> Arc<PlayboxState> {
        let state = Arc::new(PlayboxState::new(true));
        let mut client =
            crate::playbox::Client::new(reqwest::Client::new(), crate::playbox::Device::new());
        client.set_bases_for_test(&server.uri());
        *client.creds.write().unwrap() = crate::playbox::Credentials {
            phone: "9876543210".into(),
            token: "opaque-session-token".into(),
            customer_id: "cust-1".into(),
        };
        state.set_client_for_test(Arc::new(client));
        state
    }

    fn empty_store() -> Arc<Store> {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(dir.path().to_str().unwrap()).unwrap());
        std::mem::forget(dir);
        store
    }

    #[tokio::test]
    async fn playbox_becomes_the_active_catalogue_only_when_connected() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/catalogue/v3/page/livetv2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "livetv": [{"id": "ch1", "name": "One", "typeId": 106}]
            })))
            .mount(&server)
            .await;

        let disconnected = state_with_providers(
            empty_store(),
            Arc::new(ExtrasState::new(false, None)),
            Arc::new(PlayboxState::new(false)),
        );
        assert_eq!(disconnected.active_product(), ActiveProduct::Tv);
        assert!(!disconnected.is_playable("pb_ch1"));

        let s = state_with_providers(
            empty_store(),
            Arc::new(ExtrasState::new(false, None)),
            playbox_state_with_mock(&server),
        );
        assert_eq!(s.active_product(), ActiveProduct::Playbox);
        assert!(s.is_playable("pb_ch1"));

        let channels = s.effective_upstream_channels().await.unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].id, "pb_ch1");
        assert!(s.channel_allowed("pb_ch1").await);
        assert!(!s.channel_allowed("pb_other").await);
    }

    #[tokio::test]
    async fn cold_playbox_catalogue_failure_is_reported() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/catalogue/v3/page/livetv2"))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&server)
            .await;
        let s = state_with_providers(
            empty_store(),
            Arc::new(ExtrasState::new(false, None)),
            playbox_state_with_mock(&server),
        );

        assert_eq!(
            s.effective_upstream_channels()
                .await
                .unwrap_err()
                .to_string(),
            "playbox catalogue is unavailable"
        );
    }

    #[tokio::test]
    async fn jiotv_plus_keeps_precedence_over_playbox() {
        let store = empty_store();
        store
            .set(
                crate::extras::STORE_KEY_CREDENTIALS,
                &serde_json::to_string(&crate::extras::Credentials {
                    sso_token: "redacted-sso".into(),
                    subscriber_id: "redacted-sub".into(),
                    user_id: "redacted-user".into(),
                    auth_token: "redacted-access".into(),
                    ..Default::default()
                })
                .unwrap(),
            )
            .unwrap();
        let extras = Arc::new(ExtrasState::new(true, None));
        extras.init(&reqwest::Client::new(), &store);
        assert!(extras.connected());

        let server = wiremock::MockServer::start().await;
        let s = state_with_providers(empty_store(), extras, playbox_state_with_mock(&server));
        assert_eq!(s.active_product(), ActiveProduct::Extras);
    }

    #[tokio::test]
    async fn logged_out_jiotv_preference_falls_back_to_the_automatic_priority() {
        let server = wiremock::MockServer::start().await;
        let s = state_with_providers(
            empty_store(),
            Arc::new(ExtrasState::new(false, None)),
            playbox_state_with_mock(&server),
        );
        assert_eq!(s.active_product(), ActiveProduct::Playbox);
        s.set_active_product(ActiveProduct::Tv).unwrap();
        assert_eq!(s.preferred_product(), Some(ActiveProduct::Tv));
        // JioTV has no credentials in this test, so the stored JioTV choice is
        // unavailable and the connected PlayboxTV catalogue stays active.
        assert_eq!(s.active_product(), ActiveProduct::Playbox);
    }

    #[test]
    fn a_disconnected_provider_choice_falls_back_without_being_erased() {
        let s = state_with_providers(
            empty_store(),
            Arc::new(ExtrasState::new(false, None)),
            Arc::new(PlayboxState::new(false)),
        );
        s.set_active_product(ActiveProduct::Fastway).unwrap();
        assert_eq!(s.preferred_product(), Some(ActiveProduct::Fastway));
        assert_eq!(s.active_product(), ActiveProduct::Tv);
    }

    #[test]
    fn provider_store_names_round_trip() {
        for product in [
            ActiveProduct::Tv,
            ActiveProduct::Extras,
            ActiveProduct::Fastway,
            ActiveProduct::Playbox,
            ActiveProduct::Vzy,
            ActiveProduct::Airtel,
            ActiveProduct::TataPlay,
        ] {
            assert_eq!(
                ActiveProduct::from_name(product.store_name()),
                Some(product)
            );
        }
    }

    #[tokio::test]
    async fn tataplay_channels_are_dash_and_playable_without_resolving_playback() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/homescreen/pub/api/v1/lit/hierarchy/LIT_LIVETV"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{"railId": 1, "contentList": [
                    {"id": 1918, "channelName": "Star Plus"}
                ]}]
            })))
            .mount(&server)
            .await;
        let s = state();
        s.tataplay
            .set_client_for_test(Arc::new(crate::tataplay::Client::for_test(
                &server.uri(),
                crate::tataplay::Credentials {
                    access_token: "token".into(),
                    refresh_token: "refresh".into(),
                    subscriber_id: "sub-1".into(),
                    sid: "sid-1".into(),
                },
            )));
        assert_eq!(s.active_product(), ActiveProduct::TataPlay);

        let channels = s.effective_channels().await.unwrap();
        assert_eq!(channels.len(), 1);
        let id = &channels[0].id;
        assert!(s.is_playable(id));
        assert!(s.is_drm_channel(id));
        assert!(s.is_dash_channel(id));
        assert!(s.tataplay.playlist_ready(id));
    }

    #[test]
    fn stable_since_keeps_a_response_from_an_unchanged_context() {
        let s = state();
        let epoch = s.secure.current_epoch();
        let resp = s.stable_since(epoch, axum::http::StatusCode::OK.into_response());
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }

    #[test]
    fn stable_since_discards_a_response_that_straddled_a_context_switch() {
        let s = state();
        let epoch = s.secure.current_epoch();
        s.invalidate_context();
        let resp = s.stable_since(epoch, axum::http::StatusCode::OK.into_response());
        assert_eq!(resp.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    fn live(hls: &str, dash: &str, license: &str) -> crate::television::LiveUrlOutput {
        crate::television::LiveUrlOutput {
            bitrates: crate::television::Bitrates {
                auto: hls.to_string(),
                ..Default::default()
            },
            mpd: crate::television::Mpd {
                bitrates: crate::television::Bitrates {
                    auto: dash.to_string(),
                    ..Default::default()
                },
                key: license.to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn state_with_shared_store() -> (Arc<Store>, Arc<AppState>) {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(dir.path().to_str().unwrap()).unwrap());
        std::mem::forget(dir);
        let state = state_with_providers(
            store.clone(),
            Arc::new(ExtrasState::new(false, None)),
            Arc::new(PlayboxState::new(false)),
        );
        (store, state)
    }

    #[test]
    fn playback_learning_tracks_dash_separately_from_licenses() {
        let s = state();
        assert!(!s.is_dash_channel("9701"));
        assert!(!s.is_drm_channel("9701"));

        // HLS only: nothing to learn, and an applied observation clears nothing.
        s.observe_playback("9701", &live("https://cdn/a.m3u8", "", ""));
        assert!(!s.is_dash_channel("9701"));

        // Clear DASH: the MPD route, without a license.
        s.observe_playback("9701", &live("", "https://cdn/a.mpd", ""));
        assert!(s.is_dash_channel("9701"));
        assert!(!s.is_drm_channel("9701"));

        // Protected DASH: both facts.
        s.observe_playback("9702", &live("", "https://cdn/b.mpd", "https://cdn/lic"));
        assert!(s.is_dash_channel("9702"));
        assert!(s.is_drm_channel("9702"));

        // HLS coming back drops the learned routing entirely.
        s.observe_playback(
            "9702",
            &live("https://cdn/b.m3u8", "https://cdn/b.mpd", "https://cdn/lic"),
        );
        assert!(!s.is_dash_channel("9702"));
        assert!(!s.is_drm_channel("9702"));
    }

    #[test]
    fn playback_learning_ignores_channels_from_other_providers() {
        let s = state();
        for id in ["pb_1", "ex_1", "fw_1", "at_1", "tp_1", "vz_1", "custom"] {
            s.observe_playback(id, &live("", "https://cdn/a.mpd", "https://cdn/lic"));
            // Providers answer for their own ids; the learning must not claim
            // them (an `ex_` id is DRM by default before its first playback).
            let learned = s.drm_channels.read().unwrap();
            assert!(!learned.dash.contains(id), "{id} was learned as DASH");
            assert!(!learned.drm.contains(id), "{id} was learned as DRM");
        }
    }

    #[test]
    fn learned_playback_persists_for_the_same_account_only() {
        let (store, s) = state_with_shared_store();
        s.set_learned_playback_account("account-a");
        s.observe_playback("9701", &live("", "https://cdn/a.mpd", "https://cdn/lic"));

        let same = load_learned_playback(&store, "account-a");
        assert!(same.dash.contains("9701"));
        assert!(same.drm.contains("9701"));

        // Another login must not inherit account A's observations.
        let other = load_learned_playback(&store, "account-b");
        assert!(other.dash.is_empty());
        assert!(other.drm.is_empty());

        // Switching in memory drops them as well.
        s.set_learned_playback_account("account-b");
        assert!(!s.is_dash_channel("9701"));
        assert!(!s.is_drm_channel("9701"));
    }

    #[test]
    fn malformed_learned_entries_are_ignored() {
        let (store, _s) = state_with_shared_store();
        store.set(STORE_KEY_LEARNED_FOR, "acct").unwrap();
        store
            .set(STORE_KEY_LEARNED_DASH, " 154 , ,x,99999999,0001,car")
            .unwrap();
        store.set(STORE_KEY_LEARNED_DRM, "154,99999999,abc").unwrap();

        let learned = load_learned_playback(&store, "acct");
        assert_eq!(learned.dash.len(), 3, "{:?}", learned.dash);
        assert_eq!(learned.drm.len(), 2, "{:?}", learned.drm);
        assert!(learned.drm.iter().all(|id| learned.dash.contains(id)));
    }

    #[test]
    fn disabling_drm_keeps_clear_dash_and_hides_protected_dash() {
        let mut s = state();
        Arc::get_mut(&mut s).unwrap().config.drm = false;

        s.learn_playback("9702", true);
        assert!(!s.is_drm_channel("9702"));
        assert!(
            !s.is_dash_channel("9702"),
            "protected DASH cannot route while DRM is disabled"
        );

        s.learn_playback("9703", false);
        assert!(s.is_dash_channel("9703"));
        assert!(!s.is_drm_channel("9703"));
    }
}
