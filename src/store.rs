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
    /// Test hook: makes the post-rename directory sync fail, so the committed
    /// but not durable path can be exercised deterministically.
    #[cfg(test)]
    fail_dir_sync: std::sync::atomic::AtomicBool,
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
    /// The new content already replaced the file, but the directory could not
    /// be synchronised, so it is visible now while a power cut could still undo
    /// it. Distinct from a write that never happened.
    #[error("store replaced but not made durable: {0}")]
    NotDurable(#[source] std::io::Error),
}

impl StoreError {
    /// Whether the change is already visible on disk. Callers that keep an
    /// in-memory copy of what they wrote must still adopt it in this case.
    pub fn committed(&self) -> bool {
        matches!(self, StoreError::NotDurable(_))
    }
}

/// Distinguishes a write that never replaced the file from one whose rename
/// already happened but could not be made durable. The caller needs the
/// difference to keep its in-memory snapshot aligned with the disk.
#[derive(Debug)]
enum WriteFailure {
    BeforeReplace(StoreError),
    AfterReplace(StoreError),
}

impl WriteFailure {
    fn into_error(self) -> StoreError {
        match self {
            WriteFailure::BeforeReplace(e) | WriteFailure::AfterReplace(e) => e,
        }
    }
}

impl Store {
    pub fn open(path_prefix: &str) -> Result<Store, StoreError> {
        std::fs::create_dir_all(path_prefix)?;
        let store = Store {
            path: PathBuf::from(path_prefix).join(STORE_FILE),
            lock_path: PathBuf::from(path_prefix).join(LOCK_FILE),
            inner: Mutex::new(FileFormat::default()),
            #[cfg(test)]
            fail_dir_sync: std::sync::atomic::AtomicBool::new(false),
        };
        {
            let mut guard = store.inner.lock().unwrap();
            let _lock = store.lock()?;
            let (current, needs_create) = store.read_from_disk()?;
            if needs_create {
                store
                    .write_atomic(&current)
                    .map_err(WriteFailure::into_error)?;
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
    /// the same (previous) state — except when the replacement already happened
    /// and only its durability could not be confirmed.
    fn mutate<T>(&self, apply: impl FnOnce(&mut FileFormat) -> T) -> Result<T, StoreError> {
        let mut guard = self.inner.lock().unwrap();
        let _lock = self.lock()?;
        let (mut current, needs_create) = self.read_from_disk()?;
        let previous = current.data.clone();
        let result = apply(&mut current);
        if needs_create || current.data != previous {
            match self.write_atomic(&current) {
                Ok(()) => {}
                // Nothing was replaced: the previous snapshot is still on disk,
                // so leave memory alone.
                Err(WriteFailure::BeforeReplace(e)) => return Err(e),
                // The file on disk is already the new one; publish it so memory
                // matches, then report that durability is unconfirmed.
                Err(WriteFailure::AfterReplace(e)) => {
                    *guard = current;
                    return Err(e);
                }
            }
        }
        *guard = current;
        Ok(result)
    }

    /// Writes the whole document through a same-directory temporary file and a
    /// rename, then synchronises the directory where the platform supports it.
    ///
    /// A colliding temporary name (taken by a concurrent writer) is retried
    /// with a fresh name; only a temporary file this attempt created is ever
    /// removed.
    fn write_atomic(&self, data: &FileFormat) -> Result<(), WriteFailure> {
        let dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        self.write_atomic_with(data, dir, || {
            dir.join(format!(
                ".{STORE_FILE}.{}.{}.tmp",
                std::process::id(),
                next_temp_id()
            ))
        })
    }

    /// The retry loop, with the temporary-name source injected so its collision
    /// path can be exercised deterministically.
    fn write_atomic_with<F>(
        &self,
        data: &FileFormat,
        dir: &Path,
        mut temp_name: F,
    ) -> Result<(), WriteFailure>
    where
        F: FnMut() -> PathBuf,
    {
        let text = toml::to_string(data).map_err(|e| WriteFailure::BeforeReplace(e.into()))?;
        for _ in 0..8 {
            let tmp = temp_name();
            let mut file = match self.create_temp(&tmp) {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(WriteFailure::BeforeReplace(e.into())),
            };
            let result = self.write_temp_and_replace(dir, &tmp, &mut file, text.as_bytes());
            if result.is_err() {
                // `create_temp` succeeded, so this file is ours to remove.
                let _ = std::fs::remove_file(&tmp);
            }
            return result;
        }
        Err(WriteFailure::BeforeReplace(
            std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "could not create a unique temporary store file",
            )
            .into(),
        ))
    }

    fn create_temp(&self, tmp: &Path) -> std::io::Result<File> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        // The document carries credentials, so keep the temporary file private
        // even when the caller's umask would not.
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(tmp)
    }

    fn write_temp_and_replace(
        &self,
        dir: &Path,
        tmp: &Path,
        file: &mut File,
        bytes: &[u8],
    ) -> Result<(), WriteFailure> {
        file.write_all(bytes)
            .map_err(|e| WriteFailure::BeforeReplace(e.into()))?;
        // Preserve an existing store's permissions; a brand-new one stays 0600.
        if let Ok(meta) = std::fs::metadata(&self.path) {
            std::fs::set_permissions(tmp, meta.permissions())
                .map_err(|e| WriteFailure::BeforeReplace(e.into()))?;
        }
        file.sync_all()
            .map_err(|e| WriteFailure::BeforeReplace(e.into()))?;
        std::fs::rename(tmp, &self.path).map_err(|e| WriteFailure::BeforeReplace(e.into()))?;
        #[cfg(test)]
        if self.fail_dir_sync.load(Ordering::Relaxed) {
            return Err(WriteFailure::AfterReplace(StoreError::NotDurable(
                std::io::Error::other("injected directory sync failure"),
            )));
        }
        // Making the rename itself durable needs a directory handle, which
        // Windows cannot open this way. On Unix a failure here means the new
        // content is in place but may not survive a power cut, so it is
        // reported (and the caller publishes the new state).
        #[cfg(unix)]
        {
            let dir_handle =
                File::open(dir).map_err(|e| WriteFailure::AfterReplace(StoreError::NotDurable(e)))?;
            dir_handle
                .sync_all()
                .map_err(|e| WriteFailure::AfterReplace(StoreError::NotDurable(e)))?;
        }
        #[cfg(not(unix))]
        {
            let _ = dir;
        }
        Ok(())
    }

    /// Test hook: makes the post-rename directory sync fail, so the committed
    /// but not durable path can be exercised deterministically.
    #[cfg(test)]
    pub(crate) fn fail_dir_sync_for_test(&self) {
        self.fail_dir_sync.store(true, Ordering::Relaxed);
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

    /// Sets several keys in one atomic write. Nothing is written when every
    /// value is already current, so a group that describes one state — an
    /// account marker plus the observations it scopes — cannot be half-applied.
    pub fn set_many(&self, pairs: &[(&str, &str)]) -> Result<(), StoreError> {
        self.mutate(|current| {
            for (key, value) in pairs {
                current.data.insert((*key).to_string(), (*value).to_string());
            }
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
    fn set_many_writes_one_document_and_skips_unchanged_groups() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STORE_FILE);
        let store = Store::open(dir.path().to_str().unwrap()).unwrap();

        store.set_many(&[("a", "1"), ("b", "2")]).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("a = \"1\""), "{text}");
        assert!(text.contains("b = \"2\""), "{text}");

        // An identical group must not rewrite the file: an external marker in
        // the comment survives only when the store is left alone.
        let original = "# kept\n[data]\na = \"1\"\nb = \"2\"\n";
        std::fs::write(&path, original).unwrap();
        store.set_many(&[("a", "1"), ("b", "2")]).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn a_committed_write_is_visible_and_reports_only_durability() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().to_str().unwrap()).unwrap();
        store.fail_dir_sync_for_test();

        let error = store.set("k", "v").unwrap_err();

        assert!(error.committed(), "expected a committed failure: {error}");
        // Memory matches the already-replaced file, and so does the disk.
        assert_eq!(store.get_opt("k").as_deref(), Some("v"));
        let reopened = Store::open(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(reopened.get_opt("k").as_deref(), Some("v"));
    }

    #[test]
    fn a_colliding_temp_name_is_retried_and_not_removed() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().to_str().unwrap()).unwrap();
        // The name is pre-created, so the first attempt must fail with
        // AlreadyExists and the retry must use a different one. The injected
        // name source makes that deterministic rather than dependent on the
        // process-wide counter.
        let colliding = dir.path().join("collision.tmp");
        std::fs::write(&colliding, "not ours").unwrap();
        let retry = dir.path().join("retry.tmp");
        let mut names = [colliding.clone(), retry.clone()].into_iter();

        let data = FileFormat {
            data: [("k".to_string(), "v".to_string())].into(),
        };
        store
            .write_atomic_with(&data, dir.path(), || names.next().unwrap())
            .unwrap();

        assert_eq!(std::fs::read_to_string(&colliding).unwrap(), "not ours");
        assert!(!retry.exists(), "the successful temporary file was moved");
        let stored = std::fs::read_to_string(dir.path().join(STORE_FILE)).unwrap();
        assert!(stored.contains("k = \"v\""), "{stored}");
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
