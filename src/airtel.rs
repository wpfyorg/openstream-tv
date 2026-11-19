//! Airtel Xstream Play (`tv.accedo.airtel.wynk`, 1.124.6) — the Accedo-built
//! OTT client Airtel ships for Xstream Play.
//!
//! Evidence: `docs/ott-provider-airtel/index.md`. The contracts below were
//! recovered from the shipped APK (`MiddlewareRequestInterceptor`, `Utility`,
//! `Signature`, `ApiPathConstants`, `MiddlewareRetroFitDataSource`,
//! `PlayerDRMCallback`) and confirmed by replaying the anonymous credential
//! fallback against `https://content.airtel.tv`, which returns a 244 KB
//! `appConfig` body for a correctly signed request and a 401 otherwise.
//!
//! Every middleware call carries `appId=MOBILITY` and an `x-atv-utkn` header:
//! `uid + ":" + base64(HMAC-SHA1(uid's token, METHOD + path + "?" + query + body))`.
//! `ln`, ...) as query parameters on the catalogue, EPG, content and package
//! hosts. Anonymous calls are signed with the app's own fallback identity and
//! can read `appConfig` but get an empty channel list: the catalogue, EPG and
//! playback endpoints all require the account that was logged in with an OTP.
//!
//! Only live TV is implemented (no catch-up, no VOD). Playback is whatever
//! the account is entitled to: clear HLS, clear DASH, or Widevine DASH whose
//! license is proxied through `/drm`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

pub const ID_PREFIX: &str = "airtel_";

/// Linear-catalogue partners whose channels `v3/user/content/playback` serves
/// to an OTT session. `LIVETV` (Airtel's own live pack) is not one of them: it
/// is `iptv: true` and only plays through `iptv-playback.airtel.tv`, which
/// requires the STB's chip id, box serial and box-auth headers.
pub const PLAYABLE_LINEAR_CPS: &[&str] = &["MWTV"];
pub const PACKAGE: &str = "tv.accedo.airtel.wynk";
pub const VERSION_NAME: &str = "1.124.6";
pub const VERSION_CODE: u32 = 13_164;
pub const ARTIFACT_SHA256: &str =
    "f5b414f6d7148acac66523c77d515922431e8b105b60fca012f17e287e242b2b";

/// `Constants.APP_ID`; added as `appId` to every request.
pub const APP_ID: &str = "MOBILITY";
/// `Constants.VALUE_DEVICE_KEY`; the fixed `deviceKey` in the login body.
pub const DEVICE_KEY: &str = "APADdd8231gg";
pub const USER_AGENT: &str = "okhttp/4.12.0";
/// ExoPlayer's agent inside the app (`Util.getUserAgent(context, package)`).
pub const PLAYER_USER_AGENT: &str =
    "tv.accedo.airtel.wynk/1.124.6 (Linux;Android 13) ExoPlayerLib/1.4.1";

pub(crate) const BASE_API: &str = "https://api.airtel.tv";
pub(crate) const BASE_EPG: &str = "https://epg.airtel.tv";
pub(crate) const BASE_PLAY: &str = "https://play.airtel.tv";
/// `middleWareLiveTVEndPoint`: linear/DTH catalogue, guide and the IPTV play host.
pub(crate) const BASE_LIVE: &str = "https://livetv.airtel.tv";

/// `NetworkConstants.UserConstants.FALLBACK_UID`: the anonymous identity the
/// interceptor signs with before login.
const FALLBACK_UID: &str = "v72UE-WQrmdaed_gi53xZLV5pa02";
/// `NetworkConstants.UserConstants.FALLBACK_TOKEN`.
const FALLBACK_TOKEN: &str = "FKmr3XQti5C8WyRi4tRNgzgVeMk=";

/// `Keys.OTP_MESSAGE_TO_USER`'s default. The app prefixes it with `<#>` and
/// appends its SMS-retriever signature; the message text is cosmetic, the
/// signature only lets Android autofill the code.
const OTP_MESSAGE: &str = "<#> Use {OTP} as your login OTP. OTP is confidential";

/// The model name reported as `deviceType`; the app sends its own
/// `Build.MANUFACTURER` + `Build.MODEL` here, and Airtel ties the device slot
/// to it, so it must stay stable across restarts.
const DEVICE_MODEL: &str = "Airtel Xstream";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub const STORE_KEY_DEVICE: &str = "airtel_device";
pub const STORE_KEY_CREDENTIALS: &str = "airtel_credentials";

/// The app's persisted device id (`Utils.getNewUDID` -> `Settings.Secure
/// "android_id"`), plus the name/model reported in `x-atv-did`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    #[serde(default)]
    pub udid: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub model: String,
}

impl Device {
    pub fn new() -> Device {
        let mut bytes = [0u8; 8];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
        Device {
            udid: hex::encode(bytes),
            name: "openstream".to_string(),
            model: DEVICE_MODEL.to_string(),
        }
    }

    pub fn load_or_create(store: &crate::store::Store) -> anyhow::Result<Device> {
        if let Some(json) = store.get_opt(STORE_KEY_DEVICE) {
            if let Ok(d) = serde_json::from_str::<Device>(&json) {
                if !d.udid.is_empty() {
                    return Ok(d);
                }
            }
        }
        let d = Device::new();
        store.set(STORE_KEY_DEVICE, &serde_json::to_string(&d)?)?;
        Ok(d)
    }

    /// `RetrofitClient.getDeviceIdentifierHeader`, joined with
    /// `ConstantUtil.EXPERIMENTS_DELIMITTER` (`"|"`, not `":"`). The delimiter
    /// is load-bearing: the middleware answers `412 ATV001` to
    /// `v2/user/profile/generateOtp` for a colon-joined header and `200` for the
    /// pipe-joined one, and the play API rejects a colon-joined `x-atv-did`.
    pub fn identifier(&self) -> String {
        format!(
            "{}|{}|Android|34|{}|{}|{}|{}",
            self.udid,
            device_type_for(&self.model),
            VERSION_CODE,
            VERSION_NAME,
            self.name,
            self.model,
        )
    }

    /// `DeviceIdentifier.getDeviceModelName`: capitalized manufacturer plus
    /// model, which is what the login body sends as `deviceType`.
    pub fn login_device_type(&self) -> String {
        self.model.clone()
    }
}

impl Default for Device {
    fn default() -> Self {
        Device::new()
    }
}

