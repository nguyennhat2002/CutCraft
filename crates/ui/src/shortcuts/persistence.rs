//! Versioned shortcut configuration with safe replacement.
//!
//! The file lives under the user's roaming application data on Windows. Loading
//! is bounded and validation belongs to `ShortcutManager`; this module only
//! handles the file boundary and JSON shape.

use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: u32 = 1;
pub const MAX_CONFIG_BYTES: u64 = 1_048_576;
const CONFIG_DIR: &str = "CutCraft";
const CONFIG_FILE: &str = "shortcuts.json";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedShortcuts {
    pub version: u32,
    pub active_profile: String,
    pub custom_bindings: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShortcutFile {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    active_profile: String,
    #[serde(default)]
    custom_bindings: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutStore {
    path: PathBuf,
}

impl ShortcutStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// `%APPDATA%\CutCraft\shortcuts.json` on Windows; XDG config elsewhere.
    pub fn user_default() -> Option<Self> {
        let base = if cfg!(windows) {
            std::env::var_os("APPDATA").map(PathBuf::from)
        } else {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
                })
        }?;
        Some(Self::new(base.join(CONFIG_DIR).join(CONFIG_FILE)))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Option<PersistedShortcuts>, ShortcutLoadError> {
        let metadata = match fs::metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(ShortcutLoadError::Io(error.to_string())),
        };
        if metadata.len() > MAX_CONFIG_BYTES {
            return Err(ShortcutLoadError::TooLarge(metadata.len()));
        }

        let file =
            File::open(&self.path).map_err(|error| ShortcutLoadError::Io(error.to_string()))?;
        let mut bytes = Vec::new();
        file.take(MAX_CONFIG_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| ShortcutLoadError::Io(error.to_string()))?;
        if bytes.len() as u64 > MAX_CONFIG_BYTES {
            return Err(ShortcutLoadError::TooLarge(bytes.len() as u64));
        }
        let text = String::from_utf8(bytes)
            .map_err(|error| ShortcutLoadError::InvalidJson(error.to_string()))?;
        let parsed: ShortcutFile = serde_json::from_str(&text)
            .map_err(|error| ShortcutLoadError::InvalidJson(error.to_string()))?;
        Ok(Some(PersistedShortcuts {
            version: parsed.version,
            active_profile: parsed.active_profile,
            custom_bindings: parsed.custom_bindings,
        }))
    }

    /// Write a sibling temporary file, flush it, then rename it over the target.
    /// A failed write leaves the previously valid configuration untouched.
    pub fn save(&self, shortcuts: &PersistedShortcuts) -> Result<(), ShortcutSaveError> {
        let file = ShortcutFile {
            version: shortcuts.version,
            active_profile: shortcuts.active_profile.clone(),
            custom_bindings: shortcuts.custom_bindings.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&file)
            .map_err(|error| ShortcutSaveError::Serialize(error.to_string()))?;
        if bytes.len() as u64 > MAX_CONFIG_BYTES {
            return Err(ShortcutSaveError::TooLarge(bytes.len() as u64));
        }

        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|error| ShortcutSaveError::Io(error.to_string()))?;
            }
        }
        let temp_path = self.temporary_path();
        let write_result = (|| -> std::io::Result<()> {
            let mut temp = File::create(&temp_path)?;
            temp.write_all(&bytes)?;
            temp.sync_all()?;
            fs::rename(&temp_path, &self.path)?;
            Ok(())
        })();
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temp_path);
            return Err(ShortcutSaveError::Io(error.to_string()));
        }
        Ok(())
    }

    fn temporary_path(&self) -> PathBuf {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = self
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| CONFIG_FILE.to_owned());
        let parent = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_default();
        parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), counter))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortcutLoadError {
    Io(String),
    TooLarge(u64),
    InvalidJson(String),
}

impl fmt::Display for ShortcutLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "could not read configuration: {message}"),
            Self::TooLarge(size) => write!(formatter, "configuration is too large ({size} bytes)"),
            Self::InvalidJson(message) => write!(formatter, "invalid configuration: {message}"),
        }
    }
}

impl std::error::Error for ShortcutLoadError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortcutSaveError {
    Io(String),
    Serialize(String),
    TooLarge(u64),
    Validation(String),
}

impl fmt::Display for ShortcutSaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "could not write configuration: {message}"),
            Self::Serialize(message) => {
                write!(formatter, "could not serialize configuration: {message}")
            }
            Self::TooLarge(size) => write!(formatter, "configuration is too large ({size} bytes)"),
            Self::Validation(message) => {
                write!(formatter, "invalid shortcut configuration: {message}")
            }
        }
    }
}

impl std::error::Error for ShortcutSaveError {}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempFile {
        path: PathBuf,
    }

    impl TempFile {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "cutcraft-{name}-{}-{}.json",
                std::process::id(),
                TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_file(&path);
            Self { path }
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    fn sample() -> PersistedShortcuts {
        let mut bindings = BTreeMap::new();
        bindings.insert(
            "timeline.zoom_in".to_owned(),
            Some("Ctrl+Shift+Z".to_owned()),
        );
        bindings.insert("timeline.split_all".to_owned(), None);
        PersistedShortcuts {
            version: FORMAT_VERSION,
            active_profile: "custom".to_owned(),
            custom_bindings: bindings,
        }
    }

    #[test]
    fn missing_file_is_not_an_error() {
        let temp = TempFile::new("missing");
        let store = ShortcutStore::new(&temp.path);
        assert_eq!(store.load(), Ok(None));
    }

    #[test]
    fn save_and_load_round_trip_through_json() {
        let temp = TempFile::new("roundtrip");
        let store = ShortcutStore::new(&temp.path);
        store.save(&sample()).expect("save configuration");
        let loaded = store
            .load()
            .expect("load configuration")
            .expect("file exists");
        assert_eq!(loaded, sample());
    }

    #[test]
    fn saving_again_replaces_the_previous_file() {
        let temp = TempFile::new("replace");
        let store = ShortcutStore::new(&temp.path);
        store.save(&sample()).expect("first save");
        let mut second = sample();
        second.active_profile = "default".to_owned();
        second
            .custom_bindings
            .insert("timeline.zoom_in".to_owned(), Some("Ctrl+Alt+Z".to_owned()));
        store.save(&second).expect("replace configuration");
        assert_eq!(store.load().expect("load"), Some(second));
    }

    #[test]
    fn corrupted_json_is_reported_without_panicking() {
        let temp = TempFile::new("corrupt");
        fs::write(&temp.path, b"{ not json").expect("write corrupt fixture");
        let store = ShortcutStore::new(&temp.path);
        match store.load() {
            Err(ShortcutLoadError::InvalidJson(_)) => {}
            other => panic!("expected invalid json, got {other:?}"),
        }
    }

    #[test]
    fn version_and_binding_shapes_survive_serialization() {
        let temp = TempFile::new("shape");
        let store = ShortcutStore::new(&temp.path);
        store.save(&sample()).expect("save");
        let text = fs::read_to_string(&temp.path).expect("read config");
        assert!(text.contains("\"version\": 1"));
        assert!(text.contains("\"custom\""));
        assert!(text.contains("Ctrl+Shift+Z"));
    }
}
