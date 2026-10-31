//! Managed CutCraft project library.
//!
//! Projects are stored under an application-owned root and addressed by stable
//! Project IDs. Display names never become filesystem paths.

use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use capcut_engine::asset::MediaLocator;
use capcut_engine::command::{Command, NativeEditor};
use capcut_engine::project::ProjectSnapshot;

use crate::error::{ProjectOpenError, ProjectSaveError};
use crate::id::ProjectId;
use crate::recovery::{
    read_recovery_document, recovery_path, RecoveryDocument, RecoveryError,
    MAX_RECOVERY_CANDIDATES, RECOVERY_DIRECTORY, RECOVERY_FILE_SUFFIX,
};
use crate::schema::ProjectDocumentMetadata;
use crate::store::{now_unix_ms, open, save_snapshot, LoadedProject, SavedDocument};

const PROJECT_DIRECTORY_NAME: &str = "project.cutcraft";
const MAX_LIBRARY_ENTRIES: usize = 128;
const MAX_PROJECT_NAME_BYTES: usize = 256;

#[derive(Debug, Clone)]
pub struct ProjectLibrary {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedProject {
    pub project_id: ProjectId,
    pub name: String,
    pub path: PathBuf,
    pub directory: PathBuf,
    pub created_at_unix_ms: u64,
}

#[derive(Debug, Clone)]
pub struct CreatedProject {
    pub project_id: ProjectId,
    pub name: String,
    pub path: PathBuf,
    pub directory: PathBuf,
    pub created_at_unix_ms: u64,
    pub saved: SavedDocument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectEntryStatus {
    Valid,
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryCandidateStatus {
    Newer,
    SameGeneration,
    Stale,
    Orphan,
    Invalid(String),
}

#[derive(Debug, Clone)]
pub struct RecoveryCandidateInfo {
    pub path: PathBuf,
    pub project_id: Option<ProjectId>,
    pub name: Option<String>,
    pub original_project_path: Option<PathBuf>,
    pub document_generation: Option<u64>,
    pub last_saved_generation: Option<u64>,
    pub written_at_unix_ms: Option<u64>,
    pub status: RecoveryCandidateStatus,
    pub detail: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectLibraryEntry {
    pub directory_name: String,
    pub directory: PathBuf,
    pub path: PathBuf,
    pub project_id: Option<ProjectId>,
    pub name: Option<String>,
    pub schema_version: Option<u32>,
    pub created_at_unix_ms: u64,
    pub last_saved_unix_ms: u64,
    pub save_generation: Option<u64>,
    pub status: ProjectEntryStatus,
}

impl ProjectLibraryEntry {
    pub fn is_valid(&self) -> bool {
        self.status == ProjectEntryStatus::Valid && self.project_id.is_some()
    }

    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.directory_name)
    }
}

impl ProjectLibrary {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// `CUTCRAFT_PROJECTS_DIR`, otherwise a per-user local data directory.
    pub fn user_default() -> Result<Self, ProjectLibraryError> {
        if let Some(root) = std::env::var_os("CUTCRAFT_PROJECTS_DIR") {
            return Ok(Self::new(root));
        }
        let base = if cfg!(windows) {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .ok_or_else(|| {
                    ProjectLibraryError::RootUnavailable("LOCALAPPDATA is unavailable".to_owned())
                })?
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
                })
                .ok_or_else(|| {
                    ProjectLibraryError::RootUnavailable(
                        "XDG_DATA_HOME or HOME is unavailable".to_owned(),
                    )
                })?
        };
        Ok(Self::new(base.join("CutCraft").join("Projects")))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn prepare(
        &self,
        snapshot: &mut ProjectSnapshot,
        name: &str,
    ) -> Result<PreparedProject, ProjectLibraryError> {
        validate_project_name(name)?;
        snapshot
            .project
            .rename(name)
            .map_err(|error| ProjectLibraryError::InvalidProject(error.to_string()))?;
        snapshot
            .project
            .validate()
            .map_err(|error| ProjectLibraryError::InvalidProject(error.to_string()))?;
        fs::create_dir_all(&self.root).map_err(|error| ProjectLibraryError::Io {
            path: self.root.clone(),
            message: error.to_string(),
        })?;
        for _ in 0..8 {
            let project_id = ProjectId::generate();
            let directory = self.root.join(project_id.to_string());
            match fs::create_dir(&directory) {
                Ok(()) => {
                    return Ok(PreparedProject {
                        project_id,
                        name: name.to_owned(),
                        path: directory.join(PROJECT_DIRECTORY_NAME),
                        directory,
                        created_at_unix_ms: now_unix_ms(),
                    })
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(ProjectLibraryError::Io {
                        path: directory,
                        message: error.to_string(),
                    })
                }
            }
        }
        Err(ProjectLibraryError::IdCollision)
    }

