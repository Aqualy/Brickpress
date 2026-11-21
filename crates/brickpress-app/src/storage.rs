//! Files and device state.
//!
//! Project and export writes are atomic (a temporary file beside the target,
//! then a rename) and fingerprint-checked: if the file changed on disk since it
//! was opened or last saved, the write fails with [`StorageError::Conflict`]
//! instead of overwriting someone else's change.
//!
//! Device state (recovery, presets, preferences, tracing guide) lives in the
//! same application-data folder Brickpress 1.x used, so it carries over.

use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const INPUT_LIMIT: u64 = 20 * 1024 * 1024;
pub const EXPORT_LIMIT: u64 = 200 * 1024 * 1024;
pub const APP_ID: &str = "io.github.aqualy.brickpress";

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("The file changed on disk since it was opened. Save a copy to keep both versions.")]
    Conflict,
    #[error("The file is larger than {0} MB.")]
    TooLarge(u64),
    #[error("Choose a regular file.")]
    NotAFile,
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fingerprint {
    length: u64,
    digest: [u8; 32],
}

impl Fingerprint {
    pub fn to_hex(&self) -> String {
        let digest: String = self.digest.iter().map(|b| format!("{b:02x}")).collect();
        format!("{}:{digest}", self.length)
    }
    pub fn from_hex(text: &str) -> Option<Self> {
        let (length, digest) = text.split_once(':')?;
        let length = length.parse().ok()?;
        if digest.len() != 64 {
            return None;
        }
        let mut out = [0u8; 32];
        for (i, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(digest.get(i * 2..i * 2 + 2)?, 16).ok()?;
        }
        Some(Self { length, digest: out })
    }
}

/// Hash of a file, or `None` when it does not exist.
pub fn fingerprint(path: &Path, limit: u64) -> Result<Option<Fingerprint>> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(StorageError::NotAFile);
    }
    if meta.len() > limit {
        return Err(StorageError::TooLarge(limit / 1024 / 1024));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut length = 0;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        length += count as u64;
        if length > limit {
            return Err(StorageError::TooLarge(limit / 1024 / 1024));
        }
        hash.update(&buffer[..count]);
    }
    Ok(Some(Fingerprint { length, digest: hash.finalize().into() }))
}

/// Writes beside the destination, then renames over it. A failure (including
/// a failed `check` just before the rename) leaves the original intact.
pub fn atomic_replace_checked(path: &Path, bytes: &[u8], check: impl Fn() -> Result<()>) -> Result<()> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    check()?;
    temporary.persist(path).map_err(|e| StorageError::Io(e.error))?;
    #[cfg(unix)]
    if let Ok(directory) = File::open(parent) {
        let _ = directory.sync_all();
    }
    Ok(())
}

pub fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<()> {
    atomic_replace_checked(path, bytes, || Ok(()))
}

/// Reads a file with a size limit, returning its fingerprint as well.
pub fn read_limited(path: &Path, limit: u64) -> Result<(Vec<u8>, Fingerprint)> {
    let meta = fs::metadata(path)?;
    if !meta.is_file() {
        return Err(StorageError::NotAFile);
    }
    if meta.len() > limit {
        return Err(StorageError::TooLarge(limit / 1024 / 1024));
    }
    let bytes = fs::read(path)?;
    if bytes.len() as u64 > limit {
        return Err(StorageError::TooLarge(limit / 1024 / 1024));
    }
    let digest: [u8; 32] = Sha256::digest(&bytes).into();
    let print = Fingerprint { length: bytes.len() as u64, digest };
    Ok((bytes, print))
}

/// A file the user opened or saved, with the fingerprint it had then.
#[derive(Clone, Debug)]
pub struct TrackedFile {
    pub path: PathBuf,
    pub baseline: Option<Fingerprint>,
}

impl TrackedFile {
    pub fn name(&self) -> String {
        self.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }

    /// Saves over the file only if nobody changed it meanwhile.
    pub fn write(&mut self, bytes: &[u8], limit: u64) -> Result<()> {
        if bytes.len() as u64 > limit {
            return Err(StorageError::TooLarge(limit / 1024 / 1024));
        }
        let baseline = self.baseline.clone();
        let path = self.path.clone();
        atomic_replace_checked(&self.path, bytes, || {
            if fingerprint(&path, limit.max(INPUT_LIMIT))? != baseline {
                return Err(StorageError::Conflict);
            }
            Ok(())
        })?;
        let digest: [u8; 32] = Sha256::digest(bytes).into();
        self.baseline = Some(Fingerprint { length: bytes.len() as u64, digest });
        Ok(())
    }
}

/// Fixed-name records in the application data folder.
#[derive(Clone, Debug)]
pub struct StateStore {
    pub root: PathBuf,
}

pub const STATE_KEYS: [&str; 4] = ["recovery", "presets", "preferences", "trace-metadata"];

/// The record holding a document tab's unsaved changes. `recovery` itself
/// stays the 1.x-compatible record for one document.
pub fn tab_recovery_key(id: &uuid::Uuid) -> String {
    format!("recovery-{id}")
}

fn tab_recovery_id(key: &str) -> Option<uuid::Uuid> {
    uuid::Uuid::parse_str(key.strip_prefix("recovery-")?).ok()
}

impl StateStore {
    /// The platform data folder, or `BRICKPRESS_DATA_DIR` when set (tests).
    pub fn default_root() -> PathBuf {
        if let Some(dir) = std::env::var_os("BRICKPRESS_DATA_DIR") {
            return PathBuf::from(dir);
        }
        dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join(APP_ID)
    }

