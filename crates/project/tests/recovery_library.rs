//! Recovery discovery, restore preparation and locator rebasing tests.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use capcut_core::time::FrameRate;
use capcut_engine::asset::{
    MediaAssetMetadata, MediaAssetRegistration, MediaLocator, MediaRational,
};
use capcut_engine::command::{Command, NativeEditor};
use capcut_engine::project::Project;
use capcut_project::{
    publish_recovery_snapshot, save_snapshot, ProjectDocumentMetadata, ProjectId, ProjectLibrary,
    RecoveryCandidateInfo, RecoveryCandidateStatus, RecoveryJob,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cutcraft-recovery-library-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn metadata() -> MediaAssetMetadata {
    MediaAssetMetadata {
        container: "mp4".to_owned(),
        video_stream_index: 0,
        codec: "h264".to_owned(),
        width: 640,
        height: 360,
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

fn snapshot(name: &str) -> capcut_engine::project::ProjectSnapshot {
    NativeEditor::new(Project::named(FrameRate::new(30, 1).expect("fps"), name).expect("project"))
        .expect("editor")
        .snapshot()
}

fn recovery_job(
    root: &TempDir,
    editor: &NativeEditor,
    project_id: ProjectId,
    original: &std::path::Path,
    generation: u64,
    last_saved: u64,
) -> RecoveryJob {
    RecoveryJob {
        snapshot: editor.snapshot(),
        project_id,
        original_project_path: original.to_path_buf(),
        document_generation: generation,
        last_saved_generation: last_saved,
        last_saved_unix_ms: 100,
        created_at_unix_ms: 50,
        library_root: root.0.clone(),
    }
}

fn save_primary(
    library: &ProjectLibrary,
    loaded: &capcut_project::LoadedProject,
    path: &std::path::Path,
    generation: u64,
) -> capcut_project::LoadedProject {
    let metadata = ProjectDocumentMetadata {
        project_id: loaded.project_id.clone(),
        created_at_unix_ms: loaded.created_at_unix_ms,
        save_generation: generation,
    };
    save_snapshot(
        &loaded.editor.snapshot(),
        &metadata,
        path,
        Some(&loaded.snapshot),
    )
    .expect("save primary");
    let entry = library.scan().expect("scan")[0].clone();
    library.open_entry(&entry).expect("open")
}

#[test]
fn recovered_copy_preserves_internal_identity_rebases_media_and_does_not_copy() {
    let temp = TempDir::new();
    let library = ProjectLibrary::new(&temp.0);
    let created = library
        .create(snapshot("Original"), "Original")
        .expect("create");
    let media_dir = created.directory.join("media");
    std::fs::create_dir_all(&media_dir).expect("media dir");
    let media_path = media_dir.join("clip.mp4");
    std::fs::write(&media_path, b"not-real-video").expect("media placeholder");

    let entry = library.scan().expect("scan")[0].clone();
    let mut loaded = library.open_entry(&entry).expect("open");
    let asset = loaded
        .editor
        .execute(Command::AddMediaAsset {
            asset: MediaAssetRegistration::video(
                "clip.mp4",
                MediaLocator::project_relative("media/clip.mp4").expect("relative"),
                metadata(),
            )
            .expect("asset"),
        })
        .expect("add asset")
        .created_asset
        .expect("asset id");
    loaded
        .editor
        .execute(Command::AppendMediaClip { asset })
        .expect("append");
    let clip_id = loaded.editor.project().main_track().expect("main").clips()[0].id();
    let before_media = std::fs::read(&media_path).expect("media before");
    let loaded = save_primary(&library, &loaded, &created.path, 1);

    let document = publish_recovery_snapshot(&recovery_job(
        &temp,
        &loaded.editor,
        loaded.project_id.clone(),
        &created.path,
        2,
        1,
    ))
    .expect("publish recovery");
    let entries = library.scan().expect("scan");
    let candidates = library
        .scan_recovery_candidates(&entries)
        .expect("scan recovery");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].status, RecoveryCandidateStatus::Newer);
    let candidate = library
        .load_recovery_document(&candidates[0])
        .expect("load recovery");
    let (prepared, prepared_snapshot) = library
        .prepare_recovered(&candidate, Some(&created.path))
        .expect("prepare recovered");
    assert_ne!(prepared.project_id, loaded.project_id);
    assert!(prepared.name.ends_with("(Recovered)"));
    assert_eq!(
        prepared_snapshot
            .project
            .main_track()
            .expect("main")
            .clips()[0]
            .id(),
        clip_id
    );
    assert_eq!(prepared_snapshot.project.next_asset_id(), 2);
    assert_eq!(prepared_snapshot.project.next_clip_id(), 2);
    assert_eq!(prepared_snapshot.project.assets()[0].id(), asset);
    assert_eq!(
        prepared_snapshot.project.assets()[0].locator().kind(),
        capcut_engine::asset::MediaLocatorKind::Absolute
    );
    assert_eq!(
        std::path::PathBuf::from(prepared_snapshot.project.assets()[0].locator().value()),
        media_path
    );
    assert_eq!(
        std::fs::read(&media_path).expect("media after"),
        before_media
    );
    assert!(!prepared.path.exists(), "prepare must not publish yet");

    let metadata = ProjectDocumentMetadata {
        project_id: prepared.project_id.clone(),
        created_at_unix_ms: prepared.created_at_unix_ms,
        save_generation: 0,
    };
    save_snapshot(&prepared_snapshot, &metadata, &prepared.path, None).expect("publish recovered");
    assert!(prepared.path.exists());
    assert!(
        document.path.exists(),
        "recovery file remains until explicit cleanup"
    );
    library
        .discard_recovery(&candidates[0])
        .expect("discard recovery");
    assert!(!document.path.exists());
    assert!(created.path.exists(), "original primary remains");
}

#[test]
fn recovery_candidates_classify_newer_same_stale_invalid_and_orphan() {
    let temp = TempDir::new();
    let library = ProjectLibrary::new(&temp.0);
    let created = library
        .create(snapshot("Primary"), "Primary")
        .expect("create");
    let entry = library.scan().expect("scan")[0].clone();
    let mut loaded = library.open_entry(&entry).expect("open");
    let project_id = loaded.project_id.clone();

    let _ = publish_recovery_snapshot(&recovery_job(
        &temp,
        &loaded.editor,
        project_id.clone(),
        &created.path,
        1,
        0,
    ))
    .expect("recover gen 1");
    loaded = save_primary(&library, &loaded, &created.path, 1);
    let entries = library.scan().expect("scan");
    let candidates = library.scan_recovery_candidates(&entries).expect("scan");
    assert_eq!(
        candidates[0].status,
        RecoveryCandidateStatus::SameGeneration
    );

    let _ = publish_recovery_snapshot(&recovery_job(
        &temp,
        &loaded.editor,
        project_id.clone(),
        &created.path,
        2,
        1,
    ))
    .expect("recover gen 2");
    let entries = library.scan().expect("scan");
    let candidates = library.scan_recovery_candidates(&entries).expect("scan");
    assert_eq!(candidates[0].status, RecoveryCandidateStatus::Newer);

    loaded = save_primary(&library, &loaded, &created.path, 3);
    let entries = library.scan().expect("scan");
    let candidates = library.scan_recovery_candidates(&entries).expect("scan");
    assert_eq!(candidates[0].status, RecoveryCandidateStatus::Stale);

    let recovery_dir = temp.0.join(".recovery");
    std::fs::create_dir_all(&recovery_dir).expect("recovery dir");
    std::fs::write(
        recovery_dir.join(format!(
            "{project_id}{}",
            capcut_project::recovery::RECOVERY_FILE_SUFFIX
        )),
        b"{corrupt",
    )
    .expect("corrupt recovery");
    let entries = library.scan().expect("scan");
    let candidates = library.scan_recovery_candidates(&entries).expect("scan");
    assert!(candidates
        .iter()
        .any(|candidate| matches!(candidate.status, RecoveryCandidateStatus::Invalid(_))));

    let orphan = ProjectId::generate();
    let _ = publish_recovery_snapshot(&recovery_job(
        &temp,
        &loaded.editor,
        orphan,
        &created.path,
        2,
        1,
    ))
    .expect("orphan recovery");
    let entries = library.scan().expect("scan");
    let candidates = library.scan_recovery_candidates(&entries).expect("scan");
    assert!(candidates
        .iter()
        .any(|candidate| candidate.status == RecoveryCandidateStatus::Orphan));

    let outside = RecoveryCandidateInfo {
        path: temp.0.join("outside.cutcraft-recovery"),
        project_id: Some(project_id),
        name: None,
        original_project_path: None,
        document_generation: Some(1),
        last_saved_generation: Some(0),
        written_at_unix_ms: None,
        status: RecoveryCandidateStatus::Newer,
        detail: None,
    };
    assert!(library.discard_recovery(&outside).is_err());
}