    pub fn discard_prepared(&self, prepared: &PreparedProject) {
        let _ = fs::remove_file(&prepared.path);
        let _ = fs::remove_dir(&prepared.directory);
    }

    pub fn create(
        &self,
        mut snapshot: ProjectSnapshot,
        name: &str,
    ) -> Result<CreatedProject, ProjectLibraryError> {
        let prepared = self.prepare(&mut snapshot, name)?;
        let metadata = ProjectDocumentMetadata {
            project_id: prepared.project_id.clone(),
            created_at_unix_ms: prepared.created_at_unix_ms,
            save_generation: 0,
        };
        match save_snapshot(&snapshot, &metadata, &prepared.path, None) {
            Ok(saved) => Ok(CreatedProject {
                project_id: prepared.project_id,
                name: prepared.name,
                path: prepared.path,
                directory: prepared.directory,
                created_at_unix_ms: prepared.created_at_unix_ms,
                saved,
            }),
            Err(error) => {
                self.discard_prepared(&prepared);
                Err(ProjectLibraryError::Save(error))
            }
        }
    }

    pub fn open_entry(
        &self,
        entry: &ProjectLibraryEntry,
    ) -> Result<LoadedProject, ProjectLibraryError> {
        if !entry.is_valid() {
            return Err(ProjectLibraryError::InvalidEntry(
                entry.display_name().to_owned(),
            ));
        }
        open(&entry.path).map_err(ProjectLibraryError::Open)
    }

