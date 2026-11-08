//! Provision a remotely managed Cloudflare Tunnel with a scoped API token.
//! The API token stays in the process environment; only the connector token
//! is saved locally, outside the tracked repository.

use anyhow::{Context, Result, bail};
use reqwest::{Client, Method};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::Write;
use std::path::Path;
use std::time::Duration;

const API: &str = "https://api.cloudflare.com/client/v4";
const TOKEN_FILE: &str = "cloudflare-tunnel.token";
const ORIGIN_FILE: &str = "cloudflare-tunnel-origin";
const SAVED_TUNNEL_FILE: &str = "cloudflare-tunnel.json";

#[derive(Deserialize, Serialize)]
struct SavedTunnel {
    token: String,
    origin: String,
    #[serde(default)]
    port: Option<u16>,
}

#[derive(Deserialize)]
struct Envelope<T> {
    success: bool,
    result: T,
}

#[derive(Deserialize)]
struct Zone {
    id: String,
    name: String,
    status: String,
    account: Account,
}

#[derive(Deserialize)]
struct Account {
    id: String,
}

#[derive(Deserialize)]
struct Tunnel {
    id: String,
}

fn hostname(domain: &str, subdomain: &str) -> Result<(String, String)> {
    let domain = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    let subdomain = subdomain.trim().to_ascii_lowercase();
    let label_valid = |label: &str| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    };
    if domain.len() > 253 || domain.split('.').count() < 2 || !domain.split('.').all(label_valid) {
        bail!("invalid domain name; enter a domain managed in Cloudflare (example.com)");
    }
    // A single label works with Cloudflare's ordinary wildcard certificate.
    if !label_valid(&subdomain) {
        bail!("subdomain must be a single DNS label, e.g. 'tv'");
    }
    Ok((domain.clone(), format!("{subdomain}.{domain}")))
}

async fn request<T: DeserializeOwned>(
    client: &Client,
    base: &str,
    api_token: &str,
    method: Method,
    route: &str,
    body: Option<Value>,
    step: &str,
) -> Result<T> {
    let mut req = client
        .request(method, format!("{base}{route}"))
        .bearer_auth(api_token);
    if let Some(body) = body {
        req = req.json(&body);
    }
    let response = req
        .send()
        .await
        .with_context(|| format!("{step}: network request failed"))?;
    let status = response.status();
    // Avoid returning the raw API response: it can contain connector credentials.
    if !status.is_success() {
        bail!("{step}: Cloudflare returned HTTP {status}; check the API token's permissions");
    }
    let envelope: Envelope<T> = response
        .json()
        .await
        .with_context(|| format!("{step}: invalid Cloudflare response"))?;
    if !envelope.success {
        bail!("{step}: Cloudflare rejected the operation; check the API token's permissions");
    }
    Ok(envelope.result)
}

fn token_path(path_prefix: &str) -> std::path::PathBuf {
    Path::new(path_prefix).join(TOKEN_FILE)
}

fn origin_path(path_prefix: &str) -> std::path::PathBuf {
    Path::new(path_prefix).join(ORIGIN_FILE)
}

fn saved_tunnel_path(path_prefix: &str) -> std::path::PathBuf {
    Path::new(path_prefix).join(SAVED_TUNNEL_FILE)
}

fn load_saved_tunnel(path_prefix: &str) -> Result<Option<SavedTunnel>> {
    let path = saved_tunnel_path(path_prefix);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e).with_context(|| format!("reading saved tunnel at {}", path.display()));
        }
    };
    let saved: SavedTunnel = serde_json::from_str(&raw)
        .with_context(|| format!("parsing saved tunnel at {}", path.display()))?;
    if saved.port.is_none() {
        bail!(
            "saved tunnel at {} has no configured port; run tunnel setup again",
            path.display()
        );
    }
    if saved.port == Some(0) {
        bail!("saved tunnel at {} has invalid port 0", path.display());
    }
    if !saved.token.starts_with("eyJ") {
        bail!("saved connector token is invalid");
    }
    crate::public_origin::validate(&saved.origin)?;
    Ok(Some(saved))
}

