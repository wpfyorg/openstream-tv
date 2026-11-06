//! PlayboxTV provider (`playboxtv.tv.android.in`, 5.20.1/128).
//!
//! Evidence: `docs/ott-providers/playbox.md`. Every endpoint, method, header,
//! form field and response field below was recovered from the shipped app's
//! Retrofit interfaces (`playboxtv.tv.in.api.Api`, `LoginAPIService`,
//! `PlayableAPIService`) and their model classes — not guessed. Schemas that
//! could not be established (EPG programme timing, VOD, search) are not
//! implemented and are reported as unavailable by the API layer.
//!
//! IDs are collision-safe: `pb_<contentId>` never collides with numeric JioTV
//! IDs or `ex_*`. (`sl_*` is avoided: `epg.rs` strips a literal `"sl"`.)

use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use std::time::Duration;

pub const ID_PREFIX: &str = "pb_";
pub const PACKAGE: &str = "playboxtv.tv.android.in";
pub const VERSION_NAME: &str = "5.20.1";
pub const VERSION_CODE: u32 = 128;
pub const ARTIFACT_SHA256: &str =
    "570ca55d1d88f68a8f4dd9a7a9fd586682f8fa1dd6738b3b6623577d77f858f8";

// Bases recovered from `LoginAPIService` / `PlayableAPIService` / `Api`.
pub(crate) const BASE_LOGIN_V2: &str =
    "https://h6el578034.execute-api.ap-south-1.amazonaws.com/prod/v2/";
pub(crate) const BASE_PLAYER_V2: &str =
    "https://e2vqkprvpg.execute-api.ap-south-1.amazonaws.com/prod/v2/";
pub(crate) const BASE_USER_V1: &str =
    "https://bk2zz20m15.execute-api.ap-south-1.amazonaws.com/prod/v1/";
pub(crate) const BASE_CATALOGUE: &str = "https://da1scxjqfmdtz.cloudfront.net/";
pub(crate) const BASE_CONSUME: &str = "https://content-consume.playboxtv.in/";

// Client-embedded AWS API Gateway keys. These are constants shipped in the
// public client, not account credentials; each is scoped to the routes the
// same Retrofit interface uses.
const KEY_COMMUNICATION: &str = "3NpDTqFKDC1WILqOcoS6b28IJyfETN9M6gXM4Kba";
const KEY_CATALOGUE: &str = "26ltmN6aCCcIVAmUDcEm8gYkTSmS9vIaxdJ11P30";
const KEY_PLAYER: &str = "0StxirdK3I8H4dPPvuhAi1R2ABupm9v85LESwRxr";
const KEY_USER: &str = "citfh0iTJ95ll97c5VIp03QsrglazByR9iIZILek";

/// `appCode` posted by `playboxgetOTP`: `PlayEvent.TYPE` from the Mux SDK.
const APP_CODE: &str = "play";
/// `messageId` posted by `playboxgetOTP` (literal 10 in `LoginAPIService`).
const MESSAGE_ID: i32 = 10;
/// `checkIdentifier` returns "1" for `playboxtv.tv.android.in`.
const IDENTIFIER: &str = "1";
/// `checkCountryCode` returns `BaseInterceptor.BUSINESS` ("in") for the Indian
/// package.
const COUNTRY_CODE: &str = "in";
/// `SharedPreferencesHelper.getLangList()` default.
const DEFAULT_LANGS: &str = "hi";
/// `type` sent for live channels (`LiveTVVideoPlayerActivity.type`).
const LIVE_TYPE: &str = "liveTV";

#[cfg(not(test))]
const CATALOGUE_TIMEOUT: Duration = Duration::from_secs(15);
#[cfg(test)]
const CATALOGUE_TIMEOUT: Duration = Duration::from_millis(500);
#[cfg(not(test))]
const PLAYBACK_RESOLVE_TIMEOUT: Duration = Duration::from_secs(15);
#[cfg(test)]
const PLAYBACK_RESOLVE_TIMEOUT: Duration = Duration::from_millis(500);
#[cfg(not(test))]
const ACCOUNT_HTTP_TIMEOUT: Duration = Duration::from_secs(15);
#[cfg(test)]
const ACCOUNT_HTTP_TIMEOUT: Duration = Duration::from_millis(500);

pub const STORE_KEY_DEVICE: &str = "playbox_device";
pub const STORE_KEY_CREDENTIALS: &str = "playbox_credentials";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    #[serde(default)]
    pub android_id: String,
}

