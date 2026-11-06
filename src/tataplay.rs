//! Tata Play provider (`com.ryzmedia.tatasky`, 509.0/894).
//!
//! Evidence: `docs/ott-providers/tataplay.md`. Only base URLs survived static
//! analysis; the React Native bundle is encrypted, so content/auth paths are
//! unknown. This module persists device/token state, speaks small generic
//! OTP/catalogue/playback/license contracts against the observed base, and
//! marks content schemas as unresolved. IDs use `tp_` (collision-safe).

#![allow(dead_code)] // Evidence constants + runtime-gap hooks exercised by tests.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const ID_PREFIX: &str = "tp_";
pub const PACKAGE: &str = "com.ryzmedia.tatasky";
pub const VERSION_NAME: &str = "509.0";
pub const VERSION_CODE: u32 = 894;
pub const ARTIFACT_SHA256: &str =
    "ab9091ab2a3bb42eeb2be2b6c6e59cc799d17b6af852618c13d433b253b90e14";

/// Observed static bases. Content paths below are generic candidates, not
/// observed endpoints.
pub const BASE_API: &str = "https://tm.tapi.videoready.tv";
pub const BASE_CODEPUSH: &str = "https://cp.tapi.videoready.tv";
pub const BASE_IRDETO: &str = "https://tatasky-csm.live.ott.irdeto.com";
pub const PATH_SEND_OTP: &str = "auth/otp/send";
pub const PATH_VERIFY_OTP: &str = "auth/otp/verify";
pub const PATH_CHANNELS: &str = "content/live/channels";
pub const PATH_PLAYBACK: &str = "content/playback/resolve";
pub const PATH_SUBSCRIPTION: &str = "user/subscription";

pub const STORE_KEY_DEVICE: &str = "tataplay_device";
pub const STORE_KEY_CREDENTIALS: &str = "tataplay_credentials";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    #[serde(default)]
    pub android_id: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_manufacturer")]
    pub manufacturer: String,
}

fn default_model() -> String {
    "AFTKA".to_string()
}

fn default_manufacturer() -> String {
    "Amazon".to_string()
}

