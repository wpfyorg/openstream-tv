//! PlayboxTV provider (`playboxtv.tv.android.in`, 5.20.1/128).
//!
//! Evidence: `docs/ott-providers/playbox.md`. Static analysis recovered base
//! domains and endpoint *paths*; request/response schemas were not present as
//! const-strings, so every wire shape here is a tolerant candidate parsed
//! with `#[serde(default)]` + `Option` fields and covered by offline mock
//! tests. Nothing here claims runtime-observed schemas.
//!
//! IDs are collision-safe: `pb_<opaque>` never collides with numeric JioTV
//! IDs or `ex_*`. (`sl_*` is avoided: `epg.rs` strips a literal `"sl"`.)

#![allow(dead_code)] // Evidence constants + runtime-gap hooks exercised by tests.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const ID_PREFIX: &str = "pb_";
pub const PACKAGE: &str = "playboxtv.tv.android.in";
pub const VERSION_NAME: &str = "5.20.1";
pub const VERSION_CODE: u32 = 128;
pub const ARTIFACT_SHA256: &str =
    "570ca55d1d88f68a8f4dd9a7a9fd586682f8fa1dd6738b3b6623577d77f858f8";

/// Observed 1st-party bases (static strings; mapping to Retrofit services is
/// a runtime gap).
pub const BASE_USER_API: &str = "https://user-api.playboxtv.in";
pub const BASE_CONSUME: &str = "https://content-consume.playboxtv.in";
pub const PATH_SEND_OTP: &str = "customer/login";
pub const PATH_VERIFY_OTP: &str = "customer/verify";
pub const PATH_SSO_VERIFY: &str = "prod/v1/user/auth/sso/verify";
pub const PATH_RESOLVE: &str = "consume/v1/playback/resolve";
pub const PATH_PARTNER_TOKEN: &str = "consume/v1/partner-token/ltv/jio/sso";
pub const PATH_LIVE_PAGE: &str = "v3/page/livetv2";
pub const PATH_WEEKLY_EPG: &str = "channel/weekly_epg/";
pub const PATH_CHANNEL_EPG: &str = "channel/epg";
pub const PATH_SUBSCRIPTION: &str = "user/subscription";

pub const STORE_KEY_DEVICE: &str = "playbox_device";
pub const STORE_KEY_CREDENTIALS: &str = "playbox_credentials";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    #[serde(default)]
    pub android_id: String,
    #[serde(default)]
    pub advertising_id: String,
    #[serde(default = "default_model")]
    pub model: String,
}

fn default_model() -> String {
    "AFTKA".to_string()
}

impl Device {
    pub fn new() -> Device {
        let mut bytes = [0u8; 8];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
        Device {
            android_id: hex::encode(bytes),
            advertising_id: String::new(),
            model: default_model(),
        }
    }