impl Device {
    /// `CheckPackage.getDeviceID` uses `Settings.Secure.ANDROID_ID`, a 16-char
    /// lowercase hex value.
    pub fn new() -> Device {
        let mut bytes = [0u8; 8];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
        Device {
            android_id: hex::encode(bytes),
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

/// Persisted login session. The token is the opaque value the app stores from
/// `OTPRequest.data.token`; no refresh endpoint exists in the app, so none is
/// implemented here.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Credentials {
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub customer_id: String,
}

impl Credentials {
    pub fn load(store: &crate::store::Store) -> Option<Credentials> {
        let json = store.get_opt(STORE_KEY_CREDENTIALS)?;
        let cr: Credentials = serde_json::from_str(&json).ok()?;
        if cr.token.is_empty() || cr.phone.is_empty() {
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
    store.delete(STORE_KEY_CREDENTIALS)?;
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

/// The `data` half of `OTPRequest` (`model/Auth/data.kt`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SessionData {
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default, rename = "customer_id")]
    pub customer_id: String,
    #[serde(default)]
    pub id: i64,
}

/// `OTPRequest`: `{statusCode, message, data, result}`. The app checks
/// `statusCode == 200` and saves `data.token`.
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)] // `message` is part of the observed envelope.
pub struct OtpEnvelope {
    #[serde(default, rename = "statusCode")]
    pub status_code: i32,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub data: Option<SessionData>,
    #[serde(default)]
    pub result: Option<SessionData>,
}

impl OtpEnvelope {
    pub fn session(&self) -> Option<&SessionData> {
        self.data
            .as_ref()
            .filter(|d| !d.token.is_empty())
            .or_else(|| self.result.as_ref().filter(|d| !d.token.is_empty()))
    }
}

/// `model/play/Headers` — media request headers returned with a playback URL.
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)] // every field is part of the observed response model.
pub struct PlayHeaders {
    #[serde(default, rename = "userAgent")]
    pub user_agent: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub subscriber_id: String,
    #[serde(default)]
    pub platform: String,
    #[serde(default, rename = "partnerId")]
    pub partner_id: String,
}

/// `model/play/Url` — the resolved playback source.
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)] // unused fields are retained from the observed model.
pub struct PlayUrl {
    #[serde(default)]
    pub playable: String,
    #[serde(default, rename = "licenceUrl")]
    pub licence_url: String,
    #[serde(default, rename = "certificateUrl")]
    pub certificate_url: String,
    #[serde(default)]
    pub token: String,
    #[serde(default, rename = "adsUrl")]
    pub ads_url: String,
    #[serde(default, rename = "isLive")]
    pub is_live: bool,
    #[serde(default)]
    pub headers: Option<PlayHeaders>,
}

/// `model/play/Message` — `playStatus` must be true before the app plays.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PlayMessage {
    #[serde(default, rename = "playStatus")]
    pub play_status: bool,
    #[serde(default, rename = "validationMessage")]
    pub validation_message: String,
}

/// `model/play/PlayContentsRes`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PlayContents {
    #[serde(default)]
    pub url: Option<PlayUrl>,
    #[serde(default)]
    pub message: Option<PlayMessage>,
}

/// `model/play/PlayContentRespModel` — the `playback/resolve` response.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResolveEnvelope {
    #[serde(default)]
    pub code: i32,
    #[serde(default)]
    pub msg: String,
    #[serde(default)]
    pub data: Option<PlayContents>,
}

/// `model/Item` — a catalogue card, for both `page.body.rows[].items[]` and
/// the legacy `livetv` list.
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)] // `channelId`/`channelNumber` are observed but not needed here.
pub struct Item {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "channelId")]
    pub channel_id: String,
    #[serde(default, rename = "channelNumber")]
    pub channel_number: i32,
    #[serde(default, rename = "typeId")]
    pub type_id: i32,
    #[serde(default)]
    pub typeid: i32,
    #[serde(default)]
    pub media: Option<Media>,
    #[serde(default)]
    pub text: Option<Text>,
}

impl Item {
    pub fn content_id(&self) -> &str {
        &self.id
    }

    pub fn display_name(&self) -> String {
        if let Some(t) = &self.text {
            if !t.title.is_empty() {
                return t.title.clone();
            }
        }
        self.name.clone()
    }

    pub fn logo(&self) -> String {
        let Some(media) = &self.media else {
            return String::new();
        };
        if !media.thumbnail.is_empty() {
            media.thumbnail.clone()
        } else {
            media.poster.clone()
        }
    }

