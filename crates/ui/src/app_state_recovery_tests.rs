//! Managed recovery and external-conflict preservation tests.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use capcut_engine::asset::{
    MediaAssetMetadata, MediaAssetRegistration, MediaLocator, MediaRational,
};
use capcut_project::{ProjectLibrary, RecoveryCandidateStatus};

use super::*;
use crate::native_document::AutoSaveStatusKind;

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "cutcraft-recovery-ui-{tag}-{}-{}",
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
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut step = 0u64;
    loop {
        state.update_auto_save_at(context, start_ms + step * 20);
        if predicate(state) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for recovery state"
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
        width: 640,
        height: 360,
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
    MediaAssetRegistration::video(
        name,
        MediaLocator::absolute(format!("C:/media/{name}")).expect("locator"),
        metadata,
    )
    .expect("registration")
}

fn add_asset_and_clip(state: &mut AppState) -> capcut_engine::asset::MediaAssetId {
    let session = state.session_mut().expect("session");
    let asset = session
        .add_media_asset(fake_video_asset("recovery.mp4"))
        .expect("asset");
    session.append_media_asset(asset).expect("append");
    asset
}

fn primary_path(state: &AppState) -> PathBuf {
    state
        .session()
        .and_then(EditorSession::native_document)
        .and_then(|document| document.path())
        .cloned()
        .expect("primary path")
}

fn read_project_id(state: &AppState) -> capcut_project::ProjectId {
    state
        .session()
        .and_then(EditorSession::native_document)
        .map(|document| document.project_id().clone())
        .expect("project id")
}

fn assert_saved(state: &AppState) {
    assert!(matches!(
        state
            .session()
            .and_then(EditorSession::auto_save_status)
            .map(crate::native_document::AutoSaveStatus::kind),
        Some(AutoSaveStatusKind::Saved)
    ));
}

#[test]
fn startup_restore_creates_new_project_preserving_ids_and_original_primary() {
    let (mut state, _dir, context) = managed_state("restore");
    create_project(&mut state, &context);
    let asset = add_asset_and_clip(&mut state);
    state
        .session_mut()
        .expect("session")
        .toggle_main_track_magnet();
    pump_until(&mut state, &context, 1_000, |state| {
        state.session().is_some_and(|session| {
            session.persistent_generation() == session.persisted_generation()
        })
    });
    let original_id = read_project_id(&state);
    let original_path = primary_path(&state);
    let original_bytes = std::fs::read(&original_path).expect("read primary");
    let original_clip = {
        let session = state.session().expect("session");
        session
            .native_timeline
            .as_ref()
            .expect("native")
            .project()
            .main_track()
            .expect("main")
            .clips()[0]
            .id()
    };

    state
        .session_mut()
        .expect("session")
        .rename_native_project("Recovered Edit")
        .expect("rename");
    let root = state
        .project_library_root()
        .expect("library root")
        .to_path_buf();
    let recovery_file = capcut_project::recovery_path(&root, &original_id);
    let expected_generation = state
        .session()
        .and_then(EditorSession::persistent_generation)
        .expect("generation");
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut step = 0u64;
    loop {
        state.update_auto_save_at(&context, 2_000 + step * 20);
        let published = capcut_project::read_recovery_document(&recovery_file)
            .ok()
            .map(|document| document.document_generation);
        if published == Some(expected_generation) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "recovery was not published; recovery={:?}; doc={:?}/{:?}; path={}; root={}",
            state.recovery,
            state
                .session()
                .and_then(EditorSession::persistent_generation),
            state
                .session()
                .and_then(EditorSession::persisted_generation),
            recovery_file.display(),
            root.display()
        );
        std::thread::sleep(Duration::from_millis(10));
        step += 1;
    }
    assert_eq!(
        std::fs::read(&original_path).expect("primary"),
        original_bytes
    );
    drop(state);

    let mut restarted = AppState::default();
    restarted.install_project_library(ProjectLibrary::new(&root));
    let prompt = restarted.recovery_prompt().cloned();
    assert!(
        prompt.is_some(),
        "recovery prompt missing; warning={:?}; candidates={:?}; entries={:?}",
        restarted.recovery_warning(),
        restarted.recovery_candidates(),
        restarted.project_library_entries()
    );
    let prompt = prompt.expect("recovery prompt");
    assert_eq!(prompt.candidate.status, RecoveryCandidateStatus::Newer);
    assert_eq!(prompt.candidate.project_id.as_ref(), Some(&original_id));
    restarted
        .restore_recovery_candidate()
        .expect("start restore");
    pump_until(&mut restarted, &context, 1_000, |state| {
        state.screen() == Screen::Editor
            && !state.is_creating_project()
            && state.session().is_some()
    });
    let recovered_id = read_project_id(&restarted);
    assert_ne!(recovered_id, original_id);
    let session = restarted.session().expect("session");
    let recovered = session.native_timeline.as_ref().expect("native").project();
    assert_eq!(recovered.assets()[0].id(), asset);
    assert_eq!(
        recovered.main_track().expect("main").clips()[0].id(),
        original_clip
    );
    assert!(session.main_track_magnet_enabled);
    assert_eq!(
        std::fs::read(&original_path).expect("primary after restore"),
        original_bytes,
        "restore must not overwrite the original primary"
    );
    assert!(
        !recovery_file.exists(),
        "successful restore consumes the recovery file"
    );
    assert_saved(&restarted);
}