pub fn load_origin(path_prefix: &str) -> Result<Option<String>> {
    if let Some(saved) = load_saved_tunnel(path_prefix)? {
        return Ok(Some(saved.origin));
    }
    // Existing development installations stored origin and token separately.
    let path = origin_path(path_prefix);
    match std::fs::read_to_string(&path) {
        Ok(origin) => Ok(Some(crate::public_origin::validate(origin.trim())?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => {
            Err(e).with_context(|| format!("reading saved tunnel origin at {}", path.display()))
        }
    }
}

pub fn load_token(path_prefix: &str) -> Result<Option<String>> {
    if let Some(saved) = load_saved_tunnel(path_prefix)? {
        return Ok(Some(saved.token));
    }
    let path = token_path(path_prefix);
    let raw = match std::fs::read_to_string(&path) {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e).with_context(|| format!("reading saved tunnel at {}", path.display()));
        }
    };
    let token = raw.trim();
    if token.is_empty() {
        bail!("saved tunnel token at {} is empty", path.display());
    }
    Ok(Some(token.to_owned()))
}

pub fn load_port(path_prefix: &str) -> Result<Option<u16>> {
    Ok(load_saved_tunnel(path_prefix)?.map(|saved| saved.port.expect("validated saved port")))
}

fn save_tunnel(path_prefix: &str, token: &str, origin: &str, port: u16) -> Result<()> {
    if port == 0 {
        bail!("tunnel port must be greater than 0");
    }
    if !token.starts_with("eyJ") {
        bail!("Cloudflare returned an invalid connector token");
    }
    let origin = crate::public_origin::validate(origin)?;
    let path = saved_tunnel_path(path_prefix);
    let mut tmp = tempfile::NamedTempFile::new_in(path_prefix)?;
    serde_json::to_writer(
        &mut tmp,
        &SavedTunnel {
            token: token.to_string(),
            origin,
            port: Some(port),
        },
    )?;
    tmp.flush()?;
    tmp.as_file().sync_all()?;
    tmp.persist(&path)
        .with_context(|| format!("saving tunnel configuration at {}", path.display()))?;
    Ok(())
}

pub async fn setup(
    api_token: &str,
    domain: &str,
    subdomain: &str,
    port: u16,
    path_prefix: &str,
) -> Result<()> {
    let client = Client::builder().timeout(Duration::from_secs(25)).build()?;
    let url = provision(
        &client,
        API,
        api_token,
        domain,
        subdomain,
        port,
        path_prefix,
    )
    .await?;
    println!("Cloudflare Tunnel ready: {url}");
    println!(
        "Connector token saved privately in {}",
        saved_tunnel_path(path_prefix).display()
    );
    println!("Start with: jiotv serve --host 127.0.0.1 --port {port} --tunnel --tunnel-saved");
    Ok(())
}

