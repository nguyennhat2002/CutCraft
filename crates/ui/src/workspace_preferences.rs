//! Persistent workspace layout preferences.
//!
//! The file is versioned, bounded and atomically replaced. Layout remains a UI
//! preference; it never touches project data or media.

use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: u32 = 1;
pub const MAX_CONFIG_BYTES: u64 = 64 * 1024;
const CONFIG_DIR: &str = "CutCraft";
const CONFIG_FILE: &str = "workspace.json";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WorkspacePreferences {
    /// `None` uses the responsive default; `Some` preserves the user's height.
    pub timeline_height_fraction: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkspaceFile {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    timeline_height_fraction: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceStore {
    path: PathBuf,
}

impl WorkspaceStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// `%APPDATA%\CutCraft\workspace.json` on Windows; XDG config elsewhere.
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

    pub fn load(&self) -> Result<Option<WorkspacePreferences>, WorkspaceLoadError> {
        let metadata = match fs::metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(WorkspaceLoadError::Io(error.to_string())),
        };
        if metadata.len() > MAX_CONFIG_BYTES {
            return Err(WorkspaceLoadError::TooLarge(metadata.len()));
        }

        let file =
            File::open(&self.path).map_err(|error| WorkspaceLoadError::Io(error.to_string()))?;
        let mut bytes = Vec::new();
        file.take(MAX_CONFIG_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| WorkspaceLoadError::Io(error.to_string()))?;
        if bytes.len() as u64 > MAX_CONFIG_BYTES {
            return Err(WorkspaceLoadError::TooLarge(bytes.len() as u64));
        }
        let text = String::from_utf8(bytes)
            .map_err(|error| WorkspaceLoadError::InvalidJson(error.to_string()))?;
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        let parsed: WorkspaceFile = serde_json::from_str(text)
            .map_err(|error| WorkspaceLoadError::InvalidJson(error.to_string()))?;
        if parsed.version != FORMAT_VERSION {
            return Err(WorkspaceLoadError::UnsupportedVersion(parsed.version));
        }
        if parsed
            .timeline_height_fraction
            .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            return Err(WorkspaceLoadError::InvalidTimelineHeight);
        }
        Ok(Some(WorkspacePreferences {
            timeline_height_fraction: parsed.timeline_height_fraction,
        }))
    }

    /// Write a sibling temporary file, flush it, then rename it over the target.
    /// A failed write leaves the previously valid preferences untouched.
    pub fn save(&self, preferences: WorkspacePreferences) -> Result<(), WorkspaceSaveError> {
        if preferences
            .timeline_height_fraction
            .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            return Err(WorkspaceSaveError::InvalidTimelineHeight);
        }
        let file = WorkspaceFile {
            version: FORMAT_VERSION,
            timeline_height_fraction: preferences.timeline_height_fraction,
        };
        let bytes = serde_json::to_vec_pretty(&file)
            .map_err(|error| WorkspaceSaveError::Serialize(error.to_string()))?;
        if bytes.len() as u64 > MAX_CONFIG_BYTES {
            return Err(WorkspaceSaveError::TooLarge(bytes.len() as u64));
        }
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|error| WorkspaceSaveError::Io(error.to_string()))?;
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
            return Err(WorkspaceSaveError::Io(error.to_string()));
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
pub enum WorkspaceLoadError {
    Io(String),
    TooLarge(u64),
    InvalidJson(String),
    UnsupportedVersion(u32),
    InvalidTimelineHeight,
}

impl fmt::Display for WorkspaceLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "could not read workspace layout: {message}"),
            Self::TooLarge(size) => {
                write!(formatter, "workspace layout is too large ({size} bytes)")
            }
            Self::InvalidJson(message) => {
                write!(formatter, "invalid workspace layout: {message}")
            }
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported workspace layout version {version}")
            }
            Self::InvalidTimelineHeight => {
                formatter.write_str("workspace layout has an invalid timeline height")
            }
        }
    }
}

impl std::error::Error for WorkspaceLoadError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceSaveError {
    Io(String),
    Serialize(String),
    TooLarge(u64),
    InvalidTimelineHeight,
}