fn device_type_for(model: &str) -> &'static str {
    let lower = model.to_ascii_lowercase();
    if lower.contains("tv") {
        "TV"
    } else if lower.contains("pad") || lower.contains("tab") {
        "Tablet"
    } else {
        "Phone"
    }
}

/// One authenticated account, as returned by `v8/user/login`. `token` keys the
/// request signature; `auth_token` is the `x-atv-stkn` playback token.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Credentials {
    #[serde(default)]
    pub uid: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub auth_token: String,
    #[serde(default)]
    pub msisdn: String,
    /// `userConfig.userContentProperties`: `cp`, `cl`, `ln`, `ut`, ... appended
    /// as query parameters on account-scoped hosts.
    #[serde(default)]
    pub content_properties: HashMap<String, String>,
    #[serde(default)]
    pub circle: String,
    #[serde(default)]
    pub language: String,
}

impl Credentials {
    pub fn load(store: &crate::store::Store) -> Option<Credentials> {
        let json = store.get_opt(STORE_KEY_CREDENTIALS)?;
        let c: Credentials = serde_json::from_str(&json).ok()?;
        if c.uid.is_empty() || c.token.is_empty() {
            return None;
        }
        Some(c)
    }

    pub fn save(&self, store: &crate::store::Store) -> anyhow::Result<()> {
        store.set(STORE_KEY_CREDENTIALS, &serde_json::to_string(self)?)?;
        Ok(())
    }

    pub fn language_or_default(&self) -> String {
        if self.language.is_empty() {
            "hi".to_string()
        } else {
            self.language.clone()
        }
    }

    pub fn cp(&self) -> String {
        self.content_properties
            .get("cp")
            .cloned()
            .filter(|v| !v.is_empty())
            .unwrap_or_default()
    }
}

pub fn delete_credentials(store: &crate::store::Store) -> anyhow::Result<()> {
    store.delete(STORE_KEY_CREDENTIALS)?;
    Ok(())
}

pub fn parse_channel_id(channel_id: &str) -> Option<&str> {
    channel_id
        .strip_prefix(ID_PREFIX)
        .filter(|id| !id.is_empty())
}

pub fn channel_id(content_id: &str) -> String {
    format!("{ID_PREFIX}{content_id}")
}

pub fn is_airtel_channel(id: &str) -> bool {
    id.starts_with(ID_PREFIX)
}

/// `AirtelChannel` as the channel-list endpoint returns it.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AirtelChannel {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default, rename = "channelNo", deserialize_with = "de_channel_no")]
    pub channel_no: Option<i64>,
    #[serde(default)]
    pub hd: bool,
    /// Content partner the play API expects in `x-atv-cp` (`LIVETV`, `MWTV`).
    #[serde(default)]
    pub cp: String,
    /// `ImagesApiModel`: uppercase keys such as `LOGO`, `LOGO_HD`, `SQUARE`.
    /// The linear catalogue spells the field `images`.
    #[serde(default, alias = "images")]
    pub imgs: HashMap<String, String>,
}

fn de_channel_no<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Number(i64),
        Text(String),
    }

    Ok(match Option::<Raw>::deserialize(deserializer)? {
        None => None,
        Some(Raw::Number(number)) => Some(number),
        Some(Raw::Text(text)) => text.trim().parse::<i64>().ok(),
    })
}

impl AirtelChannel {
    pub fn logo(&self) -> String {
        ["LOGO_HD", "LOGO", "SQUARE", "LANDSCAPE"]
            .into_iter()
            .filter_map(|key| self.imgs.get(key))
            .find(|url| !url.is_empty())
            .cloned()
            .unwrap_or_default()
    }
}

pub fn to_television_channel(channel: &AirtelChannel) -> crate::television::Channel {
    crate::television::Channel {
        id: channel_id(&channel.id),
        name: channel.title.clone(),
        logo_url: channel.logo(),
        is_hd: channel.hd,
        ..Default::default()
    }
}

/// One `programGuide` entry. Airtel sends epoch millis; values that look like
/// seconds are normalised anyway because the field is a plain `long` in the
/// app and older middleware revisions used seconds.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AirtelProgram {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default, rename = "startTime")]
    pub start_time: i64,
    #[serde(default, rename = "endTime")]
    pub end_time: i64,
}

impl AirtelProgram {
    pub fn start_millis(&self) -> i64 {
        epoch_millis(self.start_time)
    }

    pub fn end_millis(&self) -> i64 {
        epoch_millis(self.end_time)
    }
}

fn epoch_millis(value: i64) -> i64 {
    if value.abs() < 10_000_000_000 {
        value.saturating_mul(1000)
    } else {
        value
    }
}

/// A resolved playback: the media URL, the headers its CDN wants, and the
/// Widevine license when the stream is protected.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Playback {
    pub url: String,
    pub is_dash: bool,
    pub headers: Vec<(String, String)>,
    pub license: Option<License>,
}

impl Playback {
    pub fn has_drm(&self) -> bool {
        self.license.is_some()
    }
}

/// The `drm` block of the playback response, as `PlayerDRMCallback` uses it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct License {
    pub url: String,
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub payload_key: String,
}

/// Build the license request body: the raw Widevine challenge when the
/// provider expects it, otherwise the app's JSON envelope with the challenge
/// substituted for `EXO_PAYLOAD` (`PlayerDRMCallback.c`).
pub fn license_request_body(license: &License, challenge: &[u8]) -> Vec<u8> {
    if license.payload_key.is_empty() || license.payload_key == "default" {
        return challenge.to_vec();
    }
    let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, challenge);
    let mut object = license.body.clone();
    for value in object.values_mut() {
        match value.as_str() {
            Some("EXO_PAYLOAD") | Some("lionsgate_payload") => {
                *value = serde_json::Value::String(encoded.clone());
            }
            _ => {}
        }
    }
    serde_json::to_vec(&object).unwrap_or_else(|_| challenge.to_vec())
}