#[test]
fn open_last_saved_keeps_recovery_and_corrupt_recovery_is_reported() {
    let (mut state, _dir, context) = managed_state("corrupt");
    create_project(&mut state, &context);
    let project_id = read_project_id(&state);
    let original_path = primary_path(&state);
    let original_bytes = std::fs::read(&original_path).expect("read primary");
    let root = state.project_library_root().expect("root").to_path_buf();
    let recovery = capcut_project::recovery_path(&root, &project_id);
    std::fs::create_dir_all(recovery.parent().expect("parent")).expect("recovery dir");
    std::fs::write(&recovery, b"{not valid recovery").expect("corrupt recovery");

    let mut restarted = AppState::default();
    restarted.install_project_library(ProjectLibrary::new(&root));
    assert!(restarted.recovery_prompt().is_none());
    assert!(restarted.recovery_warning().is_some());
    assert_eq!(restarted.project_library_entries().len(), 1);
    restarted.open_library_project(project_id.clone());
    pump_until(&mut restarted, &context, 1_000, |state| {
        state.screen() == Screen::Editor && state.session().is_some()
    });
    assert_eq!(
        std::fs::read(&original_path).expect("primary after open"),
        original_bytes
    );
    assert!(
        recovery.exists(),
        "invalid recovery is retained for inspection"
    );
}

#[test]
fn conflict_copy_is_a_new_project_and_never_touches_the_external_file() {
    let (mut state, _dir, context) = managed_state("conflict-copy");
    create_project(&mut state, &context);
    let asset = add_asset_and_clip(&mut state);
    pump_until(&mut state, &context, 1_000, |state| {
        state.session().is_some_and(|session| {
            session.persistent_generation() == session.persisted_generation()
        })
    });
    let original_id = read_project_id(&state);
    let original_path = primary_path(&state);
    let original_clip = {
        let session = state.session().expect("session");
        session
            .native_timeline
            .as_ref()
            .expect("native")
            .project()
            .main_track()
            .expect("main")
            .clips()[0]
            .id()
    };
    use std::io::Write;
    let mut external = std::fs::OpenOptions::new()
        .append(true)
        .open(&original_path)
        .expect("open external");
    external
        .write_all(b"\n// external edit A")
        .expect("write external");
    external.sync_all().expect("sync");
    drop(external);
    state
        .session_mut()
        .expect("session")
        .rename_native_project("Conflict Edit")
        .expect("rename");
    pump_until(&mut state, &context, 2_000, |state| {
        state.session().is_some_and(|session| {
            matches!(
                session.auto_save_status().map(|status| status.kind()),
                Some(AutoSaveStatusKind::Conflict)
            )
        })
    });
    assert!(state.can_preserve_conflict_copy());
    let external_before_copy = std::fs::read(&original_path).expect("read external");

    state.preserve_conflict_copy().expect("start conflict copy");
    let mut external = std::fs::OpenOptions::new()
        .append(true)
        .open(&original_path)
        .expect("open external again");
    external
        .write_all(b"\n// external edit B")
        .expect("write external B");
    external.sync_all().expect("sync B");
    drop(external);
    let external_after_second_edit = std::fs::read(&original_path).expect("read external B");

    pump_until(&mut state, &context, 3_000, |state| {
        !state.is_creating_project() && state.project_library_entries().len() == 2
    });
    assert_eq!(
        std::fs::read(&original_path).expect("external unchanged"),
        external_after_second_edit
    );
    assert_eq!(
        state.project_library_entries().len(),
        2,
        "the recovery copy is visible in the library"
    );
    let copy_entry = state
        .project_library_entries()
        .iter()
        .find(|entry| entry.project_id.as_ref() != Some(&original_id))
        .cloned()
        .expect("copy entry");
    assert!(copy_entry.display_name().contains("Recovery Copy"));
    let copy = capcut_project::open(&copy_entry.path).expect("open copy");
    assert_eq!(copy.editor.project().assets()[0].id(), asset);
    assert_eq!(
        copy.editor.project().main_track().expect("main").clips()[0].id(),
        original_clip
    );
    assert_eq!(
        std::fs::read(&original_path).expect("external after copy"),
        external_after_second_edit
    );
    assert_eq!(
        external_after_second_edit,
        std::fs::read(&original_path).expect("external final")
    );
    let _ = external_before_copy;
}