async fn provision(
    client: &Client,
    base: &str,
    api_token: &str,
    domain: &str,
    subdomain: &str,
    port: u16,
    path_prefix: &str,
) -> Result<String> {
    if port == 0 {
        bail!("tunnel port must be greater than 0");
    }
    if api_token.trim().is_empty() {
        bail!("Cloudflare API token is empty");
    }
    let (domain, hostname) = hostname(domain, subdomain)?;
    let zones: Vec<Zone> = request(
        client,
        base,
        api_token,
        Method::GET,
        &format!("/zones?name={}&per_page=50", urlencoding::encode(&domain)),
        None,
        "finding domain",
    )
    .await?;
    let zone = zones
        .iter()
        .find(|z| z.name.eq_ignore_ascii_case(&domain))
        .ok_or_else(|| {
            anyhow::anyhow!("domain {domain} not found; API token needs Zone Read permission")
        })?;
    if zone.status != "active" {
        bail!("Cloudflare zone {domain} is not active");
    }
    let records: Vec<Value> = request(
        client,
        base,
        api_token,
        Method::GET,
        &format!(
            "/zones/{}/dns_records?name={}",
            zone.id,
            urlencoding::encode(&hostname)
        ),
        None,
        "checking DNS",
    )
    .await?;
    if !records.is_empty() {
        bail!("DNS record {hostname} already exists; choose another subdomain");
    }

    let tunnel: Tunnel = request(
        client,
        base,
        api_token,
        Method::POST,
        &format!("/accounts/{}/cfd_tunnel", zone.account.id),
        Some(json!({"name": format!("jiotv-{subdomain}"), "config_src": "cloudflare"})),
        "creating tunnel",
    )
    .await?;
    let tunnel_route = format!("/accounts/{}/cfd_tunnel/{}", zone.account.id, tunnel.id);
    // A failed step after creation leaves a tunnel in the account. Return its
    // identifier so the user can inspect/remove it in Cloudflare's dashboard.
    let finish = async {
        let _: Value = request(
            client,
            base,
            api_token,
            Method::PUT,
            &format!("{tunnel_route}/configurations"),
            Some(json!({"config": {"ingress": [
                {"hostname": hostname, "service": format!("http://127.0.0.1:{port}")},
                {"service": "http_status:404"}
            ]}})),
            "setting ingress",
        )
        .await?;
        let token: String = request(
            client,
            base,
            api_token,
            Method::GET,
            &format!("{tunnel_route}/token"),
            None,
            "retrieving connector token",
        )
        .await?;
        if !token.starts_with("eyJ") {
            bail!("Cloudflare returned an invalid connector token");
        }
        let _: Value = request(
            client,
            base,
            api_token,
            Method::POST,
            &format!("/zones/{}/dns_records", zone.id),
            Some(json!({"type": "CNAME", "name": hostname,
                "content": format!("{}.cfargotunnel.com", tunnel.id), "proxied": true})),
            "creating DNS record",
        )
        .await?;
        save_tunnel(path_prefix, &token, &format!("https://{hostname}"), port)
            .context("DNS configured but saving tunnel configuration failed")?;
        Ok::<(), anyhow::Error>(())
    }
    .await;
    finish.with_context(|| {
        format!(
            "Cloudflare tunnel {} was created; inspect it in the dashboard before retrying",
            tunnel.id
        )
    })?;
    Ok(format!("https://{hostname}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path, query_param},
    };

    fn ok(result: Value) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(json!({"success": true, "result": result}))
    }

    #[tokio::test]
    async fn creates_configured_tunnel_and_persists_secret() {
        let api = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        Mock::given(method("GET")).and(path("/zones")).and(query_param("name", "example.com"))
            .respond_with(ok(json!([{"id": "zone123", "name": "example.com", "status": "active", "account": {"id": "account123"}}])))
            .mount(&api).await;
        Mock::given(method("GET"))
            .and(path("/zones/zone123/dns_records"))
            .respond_with(ok(json!([])))
            .mount(&api)
            .await;
        Mock::given(method("POST"))
            .and(path("/accounts/account123/cfd_tunnel"))
            .respond_with(ok(json!({"id": "uuid123"})))
            .mount(&api)
            .await;
        Mock::given(method("PUT"))
            .and(path(
                "/accounts/account123/cfd_tunnel/uuid123/configurations",
            ))
            .respond_with(ok(json!({})))
            .mount(&api)
            .await;
        Mock::given(method("GET"))
            .and(path("/accounts/account123/cfd_tunnel/uuid123/token"))
            .respond_with(ok(json!("eyJexample-connector-token")))
            .mount(&api)
            .await;
        Mock::given(method("POST"))
            .and(path("/zones/zone123/dns_records"))
            .respond_with(ok(json!({"id": "record123"})))
            .mount(&api)
            .await;
        let url = provision(
            &Client::new(),
            &api.uri(),
            "scoped-secret",
            "EXAMPLE.COM",
            "tv",
            5001,
            dir.path().to_str().unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(url, "https://tv.example.com");
        assert_eq!(
            load_origin(dir.path().to_str().unwrap())
                .unwrap()
                .as_deref(),
            Some("https://tv.example.com")
        );
        assert_eq!(
            load_token(dir.path().to_str().unwrap()).unwrap().as_deref(),
            Some("eyJexample-connector-token")
        );
        assert_eq!(load_port(dir.path().to_str().unwrap()).unwrap(), Some(5001));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(saved_tunnel_path(dir.path().to_str().unwrap()))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        let received = api.received_requests().await.unwrap();
        assert!(received.iter().all(
            |r| r.headers.get("authorization").map(|v| v.to_str().unwrap())
                == Some("Bearer scoped-secret")
        ));
        assert!(
            received
                .iter()
                .any(|r| r.url.path().ends_with("/configurations")
                    && String::from_utf8_lossy(&r.body).contains("http://127.0.0.1:5001"))
        );
        assert!(
            received
                .iter()
                .any(|r| r.url.path().ends_with("/dns_records")
                    && r.method == "POST"
                    && String::from_utf8_lossy(&r.body).contains("uuid123.cfargotunnel.com"))
        );
    }

    #[tokio::test]
    async fn existing_record_blocks_creation() {
        let api = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        Mock::given(method("GET")).and(path("/zones"))
            .respond_with(ok(json!([{"id": "zone", "name": "example.com", "status": "active", "account": {"id": "account"}}])))
            .mount(&api).await;
        Mock::given(method("GET"))
            .and(path("/zones/zone/dns_records"))
            .respond_with(ok(json!([{"type": "A"}])))
            .mount(&api)
            .await;
        assert!(
            provision(
                &Client::new(),
                &api.uri(),
                "secret",
                "example.com",
                "tv",
                5001,
                dir.path().to_str().unwrap()
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("already exists")
        );
        assert!(
            api.received_requests()
                .await
                .unwrap()
                .iter()
                .all(|r| r.method == "GET")
        );
        assert!(load_token(dir.path().to_str().unwrap()).unwrap().is_none());
        assert!(load_origin(dir.path().to_str().unwrap()).unwrap().is_none());
    }

    #[test]
    fn rejects_deep_or_invalid_labels() {
        assert!(hostname("example.com", "deep.tv").is_err());
        assert!(hostname("https://example.com", "tv").is_err());
        assert!(hostname("example.com", "-tv").is_err());
    }

    #[test]
    fn invalid_saved_tunnel_update_preserves_previous_token_and_origin() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();
        save_tunnel(path, "eyJoriginal", "https://one.example.com", 5001).unwrap();
        assert!(save_tunnel(path, "eyJupdated", "http://two.example.com", 5002).is_err());
        assert_eq!(load_token(path).unwrap().as_deref(), Some("eyJoriginal"));
        assert_eq!(
            load_origin(path).unwrap().as_deref(),
            Some("https://one.example.com")
        );
        save_tunnel(path, "eyJupdated", "https://two.example.com", 5002).unwrap();
        assert_eq!(load_token(path).unwrap().as_deref(), Some("eyJupdated"));
        assert_eq!(
            load_origin(path).unwrap().as_deref(),
            Some("https://two.example.com")
        );
        assert_eq!(load_port(path).unwrap(), Some(5002));
    }

    #[test]
    fn legacy_saved_tunnel_without_port_has_clear_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            saved_tunnel_path(dir.path().to_str().unwrap()),
            r#"{"token":"eyJlegacy","origin":"https://tv.example.com"}"#,
        )
        .unwrap();
        let error = load_port(dir.path().to_str().unwrap())
            .unwrap_err()
            .to_string();
        assert!(error.contains("no configured port"));
        assert!(error.contains("run tunnel setup again"));
    }

    #[tokio::test]
    async fn zero_port_is_rejected_before_api_calls() {
        let api = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let error = provision(
            &Client::new(),
            &api.uri(),
            "secret",
            "example.com",
            "tv",
            0,
            dir.path().to_str().unwrap(),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("port must be greater than 0"));
        assert!(api.received_requests().await.unwrap().is_empty());
    }
}