/// Unwrap the provider's JSON license response (`{"status":"ok","<payloadKey>":
/// "base64..."}`) back to the raw license the player expects.
pub fn license_response_body(license: &License, bytes: Vec<u8>) -> Vec<u8> {
    if license.payload_key.is_empty() || license.payload_key == "default" {
        return bytes;
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return bytes;
    };
    if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return bytes;
    }
    let encoded = match value.get(&license.payload_key) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Array(items)) => items
            .first()
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        _ => return bytes,
    };
    match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded) {
        Ok(decoded) => decoded,
        Err(_) => bytes,
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
struct StreamingResponse {
    #[serde(default, rename = "eligibleForPlayback")]
    eligible_for_playback: Option<bool>,
    #[serde(default, rename = "playId")]
    play_id: String,
    #[serde(default, rename = "isDrm")]
    is_drm: bool,
    #[serde(default)]
    message: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct LoginResponse {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    uid: String,
    #[serde(default)]
    token: String,
    #[serde(default, rename = "authToken")]
    auth_token: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    errortitle: String,
    #[serde(default)]
    errorcode: String,
    #[serde(default)]
    userconfig: Option<UserConfigBody>,
    #[serde(default, rename = "userConfig")]
    user_config: Option<UserConfigBody>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct UserConfigBody {
    #[serde(default, rename = "userContentProperties")]
    user_content_properties: Option<serde_json::Value>,
    #[serde(default, rename = "contentProperties")]
    content_properties: Option<serde_json::Value>,
}

impl LoginResponse {
    fn config(&self) -> Option<&UserConfigBody> {
        self.user_config.as_ref().or(self.userconfig.as_ref())
    }

    fn properties(&self) -> HashMap<String, String> {
        let Some(config) = self.config() else {
            return HashMap::new();
        };
        let raw = config
            .user_content_properties
            .as_ref()
            .or(config.content_properties.as_ref());
        let Some(value) = raw else {
            return HashMap::new();
        };
        flatten_properties(value)
    }
}

fn flatten_properties(value: &serde_json::Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                match value {
                    serde_json::Value::String(s) => {
                        if !s.is_empty() {
                            out.insert(key.clone(), s.clone());
                        }
                    }
                    serde_json::Value::Number(n) => {
                        out.insert(key.clone(), n.to_string());
                    }
                    serde_json::Value::Bool(b) => {
                        out.insert(key.clone(), b.to_string());
                    }
                    _ => {}
                }
            }
        }
        // Some middleware revisions serialise the properties as a query string.
        serde_json::Value::String(query) => {
            for pair in query.split('&') {
                let mut parts = pair.splitn(2, '=');
                let key = parts.next().unwrap_or_default();
                let value = parts.next().unwrap_or_default();
                if !key.is_empty() && !value.is_empty() {
                    out.insert(key.to_string(), value.to_string());
                }
            }
        }
        _ => {}
    }
    out
}

/// The Airtel middleware client. One per process; `creds` is the logged-in
/// account (or `None` before login, which still signs with the app fallback).
/// One signed middleware request. A builder keeps the request shape in one
/// place because catalogue, EPG and playback each retry once after a token
/// refresh with the identical request.
struct RequestSpec<'a> {
    method: reqwest::Method,
    base: &'a str,
    path: &'a str,
    query: Vec<(String, String)>,
    body: Option<String>,
    headers: Vec<(String, String)>,
}

impl<'a> RequestSpec<'a> {
    fn get(base: &'a str, path: &'a str) -> RequestSpec<'a> {
        RequestSpec {
            method: reqwest::Method::GET,
            base,
            path,
            query: Vec::new(),
            body: None,
            headers: Vec::new(),
        }
    }

    fn post(base: &'a str, path: &'a str, body: String) -> RequestSpec<'a> {
        RequestSpec {
            method: reqwest::Method::POST,
            base,
            path,
            query: Vec::new(),
            body: Some(body),
            headers: Vec::new(),
        }
    }

    fn query(mut self, key: &str, value: impl Into<String>) -> RequestSpec<'a> {
        self.query.push((key.to_string(), value.into()));
        self
    }

    fn header(mut self, key: &str, value: impl Into<String>) -> RequestSpec<'a> {
        self.headers.push((key.to_string(), value.into()));
        self
    }
}

pub struct Client {
    pub http: reqwest::Client,
    pub device: Device,
    pub api_base: String,
    pub epg_base: String,
    pub play_base: String,
    /// `middleWareLiveTVEndPoint`: the linear/DTH catalogue and its guide.
    pub live_base: String,
    pub creds: RwLock<Option<Credentials>>,
}

impl Client {
    pub fn new(http: reqwest::Client, device: Device) -> Client {
        Client {
            http,
            device,
            api_base: BASE_API.to_string(),
            epg_base: BASE_EPG.to_string(),
            play_base: BASE_PLAY.to_string(),
            live_base: BASE_LIVE.to_string(),
            creds: RwLock::new(None),
        }
    }

    pub fn from_store(
        http: reqwest::Client,
        store: &crate::store::Store,
    ) -> anyhow::Result<Client> {
        let device = Device::load_or_create(store)?;
        let client = Client::new(http, device);
        *client.creds.write().unwrap() = Credentials::load(store);
        Ok(client)
    }

    #[cfg(test)]
    pub fn set_bases_for_test(&mut self, api: &str, epg: &str, play: &str) {
        self.api_base = api.to_string();
        self.epg_base = epg.to_string();
        self.play_base = play.to_string();
        self.live_base = api.to_string();
    }

    pub fn credentials(&self) -> Option<Credentials> {
        self.creds.read().unwrap().clone()
    }

    pub fn is_logged_in(&self) -> bool {
        self.credentials()
            .is_some_and(|c| !c.uid.is_empty() && !c.token.is_empty())
    }

    fn sign_with(
        creds: Option<&Credentials>,
        method: &str,
        path: &str,
        query: &str,
        body: &str,
    ) -> String {
        let (uid, token) = creds
            .filter(|c| !c.uid.is_empty() && !c.token.is_empty())
            .map(|c| (c.uid.clone(), c.token.clone()))
            .unwrap_or_else(|| (FALLBACK_UID.to_string(), FALLBACK_TOKEN.to_string()));
        signature_header(&uid, &token, method, path, query, body)
    }

    /// Build one signed middleware request from a spec: `properties` appends
    /// the account's content properties, which the interceptor does on the
    /// catalogue, EPG, content and package hosts only.
    fn request(&self, spec: RequestSpec<'_>) -> reqwest::RequestBuilder {
        let creds = self.credentials();
        self.request_as(spec, creds.as_ref())
    }