    pub fn scan(&self) -> Result<Vec<ProjectLibraryEntry>, ProjectLibraryError> {
        let root_metadata = match fs::metadata(&self.root) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(ProjectLibraryError::Io {
                    path: self.root.clone(),
                    message: error.to_string(),
                })
            }
        };
        if !root_metadata.is_dir() {
            return Err(ProjectLibraryError::Io {
                path: self.root.clone(),
                message: "projects root is not a directory".to_owned(),
            });
        }
        let read_dir = fs::read_dir(&self.root).map_err(|error| ProjectLibraryError::Io {
            path: self.root.clone(),
            message: error.to_string(),
        })?;
        let mut entries = Vec::new();
        for item in read_dir {
            if entries.len() >= MAX_LIBRARY_ENTRIES {
                break;
            }
            let item = match item {
                Ok(item) => item,
                Err(error) => {
                    entries.push(invalid_entry(
                        self.root.clone(),
                        "<unreadable>".to_owned(),
                        error.to_string(),
                        None,
                    ));
                    continue;
                }
            };
            let directory = item.path();
            let directory_name = item.file_name().to_string_lossy().into_owned();
            if directory_name.starts_with('.') {
                continue;
            }
            let file_type = match item.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    entries.push(invalid_entry(
                        directory.clone(),
                        directory_name,
                        error.to_string(),
                        None,
                    ));
                    continue;
                }
            };
            if !file_type.is_dir() {
                continue;
            }
            let path = directory.join(PROJECT_DIRECTORY_NAME);
            let modified = fs::metadata(&path)
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(system_time_millis)
                .unwrap_or(0);
            let directory_id = ProjectId::parse(&directory_name).ok();
            match open(&path) {
                Ok(loaded) => entries.push(ProjectLibraryEntry {
                    directory_name,
                    directory: directory.clone(),
                    path,
                    project_id: Some(loaded.project_id.clone()),
                    name: Some(loaded.editor.project().name().to_owned()),
                    schema_version: Some(loaded.schema_version),
                    created_at_unix_ms: loaded.created_at_unix_ms,
                    last_saved_unix_ms: if loaded.last_saved_unix_ms == 0 {
                        modified
                    } else {
                        loaded.last_saved_unix_ms
                    },
                    save_generation: Some(loaded.save_generation),
                    status: ProjectEntryStatus::Valid,
                }),
                Err(error) => {
                    let mut entry = invalid_entry(
                        directory.clone(),
                        directory_name,
                        error.to_string(),
                        directory_id,
                    );
                    entry.last_saved_unix_ms = modified;
                    entries.push(entry);
                }
            }
        }
        entries.sort_by(|a, b| {
            b.last_saved_unix_ms
                .cmp(&a.last_saved_unix_ms)
                .then_with(|| a.directory_name.cmp(&b.directory_name))
        });
        Ok(entries)
    }

    pub fn recovery_root(&self) -> PathBuf {
        self.root.join(RECOVERY_DIRECTORY)
    }

    /// Inspect the bounded recovery area and classify every candidate against
    /// the current library entries. This never modifies a primary project.
    pub fn scan_recovery_candidates(
        &self,
        entries: &[ProjectLibraryEntry],
    ) -> Result<Vec<RecoveryCandidateInfo>, ProjectLibraryError> {
        let root = self.recovery_root();
        let read_dir = match fs::read_dir(&root) {
            Ok(read_dir) => read_dir,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(ProjectLibraryError::Io {
                    path: root,
                    message: error.to_string(),
                })
            }
        };
        let mut candidates = Vec::new();
        for item in read_dir {
            if candidates.len() >= MAX_RECOVERY_CANDIDATES {
                break;
            }
            let item = match item {
                Ok(item) => item,
                Err(_) => continue,
            };
            let is_file = item.file_type().map(|kind| kind.is_file()).unwrap_or(false);
            if !is_file {
                continue;
            }
            let path = item.path();
            if path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with('.'))
            {
                continue;
            }
            let Some(file_name) = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
            else {
                continue;
            };
            if !file_name.ends_with(RECOVERY_FILE_SUFFIX) {
                continue;
            }
            let project_id = file_name
                .strip_suffix(RECOVERY_FILE_SUFFIX)
                .and_then(|id| ProjectId::parse(id).ok());
            match read_recovery_document(&path) {
                Ok(document) => {
                    let entry = entries.iter().find(|entry| {
                        entry.is_valid() && entry.project_id.as_ref() == Some(&document.project_id)
                    });
                    let (status, detail) = match entry {
                        None => (
                            RecoveryCandidateStatus::Orphan,
                            Some("no valid primary project entry".to_owned()),
                        ),
                        Some(entry) => {
                            let primary = entry.save_generation.unwrap_or(0);
                            if document.document_generation > primary {
                                (RecoveryCandidateStatus::Newer, None)
                            } else if document.document_generation == primary {
                                (RecoveryCandidateStatus::SameGeneration, None)
                            } else {
                                (RecoveryCandidateStatus::Stale, None)
                            }
                        }
                    };
                    candidates.push(RecoveryCandidateInfo {
                        path,
                        project_id: Some(document.project_id),
                        name: Some(document.name),
                        original_project_path: Some(document.original_project_path),
                        document_generation: Some(document.document_generation),
                        last_saved_generation: Some(document.last_saved_generation),
                        written_at_unix_ms: Some(document.written_at_unix_ms),
                        status,
                        detail,
                    });
                }
                Err(error) => candidates.push(RecoveryCandidateInfo {
                    path,
                    project_id,
                    name: None,
                    original_project_path: None,
                    document_generation: None,
                    last_saved_generation: None,
                    written_at_unix_ms: None,
                    status: RecoveryCandidateStatus::Invalid(error.to_string()),
                    detail: Some(error.to_string()),
                }),
            }
        }
        candidates.sort_by(|a, b| {
            b.document_generation
                .unwrap_or(0)
                .cmp(&a.document_generation.unwrap_or(0))
                .then_with(|| a.path.cmp(&b.path))
        });
        Ok(candidates)
    }

    pub fn load_recovery_document(
        &self,
        info: &RecoveryCandidateInfo,
    ) -> Result<RecoveryDocument, ProjectLibraryError> {
        self.validate_recovery_path(info)?;
        let document = read_recovery_document(&info.path).map_err(ProjectLibraryError::Recovery)?;
        if info
            .project_id
            .as_ref()
            .is_some_and(|id| id != &document.project_id)
        {
            return Err(ProjectLibraryError::InvalidEntry(
                "recovery project identity changed during validation".to_owned(),
            ));
        }
        Ok(document)
    }

    /// Prepare a recovered document as a new managed project. Relative media
    /// locators are rebased to the original files; the original project and
    /// media bytes are never modified.
    pub fn prepare_recovered(
        &self,
        document: &RecoveryDocument,
        current_project_file: Option<&Path>,
    ) -> Result<(PreparedProject, ProjectSnapshot), ProjectLibraryError> {
        let project_file = if document.original_project_path.exists() {
            document.original_project_path.as_path()
        } else {
            current_project_file.unwrap_or(document.original_project_path.as_path())
        };
        let mut editor = document.editor.clone();
        rebase_relative_media(&mut editor, project_file)?;
        let name = recovered_copy_name(&document.name);
        let mut snapshot = editor.snapshot();
        let prepared = self.prepare(&mut snapshot, &name)?;
        Ok((prepared, snapshot))
    }

    /// Prepare a new project from the live editor snapshot for conflict
    /// preservation. The snapshot is validated and relative media is rebased
    /// against the existing project file before a new directory is allocated.
    pub fn prepare_copy_of_snapshot(
        &self,
        snapshot: ProjectSnapshot,
        project_file: &Path,
        name: &str,
    ) -> Result<(PreparedProject, ProjectSnapshot), ProjectLibraryError> {
        let mut editor = NativeEditor::from_restored(snapshot.project, snapshot.magnet_enabled)
            .map_err(|error| ProjectLibraryError::InvalidProject(error.to_string()))?;
        rebase_relative_media(&mut editor, project_file)?;
        let mut snapshot = editor.snapshot();
        let prepared = self.prepare(&mut snapshot, name)?;
        Ok((prepared, snapshot))
    }

    pub fn discard_recovery(
        &self,
        info: &RecoveryCandidateInfo,
    ) -> Result<(), ProjectLibraryError> {
        self.validate_recovery_path(info)?;
        match fs::remove_file(&info.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(ProjectLibraryError::Io {
                path: info.path.clone(),
                message: error.to_string(),
            }),
        }
    }

    fn validate_recovery_path(
        &self,
        info: &RecoveryCandidateInfo,
    ) -> Result<(), ProjectLibraryError> {
        let Some(project_id) = info.project_id.as_ref() else {
            return Err(ProjectLibraryError::InvalidEntry(
                "recovery candidate has no valid project identity".to_owned(),
            ));
        };
        let expected = recovery_path(&self.root, project_id);
        if !paths_equivalent(&expected, &info.path) {
            return Err(ProjectLibraryError::InvalidEntry(
                "recovery candidate is outside the managed recovery area".to_owned(),
            ));
        }
        Ok(())
    }

    /// Rename an inactive managed project without changing its identity,
    /// directory, media locators, or internal IDs.
    pub fn rename(
        &self,
        entry: &ProjectLibraryEntry,
        name: &str,
    ) -> Result<Option<SavedDocument>, ProjectLibraryError> {
        validate_project_name(name)?;
        let mut loaded = self.open_entry(entry)?;
        if loaded.editor.project().name() == name {
            return Ok(None);
        }
        loaded
            .editor
            .execute(Command::RenameProject {
                name: name.to_owned(),
            })
            .map_err(|error| ProjectLibraryError::InvalidProject(error.to_string()))?;
        let generation = loaded
            .save_generation
            .checked_add(1)
            .ok_or_else(|| ProjectLibraryError::InvalidProject("generation overflow".to_owned()))?;
        let metadata = ProjectDocumentMetadata {
            project_id: loaded.project_id.clone(),
            created_at_unix_ms: loaded.created_at_unix_ms,
            save_generation: generation,
        };
        let saved = save_snapshot(
            &loaded.editor.snapshot(),
            &metadata,
            loaded.snapshot.path(),
            Some(&loaded.snapshot),
        )
        .map_err(ProjectLibraryError::Save)?;
        Ok(Some(saved))
    }

    /// Duplicate a saved project under a new identity. Internal IDs remain
    /// stable; relative locators are rebased to the original files by storing
    /// absolute paths. Media is never copied.
    pub fn duplicate(
        &self,
        entry: &ProjectLibraryEntry,
    ) -> Result<CreatedProject, ProjectLibraryError> {
        let loaded = self.open_entry(entry)?;
        let mut editor = loaded.editor.clone();
        rebase_relative_media(&mut editor, loaded.snapshot.path())?;
        let name = copy_name(loaded.editor.project().name());
        self.create(editor.snapshot(), &name)
    }
}

