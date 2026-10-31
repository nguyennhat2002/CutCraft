//! End-to-end managed project creation and generation-safe AutoSave tests.

use std::time::{Duration, Instant};

use capcut_engine::asset::{
    MediaAssetMetadata, MediaAssetRegistration, MediaLocator, MediaRational,
};
use capcut_project::ProjectLibrary;

use super::*;
use crate::native_document::AutoSaveStatusKind;

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "cutcraft-autosave-ui-{tag}-{}-{}",
            std::process::id(),
            unix_millis_now()
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

fn managed_state(tag: &str) -> (AppState, TempDir, egui::Context) {
    let dir = TempDir::new(tag);
    let mut state = AppState::default();
    state.install_project_library(ProjectLibrary::new(&dir.0));
    let context = egui::Context::default();
    crate::theme::configure(&context);
    (state, dir, context)
}

fn pump_until(
    state: &mut AppState,
    context: &egui::Context,
    start_ms: u64,
    mut predicate: impl FnMut(&AppState) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut step = 0u64;
    loop {
        state.update_auto_save_at(context, start_ms + step * 20);
        if predicate(state) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for AutoSave state"
        );
        std::thread::sleep(Duration::from_millis(10));
        step += 1;
    }
}

fn create_project(state: &mut AppState, context: &egui::Context) {
    state.start_new_managed_project().expect("start creation");
    pump_until(state, context, 0, |state| {
        state.screen() == Screen::Editor && state.session().is_some()
    });
}

fn fake_video_asset(name: &str) -> MediaAssetRegistration {
    let metadata = MediaAssetMetadata {
        container: "mov,mp4".to_owned(),
        video_stream_index: 0,
        codec: "h264".to_owned(),
        width: 1920,
        height: 1080,
        pixel_format: Some("yuv420p".to_owned()),
        timebase_numerator: 1,
        timebase_denominator: 15_360,
        stream_start_ticks: 0,
        duration_ticks: Some(3 * 15_360),
        nominal_frame_rate: MediaRational::new(30, 1),
        average_frame_rate: MediaRational::new(30, 1),
        rotation_degrees: 0,
        has_audio: false,
        frame_count_hint: Some(90),
    };
    let mut registration = MediaAssetRegistration::video(
        name,
        MediaLocator::absolute(format!("C:/media/{name}")).expect("locator"),
        metadata,
    )
    .expect("registration");
    registration.file_size_bytes = Some(1024);
    registration
}

fn assert_saved(state: &AppState) {
    let session = state.session().expect("session");
    assert!(matches!(
        session
            .auto_save_status()
            .map(crate::native_document::AutoSaveStatus::kind),
        Some(AutoSaveStatusKind::Saved)
    ));
}

#[test]
fn managed_new_project_is_persisted_and_discoverable_without_save_as() {
    let (mut state, dir, context) = managed_state("create");
    create_project(&mut state, &context);
    let session = state.session().expect("session");
    let document = session.native_document().expect("document");
    let path = document.path().expect("managed path").clone();
    assert!(path.starts_with(&dir.0));
    assert!(path.exists());
    assert_eq!(state.project_library_entries().len(), 1);
    assert_eq!(
        document.project_id(),
        state.project_library_entries()[0]
            .project_id
            .as_ref()
            .expect("entry id")
    );
    assert_saved(&state);
}

#[test]
fn edit_autosaves_assets_and_clips_and_navigation_flushes_before_home() {
    let (mut state, _dir, context) = managed_state("edit");
    create_project(&mut state, &context);
    let asset = {
        let session = state.session_mut().expect("session");
        let asset = session
            .add_media_asset(fake_video_asset("A.mp4"))
            .expect("asset");
        session.append_media_asset(asset).expect("append");
        asset
    };
    state.go_home();
    pump_until(&mut state, &context, 10_000, |state| {
        state.screen() == Screen::Home
    });
    let path = state
        .session()
        .and_then(EditorSession::native_document)
        .and_then(|document| document.path())
        .cloned()
        .expect("path");
    let loaded = capcut_project::open(&path).expect("open saved project");
    assert_eq!(loaded.editor.project().assets().len(), 1);
    assert_eq!(loaded.editor.project().assets()[0].id(), asset);
    assert_eq!(
        loaded
            .editor
            .project()
            .main_track()
            .expect("main")
            .clips()
            .len(),
        1
    );
    assert_saved(&state);
}

