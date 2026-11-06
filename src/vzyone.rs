//! `com.vzy.one` file identity record: the APK is actually Uptodown.
//!
//! Evidence: `docs/ott-providers/vzyone-uptodown.md`. Uptodown is an app
//! store, not a TV service: no live catalogue, EPG, VOD, playback, or DRM
//! strings exist in the artifact. This module therefore exposes identity +
//! status only, and every media operation returns an explicit
//! `unsupported` error instead of forcing feature parity.
//!
//! The `ud_` prefix is reserved so future IDs cannot collide with `ex_`,
//! `pb_`, `tp_`, or numeric JioTV IDs.

#![allow(dead_code)] // Evidence constants + runtime-gap hooks exercised by tests.
use std::sync::RwLock;

pub const ID_PREFIX: &str = "ud_";
pub const FILE_NAME: &str = "com.vzy.one.apk";
pub const ACTUAL_PACKAGE: &str = "com.uptodown";
pub const VERSION_NAME: &str = "7.44";
pub const VERSION_CODE: u32 = 744;
pub const ARTIFACT_SHA256: &str =
    "8f8a7b2416d84d64d418dc4e194cd8ab6f4394d51651bf2e1aac47207503fd92";
pub const PRIMARY_BASE: &str = "https://api.uptodown.app:443";

pub const STORE_KEY_DEVICE: &str = "vzyone_device";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Device {
    #[serde(default)]
    pub android_id: String,
}

impl Device {
    pub fn load_or_create(store: &crate::store::Store) -> anyhow::Result<Device> {
        if let Some(json) = store.get_opt(STORE_KEY_DEVICE) {
            if let Ok(d) = serde_json::from_str::<Device>(&json) {
                if !d.android_id.is_empty() {
                    return Ok(d);
                }
            }
        }
        let mut bytes = [0u8; 8];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
        let d = Device {
            android_id: hex::encode(bytes),
        };
        store.set(STORE_KEY_DEVICE, &serde_json::to_string(&d)?)?;
        Ok(d)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VzyoneError {
    #[error(
        "vzyone: unsupported — the analyzed artifact is the Uptodown app store, not a TV service"
    )]
    Unsupported,
}

pub fn is_vzyone_channel(id: &str) -> bool {
    id.starts_with(ID_PREFIX)
}

pub struct VzyoneState {
    config_enabled: bool,
    device: RwLock<Option<Device>>,
}

impl VzyoneState {
    pub fn new(config_enabled: bool) -> VzyoneState {
        VzyoneState {
            config_enabled,
            device: RwLock::new(None),
        }
    }

    pub fn enabled(&self) -> bool {
        self.config_enabled
    }

    /// Uptodown has no TV login, so "connected" is always false. The state
    /// still loads a stable device ID so status can report presence.
    pub fn connected(&self) -> bool {
        false
    }

    pub fn init(&self, store: &crate::store::Store) {
        if !self.config_enabled {
            return;
        }
        if let Ok(d) = Device::load_or_create(store) {
            *self.device.write().unwrap() = Some(d);
        }
    }

    pub fn logout(&self, store: &crate::store::Store) -> anyhow::Result<()> {
        *self.device.write().unwrap() = None;
        let _ = store.delete(STORE_KEY_DEVICE);
        Ok(())
    }

    pub fn status(&self) -> serde_json::Value {
        serde_json::json!({
            "enabled": self.config_enabled,
            "connected": false,
            "package": ACTUAL_PACKAGE,
            "fileName": FILE_NAME,
            "version": VERSION_NAME,
            "versionCode": VERSION_CODE,
            "note": "app-store artifact, no live/EPG/VOD support",
        })
    }
}

pub fn unsupported<T>() -> anyhow::Result<T> {
    Err(VzyoneError::Unsupported.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_constants_match_evidence() {
        assert_eq!(ACTUAL_PACKAGE, "com.uptodown");
        assert_eq!(VERSION_NAME, "7.44");
        assert_eq!(VERSION_CODE, 744);
        assert!(is_vzyone_channel("ud_x"));
        assert!(!is_vzyone_channel("ex_x"));
    }

    #[test]
    fn status_is_always_disconnected() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().to_str().unwrap()).unwrap();
        let s = VzyoneState::new(true);
        s.init(&store);
        assert!(!s.connected());
        let v = s.status();
        assert_eq!(v["connected"], false);
        assert_eq!(v["package"], "com.uptodown");
        s.logout(&store).unwrap();
    }

    #[test]
    fn media_operations_are_unsupported() {
        assert!(unsupported::<()>().is_err());
    }
}