/// Resolve a persisted locator for reading. Project-relative paths are joined
/// only to the project file's directory; no traversal is accepted upstream.
pub fn resolve_media_locator(
    project_file: &Path,
    locator: &MediaLocator,
) -> Result<PathBuf, ProjectLibraryError> {
    match locator.kind() {
        capcut_engine::asset::MediaLocatorKind::ProjectRelative => {
            let base = project_file.parent().ok_or_else(|| {
                ProjectLibraryError::InvalidProject(
                    "project file has no parent directory".to_owned(),
                )
            })?;
            Ok(base.join(locator.value()))
        }
        capcut_engine::asset::MediaLocatorKind::Absolute => Ok(PathBuf::from(locator.value())),
    }
}

/// Choose a deterministic locator at import time. Only files inside the saved
/// project directory use project-relative paths.
pub fn locator_for_import(
    project_file: Option<&Path>,
    media_path: &Path,
) -> Result<MediaLocator, ProjectLibraryError> {
    let absolute = normalize_absolute(media_path)?;
    if let Some(project_file) = project_file {
        if let Some(project_dir) = project_file.parent() {
            if let (Ok(media_canonical), Ok(project_canonical)) =
                (fs::canonicalize(&absolute), fs::canonicalize(project_dir))
            {
                if let Ok(relative) = media_canonical.strip_prefix(&project_canonical) {
                    if !relative.as_os_str().is_empty() {
                        let normalized = relative
                            .components()
                            .filter_map(|component| match component {
                                Component::Normal(value) => {
                                    Some(value.to_string_lossy().into_owned())
                                }
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("/");
                        if !normalized.is_empty() {
                            return MediaLocator::project_relative(normalized).map_err(|error| {
                                ProjectLibraryError::InvalidProject(error.to_string())
                            });
                        }
                    }
                }
            }
        }
    }
    MediaLocator::absolute(absolute.to_string_lossy().into_owned())
        .map_err(|error| ProjectLibraryError::InvalidProject(error.to_string()))
}

pub fn paths_equivalent(left: &Path, right: &Path) -> bool {
    let left = comparable_path_text(left);
    let right = comparable_path_text(right);
    if cfg!(windows) {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

/// Convert project-relative media references to absolute locators pointing at
/// the original files. Used by duplicate, recovery restore, and conflict copy;
/// it never copies or rewrites media bytes.
fn rebase_relative_media(
    editor: &mut NativeEditor,
    project_file: &Path,
) -> Result<(), ProjectLibraryError> {
    let relative_assets = editor
        .project()
        .assets()
        .iter()
        .filter(|asset| {
            asset.locator().kind() == capcut_engine::asset::MediaLocatorKind::ProjectRelative
        })
        .map(|asset| {
            (
                asset.id(),
                asset.metadata().clone(),
                asset.file_size_bytes(),
                asset.locator().clone(),
            )
        })
        .collect::<Vec<_>>();
    for (asset, metadata, file_size, locator) in relative_assets {
        let resolved = resolve_media_locator(project_file, &locator)?;
        let absolute = MediaLocator::absolute(resolved.to_string_lossy().into_owned())
            .map_err(|error| ProjectLibraryError::InvalidProject(error.to_string()))?;
        editor
            .execute(Command::RelinkMediaAsset {
                asset,
                locator: absolute,
                metadata,
                file_size_bytes: file_size,
            })
            .map_err(|error| ProjectLibraryError::InvalidProject(error.to_string()))?;
    }
    Ok(())
}

fn comparable_path_text(path: &Path) -> String {
    let text = path.to_string_lossy();
    if cfg!(windows) {
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{rest}");
        }
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            return rest.to_owned();
        }
    }
    text.into_owned()
}

fn normalize_absolute(path: &Path) -> Result<PathBuf, ProjectLibraryError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .map_err(|error| ProjectLibraryError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }
}

fn copy_name(original: &str) -> String {
    let suffix = " copy";
    let limit = MAX_PROJECT_NAME_BYTES.saturating_sub(suffix.len());
    let mut base = String::new();
    for character in original.chars() {
        if base.len() + character.len_utf8() > limit {
            break;
        }
        base.push(character);
    }
    let mut name = format!("{base}{suffix}");
    if name.trim().is_empty() {
        name = "Project copy".to_owned();
    }
    name
}

fn recovered_copy_name(original: &str) -> String {
    let suffix = " (Recovered)";
    let limit = MAX_PROJECT_NAME_BYTES.saturating_sub(suffix.len());
    let mut base = String::new();
    for character in original.chars() {
        if base.len() + character.len_utf8() > limit {
            break;
        }
        base.push(character);
    }
    let mut name = format!("{base}{suffix}");
    if name.trim().is_empty() {
        name = "Recovered project".to_owned();
    }
    name
}

fn validate_project_name(name: &str) -> Result<(), ProjectLibraryError> {
    if name.trim().is_empty()
        || name.len() > MAX_PROJECT_NAME_BYTES
        || name.chars().any(char::is_control)
    {
        return Err(ProjectLibraryError::InvalidProjectName);
    }
    Ok(())
}

fn invalid_entry(
    directory: PathBuf,
    directory_name: String,
    message: String,
    project_id: Option<ProjectId>,
) -> ProjectLibraryEntry {
    ProjectLibraryEntry {
        path: directory.join(PROJECT_DIRECTORY_NAME),
        directory,
        directory_name,
        project_id,
        name: None,
        schema_version: None,
        created_at_unix_ms: 0,
        last_saved_unix_ms: 0,
        save_generation: None,
        status: ProjectEntryStatus::Invalid(message),
    }
}

fn system_time_millis(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

#[derive(Debug)]
pub enum ProjectLibraryError {
    RootUnavailable(String),
    Io { path: PathBuf, message: String },
    Open(ProjectOpenError),
    Save(ProjectSaveError),
    Recovery(RecoveryError),
    InvalidProject(String),
    InvalidProjectName,
    InvalidEntry(String),
    IdCollision,
}

impl fmt::Display for ProjectLibraryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootUnavailable(message) => {
                write!(
                    formatter,
                    "project library location is unavailable: {message}"
                )
            }
            Self::Io { path, message } => {
                write!(
                    formatter,
                    "project library I/O failed at {}: {message}",
                    path.display()
                )
            }
            Self::Open(error) => write!(formatter, "could not open project: {error}"),
            Self::Save(error) => write!(formatter, "could not save project: {error}"),
            Self::Recovery(error) => write!(formatter, "recovery failed: {error}"),
            Self::InvalidProject(message) => write!(formatter, "invalid project: {message}"),
            Self::InvalidProjectName => {
                formatter.write_str("project name must be 1..=256 bytes without control characters")
            }
            Self::InvalidEntry(name) => write!(formatter, "invalid project entry: {name}"),
            Self::IdCollision => {
                formatter.write_str("could not allocate a unique project directory")
            }
        }
    }
}