    /// Like [`Client::request`], but signs with the credentials the caller
    /// captured. Refresh uses this so the request and the later replacement
    /// check refer to the same login.
    fn request_as(
        &self,
        spec: RequestSpec<'_>,
        creds: Option<&Credentials>,
    ) -> reqwest::RequestBuilder {
        let mut pairs: Vec<(String, String)> = Vec::with_capacity(spec.query.len() + 8);
        pairs.push(("appId".to_string(), APP_ID.to_string()));
        pairs.extend(spec.query);
        let encoded: Vec<String> = pairs
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect();
        let query_string = encoded.join("&");
        let body = spec.body.unwrap_or_default();
        let url = if query_string.is_empty() {
            format!("{}{}", spec.base, spec.path)
        } else {
            format!("{}{}?{query_string}", spec.base, spec.path)
        };
        let utkn = Self::sign_with(creds, spec.method.as_str(), spec.path, &query_string, &body);
        let mut req = self
            .http
            .request(spec.method, &url)
            .version(reqwest::Version::HTTP_11)
            .header("Content-Type", "application/json")
            .header(reqwest::header::ACCEPT_ENCODING, "identity")
            .header("x-atv-utkn", utkn)
            .header("x-atv-did", self.device.identifier())
            .header("x-atv-traceid", trace_id())
            .header(reqwest::header::USER_AGENT, USER_AGENT);
        if !body.is_empty() {
            req = req.body(body);
        }
        if let Some(creds) = creds {
            if !creds.language_or_default().is_empty() {
                req = req.header("x-atv-lang", creds.language_or_default());
            }
        }
        for (name, value) in spec.headers {
            req = req.header(name, value);
        }
        req
    }