    pub fn open(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    fn path(&self, key: &str) -> PathBuf {
        debug_assert!(STATE_KEYS.contains(&key) || key == "app" || tab_recovery_id(key).is_some(), "{key}");
        self.root.join(format!("{key}.json"))
    }

    /// Keys of the per-tab recovery records (`recovery-<uuid>`) on disk.
    pub fn tab_recovery_keys(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(&self.root) else { return Vec::new() };
        entries
            .flatten()
            .filter_map(|e| e.file_name().to_str()?.strip_suffix(".json").map(str::to_string))
            .filter(|key| tab_recovery_id(key).is_some())
            .collect()
    }

    pub fn read(&self, key: &str) -> Result<Option<String>> {
        let path = self.path(key);
        match read_limited(&path, INPUT_LIMIT) {
            Ok((bytes, _)) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
            Err(StorageError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn write(&self, key: &str, value: Option<&str>) -> Result<()> {
        let path = self.path(key);
        match value {
            Some(text) => atomic_replace(&path, text.as_bytes()),
            None => match fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
                _ => Ok(()),
            },
        }
    }

    /// Moves an unreadable record aside for inspection instead of deleting it.
    pub fn quarantine(&self, key: &str) -> Result<()> {
        let path = self.path(key);
        if path.exists() {
            let aside = path.with_extension(format!("invalid-{}.json", uuid::Uuid::new_v4()));
            fs::rename(&path, aside)?;
        }
        Ok(())
    }

    fn asset_path(&self, id: &str) -> Option<PathBuf> {
        let uuid = uuid::Uuid::parse_str(id).ok()?;
        Some(self.root.join("tracing").join(format!("{uuid}.bin")))
    }

    pub fn write_asset(&self, bytes: &[u8]) -> Result<String> {
        if bytes.len() as u64 > INPUT_LIMIT {
            return Err(StorageError::TooLarge(20));
        }
        let id = uuid::Uuid::new_v4().to_string();
        fs::create_dir_all(self.root.join("tracing"))?;
        atomic_replace(&self.asset_path(&id).expect("valid uuid"), bytes)?;
        Ok(id)
    }

    pub fn read_asset(&self, id: &str) -> Result<Vec<u8>> {
        let path = self.asset_path(id).ok_or(StorageError::NotAFile)?;
        Ok(read_limited(&path, INPUT_LIMIT)?.0)
    }

    /// Removes guide images other than those in `keep`.
    pub fn prune_assets(&self, keep: &[&str]) -> Result<()> {
        let directory = self.root.join("tracing");
        let Ok(entries) = fs::read_dir(&directory) else { return Ok(()) };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let stem = name.strip_suffix(".bin").unwrap_or(&name);
            if !keep.contains(&stem) && uuid::Uuid::parse_str(stem).is_ok() {
                let _ = fs::remove_file(entry.path());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracked(path: &Path) -> TrackedFile {
        TrackedFile { path: path.into(), baseline: fingerprint(path, INPUT_LIMIT).unwrap() }
    }

    #[test]
    fn unicode_save_replaces_and_detects_external_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("印刷 café 🧱.brickpress.json");
        let mut file = tracked(&path);
        file.write(b"one", INPUT_LIMIT).unwrap();
        file.write(b"two", INPUT_LIMIT).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"two");
        fs::write(&path, b"someone else").unwrap();
        assert!(matches!(file.write(b"three", INPUT_LIMIT), Err(StorageError::Conflict)));
        assert_eq!(fs::read(&path).unwrap(), b"someone else");
    }

    #[test]
    fn new_targets_created_or_deleted_externally_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project.json");
        let mut file = tracked(&path);
        fs::write(&path, b"appeared").unwrap();
        assert!(matches!(file.write(b"mine", INPUT_LIMIT), Err(StorageError::Conflict)));
        let mut file = tracked(&path);
        fs::remove_file(&path).unwrap();
        assert!(matches!(file.write(b"mine", INPUT_LIMIT), Err(StorageError::Conflict)));
    }

    #[test]
    fn failed_commit_preserves_original() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project.json");
        fs::write(&path, b"original").unwrap();
        let result = atomic_replace_checked(&path, b"replacement", || Err(StorageError::Conflict));
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1, "no temporary file left behind");
    }

    #[test]
    fn state_survives_reopen_and_invalid_records_are_quarantined() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::open(dir.path().into()).unwrap();
        store.write("recovery", Some("{}")).unwrap();
        let reopened = StateStore::open(dir.path().into()).unwrap();
        assert_eq!(reopened.read("recovery").unwrap().as_deref(), Some("{}"));
        reopened.quarantine("recovery").unwrap();
        assert_eq!(reopened.read("recovery").unwrap(), None);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        store.write("presets", None).unwrap();
        let tab = tab_recovery_key(&uuid::Uuid::new_v4());
        store.write(&tab, Some("{}")).unwrap();
        assert_eq!(store.tab_recovery_keys(), vec![tab.clone()]);
        store.write(&tab, None).unwrap();
        assert!(store.tab_recovery_keys().is_empty());
    }

    #[test]
    fn trace_assets_are_pruned_only_after_the_new_one_exists() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::open(dir.path().into()).unwrap();
        let old = store.write_asset(b"old").unwrap();
        let new = store.write_asset(b"new").unwrap();
        store.prune_assets(&[&new]).unwrap();
        assert!(store.read_asset(&old).is_err());
        assert_eq!(store.read_asset(&new).unwrap(), b"new");
        assert!(store.read_asset("../../etc/passwd").is_err(), "ids are not paths");
    }

    #[test]
    fn oversized_input_is_rejected_before_reading() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.json");
        let file = File::create(&path).unwrap();
        file.set_len(INPUT_LIMIT + 1).unwrap();
        assert!(matches!(read_limited(&path, INPUT_LIMIT), Err(StorageError::TooLarge(_))));
    }
}