impl std::error::Error for ProjectLibraryError {}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use capcut_core::time::FrameRate;
    use capcut_engine::command::NativeEditor;
    use capcut_engine::project::Project;

    use super::*;

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    pub(crate) struct TempDir(pub(crate) PathBuf);

    impl TempDir {
        pub(crate) fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "cutcraft-library-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("temp");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    pub(crate) fn snapshot(name: &str) -> ProjectSnapshot {
        let project = Project::named(FrameRate::new(30, 1).expect("rate"), name).expect("project");
        NativeEditor::new(project).expect("editor").snapshot()
    }

    #[test]
    fn create_scan_rename_and_duplicate_are_identity_safe() {
        let temp = TempDir::new();
        let library = ProjectLibrary::new(&temp.0);
        let created = library
            .create(snapshot("Project A"), "Project A")
            .expect("create");
        assert_eq!(
            created.path,
            temp.0
                .join(created.project_id.to_string())
                .join(PROJECT_DIRECTORY_NAME)
        );
        let entries = library.scan().expect("scan");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name.as_deref(), Some("Project A"));
        assert_eq!(entries[0].project_id.as_ref(), Some(&created.project_id));
        assert!(entries[0].is_valid());

        let saved = library
            .rename(&entries[0], "Renamed")
            .expect("rename")
            .expect("changed");
        assert_eq!(saved.save_generation, 1);
        let renamed = library.scan().expect("scan");
        assert_eq!(renamed[0].name.as_deref(), Some("Renamed"));
        assert_eq!(renamed[0].path, created.path);
        assert_eq!(renamed[0].project_id.as_ref(), Some(&created.project_id));

        let copy = library.duplicate(&renamed[0]).expect("duplicate");
        assert_ne!(copy.project_id, created.project_id);
        assert_ne!(copy.path, created.path);
        assert_eq!(copy.name, "Renamed copy");
        assert!(created.path.exists());
        assert!(copy.path.exists());
        let reopened = library
            .open_entry(&library.scan().expect("scan")[0])
            .expect("open");
        assert!(reopened.editor.project().tracks().is_empty());
    }

    #[test]
    fn scan_reports_invalid_entries_and_empty_root_is_safe() {
        let temp = TempDir::new();
        let library = ProjectLibrary::new(temp.0.join("missing"));
        assert!(library.scan().expect("missing root").is_empty());

        let root = temp.0.join("projects");
        fs::create_dir_all(root.join("broken")).expect("broken");
        fs::write(root.join("broken").join(PROJECT_DIRECTORY_NAME), b"{bad").expect("write");
        fs::create_dir_all(root.join("empty")).expect("empty");
        fs::write(root.join("stray.txt"), b"x").expect("stray");
        let entries = ProjectLibrary::new(&root).scan().expect("scan");
        assert_eq!(entries.len(), 2);
        assert!(entries
            .iter()
            .all(|entry| matches!(entry.status, ProjectEntryStatus::Invalid(_))));
        assert!(entries.iter().all(|entry| !entry.is_valid()));
    }

    #[test]
    fn locator_policy_is_relative_inside_project_and_absolute_outside() {
        let temp = TempDir::new();
        let project_dir = temp.0.join("project");
        let media_dir = project_dir.join("media");
        fs::create_dir_all(&media_dir).expect("media");
        let project_file = project_dir.join(PROJECT_DIRECTORY_NAME);
        fs::write(&project_file, b"placeholder").expect("project");
        let inside = media_dir.join("clip.mp4");
        fs::write(&inside, b"x").expect("inside");
        let locator = locator_for_import(Some(&project_file), &inside).expect("locator");
        assert_eq!(
            locator.kind(),
            capcut_engine::asset::MediaLocatorKind::ProjectRelative
        );
        assert_eq!(locator.value(), "media/clip.mp4");
        let resolved = resolve_media_locator(&project_file, &locator).expect("resolve");
        assert_eq!(
            fs::canonicalize(resolved).unwrap(),
            fs::canonicalize(&inside).unwrap()
        );

        let outside_dir = temp.0.join("elsewhere");
        fs::create_dir_all(&outside_dir).expect("outside");
        let outside = outside_dir.join("clip.mp4");
        fs::write(&outside, b"x").expect("outside");
        let locator = locator_for_import(Some(&project_file), &outside).expect("locator");
        assert_eq!(
            locator.kind(),
            capcut_engine::asset::MediaLocatorKind::Absolute
        );
    }

    #[test]
    fn rename_rejects_empty_names_without_touching_the_file() {
        let temp = TempDir::new();
        let library = ProjectLibrary::new(&temp.0);
        let created = library
            .create(snapshot("Original"), "Original")
            .expect("create");
        let before = fs::read(&created.path).expect("read");
        let entries = library.scan().expect("scan");
        assert!(matches!(
            library.rename(&entries[0], "  "),
            Err(ProjectLibraryError::InvalidProjectName)
        ));
        assert_eq!(fs::read(&created.path).expect("read"), before);
    }
}