    async fn send(
        &self,
        req: reqwest::RequestBuilder,
        label: &str,
    ) -> anyhow::Result<(u16, Vec<u8>)> {
        let (status, bytes) = tokio::time::timeout(REQUEST_TIMEOUT, async {
            let response = req
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("airtel: {label} failed: {e}"))?;
            let status = response.status().as_u16();
            let bytes = response
                .bytes()
                .await
                .map_err(|e| anyhow::anyhow!("airtel: {label} body failed: {e}"))?
                .to_vec();
            Ok::<_, anyhow::Error>((status, bytes))
        })
        .await
        .map_err(|_| anyhow::anyhow!("airtel: {label} timed out"))??;
        Ok((status, bytes))
    }

    /// `GET livetv.airtel.tv/v1/livechannel`: the linear catalogue (783
    /// channels), which is where DTH channels live. The OTT catalogue on the
    /// EPG host carries the OTT live set only.
    /// The linear host answers the public catalogue without account signing;
    /// a signed request for the same URL comes back as `404 Channels not found`
    /// because that edge expects the box identity for signed traffic.
    fn linear_channels_request(&self) -> reqwest::RequestBuilder {
        self.http
            .get(format!("{}/v1/livechannel", self.live_base))
            .query(&[
                ("appId", APP_ID.to_string()),
                ("bn", VERSION_CODE.to_string()),
                ("os", "ANDROID".to_string()),
                ("dt", "phone".to_string()),
                ("page", "0".to_string()),
                ("size", "5000".to_string()),
            ])
            .version(reqwest::Version::HTTP_11)
            .header(reqwest::header::ACCEPT_ENCODING, "identity")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .header("x-atv-did", self.device.identifier())
    }

    /// `app/v2/content/channel/epg`: one channel's guide. The account-wide
    /// `app/v3/content/epg` covers the OTT catalogue only, so linear channels
    /// need this per-channel call.
    fn channel_epg_request(
        &self,
        content_id: &str,
        start_millis: i64,
        end_millis: i64,
    ) -> reqwest::RequestBuilder {
        self.request(
            RequestSpec::get(&self.epg_base, "/app/v2/content/channel/epg")
                .query("channelId", content_id)
                .query("startTime", start_millis.to_string())
                .query("endTime", end_millis.to_string()),
        )
    }

    /// `v3/user/content/playback` on the play host, with the account headers the
    /// app's interceptor adds (`x-atv-cp` from the channel's partner,
    /// `x-atv-customer`/`circle`/`segment` from the account's content
    /// properties). `v4/user/content/playback`, which this provider used
    /// before, is retired on the live service and answers a bare 404.
    fn playback_request(&self, content_id: &str, cp: &str) -> reqwest::RequestBuilder {
        let creds = self.credentials().unwrap_or_default();
        let mut spec = RequestSpec::get(&self.play_base, "/v3/user/content/playback")
            .query("dt", "phone")
            .query("os", "ANDROID")
            .query("bn", VERSION_CODE.to_string())
            .query("contentId", content_id);
        let cp = if cp.is_empty() {
            creds.cp()
        } else {
            cp.to_string()
        };
        if !cp.is_empty() {
            spec = spec.header("x-atv-cp", cp);
        }
        if !creds.auth_token.is_empty() {
            spec = spec.header("x-atv-stkn", creds.auth_token);
        }
        self.request(spec)
    }

    /// `v2/user/profile/generateOtp`: asks Airtel to SMS an OTP to `msisdn`
    /// (already in `+91XXXXXXXXXX` form).
    pub async fn send_otp(&self, msisdn: &str) -> anyhow::Result<String> {
        let body = serde_json::json!({
            "msisdn": msisdn,
            "msgTxt": OTP_MESSAGE,
        })
        .to_string();
        let req = self.request(RequestSpec::post(
            &self.api_base,
            "/v2/user/profile/generateOtp",
            body,
        ));
        let (status, bytes) = self.send(req, "generateOtp").await?;
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
        if !(200..300).contains(&status)
            || value.get("success").and_then(|v| v.as_bool()) == Some(false)
        {
            anyhow::bail!(
                "airtel: OTP request failed: {}",
                upstream_error(status, &value)
            );
        }
        Ok(value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("OTP sent")
            .to_string())
    }

    /// `v8/user/login` with `requireOtp:"false"` and the received OTP. Airtel
    /// toggles between `v8` and `v5` via remote config, so both are tried.
    pub async fn verify_otp(
        &self,
        store: &crate::store::Store,
        msisdn: &str,
        otp: &str,
        on_installed: impl FnOnce(),
    ) -> anyhow::Result<Credentials> {
        let body = serde_json::json!({
            "deviceType": self.device.login_device_type(),
            "deviceKey": DEVICE_KEY,
            "requireOtp": "false",
            "msisdn": msisdn,
            "otp": otp,
            "wifi": "1",
        })
        .to_string();
        let mut last = String::new();
        for path in ["/v8/user/login", "/v5/user/login"] {
            let req = self.request(
                RequestSpec::post(&self.api_base, path, body.clone())
                    .header("x-atv-imsi", "")
                    .header("x-atv-network-type", "1")
                    .header("x-atv-last-purge", "0"),
            );
            let (status, bytes) = self.send(req, "login").await?;
            let parsed: LoginResponse = serde_json::from_slice(&bytes).unwrap_or_default();
            if (200..300).contains(&status) && parsed.success && !parsed.token.is_empty() {
                let properties = parsed.properties();
                let creds = Credentials {
                    uid: parsed.uid,
                    token: parsed.token,
                    auth_token: parsed.auth_token,
                    msisdn: msisdn.to_string(),
                    circle: properties.get("cl").cloned().unwrap_or_default(),
                    language: properties
                        .get("ln")
                        .cloned()
                        .unwrap_or_else(|| "hi".to_string()),
                    content_properties: properties,
                };
                // Rotate the account context before the new credentials are
                // installed, so the previous account's URLs and caches cannot
                // be reused.
                on_installed();
                creds.save(store)?;
                *self.creds.write().unwrap() = Some(creds.clone());
                return Ok(creds);
            }
            let value: serde_json::Value =
                serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
            let error = upstream_error(status, &value);
            if parsed.uid.is_empty() && parsed.token.is_empty() && status == 404 {
                // Keep the previous endpoint's rejection when this fallback
                // path responds with an empty 404 but may still succeed.
                if last.is_empty() {
                    last = error;
                }
                continue;
            }
            last = error;
            if !parsed.message.is_empty() {
                last = parsed.message;
            }
            if !parsed.errortitle.is_empty() {
                last = if parsed.errorcode.is_empty() {
                    parsed.errortitle
                } else {
                    format!("{} {}", parsed.errorcode, parsed.errortitle)
                };
            }
        }
        anyhow::bail!("airtel: login failed: {last}")
    }

    /// `v2/user/session/refreshAuthToken`. Airtel rotates the signing token,
    /// so a 401 from any call means refresh (or log in again).
    pub async fn refresh(&self, store: &crate::store::Store) -> anyhow::Result<Credentials> {
        let body = serde_json::json!({
            "deviceType": self.device.login_device_type(),
            "deviceKey": DEVICE_KEY,
            "requireOtp": "false",
        })
        .to_string();
        let original = self.credentials();
        let req = self.request_as(
            RequestSpec::post(&self.api_base, "/v2/user/session/refreshAuthToken", body),
            original.as_ref(),
        );
        let (status, bytes) = self.send(req, "refresh").await?;
        let parsed: LoginResponse = serde_json::from_slice(&bytes).unwrap_or_default();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
        if !(200..300).contains(&status) || parsed.token.is_empty() {
            anyhow::bail!(
                "airtel: token refresh failed: {}",
                upstream_error(status, &value)
            );
        }
        let Some(original) = original else {
            anyhow::bail!("airtel: account logged out during token refresh");
        };
        let mut guard = self.creds.write().unwrap();
        let Some(creds) = guard.as_mut() else {
            anyhow::bail!("airtel: account logged out during token refresh");
        };
        if creds.uid != original.uid || creds.token != original.token {
            anyhow::bail!("airtel: credentials changed during token refresh");
        }
        if !parsed.uid.is_empty() {
            creds.uid = parsed.uid;
        }
        creds.token = parsed.token;
        if !parsed.auth_token.is_empty() {
            creds.auth_token = parsed.auth_token;
        }
        creds.save(store)?;
        Ok(creds.clone())
    }

    /// Airtel rotates the signing token without notice; a 401 means the
    /// stored one expired. Refresh once and retry, mirroring
    /// `AuthTokenRefreshHelper`.
    async fn refresh_after_unauthorized(&self, status: u16, store: &crate::store::Store) -> bool {
        if status != 401 || !self.is_logged_in() {
            return false;
        }
        self.refresh(store).await.is_ok()
    }

    /// The playable slice of the linear catalogue. A channel plays over
    /// `v3/user/content/playback` only when its partner is one the play API
    /// serves to an OTT session; Airtel's own live pack needs the STB host.
    pub async fn channels(
        &self,
        _store: &crate::store::Store,
    ) -> anyhow::Result<Vec<AirtelChannel>> {
        let (status, bytes) = self
            .send(self.linear_channels_request(), "livechannel")
            .await?;
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
        if !(200..300).contains(&status) {
            anyhow::bail!(
                "airtel: channel list failed: {}",
                upstream_error(status, &value)
            );
        }
        #[derive(Deserialize, Default)]
        struct Catalogue {
            #[serde(default)]
            data: Vec<AirtelChannel>,
        }
        let parsed: Catalogue = serde_json::from_value(value).unwrap_or_default();
        Ok(parsed
            .data
            .into_iter()
            .filter(|channel| {
                !channel.id.is_empty()
                    && !channel.title.is_empty()
                    && PLAYABLE_LINEAR_CPS.contains(&channel.cp.as_str())
            })
            .collect())
    }

    /// One linear channel's guide, for the catalogue built from
    /// `livetv.airtel.tv`.
    pub async fn channel_epg(
        &self,
        store: &crate::store::Store,
        content_id: &str,
        start_millis: i64,
        end_millis: i64,
    ) -> anyhow::Result<Vec<AirtelProgram>> {
        let (mut status, mut bytes) = self
            .send(
                self.channel_epg_request(content_id, start_millis, end_millis),
                "epg",
            )
            .await?;
        if self.refresh_after_unauthorized(status, store).await {
            (status, bytes) = self
                .send(
                    self.channel_epg_request(content_id, start_millis, end_millis),
                    "epg",
                )
                .await?;
        }
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
        if !(200..300).contains(&status) {
            anyhow::bail!(
                "airtel: channel guide failed: {}",
                upstream_error(status, &value)
            );
        }
        let mut guide: HashMap<String, Vec<AirtelProgram>> = serde_json::from_value(
            value
                .get("programGuide")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        )
        .unwrap_or_default();
        Ok(guide.remove(content_id).unwrap_or_default())
    }

    /// `v3/user/content/playback` is the route the current clients use; it
    /// answers a flat payload (`eligibleForPlayback`, `playId`, `isDrm`)
    /// instead of the old `playback`/`drm` blocks.
    pub async fn playback(
        &self,
        store: &crate::store::Store,
        content_id: &str,
        cp: &str,
    ) -> anyhow::Result<Playback> {
        let (mut status, mut bytes) = self
            .send(self.playback_request(content_id, cp), "playback")
            .await?;
        if self.refresh_after_unauthorized(status, store).await {
            (status, bytes) = self
                .send(self.playback_request(content_id, cp), "playback")
                .await?;
        }
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
        if !(200..300).contains(&status) {
            anyhow::bail!(
                "airtel: playback failed: {}",
                upstream_error(status, &value)
            );
        }
        let parsed: StreamingResponse = serde_json::from_value(value).unwrap_or_default();
        if parsed.eligible_for_playback == Some(false) {
            let why = if parsed.message.is_empty() {
                "this account is not eligible for the channel".to_string()
            } else {
                parsed.message.clone()
            };
            anyhow::bail!("airtel: {why}");
        }
        if !parsed.play_id.starts_with("http") {
            anyhow::bail!("airtel: playback response has no play URL");
        }
        if parsed.is_drm {
            anyhow::bail!(
                "airtel: this channel is DRM-protected and the license is only issued to the STB client"
            );
        }
        let is_dash = looks_like_dash(&parsed.play_id);
        Ok(Playback {
            url: parsed.play_id,
            is_dash,
            headers: Vec::new(),
            license: None,
        })
    }
}