#[test]
fn external_modification_is_a_conflict_and_never_clears_dirty_state() {
    let (mut state, _dir, context) = managed_state("conflict");
    create_project(&mut state, &context);
    let path = state
        .session()
        .and_then(EditorSession::native_document)
        .and_then(|document| document.path())
        .cloned()
        .expect("path");
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("open");
    file.write_all(b"\n// external edit")
        .expect("external write");
    file.sync_all().expect("sync");
    drop(file);

    state
        .session_mut()
        .expect("session")
        .rename_native_project("Changed")
        .expect("rename");
    pump_until(&mut state, &context, 30_000, |state| {
        state.session().is_some_and(|session| {
            matches!(
                session
                    .auto_save_status()
                    .map(crate::native_document::AutoSaveStatus::kind),
                Some(AutoSaveStatusKind::Conflict)
            )
        })
    });
    let session = state.session().expect("session");
    assert!(session.native_is_dirty());
    let contents = std::fs::read_to_string(&path).expect("read");
    assert!(contents.contains("external edit"));
}

#[test]
fn active_rename_and_duplicate_keep_identity_rules() {
    let (mut state, _dir, context) = managed_state("rename");
    create_project(&mut state, &context);
    let original_id = state
        .session()
        .and_then(EditorSession::native_document)
        .map(|document| document.project_id().clone())
        .expect("id");
    let original_path = state
        .session()
        .and_then(EditorSession::native_document)
        .and_then(|document| document.path())
        .cloned()
        .expect("path");
    state
        .session_mut()
        .expect("session")
        .rename_native_project("Renamed Project")
        .expect("rename");
    pump_until(&mut state, &context, 40_000, |state| {
        state.session().is_some_and(|session| {
            session.persistent_generation() == session.persisted_generation()
        })
    });
    let loaded = capcut_project::open(&original_path).expect("open renamed");
    assert_eq!(loaded.editor.project().name(), "Renamed Project");
    assert_eq!(loaded.project_id, original_id);
    state.refresh_project_library();
    let copy = state
        .duplicate_library_project(&original_id)
        .expect("duplicate");
    assert_ne!(copy.project_id, original_id);
    assert_ne!(copy.path, original_path);
    assert!(original_path.exists());
    assert!(copy.path.exists());
    assert_eq!(
        capcut_project::open(&original_path)
            .expect("original")
            .editor
            .project()
            .name(),
        "Renamed Project"
    );
}

#[test]
fn graceful_exit_waits_for_pending_autosave() {
    let (mut state, _dir, context) = managed_state("exit");
    create_project(&mut state, &context);
    state
        .session_mut()
        .expect("session")
        .rename_native_project("Exit Flush")
        .expect("rename");
    assert!(!state.request_app_close());
    pump_until(&mut state, &context, 50_000, |state| state.exit_requested);
    let session = state.session().expect("session");
    assert_eq!(
        session.persistent_generation(),
        session.persisted_generation()
    );
    let path = session
        .native_document()
        .and_then(|document| document.path())
        .cloned()
        .expect("path");
    assert_eq!(
        capcut_project::open(&path)
            .expect("open")
            .editor
            .project()
            .name(),
        "Exit Flush"
    );
}

