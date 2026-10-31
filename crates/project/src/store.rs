//! Bounded, staged, conflict-aware native project IO.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use capcut_engine::command::NativeEditor;
use capcut_engine::project::ProjectSnapshot;
use sha2::{Digest, Sha256};

use crate::error::{ProjectOpenError, ProjectSaveError};
use crate::id::ProjectId;
use crate::schema::{
    parse_project_bytes, parse_staged_v2, serialize_project, ParsedProjectFile,
    ProjectDocumentMetadata, ProjectFileV2, MAX_PROJECT_FILE_BYTES,
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
const STAGING_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_CLEANUP_ENTRIES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSnapshot {
    path: PathBuf,
    sha256: String,
}

impl FileSnapshot {
    fn new(path: PathBuf, sha256: String) -> Self {
        Self { path, sha256 }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub fn matches_path(&self, path: &Path) -> bool {
        paths_equivalent(path, &self.path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProject {
    pub project_id: ProjectId,
    pub editor: NativeEditor,
    pub snapshot: FileSnapshot,
    pub schema_version: u32,
    pub created_at_unix_ms: u64,
    pub last_saved_unix_ms: u64,
    pub save_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedDocument {
    pub snapshot: FileSnapshot,
    pub save_generation: u64,
    pub last_saved_unix_ms: u64,
}

/// Read and fully validate one native project file.
pub fn open(path: &Path) -> Result<LoadedProject, ProjectOpenError> {
    if !has_project_extension(path) {
        return Err(ProjectOpenError::InvalidExtension(path.to_path_buf()));
    }
    cleanup_abandoned_staging_files(path);
    let metadata = fs::metadata(path).map_err(|error| ProjectOpenError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    if !metadata.is_file() {
        return Err(ProjectOpenError::NotAFile(path.to_path_buf()));
    }
    let (fallback_created, fallback_last_saved) = file_timestamps(&metadata);
    let bytes = read_bounded(path, MAX_PROJECT_FILE_BYTES)?;
    let opened = parse_project_bytes(&bytes)?.into_opened(fallback_created, fallback_last_saved)?;
    let snapshot = FileSnapshot::new(path.to_path_buf(), sha256_hex(&bytes));
    Ok(LoadedProject {
        project_id: opened.project_id,
        editor: opened.editor,
        snapshot,
        schema_version: opened.schema_version,
        created_at_unix_ms: opened.created_at_unix_ms,
        last_saved_unix_ms: opened.last_saved_unix_ms,
        save_generation: opened.save_generation,
    })
}

/// Save a live editor through the snapshot API. The UI uses
/// `save_snapshot` so background workers never borrow a live session.
pub fn save(
    editor: &NativeEditor,
    project_id: &ProjectId,
    target: &Path,
    allowed_existing: Option<&FileSnapshot>,
) -> Result<FileSnapshot, ProjectSaveError> {
    let metadata = ProjectDocumentMetadata {
        project_id: project_id.clone(),
        created_at_unix_ms: now_unix_ms(),
        save_generation: 0,
    };
    save_snapshot(&editor.snapshot(), &metadata, target, allowed_existing)
        .map(|saved| saved.snapshot)
}

/// Serialize one immutable snapshot to the schema-v2 document bytes. This is a
/// diagnostic/benchmark helper; `save_snapshot` remains the persistence path
/// with validation, staging, and atomic publication.
pub fn serialize_snapshot(
    snapshot: &ProjectSnapshot,
    metadata: &ProjectDocumentMetadata,
    last_saved_unix_ms: u64,
) -> Result<Vec<u8>, ProjectSaveError> {
    let file = ProjectFileV2::from_snapshot(snapshot, metadata, last_saved_unix_ms);
    serialize_project(&file)
}

/// Save one immutable snapshot with conflict checks and staged validation.
pub fn save_snapshot(
    snapshot: &ProjectSnapshot,
    metadata: &ProjectDocumentMetadata,
    target: &Path,
    allowed_existing: Option<&FileSnapshot>,
) -> Result<SavedDocument, ProjectSaveError> {
    if !has_project_extension(target) {
        return Err(ProjectSaveError::InvalidExtension(target.to_path_buf()));
    }
    let validation_editor =
        NativeEditor::from_restored(snapshot.project.clone(), snapshot.magnet_enabled)
            .map_err(|error| ProjectSaveError::InvalidSnapshot(error.to_string()))?;
    validation_editor
        .validate_magnet_state()
        .map_err(ProjectSaveError::InvalidMagnet)?;

    match allowed_existing {
        Some(snapshot) => {
            if !paths_equivalent(target, snapshot.path()) {
                return Err(ProjectSaveError::TargetExists(target.to_path_buf()));
            }
            match fs::metadata(target) {
                Ok(metadata) if metadata.is_file() => {}
                Ok(_) => return Err(ProjectSaveError::NotAFile(target.to_path_buf())),
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    return Err(ProjectSaveError::ExternalMissing(target.to_path_buf()));
                }
                Err(error) => {
                    return Err(ProjectSaveError::Io {
                        path: target.to_path_buf(),
                        message: error.to_string(),
                    })
                }
            }
            let current = read_bounded_save(target)?;
            if sha256_hex(&current) != snapshot.sha256() {
                return Err(ProjectSaveError::ExternalModified(target.to_path_buf()));
            }
        }
        None => match fs::metadata(target) {
            Ok(metadata) if metadata.is_file() => {
                return Err(ProjectSaveError::TargetExists(target.to_path_buf()));
            }
            Ok(_) => return Err(ProjectSaveError::NotAFile(target.to_path_buf())),
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(ProjectSaveError::Io {
                    path: target.to_path_buf(),
                    message: error.to_string(),
                })
            }
        },
    }

    let last_saved_unix_ms = now_unix_ms();
    let file = ProjectFileV2::from_snapshot(snapshot, metadata, last_saved_unix_ms);
    let bytes = serialize_project(&file)?;
    if bytes.len() as u64 > MAX_PROJECT_FILE_BYTES {
        return Err(ProjectSaveError::TooLarge {
            size: bytes.len(),
            limit: MAX_PROJECT_FILE_BYTES,
        });
    }
    let parent = target.parent().unwrap_or_else(|| Path::new(""));
    if !parent.as_os_str().is_empty() {
        fs::create_dir_all(parent).map_err(|error| ProjectSaveError::Io {
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    cleanup_abandoned_staging_files(target);

    let temporary = create_stage(target, &bytes)?;
    let verification = verify_stage(&temporary, &file, snapshot);
    if let Err(error) = verification {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temporary, target) {
        let _ = fs::remove_file(&temporary);
        return Err(ProjectSaveError::Io {
            path: target.to_path_buf(),
            message: error.to_string(),
        });
    }
    sync_parent_best_effort(parent);
    Ok(SavedDocument {
        snapshot: FileSnapshot::new(target.to_path_buf(), sha256_hex(&bytes)),
        save_generation: metadata.save_generation,
        last_saved_unix_ms,
    })
}

fn verify_stage(
    temporary: &Path,
    intended: &ProjectFileV2,
    snapshot: &ProjectSnapshot,
) -> Result<(), ProjectSaveError> {
    let bytes = read_bounded_save(temporary)?;
    let staged = parse_staged_v2(&bytes)?;
    if &staged != intended {
        return Err(ProjectSaveError::CompareFailed);
    }
    let opened = ParsedProjectFile::V2(staged)
        .into_opened(0, 0)
        .map_err(staged_error)?;
    if opened.project_id != intended_project_id(intended)?
        || opened.editor.magnet_enabled() != snapshot.magnet_enabled
        || opened.editor.project().tracks() != snapshot.project.tracks()
        || opened.editor.project().assets() != snapshot.project.assets()
        || opened.editor.project().name() != snapshot.project.name()
        || opened.editor.project().fps() != snapshot.project.fps()
        || opened.editor.project().main_video_track() != snapshot.project.main_video_track()
        || opened.editor.project().next_track_id() != snapshot.project.next_track_id()
        || opened.editor.project().next_clip_id() != snapshot.project.next_clip_id()
        || opened.editor.project().next_asset_id() != snapshot.project.next_asset_id()
    {
        return Err(ProjectSaveError::StagedVerification(
            "staged editor state does not match the saved snapshot".to_owned(),
        ));
    }
    Ok(())
}

fn intended_project_id(file: &ProjectFileV2) -> Result<ProjectId, ProjectSaveError> {
    ProjectId::parse(file.project_id.clone())
        .map_err(|error| ProjectSaveError::StagedVerification(error.to_string()))
}

/// Best-effort removal of abandoned staging files for one target. Only our
/// deterministic `. <target>.<pid>.<counter>.tmp` shape older than 24 hours is
/// considered; recent files may belong to another writer.
pub(crate) fn cleanup_abandoned_staging_files(target: &Path) -> usize {
    let Some(parent) = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return 0;
    };
    let Some(name) = target.file_name() else {
        return 0;
    };
    let prefix = format!(".{}.", name.to_string_lossy());
    let Ok(entries) = fs::read_dir(parent) else {
        return 0;
    };
    let now = SystemTime::now();
    let mut removed = 0usize;
    for entry in entries.take(MAX_CLEANUP_ENTRIES).flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if !file_name.starts_with(&prefix) || !file_name.ends_with(".tmp") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        let Ok(age) = now.duration_since(modified) else {
            continue;
        };
        if age >= STAGING_MAX_AGE && fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

fn create_stage(target: &Path, bytes: &[u8]) -> Result<PathBuf, ProjectSaveError> {
    for _ in 0..8 {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temporary = temporary_path(target, counter);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(mut file) => {
                let result = (|| -> std::io::Result<()> {
                    file.write_all(bytes)?;
                    file.sync_all()?;
                    Ok(())
                })();
                if let Err(error) = result {
                    let _ = fs::remove_file(&temporary);
                    return Err(ProjectSaveError::Io {
                        path: temporary,
                        message: error.to_string(),
                    });
                }
                return Ok(temporary);
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(ProjectSaveError::Io {
                    path: temporary,
                    message: error.to_string(),
                })
            }
        }
    }
    Err(ProjectSaveError::Io {
        path: target.to_path_buf(),
        message: "could not allocate a unique staging file".to_owned(),
    })
}

fn temporary_path(target: &Path, counter: u64) -> PathBuf {
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project.cutcraft".to_owned());
    let parent = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_default();
    parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), counter))
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, ProjectOpenError> {
    let metadata = fs::metadata(path).map_err(|error| ProjectOpenError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    if !metadata.is_file() {
        return Err(ProjectOpenError::NotAFile(path.to_path_buf()));
    }
    if metadata.len() > limit {
        return Err(ProjectOpenError::TooLarge {
            size: metadata.len(),
            limit,
        });
    }
    let file = File::open(path).map_err(|error| ProjectOpenError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| ProjectOpenError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    if bytes.len() as u64 > limit {
        return Err(ProjectOpenError::TooLarge {
            size: bytes.len() as u64,
            limit,
        });
    }
    Ok(bytes)
}

fn read_bounded_save(path: &Path) -> Result<Vec<u8>, ProjectSaveError> {
    let metadata = fs::metadata(path).map_err(|error| ProjectSaveError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    if !metadata.is_file() {
        return Err(ProjectSaveError::NotAFile(path.to_path_buf()));
    }
    if metadata.len() > MAX_PROJECT_FILE_BYTES {
        return Err(ProjectSaveError::TooLarge {
            size: metadata.len() as usize,
            limit: MAX_PROJECT_FILE_BYTES,
        });
    }
    let file = File::open(path).map_err(|error| ProjectSaveError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_PROJECT_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| ProjectSaveError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    Ok(bytes)
}

fn file_timestamps(metadata: &fs::Metadata) -> (u64, u64) {
    let last_saved = metadata
        .modified()
        .ok()
        .and_then(system_time_millis)
        .unwrap_or(0);
    let created = metadata
        .created()
        .ok()
        .and_then(system_time_millis)
        .unwrap_or(last_saved);
    (created, last_saved)
}

fn system_time_millis(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

pub(crate) fn now_unix_ms() -> u64 {
    system_time_millis(SystemTime::now()).unwrap_or(0)
}

fn staged_error(error: ProjectOpenError) -> ProjectSaveError {
    ProjectSaveError::StagedVerification(error.to_string())
}

fn has_project_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("cutcraft"))
}

fn paths_equivalent(first: &Path, second: &Path) -> bool {
    if first == second {
        return true;
    }
    match (fs::canonicalize(first), fs::canonicalize(second)) {
        (Ok(first), Ok(second)) => first == second,
        _ => resolved_for_compare(first) == resolved_for_compare(second),
    }
}

fn resolved_for_compare(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?;
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let parent = if parent.as_os_str().is_empty() {
        std::env::current_dir().ok()?
    } else {
        fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf())
    };
    Some(parent.join(name))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(unix)]
fn sync_parent_best_effort(parent: &Path) {
    if let Ok(directory) = File::open(parent) {
        let _ = directory.sync_all();
    }
}

#[cfg(not(unix))]
fn sync_parent_best_effort(_parent: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use capcut_core::time::{
        FrameRate, SourceRange, SourceTicks, Timebase, TimelineRange, TimelineTicks,
    };
    use capcut_engine::command::Command;
    use capcut_engine::project::{
        ClipId, ClipKind, ClipSpec, Project, SourceMapping, TrackId, TrackKind,
    };
    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "cutcraft-m3a-{}-{}",
                std::process::id(),
                TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).expect("temp dir");
            Self(path)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn rate() -> FrameRate {
        FrameRate::new(30, 1).expect("fps")
    }

    fn ticks(value: i64) -> TimelineTicks {
        TimelineTicks::new(value)
    }

    fn range(start: i64, end: i64) -> TimelineRange {
        TimelineRange::new(ticks(start), ticks(end)).expect("range")
    }

    fn source(duration: i64) -> SourceMapping {
        SourceMapping::new(
            SourceRange::new(SourceTicks::ZERO, SourceTicks::new(duration)).expect("available"),
            SourceRange::new(SourceTicks::ZERO, SourceTicks::new(duration)).expect("used"),
            Timebase::new(120_000).expect("timebase"),
            1,
            1,
        )
        .expect("source")
    }

    fn spec(name: &str, start: i64, end: i64) -> ClipSpec {
        ClipSpec {
            name: name.to_owned(),
            kind: ClipKind::Video,
            range: range(start, end),
            source: Some(source(end - start)),
            media_asset_id: None,
            linked_clip_ids: BTreeSet::new(),
            nested: false,
        }
    }

    fn project_abc() -> Project {
        let mut project = Project::named(rate(), "Project ABC").expect("project");
        let track = project
            .add_track("V1", TrackKind::Video, false)
            .expect("track");
        project.assign_main_track(track).expect("main");
        project.add_clip(track, spec("A", 0, 600_000)).expect("A");
        project
            .add_clip(track, spec("B", 1_200_000, 1_800_000))
            .expect("B");
        project
            .add_clip(track, spec("C", 2_400_000, 3_000_000))
            .expect("C");
        project
    }

    fn ranges(editor: &NativeEditor) -> Vec<(ClipId, i64, i64)> {
        editor
            .project()
            .main_track()
            .expect("main")
            .clips()
            .iter()
            .map(|clip| {
                (
                    clip.id(),
                    clip.range().start().get(),
                    clip.range().end().get(),
                )
            })
            .collect()
    }

    fn save_current(
        editor: &NativeEditor,
        id: &ProjectId,
        path: &Path,
        snapshot: Option<&FileSnapshot>,
    ) -> FileSnapshot {
        save(editor, id, path, snapshot).expect("save")
    }

    #[test]
    fn save_is_read_only_for_history_selection_revision_and_magnet() {
        let dir = TempDir::new();
        let path = dir.path("read-only-save.cutcraft");
        let id = ProjectId::generate();
        let mut editor = NativeEditor::new(project_abc()).expect("editor");
        editor.enable_magnet().expect("activate");
        editor
            .execute(Command::DeleteClip {
                clip: ClipId::new(2),
            })
            .expect("delete");
        editor.select_clip(ClipId::new(3));
        let revision = editor.project().revision();
        let selection = editor.selection().ids().clone();
        let can_undo = editor.can_undo();
        let can_redo = editor.can_redo();
        let magnet = editor.magnet_enabled();

        let first = save_current(&editor, &id, &path, None);
        assert_eq!(editor.project().revision(), revision);
        assert_eq!(editor.selection().ids(), &selection);
        assert_eq!(editor.can_undo(), can_undo);
        assert_eq!(editor.can_redo(), can_redo);
        assert_eq!(editor.magnet_enabled(), magnet);

        editor.undo().expect("undo");
        let redo_available = editor.can_redo();
        save_current(&editor, &id, &path, Some(&first));
        assert_eq!(editor.can_redo(), redo_available, "save preserves redo");
        assert_eq!(editor.project().revision(), revision + 1);

        let loaded = open(&path).expect("open");
        assert!(loaded.editor.selection().ids().is_empty());
        assert!(!loaded.editor.can_undo());
        assert!(!loaded.editor.can_redo());
    }

    #[test]
    fn empty_project_round_trip() {
        let dir = TempDir::new();
        let path = dir.path("empty.cutcraft");
        let id = ProjectId::generate();
        let editor = NativeEditor::new(Project::empty(rate()).expect("project")).expect("editor");
        let snapshot = save_current(&editor, &id, &path, None);
        let loaded = open(&path).expect("open");
        assert_eq!(loaded.project_id, id);
        assert_eq!(loaded.snapshot.sha256(), snapshot.sha256());
        assert!(loaded.editor.project().tracks().is_empty());
        assert_eq!(loaded.editor.project().next_track_id(), 1);
        assert_eq!(loaded.editor.project().next_clip_id(), 1);
        assert!(!loaded.editor.magnet_enabled());
        assert!(!loaded.editor.can_undo());
        assert!(!loaded.editor.can_redo());
    }

    #[test]
    fn magnet_cases_a_b_c_d_round_trip_exact_state() {
        let dir = TempDir::new();
        let path = dir.path("magnet.cutcraft");
        let id = ProjectId::generate();
        let mut editor = NativeEditor::new(project_abc()).expect("editor");
        let original_gaps = ranges(&editor);
        let revision = editor.project().revision();
        let first = save_current(&editor, &id, &path, None);
        assert_eq!(editor.project().revision(), revision, "save must not edit");
        assert!(!editor.magnet_enabled(), "save must not change Magnet");

        let loaded = open(&path).expect("open OFF");
        assert_eq!(ranges(&loaded.editor), original_gaps);
        assert!(!loaded.editor.magnet_enabled());

        editor.enable_magnet().expect("activate");
        let compacted = ranges(&editor);
        assert_eq!(
            compacted,
            vec![
                (ClipId::new(1), 0, 600_000),
                (ClipId::new(2), 600_000, 1_200_000),
                (ClipId::new(3), 1_200_000, 1_800_000)
            ]
        );
        let second = save_current(&editor, &id, &path, Some(&first));
        let loaded = open(&path).expect("open ON");
        assert_eq!(ranges(&loaded.editor), compacted);
        assert!(loaded.editor.magnet_enabled());
        assert!(!loaded.editor.can_undo());

        editor.undo().expect("undo activation");
        let third = save_current(&editor, &id, &path, Some(&second));
        let loaded = open(&path).expect("open after undo");
        assert_eq!(ranges(&loaded.editor), original_gaps);
        assert!(!loaded.editor.magnet_enabled());

        editor.redo().expect("redo activation");
        save_current(&editor, &id, &path, Some(&third));
        let loaded = open(&path).expect("open after redo");
        assert_eq!(ranges(&loaded.editor), compacted);
        assert!(loaded.editor.magnet_enabled());
    }

    #[test]
    fn populated_round_trip_preserves_tracks_links_locks_and_exact_ticks() {
        let dir = TempDir::new();
        let path = dir.path("rich.cutcraft");
        let id = ProjectId::generate();
        let mut project =
            Project::named(FrameRate::new(24_000, 1_001).expect("fps"), "Rich").expect("project");
        let main = project
            .add_track("V1", TrackKind::Video, false)
            .expect("main track");
        project.assign_main_track(main).expect("main");
        let a = project.add_clip(main, spec("A", 4000, 28004)).expect("A");
        let audio = project
            .add_track("A1", TrackKind::Audio, true)
            .expect("audio");
        let mut linked = spec("Audio", 0, 24004);
        linked.kind = ClipKind::Audio;
        linked.linked_clip_ids.insert(a);
        let audio_clip = project.add_clip(audio, linked).expect("audio clip");
        let overlay = project
            .add_track("V2", TrackKind::Video, false)
            .expect("overlay");
        let mut nested = ClipSpec {
            name: "Nested".to_owned(),
            kind: ClipKind::NestedComposition,
            range: range(4000, 16004),
            source: None,
            media_asset_id: None,
            linked_clip_ids: BTreeSet::new(),
            nested: true,
        };
        nested.linked_clip_ids.insert(audio_clip);
        project.add_clip(overlay, nested).expect("nested");
        let expected_tracks = project.tracks().to_vec();
        let expected_main = project.main_video_track();
        let expected_track_allocator = project.next_track_id();
        let expected_clip_allocator = project.next_clip_id();
        let editor = NativeEditor::new(project).expect("editor");

        let snapshot = save_current(&editor, &id, &path, None);
        let loaded = open(&path).expect("open");
        assert_eq!(loaded.project_id, id);
        assert_eq!(loaded.snapshot.sha256(), snapshot.sha256());
        assert_eq!(loaded.editor.project().tracks(), expected_tracks.as_slice());
        assert_eq!(loaded.editor.project().main_video_track(), expected_main);
        assert_eq!(
            loaded.editor.project().next_track_id(),
            expected_track_allocator
        );
        assert_eq!(
            loaded.editor.project().next_clip_id(),
            expected_clip_allocator
        );
        assert_eq!(loaded.editor.project().name(), "Rich");
        assert_eq!(
            loaded.editor.project().fps(),
            FrameRate::new(24_000, 1_001).expect("fps")
        );
        assert_eq!(
            loaded
                .editor
                .project()
                .clip(a)
                .expect("A")
                .source()
                .expect("source")
                .used(),
            SourceRange::new(SourceTicks::new(0), SourceTicks::new(24004)).expect("used")
        );
        assert!(loaded
            .editor
            .project()
            .track(TrackId::new(2))
            .expect("audio")
            .locked());
    }

    #[test]
    fn reopen_continue_editing_uses_persisted_allocator_and_fresh_history() {
        let dir = TempDir::new();
        let path = dir.path("edit.cutcraft");
        let id = ProjectId::generate();
        let mut editor = NativeEditor::new(project_abc()).expect("editor");
        editor.enable_magnet().expect("activate");
        let first = save_current(&editor, &id, &path, None);
        drop(editor);

        let mut loaded = open(&path).expect("open").editor;
        assert!(!loaded.can_undo());
        assert!(!loaded.can_redo());
        let next_id = loaded.project().next_clip_id();
        let outcome = loaded
            .execute(Command::SplitClip {
                clip: ClipId::new(2),
                at: ticks(900_000),
            })
            .expect("split after reopen");
        assert!(outcome.changed);
        assert_eq!(
            loaded.project().next_clip_id(),
            next_id + 1,
            "new IDs must continue above the persisted high-water mark"
        );
        let split_state = ranges(&loaded);
        loaded.undo().expect("undo in reopened session");
        assert_eq!(loaded.project().next_clip_id(), next_id + 1);
        loaded.redo().expect("redo in reopened session");
        assert_eq!(ranges(&loaded), split_state);
        let _ = first;
    }

    #[test]
    fn save_after_trim_and_delete_round_trips_committed_state() {
        let dir = TempDir::new();
        let path = dir.path("edits.cutcraft");
        let id = ProjectId::generate();
        let mut editor = NativeEditor::new(project_abc()).expect("editor");
        editor.enable_magnet().expect("activate");
        editor
            .execute(Command::DeleteClip {
                clip: ClipId::new(2),
            })
            .expect("delete");
        editor
            .execute(Command::TrimClipEnd {
                clip: ClipId::new(3),
                new_end: ticks(900_000),
            })
            .expect("trim");
        let expected = ranges(&editor);
        let expected_sources = editor
            .project()
            .tracks()
            .iter()
            .flat_map(|track| track.clips())
            .map(|clip| (clip.id(), clip.source().cloned()))
            .collect::<Vec<_>>();
        save_current(&editor, &id, &path, None);
        let loaded = open(&path).expect("open");
        assert_eq!(ranges(&loaded.editor), expected);
        assert!(loaded.editor.magnet_enabled());
        assert_eq!(
            loaded
                .editor
                .project()
                .tracks()
                .iter()
                .flat_map(|track| track.clips())
                .map(|clip| (clip.id(), clip.source().cloned()))
                .collect::<Vec<_>>(),
            expected_sources
        );
    }

    #[test]
    fn target_conflicts_and_external_changes_are_reported_without_overwrite() {
        let dir = TempDir::new();
        let path = dir.path("conflict.cutcraft");
        let id = ProjectId::generate();
        let editor = NativeEditor::new(project_abc()).expect("editor");
        fs::write(&path, b"not a project").expect("seed");
        assert_eq!(
            save(&editor, &id, &path, None),
            Err(ProjectSaveError::TargetExists(path.clone()))
        );
        assert_eq!(fs::read(&path).expect("read"), b"not a project");

        let other = dir.path("other.cutcraft");
        let snapshot = save_current(&editor, &id, &other, None);
        fs::write(&other, b"externally changed").expect("external edit");
        assert_eq!(
            save(&editor, &id, &other, Some(&snapshot)),
            Err(ProjectSaveError::ExternalModified(other.clone()))
        );
        assert_eq!(fs::read(&other).expect("read"), b"externally changed");

        fs::remove_file(&other).expect("remove");
        assert_eq!(
            save(&editor, &id, &other, Some(&snapshot)),
            Err(ProjectSaveError::ExternalMissing(other.clone()))
        );
    }

    #[test]
    fn failed_staging_or_directory_creation_leaves_files_untouched() {
        let dir = TempDir::new();
        let blocker = dir.path("blocker");
        fs::write(&blocker, b"keep").expect("blocker");
        let target = blocker.join("nested.cutcraft");
        let editor = NativeEditor::new(project_abc()).expect("editor");
        let error = save(&editor, &ProjectId::generate(), &target, None).expect_err("must fail");
        assert!(matches!(error, ProjectSaveError::Io { .. }));
        assert_eq!(fs::read(&blocker).expect("read"), b"keep");
        assert!(!dir.path("nested.cutcraft").exists());
    }

    #[test]
    fn save_rejects_wrong_extension() {
        let dir = TempDir::new();
        let editor = NativeEditor::new(project_abc()).expect("editor");
        let id = ProjectId::generate();
        let error = save(&editor, &id, &dir.path("project.json"), None).expect_err("extension");
        assert!(matches!(error, ProjectSaveError::InvalidExtension(_)));
        assert!(!dir.path("project.json").exists());
    }

    #[test]
    fn open_rejects_bad_inputs_and_magnet_gaps() {
        let dir = TempDir::new();
        let id = ProjectId::generate();
        let editor = NativeEditor::new(project_abc()).expect("editor");
        let valid = dir.path("valid.cutcraft");
        save_current(&editor, &id, &valid, None);
        let valid_bytes = fs::read(&valid).expect("read");

        let missing_extension = dir.path("missing.txt");
        assert_eq!(
            open(&missing_extension),
            Err(ProjectOpenError::InvalidExtension(missing_extension))
        );
        assert!(matches!(
            open(&dir.path("missing.cutcraft")),
            Err(ProjectOpenError::Io { .. })
        ));

        let directory = dir.path("dir.cutcraft");
        fs::create_dir(&directory).expect("dir");
        assert_eq!(open(&directory), Err(ProjectOpenError::NotAFile(directory)));

        let truncated = dir.path("truncated.cutcraft");
        fs::write(&truncated, &valid_bytes[..valid_bytes.len() / 2]).expect("write");
        assert!(matches!(
            open(&truncated),
            Err(ProjectOpenError::InvalidJson(_))
        ));

        let future = dir.path("future.cutcraft");
        fs::write(
            &future,
            br#"{"format":"cutcraft-project","schema_version":3}"#,
        )
        .expect("write");
        assert_eq!(
            open(&future),
            Err(ProjectOpenError::UnsupportedSchemaVersion(3))
        );

        let deep = dir.path("deep.cutcraft");
        fs::write(&deep, b"[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[").expect("write");
        assert_eq!(
            open(&deep),
            Err(ProjectOpenError::JsonDepthExceeded { limit: 32 })
        );

        let oversized = dir.path("oversized.cutcraft");
        let file = File::create(&oversized).expect("create");
        file.set_len(MAX_PROJECT_FILE_BYTES + 1).expect("sparse");
        drop(file);
        assert_eq!(
            open(&oversized),
            Err(ProjectOpenError::TooLarge {
                size: MAX_PROJECT_FILE_BYTES + 1,
                limit: MAX_PROJECT_FILE_BYTES
            })
        );

        let linked = dir.path("linked.cutcraft");
        let mut value: serde_json::Value = serde_json::from_slice(&valid_bytes).expect("json");
        value["tracks"][0]["clips"][0]["linked_clip_ids"] = serde_json::json!([999]);
        fs::write(&linked, serde_json::to_vec(&value).expect("encode")).expect("write");
        assert!(matches!(
            open(&linked),
            Err(ProjectOpenError::Project(
                capcut_engine::project::ProjectError::UnknownLinkedClip(_)
            ))
        ));

        let duplicate = dir.path("duplicate.cutcraft");
        let mut value: serde_json::Value = serde_json::from_slice(&valid_bytes).expect("json");
        value["tracks"][0]["clips"][1]["id"] = value["tracks"][0]["clips"][0]["id"].clone();
        fs::write(&duplicate, serde_json::to_vec(&value).expect("encode")).expect("write");
        assert!(matches!(
            open(&duplicate),
            Err(ProjectOpenError::Project(
                capcut_engine::project::ProjectError::DuplicateClipId(_)
            ))
        ));

        let allocator = dir.path("allocator.cutcraft");
        let mut value: serde_json::Value = serde_json::from_slice(&valid_bytes).expect("json");
        value["next_clip_id"] = serde_json::json!(1);
        fs::write(&allocator, serde_json::to_vec(&value).expect("encode")).expect("write");
        assert!(matches!(
            open(&allocator),
            Err(ProjectOpenError::Project(
                capcut_engine::project::ProjectError::AllocatorOutOfSync
            ))
        ));

        let fps = dir.path("fps.cutcraft");
        let mut value: serde_json::Value = serde_json::from_slice(&valid_bytes).expect("json");
        value["fps"]["numerator"] = serde_json::json!(0);
        fs::write(&fps, serde_json::to_vec(&value).expect("encode")).expect("write");
        assert!(matches!(open(&fps), Err(ProjectOpenError::InvalidFps(_))));

        let source_timebase = dir.path("source-timebase.cutcraft");
        let mut value: serde_json::Value = serde_json::from_slice(&valid_bytes).expect("json");
        value["tracks"][0]["clips"][0]["source"]["timebase"] = serde_json::json!(90_000);
        fs::write(
            &source_timebase,
            serde_json::to_vec(&value).expect("encode"),
        )
        .expect("write");
        assert!(matches!(
            open(&source_timebase),
            Err(ProjectOpenError::InvalidSource(_))
        ));

        let magnet_gap = dir.path("magnet-gap.cutcraft");
        let mut value: serde_json::Value = serde_json::from_slice(&valid_bytes).expect("json");
        value["magnet_enabled"] = serde_json::json!(true);
        fs::write(&magnet_gap, serde_json::to_vec(&value).expect("encode")).expect("write");
        assert!(matches!(
            open(&magnet_gap),
            Err(ProjectOpenError::Engine(
                capcut_engine::command::CommandError::MagnetTrackNotContiguous
            ))
        ));

        assert_eq!(
            open(&valid).expect("valid open").project_id,
            id,
            "failure cases must not corrupt a valid file"
        );
    }

    #[test]
    fn open_rejects_wrong_extension_for_existing_file() {
        let dir = TempDir::new();
        let editor = NativeEditor::new(project_abc()).expect("editor");
        let path = dir.path("project.cutcraft");
        save_current(&editor, &ProjectId::generate(), &path, None);
        let renamed = dir.path("project.json");
        fs::rename(&path, &renamed).expect("rename");
        assert_eq!(
            open(&renamed),
            Err(ProjectOpenError::InvalidExtension(renamed))
        );
    }
}

#[cfg(test)]
mod managed_v2_tests {
    use super::*;
    use crate::schema::ProjectDocumentMetadata;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "cutcraft-v1-v2-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0)
            ));
            std::fs::create_dir_all(&path).expect("temp");
            Self(path)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn v1_open_is_unchanged_and_explicit_save_writes_v2() {
        let temp = TempDir::new();
        let path = temp.0.join("legacy.cutcraft");
        let project_id = ProjectId::generate();
        let json = format!(
            r#"{{"format":"cutcraft-project","schema_version":1,"project_id":"{}","name":"Legacy","fps":{{"numerator":30,"denominator":1}},"timebase":120000,"main_video_track":null,"next_track_id":1,"next_clip_id":1,"magnet_enabled":false,"tracks":[]}}"#,
            project_id
        );
        std::fs::write(&path, json.as_bytes()).expect("v1 write");
        let before = std::fs::read(&path).expect("read");
        let loaded = open(&path).expect("v1 open");
        assert_eq!(loaded.schema_version, 1);
        assert!(loaded.editor.project().assets().is_empty());
        assert_eq!(
            std::fs::read(&path).expect("read"),
            before,
            "open must not rewrite v1"
        );

        let metadata = ProjectDocumentMetadata {
            project_id: loaded.project_id.clone(),
            created_at_unix_ms: loaded.created_at_unix_ms,
            save_generation: 1,
        };
        save_snapshot(
            &loaded.editor.snapshot(),
            &metadata,
            &path,
            Some(&loaded.snapshot),
        )
        .expect("explicit save");
        let reopened = open(&path).expect("v2 open");
        assert_eq!(reopened.schema_version, 2);
        assert_eq!(reopened.project_id, project_id);
        assert!(reopened.editor.project().tracks().is_empty());
        assert_eq!(reopened.save_generation, 1);
    }

    #[test]
    fn open_removes_only_stale_abandoned_staging_files() {
        let temp = TempDir::new();
        let path = temp.0.join("project.cutcraft");
        let project_id = ProjectId::generate();
        let json = format!(
            r#"{{"format":"cutcraft-project","schema_version":1,"project_id":"{}","name":"Legacy","fps":{{"numerator":30,"denominator":1}},"timebase":120000,"main_video_track":null,"next_track_id":1,"next_clip_id":1,"magnet_enabled":false,"tracks":[]}}"#,
            project_id
        );
        std::fs::write(&path, json.as_bytes()).expect("project");
        let old = temp.0.join(".project.cutcraft.999.0.tmp");
        let recent = temp.0.join(".project.cutcraft.999.1.tmp");
        std::fs::write(&old, b"old").expect("old stage");
        std::fs::write(&recent, b"recent").expect("recent stage");
        let old_file = std::fs::OpenOptions::new()
            .write(true)
            .open(&old)
            .expect("old stage file");
        old_file
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(SystemTime::now() - Duration::from_secs(25 * 60 * 60)),
            )
            .expect("old mtime");
        open(&path).expect("open");
        assert!(!old.exists(), "stale staging file must be removed");
        assert!(recent.exists(), "recent staging file must be retained");
    }

    #[cfg(windows)]
    #[allow(clippy::permissions_set_readonly_false)]
    #[test]
    fn read_only_target_failure_preserves_the_previous_file() {
        use capcut_core::time::FrameRate;
        use capcut_engine::project::Project;

        let temp = TempDir::new();
        let path = temp.0.join("readonly.cutcraft");
        let project =
            Project::named(FrameRate::new(30, 1).expect("fps"), "Readonly").expect("project");
        let editor = NativeEditor::new(project).expect("editor");
        let project_id = ProjectId::generate();
        let first = save_snapshot(
            &editor.snapshot(),
            &ProjectDocumentMetadata {
                project_id: project_id.clone(),
                created_at_unix_ms: 1,
                save_generation: 1,
            },
            &path,
            None,
        )
        .expect("first save");
        let before = std::fs::read(&path).expect("read before");
        let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&path, permissions).expect("readonly");
        let result = save_snapshot(
            &editor.snapshot(),
            &ProjectDocumentMetadata {
                project_id,
                created_at_unix_ms: 1,
                save_generation: 2,
            },
            &path,
            Some(&first.snapshot),
        );
        let mut permissions = std::fs::metadata(&path)
            .expect("metadata after")
            .permissions();
        permissions.set_readonly(false);
        std::fs::set_permissions(&path, permissions).expect("clear readonly");
        assert!(result.is_err(), "a read-only target must not be replaced");
        assert_eq!(std::fs::read(&path).expect("read after"), before);
    }
}