    /// The app passes `Item.typeId` to playback; the legacy `typeid` field is
    /// used only when `typeId` is absent.
    pub fn playback_type_id(&self) -> i32 {
        if self.type_id != 0 {
            self.type_id
        } else {
            self.typeid
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Media {
    #[serde(default)]
    pub thumbnail: String,
    #[serde(default)]
    pub poster: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Text {
    #[serde(default)]
    pub title: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Rows {
    #[serde(default)]
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Body {
    #[serde(default)]
    pub rows: Vec<Rows>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Page {
    #[serde(default)]
    pub body: Option<Body>,
}

/// `model/ImageSLider` — the catalogue envelope.
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)]
pub struct CatalogueEnvelope {
    #[serde(default, rename = "statusCode")]
    pub status_code: i64,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub page: Option<Page>,
    #[serde(default)]
    pub livetv: Vec<Item>,
    #[serde(default)]
    pub data: Vec<Item>,
}

impl CatalogueEnvelope {
    /// Walks the page rows first (the live-TV screen renders those), then the
    /// legacy `livetv`/`data` lists, de-duplicating by content id.
    pub fn channels(&self) -> Vec<Item> {
        let mut out: Vec<Item> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut push = |item: &Item| {
            if item.content_id().is_empty() || !valid_content_id(item.content_id()) {
                return;
            }
            if seen.insert(item.content_id().to_string()) {
                out.push(item.clone());
            }
        };
        if let Some(page) = &self.page {
            if let Some(body) = &page.body {
                for row in &body.rows {
                    for item in &row.items {
                        push(item);
                    }
                }
            }
        }
        for item in self.livetv.iter().chain(self.data.iter()) {
            push(item);
        }
        out
    }
}

/// `model/activepack/ActivePacks` (only the fields this backend reads).
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)] // statusCode/message are observed but their semantics are not.
pub struct ActivePacks {
    #[serde(default)]
    pub active: bool,
    #[serde(default, rename = "bundleActive")]
    pub bundle_active: bool,
    #[serde(default, rename = "statusCode")]
    pub status_code: i32,
    #[serde(default)]
    pub message: String,
}

impl ActivePacks {
    pub fn entitled(&self) -> bool {
        self.active || self.bundle_active
    }
}

/// Content ids are used verbatim in local route paths and playlists, so only
/// URL- and M3U-safe characters are accepted.
pub fn valid_content_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// A resolved playback, cached briefly so the media/license proxy can reuse
/// the response headers and DRM token without re-resolving on every segment.
#[derive(Debug, Clone)]
pub struct ResolvedPlayback {
    pub playable: String,
    pub license_url: String,
    pub media_headers: Vec<(String, String)>,
    pub license_headers: Vec<(String, String)>,
    pub is_dash: bool,
}

impl ResolvedPlayback {
    fn from_contents(contents: &PlayContents) -> anyhow::Result<ResolvedPlayback> {
        let url = contents
            .url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("playbox: playback response has no url"))?;
        if url.playable.is_empty() {
            anyhow::bail!("playbox: playback response has an empty playable URL");
        }
        if let Some(message) = &contents.message {
            if !message.play_status {
                anyhow::bail!(
                    "playbox: playback refused: {}",
                    message.validation_message()
                );
            }
        }
        let media_headers = media_headers(url);
        let license_headers = license_headers(&url.token);
        Ok(ResolvedPlayback {
            playable: url.playable.clone(),
            license_url: url.licence_url.clone(),
            media_headers,
            license_headers,
            is_dash: url.playable.contains(".mpd"),
        })
    }
}

impl PlayMessage {
    fn validation_message(&self) -> &str {
        if self.validation_message.is_empty() {
            "not playable"
        } else {
            &self.validation_message
        }
    }
}

/// Media headers exactly as `PlayerActivity.preparePlayerApp$lambda$3` and
/// `IntroVideoPlayerActivity` set them.
fn media_headers(url: &PlayUrl) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(h) = &url.headers {
        if !h.user_id.is_empty() {
            out.push(("user_id".to_string(), h.user_id.clone()));
        }
        if !h.user_agent.is_empty() {
            out.push(("User-Agent".to_string(), h.user_agent.clone()));
        }
        if !h.subscriber_id.is_empty() {
            out.push(("subscriber_id".to_string(), h.subscriber_id.clone()));
        }
    }
    if !url.token.is_empty() {
        // `tokenArgs` is either a CloudFront cookie string or a bearer token.
        if url.token.contains("CloudFront-") {
            out.push(("Cookie".to_string(), url.token.clone()));
        } else {
            out.push(("Authorization".to_string(), format!("Bearer {}", url.token)));
        }
    }
    out
}

/// License request headers exactly as
/// `PlayerActivity.buildCustomDrmSessionManager` sets them: the playback token
/// is presented in each multi-DRM header the app supports.
fn license_headers(token: &str) -> Vec<(String, String)> {
    if token.is_empty() {
        return Vec::new();
    }
    [
        "token",
        "X-AxDRM-Message",
        "pallycon-customdata-v2",
        "nv-authorizations",
    ]
    .into_iter()
    .map(|name| (name.to_string(), token.to_string()))
    .chain([
        ("Authorization".to_string(), format!("Bearer {token}")),
        ("Cookie".to_string(), token.to_string()),
    ])
    .collect()
}

pub struct Client {
    pub http: reqwest::Client,
    pub login_v2: String,
    pub player_v2: String,
    pub user_v1: String,
    pub catalogue: String,
    pub consume: String,
    pub device: Device,
    pub creds: RwLock<Credentials>,
}

impl Client {
    pub fn new(http: reqwest::Client, device: Device) -> Client {
        Client {
            http,
            login_v2: BASE_LOGIN_V2.to_string(),
            player_v2: BASE_PLAYER_V2.to_string(),
            user_v1: BASE_USER_V1.to_string(),
            catalogue: BASE_CATALOGUE.to_string(),
            consume: BASE_CONSUME.to_string(),
            device,
            creds: RwLock::new(Credentials::default()),
        }
    }

