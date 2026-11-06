//! VZY (`com.vzy.one.atv`, 1.0.253) — Dish/Watcho white-label OTT.
//!
//! Evidence: `docs/ott-providers/vzy.md`. Contracts below were recovered from
//! the app's own `BuildConfig`, `SplashRepository`, `KalturaServices`,
//! `DataViewModel`, `EpgInterface`, and verified with live anonymous requests
//! (no account): DMS bootstrap, anonymous Kaltura session, 161-channel
//! catalogue, free-tier HLS playback and EPG.
//!
//! Only the free tier is implemented: `EntitlementType == "Free"` channels.
//! Premium/login-required channels are never advertised or proxied because
//! doing so would require an entitlement we do not have.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const ID_PREFIX: &str = "vzy_";
pub const PACKAGE: &str = "com.vzy.one.atv";
pub const VERSION_NAME: &str = "1.0.253";
pub const VERSION_CODE: u32 = 4_101_253;
pub const ARTIFACT_SHA256: &str =
    "7cd4724cd030bc0566d8732eba4961d1ba7d0170503b615ac5b4c0d1ce80b275";

pub const PARTNER_ID: i64 = 5093;
const API_VERSION: &str = "11.0.2.1";
const APPLICATION_NAME: &str = "com.Kaltura.dishtv.androidTV";
const CLIENT_VERSION: &str = "1.0.0";
const PLATFORM: &str = "STB";
/// `BuildConfig.KALTURA_BASE_URL`.
pub(crate) const BASE_KALTURA: &str = "https://5093.dsp1.ott.kaltura.com/api_v3/service/";
/// `BuildConfig.EPG_BASE_URL`.
pub(crate) const BASE_EPG: &str = "https://cache.dsp1.ott.kaltura.com/api_v3/";
/// `DmsResponse.params.miniEPGCollectionId`, verified live.
pub const DEFAULT_COLLECTION_ID: i64 = 366800;
const USER_AGENT: &str = "okhttp/4.12.0";

pub const STORE_KEY_DEVICE: &str = "vzy_device";
pub const STORE_KEY_SESSION: &str = "vzy_session";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    #[serde(default)]
    pub udid: String,
}

impl Device {
    pub fn new() -> Device {
        let mut bytes = [0u8; 8];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
        Device {
            udid: format!("ATV_{}", hex::encode(bytes)),
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub ks: String,
    /// Unix seconds from `result.expiry`.
    pub expiry: u64,
}

impl Session {
    pub fn is_fresh(&self, now: SystemTime) -> bool {
        let now = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        self.expiry > now + 60
    }

    pub fn load(store: &crate::store::Store) -> Option<Session> {
        let json = store.get_opt(STORE_KEY_SESSION)?;
        let s: Session = serde_json::from_str(&json).ok()?;
        if s.ks.is_empty() {
            return None;
        }
        Some(s)
    }

    pub fn save(&self, store: &crate::store::Store) -> anyhow::Result<()> {
        store.set(STORE_KEY_SESSION, &serde_json::to_string(self)?)?;
        Ok(())
    }
}

pub fn delete_session(store: &crate::store::Store) -> anyhow::Result<()> {
    let _ = store.delete(STORE_KEY_SESSION);
    Ok(())
}

pub fn content_id(channel_id: &str) -> Option<&str> {
    channel_id.strip_prefix(ID_PREFIX)
}

pub fn channel_id(content_id: &str) -> String {
    format!("{ID_PREFIX}{content_id}")
}

pub fn is_vzy_channel(id: &str) -> bool {
    id.starts_with(ID_PREFIX)
}

fn meta_value<'a>(asset: &'a Asset, key: &str) -> Option<&'a serde_json::Value> {
    asset.metas.get(key).map(|m| &m.value)
}

