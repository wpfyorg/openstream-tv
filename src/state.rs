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

/// Store keys for playback observations learned from successful resolutions.
/// Each key is suffixed with the provider it describes (`… .jiotv`,
/// `… .fastway`), so switching providers keeps every catalogue's facts instead
/// of overwriting them. `...FOR` records the scope the facts belong to
/// (provider + account identity), so another login is not trusted with them.
pub const STORE_KEY_LEARNED_DASH: &str = "playbackLearnedDash";
pub const STORE_KEY_LEARNED_DRM: &str = "playbackLearnedDrm";
pub const STORE_KEY_LEARNED_FOR: &str = "playbackLearnedFor";

/// What this server has observed about a provider's playback: channels whose
/// catalogue entry has no HLS rendition, and which of those also need a
/// Widevine license (a DASH channel is not automatically protected).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LearnedPlayback {
    dash: HashSet<String>,
    drm: HashSet<String>,
    /// Scope (hashed provider + account identity) the facts belong to; empty
    /// when there is no identity to scope them with.
    account: String,
    /// Provider store-key suffix the facts were loaded for.
    product: String,
}

/// Channel IDs this server may record: JioTV's numeric ids and the providers'
/// prefixed id spaces. Deliberately narrow, so a malformed store entry cannot
/// inject anything odd into the playlist path.
pub fn is_learnable_channel_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
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

fn learned_store_key(base: &str, product: &str) -> String {
    format!("{base}.{product}")
}

fn learned_scope_hash(parts: &[&str]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(b"\0");
    }
    hex::encode(hasher.finalize())
}