/// Airtel's middleware stores the number as `+91XXXXXXXXXX`; accept a bare
/// 10-digit number as well and reject anything else before spending an SMS.
pub fn normalize_msisdn(raw: &str) -> Option<String> {
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    let local = match digits.len() {
        10 => digits,
        12 if digits.starts_with("91") => digits[2..].to_string(),
        _ => return None,
    };
    if !matches!(local.as_bytes().first(), Some(b'6'..=b'9')) {
        return None;
    }
    Some(format!("+91{local}"))
}

/// Airtel serves manifests from its own CDN hosts and from partner CDNs that
/// the app reaches through the same player agent.
pub fn looks_like_airtel_cdn(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host.ends_with("airtel.tv") || host.ends_with("airtel.in")
}

pub fn looks_like_dash(url: &str) -> bool {
    let path = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    path.ends_with(".mpd")
}

fn trace_id() -> String {
    let mut bytes = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    let mut hexed = hex::encode(bytes);
    if hexed.len() >= 36 {
        hexed.truncate(36);
    }
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    format!(
        "{}-{}-{}-{}-{}-{millis}",
        &hexed[0..8],
        &hexed[8..12],
        &hexed[12..16],
        &hexed[16..20],
        &hexed[20..32],
    )
}

fn upstream_error(status: u16, value: &serde_json::Value) -> String {
    let detail = [
        value.get("errortitle").and_then(|v| v.as_str()),
        value.get("message").and_then(|v| v.as_str()),
        value.get("error").and_then(|v| v.as_str()),
    ]
    .into_iter()
    .flatten()
    .find(|s| !s.is_empty())
    .unwrap_or("upstream rejected the request");
    match value.get("errorcode").and_then(|v| v.as_str()) {
        Some(code) if !code.is_empty() => format!("HTTP {status} {code}: {detail}"),
        _ => format!("HTTP {status}: {detail}"),
    }
}

/// `x-atv-utkn`: `uid + ":" + base64(HMAC-SHA1(token, METHOD + path + "?" +
/// query + body))`, exactly as `Utility.getSignatureString` +
/// `Signature.calculateRFC2104HMAC` build it.
pub fn signature_header(
    uid: &str,
    token: &str,
    method: &str,
    path: &str,
    query: &str,
    body: &str,
) -> String {
    let mut signed =
        String::with_capacity(method.len() + path.len() + query.len() + body.len() + 1);
    signed.push_str(&method.to_ascii_uppercase());
    signed.push_str(path);
    if !query.is_empty() {
        signed.push('?');
        signed.push_str(query);
    }
    signed.push_str(body);
    let mac = hmac_sha1(token.as_bytes(), signed.as_bytes());
    let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, mac);
    format!("{uid}:{encoded}")
}

fn hmac_sha1(key: &[u8], message: &[u8]) -> [u8; 20] {
    const BLOCK: usize = 64;
    let mut block_key = [0u8; BLOCK];
    if key.len() > BLOCK {
        block_key[..20].copy_from_slice(&sha1::digest(key));
    } else {
        block_key[..key.len()].copy_from_slice(key);
    }
    let mut inner = [0u8; BLOCK];
    let mut outer = [0u8; BLOCK];
    for i in 0..BLOCK {
        inner[i] = block_key[i] ^ 0x36;
        outer[i] = block_key[i] ^ 0x5c;
    }
    let mut inner_input = Vec::with_capacity(BLOCK + message.len());
    inner_input.extend_from_slice(&inner);
    inner_input.extend_from_slice(message);
    let inner_hash = sha1::digest(&inner_input);
    let mut outer_input = Vec::with_capacity(BLOCK + 20);
    outer_input.extend_from_slice(&outer);
    outer_input.extend_from_slice(&inner_hash);
    sha1::digest(&outer_input)
}