impl fmt::Display for WorkspaceSaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "could not write workspace layout: {message}"),
            Self::Serialize(message) => {
                write!(formatter, "could not serialize workspace layout: {message}")
            }
            Self::TooLarge(size) => {
                write!(formatter, "workspace layout is too large ({size} bytes)")
            }
            Self::InvalidTimelineHeight => {
                formatter.write_str("workspace layout has an invalid timeline height")
            }
        }
    }
}

impl std::error::Error for WorkspaceSaveError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "cutcraft-workspace-{name}-{}-{}",
                std::process::id(),
                TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("temp dir");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn store(temp: &TempDir) -> WorkspaceStore {
        WorkspaceStore::new(temp.0.join("workspace.json"))
    }

    #[test]
    fn missing_file_loads_none() {
        let temp = TempDir::new("missing");
        assert_eq!(store(&temp).load(), Ok(None));
    }

    #[test]
    fn saves_and_restores_both_preference_states() {
        let temp = TempDir::new("round-trip");
        let store = store(&temp);
        let custom = WorkspacePreferences {
            timeline_height_fraction: Some(0.62),
        };

        store.save(custom).expect("save custom height");
        assert_eq!(store.load(), Ok(Some(custom)));

        store
            .save(WorkspacePreferences::default())
            .expect("save automatic height");
        assert_eq!(
            store.load(),
            Ok(Some(WorkspacePreferences {
                timeline_height_fraction: None
            }))
        );
    }

    #[test]
    fn save_replaces_the_previous_file_without_leaving_temps() {
        let temp = TempDir::new("replace");
        let store = store(&temp);
        store
            .save(WorkspacePreferences {
                timeline_height_fraction: Some(0.4),
            })
            .expect("first save");
        store
            .save(WorkspacePreferences {
                timeline_height_fraction: Some(0.7),
            })
            .expect("second save");

        assert_eq!(
            store.load().expect("load").expect("preferences"),
            WorkspacePreferences {
                timeline_height_fraction: Some(0.7)
            }
        );
        let entries = fs::read_dir(&temp.0).expect("entries").count();
        assert_eq!(entries, 1, "atomic replacement must not leave temp files");
    }

    #[test]
    fn missing_height_uses_automatic_layout() {
        let temp = TempDir::new("missing-height");
        let store = store(&temp);
        fs::write(store.path(), br#"{"version":1}"#).expect("write");

        assert_eq!(
            store.load(),
            Ok(Some(WorkspacePreferences {
                timeline_height_fraction: None
            }))
        );
    }

    #[test]
    fn invalid_json_version_and_fraction_are_typed_errors() {
        let temp = TempDir::new("invalid");
        let store = store(&temp);

        fs::write(store.path(), b"{ not json").expect("write corrupt");
        assert!(matches!(
            store.load(),
            Err(WorkspaceLoadError::InvalidJson(_))
        ));

        fs::write(
            store.path(),
            br#"{"version":2,"timeline_height_fraction":0.5}"#,
        )
        .expect("write version");
        assert_eq!(store.load(), Err(WorkspaceLoadError::UnsupportedVersion(2)));

        fs::write(
            store.path(),
            br#"{"version":1,"timeline_height_fraction":1.5}"#,
        )
        .expect("write fraction");
        assert_eq!(store.load(), Err(WorkspaceLoadError::InvalidTimelineHeight));
    }

    #[test]
    fn utf8_bom_preferences_are_accepted() {
        let temp = TempDir::new("bom");
        let store = store(&temp);
        fs::write(
            store.path(),
            b"\xEF\xBB\xBF{\"version\":1,\"timeline_height_fraction\":0.55}",
        )
        .expect("write bom");

        assert_eq!(
            store.load(),
            Ok(Some(WorkspacePreferences {
                timeline_height_fraction: Some(0.55)
            }))
        );
    }

    #[test]
    fn oversized_file_is_rejected_before_parsing() {
        let temp = TempDir::new("too-large");
        let store = store(&temp);
        fs::write(store.path(), vec![b' '; MAX_CONFIG_BYTES as usize + 1])
            .expect("write oversized");

        assert!(matches!(store.load(), Err(WorkspaceLoadError::TooLarge(_))));
    }
}