/// Restores persisted observations for `product`, but only when they were made
/// under `scope`. A missing, malformed or foreign record is ignored.
pub fn load_learned_playback(store: &Store, product: &str, scope: &str) -> LearnedPlayback {
    let mut learned = LearnedPlayback {
        product: product.to_string(),
        ..Default::default()
    };
    if scope.is_empty() {
        return learned;
    }
    match store.get_opt(&learned_store_key(STORE_KEY_LEARNED_FOR, product)) {
        Some(stored) if stored == scope => {}
        _ => return learned,
    }
    learned.dash = parse_learned_ids(
        &store
            .get_opt(&learned_store_key(STORE_KEY_LEARNED_DASH, product))
            .unwrap_or_default(),
    );
    learned.drm = parse_learned_ids(
        &store
            .get_opt(&learned_store_key(STORE_KEY_LEARNED_DRM, product))
            .unwrap_or_default(),
    );
    // Only channels known to be DASH-only can require DASH.
    learned.drm.retain(|id| learned.dash.contains(id));
    learned.account = scope.to_string();
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
        // Learned playback routing belongs to one provider + account; adopting
        // the new context here keeps a switch from carrying facts across.
        self.init_learned_playback();
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
        // Observations only ever add routing; a provider's own answer wins
        // when it has one.
        let learned = self.learned_drm(id);
        if FastwayState::is_fastway_channel(id) {
            return self.fastway.is_drm(id) || learned;
        }
        if crate::playbox::is_playbox_channel(id) {
            return self.playbox.is_drm(id) || learned;
        }
        if crate::airtel::is_airtel_channel(id) {
            return self.airtel.is_drm(id) || learned;
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
                    .unwrap_or_else(|| crate::drm_channels::is_drm_channel(id) || learned)
            }
            None => crate::drm_channels::is_drm_channel(id) || learned,
        }
    }

    /// Whether the channel is DASH even without a Widevine license. A
    /// provider's own metadata answers when it has any (Fastway's catalogue,
    /// Playbox/Airtel's resolved playback, Tata Play's fixed format); the
    /// learned facts cover the rest, including providers whose metadata is not
    /// persisted.
    pub fn is_dash_channel(&self, id: &str) -> bool {
        // A learned channel that is also known to need a license only routes
        // while the DRM pathway is enabled; a clear learned channel stays
        // routable either way.
        let learned = self.learned_dash(id) && (self.config.drm || !self.learned_drm(id));
        if FastwayState::is_fastway_channel(id) {
            // A protected Fastway row is DASH but cannot play without the DRM
            // pathway, so it is not offered as a DASH route while DRM is off.
            let provider_dash =
                self.fastway.is_dash(id) && (self.config.drm || !self.fastway.is_drm(id));
            return provider_dash || learned;
        }
        if crate::playbox::is_playbox_channel(id) {
            return self.playbox.is_dash(id) || learned;
        }
        if crate::airtel::is_airtel_channel(id) {
            return self.airtel.is_dash(id) || learned;
        }
        if crate::tataplay::is_tataplay_channel(id) {
            return true;
        }
        learned
    }

    fn learned_dash(&self, id: &str) -> bool {
        self.drm_channels.read().unwrap().dash.contains(id)
    }

    fn learned_drm(&self, id: &str) -> bool {
        self.drm_channels.read().unwrap().drm.contains(id)
    }

    /// Scope for this session's observations: the active provider plus whatever
    /// account identity it exposes. Empty when nothing identifies an account,
    /// which keeps a logged-out server from persisting or trusting facts.
    pub fn learned_playback_scope(&self) -> String {
        let product = self.active_product();
        let identity = match product {
            ActiveProduct::Tv => crate::login::load(&self.store)
                .map(|creds| creds.learned_account())
                .unwrap_or_default(),
            ActiveProduct::Extras => self
                .extras
                .credentials_marker()
                .map(|(number, sso, auth)| learned_scope_hash(&["extras", &number, &sso, &auth]))
                .unwrap_or_default(),
            ActiveProduct::Fastway => self.fastway.account_fingerprint(),
            ActiveProduct::Playbox => self.playbox.account_fingerprint(),
            ActiveProduct::Vzy => self.vzy.account_fingerprint(),
            ActiveProduct::Airtel => self.airtel.account_fingerprint(),
            ActiveProduct::TataPlay => self.tataplay.account_fingerprint(),
        };
        if identity.is_empty() {
            return String::new();
        }
        learned_scope_hash(&["openstream-learned-v1", product.store_name(), &identity])
    }

    /// Loads the facts that belong to this provider and account. Called at
    /// startup and at every account or provider transition (from
    /// `invalidate_context`), so a switch never carries facts across.
    pub fn init_learned_playback(&self) {
        let product = self.active_product().store_name().to_string();
        let scope = self.learned_playback_scope();
        let restored = load_learned_playback(&self.store, &product, &scope);
        let mut learned = self.drm_channels.write().unwrap();
        if learned.product == product
            && learned.account == scope
            && learned.dash == restored.dash
            && learned.drm == restored.drm
        {
            return;
        }
        learned.product = product;
        // Adopt the current scope even when nothing was restored, so the first
        // observation is persisted under this account instead of being dropped.
        learned.account = scope.clone();
        learned.dash = restored.dash;
        learned.drm = restored.drm;
        // Without an identity there is nothing to persist; the stored facts of
        // a logged-out session are left for the next login of the same account.
        if !scope.is_empty() {
            self.store_learned(&learned);
        }
    }

    /// Records what a successful resolution showed: a channel with an HLS
    /// rendition plays over HLS, a channel that only has DASH needs the MPD
    /// route, and a DASH channel with a license URL needs a license too. Only
    /// successful resolutions are passed in, so transient errors never change
    /// what is stored. Applies to every provider, not just JioTV.
    pub fn observe_playback(&self, id: &str, live: &crate::television::LiveUrlOutput) {
        // A custom channel is a user-supplied URL, not a provider resolution.
        if self.custom_channels.contains(id) || !is_learnable_channel_id(id) {
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
    /// license. A channel that turns out to be clear DASH drops a license fact
    /// learned earlier. Persisted only when an observation actually changes.
    pub(crate) fn learn_playback(&self, id: &str, needs_license: bool) {
        if !is_learnable_channel_id(id) {
            return;
        }
        let mut learned = self.drm_channels.write().unwrap();
        let mut changed = learned.dash.insert(id.to_string());
        if needs_license {
            changed |= learned.drm.insert(id.to_string());
        } else {
            changed |= learned.drm.remove(id);
        }
        if !changed {
            return;
        }
        tracing::info!(
            channel = id,
            needs_license,
            provider = %self.active_product().store_name(),
            "learned a channel's playback route"
        );
        // Persist while the write lock is held: a concurrent observation cannot
        // then write an older snapshot after this one, and both lists always
        // come from a single snapshot.
        self.store_learned(&learned);
    }

    /// Forgets a channel that has an HLS rendition again.
    fn forget_learned_playback(&self, id: &str) {
        let mut learned = self.drm_channels.write().unwrap();
        let removed_dash = learned.dash.remove(id);
        let removed_drm = learned.drm.remove(id);
        if !removed_dash && !removed_drm {
            return;
        }
        tracing::info!(channel = id, "channel has HLS again");
        self.store_learned(&learned);
    }

    /// Writes the account marker and both observation sets in one store
    /// mutation. The caller holds the write lock that produced the snapshot, so
    /// store order matches mutation order and a restart can never read one
    /// account's facts under another's marker.
    fn store_learned(&self, learned: &LearnedPlayback) {
        if learned.account.is_empty() || learned.product.is_empty() {
            return;
        }
        let marker_key = learned_store_key(STORE_KEY_LEARNED_FOR, &learned.product);
        let dash_key = learned_store_key(STORE_KEY_LEARNED_DASH, &learned.product);
        let drm_key = learned_store_key(STORE_KEY_LEARNED_DRM, &learned.product);
        let dash_csv = learned_ids_csv(&learned.dash);
        let drm_csv = learned_ids_csv(&learned.drm);
        if let Err(e) = self.store.set_many(&[
            (marker_key.as_str(), learned.account.as_str()),
            (dash_key.as_str(), dash_csv.as_str()),
            (drm_key.as_str(), drm_csv.as_str()),
        ]) {
            tracing::warn!("cannot save learned playback routing: {e}");
        }
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

    fn jiotv_creds(unique: &str) -> crate::television::Credentials {
        crate::television::Credentials {
            sso_token: "sso".into(),
            crm: "crm".into(),
            unique_id: unique.into(),
            access_token: "access".into(),
            refresh_token: "refresh".into(),
        }
    }

    fn logged_in_state(store: Arc<Store>, unique: &str) -> Arc<AppState> {
        crate::login::save(&store, &jiotv_creds(unique), crate::login::TOUCH_ALL).unwrap();
        let state = state_with_providers(
            store,
            Arc::new(ExtrasState::new(false, None)),
            Arc::new(PlayboxState::new(false)),
        );
        state.init_learned_playback();
        state
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
    fn playback_learning_covers_every_provider_id_space() {
        let s = state();
        for id in ["9701", "fw_77", "pb_1", "ex_1", "at_1", "tp_1", "vz_1"] {
            s.observe_playback(id, &live("", "https://cdn/a.mpd", "https://cdn/lic"));
            let learned = s.drm_channels.read().unwrap();
            assert!(learned.dash.contains(id), "{id} was not learned as DASH");
            assert!(learned.drm.contains(id), "{id} was not learned as DRM");
        }

        // A clear resolution drops the license fact but keeps the route.
        s.observe_playback("fw_77", &live("", "https://cdn/a.mpd", ""));
        assert!(s.is_dash_channel("fw_77"));
        assert!(!s.is_drm_channel("fw_77"));

        // A custom channel is a user URL, never a provider observation.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("custom.json");
        std::fs::write(
            &path,
            r#"{"channels":[{"id":"mine","name":"Mine","url":"https://example.test/mine.m3u8"}]}"#,
        )
        .unwrap();
        s.custom_channels.load(path.to_str().unwrap()).unwrap();
        for channel in s.custom_channels.all() {
            s.observe_playback(&channel.id, &live("", "https://cdn/a.mpd", ""));
            assert!(!s.drm_channels.read().unwrap().dash.contains(&channel.id));
        }
    }

    #[test]
    fn learned_playback_survives_a_restart_for_the_same_account_only() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(dir.path().to_str().unwrap()).unwrap());
        std::mem::forget(dir);

        let first = logged_in_state(store.clone(), "user-a");
        first.observe_playback("9701", &live("", "https://cdn/a.mpd", "https://cdn/lic"));

        // A restart with the same login restores both facts.
        let restarted = logged_in_state(store.clone(), "user-a");
        assert!(restarted.is_dash_channel("9701"));
        assert!(restarted.is_drm_channel("9701"));

        // Another login must not inherit account A's observations.
        crate::login::save(&store, &jiotv_creds("user-b"), crate::login::TOUCH_ALL).unwrap();
        let other = state_with_providers(
            store.clone(),
            Arc::new(ExtrasState::new(false, None)),
            Arc::new(PlayboxState::new(false)),
        );
        other.init_learned_playback();
        assert!(!other.is_dash_channel("9701"));
        assert!(!other.is_drm_channel("9701"));
    }

    #[test]
    fn logging_in_after_startup_scopes_later_observations() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(dir.path().to_str().unwrap()).unwrap());
        std::mem::forget(dir);

        // Server starts logged out, so there is no scope to record facts under.
        let state = state_with_providers(
            store.clone(),
            Arc::new(ExtrasState::new(false, None)),
            Arc::new(PlayboxState::new(false)),
        );
        state.init_learned_playback();
        assert!(state.learned_playback_scope().is_empty());

        // A login with no earlier observation record must adopt the new scope,
        // otherwise the first observation has no account and is never saved.
        crate::login::save(&store, &jiotv_creds("user-a"), crate::login::TOUCH_ALL).unwrap();
        state.init_learned_playback();
        state.observe_playback("9701", &live("", "https://cdn/a.mpd", "https://cdn/lic"));

        let restarted = state_with_providers(
            store.clone(),
            Arc::new(ExtrasState::new(false, None)),
            Arc::new(PlayboxState::new(false)),
        );
        restarted.init_learned_playback();
        assert!(restarted.is_dash_channel("9701"));
        assert!(restarted.is_drm_channel("9701"));
    }

    #[test]
    fn concurrent_observations_all_survive_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(dir.path().to_str().unwrap()).unwrap());
        std::mem::forget(dir);
        let state = logged_in_state(store.clone(), "user-a");

        std::thread::scope(|scope| {
            for worker in 0..8 {
                let state = state.clone();
                scope.spawn(move || {
                    for step in 0..3 {
                        let id = format!("970{worker}{step}");
                        state.observe_playback(&id, &live("", "https://cdn/a.mpd", ""));
                    }
                });
            }
        });

        let reloaded = logged_in_state(store, "user-a");
        for worker in 0..8 {
            for step in 0..3 {
                let id = format!("970{worker}{step}");
                assert!(reloaded.is_dash_channel(&id), "{id} was lost on disk");
            }
        }
    }

    #[test]
    fn protected_fastway_channel_is_not_dash_while_drm_is_disabled() {
        fn fastway_channel(id: i64, encryption_url: &str) -> crate::fastway::FastwayChannel {
            crate::fastway::FastwayChannel {
                id,
                channel_number: id,
                title: format!("Fastway {id}"),
                stream_url: format!("https://cdn.example/{id}.mpd"),
                channel_mode: "live".into(),
                encryption_url: encryption_url.into(),
                ..Default::default()
            }
        }

        let mut s = state();
        s.fastway
            .prime_for_test(vec![fastway_channel(1, "https://cdn.example/license")]);
        assert!(s.is_dash_channel("fw_1"));

        Arc::get_mut(&mut s).unwrap().config.drm = false;
        assert!(
            !s.is_dash_channel("fw_1"),
            "protected Fastway rows need the DRM pathway"
        );

        // A clear Fastway row stays routable with DRM disabled.
        s.fastway.prime_for_test(vec![fastway_channel(2, "")]);
        assert!(s.is_dash_channel("fw_2"));
    }

    #[test]
    fn malformed_learned_entries_are_ignored() {
        let (store, _s) = state_with_shared_store();
        let key = |base: &str| format!("{base}.jiotv");
        store.set(&key(STORE_KEY_LEARNED_FOR), "scope").unwrap();
        store
            .set(
                &key(STORE_KEY_LEARNED_DASH),
                " 154 , ,bad id,a/b,99999999,0001,car",
            )
            .unwrap();
        store
            .set(&key(STORE_KEY_LEARNED_DRM), "154,bad id,99999999")
            .unwrap();

        let learned = load_learned_playback(&store, "jiotv", "scope");
        // Entries with a space or a slash are rejected; ids with letters
        // (another provider's id space) are valid channel identifiers.
        assert_eq!(learned.dash.len(), 4, "{:?}", learned.dash);
        assert_eq!(learned.drm.len(), 2, "{:?}", learned.drm);
        assert!(learned.drm.iter().all(|id| learned.dash.contains(id)));

        // A marker written for another scope is not trusted.
        let foreign = load_learned_playback(&store, "jiotv", "other");
        assert!(foreign.dash.is_empty() && foreign.drm.is_empty());
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

        // The same rule holds for a learned id from another provider's space.
        s.learn_playback("fw_88", true);
        assert!(!s.is_dash_channel("fw_88"));
        s.learn_playback("fw_89", false);
        assert!(s.is_dash_channel("fw_89"));
    }
}
