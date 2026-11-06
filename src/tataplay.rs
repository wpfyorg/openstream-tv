//! Tata Play Smart experimental provider.
//!
//! Contracts come from the com.tataplay.livetv 1.0.10 Retrofit layer.
//! This provider is diagnostic-only until runtime parity is proven.

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const PACKAGE: &str = "com.tataplay.livetv";
pub const VERSION_NAME: &str = "1.0.10";
pub const VERSION_CODE: u32 = 101;
pub const ARTIFACT_SHA256: &str =
    "303c82f8f97db44e0da49a4bf394f31b65774541d0b74611fb56a1b00fc05e75";
pub const BASE_URL: &str = "https://tm.tapi.videoready.tv/";
pub const PLATFORM: &str = "STV";

pub const STORE_KEY_DEVICE: &str = "tataplay_device";
pub const STORE_KEY_PENDING: &str = "tataplay_pending_login";
pub const STORE_KEY_CREDENTIALS: &str = "tataplay_credentials";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub device_id: String,
    pub device_name: String,
}

impl Device {
    pub fn new() -> Device {
        let mut bytes = [0u8; 8];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
        Device {
            device_id: hex::encode(bytes),
            device_name: "JioTV Go Tata Play Beta".to_string(),
        }
    }

    pub fn load_or_create(store: &crate::store::Store) -> anyhow::Result<Device> {
        if let Some(raw) = store.get_opt(STORE_KEY_DEVICE) {
            if let Ok(device) = serde_json::from_str::<Device>(&raw) {
                if !device.device_id.is_empty() && !device.device_name.is_empty() {
                    return Ok(device);
                }
            }
        }
        let device = Device::new();
        store.set(STORE_KEY_DEVICE, &serde_json::to_string(&device)?)?;
        Ok(device)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Credentials {
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub subscriber_id: String,
    #[serde(default)]
    pub sid: String,
}

impl Credentials {
    pub fn load(store: &crate::store::Store) -> Option<Credentials> {
        let raw = store.get_opt(STORE_KEY_CREDENTIALS)?;
        let credentials: Credentials = serde_json::from_str(&raw).ok()?;
        (!credentials.access_token.is_empty()).then_some(credentials)
    }

    fn save(&self, store: &crate::store::Store) -> anyhow::Result<()> {
        store.set(STORE_KEY_CREDENTIALS, &serde_json::to_string(self)?)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoginRef {
    pub ref_id: String,
    pub services: Vec<String>,
    pub account_service_type: String,
    pub native_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PendingLogin {
    rmn: String,
    sid: String,
    jwt_token: String,
    refs: Vec<LoginRef>,
}

impl PendingLogin {
    fn load(store: &crate::store::Store) -> Option<PendingLogin> {
        let raw = store.get_opt(STORE_KEY_PENDING)?;
        serde_json::from_str(&raw).ok()
    }

    fn save(&self, store: &crate::store::Store) -> anyhow::Result<()> {
        store.set(STORE_KEY_PENDING, &serde_json::to_string(self)?)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base: String,
    device: Device,
    credentials: Option<Credentials>,
}

impl Client {
    pub fn from_store(
        http: reqwest::Client,
        store: &crate::store::Store,
    ) -> anyhow::Result<Client> {
        Ok(Client {
            http,
            base: BASE_URL.to_string(),
            device: Device::load_or_create(store)?,
            credentials: Credentials::load(store),
        })
    }

    #[cfg(test)]
    fn with_base(http: reqwest::Client, device: Device, base: &str) -> Client {
        Client {
            http,
            base: format!("{}/", base.trim_end_matches('/')),
            device,
            credentials: None,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path.trim_start_matches('/'))
    }

    fn auth(&self) -> anyhow::Result<String> {
        let token = self
            .credentials
            .as_ref()
            .map(|c| c.access_token.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("tataplay: not logged in"))?;
        Ok(format!("bearer {token}"))
    }

    fn credentials(&self) -> anyhow::Result<&Credentials> {
        self.credentials
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("tataplay: not logged in"))
    }

    async fn checked_json(
        &self,
        request: reqwest::RequestBuilder,
        operation: &str,
    ) -> anyhow::Result<Value> {
        let response = request.send().await?;
        if !response.status().is_success() {
            anyhow::bail!("tataplay: {operation} failed: HTTP {}", response.status());
        }
        Ok(response.json::<Value>().await?)
    }

    pub async fn send_otp(
        &self,
        store: &crate::store::Store,
        rmn: &str,
        sid: &str,
    ) -> anyhow::Result<Vec<LoginRef>> {
        if rmn.trim().is_empty() {
            anyhow::bail!("tataplay: RMN is required");
        }
        let value = self
            .checked_json(
                self.http
                    .post(self.url("login-service/pub/api/v3/signin/generate/otp"))
                    .header("version", VERSION_NAME)
                    .header("platform", PLATFORM)
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .header("deviceId", &self.device.device_id)
                    .header("deviceName", &self.device.device_name)
                    .json(&json!({"rmn": rmn, "sid": sid})),
                "send OTP",
            )
            .await?;
        let data = payload(&value);
        let jwt_token = string_field(data, "jwtToken");
        if jwt_token.is_empty() {
            anyhow::bail!("tataplay: send OTP response has no jwtToken");
        }
        let refs = decode_login_refs(&jwt_token)?;
        PendingLogin {
            rmn: rmn.to_string(),
            sid: sid.to_string(),
            jwt_token,
            refs: refs.clone(),
        }
        .save(store)?;
        Ok(refs)
    }

    pub async fn verify_otp(
        &self,
        store: &crate::store::Store,
        otp: &str,
        ref_id: &str,
        ref_id_mode: bool,
    ) -> anyhow::Result<Credentials> {
        let pending =
            PendingLogin::load(store).ok_or_else(|| anyhow::anyhow!("tataplay: send OTP first"))?;
        if otp.trim().is_empty() || ref_id.trim().is_empty() {
            anyhow::bail!("tataplay: OTP and refId are required");
        }
        if !pending
            .refs
            .iter()
            .any(|candidate| candidate.ref_id == ref_id)
        {
            anyhow::bail!("tataplay: refId was not offered by the pending login");
        }
        let body = json!({
            "authorization": otp.trim(),
            "rmn": pending.rmn,
            "sid": if ref_id_mode { ref_id } else { "" },
            "refId": ref_id,
            "loginOption": "OTP",
            "jwtToken": pending.jwt_token,
            "litTestLoginFlow": true,
            "mixPanelId": "",
            "moEngageId": ""
        });
        let value = self
            .checked_json(
                self.http
                    .post(self.url("login-service/pub/api/v3/signin/validate/otp"))
                    .header("version", VERSION_NAME)
                    .header("platform", PLATFORM)
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    // The app forwards an opaque persisted DEVICE_DETAILS string.
                    // Its encoding is not recovered, and the app's missing-value
                    // behavior is the empty string, so the beta deliberately does
                    // not fabricate a device-details payload.
                    .header("device_details", "")
                    .json(&body),
                "verify OTP",
            )
            .await?;
        let credentials = credentials_from_response(&value)?;
        credentials.save(store)?;
        store.delete(STORE_KEY_PENDING)?;
        Ok(credentials)
    }

    pub async fn refresh(&self, store: &crate::store::Store) -> anyhow::Result<Credentials> {
        let current = self.credentials()?.clone();
        if current.refresh_token.is_empty() || current.subscriber_id.is_empty() {
            anyhow::bail!("tataplay: refresh token or subscriberId is missing");
        }
        let value = self
            .checked_json(
                self.http
                    .post(self.url("rest-api/pub/api/v3/refreshToken"))
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .header("device_details", "")
                    .header("platform", PLATFORM)
                    .header(reqwest::header::AUTHORIZATION, self.auth()?)
                    .header("locale", "ENG")
                    .json(&json!({
                        "refreshToken": current.refresh_token,
                        "subscriberId": current.subscriber_id
                    })),
                "refresh token",
            )
            .await?;
        let mut refreshed = credentials_from_response(&value)?;
        if refreshed.refresh_token.is_empty() {
            refreshed.refresh_token = current.refresh_token;
        }
        if refreshed.subscriber_id.is_empty() {
            refreshed.subscriber_id = current.subscriber_id;
        }
        if refreshed.sid.is_empty() {
            refreshed.sid = current.sid;
        }
        refreshed.save(store)?;
        Ok(refreshed)
    }

    pub async fn catalogue_seed(&self, limit: u32, offset: u32) -> anyhow::Result<Value> {
        let value = self
            .checked_json(
                self.http
                    .get(self.url("portal-search/pub/api/v1/channels/all"))
                    .header("version", VERSION_NAME)
                    .header("platform", PLATFORM)
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .query(&[("limit", limit), ("offset", offset)]),
                "catalogue seed",
            )
            .await?;
        Ok(sanitize_catalogue_seed(&value))
    }

    pub async fn guide(&self, query: &GuideQuery) -> anyhow::Result<Value> {
        query.validate()?;
        let authenticated = self.credentials.is_some();
        let params = query.params(authenticated);
        let request = if let Some(credentials) = &self.credentials {
            self.http
                .get(self.url("lit-mobile-common-bff/api/v2/channels/guide"))
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .header("sid", &credentials.sid)
                .header(reqwest::header::AUTHORIZATION, self.auth()?)
                .header("platform", PLATFORM)
                .query(&params)
        } else {
            self.http
                .get(self.url("portal-search/pub/api/v2/channels/guide"))
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .header("device_details", "")
                .header("platform", PLATFORM)
                .query(&params)
        };
        let value = self.checked_json(request, "EPG guide").await?;
        Ok(sanitize_guide(&value))
    }

    pub async fn entitlement(&self, force_refresh: Option<bool>) -> anyhow::Result<Value> {
        let mut request = self
            .http
            .get(self.url("lit-subscription-service/api/v1/lit/getEntitlementDetails"))
            .header(reqwest::header::AUTHORIZATION, self.auth()?);
        if let Some(force_refresh) = force_refresh {
            request = request.query(&[("forceRefresh", force_refresh)]);
        }
        let value = self.checked_json(request, "entitlement").await?;
        Ok(sanitize_entitlement(&value))
    }

    pub async fn access_control(
        &self,
        channel_id: &str,
        content_type: &str,
    ) -> anyhow::Result<Value> {
        let credentials = self.credentials()?;
        if channel_id.trim().is_empty() {
            anyhow::bail!("tataplay: channel id is required");
        }
        let content_type = if content_type.trim().is_empty() {
            "LIVE"
        } else {
            content_type.trim()
        };
        let response = self
            .http
            .get(self.url(&format!(
                "pas/api/v1/access/control/{}/{}",
                urlencoding::encode(content_type),
                urlencoding::encode(channel_id)
            )))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::AUTHORIZATION, self.auth()?)
            .header("x-subscriber-id", &credentials.subscriber_id)
            .header("platform", PLATFORM)
            .header("x-device-id", &self.device.device_id)
            .header("x-device-platform", "MOBILE")
            .header("x-device-type", "ANDROID")
            .header("x-subscriber-name", "Ancfg Both")
            .header("x-device-name", &self.device.device_name)
            .send()
            .await?;
        let status = response.status();
        let value = response.json::<Value>().await.unwrap_or(Value::Null);
        Ok(sanitize_access_control(
            status.as_u16(),
            content_type,
            &value,
        ))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideQuery {
    #[serde(default)]
    pub offset: Option<u32>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub category_filters: Option<String>,
    #[serde(default)]
    pub language_filters: Option<String>,
    pub date: Option<String>,
    pub epg_start_time: Option<String>,
    pub epg_end_time: Option<String>,
    #[serde(default)]
    pub subscribed_filter: Option<bool>,
}

impl GuideQuery {
    fn validate(&self) -> anyhow::Result<()> {
        if self.date.as_deref().unwrap_or("").is_empty()
            || self.epg_start_time.as_deref().unwrap_or("").is_empty()
            || self.epg_end_time.as_deref().unwrap_or("").is_empty()
        {
            anyhow::bail!(
                "tataplay: date, epgStartTime and epgEndTime are required; runtime encoding is intentionally not guessed"
            );
        }
        Ok(())
    }

    fn params(&self, authenticated: bool) -> Vec<(String, String)> {
        let mut out = vec![
            ("offset".into(), self.offset.unwrap_or(0).to_string()),
            ("limit".into(), self.limit.unwrap_or(100).to_string()),
            ("date".into(), self.date.clone().unwrap_or_default()),
            (
                "epgStartTime".into(),
                self.epg_start_time.clone().unwrap_or_default(),
            ),
            (
                "epgEndTime".into(),
                self.epg_end_time.clone().unwrap_or_default(),
            ),
        ];
        if let Some(value) = &self.category_filters {
            out.push(("categoryFilters".into(), value.clone()));
        }
        if let Some(value) = &self.language_filters {
            out.push(("languageFilters".into(), value.clone()));
        }
        if authenticated {
            out.push((
                "subscribedFilter".into(),
                self.subscribed_filter.unwrap_or(false).to_string(),
            ));
        }
        out
    }
}

fn payload(value: &Value) -> &Value {
    value.get("data").filter(|v| !v.is_null()).unwrap_or(value)
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn credentials_from_response(value: &Value) -> anyhow::Result<Credentials> {
    let data = payload(value);
    let access_token = string_field(data, "accessToken");
    if access_token.is_empty() {
        anyhow::bail!("tataplay: login response has no accessToken");
    }
    let sid = data
        .get("userDetails")
        .and_then(|u| u.get("sid"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| data.get("sid").and_then(Value::as_str).unwrap_or_default())
        .to_string();
    Ok(Credentials {
        access_token,
        refresh_token: string_field(data, "refreshToken"),
        subscriber_id: string_field(data, "subscriberId"),
        sid,
    })
}

fn decode_login_refs(jwt: &str) -> anyhow::Result<Vec<LoginRef>> {
    let encoded = jwt
        .split('.')
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("tataplay: malformed login JWT"))?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(encoded))?;
    let value: Value = serde_json::from_slice(&decoded)?;
    let refs = value
        .get("refIds")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("tataplay: login JWT has no refIds"))?;
    Ok(refs
        .iter()
        .filter_map(|candidate| {
            let services: Vec<String> = candidate
                .get("services")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            if !services.iter().any(|service| {
                let upper = service.to_ascii_uppercase();
                upper.contains("LIT") || upper.contains("DTH")
            }) {
                return None;
            }
            let ref_id = string_field(candidate, "refId");
            if ref_id.is_empty() {
                return None;
            }
            Some(LoginRef {
                ref_id,
                services,
                account_service_type: string_field(candidate, "accountServiceType"),
                native_name: string_field(candidate, "nativeName"),
            })
        })
        .collect())
}

fn sanitize_catalogue_seed(value: &Value) -> Value {
    let data = payload(value);
    json!({
        "documentIds": data.get("documentIds").cloned().unwrap_or_else(|| json!({})),
        "hydratedChannels": false
    })
}

fn sanitize_guide(value: &Value) -> Value {
    let data = payload(value);
    let channels = data
        .get("channelList")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(sanitize_guide_channel).collect::<Vec<_>>())
        .unwrap_or_default();
    json!({
        "total": data.get("total").cloned().unwrap_or(Value::Null),
        "offset": data.get("offset").cloned().unwrap_or(Value::Null),
        "limit": data.get("limit").cloned().unwrap_or(Value::Null),
        "date": data.get("date").cloned().unwrap_or(Value::Null),
        "epochDate": data.get("epochDate").cloned().unwrap_or(Value::Null),
        "channels": channels
    })
}

fn sanitize_guide_channel(channel: &Value) -> Value {
    let programmes = channel
        .get("epg")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    json!({
                        "id": item.get("id").cloned().unwrap_or(Value::Null),
                        "startTime": item.get("startTime").cloned().unwrap_or(Value::Null),
                        "endTime": item.get("endTime").cloned().unwrap_or(Value::Null),
                        "title": item.get("title").cloned().unwrap_or(Value::Null),
                        "desc": item.get("desc").cloned().unwrap_or(Value::Null),
                        "seriesId": item.get("seriesId").cloned().unwrap_or(Value::Null),
                        "epgState": item.get("epgState").cloned().unwrap_or(Value::Null),
                        "epgSlotId": item.get("epgSlotId").cloned().unwrap_or(Value::Null),
                        "catchup": item.get("catchup").cloned().unwrap_or(Value::Null)
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "id": channel.get("id").cloned().unwrap_or(Value::Null),
        "title": channel.get("title").cloned().unwrap_or(Value::Null),
        "channelNumber": channel.get("channelNumber").cloned().unwrap_or(Value::Null),
        "contentType": channel.get("contentType").cloned().unwrap_or(Value::Null),
        "genres": channel.get("genres").cloned().unwrap_or(Value::Null),
        "entitlements": channel.get("entitlements").cloned().unwrap_or(Value::Null),
        "provider": channel.get("provider").cloned().unwrap_or(Value::Null),
        "hdChannel": channel.get("hdChannel").cloned().unwrap_or(Value::Null),
        "restartTv": channel.get("restartTv").cloned().unwrap_or(Value::Null),
        "cloudDvr": channel.get("cloudDvr").cloned().unwrap_or(Value::Null),
        "epg": programmes
    })
}

fn sanitize_entitlement(value: &Value) -> Value {
    let data = payload(value);
    json!({
        "acStatus": data.get("acStatus").cloned().unwrap_or(Value::Null),
        "accountStatus": data.get("accountStatus").cloned().unwrap_or(Value::Null),
        "subscriptionStatus": data.get("subscriptionStatus").cloned().unwrap_or(Value::Null),
        "litSubscribed": data.get("litSubscribed").cloned().unwrap_or(Value::Null),
        "litUser": data.get("litUser").cloned().unwrap_or(Value::Null),
        "dthUser": data.get("dthUser").cloned().unwrap_or(Value::Null),
        "concurrency": data.get("concurrency").cloned().unwrap_or(Value::Null),
        "startDate": data.get("startDate").cloned().unwrap_or(Value::Null),
        "endDate": data.get("endDate").cloned().unwrap_or(Value::Null),
        "litPackRenewalDate": data.get("litPackRenewalDate").cloned().unwrap_or(Value::Null),
        "packNames": data.get("packNames").cloned().unwrap_or(Value::Null),
        "packGenres": data.get("packGenres").cloned().unwrap_or(Value::Null),
        "packLanguages": data.get("packLanguages").cloned().unwrap_or(Value::Null),
        "packPrice": data.get("packPrice").cloned().unwrap_or(Value::Null)
    })
}

fn sanitize_access_control(status: u16, content_type: &str, value: &Value) -> Value {
    let data = payload(value);
    let has_play_url = [
        "dashWidewinePlayUrl",
        "dashPlayreadyPlayUrl",
        "fairplayPlayUrl",
        "trailerUrl",
    ]
    .iter()
    .any(|key| !string_field(data, key).is_empty());
    let entitlement_count = data
        .get("entitlements")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    json!({
        "upstreamStatus": status,
        "authorized": (200..300).contains(&status) && has_play_url,
        "contentType": data.get("contentType").and_then(Value::as_str).unwrap_or(content_type),
        "contentId": data.get("contentId").cloned().unwrap_or(Value::Null),
        "hasDashWidevinePlayUrl": !string_field(data, "dashWidewinePlayUrl").is_empty(),
        "hasDashWidevineLicenseUrl": !string_field(data, "dashWidewineLicenseUrl").is_empty(),
        "hasDashPlayreadyPlayUrl": !string_field(data, "dashPlayreadyPlayUrl").is_empty(),
        "hasDashPlayreadyLicenseUrl": !string_field(data, "dashPlayreadyLicenseUrl").is_empty(),
        "hasSessionId": !string_field(data, "sessionId").is_empty(),
        "heartbeatInterval": data.get("heartbeatInterval").cloned().unwrap_or(Value::Null),
        "entitlementCount": entitlement_count
    })
}

pub fn logout(store: &crate::store::Store) -> anyhow::Result<()> {
    store.delete(STORE_KEY_PENDING)?;
    store.delete(STORE_KEY_CREDENTIALS)?;
    Ok(())
}

pub fn status(enabled: bool, connected: bool) -> Value {
    json!({
        "enabled": enabled,
        "connected": connected,
        "beta": true,
        "package": PACKAGE,
        "version": VERSION_NAME,
        "versionCode": VERSION_CODE,
        "sha256": ARTIFACT_SHA256,
        "sharedPlayback": false,
        "syntheticDeviceIdentity": true,
        "implemented": {
            "login": true,
            "refresh": true,
            "catalogueSeed": true,
            "epg": true,
            "entitlement": true,
            "accessControlDiagnostic": true,
            "sharedCatalogue": false,
            "liveProxy": false,
            "licenseProxy": false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn device() -> Device {
        Device {
            device_id: "0123456789abcdef".into(),
            device_name: "Test TV".into(),
        }
    }

    fn jwt_with_refs() -> String {
        let payload = json!({
            "refIds": [
                {"refId":"lit-1","services":["LIT"],"accountServiceType":"LIT","nativeName":"Primary"},
                {"refId":"dth-2","services":["DTH"],"accountServiceType":"DTH","nativeName":"DTH"},
                {"refId":"other","services":["OTHER"],"accountServiceType":"OTHER","nativeName":"Other"}
            ]
        });
        format!(
            "e30.{}.sig",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string())
        )
    }

    #[test]
    fn device_identity_persists() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let first = Device::load_or_create(&store).unwrap();
        let second = Device::load_or_create(&store).unwrap();
        assert_eq!(first.device_id, second.device_id);
        assert_eq!(first.device_name, second.device_name);
    }

    #[test]
    fn jwt_refs_keep_only_lit_and_dth_accounts() {
        let refs = decode_login_refs(&jwt_with_refs()).unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].ref_id, "lit-1");
        assert_eq!(refs[1].ref_id, "dth-2");
    }

    #[tokio::test]
    async fn send_otp_uses_recovered_retrofit_contract() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login-service/pub/api/v3/signin/generate/otp"))
            .and(header("version", VERSION_NAME))
            .and(header("platform", PLATFORM))
            .and(header("deviceId", "0123456789abcdef"))
            .and(header("deviceName", "Test TV"))
            .and(body_json(json!({"rmn":"9876543210","sid":""})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {"jwtToken": jwt_with_refs()}
            })))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let client = Client::with_base(reqwest::Client::new(), device(), &server.uri());
        let refs = client.send_otp(&store, "9876543210", "").await.unwrap();
        assert_eq!(refs.len(), 2);
        assert!(PendingLogin::load(&store).is_some());
    }

