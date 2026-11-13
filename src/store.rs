//! A tiny TOML key/value store, file-compatible with the Go version's
//! `store_v4.toml` (`{ data = { k = "v", ... } }`), so an existing store
//! keeps working when a router is moved from the Go binary to this one.
//!
//! Mutations replace the file atomically: the new document is written to a
//! temporary file in the same directory and renamed over the old one, instead
//! of truncating the destination in place. A crash, a power cut or a full
//! filesystem therefore leaves either the previous store or the new one, never
//! a half-written file that would lose the stored credentials.
//!
//! Every writer serialises on a sidecar lock file (`store_v4.lock`) so the
//! CLI and a running server still see each other's keys. Locking the data file
//! itself is not enough once writes replace it: the lock would be held on the
//! old file identity while the next writer opens the new one.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

const STORE_FILE: &str = "store_v4.toml";
const LOCK_FILE: &str = "store_v4.lock";

#[derive(Debug, Default, Serialize, Deserialize)]
struct FileFormat {
    #[serde(default)]
    data: BTreeMap<String, String>,
}

pub struct Store {
    path: PathBuf,
    lock_path: PathBuf,
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
        let store = Store {
            path: PathBuf::from(path_prefix).join(STORE_FILE),
            lock_path: PathBuf::from(path_prefix).join(LOCK_FILE),
            inner: Mutex::new(FileFormat::default()),
        };
        {
            let mut guard = store.inner.lock().unwrap();
            let _lock = store.lock()?;
            let (current, needs_create) = store.read_from_disk()?;
            if needs_create {
                store.write_atomic(&current)?;
            }
            *guard = current;
        }
        Ok(store)
    }

    /// Takes the sidecar lock that every store mutation goes through. The lock
    /// is released when the returned handle drops.
    fn lock(&self) -> Result<File, StoreError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.lock_path)?;
        file.lock()?;
        Ok(file)
    }

    /// Reads the authoritative on-disk state. `needs_create` is true when the
    /// file is missing or empty, which the caller resolves by writing the
    /// default document.
    fn read_from_disk(&self) -> Result<(FileFormat, bool), StoreError> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) if !text.trim().is_empty() => Ok((toml::from_str(&text)?, false)),
            Ok(_) => Ok((FileFormat::default(), true)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok((FileFormat::default(), true))
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Re-reads the file under the lock, applies `apply`, and writes the result
    /// back only when something changed. The in-memory snapshot is published
    /// after a successful write, so a failed write leaves memory and disk on
    /// the same (previous) state.
    fn mutate<T>(&self, apply: impl FnOnce(&mut FileFormat) -> T) -> Result<T, StoreError> {
        let mut guard = self.inner.lock().unwrap();
        let _lock = self.lock()?;
        let (mut current, needs_create) = self.read_from_disk()?;
        let previous = current.data.clone();
        let result = apply(&mut current);
        if needs_create || current.data != previous {
            self.write_atomic(&current)?;
        }
        *guard = current;
        Ok(result)
    }

    /// Writes the whole document through a same-directory temporary file and a
    /// rename, then synchronises the directory where the platform supports it.
    fn write_atomic(&self, data: &FileFormat) -> Result<(), StoreError> {
        let text = toml::to_string(data)?;
        let dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        let tmp = dir.join(format!(
            ".{STORE_FILE}.{}.{}.tmp",
            std::process::id(),
            next_temp_id()
        ));
        let result = self.write_temp_and_replace(dir, &tmp, text.as_bytes());
        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        result.map_err(StoreError::from)
    }

    fn write_temp_and_replace(
        &self,
        dir: &Path,
        tmp: &Path,
        bytes: &[u8],
    ) -> std::io::Result<()> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        // The document carries credentials, so keep the temporary file private
        // even when the caller's umask would not.
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(tmp)?;
        file.write_all(bytes)?;
        // Preserve an existing store's permissions; a brand-new one stays 0600.
        if let Ok(meta) = std::fs::metadata(&self.path) {
            std::fs::set_permissions(tmp, meta.permissions())?;
        }
        file.sync_all()?;
        drop(file);
        std::fs::rename(tmp, &self.path)?;
        // Make the rename itself durable where a directory handle can be
        // synchronised (not on Windows).
        if let Ok(dir_handle) = File::open(dir) {
            let _ = dir_handle.sync_all();
        }
        Ok(())
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

    /// Returns the stored value for `key`, or stores `value` when the key is
    /// absent (or empty) and returns it. Concurrent callers converge on one
    /// value because the read and the write happen under the same lock.
    pub fn get_or_insert(&self, key: &str, value: &str) -> Result<String, StoreError> {
        let mut found = value.to_string();
        self.mutate(|current| {
            if let Some(existing) = current.data.get(key).filter(|v| !v.is_empty()) {
                found = existing.clone();
            } else {
                current.data.insert(key.to_string(), value.to_string());
            }
        })?;
        Ok(found)
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), StoreError> {
        self.mutate(|current| {
            current.data.insert(key.to_string(), value.to_string());
        })
    }

    pub fn delete(&self, key: &str) -> Result<(), StoreError> {
        self.mutate(|current| {
            current.data.remove(key);
        })
    }
}

