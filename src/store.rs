//! A tiny TOML key/value store, file-compatible with the Go version's
//! `store_v4.toml` (`{ data = { k = "v", ... } }`), so an existing store
//! keeps working when a router is moved from the Go binary to this one.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Default, Serialize, Deserialize)]
struct FileFormat {
    #[serde(default)]
    data: BTreeMap<String, String>,
}

pub struct Store {
    path: PathBuf,
    inner: Mutex<FileFormat>,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Returned by `get` (the non-`Option` companion to `get_opt`, which
    /// every current caller uses instead); kept as public API.
    #[allow(dead_code)]
    #[error("key not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Decode(#[from] toml::de::Error),
    #[error(transparent)]
    Encode(#[from] toml::ser::Error),
}

impl Store {
    pub fn open(path_prefix: &str) -> Result<Store, StoreError> {
        std::fs::create_dir_all(path_prefix)?;
        let path = PathBuf::from(path_prefix).join("store_v4.toml");
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;
        file.lock()?;

        let mut text = String::new();
        file.read_to_string(&mut text)?;
        let inner = if text.trim().is_empty() {
            let current = FileFormat::default();
            let encoded = toml::to_string(&current)?;
            file.seek(SeekFrom::Start(0))?;
            file.write_all(encoded.as_bytes())?;
            current
        } else {
            toml::from_str(&text)?
        };

        Ok(Store {
            path,
            inner: Mutex::new(inner),
        })
    }

    fn update<T>(&self, apply: impl FnOnce(&mut FileFormat) -> T) -> Result<T, StoreError> {
        let mut guard = self.inner.lock().unwrap();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)?;
        file.lock()?;

        let mut text = String::new();
        file.read_to_string(&mut text)?;
        let mut current = if text.trim().is_empty() {
            FileFormat::default()
        } else {
            toml::from_str(&text)?
        };
        let result = apply(&mut current);

        let encoded = toml::to_string(&current)?;
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(encoded.as_bytes())?;
        *guard = current;
        Ok(result)
    }

    #[allow(dead_code)]
    pub fn get(&self, key: &str) -> Result<String, StoreError> {
        let guard = self.inner.lock().unwrap();
        guard
            .data
            .get(key)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(key.to_string()))
    }

    pub fn get_opt(&self, key: &str) -> Option<String> {
        self.inner.lock().unwrap().data.get(key).cloned()
    }

    pub fn get_or_insert(&self, key: &str, value: &str) -> Result<String, StoreError> {
        let mut guard = self.inner.lock().unwrap();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)?;
        file.lock()?;

        let mut text = String::new();
        file.read_to_string(&mut text)?;
        let mut current = if text.trim().is_empty() {
            FileFormat::default()
        } else {
            toml::from_str(&text)?
        };
        if let Some(existing) = current
            .data
            .get(key)
            .filter(|existing| !existing.is_empty())
            .cloned()
        {
            *guard = current;
            return Ok(existing);
        }

        current.data.insert(key.to_string(), value.to_string());
        let encoded = toml::to_string(&current)?;
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(encoded.as_bytes())?;
        *guard = current;
        Ok(value.to_string())
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), StoreError> {
        self.update(|current| {
            current.data.insert(key.to_string(), value.to_string());
        })
    }

    pub fn delete(&self, key: &str) -> Result<(), StoreError> {
        self.update(|current| {
            current.data.remove(key);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_values() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().to_str().unwrap()).unwrap();
        store.set("access_key", "abc123").unwrap();
        assert_eq!(store.get("access_key").unwrap(), "abc123");
        store.delete("access_key").unwrap();
        assert!(store.get("access_key").is_err());
    }

    #[test]
    fn reads_go_format_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store_v4.toml");
        std::fs::write(&path, "[data]\naccess_key = \"deadbeef\"\n").unwrap();
        let store = Store::open(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(store.get("access_key").unwrap(), "deadbeef");
    }

    #[test]
    fn persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        {
            let store = Store::open(dir.path().to_str().unwrap()).unwrap();
            store.set("k", "v").unwrap();
        }
        let store = Store::open(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(store.get("k").unwrap(), "v");
    }

    #[test]
    fn stale_store_write_preserves_newer_external_value() {
        let dir = tempfile::tempdir().unwrap();
        let prefix = dir.path().to_str().unwrap();
        let service = Store::open(prefix).unwrap();
        let cli = Store::open(prefix).unwrap();

        cli.set("epg_enabled", "true").unwrap();
        service.set("accessToken", "refreshed").unwrap();

        let reopened = Store::open(prefix).unwrap();
        assert_eq!(reopened.get_opt("epg_enabled").as_deref(), Some("true"));
        assert_eq!(
            reopened.get_opt("accessToken").as_deref(),
            Some("refreshed")
        );
    }

    #[test]
    fn get_or_insert_reuses_value_created_by_stale_peer() {
        let dir = tempfile::tempdir().unwrap();
        let prefix = dir.path().to_str().unwrap();
        let first = Store::open(prefix).unwrap();
        let stale = Store::open(prefix).unwrap();

        assert_eq!(first.get_or_insert("access_key", "first").unwrap(), "first");
        assert_eq!(
            stale.get_or_insert("access_key", "second").unwrap(),
            "first"
        );

        let reopened = Store::open(prefix).unwrap();
        assert_eq!(reopened.get_opt("access_key").as_deref(), Some("first"));
    }

    #[test]
    fn get_or_insert_existing_value_does_not_rewrite_file() {
        let dir = tempfile::tempdir().unwrap();
        let prefix = dir.path().to_str().unwrap();
        let path = dir.path().join("store_v4.toml");
        let original = "# keep this formatting\n[data]\naccess_key = \"existing\"\n";
        std::fs::write(&path, original).unwrap();
        let store = Store::open(prefix).unwrap();

        assert_eq!(
            store.get_or_insert("access_key", "replacement").unwrap(),
            "existing"
        );
        assert_eq!(std::fs::read_to_string(path).unwrap(), original);
    }
}
