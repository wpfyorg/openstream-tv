//! Tata Play (`com.ryzmedia.tatasky`, 509.0/894) — identity record only.
//!
//! Evidence: `docs/ott-providers/tataplay.md`. Static analysis recovered the
//! package identity, base URLs and header names, but **no** login, catalogue,
//! EPG or playback contract: the React Native bundle is AES-encrypted
//! (`index.android.bundle`, entropy ≈ 7.997, no Hermes magic) and the app's
//! network layer lives entirely in that bundle. Guessed paths such as
//! `content/playback/resolve` must never be called, so this module exposes a
//! status record that names every unimplemented feature and the exact gap.
//!
//! A later runtime capture (mitmproxy against the real client, or decrypting
//! the CodePush-delivered bundle) is required before any request can be added.

pub const PACKAGE: &str = "com.ryzmedia.tatasky";
pub const VERSION_NAME: &str = "509.0";
pub const VERSION_CODE: u32 = 894;
pub const ARTIFACT_SHA256: &str =
    "ab9091ab2a3bb42eeb2be2b6c6e59cc799d17b6af852618c13d433b253b90e14";

/// Static evidence: no content contract is recoverable from the shipped
/// artifact.
pub const GAP: &str = "Tata Play login, catalogue, EPG, playback and Widevine \
contracts are not recoverable statically: index.android.bundle is encrypted \
(no Hermes magic) and the app's network layer exists only inside that bundle. \
A runtime capture with an authorised account is required.";

/// The status record exposed at `/api/tataplay/status`. It intentionally
/// reports no connection and no channels.
pub fn status() -> serde_json::Value {
    serde_json::json!({
        "package": PACKAGE,
        "version": VERSION_NAME,
        "versionCode": VERSION_CODE,
        "artifactId": ARTIFACT_SHA256,
        "available": false,
        "implemented": {
            "login": false,
            "catalogue": false,
            "epg": false,
            "playback": false,
            "license": false,
            "subscription": false
        },
        "gap": GAP
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_constants_match_the_artifact() {
        assert_eq!(PACKAGE, "com.ryzmedia.tatasky");
        assert_eq!(VERSION_NAME, "509.0");
        assert_eq!(VERSION_CODE, 894);
        assert!(!ARTIFACT_SHA256.is_empty());
    }

    #[test]
    fn status_reports_every_feature_as_unavailable() {
        let s = status();
        assert_eq!(s["available"], false);
        for feature in [
            "login",
            "catalogue",
            "epg",
            "playback",
            "license",
            "subscription",
        ] {
            assert_eq!(s["implemented"][feature], false, "{feature}");
        }
        assert!(s["gap"].as_str().unwrap().contains("encrypted"));
    }
}
