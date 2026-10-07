use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const INPUT_LIMIT: u64 = 20 * 1024 * 1024;
const EXPORT_LIMIT: u64 = 200 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct BridgeError {
    pub code: &'static str,
    pub message: String,
}

impl BridgeError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
impl From<std::io::Error> for BridgeError {
    fn from(error: std::io::Error) -> Self {
        Self::new("io", error.to_string())
    }
}
pub type Result<T> = std::result::Result<T, BridgeError>;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    Project,
    Presets,
    Trace,
    Png,
    Svg,
}
impl FileKind {
    pub fn filter(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Project => ("Brickpress project", &["json"]),
            Self::Presets => ("Brickpress presets", &["json"]),
            Self::Trace => (
                "Tracing image",
                &["png", "jpg", "jpeg", "webp", "gif", "avif", "bmp"],
            ),
            Self::Png => ("PNG image", &["png"]),
            Self::Svg => ("SVG image", &["svg"]),
        }
    }
    fn limit(self) -> u64 {
        match self {
            Self::Png | Self::Svg => EXPORT_LIMIT,
            _ => INPUT_LIMIT,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedFile {
    pub token: String,
    pub name: String,
    pub path: String,
    pub mime: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Fingerprint {
    length: u64,
    digest: [u8; 32],
}
fn fingerprint(path: &Path, limit: u64) -> Result<Option<Fingerprint>> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !file.metadata()?.is_file() {
        return Err(BridgeError::new("invalid", "Choose a regular file."));
    }
    if file.metadata()?.len() > limit {
        return Err(BridgeError::new(
            "too-large",
            "The file exceeds the permitted size.",
        ));
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
            return Err(BridgeError::new(
                "too-large",
                "The file exceeds the permitted size.",
            ));
        }
        hash.update(&buffer[..count]);
    }
    Ok(Some(Fingerprint {
        length,
        digest: hash.finalize().into(),
    }))
}

// Temporary file stays beside its destination, so persist uses an atomic rename
// on one filesystem. A failed commit leaves the original destination intact.
fn atomic_replace_checked(path: &Path, bytes: &[u8], check: impl Fn() -> Result<()>) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| BridgeError::new("invalid", "File has no parent directory."))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    check()?;
    temporary
        .persist(path)
        .map_err(|error| BridgeError::from(error.error))?;
    // Sync directory entries where supported. Windows does not allow opening a
    // directory with File::open; the flushed file and atomic replacement apply.
    #[cfg(unix)]
    if let Ok(directory) = File::open(parent) {
        let _ = directory.sync_all();
    }
    Ok(())
}
fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<()> {
    atomic_replace_checked(path, bytes, || Ok(()))
}