/// SHA-1 (RFC 3174), used only to key the Airtel request signature. Kept in
/// this module so the provider needs no new dependency; the test module pins
/// it against the RFC's vectors and against HMAC-SHA1 (RFC 2202).
mod sha1 {
    pub fn digest(input: &[u8]) -> [u8; 20] {
        let mut state: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
        let bit_len = (input.len() as u64).wrapping_mul(8);
        let mut padded = Vec::with_capacity(input.len() + 72);
        padded.extend_from_slice(input);
        padded.push(0x80);
        while padded.len() % 64 != 56 {
            padded.push(0);
        }
        padded.extend_from_slice(&bit_len.to_be_bytes());

        for block in padded.as_chunks::<64>().0 {
            let mut w = [0u32; 80];
            for (i, word) in block.as_chunks::<4>().0.iter().enumerate() {
                w[i] = u32::from_be_bytes(*word);
            }
            for i in 16..80 {
                w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
            }
            let (mut a, mut b, mut c, mut d, mut e) =
                (state[0], state[1], state[2], state[3], state[4]);
            for (i, word) in w.iter().enumerate() {
                let (f, k) = match i {
                    0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                    20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                    40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                    _ => (b ^ c ^ d, 0xCA62C1D6),
                };
                let temp = a
                    .rotate_left(5)
                    .wrapping_add(f)
                    .wrapping_add(e)
                    .wrapping_add(k)
                    .wrapping_add(*word);
                e = d;
                d = c;
                c = b.rotate_left(30);
                b = a;
                a = temp;
            }
            state[0] = state[0].wrapping_add(a);
            state[1] = state[1].wrapping_add(b);
            state[2] = state[2].wrapping_add(c);
            state[3] = state[3].wrapping_add(d);
            state[4] = state[4].wrapping_add(e);
        }

        let mut out = [0u8; 20];
        for (i, word) in state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{
        body_string_contains, header, header_exists, method, path, query_param,
    };
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn client(server: &MockServer) -> Client {
        let mut c = Client::new(reqwest::Client::new(), Device::new());
        c.set_bases_for_test(&server.uri(), &server.uri(), &server.uri());
        c
    }

    #[test]
    fn sha1_matches_rfc_3174_vectors() {
        assert_eq!(
            hex::encode(sha1::digest(b"abc")),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hex::encode(sha1::digest(b"")),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
        assert_eq!(
            hex::encode(sha1::digest(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
    }

    #[test]
    fn hmac_sha1_matches_rfc_2202() {
        assert_eq!(
            hex::encode(hmac_sha1(
                b"key",
                b"The quick brown fox jumps over the lazy dog"
            )),
            "de7c9b85b8b78aa6bc8a7a36f70a90701c9db4d9"
        );
        assert_eq!(
            hex::encode(hmac_sha1(
                &[0xaa; 80],
                b"Test Using Larger Than Block-Size Key - Hash Key First"
            )),
            "aa4ae5e15272d00e95705637ce8a3b55ed402112"
        );
    }

    #[test]
    fn signature_header_matches_the_apk_scheme() {
        // Expected value computed independently with Python's hmac/hashlib for
        // the same signing string the Airtel interceptor builds.
        let header = signature_header(
            FALLBACK_UID,
            FALLBACK_TOKEN,
            "GET",
            "/app/v1/config/appConfig",
            "appId=MOBILITY&os=Android&bn=13164",
            "",
        );
        assert_eq!(
            header,
            "v72UE-WQrmdaed_gi53xZLV5pa02:z6jsflBdLyLvwDid7NWkPgOy/EA="
        );
    }

    #[test]
    fn login_body_signature_covers_the_body() {
        let header = signature_header(
            FALLBACK_UID,
            FALLBACK_TOKEN,
            "post",
            "/v8/user/login",
            "",
            "{\"deviceType\":\"Airtel Xstream\",\"deviceKey\":\"APADdd8231gg\",\"requireOtp\":\"true\",\"msisdn\":\"+919999999999\"}",
        );
        assert_eq!(
            header,
            "v72UE-WQrmdaed_gi53xZLV5pa02:QwTQjPcE7APkzby88Moo8C+Z0Jc="
        );
    }

    #[test]
    fn channel_numbers_parse_as_strings_and_numbers() {
        let text: AirtelChannel = serde_json::from_str(
            r#"{"id":"MWTV_LIVETVCHANNEL_1","title":"Republic","channelNo":"101","images":{"LOGO":"https://cdn/1.png"}}"#,
        )
        .expect("string channelNo");
        assert_eq!(text.channel_no, Some(101));
        assert_eq!(text.logo(), "https://cdn/1.png");

        let number: AirtelChannel =
            serde_json::from_str(r#"{"id":"1","title":"News","channelNo":154}"#)
                .expect("numeric channelNo");
        assert_eq!(number.channel_no, Some(154));

        let missing: AirtelChannel =
            serde_json::from_str(r#"{"id":"1","title":"News"}"#).expect("absent channelNo");
        assert_eq!(missing.channel_no, None);
    }

    #[test]
    fn channel_ids_round_trip() {
        assert!(is_airtel_channel("airtel_1001"));
        assert!(!is_airtel_channel("vzy_1001"));
        assert_eq!(parse_channel_id("airtel_1001"), Some("1001"));
        assert_eq!(channel_id("1001"), "airtel_1001");
        let tv = to_television_channel(&AirtelChannel {
            id: "1001".into(),
            title: "Star Plus".into(),
            hd: true,
            imgs: HashMap::from([("LOGO".to_string(), "https://cdn/logo.png".to_string())]),
            ..Default::default()
        });
        assert_eq!(tv.id, "airtel_1001");
        assert_eq!(tv.name, "Star Plus");
        assert_eq!(tv.logo_url, "https://cdn/logo.png");
        assert!(tv.is_hd);
    }

    #[test]
    fn device_identifier_matches_the_app_layout() {
        let device = Device {
            udid: "0123456789abcdef".into(),
            name: "openstream".into(),
            model: "Airtel Xstream".into(),
        };
        assert_eq!(
            device.identifier(),
            "0123456789abcdef|Phone|Android|34|13164|1.124.6|openstream|Airtel Xstream"
        );
        assert_eq!(device.login_device_type(), "Airtel Xstream");
    }

    #[test]
    fn license_body_wraps_the_challenge_when_the_provider_expects_json() {
        let license = License {
            url: "https://play.airtel.tv/drm".into(),
            headers: Vec::new(),
            body: serde_json::Map::from_iter([
                (
                    "token".to_string(),
                    serde_json::Value::String("static-token".into()),
                ),
                (
                    "licenseChallenge".to_string(),
                    serde_json::Value::String("EXO_PAYLOAD".into()),
                ),
            ]),
            payload_key: "license".into(),
        };
        let body = license_request_body(&license, b"challenge");
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["token"], "static-token");
        assert_eq!(parsed["licenseChallenge"], "Y2hhbGxlbmdl");

        let response = license_response_body(
            &license,
            br#"{"status":"ok","license":"bGljZW5zZS1ieXRlcw=="}"#.to_vec(),
        );
        assert_eq!(response, b"license-bytes");

        let raw = License {
            payload_key: "default".into(),
            ..license.clone()
        };
        assert_eq!(license_request_body(&raw, b"challenge"), b"challenge");
        assert_eq!(
            license_response_body(&raw, b"raw-response".to_vec()),
            b"raw-response"
        );
    }

    #[test]
    fn epoch_normalisation_handles_seconds_and_millis() {
        assert_eq!(epoch_millis(1_700_000_000), 1_700_000_000_000);
        assert_eq!(epoch_millis(1_700_000_000_000), 1_700_000_000_000);
    }

    #[tokio::test]
    async fn channel_list_keeps_only_playable_partners() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/livechannel"))
            .and(query_param("appId", "MOBILITY"))
            .and(header_exists("x-atv-did"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"success":true,"data":[
                    {"id":"MWTV_LIVETVCHANNEL_1","title":"Republic Tv","cp":"MWTV","images":{"LOGO":"https://cdn/1.png"}},
                    {"id":"LIVETV_LIVETVCHANNEL_STAR_PLUS","title":"Star Plus","cp":"LIVETV"}
                ]}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        let client = client(&server);
        *client.creds.write().unwrap() = Some(Credentials {
            uid: "uid".into(),
            token: "token".into(),
            ..Default::default()
        });
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let channels = client.channels(&store).await.unwrap();
        assert_eq!(channels.len(), 1, "LIVETV is a STB-only partner");
        assert_eq!(channels[0].title, "Republic Tv");
        assert_eq!(channels[0].logo(), "https://cdn/1.png");
        assert_eq!(channels[0].cp, "MWTV");
    }

    #[tokio::test]
    async fn playback_uses_the_v3_route_and_the_channel_partner() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v3/user/content/playback"))
            .and(query_param("contentId", "1001"))
            .and(header("x-atv-cp", "MWTV"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"eligibleForPlayback":true,"playId":"https://cdn/1001/hls.m3u8","playbackType":"URL","isDrm":false,"message":"success","cp":"MWTV"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        let client = client(&server);
        *client.creds.write().unwrap() = Some(Credentials {
            uid: "uid".into(),
            token: "token".into(),
            language: "hi".into(),
            content_properties: HashMap::from([("cp".to_string(), "AIRTEL".to_string())]),
            ..Default::default()
        });
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let playback = client.playback(&store, "1001", "MWTV").await.unwrap();
        assert_eq!(playback.url, "https://cdn/1001/hls.m3u8");
        assert!(!playback.is_dash);
        assert!(playback.license.is_none());
    }

    #[tokio::test]
    async fn playback_refuses_a_drm_channel_without_a_license() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v3/user/content/playback"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"eligibleForPlayback":true,"playId":"https://cdn/1/manifest.mpd","isDrm":true,"message":"success"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        let client = client(&server);
        *client.creds.write().unwrap() = Some(Credentials {
            uid: "uid".into(),
            token: "token".into(),
            ..Default::default()
        });
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let error = client.playback(&store, "1", "MWTV").await.unwrap_err();
        assert!(error.to_string().contains("DRM"));
    }

    #[tokio::test]
    async fn verify_otp_falls_back_to_the_legacy_login_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v8/user/login"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v5/user/login"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"success":true,"uid":"uid-1","token":"token-1","authToken":"stkn-1","userConfig":{"userContentProperties":{"cp":"airtel","cl":"KA","ln":"hi"}}}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        let client = client(&server);
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let installed = std::sync::atomic::AtomicBool::new(false);
        let creds = client
            .verify_otp(&store, "+919999999999", "123456", || {
                installed.store(true, std::sync::atomic::Ordering::SeqCst);
            })
            .await
            .unwrap();
        assert!(installed.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(creds.uid, "uid-1");
        assert_eq!(creds.token, "token-1");
        assert_eq!(creds.auth_token, "stkn-1");
        assert_eq!(creds.circle, "KA");
        assert_eq!(creds.cp(), "airtel");
        assert!(Credentials::load(&store).is_some());
    }

    #[tokio::test]
    async fn send_otp_posts_the_signed_json_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/user/profile/generateOtp"))
            .and(body_string_contains("\"msisdn\":\"+919999999999\""))
            .and(body_string_contains("\"msgTxt\""))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"success":true,"message":"OTP sent"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        let client = client(&server);
        let message = client.send_otp("+919999999999").await.unwrap();
        assert_eq!(message, "OTP sent");
    }

    #[tokio::test]
    async fn refresh_preserves_uid_when_response_omits_it() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/user/session/refreshAuthToken"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"success":true,"token":"new-token"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        let client = client(&server);
        *client.creds.write().unwrap() = Some(Credentials {
            uid: "uid-1".into(),
            token: "old-token".into(),
            auth_token: "stkn-1".into(),
            ..Default::default()
        });
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let creds = client.refresh(&store).await.unwrap();
        assert_eq!(creds.uid, "uid-1");
        assert_eq!(creds.token, "new-token");
        assert_eq!(creds.auth_token, "stkn-1");
    }

    #[tokio::test]
    async fn refresh_rejects_credentials_replaced_by_a_login() {
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
                        r#"{"success":true,"token":"refreshed-token"}"#,
                        "application/json",
                    )
            })
            .expect(1)
            .mount(&server)
            .await;
        let client = std::sync::Arc::new(client(&server));
        *client.creds.write().unwrap() = Some(Credentials {
            uid: "uid-1".into(),
            token: "old-token".into(),
            ..Default::default()
        });
        let dir = tempfile::tempdir().unwrap();
        let store =
            std::sync::Arc::new(crate::store::Store::open(dir.path().to_str().unwrap()).unwrap());

        let refresh_store = store.clone();
        let refresh_client = client.clone();
        let refresh = tokio::spawn(async move { refresh_client.refresh(&refresh_store).await });
        refresh_started.notified().await;
        let login = Credentials {
            uid: "uid-2".into(),
            token: "login-token".into(),
            ..Default::default()
        };
        login.save(&store).unwrap();
        *client.creds.write().unwrap() = Some(login);
        let error = refresh.await.unwrap().unwrap_err();
        assert!(error
            .to_string()
            .contains("credentials changed during token refresh"));
        assert_eq!(client.credentials().unwrap().token, "login-token");
        assert_eq!(Credentials::load(&store).unwrap().token, "login-token");
    }