fn meta_string(asset: &Asset, key: &str) -> Option<String> {
    match meta_value(asset, key)? {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// A catalogue item. Only `EntitlementType == "Free"` assets are exposed.
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)] // fields are part of the observed Kaltura asset model.
pub struct Asset {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "externalId")]
    pub external_id: String,
    #[serde(default, rename = "entryId")]
    pub entry_id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub images: Vec<Image>,
    #[serde(default)]
    pub metas: HashMap<String, Meta>,
    #[serde(default, rename = "mediaFiles")]
    pub media_files: Vec<MediaFile>,
}

impl Asset {
    pub fn is_free(&self) -> bool {
        meta_string(self, "EntitlementType").as_deref() == Some("Free")
    }

    pub fn logo(&self) -> String {
        for wanted in ["16x9", "1x1", "2x3"] {
            if let Some(img) = self
                .images
                .iter()
                .find(|i| i.image_type_name == wanted && !i.url.is_empty())
            {
                return img.url.clone();
            }
        }
        self.images
            .iter()
            .find(|i| !i.url.is_empty())
            .map(|i| i.url.clone())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Image {
    #[serde(default, rename = "imageTypeName")]
    pub image_type_name: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MediaFile {
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EpgProgram {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, rename = "startDate")]
    pub start_date: i64,
    #[serde(default, rename = "endDate")]
    pub end_date: i64,
}

#[derive(Debug, Clone, Default)]
pub struct Playback {
    pub hls: String,
    pub dash: String,
}

#[derive(Debug, Deserialize)]
struct DmsEnvelope {
    #[serde(default)]
    params: Option<DmsParams>,
}

#[derive(Debug, Default, Deserialize)]
struct DmsParams {
    #[serde(default, rename = "miniEPGCollectionId")]
    mini_epg_collection_id: Option<serde_json::Value>,
    #[serde(default, rename = "apiProxyUrlKaltura")]
    api_proxy_url_kaltura: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginEnvelope {
    #[serde(default)]
    result: Option<LoginResult>,
}

#[derive(Debug, Deserialize)]
struct LoginResult {
    #[serde(default)]
    ks: String,
    #[serde(default)]
    expiry: u64,
}

#[derive(Debug, Deserialize)]
struct AssetListEnvelope {
    #[serde(default)]
    result: Option<AssetListResult>,
}

#[derive(Debug, Default, Deserialize)]
struct AssetListResult {
    #[serde(default)]
    objects: Vec<Asset>,
    #[serde(default, rename = "totalCount")]
    total_count: i64,
}

#[derive(Debug, Deserialize)]
struct AssetEnvelope {
    #[serde(default)]
    result: Option<Asset>,
}

#[derive(Debug, Deserialize)]
struct EpgEnvelope {
    #[serde(default, rename = "epgChunk")]
    epg_chunk: HashMap<String, Vec<EpgProgram>>,
}

pub struct Client {
    pub http: reqwest::Client,
    pub device: Device,
    pub base: RwLock<String>,
    pub epg_base: String,
    pub collection_id: RwLock<i64>,
    pub session: RwLock<Option<Session>>,
}

impl Client {
    pub fn new(http: reqwest::Client, device: Device) -> Client {
        Client {
            http,
            device,
            base: RwLock::new(BASE_KALTURA.to_string()),
            epg_base: BASE_EPG.to_string(),
            collection_id: RwLock::new(DEFAULT_COLLECTION_ID),
            session: RwLock::new(None),
        }
    }

    #[cfg(test)]
    pub fn set_bases_for_test(&mut self, base: &str, epg_base: &str) {
        *self.base.write().unwrap() = format!("{base}/");
        self.epg_base = format!("{epg_base}/");
    }

    fn service_url(&self, path: &str) -> String {
        format!("{}{path}", self.base.read().unwrap())
    }

    /// `configurations/action/serveByDevice`; supplies the EPG collection id
    /// and, when present, the Kaltura proxy base.
    pub async fn bootstrap(&self, store: &crate::store::Store) -> anyhow::Result<()> {
        let url = format!("{BASE_KALTURA}configurations/action/serveByDevice");
        let body = serde_json::json!({
            "apiVersion": API_VERSION,
            "applicationName": APPLICATION_NAME,
            "clientVersion": CLIENT_VERSION,
            "partnerId": PARTNER_ID,
            "platform": PLATFORM,
            "tag": "",
            "udid": self.device.udid,
        });
        let resp = self
            .http
            .post(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("vzy: DMS bootstrap failed: HTTP {}", resp.status());
        }
        let dms: DmsEnvelope = resp.json().await?;
        let params = dms.params.unwrap_or_default();
        let collection_id = params
            .mini_epg_collection_id
            .as_ref()
            .and_then(|v| match v {
                serde_json::Value::Number(n) => n.as_i64(),
                serde_json::Value::String(s) => s.parse().ok(),
                _ => None,
            })
            .unwrap_or(DEFAULT_COLLECTION_ID);
        if let Some(base) = params.api_proxy_url_kaltura.filter(|s| !s.is_empty()) {
            *self.base.write().unwrap() = if base.ends_with('/') {
                base
            } else {
                format!("{base}/")
            };
        }
        *self.collection_id.write().unwrap() = collection_id;
        // Keep the device row alive even when this is a fresh install.
        if store.get_opt(STORE_KEY_DEVICE).is_none() {
            store.set(STORE_KEY_DEVICE, &serde_json::to_string(&self.device)?)?;
        }
        Ok(())
    }

    async fn anonymous_login(&self, store: &crate::store::Store) -> anyhow::Result<Session> {
        let url = self.service_url("ottuser/action/anonymousLogin");
        let body = serde_json::json!({
            "partnerId": PARTNER_ID,
            "udid": self.device.udid,
            "language": "en",
        });
        let resp = self
            .http
            .post(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("vzy: anonymous login failed: HTTP {}", resp.status());
        }
        let login: LoginEnvelope = resp.json().await?;
        let result = login
            .result
            .ok_or_else(|| anyhow::anyhow!("vzy: anonymous login returned no session"))?;
        if result.ks.is_empty() {
            anyhow::bail!("vzy: anonymous login returned an empty KS");
        }
        let session = Session {
            ks: result.ks,
            expiry: result.expiry,
        };
        session.save(store)?;
        *self.session.write().unwrap() = Some(session.clone());
        Ok(session)
    }

    pub async fn ensure_session(&self, store: &crate::store::Store) -> anyhow::Result<Session> {
        if let Some(s) = self.session.read().unwrap().clone() {
            if s.is_fresh(SystemTime::now()) {
                return Ok(s);
            }
        }
        if let Some(s) = Session::load(store) {
            if s.is_fresh(SystemTime::now()) {
                *self.session.write().unwrap() = Some(s.clone());
                return Ok(s);
            }
        }
        self.anonymous_login(store).await
    }

    async fn post_ks<T: serde::de::DeserializeOwned>(
        &self,
        store: &crate::store::Store,
        path: &str,
        mut body: serde_json::Value,
    ) -> anyhow::Result<T> {
        for attempt in 0..2 {
            let session = self.ensure_session(store).await?;
            if let Some(obj) = body.as_object_mut() {
                obj.insert("ks".to_string(), serde_json::json!(session.ks));
                obj.insert("apiVersion".to_string(), serde_json::json!(API_VERSION));
            }
            let resp = self
                .http
                .post(self.service_url(path))
                .header(reqwest::header::USER_AGENT, USER_AGENT)
                .json(&body)
                .send()
                .await?;
            if matches!(resp.status().as_u16(), 401 | 403) && attempt == 0 {
                delete_session(store)?;
                *self.session.write().unwrap() = None;
                continue;
            }
            if !resp.status().is_success() {
                anyhow::bail!("vzy: {path} failed: HTTP {}", resp.status());
            }
            return Ok(resp.json::<T>().await?);
        }
        anyhow::bail!("vzy: {path} rejected the anonymous session")
    }

    /// One page of the `ChannelFilter` catalogue. `page_size` 200 covers the
    /// 161-channel live set; the caller pages if a tenant grows.
    pub async fn channels_page(
        &self,
        store: &crate::store::Store,
        page_index: i64,
        page_size: i64,
    ) -> anyhow::Result<(Vec<Asset>, i64)> {
        let collection_id = *self.collection_id.read().unwrap();
        let body = serde_json::json!({
            "filter": {
                "objectType": "KalturaChannelFilter",
                "idEqual": collection_id,
                "dynamicOrderBy": {
                    "objectType": "KalturaDynamicOrderBy",
                    "name": "ChannelNumber",
                    "orderBy": "META_ASC"
                }
            },
            "pager": {"pageSize": page_size, "pageIndex": page_index}
        });
        let env: AssetListEnvelope = self.post_ks(store, "asset/action/list", body).await?;
        let result = env
            .result
            .ok_or_else(|| anyhow::anyhow!("vzy: catalogue returned no result"))?;
        Ok((result.objects, result.total_count))
    }

    pub async fn playback(&self, store: &crate::store::Store, id: i64) -> anyhow::Result<Playback> {
        let body = serde_json::json!({
            "id": id,
            "assetReferenceType": "MEDIA"
        });
        let env: AssetEnvelope = self.post_ks(store, "asset/action/get", body).await?;
        let asset = env
            .result
            .ok_or_else(|| anyhow::anyhow!("vzy: asset {id} was not found"))?;
        if !asset.is_free() {
            anyhow::bail!("vzy: asset {id} is not in the free tier");
        }
        let mut hls = String::new();
        let mut dash = String::new();
        for file in &asset.media_files {
            if file.url.is_empty() {
                continue;
            }
            if hls.is_empty() && file.url.contains(".m3u8") {
                hls = file.url.clone();
            } else if dash.is_empty() && file.url.contains(".mpd") {
                dash = file.url.clone();
            }
        }
        if hls.is_empty() && dash.is_empty() {
            anyhow::bail!("vzy: asset {id} has no playable manifest");
        }
        Ok(Playback { hls, dash })
    }

    /// `epg/action/get/date/{timestamp}/slots/all`. `page_size` must be >= 100.
    pub async fn epg(
        &self,
        store: &crate::store::Store,
        channel_ids: &[i64],
        date_epoch: i64,
        offset_minutes: i32,
    ) -> anyhow::Result<HashMap<String, Vec<EpgProgram>>> {
        if channel_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let session = self.ensure_session(store).await?;
        let channels = channel_ids
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let url = format!(
            "{}service/epg/action/get/date/{date_epoch}/slots/all?channels={channels}&pageIndex=1&pageSize=100&offsetInMin={offset_minutes}",
            self.epg_base
        );
        let resp = self
            .http
            .get(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", session.ks),
            )
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("vzy: EPG failed: HTTP {}", resp.status());
        }
        Ok(resp.json::<EpgEnvelope>().await?.epg_chunk)
    }
}

pub fn to_television_channel(asset: &Asset) -> crate::television::Channel {
    crate::television::Channel {
        id: channel_id(&asset.id.to_string()),
        name: asset.name.clone(),
        logo_url: asset.logo(),
        ..Default::default()
    }
}

/// Midnight UTC epoch seconds, matching the app's `getCurrentTimeStamp`.
pub fn midnight_utc(now: SystemTime) -> i64 {
    let secs = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    secs - secs.rem_euclid(86_400)
}

/// Local offset in minutes for the EPG request, using the server's zone.
pub fn local_offset_minutes() -> i32 {
    time::UtcOffset::current_local_offset()
        .map(|o| i32::from(o.whole_minutes()))
        .unwrap_or(0)
}

pub fn parse_channel_id(channel_id: &str) -> Option<i64> {
    content_id(channel_id)?.parse().ok()
}

#[allow(dead_code)]
pub fn header_timeout() -> Duration {
    Duration::from_secs(30)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn free_asset() -> Asset {
        serde_json::from_value(serde_json::json!({
            "id": 6792351,
            "name": "TIMES NOW NAVBHARAT",
            "externalId": "vzy_32",
            "entryId": "TIMES NOW NAVBHARAT",
            "images": [
                {"imageTypeName": "1x1", "url": "https://img/1.png"},
                {"imageTypeName": "16x9", "url": "https://img/16.png"}
            ],
            "metas": {
                "EntitlementType": {"value": "Free"},
                "DRMType": {"value": "Nagra"},
                "ChannelNumber": {"value": 32.0}
            },
            "mediaFiles": [{"url": "https://cdn/live.m3u8"}]
        }))
        .unwrap()
    }

    #[test]
    fn ids_and_free_gate() {
        assert!(is_vzy_channel("vzy_1"));
        assert!(!is_vzy_channel("pb_1"));
        assert_eq!(channel_id("1"), "vzy_1");
        assert_eq!(parse_channel_id("vzy_1"), Some(1));
        let a = free_asset();
        assert!(a.is_free());
        assert_eq!(a.logo(), "https://img/16.png");
    }

    #[test]
    fn premium_asset_is_not_free() {
        let a: Asset = serde_json::from_value(serde_json::json!({
            "id": 1,
            "metas": {"EntitlementType": {"value": "Premium"}}
        }))
        .unwrap();
        assert!(!a.is_free());
    }

    #[tokio::test]
    async fn bootstrap_login_catalogue_and_playback_match_observed_contracts() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/configurations/action/serveByDevice"))
            .and(body_string_contains("\"partnerId\":5093"))
            .and(body_string_contains("\"platform\":\"STB\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "registered",
                "params": {"miniEPGCollectionId": "366800"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/ottuser/action/anonymousLogin"))
            .and(body_string_contains("\"partnerId\":5093"))
            .and(body_string_contains("ATV_"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"ks": "anon-ks", "expiry": 9999999999u64}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/asset/action/list"))
            .and(body_string_contains("KalturaChannelFilter"))
            .and(body_string_contains("\"idEqual\":366800"))
            .and(body_string_contains("anon-ks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"totalCount": 2, "objects": [
                    {"id": 1, "name": "Free One", "metas": {"EntitlementType": {"value": "Free"}}},
                    {"id": 2, "name": "Paid Two", "metas": {"EntitlementType": {"value": "Premium"}}}
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
        Mock::given(method("GET"))
            .and(path("/api_v3/service/epg/action/get/date/1793923200/slots/all"))
            .and(header("Authorization", "Bearer anon-ks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "epgChunk": {"1": [{"id": 9, "name": "Show", "startDate": 1793923200, "endDate": 1793926800, "linearAssetId": 1}]}
            })))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let mut client = Client::new(reqwest::Client::new(), Device::new());
        client.set_bases_for_test(&server.uri(), &format!("{}/api_v3", server.uri()));
        // The EPG base must therefore be `<server>/api_v3/`.
        client.epg_base = format!("{}/api_v3/", server.uri());

        client.bootstrap(&store).await.unwrap();
        assert_eq!(*client.collection_id.read().unwrap(), 366800);
        assert!(client.ensure_session(&store).await.unwrap().ks == "anon-ks");
        let (assets, total) = client.channels_page(&store, 1, 200).await.unwrap();
        assert_eq!(total, 2);
        assert_eq!(assets.len(), 2);
        let playback = client.playback(&store, 1).await.unwrap();
        assert_eq!(playback.hls, "https://cdn/free.m3u8");
        assert!(client.playback(&store, 2).await.is_err());
        let epg = client.epg(&store, &[1], 1793923200, 330).await.unwrap();
        assert_eq!(epg["1"][0].name, "Show");
    }

    #[test]
    fn midnight_utc_is_flooring() {
        let t = UNIX_EPOCH + Duration::from_secs(1793923200 + 1234);
        assert_eq!(midnight_utc(t), 1793923200);
    }
}
