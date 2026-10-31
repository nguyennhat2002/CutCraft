//! Pure AutoSave scheduling and one bounded persistence worker.

use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

use capcut_engine::project::ProjectSnapshot;

use crate::error::ProjectSaveError;
use crate::id::ProjectId;
use crate::schema::ProjectDocumentMetadata;
use crate::store::{save_snapshot, FileSnapshot, SavedDocument};

pub const AUTOSAVE_DEBOUNCE_MS: u64 = 1_500;
pub const AUTOSAVE_MAX_INTERVAL_MS: u64 = 15_000;
pub const AUTOSAVE_FAILURE_RETRY_MS: u64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoSavePolicy {
    pub debounce_ms: u64,
    pub max_interval_ms: u64,
    pub failure_retry_ms: u64,
}

impl Default for AutoSavePolicy {
    fn default() -> Self {
        Self {
            debounce_ms: AUTOSAVE_DEBOUNCE_MS,
            max_interval_ms: AUTOSAVE_MAX_INTERVAL_MS,
            failure_retry_ms: AUTOSAVE_FAILURE_RETRY_MS,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AutoSaveScheduler {
    policy: AutoSavePolicy,
    current_generation: u64,
    saved_generation: u64,
    last_change_ms: Option<u64>,
    unsaved_since_ms: Option<u64>,
    in_flight_generation: Option<u64>,
    retry_not_before_ms: Option<u64>,
}

impl AutoSaveScheduler {
    pub fn new(policy: AutoSavePolicy) -> Self {
        Self {
            policy,
            current_generation: 0,
            saved_generation: 0,
            last_change_ms: None,
            unsaved_since_ms: None,
            in_flight_generation: None,
            retry_not_before_ms: None,
        }
    }

    pub fn with_saved_generation(policy: AutoSavePolicy, saved_generation: u64) -> Self {
        let mut scheduler = Self::new(policy);
        scheduler.reset_saved_generation(saved_generation);
        scheduler
    }

    pub fn reset_saved_generation(&mut self, saved_generation: u64) {
        self.saved_generation = saved_generation;
        self.current_generation = saved_generation;
        self.last_change_ms = None;
        self.unsaved_since_ms = None;
        self.in_flight_generation = None;
        self.retry_not_before_ms = None;
    }

    pub fn note_change(&mut self, now_ms: u64, generation: u64) {
        self.current_generation = self.current_generation.max(generation);
        if self.current_generation <= self.saved_generation {
            return;
        }
        self.last_change_ms = Some(now_ms);
        self.unsaved_since_ms.get_or_insert(now_ms);
        self.retry_not_before_ms = None;
    }

    pub fn should_save(&self, now_ms: u64) -> bool {
        if self.in_flight_generation.is_some()
            || self.current_generation <= self.saved_generation
            || self.unsaved_since_ms.is_none()
        {
            return false;
        }
        if self
            .retry_not_before_ms
            .is_some_and(|not_before| now_ms < not_before)
        {
            return false;
        }
        self.next_deadline_ms()
            .is_some_and(|deadline| now_ms >= deadline)
    }

    pub fn next_deadline_ms(&self) -> Option<u64> {
        if self.in_flight_generation.is_some() || self.current_generation <= self.saved_generation {
            return None;
        }
        let last_change = self.last_change_ms?;
        let unsaved_since = self.unsaved_since_ms?;
        let debounce = last_change.saturating_add(self.policy.debounce_ms);
        let maximum = unsaved_since.saturating_add(self.policy.max_interval_ms);
        let mut deadline = debounce.min(maximum);
        if let Some(not_before) = self.retry_not_before_ms {
            deadline = deadline.max(not_before);
        }
        Some(deadline)
    }

    pub fn note_save_started(&mut self, _now_ms: u64, generation: u64) {
        self.in_flight_generation = Some(generation);
        self.retry_not_before_ms = None;
    }

    pub fn note_save_finished(&mut self, now_ms: u64, generation: u64, success: bool) {
        self.in_flight_generation = None;
        if success {
            self.saved_generation = self.saved_generation.max(generation);
            if self.current_generation <= self.saved_generation {
                self.last_change_ms = None;
                self.unsaved_since_ms = None;
                self.retry_not_before_ms = None;
            } else {
                self.retry_not_before_ms = None;
            }
        } else {
            self.retry_not_before_ms = Some(now_ms.saturating_add(self.policy.failure_retry_ms));
        }
    }

    pub fn current_generation(&self) -> u64 {
        self.current_generation
    }

    pub fn saved_generation(&self) -> u64 {
        self.saved_generation
    }

    pub fn in_flight_generation(&self) -> Option<u64> {
        self.in_flight_generation
    }

    pub fn is_dirty(&self) -> bool {
        self.current_generation != self.saved_generation
    }

    pub fn is_save_in_flight(&self) -> bool {
        self.in_flight_generation.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveJobKind {
    AutoSave,
    CreateProject,
}

#[derive(Debug, Clone)]
pub struct SaveJob {
    pub kind: SaveJobKind,
    pub generation: u64,
    pub snapshot: ProjectSnapshot,
    pub metadata: ProjectDocumentMetadata,
    pub target: PathBuf,
    pub allowed_existing: Option<FileSnapshot>,
}

#[derive(Debug)]
pub struct SaveOutcome {
    pub kind: SaveJobKind,
    pub project_id: ProjectId,
    pub generation: u64,
    pub result: Result<SavedDocument, ProjectSaveError>,
}

pub trait SaveExecutor: Send + 'static {
    fn execute(&self, job: SaveJob) -> SaveOutcome;
}

pub struct FilesystemSaveExecutor;

impl SaveExecutor for FilesystemSaveExecutor {
    fn execute(&self, job: SaveJob) -> SaveOutcome {
        let result = save_snapshot(
            &job.snapshot,
            &job.metadata,
            &job.target,
            job.allowed_existing.as_ref(),
        );
        SaveOutcome {
            kind: job.kind,
            project_id: job.metadata.project_id,
            generation: job.generation,
            result,
        }
    }
}

struct WorkerShared {
    pending: Mutex<Option<SaveJob>>,
    result: Mutex<Option<SaveOutcome>>,
    in_flight: AtomicBool,
    wake: Condvar,
    stop: AtomicBool,
    submitted: AtomicU64,
    published: AtomicU64,
}

pub struct PersistenceWorker {
    shared: Arc<WorkerShared>,
    worker: Option<JoinHandle<()>>,
}

impl PersistenceWorker {
    pub fn spawn() -> Self {
        Self::spawn_with_executor(Box::new(FilesystemSaveExecutor))
    }

    pub fn spawn_with_executor(executor: Box<dyn SaveExecutor>) -> Self {
        let shared = Arc::new(WorkerShared {
            pending: Mutex::new(None),
            result: Mutex::new(None),
            in_flight: AtomicBool::new(false),
            wake: Condvar::new(),
            stop: AtomicBool::new(false),
            submitted: AtomicU64::new(0),
            published: AtomicU64::new(0),
        });
        let worker_shared = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("cutcraft-persistence".to_owned())
            .spawn(move || worker_loop(worker_shared, executor))
            .ok();
        if worker.is_none() {
            shared.stop.store(true, Ordering::SeqCst);
        }
        Self { shared, worker }
    }

    pub fn submit(&self, job: SaveJob) -> Result<(), ProjectSaveError> {
        if self.shared.stop.load(Ordering::SeqCst) {
            return Err(ProjectSaveError::Io {
                path: job.target,
                message: "persistence worker is not running".to_owned(),
            });
        }
        let mut pending = self
            .shared
            .pending
            .lock()
            .map_err(|_| ProjectSaveError::Io {
                path: job.target.clone(),
                message: "persistence worker state is unavailable".to_owned(),
            })?;
        *pending = Some(job);
        self.shared.submitted.fetch_add(1, Ordering::Relaxed);
        self.shared.wake.notify_one();
        Ok(())
    }

    pub fn poll(&self) -> Option<SaveOutcome> {
        self.shared
            .result
            .lock()
            .ok()
            .and_then(|mut result| result.take())
    }

    pub fn metrics(&self) -> PersistenceMetrics {
        PersistenceMetrics {
            worker_count: usize::from(
                self.worker.is_some() && !self.shared.stop.load(Ordering::SeqCst),
            ),
            in_flight: self.shared.in_flight.load(Ordering::SeqCst),
            queued: self
                .shared
                .pending
                .lock()
                .map(|pending| usize::from(pending.is_some()))
                .unwrap_or(0),
            submitted: self.shared.submitted.load(Ordering::Relaxed),
            published: self.shared.published.load(Ordering::Relaxed),
        }
    }
}

impl Drop for PersistenceWorker {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Ok(mut pending) = self.shared.pending.lock() {
            *pending = None;
        }
        self.shared.wake.notify_all();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistenceMetrics {
    pub worker_count: usize,
    pub in_flight: bool,
    pub queued: usize,
    pub submitted: u64,
    pub published: u64,
}

fn worker_loop(shared: Arc<WorkerShared>, executor: Box<dyn SaveExecutor>) {
    loop {
        let job = {
            let mut pending = match shared.pending.lock() {
                Ok(pending) => pending,
                Err(_) => return,
            };
            while pending.is_none() && !shared.stop.load(Ordering::SeqCst) {
                pending = match shared.wake.wait(pending) {
                    Ok(pending) => pending,
                    Err(_) => return,
                };
            }
            if shared.stop.load(Ordering::SeqCst) {
                return;
            }
            pending.take()
        };
        let Some(job) = job else {
            return;
        };
        let project_id = job.metadata.project_id.clone();
        let generation = job.generation;
        let kind = job.kind;
        let target = job.target.clone();
        shared.in_flight.store(true, Ordering::SeqCst);
        let outcome =
            catch_unwind(AssertUnwindSafe(|| executor.execute(job))).unwrap_or_else(|_| {
                SaveOutcome {
                    kind,
                    project_id,
                    generation,
                    result: Err(ProjectSaveError::Io {
                        path: target,
                        message: "persistence worker panicked".to_owned(),
                    }),
                }
            });
        shared.in_flight.store(false, Ordering::SeqCst);
        if let Ok(mut result) = shared.result.lock() {
            *result = Some(outcome);
        }
        shared.published.fetch_add(1, Ordering::Relaxed);
    }
}

impl fmt::Debug for PersistenceWorker {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PersistenceWorker")
            .field("metrics", &self.metrics())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debounce_coalesces_and_max_interval_forces_continuous_save() {
        let mut scheduler = AutoSaveScheduler::new(AutoSavePolicy::default());
        scheduler.note_change(1_000, 1);
        scheduler.note_change(2_000, 2);
        assert_eq!(scheduler.next_deadline_ms(), Some(3_500));
        assert!(!scheduler.should_save(3_499));
        assert!(scheduler.should_save(3_500));
        scheduler.note_save_started(3_500, 2);
        assert!(!scheduler.should_save(20_000));
        scheduler.note_save_finished(4_000, 2, true);
        assert!(!scheduler.is_dirty());

        scheduler.note_change(5_000, 3);
        scheduler.note_change(20_500, 4);
        assert_eq!(scheduler.next_deadline_ms(), Some(20_000));
        assert!(scheduler.should_save(20_500));
    }

    #[test]
    fn failed_save_keeps_dirty_and_schedules_bounded_retry() {
        let mut scheduler = AutoSaveScheduler::new(AutoSavePolicy::default());
        scheduler.note_change(1_000, 1);
        scheduler.note_save_started(2_500, 1);
        scheduler.note_save_finished(3_000, 1, false);
        assert!(scheduler.is_dirty());
        assert!(!scheduler.should_save(3_000));
        assert_eq!(scheduler.next_deadline_ms(), Some(8_000));
        assert!(scheduler.should_save(8_000));
    }

    #[test]
    fn newer_generation_remains_dirty_after_older_save_succeeds() {
        let mut scheduler = AutoSaveScheduler::new(AutoSavePolicy::default());
        scheduler.note_change(1_000, 10);
        scheduler.note_save_started(2_500, 10);
        scheduler.note_change(3_000, 11);
        scheduler.note_save_finished(4_000, 10, true);
        assert_eq!(scheduler.saved_generation(), 10);
        assert_eq!(scheduler.current_generation(), 11);
        assert!(scheduler.is_dirty());
        assert_eq!(scheduler.next_deadline_ms(), Some(4_500));
    }
}

#[cfg(test)]
mod worker_tests {
    use std::path::PathBuf;
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use capcut_core::time::FrameRate;
    use capcut_engine::command::NativeEditor;
    use capcut_engine::project::Project;

    use super::*;
    use crate::id::ProjectId;

    struct ControlledExecutor {
        started: Sender<u64>,
        release: Mutex<Receiver<()>>,
    }

    impl SaveExecutor for ControlledExecutor {
        fn execute(&self, job: SaveJob) -> SaveOutcome {
            let _ = self.started.send(job.generation);
            if let Ok(release) = self.release.lock() {
                let _ = release.recv();
            }
            let result = save_snapshot(
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

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "cutcraft-autosave-{}-{}",
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

    fn job(temp: &TempDir, generation: u64) -> SaveJob {
        let project =
            Project::named(FrameRate::new(30, 1).expect("rate"), "AutoSave").expect("project");
        let editor = NativeEditor::new(project).expect("editor");
        SaveJob {
            kind: SaveJobKind::AutoSave,
            generation,
            snapshot: editor.snapshot(),
            metadata: ProjectDocumentMetadata {
                project_id: ProjectId::generate(),
                created_at_unix_ms: 1,
                save_generation: generation,
            },
            target: temp.0.join(format!("project-{generation}.cutcraft")),
            allowed_existing: None,
        }
    }

    fn wait_for_result(worker: &PersistenceWorker) -> SaveOutcome {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(outcome) = worker.poll() {
                return outcome;
            }
            assert!(Instant::now() < deadline, "timed out waiting for save");
            std::thread::yield_now();
        }
    }

    #[test]
    fn one_save_runs_and_newer_pending_job_runs_after_it() {
        let temp = TempDir::new();
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let worker = PersistenceWorker::spawn_with_executor(Box::new(ControlledExecutor {
            started: started_tx,
            release: Mutex::new(release_rx),
        }));
        worker.submit(job(&temp, 10)).expect("job 10");
        assert_eq!(started_rx.recv_timeout(Duration::from_secs(5)).unwrap(), 10);
        worker.submit(job(&temp, 11)).expect("job 11");
        assert!(worker.metrics().in_flight);
        assert_eq!(worker.metrics().queued, 1);
        release_tx.send(()).expect("release 10");
        let first = wait_for_result(&worker);
        assert_eq!(first.generation, 10);
        assert!(first.result.is_ok());
        assert_eq!(started_rx.recv_timeout(Duration::from_secs(5)).unwrap(), 11);
        release_tx.send(()).expect("release 11");
        let second = wait_for_result(&worker);
        assert_eq!(second.generation, 11);
        assert!(second.result.is_ok());
        let metrics = worker.metrics();
        assert_eq!(metrics.worker_count, 1);
        assert!(metrics.queued <= 1);
    }

    struct PanickingExecutor;

    impl SaveExecutor for PanickingExecutor {
        fn execute(&self, _job: SaveJob) -> SaveOutcome {
            panic!("injected persistence executor panic");
        }
    }

    #[test]
    fn executor_panic_publishes_a_failure_without_hanging_the_worker() {
        let temp = TempDir::new();
        let worker = PersistenceWorker::spawn_with_executor(Box::new(PanickingExecutor));
        worker.submit(job(&temp, 1)).expect("job");
        let outcome = wait_for_result(&worker);
        assert_eq!(outcome.generation, 1);
        assert!(matches!(
            outcome.result,
            Err(ProjectSaveError::Io { ref message, .. }) if message.contains("panicked")
        ));
        assert!(!worker.metrics().in_flight);
        assert_eq!(worker.metrics().worker_count, 1);
    }

    #[test]
    fn dropping_worker_stops_cleanly() {
        let temp = TempDir::new();
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let worker = PersistenceWorker::spawn_with_executor(Box::new(ControlledExecutor {
            started: started_tx,
            release: Mutex::new(release_rx),
        }));
        worker.submit(job(&temp, 1)).expect("job");
        assert_eq!(started_rx.recv_timeout(Duration::from_secs(5)).unwrap(), 1);
        release_tx.send(()).expect("release");
        assert!(wait_for_result(&worker).result.is_ok());
        drop(worker);
    }
}