impl Device {
    pub fn new() -> Device {
        let mut bytes = [0u8; 8];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
        Device {
            android_id: hex::encode(bytes),
            model: default_model(),
            manufacturer: default_manufacturer(),
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
    #[serde(default, rename = "accessToken")]
    pub access_token: String,
    #[serde(default, rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(default)]
    pub expires_at: u64,
}

impl Credentials {
    pub fn needs_refresh(&self, now: SystemTime, margin: Duration) -> bool {
        if self.access_token.is_empty() {
            return true;
        }
        if self.expires_at == 0 {
            return true;
        }
        now + margin >= UNIX_EPOCH + Duration::from_secs(self.expires_at)
    }

    pub fn load(store: &crate::store::Store) -> Option<Credentials> {
        let json = store.get_opt(STORE_KEY_CREDENTIALS)?;
        let cr: Credentials = serde_json::from_str(&json).ok()?;
        if cr.access_token.is_empty() {
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

pub fn is_tataplay_channel(id: &str) -> bool {
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
    #[serde(default, rename = "accessToken")]
    pub access_token: String,
    #[serde(default, rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(default, rename = "expiresIn")]
    pub expires_in: u64,
    #[serde(default)]
    pub status: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChannelItem {
    #[serde(default)]
    pub id: String,
    #[serde(default, rename = "contentId")]
    pub content_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "logoUrl")]
    pub logo_url: String,
}

impl ChannelItem {
    pub fn opaque_id(&self) -> String {
        if !self.content_id.is_empty() {
            self.content_id.clone()
        } else if !self.id.is_empty() {
            self.id.clone()
        } else {
            self.title.clone()
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChannelsResponse {
    #[serde(default)]
    pub result: Vec<ChannelItem>,
    #[serde(default)]
    pub data: Vec<ChannelItem>,
}

impl ChannelsResponse {
    pub fn items(&self) -> Vec<ChannelItem> {
        if !self.result.is_empty() {
            self.result.clone()
        } else {
            self.data.clone()
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PlaybackResponse {
    #[serde(default, rename = "hlsUrl")]
    pub hls_url: String,
    #[serde(default, rename = "mpdUrl")]
    pub mpd_url: String,
    #[serde(default)]
    pub url: String,
    #[serde(default, rename = "licenseUrl")]
    pub license_url: String,
}

impl PlaybackResponse {
    pub fn stream_url(&self) -> &str {
        if !self.mpd_url.is_empty() {
            &self.mpd_url
        } else if !self.hls_url.is_empty() {
            &self.hls_url
        } else {
            &self.url
        }
    }

    pub fn to_live_url_output(&self) -> crate::television::LiveUrlOutput {
        let mut out = crate::television::LiveUrlOutput {
            result: "OK".to_string(),
            ..Default::default()
        };
        let url = self.stream_url().to_string();
        if url.contains(".mpd") || !self.mpd_url.is_empty() {
            out.mpd.auto = url;
        } else {
            out.bitrates.auto = url;
        }
        if !self.license_url.is_empty() {
            out.mpd.key = self.license_url.clone();
            out.is_drm = true;
        }
        out
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SubscriptionResponse {
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub packs: Vec<String>,
}

impl SubscriptionResponse {
    pub fn entitled(&self) -> bool {
        self.active || !self.packs.is_empty()
    }
}

pub struct Client {
    pub http: reqwest::Client,
    pub base_api: String,
    pub device: Device,
    pub creds: RwLock<Credentials>,
}

impl Client {
    pub fn new(http: reqwest::Client, device: Device) -> Client {
        Client {
            http,
            base_api: BASE_API.to_string(),
            device,
            creds: RwLock::new(Credentials::default()),
        }
    }

    pub fn set_base_for_test(&mut self, base: &str) {
        self.base_api = base.to_string();
    }

    fn bearer(&self) -> Option<String> {
        let c = self.creds.read().unwrap();
        if c.access_token.is_empty() {
            None
        } else {
            Some(format!("Bearer {}", c.access_token))
        }
    }

    fn valid_number(number: &str) -> bool {
        number.chars().filter(|c| c.is_ascii_digit()).count() == 10
    }

    pub async fn send_otp(&self, number: &str) -> anyhow::Result<OtpSendResponse> {
        if !Self::valid_number(number) {
            anyhow::bail!("tataplay: mobile number must have 10 digits");
        }
        let url = format!("{}/{PATH_SEND_OTP}", self.base_api);
        let resp = self
            .http
            .post(&url)
            .json(&serde_json::json!({"mobileNumber": number, "deviceId": self.device.android_id}))
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("tataplay: send OTP failed: {}", resp.status());
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
            anyhow::bail!("tataplay: bad number or OTP");
        }
        let url = format!("{}/{PATH_VERIFY_OTP}", self.base_api);
        let resp = self
            .http
            .post(&url)
            .json(&serde_json::json!({"mobileNumber": number, "otp": otp, "deviceId": self.device.android_id}))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Ok(false);
        }
        let body: OtpVerifyResponse = resp.json().await?;
        if body.access_token.is_empty() {
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
            access_token: body.access_token,
            refresh_token: body.refresh_token,
            expires_at,
        };
        creds.save(store)?;
        *self.creds.write().unwrap() = creds;
        Ok(true)
    }

    pub async fn channels(&self) -> anyhow::Result<Vec<ChannelItem>> {
        let url = format!("{}/{PATH_CHANNELS}", self.base_api);
        let mut req = self.http.get(&url);
        if let Some(auth) = self.bearer() {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("tataplay: catalogue failed: {}", resp.status());
        }
        Ok(resp.json::<ChannelsResponse>().await?.items())
    }

    pub async fn playback(&self, content_id: &str) -> anyhow::Result<PlaybackResponse> {
        let url = format!("{}/{PATH_PLAYBACK}", self.base_api);
        let mut req = self.http.post(&url).json(
            &serde_json::json!({"contentId": content_id, "deviceId": self.device.android_id}),
        );
        if let Some(auth) = self.bearer() {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("tataplay: playback resolve failed: {}", resp.status());
        }
        let body: PlaybackResponse = resp.json().await?;
        if body.stream_url().is_empty() {
            anyhow::bail!("tataplay: empty playback URL");
        }
        Ok(body)
    }

    pub async fn subscription(&self) -> anyhow::Result<SubscriptionResponse> {
        let url = format!("{}/{PATH_SUBSCRIPTION}", self.base_api);
        let mut req = self.http.get(&url);
        if let Some(auth) = self.bearer() {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("tataplay: subscription check failed: {}", resp.status());
        }
        Ok(resp.json::<SubscriptionResponse>().await?)
    }

    /// Irdeto/Widevine license forwarding headers. The license URL itself is
    /// expected at runtime (Irdeto CSM); this only supplies auth/UA like the
    /// official client would.
    pub fn license_headers(&self) -> HashMap<String, String> {
        let mut h = HashMap::new();
        if let Some(auth) = self.bearer() {
            h.insert("Authorization".to_string(), auth);
        }
        h.insert(
            "User-Agent".to_string(),
            "TataPlay/509.0 (Linux;Android 8.1) AndroidXMedia3/1.4.1".to_string(),
        );
        h
    }
}

pub fn to_television_channel(item: &ChannelItem) -> crate::television::Channel {
    let name = if !item.title.is_empty() {
        item.title.clone()
    } else {
        item.name.clone()
    };
    crate::television::Channel {
        id: channel_id(&item.opaque_id()),
        name,
        logo_url: item.logo_url.clone(),
        ..Default::default()
    }
}

pub struct TataplayState {
    config_enabled: bool,
    client: RwLock<Option<std::sync::Arc<Client>>>,
    catalogue: RwLock<Vec<ChannelItem>>,
    generation: std::sync::atomic::AtomicU64,
    commit: std::sync::Mutex<()>,
}

impl TataplayState {
    pub fn new(config_enabled: bool) -> TataplayState {
        TataplayState {
            config_enabled,
            client: RwLock::new(None),
            catalogue: RwLock::new(Vec::new()),
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
                base_api: BASE_API.to_string(),
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
        assert!(is_tataplay_channel("tp_1"));
        assert!(!is_tataplay_channel("1"));
        assert!(!is_tataplay_channel("ex_1"));
        assert!(!is_tataplay_channel("pb_1"));
        assert_eq!(channel_id("a"), "tp_a");
        assert_eq!(content_id("tp_a"), Some("a"));
    }

    #[test]
    fn credentials_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let c = Credentials {
            number: "9876543210".to_string(),
            access_token: "AT".to_string(),
            refresh_token: "RT".to_string(),
            expires_at: 9999999999,
        };
        c.save(&store).unwrap();
        assert_eq!(Credentials::load(&store).unwrap().access_token, "AT");
        delete_credentials(&store).unwrap();
        assert!(Credentials::load(&store).is_none());
    }

    #[tokio::test]
    async fn login_catalogue_playback_subscription_flow() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/auth/otp/send"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": true})),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/auth/otp/verify"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": true, "accessToken": "AT", "refreshToken": "RT", "expiresIn": 3600
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/content/live/channels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": [{"contentId": "sony-sab", "title": "Sony SAB", "logoUrl": "https://x/s.png"}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/content/playback/resolve"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "mpdUrl": "https://cdn.example/live.mpd", "licenseUrl": "https://lic.example/wv"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/user/subscription"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"active": false, "packs": ["base"]})),
            )
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let mut client = Client::new(reqwest::Client::new(), Device::new());
        client.set_base_for_test(&server.uri());

        assert!(client.send_otp("9876543210").await.unwrap().status);
        assert!(client
            .verify_otp("9876543210", "123456", &store)
            .await
            .unwrap());

        let items = client.channels().await.unwrap();
        assert_eq!(to_television_channel(&items[0]).id, "tp_sony-sab");

        let pb = client.playback("sony-sab").await.unwrap();
        assert_eq!(pb.stream_url(), "https://cdn.example/live.mpd");
        let out = pb.to_live_url_output();
        assert_eq!(out.mpd.key, "https://lic.example/wv");
        assert!(out.is_drm);

        assert!(client.subscription().await.unwrap().entitled());
        assert!(client.send_otp("12").await.is_err());
    }
}