    #[tokio::test]
    async fn verify_otp_preserves_ref_id_mode_and_persists_session() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login-service/pub/api/v3/signin/validate/otp"))
            .and(header("platform", PLATFORM))
            .and(header("device_details", ""))
            .and(body_json(json!({
                "authorization":"123456",
                "rmn":"9876543210",
                "sid":"",
                "refId":"lit-1",
                "loginOption":"OTP",
                "jwtToken":jwt_with_refs(),
                "litTestLoginFlow":true,
                "mixPanelId":"",
                "moEngageId":""
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "accessToken":"access-value",
                    "refreshToken":"refresh-value",
                    "subscriberId":"sub-1",
                    "userDetails":{"sid":"sid-1"}
                }
            })))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        PendingLogin {
            rmn: "9876543210".into(),
            sid: "".into(),
            jwt_token: jwt_with_refs(),
            refs: decode_login_refs(&jwt_with_refs()).unwrap(),
        }
        .save(&store)
        .unwrap();
        let client = Client::with_base(reqwest::Client::new(), device(), &server.uri());
        let credentials = client
            .verify_otp(&store, "123456", "lit-1", false)
            .await
            .unwrap();
        assert_eq!(credentials.subscriber_id, "sub-1");
        assert_eq!(credentials.sid, "sid-1");
        assert!(Credentials::load(&store).is_some());
        assert!(store.get_opt(STORE_KEY_PENDING).is_none());
    }

    #[tokio::test]
    async fn catalogue_seed_stays_document_ids_only() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/portal-search/pub/api/v1/channels/all"))
            .and(query_param("limit", "50"))
            .and(query_param("offset", "0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data":{"documentIds":{"10":"doc-a","11":"doc-b"}}
            })))
            .mount(&server)
            .await;
        let client = Client::with_base(reqwest::Client::new(), device(), &server.uri());
        let seed = client.catalogue_seed(50, 0).await.unwrap();
        assert_eq!(seed["documentIds"]["10"], "doc-a");
        assert_eq!(seed["hydratedChannels"], false);
        assert!(seed.get("channels").is_none());
    }

    #[tokio::test]
    async fn access_control_uses_recovered_headers_and_returns_only_diagnostics() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/pas/api/v1/access/control/LIVE/123"))
            .and(header("Authorization", "bearer access-value"))
            .and(header("x-subscriber-id", "sub-1"))
            .and(header("platform", PLATFORM))
            .and(header("x-device-id", "0123456789abcdef"))
            .and(header("x-device-platform", "MOBILE"))
            .and(header("x-device-type", "ANDROID"))
            .and(header("x-subscriber-name", "Ancfg Both"))
            .and(header("x-device-name", "Test TV"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "contentId":"123",
                    "contentType":"LIVE",
                    "dashWidewinePlayUrl":"https://cdn.invalid/manifest.mpd",
                    "dashWidewineLicenseUrl":"https://license.invalid/wv",
                    "sessionId":"session-value"
                }
            })))
            .mount(&server)
            .await;
        let client = Client {
            http: reqwest::Client::new(),
            base: format!("{}/", server.uri()),
            device: device(),
            credentials: Some(Credentials {
                access_token: "access-value".into(),
                refresh_token: "refresh-value".into(),
                subscriber_id: "sub-1".into(),
                sid: "sid-1".into(),
            }),
        };
        let safe = client.access_control("123", "LIVE").await.unwrap();
        assert_eq!(safe["authorized"], true);
        let encoded = safe.to_string();
        assert!(!encoded.contains("cdn.invalid"));
        assert!(!encoded.contains("license.invalid"));
        assert!(!encoded.contains("session-value"));
    }

    #[test]
    fn access_diagnostic_drops_urls_and_session_id() {
        let raw = json!({
            "data": {
                "contentId":"123",
                "contentType":"LIVE",
                "dashWidewinePlayUrl":"https://cdn.invalid/manifest.mpd",
                "dashWidewineLicenseUrl":"https://license.invalid/wv",
                "sessionId":"session-value",
                "heartbeatInterval":30,
                "entitlements":["pack-a"]
            }
        });
        let safe = sanitize_access_control(200, "LIVE", &raw);
        assert_eq!(safe["authorized"], true);
        assert_eq!(safe["hasDashWidevinePlayUrl"], true);
        assert_eq!(safe["hasDashWidevineLicenseUrl"], true);
        let encoded = safe.to_string();
        assert!(!encoded.contains("cdn.invalid"));
        assert!(!encoded.contains("license.invalid"));
        assert!(!encoded.contains("session-value"));
    }
}