    pub fn load_or_create(store: &crate::store::Store) -> anyhow::Result<Device> {
        if let Some(json) = store.get_opt(STORE_KEY_DEVICE) {
            if let Ok(d) = serde_json::from_str::<Device>(&json) {
                if !d.android_id.is_empty() {
                    return Ok(d);
                }
            }
        }
        let d = Device::new();
        store.set(STORE_KEY_DEVICE, &serde_json::to_string(&d)?)?;
        Ok(d)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Credentials {
    #[serde(default)]
    pub number: String,
    #[serde(default, rename = "authToken")]
    pub auth_token: String,
    #[serde(default, rename = "partnerToken")]
    pub partner_token: String,
    #[serde(default, rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(default)]
    pub expires_at: u64,
}

impl Credentials {
    pub fn needs_refresh(&self, now: SystemTime, margin: Duration) -> bool {
        if self.auth_token.is_empty() {
            return true;
        }
        if self.expires_at == 0 {
            return true;
        }
        let exp = UNIX_EPOCH + Duration::from_secs(self.expires_at);
        now + margin >= exp
    }

    pub fn load(store: &crate::store::Store) -> Option<Credentials> {
        let json = store.get_opt(STORE_KEY_CREDENTIALS)?;
        let cr: Credentials = serde_json::from_str(&json).ok()?;
        if cr.auth_token.is_empty() {
            return None;
        }
        Some(cr)
    }

    pub fn save(&self, store: &crate::store::Store) -> anyhow::Result<()> {
        store.set(STORE_KEY_CREDENTIALS, &serde_json::to_string(self)?)?;
        Ok(())
    }
}

pub fn delete_credentials(store: &crate::store::Store) -> anyhow::Result<()> {
    let _ = store.delete(STORE_KEY_CREDENTIALS);
    Ok(())
}

pub fn content_id(channel_id: &str) -> Option<&str> {
    channel_id.strip_prefix(ID_PREFIX)
}

pub fn channel_id(content_id: &str) -> String {
    format!("{ID_PREFIX}{content_id}")
}

pub fn is_playbox_channel(id: &str) -> bool {
    id.starts_with(ID_PREFIX)
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct OtpSendResponse {
    #[serde(default)]
    pub status: bool,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct OtpVerifyResponse {
    #[serde(default, rename = "authToken")]
    pub auth_token: String,
    #[serde(default, rename = "partnerToken")]
    pub partner_token: String,
    #[serde(default, rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(default, rename = "expiresIn")]
    pub expires_in: u64,
    #[serde(default)]
    pub status: bool,
}

/// Tolerant catalogue item: CloudFront page items carry per-provider
/// subscription flags; every field is optional so schema drift failssoft.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PageItem {
    #[serde(default)]
    pub id: String,
    #[serde(default, rename = "contentId")]
    pub content_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub logo: String,
    #[serde(default, rename = "logoUrl")]
    pub logo_url: String,
    #[serde(default)]
    pub genre: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub subscription: Option<bool>,
}

impl PageItem {
    pub fn opaque_id(&self) -> String {
        if !self.content_id.is_empty() {
            self.content_id.clone()
        } else if !self.id.is_empty() {
            self.id.clone()
        } else {
            self.title.clone()
        }
    }

    pub fn display_name(&self) -> String {
        if !self.title.is_empty() {
            self.title.clone()
        } else {
            self.name.clone()
        }
    }

    pub fn logo_img(&self) -> String {
        if !self.logo_url.is_empty() {
            self.logo_url.clone()
        } else {
            self.logo.clone()
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PageResponse {
    #[serde(default)]
    pub result: Vec<PageItem>,
    #[serde(default)]
    pub data: Vec<PageItem>,
    #[serde(default)]
    pub items: Vec<PageItem>,
}

impl PageResponse {
    pub fn items(&self) -> Vec<PageItem> {
        if !self.result.is_empty() {
            self.result.clone()
        } else if !self.data.is_empty() {
            self.data.clone()
        } else {
            self.items.clone()
        }
    }
}

/// Observed `PlayContentRespModel` shape: `{Url, Provider, Headers, …}`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PlaybackResponse {
    #[serde(default, rename = "Url")]
    pub url: String,
    #[serde(default, rename = "url")]
    pub url_lower: String,
    #[serde(default, rename = "Provider")]
    pub provider: String,
    #[serde(default, rename = "Headers")]
    pub headers: HashMap<String, String>,
    #[serde(default, rename = "headers")]
    pub headers_lower: HashMap<String, String>,
    #[serde(default, rename = "Message")]
    pub message: String,
    #[serde(default, rename = "LicenseUrl")]
    pub license_url: String,
    #[serde(default, rename = "licenseUrl")]
    pub license_url_lower: String,
}

impl PlaybackResponse {
    pub fn stream_url(&self) -> &str {
        if !self.url.is_empty() {
            &self.url
        } else {
            &self.url_lower
        }
    }

    pub fn merged_headers(&self) -> HashMap<String, String> {
        let mut out = self.headers_lower.clone();
        for (k, v) in &self.headers {
            out.insert(k.clone(), v.clone());
        }
        out
    }

    pub fn resolved_license_url(&self) -> &str {
        if !self.license_url.is_empty() {
            &self.license_url
        } else {
            &self.license_url_lower
        }
    }

    pub fn to_live_url_output(&self) -> crate::television::LiveUrlOutput {
        let url = self.stream_url().to_string();
        let is_mpd = url.contains(".mpd") || url.contains("mpd");
        let license = self.resolved_license_url().to_string();
        let is_drm = !license.is_empty();
        let mut out = crate::television::LiveUrlOutput {
            result: "OK".to_string(),
            ..Default::default()
        };
        if is_mpd {
            out.mpd.auto = url;
        } else {
            out.bitrates.auto = url;
        }
        if is_drm {
            out.mpd.key = license;
            out.is_drm = true;
        }
        out
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SubscriptionResponse {
    #[serde(default)]
    pub active: bool,
    #[serde(default, rename = "hasActivePacks")]
    pub has_active_packs: bool,
    #[serde(default)]
    pub packs: Vec<String>,
}

impl SubscriptionResponse {
    pub fn entitled(&self) -> bool {
        self.active || self.has_active_packs || !self.packs.is_empty()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EpgEntry {
    #[serde(default, rename = "channelId")]
    pub channel_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub start: i64,
    #[serde(default)]
    pub end: i64,
}

pub struct Client {
    pub http: reqwest::Client,
    pub base_user_api: String,
    pub base_consume: String,
    pub device: Device,
    pub creds: RwLock<Credentials>,
}

impl Client {
    pub fn new(http: reqwest::Client, device: Device) -> Client {
        Client {
            http,
            base_user_api: BASE_USER_API.to_string(),
            base_consume: BASE_CONSUME.to_string(),
            device,
            creds: RwLock::new(Credentials::default()),
        }
    }

    pub fn set_endpoints_for_test(&mut self, base_user_api: &str, base_consume: &str) {
        self.base_user_api = base_user_api.to_string();
        self.base_consume = base_consume.to_string();
    }

    fn auth_header(&self) -> Option<String> {
        let c = self.creds.read().unwrap();
        if c.auth_token.is_empty() {
            None
        } else {
            Some(c.auth_token.clone())
        }
    }

    fn valid_number(number: &str) -> bool {
        let digits: String = number.chars().filter(|c| c.is_ascii_digit()).collect();
        digits.len() == 10
    }

    pub async fn send_otp(&self, number: &str) -> anyhow::Result<OtpSendResponse> {
        if !Self::valid_number(number) {
            anyhow::bail!("playbox: mobile number must have 10 digits");
        }
        let url = format!("{}/{PATH_SEND_OTP}", self.base_user_api);
        let resp = self
            .http
            .post(&url)
            .json(&serde_json::json!({"phone": number, "deviceId": self.device.android_id}))
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("playbox: send OTP failed: {}", resp.status());
        }
        Ok(resp.json::<OtpSendResponse>().await?)
    }

    pub async fn verify_otp(
        &self,
        number: &str,
        otp: &str,
        store: &crate::store::Store,
    ) -> anyhow::Result<bool> {
        if !Self::valid_number(number) || otp.trim().is_empty() {
            anyhow::bail!("playbox: bad number or OTP");
        }
        let url = format!("{}/{PATH_VERIFY_OTP}", self.base_user_api);
        let resp = self
            .http
            .post(&url)
            .json(&serde_json::json!({"phone": number, "otp": otp, "deviceId": self.device.android_id}))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Ok(false);
        }
        let body: OtpVerifyResponse = resp.json().await?;
        if body.auth_token.is_empty() {
            return Ok(false);
        }
        let expires_at = if body.expires_in > 0 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                + body.expires_in
        } else {
            0
        };
        let creds = Credentials {
            number: number.to_string(),
            auth_token: body.auth_token,
            partner_token: body.partner_token,
            refresh_token: body.refresh_token,
            expires_at,
        };
        creds.save(store)?;
        *self.creds.write().unwrap() = creds;
        Ok(true)
    }

    pub async fn channels(&self) -> anyhow::Result<Vec<PageItem>> {
        let url = format!("{}/{PATH_LIVE_PAGE}", self.base_user_api);
        let mut req = self.http.get(&url);
        if let Some(token) = self.auth_header() {
            req = req.header("authToken", token);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("playbox: catalogue failed: {}", resp.status());
        }
        Ok(resp.json::<PageResponse>().await?.items())
    }

    pub async fn playback(&self, content_id: &str) -> anyhow::Result<PlaybackResponse> {
        let url = format!("{}/{PATH_RESOLVE}", self.base_consume);
        let mut req = self.http.post(&url).json(
            &serde_json::json!({"contentId": content_id, "deviceId": self.device.android_id}),
        );
        if let Some(token) = self.auth_header() {
            req = req.header("authToken", token);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("playbox: playback resolve failed: {}", resp.status());
        }
        let body: PlaybackResponse = resp.json().await?;
        if body.stream_url().is_empty() {
            anyhow::bail!("playbox: empty playback URL");
        }
        Ok(body)
    }

    pub async fn subscription(&self) -> anyhow::Result<SubscriptionResponse> {
        let url = format!("{}/{PATH_SUBSCRIPTION}", self.base_user_api);
        let mut req = self.http.get(&url);
        if let Some(token) = self.auth_header() {
            req = req.header("authToken", token);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("playbox: subscription check failed: {}", resp.status());
        }
        Ok(resp.json::<SubscriptionResponse>().await?)
    }

    /// Widevine license forwarding: same role as the official client — proxy
    /// the provider-issued license request, never extract keys.
    pub fn license_headers(&self) -> HashMap<String, String> {
        let mut h = HashMap::new();
        if let Some(token) = self.auth_header() {
            h.insert("authToken".to_string(), token);
        }
        h.insert(
            "User-Agent".to_string(),
            "PlayboxTV/5.20.1 (Linux;Android 7.0) AndroidXMedia3/1.4.1".to_string(),
        );
        h
    }
}

pub fn to_television_channel(item: &PageItem) -> crate::television::Channel {
    crate::television::Channel {
        id: channel_id(&item.opaque_id()),
        name: item.display_name(),
        logo_url: item.logo_img(),
        ..Default::default()
    }
}

/// Routing/cache layer for PlayboxTV. Mirrors the `ExtrasState` shape at a
/// smaller scale: gate + connected() + catalogue cache + generation guard so
/// an account switch cannot republish the previous account's data.
pub struct PlayboxState {
    config_enabled: bool,
    client: RwLock<Option<std::sync::Arc<Client>>>,
    catalogue: RwLock<Vec<PageItem>>,
    catalogue_fetched_at: RwLock<Option<std::time::Instant>>,
    generation: std::sync::atomic::AtomicU64,
    commit: std::sync::Mutex<()>,
}

impl PlayboxState {
    pub fn new(config_enabled: bool) -> PlayboxState {
        PlayboxState {
            config_enabled,
            client: RwLock::new(None),
            catalogue: RwLock::new(Vec::new()),
            catalogue_fetched_at: RwLock::new(None),
            generation: std::sync::atomic::AtomicU64::new(0),
            commit: std::sync::Mutex::new(()),
        }
    }

    pub fn enabled(&self) -> bool {
        self.config_enabled
    }

    pub fn connected(&self) -> bool {
        self.client.read().unwrap().is_some()
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
                base_user_api: BASE_USER_API.to_string(),
                base_consume: BASE_CONSUME.to_string(),
                device,
                creds: RwLock::new(creds),
            };
            *self.client.write().unwrap() = Some(std::sync::Arc::new(client));
        }
    }

    pub fn set_client_for_test(&self, client: std::sync::Arc<Client>) {
        *self.client.write().unwrap() = Some(client);
    }

    pub fn clear(&self) {
        let _guard = self.commit.lock().unwrap();
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        *self.catalogue.write().unwrap() = Vec::new();
        *self.catalogue_fetched_at.write().unwrap() = None;
    }

    pub fn logout(&self, store: &crate::store::Store) -> anyhow::Result<()> {
        self.clear();
        *self.client.write().unwrap() = None;
        delete_credentials(store)?;
        Ok(())
    }

    pub fn catalogue_channels(&self) -> Vec<crate::television::Channel> {
        self.catalogue
            .read()
            .unwrap()
            .iter()
            .map(to_television_channel)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn ids_are_collision_safe() {
        assert!(is_playbox_channel("pb_123"));
        assert!(!is_playbox_channel("123"));
        assert!(!is_playbox_channel("ex_123"));
        assert_eq!(content_id("pb_abc"), Some("abc"));
        assert_eq!(channel_id("abc"), "pb_abc");
    }

    #[test]
    fn credentials_round_trip_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let c = Credentials {
            number: "9876543210".to_string(),
            auth_token: "tok".to_string(),
            partner_token: "p".to_string(),
            refresh_token: "r".to_string(),
            expires_at: 9999999999,
        };
        c.save(&store).unwrap();
        let back = Credentials::load(&store).unwrap();
        assert_eq!(back.auth_token, "tok");
        assert!(!back.needs_refresh(SystemTime::now(), Duration::from_secs(60)));
        delete_credentials(&store).unwrap();
        assert!(Credentials::load(&store).is_none());
    }

    #[tokio::test]
    async fn login_catalogue_playback_subscription_flow() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/customer/login"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": true})),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/customer/verify"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": true, "authToken": "AT", "partnerToken": "PT",
                "refreshToken": "RT", "expiresIn": 3600
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v3/page/livetv2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": [{"contentId": "star-plus", "title": "Star Plus",
                            "logoUrl": "https://x/logo.png", "language": "Hindi"}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/consume/v1/playback/resolve"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Url": "https://cdn.example/hls.m3u8", "Provider": "jio",
                "Headers": {"Cookie": "hdnea=1"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/user/subscription"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"active": true})),
            )
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let mut client = Client::new(reqwest::Client::new(), Device::new());
        client.set_endpoints_for_test(&server.uri(), &server.uri());

        let sent = client.send_otp("9876543210").await.unwrap();
        assert!(sent.status);
        assert!(client
            .verify_otp("9876543210", "123456", &store)
            .await
            .unwrap());

        let items = client.channels().await.unwrap();
        assert_eq!(items.len(), 1);
        let tv = to_television_channel(&items[0]);
        assert_eq!(tv.id, "pb_star-plus");

        let pb = client.playback("star-plus").await.unwrap();
        assert_eq!(pb.stream_url(), "https://cdn.example/hls.m3u8");
        let out = pb.to_live_url_output();
        assert!(!out.bitrates.auto.is_empty());

        let sub = client.subscription().await.unwrap();
        assert!(sub.entitled());

        assert!(client.verify_otp("bad", "123456", &store).await.is_err());
        assert!(client.send_otp("123").await.is_err());
    }

    #[tokio::test]
    async fn playback_empty_url_is_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/consume/v1/playback/resolve"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"Url": ""})))
            .mount(&server)
            .await;
        let mut client = Client::new(reqwest::Client::new(), Device::new());
        client.set_endpoints_for_test(&server.uri(), &server.uri());
        assert!(client.playback("x").await.is_err());
    }
}