    #[cfg(test)]
    pub fn set_bases_for_test(&mut self, base: &str) {
        self.login_v2 = format!("{base}/login/v2/");
        self.player_v2 = format!("{base}/player/v2/");
        self.user_v1 = format!("{base}/user/v1/");
        self.catalogue = format!("{base}/catalogue/");
        self.consume = format!("{base}/");
    }

    fn bearer(&self) -> Option<String> {
        let c = self.creds.read().unwrap();
        if c.token.is_empty() {
            None
        } else {
            Some(format!("Bearer {}", c.token))
        }
    }

    fn phone(&self) -> Option<String> {
        let c = self.creds.read().unwrap();
        if c.phone.is_empty() {
            None
        } else {
            Some(c.phone.clone())
        }
    }

    fn valid_number(number: &str) -> bool {
        number.chars().filter(|c| c.is_ascii_digit()).count() == 10
    }

    /// `LoginAPIService.playboxgetOTP` → `Api.OTPRequest`:
    /// `POST prod/v2/communication` form-encoded.
    pub async fn send_otp(&self, number: &str) -> anyhow::Result<OtpEnvelope> {
        if !Self::valid_number(number) {
            anyhow::bail!("playbox: mobile number must have 10 digits");
        }
        let url = format!("{}communication", self.login_v2);
        tokio::time::timeout(ACCOUNT_HTTP_TIMEOUT, async {
            let resp = self
                .http
                .post(&url)
                .header("X-API-Key", KEY_COMMUNICATION)
                .form(&[
                    ("appCode", APP_CODE),
                    ("mobileNo", number),
                    ("messageId", &MESSAGE_ID.to_string()),
                    ("countryCode", COUNTRY_CODE),
                ])
                .send()
                .await?;
            if !resp.status().is_success() {
                anyhow::bail!("playbox: send OTP failed: HTTP {}", resp.status());
            }
            Ok(resp.json::<OtpEnvelope>().await?)
        })
        .await
        .map_err(|_| anyhow::anyhow!("playbox: send OTP timed out"))?
    }

    /// `Api.OTPVerification`: `POST prod/v2/communication/otp/verify`.
    pub async fn verify_otp(&self, number: &str, otp: &str) -> anyhow::Result<OtpEnvelope> {
        if !Self::valid_number(number) || otp.trim().is_empty() {
            anyhow::bail!("playbox: bad mobile number or OTP");
        }
        let url = format!("{}communication/otp/verify", self.login_v2);
        tokio::time::timeout(ACCOUNT_HTTP_TIMEOUT, async {
            let resp = self
                .http
                .post(&url)
                .header("X-API-Key", KEY_COMMUNICATION)
                .form(&[
                    ("appCode", APP_CODE),
                    ("mobileNo", number),
                    ("otp", otp.trim()),
                ])
                .send()
                .await?;
            if !resp.status().is_success() {
                anyhow::bail!("playbox: verify OTP failed: HTTP {}", resp.status());
            }
            Ok(resp.json::<OtpEnvelope>().await?)
        })
        .await
        .map_err(|_| anyhow::anyhow!("playbox: verify OTP timed out"))?
    }