#[test]
fn failed_autosave_preserves_dirty_state_and_blocks_navigation() {
    use crate::auto_save_state::AutoSaveCoordinator;
    use capcut_project::autosave::{
        AutoSavePolicy, PersistenceWorker, SaveExecutor, SaveJob, SaveOutcome,
    };
    use capcut_project::ProjectSaveError;

    struct FailingExecutor;

    impl SaveExecutor for FailingExecutor {
        fn execute(&self, job: SaveJob) -> SaveOutcome {
            SaveOutcome {
                kind: job.kind,
                project_id: job.metadata.project_id.clone(),
                generation: job.generation,
                result: Err(ProjectSaveError::Io {
                    path: job.target,
                    message: "simulated disk failure".to_owned(),
                }),
            }
        }
    }

    let (mut state, _dir, context) = managed_state("failure");
    create_project(&mut state, &context);
    let path = state
        .session()
        .and_then(EditorSession::native_document)
        .and_then(|document| document.path())
        .cloned()
        .expect("path");
    let before = std::fs::read(&path).expect("read saved project");
    state.auto_save = AutoSaveCoordinator::with_worker(
        AutoSavePolicy::default(),
        PersistenceWorker::spawn_with_executor(Box::new(FailingExecutor)),
    );
    state
        .session_mut()
        .expect("session")
        .rename_native_project("Will Fail")
        .expect("rename");
    state.go_home();
    pump_until(&mut state, &context, 10_000, |state| {
        state.navigation_problem().is_some()
    });
    assert_eq!(state.screen(), Screen::Editor);
    let session = state.session().expect("session");
    assert!(session.native_is_dirty());
    assert!(matches!(
        session
            .auto_save_status()
            .map(crate::native_document::AutoSaveStatus::kind),
        Some(AutoSaveStatusKind::Failed)
    ));
    assert_eq!(
        std::fs::read(&path).expect("read after failed save"),
        before,
        "a failed save must not publish a partial document"
    );
    state.cancel_pending_navigation();
}

#[test]
fn edit_during_save_keeps_newer_generation_dirty_until_follow_up_save() {
    use std::sync::mpsc::{self, Receiver};
    use std::sync::Mutex;

    use crate::auto_save_state::AutoSaveCoordinator;
    use capcut_project::autosave::{
        AutoSavePolicy, PersistenceWorker, SaveExecutor, SaveJob, SaveOutcome,
    };

    struct ControlledExecutor {
        started: mpsc::Sender<u64>,
        release: Mutex<Receiver<()>>,
    }

    impl SaveExecutor for ControlledExecutor {
        fn execute(&self, job: SaveJob) -> SaveOutcome {
            let _ = self.started.send(job.generation);
            if let Ok(release) = self.release.lock() {
                let _ = release.recv();
            }
            let result = capcut_project::save_snapshot(
                &job.snapshot,
                &job.metadata,
                &job.target,
                job.allowed_existing.as_ref(),
            );
            SaveOutcome {
                kind: job.kind,
                project_id: job.metadata.project_id.clone(),
                generation: job.generation,
                result,
            }
        }
    }

    let (mut state, _dir, context) = managed_state("during-save");
    create_project(&mut state, &context);
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    state.auto_save = AutoSaveCoordinator::with_worker(
        AutoSavePolicy::default(),
        PersistenceWorker::spawn_with_executor(Box::new(ControlledExecutor {
            started: started_tx,
            release: Mutex::new(release_rx),
        })),
    );

    state
        .session_mut()
        .expect("session")
        .rename_native_project("Generation One")
        .expect("rename one");
    state.update_auto_save_at(&context, 1_000);
    state.update_auto_save_at(&context, 3_000);
    assert_eq!(
        started_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("started"),
        1
    );
    state
        .session_mut()
        .expect("session")
        .rename_native_project("Generation Two")
        .expect("rename two");
    state.update_auto_save_at(&context, 3_100);

    release_tx.send(()).expect("release one");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        state.update_auto_save_at(&context, 3_100);
        let session = state.session().expect("session");
        if session.persisted_generation() == Some(1) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for first save"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let session = state.session().expect("session");
    assert_eq!(session.persistent_generation(), Some(2));
    assert_eq!(session.persisted_generation(), Some(1));
    assert!(session.native_is_dirty());
    assert!(matches!(
        session
            .auto_save_status()
            .map(crate::native_document::AutoSaveStatus::kind),
        Some(AutoSaveStatusKind::UnsavedChanges)
    ));

    state.update_auto_save_at(&context, 5_000);
    assert_eq!(
        started_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("started"),
        2
    );
    release_tx.send(()).expect("release two");
    loop {
        state.update_auto_save_at(&context, 5_000);
        let session = state.session().expect("session");
        if session.persisted_generation() == Some(2) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for second save"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let session = state.session().expect("session");
    assert_eq!(session.persistent_generation(), Some(2));
    assert_eq!(session.persisted_generation(), Some(2));
    assert!(matches!(
        session
            .auto_save_status()
            .map(crate::native_document::AutoSaveStatus::kind),
        Some(AutoSaveStatusKind::Saved)
    ));
}
