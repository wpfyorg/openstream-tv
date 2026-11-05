//! Public URL selection for HTTPS-terminating proxies and Cloudflare tunnels.
//! Only an explicitly configured origin or one obtained from the local tunnel
//! lifecycle may override the listener scheme. Forwarded headers are ignored.

use anyhow::{bail, Result};
use axum::http::{header, uri::Authority, HeaderMap};
use url::Url;

/// Accept only a complete HTTPS origin, without credentials or URL components
/// that could redirect playlist/DRM requests outside the expected server.
pub fn validate(raw: &str) -> Result<String> {
    if raw.trim() != raw {
        bail!("public_origin must not contain surrounding whitespace");
    }
    let url = Url::parse(raw)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!(
            "public_origin must be an HTTPS origin (e.g. https://tv.example.com), without a path, query, fragment, or credentials"
        );
    }
    Ok(url.origin().ascii_serialization())
}

/// Select the external HTTPS origin only for an exact public Host match.
/// Internal/LAN requests retain the actual connection's HTTP or HTTPS scheme.
pub fn for_request(tls: bool, headers: &HeaderMap, public_origin: Option<&str>) -> String {
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .and_then(safe_authority);

    if let (Some(host), Some(origin)) = (host.as_ref(), public_origin) {
        // Validate at config load and tunnel discovery; parse once more here to
        // compare origins (including non-default ports) without string tricks.
        if let (Ok(external), Ok(requested)) =
            (Url::parse(origin), Url::parse(&format!("https://{host}")))
        {
            if requested.origin() == external.origin() {
                return external.origin().ascii_serialization();
            }
        }
    }

    let scheme = if tls { "https" } else { "http" };
    format!("{scheme}://{}", host.as_deref().unwrap_or("localhost"))
}

fn safe_authority(raw: &str) -> Option<String> {
    if raw.is_empty()
        || raw.bytes().any(|b| {
            b.is_ascii_whitespace()
                || b.is_ascii_control()
                || matches!(b, b'/' | b'\\' | b'@' | b'?' | b'#' | b'%')
        })
    {
        return None;
    }
    let authority: Authority = raw.parse().ok()?;
    let url = Url::parse(&format!("https://{authority}")).ok()?;
    if url.host_str().is_none() || url.path() != "/" || url.query().is_some() {
        return None;
    }
    Some(authority.as_str().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(host: &str, forwarded_proto: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, host.parse().unwrap());
        headers.insert("x-forwarded-proto", forwarded_proto.parse().unwrap());
        headers.insert(
            "forwarded",
            "proto=https;host=evil.example".parse().unwrap(),
        );
        headers
    }

    #[test]
    fn rejects_untrusted_origins() {
        for value in [
            "http://tv.example.com",
            "https://user:pass@tv.example.com",
            "https://tv.example.com/subdir",
            "https://tv.example.com/?x=y",
            "https://tv.example.com/#fragment",
            " https://tv.example.com",
            "https://tv.example.com ",
            "https://",
        ] {
            assert!(validate(value).is_err(), "accepted {value}");
        }
        assert_eq!(
            validate("https://TV.EXAMPLE.COM:443/").unwrap(),
            "https://tv.example.com"
        );
        assert_eq!(
            validate("https://tv.example.com:8443").unwrap(),
            "https://tv.example.com:8443"
        );
    }

    #[test]
    fn cloudflare_host_uses_https_but_lan_stays_http() {
        let origin = Some("https://tv.example.com");
        assert_eq!(
            for_request(false, &request("tv.example.com", "http"), origin),
            "https://tv.example.com"
        );
        assert_eq!(
            for_request(false, &request("TV.EXAMPLE.COM:443", "http"), origin),
            "https://tv.example.com"
        );
        assert_eq!(
            for_request(false, &request("192.168.1.10:5001", "https"), origin),
            "http://192.168.1.10:5001"
        );
        assert_eq!(
            for_request(true, &request("lan:5443", "http"), origin),
            "https://lan:5443"
        );
    }

    #[test]
    fn untrusted_headers_cannot_select_https() {
        assert_eq!(
            for_request(false, &request("lan:5001", "https"), None),
            "http://lan:5001"
        );
        assert_eq!(
            for_request(
                false,
                &request("lan:5001", "https"),
                Some("https://tv.example.com")
            ),
            "http://lan:5001"
        );
        assert_eq!(
            for_request(
                false,
                &request("tv.example.com:8443", "https"),
                Some("https://tv.example.com:8443")
            ),
            "https://tv.example.com:8443"
        );
        for bad in [
            "evil.example/path",
            "evil.example@tv.example.com",
            "bad%0d.example",
            "evil.example?x=y",
        ] {
            assert_eq!(
                for_request(false, &request(bad, "https"), None),
                "http://localhost"
            );
        }
    }
}