#[test]
fn conflict_copy_failure_keeps_the_live_editor_and_external_file() {
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
                    message: "simulated conflict-copy failure".to_owned(),
                }),
            }
        }
    }

    let (mut state, _dir, context) = managed_state("conflict-copy-fail");
    create_project(&mut state, &context);
    add_asset_and_clip(&mut state);
    pump_until(&mut state, &context, 1_000, |state| {
        state.session().is_some_and(|session| {
            session.persistent_generation() == session.persisted_generation()
        })
    });
    let original_path = primary_path(&state);
    use std::io::Write;
    let mut external = std::fs::OpenOptions::new()
        .append(true)
        .open(&original_path)
        .expect("open external");
    external
        .write_all(b"\n// external failure edit")
        .expect("write");
    external.sync_all().expect("sync");
    drop(external);
    state
        .session_mut()
        .expect("session")
        .rename_native_project("Conflict Failure")
        .expect("rename");
    pump_until(&mut state, &context, 2_000, |state| {
        state.session().is_some_and(|session| {
            matches!(
                session.auto_save_status().map(|status| status.kind()),
                Some(AutoSaveStatusKind::Conflict)
            )
        })
    });
    let external_bytes = std::fs::read(&original_path).expect("external");
    let session_id = read_project_id(&state);
    state.auto_save = AutoSaveCoordinator::with_worker(
        AutoSavePolicy::default(),
        PersistenceWorker::spawn_with_executor(Box::new(FailingExecutor)),
    );
    state.preserve_conflict_copy().expect("start failure copy");
    pump_until(&mut state, &context, 3_000, |state| {
        !state.is_creating_project()
    });
    assert_eq!(read_project_id(&state), session_id);
    assert!(state.session().is_some());
    assert_eq!(
        std::fs::read(&original_path).expect("external after failure"),
        external_bytes
    );
    assert_eq!(state.project_library_entries().len(), 1);
}

#[test]
fn recovery_worker_cannot_block_the_ui_tick() {
    use std::sync::mpsc;
    use std::sync::Mutex;

    use crate::recovery_state::RecoveryCoordinator;
    use capcut_project::recovery::{
        RecoveryError, RecoveryExecutor, RecoveryJob, RecoveryOutcome, RecoveryWorker,
    };

    struct BlockedExecutor {
        started: mpsc::Sender<()>,
        release: Mutex<mpsc::Receiver<()>>,
    }

    impl RecoveryExecutor for BlockedExecutor {
        fn execute(&self, job: RecoveryJob) -> RecoveryOutcome {
            let _ = self.started.send(());
            if let Ok(release) = self.release.lock() {
                let _ = release.recv();
            }
            RecoveryOutcome {
                project_id: job.project_id,
                generation: job.document_generation,
                result: Err(RecoveryError::Invalid("test release".to_owned())),
            }
        }
    }

    let (mut state, _dir, context) = managed_state("ui-latency");
    create_project(&mut state, &context);
    pump_until(&mut state, &context, 1_000, |state| {
        state.screen() == Screen::Editor
    });
    let root = state.project_library_root().expect("root").to_path_buf();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    state.recovery = RecoveryCoordinator::with_worker(
        Some(root),
        RecoveryWorker::spawn_with_executor(Box::new(BlockedExecutor {
            started: started_tx,
            release: Mutex::new(release_rx),
        })),
    );
    state
        .session_mut()
        .expect("session")
        .rename_native_project("Latency edit")
        .expect("rename");
    state.update_auto_save_at(&context, 2_000);
    state.update_auto_save_at(&context, 2_400);
    started_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("recovery worker started");
    let mut samples = Vec::new();
    for _ in 0..50 {
        let started = Instant::now();
        state.update_auto_save_at(&context, 2_400);
        samples.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p95 = samples[(samples.len() as f64 * 0.95) as usize - 1];
    println!("RECOVERY_UI_TICK p95_ms={p95:.3}");
    assert!(p95 < 100.0, "recovery tick blocked the UI: p95={p95:.3}ms");
    release_tx.send(()).expect("release");
    state.update_auto_save_at(&context, 2_400);
}
