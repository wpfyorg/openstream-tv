//! `com.vzy.one.apk` — identity record for a mislabelled artifact.
//!
//! Evidence: `docs/ott-providers/vzyone-uptodown.md`. The file named
//! `com.vzy.one.apk` contains package `com.uptodown` (Uptodown App Store
//! 7.44), with no live TV, EPG, VOD, playback or DRM surface. It is an app
//! store, not a media service.
//!
//! There is no runtime configuration, no persisted device state and no login
//! to expose. A single status record keeps the artifact provenance visible
//! without pretending a provider exists.

pub const FILE_NAME: &str = "com.vzy.one.apk";
pub const ACTUAL_PACKAGE: &str = "com.uptodown";
pub const VERSION_NAME: &str = "7.44";
pub const VERSION_CODE: u32 = 744;
pub const ARTIFACT_SHA256: &str =
    "8f8a7b2416d84d64d418dc4e194cd8ab6f4394d51651bf2e1aac47207503fd92";

pub const REASON: &str = "the supplied com.vzy.one.apk is Uptodown App Store \
(com.uptodown 7.44): an app-distribution client with no live/EPG/VOD/DRM surface";

pub fn status() -> serde_json::Value {
    serde_json::json!({
        "fileName": FILE_NAME,
        "package": ACTUAL_PACKAGE,
        "version": VERSION_NAME,
        "versionCode": VERSION_CODE,
        "artifactId": ARTIFACT_SHA256,
        "provider": false,
        "reason": REASON
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_identifies_the_real_artifact() {
        let s = status();
        assert_eq!(s["package"], "com.uptodown");
        assert_eq!(s["version"], "7.44");
        assert_eq!(s["provider"], false);
        assert!(s["reason"].as_str().unwrap().contains("Uptodown"));
    }
}