    #[tokio::test]
    async fn rejected_v8_keeps_its_message_when_v5_returns_empty_404() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v8/user/login"))
            .respond_with(ResponseTemplate::new(403).set_body_raw(
                r#"{"success":false,"errorcode":"ATV403","errortitle":"OTP Incorrect"}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v5/user/login"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let client = client(&server);
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let installed = std::sync::atomic::AtomicBool::new(false);
        let err = client
            .verify_otp(&store, "+919999999999", "000000", || {
                installed.store(true, std::sync::atomic::Ordering::SeqCst);
            })
            .await
            .unwrap_err();
        assert!(err.to_string().contains("OTP Incorrect"));
        assert!(!installed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn channel_epg_parses_the_program_guide_map() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/app/v2/content/channel/epg"))
            .and(query_param("channelId", "1001"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"programGuide":{"1001":[{"channelId":"1001","title":"Show","subtitle":"Episode 1","startTime":1700000000000,"endTime":1700003600000}]}}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        let client = client(&server);
        *client.creds.write().unwrap() = Some(Credentials {
            uid: "uid".into(),
            token: "token".into(),
            ..Default::default()
        });
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let programs = client
            .channel_epg(&store, "1001", 1_700_000_000_000, 1_700_003_600_000)
            .await
            .unwrap();
        assert_eq!(programs.len(), 1);
        assert_eq!(programs[0].title, "Show");
        assert_eq!(programs[0].start_millis(), 1_700_000_000_000);
    }
}