/// Distinguishes temporary files written within one process (and across
/// processes, together with the pid in the name).
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_temp_id() -> u64 {
    TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
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

    #[test]
    fn unchanged_values_do_not_rewrite_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store_v4.toml");
        let original = "# keep this formatting\n[data]\naccess_key = \"existing\"\n";
        std::fs::write(&path, original).unwrap();
        let store = Store::open(dir.path().to_str().unwrap()).unwrap();

        // Same value, and a delete of an absent key: nothing to write.
        store.set("access_key", "existing").unwrap();
        store.delete("missing").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn writes_replace_the_file_without_leftovers() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().to_str().unwrap()).unwrap();
        store.set("a", "1").unwrap();
        store.set("b", "2").unwrap();

        let mut names: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, vec![LOCK_FILE.to_string(), STORE_FILE.to_string()]);

        let text = std::fs::read_to_string(dir.path().join(STORE_FILE)).unwrap();
        assert!(text.contains("a = \"1\""), "{text}");
        assert!(text.contains("b = \"2\""), "{text}");
    }

    #[test]
    fn concurrent_writers_preserve_each_others_keys() {
        let dir = tempfile::tempdir().unwrap();
        let prefix = dir.path().to_str().unwrap().to_string();
        let first = Store::open(&prefix).unwrap();
        let second = Store::open(&prefix).unwrap();

        std::thread::scope(|scope| {
            scope.spawn(|| {
                for i in 0..20 {
                    first.set(&format!("first_{i}"), "a").unwrap();
                }
            });
            scope.spawn(|| {
                for i in 0..20 {
                    second.set(&format!("second_{i}"), "b").unwrap();
                }
            });
        });

        let reopened = Store::open(&prefix).unwrap();
        for i in 0..20 {
            assert_eq!(reopened.get_opt(&format!("first_{i}")).as_deref(), Some("a"));
            assert_eq!(
                reopened.get_opt(&format!("second_{i}")).as_deref(),
                Some("b")
            );
        }
    }

    #[test]
    fn concurrent_get_or_insert_converges_on_one_value() {
        let dir = tempfile::tempdir().unwrap();
        let prefix = dir.path().to_str().unwrap().to_string();
        let first = Store::open(&prefix).unwrap();
        let second = Store::open(&prefix).unwrap();

        let (a, b) = std::thread::scope(|scope| {
            let a = scope.spawn(|| first.get_or_insert("access_key", "first").unwrap());
            let b = scope.spawn(|| second.get_or_insert("access_key", "second").unwrap());
            (a.join().unwrap(), b.join().unwrap())
        });

        assert_eq!(a, b);
        let reopened = Store::open(&prefix).unwrap();
        assert_eq!(reopened.get_opt("access_key").as_deref(), Some(a.as_str()));
    }

    #[cfg(unix)]
    #[test]
    fn failed_write_keeps_the_previous_store() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STORE_FILE);
        let store = Store::open(dir.path().to_str().unwrap()).unwrap();
        store.set("a", "1").unwrap();
        let before = std::fs::read_to_string(&path).unwrap();

        // A read-only directory makes the temporary file impossible to create.
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
        let failed = store.set("b", "2");
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();

        if failed.is_ok() {
            // Running as root bypasses directory permissions; nothing to assert.
            return;
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
        // Memory was not published either, so the next write keeps them in step.
        assert!(store.get_opt("b").is_none());
        store.set("b", "2").unwrap();
        let reopened = Store::open(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(reopened.get_opt("b").as_deref(), Some("2"));
    }

    #[cfg(unix)]
    #[test]
    fn new_store_is_private_and_existing_permissions_are_preserved() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STORE_FILE);
        let store = Store::open(dir.path().to_str().unwrap()).unwrap();
        store.set("k", "v").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "new store files hold credentials");

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        store.set("k2", "v2").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640, "an existing store keeps its own mode");
    }
}