struct Handle {
    path: PathBuf,
    baseline: Option<Fingerprint>,
    limit: u64,
    writable: bool,
}
pub struct FileStore {
    handles: HashMap<String, Handle>,
    pub root: PathBuf,
}
impl FileStore {
    pub fn uninitialized(root: PathBuf) -> Self {
        Self {
            handles: HashMap::new(),
            root,
        }
    }
    pub fn new(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;
        Ok(Self {
            handles: HashMap::new(),
            root,
        })
    }
    pub fn select(&mut self, path: PathBuf, kind: FileKind, save: bool) -> Result<SelectedFile> {
        let limit = if save { kind.limit() } else { INPUT_LIMIT };
        let baseline = fingerprint(&path, limit)?;
        if !save && baseline.is_none() {
            return Err(BridgeError::new(
                "io",
                "The selected file no longer exists.",
            ));
        }
        let token = Uuid::new_v4().to_string();
        let name = path
            .file_name()
            .ok_or_else(|| BridgeError::new("invalid", "Choose a file."))?
            .to_string_lossy()
            .into_owned();
        let mime = match path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "avif" => "image/avif",
            "bmp" => "image/bmp",
            "svg" => "image/svg+xml",
            _ => "application/json",
        }
        .to_string();
        let selected = SelectedFile {
            token: token.clone(),
            name,
            path: path.to_string_lossy().into_owned(),
            mime,
        };
        // An opened project can be saved back; other input tokens cannot write.
        self.handles.insert(
            token,
            Handle {
                path,
                baseline,
                limit,
                writable: save || matches!(kind, FileKind::Project),
            },
        );
        Ok(selected)
    }
    fn handle(&self, token: &str) -> Result<&Handle> {
        self.handles.get(token).ok_or_else(|| {
            BridgeError::new(
                "invalid-handle",
                "The selected file is no longer available. Choose it again.",
            )
        })
    }
    fn check(handle: &Handle) -> Result<()> {
        if fingerprint(&handle.path, handle.limit)? != handle.baseline {
            return Err(BridgeError::new("conflict", "This file was changed or removed outside Brickpress. Use Save As to keep your changes."));
        }
        Ok(())
    }
    pub fn read(&self, token: &str) -> Result<Vec<u8>> {
        let handle = self.handle(token)?;
        Self::check(handle)?;
        let mut bytes = Vec::new();
        File::open(&handle.path)?
            .take(INPUT_LIMIT + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > INPUT_LIMIT {
            return Err(BridgeError::new(
                "too-large",
                "Inputs must be at most 20 MB.",
            ));
        }
        if Some(Fingerprint {
            length: bytes.len() as u64,
            digest: Sha256::digest(&bytes).into(),
        }) != handle.baseline
        {
            return Err(BridgeError::new(
                "conflict",
                "The file changed while reading. Open it again.",
            ));
        }
        Ok(bytes)
    }
    pub fn write(&mut self, token: &str, bytes: &[u8]) -> Result<()> {
        let handle = self.handle(token)?;
        if !handle.writable {
            return Err(BridgeError::new("forbidden", "This handle is read-only."));
        }
        if bytes.len() as u64 > handle.limit {
            return Err(BridgeError::new(
                "too-large",
                "Output exceeds the permitted size.",
            ));
        }
        Self::check(handle)?;
        atomic_replace_checked(&handle.path, bytes, || Self::check(handle))?;
        self.handles
            .get_mut(token)
            .expect("validated handle")
            .baseline = Some(Fingerprint {
            length: bytes.len() as u64,
            digest: Sha256::digest(bytes).into(),
        });
        Ok(())
    }
    pub fn release(&mut self, token: &str) {
        self.handles.remove(token);
    }
    fn state_path(&self, key: &str) -> Result<PathBuf> {
        match key {
            "recovery" | "presets" | "preferences" | "trace-metadata" => {
                Ok(self.root.join(format!("{key}.json")))
            }
            _ => Err(BridgeError::new("invalid", "Unknown storage record.")),
        }
    }
    pub fn read_state(&self, key: &str) -> Result<Option<String>> {
        // Retry a failed startup directory initialization through the frontend
        // hydration path, so it can display an error instead of closing the app.
        fs::create_dir_all(&self.root)?;
        let path = self.state_path(key)?;
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if metadata.len() > INPUT_LIMIT {
            return Err(BridgeError::new(
                "too-large",
                "The stored record exceeds 20 MB.",
            ));
        }
        Ok(Some(fs::read_to_string(path)?))
    }
    pub fn write_state(&self, key: &str, value: Option<&str>) -> Result<()> {
        let path = self.state_path(key)?;
        if let Some(value) = value {
            if value.len() as u64 > INPUT_LIMIT {
                return Err(BridgeError::new(
                    "too-large",
                    "Stored records must be at most 20 MB.",
                ));
            }
            atomic_replace(&path, value.as_bytes())
        } else {
            match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.into()),
            }
        }
    }
    pub fn quarantine(&self, key: &str) -> Result<()> {
        let path = self.state_path(key)?;
        if path.exists() {
            fs::rename(
                &path,
                path.with_extension(format!("invalid-{}.json", Uuid::new_v4())),
            )?;
        }
        Ok(())
    }
    fn asset_path(&self, id: &str) -> Result<PathBuf> {
        let uuid = Uuid::parse_str(id)
            .map_err(|_| BridgeError::new("invalid", "Unknown tracing asset."))?;
        Ok(self.root.join("tracing").join(format!("{uuid}.bin")))
    }
    pub fn write_asset(&self, bytes: &[u8]) -> Result<String> {
        if bytes.len() as u64 > INPUT_LIMIT {
            return Err(BridgeError::new(
                "too-large",
                "Tracing images must be at most 20 MB.",
            ));
        }
        fs::create_dir_all(self.root.join("tracing"))?;
        let id = Uuid::new_v4().to_string();
        atomic_replace(&self.asset_path(&id)?, bytes)?;
        Ok(id)
    }
    pub fn read_asset(&self, id: &str) -> Result<Vec<u8>> {
        let path = self.asset_path(id)?;
        let mut bytes = Vec::new();
        File::open(path)?
            .take(INPUT_LIMIT + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > INPUT_LIMIT {
            return Err(BridgeError::new("too-large", "Stored image exceeds 20 MB."));
        }
        Ok(bytes)
    }
    pub fn prune_assets(&self, keep: Option<&str>) -> Result<()> {
        let keep = keep.map(|id| self.asset_path(id)).transpose()?;
        let directory = self.root.join("tracing");
        if !directory.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path.extension().and_then(|v| v.to_str()) == Some("bin")
                && path
                    .file_stem()
                    .and_then(|v| v.to_str())
                    .is_some_and(|v| Uuid::parse_str(v).is_ok())
                && Some(&path) != keep.as_ref()
            {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }
    #[cfg(feature = "desktop-e2e")]
    pub fn reset_handles(&mut self) {
        self.handles.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_save_replaces_and_reads_then_detects_external_changes() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = FileStore::new(dir.path().join("state")).unwrap();
        let path = dir.path().join("印刷 café 🧱.brickpress.json");
        let selected = store.select(path.clone(), FileKind::Project, true).unwrap();
        store.write(&selected.token, b"first").unwrap();
        store.write(&selected.token, b"second").unwrap();
        assert_eq!(store.read(&selected.token).unwrap(), b"second");
        fs::write(&path, b"outside").unwrap();
        assert_eq!(
            store.write(&selected.token, b"third").unwrap_err().code,
            "conflict"
        );
        assert_eq!(fs::read(path).unwrap(), b"outside");
    }
    #[test]
    fn cancelled_or_failed_commit_preserves_original() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project.json");
        fs::write(&path, b"original").unwrap();
        let error = atomic_replace_checked(&path, b"replacement", || {
            Err(BridgeError::new("io", "simulated failure"))
        })
        .unwrap_err();
        assert_eq!(error.code, "io");
        assert_eq!(fs::read(path).unwrap(), b"original");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    #[test]
    fn new_target_created_externally_and_deleted_target_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = FileStore::new(dir.path().join("state")).unwrap();
        let path = dir.path().join("project.json");
        let selected = store.select(path.clone(), FileKind::Project, true).unwrap();
        fs::write(&path, b"outside").unwrap();
        assert_eq!(
            store.write(&selected.token, b"new").unwrap_err().code,
            "conflict"
        );
        let opened = store
            .select(path.clone(), FileKind::Project, false)
            .unwrap();
        fs::remove_file(path).unwrap();
        assert_eq!(
            store.write(&opened.token, b"new").unwrap_err().code,
            "conflict"
        );
    }
    #[test]
    fn tokens_and_state_keys_do_not_grant_arbitrary_paths() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = FileStore::new(dir.path().to_owned()).unwrap();
        assert!(store.read("../../private").is_err());
        assert!(store.write_state("../private", Some("bad")).is_err());
        assert!(store.read_asset("../private").is_err());
        let path = dir.path().join("image.png");
        fs::write(&path, b"image").unwrap();
        let file = store.select(path, FileKind::Trace, false).unwrap();
        assert_eq!(
            store.write(&file.token, b"overwrite").unwrap_err().code,
            "forbidden"
        );
        store.release(&file.token);
        assert_eq!(store.read(&file.token).unwrap_err().code, "invalid-handle");
    }
    #[test]
    fn persistent_state_survives_reopen_and_invalid_record_is_quarantined() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::new(dir.path().to_owned()).unwrap();
        store.write_state("recovery", Some("{invalid")).unwrap();
        let reopened = FileStore::new(dir.path().to_owned()).unwrap();
        assert_eq!(
            reopened.read_state("recovery").unwrap().as_deref(),
            Some("{invalid")
        );
        reopened.quarantine("recovery").unwrap();
        assert_eq!(reopened.read_state("recovery").unwrap(), None);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        reopened.write_state("preferences", Some("{}")).unwrap();
        reopened.write_state("preferences", None).unwrap();
        assert_eq!(reopened.read_state("preferences").unwrap(), None);
    }
    #[test]
    fn trace_pair_failure_keeps_previous_asset_until_metadata_commits() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::new(dir.path().to_owned()).unwrap();
        let old = store.write_asset(b"old").unwrap();
        store.write_state("trace-metadata", Some(&old)).unwrap();
        let new = store.write_asset(b"new").unwrap();
        assert_eq!(store.read_asset(&old).unwrap(), b"old");
        store.write_state("trace-metadata", Some(&new)).unwrap();
        store.prune_assets(Some(&new)).unwrap();
        assert!(store.read_asset(&old).is_err());
        assert_eq!(store.read_asset(&new).unwrap(), b"new");
    }
    #[test]
    fn oversized_input_is_rejected_without_reading_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oversized.json");
        File::create(&path)
            .unwrap()
            .set_len(INPUT_LIMIT + 1)
            .unwrap();
        let mut store = FileStore::new(dir.path().join("state")).unwrap();
        assert_eq!(
            store
                .select(path, FileKind::Project, false)
                .err()
                .unwrap()
                .code,
            "too-large"
        );
    }

    #[test]
    fn unavailable_storage_returns_an_error_for_frontend_hydration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-a-directory");
        fs::write(&path, b"keep this file").unwrap();
        let store = FileStore::uninitialized(path.clone());
        assert_eq!(store.read_state("recovery").unwrap_err().code, "io");
        assert_eq!(fs::read(path).unwrap(), b"keep this file");
    }
}