    /// Stores `data.token` from a successful verification, matching
    /// `OTPVerifyViewModel` (`statusCode == 200`, then `saveUserToken`).
    pub fn install_session(
        &self,
        number: &str,
        envelope: &OtpEnvelope,
        store: &crate::store::Store,
    ) -> anyhow::Result<bool> {
        if envelope.status_code != 200 {
            return Ok(false);
        }
        let Some(session) = envelope.session() else {
            return Ok(false);
        };
        let creds = Credentials {
            phone: if session.phone.is_empty() {
                number.to_string()
            } else {
                session.phone.clone()
            },
            token: session.token.clone(),
            customer_id: if session.customer_id.is_empty() {
                session.id.to_string()
            } else {
                session.customer_id.clone()
            },
        };
        creds.save(store)?;
        *self.creds.write().unwrap() = creds;
        Ok(true)
    }

    /// `Api.listLiveTV`: `GET v3/page/livetv2` on the CloudFront catalogue.
    pub async fn channels(&self) -> anyhow::Result<Vec<Item>> {
        tokio::time::timeout(CATALOGUE_TIMEOUT, self.channels_inner())
            .await
            .map_err(|_| anyhow::anyhow!("playbox: catalogue timed out"))?
    }

    async fn channels_inner(&self) -> anyhow::Result<Vec<Item>> {
        let url = format!(
            "{}v3/page/livetv2?langs={DEFAULT_LANGS}&identifier={IDENTIFIER}&countryCode={COUNTRY_CODE}",
            self.catalogue
        );
        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", KEY_CATALOGUE)
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("playbox: catalogue failed: HTTP {}", resp.status());
        }
        Ok(resp.json::<CatalogueEnvelope>().await?.channels())
    }

    /// `Api.getPlayNew`: `POST consume/v1/playback/resolve` with
    /// `Authorization: Bearer <token>` and a `PlayRequestNew` JSON body.
    pub async fn resolve(
        &self,
        content_id: &str,
        type_id: i32,
    ) -> anyhow::Result<ResolvedPlayback> {
        tokio::time::timeout(
            PLAYBACK_RESOLVE_TIMEOUT,
            self.resolve_with_fallback(content_id, type_id),
        )
        .await
        .map_err(|_| anyhow::anyhow!("playbox: playback resolution timed out"))?
    }

    async fn resolve_with_fallback(
        &self,
        content_id: &str,
        type_id: i32,
    ) -> anyhow::Result<ResolvedPlayback> {
        let Some(bearer) = self.bearer() else {
            anyhow::bail!("playbox: not logged in");
        };
        let url = format!("{}consume/v1/playback/resolve", self.consume);
        let body = serde_json::json!({
            "contentId": content_id,
            "typeId": type_id,
            "deviceId": self.device.android_id,
            "platform": "androidtv",
            "pageId": "0",
            "appVersion": VERSION_NAME,
            "isEncrypted": false,
        });
        let resp = self
            .http
            .post(&url)
            .header("Authorization", bearer)
            .json(&body)
            .send()
            .await;

        if let Ok(resp) = resp {
            if resp.status().is_success() {
                if let Ok(envelope) = resp.json::<ResolveEnvelope>().await {
                    if envelope.code == 200 {
                        if let Some(contents) = envelope.data {
                            return ResolvedPlayback::from_contents(&contents);
                        }
                    } else if !envelope.msg.is_empty() {
                        anyhow::bail!("playbox: playback refused: {}", envelope.msg);
                    }
                }
            }
        }

        // The app also ships the legacy `PUT playableURL` path, selected by a
        // remote flag; use it as a fallback when resolve is unavailable.
        self.legacy_play(content_id, type_id).await
    }

    async fn legacy_play(
        &self,
        content_id: &str,
        type_id: i32,
    ) -> anyhow::Result<ResolvedPlayback> {
        let Some(phone) = self.phone() else {
            anyhow::bail!("playbox: not logged in");
        };
        let url = format!("{}playableURL", self.player_v2);
        let type_id = type_id.to_string();
        let resp = self
            .http
            .put(&url)
            .header("X-API-Key", KEY_PLAYER)
            .form(&[
                ("phone", phone.as_str()),
                ("contentId", content_id),
                ("type", LIVE_TYPE),
                ("typeid", type_id.as_str()),
                ("deviceId", self.device.android_id.as_str()),
                ("identifier", IDENTIFIER),
                ("hasSubscription", "false"),
            ])
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!(
                "playbox: playback resolve failed (HTTP {} on both resolve and playableURL)",
                resp.status()
            );
        }
        let contents: PlayContents = resp.json().await?;
        ResolvedPlayback::from_contents(&contents)
    }

    /// `Api.getActivePack`: `GET customer/{phone}/hasActivePacks`.
    pub async fn subscription(&self) -> anyhow::Result<ActivePacks> {
        let Some(phone) = self.phone() else {
            anyhow::bail!("playbox: not logged in");
        };
        let url = format!("{}customer/{phone}/hasActivePacks", self.user_v1);
        tokio::time::timeout(ACCOUNT_HTTP_TIMEOUT, async {
            let resp = self
                .http
                .get(&url)
                .header("X-API-Key", KEY_USER)
                .send()
                .await?;
            if !resp.status().is_success() {
                anyhow::bail!("playbox: subscription check failed: HTTP {}", resp.status());
            }
            Ok(resp.json::<ActivePacks>().await?)
        })
        .await
        .map_err(|_| anyhow::anyhow!("playbox: subscription check timed out"))?
    }
}

