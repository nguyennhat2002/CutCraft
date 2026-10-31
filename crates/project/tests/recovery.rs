//! Recovery envelope, publisher and worker integration tests.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use capcut_core::time::FrameRate;
use capcut_engine::asset::{
    MediaAssetMetadata, MediaAssetRegistration, MediaLocator, MediaRational,
};
use capcut_engine::command::{Command, NativeEditor};
use capcut_engine::project::{Project, TrackKind};
use capcut_project::{
    publish_recovery_snapshot, read_recovery_document, ProjectId, RecoveryError, RecoveryExecutor,
    RecoveryJob, RecoveryOutcome, RecoveryWorker,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cutcraft-recovery-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp");
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
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

fn editor() -> NativeEditor {
    let mut project =
        Project::named(FrameRate::new(30, 1).expect("fps"), "Recovery").expect("project");
    let track = project
        .add_track("V1", TrackKind::Video, false)
        .expect("track");
    project.assign_main_track(track).expect("main");
    let mut editor = NativeEditor::new(project).expect("editor");
    let asset = editor
        .execute(Command::AddMediaAsset {
            asset: MediaAssetRegistration::video(
                "clip.mp4",
                MediaLocator::absolute("C:/media/clip.mp4").expect("locator"),
                metadata(),
            )
            .expect("asset"),
        })
        .expect("add asset")
        .created_asset
        .expect("asset id");
    editor
        .execute(Command::AppendMediaClip { asset })
        .expect("append");
    editor.enable_magnet().expect("magnet");
    editor
}

fn job(
    temp: &TempDir,
    editor: &NativeEditor,
    project_id: ProjectId,
    generation: u64,
    last_saved: u64,
) -> RecoveryJob {
    RecoveryJob {
        snapshot: editor.snapshot(),
        project_id,
        original_project_path: temp.path("project.cutcraft"),
        document_generation: generation,
        last_saved_generation: last_saved,
        last_saved_unix_ms: 100,
        created_at_unix_ms: 50,
        library_root: temp.0.clone(),
    }
}

#[test]
fn recovery_round_trip_preserves_generation_checksum_magnet_and_assets() {
    let temp = TempDir::new();
    let editor = editor();
    let project_id = ProjectId::generate();
    let written =
        publish_recovery_snapshot(&job(&temp, &editor, project_id.clone(), 3, 2)).expect("publish");
    assert!(written.path.exists());
    assert_eq!(written.generation, 3);
    let document = read_recovery_document(&written.path).expect("read");
    assert_eq!(document.project_id, project_id);
    assert_eq!(document.document_generation, 3);
    assert_eq!(document.last_saved_generation, 2);
    assert!(document.editor.magnet_enabled());
    assert_eq!(document.editor.project().assets().len(), 1);
    assert_eq!(
        document
            .editor
            .project()
            .main_track()
            .expect("main")
            .clips()
            .len(),
        1
    );
    assert_eq!(
        document.editor.project().next_asset_id(),
        editor.project().next_asset_id()
    );
    assert_eq!(
        document.editor.project().next_clip_id(),
        editor.project().next_clip_id()
    );
}

#[test]
fn corrupt_checksum_identity_and_generations_are_rejected() {
    let temp = TempDir::new();
    let editor = editor();
    let project_id = ProjectId::generate();
    let written =
        publish_recovery_snapshot(&job(&temp, &editor, project_id.clone(), 3, 2)).expect("publish");
    let bytes = std::fs::read(&written.path).expect("read recovery");
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    value["document_json"] = serde_json::Value::String(format!(
        "{} ",
        value["document_json"].as_str().expect("document")
    ));
    std::fs::write(&written.path, serde_json::to_vec(&value).expect("encode")).expect("write");
    assert!(matches!(
        read_recovery_document(&written.path),
        Err(RecoveryError::ChecksumMismatch)
    ));

    publish_recovery_snapshot(&job(&temp, &editor, project_id.clone(), 3, 2)).expect("publish");
    let bytes = std::fs::read(&written.path).expect("read recovery");
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    value["project_id"] = serde_json::Value::String(ProjectId::generate().to_string());
    std::fs::write(&written.path, serde_json::to_vec(&value).expect("encode")).expect("write");
    assert!(matches!(
        read_recovery_document(&written.path),
        Err(RecoveryError::Invalid(_))
    ));

    publish_recovery_snapshot(&job(&temp, &editor, project_id, 3, 2)).expect("publish");
    let bytes = std::fs::read(&written.path).expect("read recovery");
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    value["document_generation"] = serde_json::json!(0);
    std::fs::write(&written.path, serde_json::to_vec(&value).expect("encode")).expect("write");
    assert!(matches!(
        read_recovery_document(&written.path),
        Err(RecoveryError::Invalid(_))
    ));
}

#[test]
fn publisher_keeps_newer_recovery_and_replaces_older_or_invalid_one() {
    let temp = TempDir::new();
    let editor = editor();
    let project_id = ProjectId::generate();
    publish_recovery_snapshot(&job(&temp, &editor, project_id.clone(), 3, 2)).expect("publish 3");
    let older =
        publish_recovery_snapshot(&job(&temp, &editor, project_id.clone(), 2, 1)).expect("older");
    assert_eq!(older.generation, 3);
    assert_eq!(
        read_recovery_document(&older.path)
            .expect("read")
            .document_generation,
        3
    );

    let newer =
        publish_recovery_snapshot(&job(&temp, &editor, project_id.clone(), 4, 3)).expect("newer");
    assert_eq!(newer.generation, 4);
    assert_eq!(
        read_recovery_document(&newer.path)
            .expect("read")
            .document_generation,
        4
    );

    std::fs::write(&newer.path, b"{broken").expect("corrupt");
    let recovered =
        publish_recovery_snapshot(&job(&temp, &editor, project_id, 5, 4)).expect("recover corrupt");
    assert_eq!(recovered.generation, 5);
    assert_eq!(
        read_recovery_document(&recovered.path)
            .expect("read")
            .document_generation,
        5
    );
}

struct FakeExecutor {
    started: mpsc::Sender<u64>,
}

impl RecoveryExecutor for FakeExecutor {
    fn execute(&self, job: RecoveryJob) -> RecoveryOutcome {
        let _ = self.started.send(job.document_generation);
        let result = publish_recovery_snapshot(&job);
        RecoveryOutcome {
            project_id: job.project_id,
            generation: job.document_generation,
            result,
        }
    }
}

struct PanicExecutor;

impl RecoveryExecutor for PanicExecutor {
    fn execute(&self, _job: RecoveryJob) -> RecoveryOutcome {
        panic!("injected recovery worker panic");
    }
}

#[test]
fn worker_publishes_and_contains_executor_panics() {
    let temp = TempDir::new();
    let editor = editor();
    let project_id = ProjectId::generate();
    let (started_tx, started_rx) = mpsc::channel();
    let worker = RecoveryWorker::spawn_with_executor(Box::new(FakeExecutor {
        started: started_tx,
    }));
    worker
        .submit(job(&temp, &editor, project_id.clone(), 3, 2))
        .expect("submit");
    assert_eq!(
        started_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("started"),
        3
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    let outcome = loop {
        if let Some(outcome) = worker.poll() {
            break outcome;
        }
        assert!(Instant::now() < deadline, "timed out");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(outcome.result.is_ok());
    drop(worker);

    let worker = RecoveryWorker::spawn_with_executor(Box::new(PanicExecutor));
    worker
        .submit(job(&temp, &editor, project_id, 4, 3))
        .expect("submit panicking");
    let deadline = Instant::now() + Duration::from_secs(5);
    let outcome = loop {
        if let Some(outcome) = worker.poll() {
            break outcome;
        }
        assert!(
            Instant::now() < deadline,
            "panicked worker did not publish an outcome"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(matches!(outcome.result, Err(RecoveryError::Invalid(_))));
}