#[cfg(test)]
mod media_locator_tests {
    use super::*;
    use capcut_engine::asset::{MediaAssetMetadata, MediaAssetRegistration, MediaRational};
    use capcut_engine::command::Command;

    use super::tests::{snapshot, TempDir};

    fn metadata() -> MediaAssetMetadata {
        MediaAssetMetadata {
            container: "mov,mp4".to_owned(),
            video_stream_index: 0,
            codec: "h264".to_owned(),
            width: 1920,
            height: 1080,
            pixel_format: Some("yuv420p".to_owned()),
            timebase_numerator: 1,
            timebase_denominator: 15_360,
            stream_start_ticks: 0,
            duration_ticks: Some(2 * 15_360),
            nominal_frame_rate: MediaRational::new(30, 1),
            average_frame_rate: MediaRational::new(30, 1),
            rotation_degrees: 0,
            has_audio: false,
            frame_count_hint: None,
        }
    }

    #[test]
    fn duplicate_rebases_relative_media_to_absolute_paths_without_copying() {
        let temp = TempDir::new();
        let library = ProjectLibrary::new(&temp.0);
        let created = library
            .create(snapshot("Relative"), "Relative")
            .expect("create");
        let media_dir = created.directory.join("media");
        fs::create_dir_all(&media_dir).expect("media dir");
        let media_path = media_dir.join("clip.mp4");
        fs::write(&media_path, b"not-real-video").expect("media placeholder");

        let entry = library.scan().expect("scan")[0].clone();
        let mut loaded = library.open_entry(&entry).expect("open");
        let registration = MediaAssetRegistration::video(
            "clip.mp4",
            MediaLocator::project_relative("media/clip.mp4").expect("relative locator"),
            metadata(),
        )
        .expect("registration");
        let asset = loaded
            .editor
            .execute(Command::AddMediaAsset {
                asset: registration,
            })
            .expect("asset")
            .created_asset
            .expect("asset id");
        loaded
            .editor
            .execute(Command::AppendMediaClip { asset })
            .expect("append");
        let save_metadata = crate::schema::ProjectDocumentMetadata {
            project_id: loaded.project_id.clone(),
            created_at_unix_ms: loaded.created_at_unix_ms,
            save_generation: 1,
        };
        save_snapshot(
            &loaded.editor.snapshot(),
            &save_metadata,
            &created.path,
            Some(&loaded.snapshot),
        )
        .expect("save asset project");

        let entry = library.scan().expect("rescan")[0].clone();
        let copy = library.duplicate(&entry).expect("duplicate");
        let copied = open(&copy.path).expect("open duplicate");
        let copied_asset = &copied.editor.project().assets()[0];
        assert_eq!(
            copied_asset.locator().kind(),
            capcut_engine::asset::MediaLocatorKind::Absolute
        );
        assert_eq!(PathBuf::from(copied_asset.locator().value()), media_path);
        assert!(media_path.exists(), "duplicate must not move or copy media");
        assert_ne!(copy.directory, created.directory);
        assert_eq!(
            loaded.editor.project().assets()[0].locator().kind(),
            capcut_engine::asset::MediaLocatorKind::ProjectRelative,
            "original locator must not be rewritten"
        );
    }