pub fn to_television_channel(item: &Item) -> crate::television::Channel {
    crate::television::Channel {
        id: channel_id(item.content_id()),
        name: item.display_name(),
        logo_url: item.logo(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn ids_are_collision_safe() {
        assert!(is_playbox_channel("pb_123"));
        assert!(!is_playbox_channel("123"));
        assert!(!is_playbox_channel("ex_123"));
        assert_eq!(content_id("pb_abc"), Some("abc"));
        assert_eq!(channel_id("abc"), "pb_abc");
        assert!(valid_content_id("abc-123_X.mp4"));
        assert!(!valid_content_id("abc/../def"));
        assert!(!valid_content_id(""));
    }

    #[test]
    fn credentials_round_trip_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let c = Credentials {
            phone: "9876543210".to_string(),
            token: "tok".to_string(),
            customer_id: "42".to_string(),
        };
        c.save(&store).unwrap();
        assert_eq!(Credentials::load(&store).unwrap().token, "tok");
        delete_credentials(&store).unwrap();
        assert!(Credentials::load(&store).is_none());
    }

    #[test]
    fn credential_delete_surfaces_store_write_failures() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let path = dir.path().join("store_v4.toml");
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();

        assert!(delete_credentials(&store).is_err());
    }

    #[test]
    fn catalogue_walks_page_rows_then_legacy_lists_without_duplicates() {
        let json = serde_json::json!({
            "statusCode": 200,
            "page": {"body": {"rows": [
                {"items": [
                    {"id": "ch1", "name": "One", "typeId": 106,
                     "text": {"title": "Channel One"}, "media": {"thumbnail": "https://x/1.png"}},
                    {"id": "bad/id", "name": "Bad"}
                ]}
            ]}},
            "livetv": [
                {"id": "ch1", "name": "One duplicate"},
                {"id": "ch2", "name": "Two", "typeid": 107}
            ]
        });
        let env: CatalogueEnvelope = serde_json::from_value(json).unwrap();
        let items = env.channels();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].content_id(), "ch1");
        assert_eq!(items[0].display_name(), "Channel One");
        assert_eq!(items[0].logo(), "https://x/1.png");
        assert_eq!(items[0].playback_type_id(), 106);
        assert_eq!(items[1].content_id(), "ch2");
        assert_eq!(items[1].playback_type_id(), 107);
    }

    #[test]
    fn media_headers_match_the_player_contract() {
        let mut url = PlayUrl {
            playable: "https://cdn/x.mpd".into(),
            headers: Some(PlayHeaders {
                user_agent: "PlayboxUA".into(),
                user_id: "u1".into(),
                subscriber_id: "s1".into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let h = media_headers(&url);
        assert!(h.contains(&("user_id".into(), "u1".into())));
        assert!(h.contains(&("User-Agent".into(), "PlayboxUA".into())));
        assert!(h.contains(&("subscriber_id".into(), "s1".into())));

        url.token = "CloudFront-Policy=abc".into();
        let h = media_headers(&url);
        assert!(h.contains(&("Cookie".into(), "CloudFront-Policy=abc".into())));

        url.token = "plain-token".into();
        let h = media_headers(&url);
        assert!(h.contains(&("Authorization".into(), "Bearer plain-token".into())));
    }

    #[test]
    fn license_headers_present_the_token_in_every_observed_header() {
        let h = license_headers("tok");
        for name in [
            "token",
            "X-AxDRM-Message",
            "pallycon-customdata-v2",
            "nv-authorizations",
        ] {
            assert!(h.contains(&(name.to_string(), "tok".to_string())), "{name}");
        }
        assert!(h.contains(&("Authorization".to_string(), "Bearer tok".to_string())));
        assert!(h.contains(&("Cookie".to_string(), "tok".to_string())));
        assert!(license_headers("").is_empty());
    }

    fn successful_session() -> serde_json::Value {
        serde_json::json!({
            "statusCode": 200,
            "message": "success",
            "data": {
                "token": "opaque-session-token",
                "phone": "9876543210",
                "customer_id": "cust-1",
                "id": 7
            }
        })
    }

    #[tokio::test]
    async fn otp_login_uses_the_decompiled_communication_contract() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login/v2/communication"))
            .and(header("X-API-Key", KEY_COMMUNICATION))
            .and(body_string_contains("appCode=play"))
            .and(body_string_contains("mobileNo=9876543210"))
            .and(body_string_contains("messageId=10"))
            .and(body_string_contains("countryCode=in"))
            .respond_with(ResponseTemplate::new(200).set_body_json(successful_session()))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/login/v2/communication/otp/verify"))
            .and(header("X-API-Key", KEY_COMMUNICATION))
            .and(body_string_contains("otp=123456"))
            .respond_with(ResponseTemplate::new(200).set_body_json(successful_session()))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let mut client = Client::new(reqwest::Client::new(), Device::new());
        client.set_bases_for_test(&server.uri());

        let sent = client.send_otp("9876543210").await.unwrap();
        assert_eq!(sent.status_code, 200);
        let verified = client.verify_otp("9876543210", "123456").await.unwrap();
        assert!(client
            .install_session("9876543210", &verified, &store)
            .unwrap());
        assert_eq!(
            Credentials::load(&store).unwrap().token,
            "opaque-session-token"
        );

        assert!(client.send_otp("123").await.is_err());
        assert!(client.verify_otp("9876543210", "").await.is_err());
    }

    #[tokio::test]
    async fn rejected_otp_is_not_installed() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let client = Client::new(reqwest::Client::new(), Device::new());
        let envelope = OtpEnvelope {
            status_code: 401,
            message: "invalid".into(),
            ..Default::default()
        };
        assert!(!client
            .install_session("9876543210", &envelope, &store)
            .unwrap());
        assert!(Credentials::load(&store).is_none());
    }

    #[test]
    fn negative_play_status_is_rejected_without_validation_text() {
        let contents: PlayContents = serde_json::from_value(serde_json::json!({
            "message": {"playStatus": false, "validationMessage": ""},
            "url": {"playable": "https://cdn.example/live/ch1.m3u8", "token": ""}
        }))
        .unwrap();

        assert_eq!(
            ResolvedPlayback::from_contents(&contents)
                .unwrap_err()
                .to_string(),
            "playbox: playback refused: not playable"
        );
    }

    #[tokio::test]
    async fn resolve_uses_the_decompiled_resolve_contract() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/consume/v1/playback/resolve"))
            .and(header("Authorization", "Bearer opaque-session-token"))
            .and(body_string_contains("\"contentId\":\"ch1\""))
            .and(body_string_contains("\"platform\":\"androidtv\""))
            .and(body_string_contains("\"isEncrypted\":false"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": 200,
                "msg": "success",
                "data": {
                    "message": {"playStatus": true, "validationMessage": ""},
                    "url": {
                        "playable": "https://cdn.example/live/ch1.mpd",
                        "licenceUrl": "https://lic.example/widevine",
                        "token": "drm-token",
                        "isLive": true,
                        "headers": {
                            "userAgent": "PlayboxUA",
                            "user_id": "u1",
                            "subscriber_id": "s1"
                        }
                    }
                }
            })))
            .mount(&server)
            .await;

        let mut client = Client::new(reqwest::Client::new(), Device::new());
        client.set_bases_for_test(&server.uri());
        *client.creds.write().unwrap() = Credentials {
            phone: "9876543210".into(),
            token: "opaque-session-token".into(),
            customer_id: "cust-1".into(),
        };

        let p = client.resolve("ch1", 106).await.unwrap();
        assert_eq!(p.playable, "https://cdn.example/live/ch1.mpd");
        assert_eq!(p.license_url, "https://lic.example/widevine");
        assert!(p.is_dash);
        assert!(p
            .media_headers
            .contains(&("Authorization".into(), "Bearer drm-token".into())));
        assert!(p
            .license_headers
            .contains(&("X-AxDRM-Message".into(), "drm-token".into())));
    }

    #[tokio::test]
    async fn resolve_falls_back_to_legacy_playable_url() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/consume/v1/playback/resolve"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/player/v2/playableURL"))
            .and(header("X-API-Key", KEY_PLAYER))
            .and(body_string_contains("contentId=ch1"))
            .and(body_string_contains("type=liveTV"))
            .and(body_string_contains("typeid=106"))
            .and(body_string_contains("identifier=1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": {"playStatus": true, "validationMessage": ""},
                "url": {"playable": "https://cdn.example/live/ch1.m3u8", "token": ""}
            })))
            .mount(&server)
            .await;

        let mut client = Client::new(reqwest::Client::new(), Device::new());
        client.set_bases_for_test(&server.uri());
        *client.creds.write().unwrap() = Credentials {
            phone: "9876543210".into(),
            token: "opaque-session-token".into(),
            customer_id: "cust-1".into(),
        };
        let p = client.resolve("ch1", 106).await.unwrap();
        assert_eq!(p.playable, "https://cdn.example/live/ch1.m3u8");
        assert!(!p.is_dash);
    }

    #[tokio::test]
    async fn resolve_timeout_bounds_the_legacy_fallback_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/consume/v1/playback/resolve"))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/player/v2/playableURL"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_secs(2))
                    .set_body_json(serde_json::json!({
                        "message": {"playStatus": true, "validationMessage": ""},
                        "url": {"playable": "https://cdn.example/live/ch1.m3u8", "token": ""}
                    })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let mut client = Client::new(reqwest::Client::new(), Device::new());
        client.set_bases_for_test(&server.uri());
        *client.creds.write().unwrap() = Credentials {
            phone: "9876543210".into(),
            token: "opaque-session-token".into(),
            customer_id: "cust-1".into(),
        };

        assert_eq!(
            client.resolve("ch1", 106).await.unwrap_err().to_string(),
            "playbox: playback resolution timed out"
        );
    }

    #[tokio::test]
    async fn catalogue_and_subscription_use_the_decompiled_routes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/catalogue/v3/page/livetv2"))
            .and(query_param("langs", "hi"))
            .and(query_param("identifier", "1"))
            .and(query_param("countryCode", "in"))
            .and(header("X-API-Key", KEY_CATALOGUE))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "statusCode": 200,
                "livetv": [{"id": "ch1", "name": "One", "typeId": 106}]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/user/v1/customer/9876543210/hasActivePacks"))
            .and(header("X-API-Key", KEY_USER))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "active": true, "statusCode": 200
            })))
            .mount(&server)
            .await;

        let mut client = Client::new(reqwest::Client::new(), Device::new());
        client.set_bases_for_test(&server.uri());
        *client.creds.write().unwrap() = Credentials {
            phone: "9876543210".into(),
            token: "opaque-session-token".into(),
            customer_id: "cust-1".into(),
        };
        let channels = client.channels().await.unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(to_television_channel(&channels[0]).id, "pb_ch1");
        assert!(client.subscription().await.unwrap().entitled());
    }

    #[tokio::test]
    async fn catalogue_timeout_bounds_the_complete_fetch() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/catalogue/v3/page/livetv2"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_secs(2))
                    .set_body_json(serde_json::json!({
                        "statusCode": 200,
                        "livetv": [{"id": "ch1", "name": "One", "typeId": 106}]
                    })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let mut client = Client::new(reqwest::Client::new(), Device::new());
        client.set_bases_for_test(&server.uri());

        assert_eq!(
            client.channels().await.unwrap_err().to_string(),
            "playbox: catalogue timed out"
        );
    }

    #[tokio::test]
    async fn account_http_timeouts_bound_send_verify_and_subscription() {
        let server = MockServer::start().await;
        for path_name in [
            "/login/v2/communication",
            "/login/v2/communication/otp/verify",
            "/user/v1/customer/9876543210/hasActivePacks",
        ] {
            Mock::given(path(path_name))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_delay(Duration::from_secs(2))
                        .set_body_json(successful_session()),
                )
                .expect(1)
                .mount(&server)
                .await;
        }

        let mut client = Client::new(reqwest::Client::new(), Device::new());
        client.set_bases_for_test(&server.uri());
        *client.creds.write().unwrap() = Credentials {
            phone: "9876543210".into(),
            token: "opaque-session-token".into(),
            customer_id: "cust-1".into(),
        };

        assert_eq!(
            client.send_otp("9876543210").await.unwrap_err().to_string(),
            "playbox: send OTP timed out"
        );
        assert_eq!(
            client
                .verify_otp("9876543210", "123456")
                .await
                .unwrap_err()
                .to_string(),
            "playbox: verify OTP timed out"
        );
        assert_eq!(
            client.subscription().await.unwrap_err().to_string(),
            "playbox: subscription check timed out"
        );
    }
}