    #[test]
    fn moving_the_library_root_keeps_relative_media_resolvable() {
        let temp = TempDir::new();
        let library = ProjectLibrary::new(&temp.0);
        let created = library.create(snapshot("Moved"), "Moved").expect("create");
        let media_dir = created.directory.join("media");
        fs::create_dir_all(&media_dir).expect("media dir");
        let media_path = media_dir.join("clip.mp4");
        fs::write(&media_path, b"not-real-video").expect("media placeholder");

        let entry = library.scan().expect("scan")[0].clone();
        let mut loaded = library.open_entry(&entry).expect("open");
        let registration = MediaAssetRegistration::video(
            "clip.mp4",
            MediaLocator::project_relative("media/clip.mp4").expect("relative locator"),
            metadata(),
        )
        .expect("registration");
        let asset = loaded
            .editor
            .execute(Command::AddMediaAsset {
                asset: registration,
            })
            .expect("asset")
            .created_asset
            .expect("asset id");
        loaded
            .editor
            .execute(Command::AppendMediaClip { asset })
            .expect("append");
        let save_metadata = crate::schema::ProjectDocumentMetadata {
            project_id: loaded.project_id.clone(),
            created_at_unix_ms: loaded.created_at_unix_ms,
            save_generation: 1,
        };
        save_snapshot(
            &loaded.editor.snapshot(),
            &save_metadata,
            &created.path,
            Some(&loaded.snapshot),
        )
        .expect("save relative asset project");

        let moved_root = TempDir::new();
        let moved_directory = moved_root.0.join(created.project_id.as_str());
        fs::rename(&created.directory, &moved_directory).expect("move project directory");
        assert!(!created.directory.exists());

        let moved_library = ProjectLibrary::new(&moved_root.0);
        let moved_entries = moved_library.scan().expect("scan moved root");
        assert_eq!(moved_entries.len(), 1);
        let moved = moved_library
            .open_entry(&moved_entries[0])
            .expect("open moved project");
        assert_eq!(moved.project_id, created.project_id);
        assert_eq!(moved.editor.project().assets().len(), 1);
        let locator = moved.editor.project().assets()[0].locator();
        assert_eq!(
            locator.kind(),
            capcut_engine::asset::MediaLocatorKind::ProjectRelative
        );
        let resolved =
            resolve_media_locator(&moved_entries[0].path, locator).expect("resolve moved");
        assert_eq!(resolved, moved_directory.join("media").join("clip.mp4"));
        assert!(resolved.exists());
    }

    #[test]
    fn windows_verbatim_prefixes_compare_equivalent() {
        if !cfg!(windows) {
            return;
        }
        assert!(paths_equivalent(
            Path::new(r"\\?\C:\Media\Clip.mp4"),
            Path::new(r"C:\Media\Clip.mp4")
        ));
        assert!(paths_equivalent(
            Path::new(r"\\?\UNC\server\share\clip.mp4"),
            Path::new(r"\\server\share\clip.mp4")
        ));
        assert!(!paths_equivalent(
            Path::new(r"C:\Media\A.mp4"),
            Path::new(r"C:\Media\B.mp4")
        ));
    }
}
