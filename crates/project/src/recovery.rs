//! Bounded, checksum-validated recovery snapshots.
//!
//! Recovery is deliberately separate from the authoritative staged project
//! store. One recovery envelope per managed project is retained; writing a
//! recovery snapshot never modifies the primary `.cutcraft` file.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

use capcut_engine::command::NativeEditor;
use capcut_engine::project::ProjectSnapshot;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::id::ProjectId;
use crate::schema::{
    json_depth_within, parse_staged_v2, serialize_project, ProjectDocumentMetadata, ProjectFileV2,
    MAX_JSON_NESTING_DEPTH, MAX_PROJECT_FILE_BYTES,
};

pub const RECOVERY_FORMAT: &str = "cutcraft-recovery";
pub const RECOVERY_VERSION: u32 = 1;
pub const RECOVERY_DIRECTORY: &str = ".recovery";
pub const RECOVERY_FILE_SUFFIX: &str = ".cutcraft-recovery";
pub const MAX_RECOVERY_CANDIDATES: usize = 128;
pub const RECOVERY_DEBOUNCE_MS: u64 = 300;
pub const RECOVERY_RETRY_MS: u64 = 5_000;
/// The document is embedded as a JSON string, so quoting can roughly double
/// the payload before the envelope is serialized.
pub const MAX_RECOVERY_FILE_BYTES: u64 = MAX_PROJECT_FILE_BYTES.saturating_mul(2) + 64 * 1024;

static RECOVERY_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryError {
    Invalid(String),
    Io { path: PathBuf, message: String },
    TooLarge { size: u64, limit: u64 },
    Serialize(String),
    ChecksumMismatch,
    UnsupportedVersion(u32),
    Verification(String),
}

impl std::fmt::Display for RecoveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "invalid recovery snapshot: {message}"),
            Self::Io { path, message } => {
                write!(
                    formatter,
                    "recovery I/O failed at {}: {message}",
                    path.display()
                )
            }
            Self::TooLarge { size, limit } => write!(
                formatter,
                "recovery snapshot is too large ({size} bytes; limit {limit})"
            ),
            Self::Serialize(message) => {
                write!(formatter, "could not serialize recovery: {message}")
            }
            Self::ChecksumMismatch => {
                formatter.write_str("recovery document checksum does not match")
            }
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported recovery version {version}")
            }
            Self::Verification(message) => {
                write!(
                    formatter,
                    "recovery publication failed verification: {message}"
                )
            }
        }
    }
}

impl std::error::Error for RecoveryError {}

#[derive(Debug, Clone)]
pub struct RecoveryJob {
    pub snapshot: ProjectSnapshot,
    pub project_id: ProjectId,
    pub original_project_path: PathBuf,
    pub document_generation: u64,
    pub last_saved_generation: u64,
    pub last_saved_unix_ms: u64,
    pub created_at_unix_ms: u64,
    pub library_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryWritten {
    pub path: PathBuf,
    pub project_id: ProjectId,
    pub generation: u64,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug)]
pub struct RecoveryOutcome {
    pub project_id: ProjectId,
    pub generation: u64,
    pub result: Result<RecoveryWritten, RecoveryError>,
}

#[derive(Debug, Clone)]
pub struct RecoveryDocument {
    pub path: PathBuf,
    pub project_id: ProjectId,
    pub original_project_path: PathBuf,
    pub name: String,
    pub document_generation: u64,
    pub last_saved_generation: u64,
    pub last_saved_unix_ms: u64,
    pub written_at_unix_ms: u64,
    pub editor: NativeEditor,
}

pub trait RecoveryExecutor: Send + 'static {
    fn execute(&self, job: RecoveryJob) -> RecoveryOutcome;
}

pub struct FilesystemRecoveryExecutor;

impl RecoveryExecutor for FilesystemRecoveryExecutor {
    fn execute(&self, job: RecoveryJob) -> RecoveryOutcome {
        let project_id = job.project_id.clone();
        let generation = job.document_generation;
        let result = publish_recovery_snapshot(&job);
        RecoveryOutcome {
            project_id,
            generation,
            result,
        }
    }
}

struct WorkerShared {
    pending: Mutex<Option<RecoveryJob>>,
    result: Mutex<Option<RecoveryOutcome>>,
    in_flight: AtomicBool,
    wake: Condvar,
    stop: AtomicBool,
    submitted: AtomicU64,
    published: AtomicU64,
}

pub struct RecoveryWorker {
    shared: Arc<WorkerShared>,
    worker: Option<JoinHandle<()>>,
}

impl RecoveryWorker {
    pub fn spawn() -> Self {
        Self::spawn_with_executor(Box::new(FilesystemRecoveryExecutor))
    }

    pub fn spawn_with_executor(executor: Box<dyn RecoveryExecutor>) -> Self {
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
            .name("cutcraft-recovery".to_owned())
            .spawn(move || recovery_worker_loop(worker_shared, executor))
            .ok();
        if worker.is_none() {
            shared.stop.store(true, Ordering::SeqCst);
        }
        Self { shared, worker }
    }

    pub fn submit(&self, job: RecoveryJob) -> Result<(), RecoveryError> {
        if self.shared.stop.load(Ordering::SeqCst) {
            return Err(RecoveryError::Io {
                path: job.library_root,
                message: "recovery worker is not running".to_owned(),
            });
        }
        let mut pending = self.shared.pending.lock().map_err(|_| RecoveryError::Io {
            path: job.library_root.clone(),
            message: "recovery worker state is unavailable".to_owned(),
        })?;
        *pending = Some(job);
        self.shared.submitted.fetch_add(1, Ordering::Relaxed);
        self.shared.wake.notify_one();
        Ok(())
    }

    pub fn poll(&self) -> Option<RecoveryOutcome> {
        self.shared
            .result
            .lock()
            .ok()
            .and_then(|mut result| result.take())
    }

    pub fn metrics(&self) -> RecoveryMetrics {
        RecoveryMetrics {
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

impl Drop for RecoveryWorker {
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
pub struct RecoveryMetrics {
    pub worker_count: usize,
    pub in_flight: bool,
    pub queued: usize,
    pub submitted: u64,
    pub published: u64,
}

impl std::fmt::Debug for RecoveryWorker {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecoveryWorker")
            .field("metrics", &self.metrics())
            .finish()
    }
}

fn recovery_worker_loop(shared: Arc<WorkerShared>, executor: Box<dyn RecoveryExecutor>) {
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
        let project_id = job.project_id.clone();
        let generation = job.document_generation;
        shared.in_flight.store(true, Ordering::SeqCst);
        let outcome =
            catch_unwind(AssertUnwindSafe(|| executor.execute(job))).unwrap_or_else(|_| {
                RecoveryOutcome {
                    project_id,
                    generation,
                    result: Err(RecoveryError::Invalid(
                        "recovery worker panicked".to_owned(),
                    )),
                }
            });
        shared.in_flight.store(false, Ordering::SeqCst);
        if let Ok(mut result) = shared.result.lock() {
            *result = Some(outcome);
        }
        shared.published.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn recovery_path(root: &Path, project_id: &ProjectId) -> PathBuf {
    root.join(RECOVERY_DIRECTORY)
        .join(format!("{project_id}{RECOVERY_FILE_SUFFIX}"))
}

/// Serialize, verify, and atomically publish one recovery envelope.
pub fn publish_recovery_snapshot(job: &RecoveryJob) -> Result<RecoveryWritten, RecoveryError> {
    validate_job(job)?;
    let metadata = ProjectDocumentMetadata {
        project_id: job.project_id.clone(),
        created_at_unix_ms: job.created_at_unix_ms,
        save_generation: job.document_generation,
    };
    let file = ProjectFileV2::from_snapshot(&job.snapshot, &metadata, job.last_saved_unix_ms);
    let document =
        serialize_project(&file).map_err(|error| RecoveryError::Serialize(error.to_string()))?;
    let document_json = String::from_utf8(document)
        .map_err(|_| RecoveryError::Serialize("project JSON was not UTF-8".to_owned()))?;
    let document_sha256 = sha256_hex(document_json.as_bytes());
    let written_at_unix_ms = unix_millis_now();
    let envelope = RecoveryEnvelope {
        format: RECOVERY_FORMAT.to_owned(),
        recovery_version: RECOVERY_VERSION,
        project_id: job.project_id.to_string(),
        original_project_path: job.original_project_path.to_string_lossy().into_owned(),
        document_generation: job.document_generation,
        last_saved_generation: job.last_saved_generation,
        last_saved_unix_ms: job.last_saved_unix_ms,
        created_at_unix_ms: job.created_at_unix_ms,
        written_at_unix_ms,
        document_sha256,
        document_json,
    };
    let bytes = serde_json::to_vec_pretty(&envelope)
        .map_err(|error| RecoveryError::Serialize(error.to_string()))?;
    if bytes.len() as u64 > MAX_RECOVERY_FILE_BYTES {
        return Err(RecoveryError::TooLarge {
            size: bytes.len() as u64,
            limit: MAX_RECOVERY_FILE_BYTES,
        });
    }

    let directory = job.library_root.join(RECOVERY_DIRECTORY);
    fs::create_dir_all(&directory).map_err(|error| io_error(&directory, error))?;
    let target = directory.join(format!("{}{RECOVERY_FILE_SUFFIX}", job.project_id));
    if let Ok(existing) = read_recovery_document(&target) {
        if existing.document_generation >= job.document_generation {
            let bytes = fs::metadata(&target)
                .map(|metadata| metadata.len())
                .unwrap_or(0);
            return Ok(RecoveryWritten {
                path: target,
                project_id: existing.project_id,
                generation: existing.document_generation,
                bytes,
                sha256: envelope.document_sha256,
            });
        }
    }

    let temporary = create_temporary(&directory, &job.project_id, &bytes)?;
    let verification = read_recovery_document(&temporary).and_then(|staged| {
        if staged.project_id == job.project_id
            && staged.document_generation == job.document_generation
            && staged.original_project_path == job.original_project_path
        {
            Ok(())
        } else {
            Err(RecoveryError::Verification(
                "staged recovery identity or generation differs".to_owned(),
            ))
        }
    });
    if let Err(error) = verification {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temporary, &target) {
        let _ = fs::remove_file(&temporary);
        return Err(io_error(&target, error));
    }
    sync_parent_best_effort(&directory);
    let bytes = fs::metadata(&target)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    Ok(RecoveryWritten {
        path: target,
        project_id: job.project_id.clone(),
        generation: job.document_generation,
        bytes,
        sha256: sha256_hex(envelope.document_json.as_bytes()),
    })
}

/// Read and fully validate one recovery envelope and its embedded native
/// document. Invalid data is rejected before it can reach engine state.
pub fn read_recovery_document(path: &Path) -> Result<RecoveryDocument, RecoveryError> {
    let envelope = read_envelope(path)?;
    if sha256_hex(envelope.document_json.as_bytes()) != envelope.document_sha256 {
        return Err(RecoveryError::ChecksumMismatch);
    }
    let file = parse_staged_v2(envelope.document_json.as_bytes())
        .map_err(|error| RecoveryError::Invalid(error.to_string()))?;
    if file.project_id != envelope.project_id {
        return Err(RecoveryError::Invalid(
            "embedded project identity does not match the recovery envelope".to_owned(),
        ));
    }
    if file.save_generation != envelope.document_generation {
        return Err(RecoveryError::Invalid(
            "embedded document generation does not match the envelope".to_owned(),
        ));
    }
    let project_id = ProjectId::parse(file.project_id.clone())
        .map_err(|error| RecoveryError::Invalid(error.to_string()))?;
    let name = file.name.clone();
    let (opened_id, editor) = file
        .into_opened()
        .map_err(|error| RecoveryError::Invalid(error.to_string()))?;
    if opened_id != project_id {
        return Err(RecoveryError::Invalid(
            "opened project identity does not match the recovery envelope".to_owned(),
        ));
    }
    Ok(RecoveryDocument {
        path: path.to_path_buf(),
        project_id,
        original_project_path: PathBuf::from(envelope.original_project_path),
        name,
        document_generation: envelope.document_generation,
        last_saved_generation: envelope.last_saved_generation,
        last_saved_unix_ms: envelope.last_saved_unix_ms,
        written_at_unix_ms: envelope.written_at_unix_ms,
        editor,
    })
}

fn validate_job(job: &RecoveryJob) -> Result<(), RecoveryError> {
    if job.document_generation == 0 {
        return Err(RecoveryError::Invalid(
            "recovery generation must be nonzero".to_owned(),
        ));
    }
    if job.last_saved_generation > job.document_generation {
        return Err(RecoveryError::Invalid(
            "last saved generation is newer than the recovery document".to_owned(),
        ));
    }
    validate_original_path(&job.original_project_path)?;
    if job.library_root.as_os_str().is_empty() {
        return Err(RecoveryError::Invalid("recovery root is empty".to_owned()));
    }
    let validation =
        NativeEditor::from_restored(job.snapshot.project.clone(), job.snapshot.magnet_enabled)
            .map_err(|error| RecoveryError::Invalid(error.to_string()))?;
    validation
        .validate_magnet_state()
        .map_err(|error| RecoveryError::Invalid(error.to_string()))?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryEnvelope {
    format: String,
    recovery_version: u32,
    project_id: String,
    original_project_path: String,
    document_generation: u64,
    last_saved_generation: u64,
    last_saved_unix_ms: u64,
    created_at_unix_ms: u64,
    written_at_unix_ms: u64,
    document_sha256: String,
    document_json: String,
}

fn read_envelope(path: &Path) -> Result<RecoveryEnvelope, RecoveryError> {
    let metadata = fs::metadata(path).map_err(|error| io_error(path, error))?;
    if !metadata.is_file() {
        return Err(RecoveryError::Invalid(
            "recovery path is not a file".to_owned(),
        ));
    }
    if metadata.len() > MAX_RECOVERY_FILE_BYTES {
        return Err(RecoveryError::TooLarge {
            size: metadata.len(),
            limit: MAX_RECOVERY_FILE_BYTES,
        });
    }
    let bytes = read_bounded(path)?;
    if !json_depth_within(&bytes, MAX_JSON_NESTING_DEPTH) {
        return Err(RecoveryError::Invalid(
            "recovery JSON nesting is too deep".to_owned(),
        ));
    }
    let envelope: RecoveryEnvelope = serde_json::from_slice(&bytes)
        .map_err(|error| RecoveryError::Invalid(error.to_string()))?;
    validate_envelope(&envelope)?;
    Ok(envelope)
}

fn validate_envelope(envelope: &RecoveryEnvelope) -> Result<(), RecoveryError> {
    if envelope.format != RECOVERY_FORMAT {
        return Err(RecoveryError::Invalid(
            "recovery format marker is incorrect".to_owned(),
        ));
    }
    if envelope.recovery_version != RECOVERY_VERSION {
        return Err(RecoveryError::UnsupportedVersion(envelope.recovery_version));
    }
    ProjectId::parse(envelope.project_id.clone())
        .map_err(|error| RecoveryError::Invalid(error.to_string()))?;
    validate_original_path(Path::new(&envelope.original_project_path))?;
    if envelope.document_generation == 0 {
        return Err(RecoveryError::Invalid(
            "recovery generation must be nonzero".to_owned(),
        ));
    }
    if envelope.last_saved_generation > envelope.document_generation {
        return Err(RecoveryError::Invalid(
            "last saved generation is newer than the recovery document".to_owned(),
        ));
    }
    if envelope.document_sha256.len() != 64
        || !envelope
            .document_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(RecoveryError::Invalid(
            "recovery checksum must be a 64-character hex string".to_owned(),
        ));
    }
    if envelope.document_json.is_empty() {
        return Err(RecoveryError::Invalid(
            "recovery document is empty".to_owned(),
        ));
    }
    Ok(())
}

fn validate_original_path(path: &Path) -> Result<(), RecoveryError> {
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err(RecoveryError::Invalid(
            "original project path must be absolute".to_owned(),
        ));
    }
    if path.to_string_lossy().chars().any(char::is_control) {
        return Err(RecoveryError::Invalid(
            "original project path contains control characters".to_owned(),
        ));
    }
    Ok(())
}

fn create_temporary(
    directory: &Path,
    project_id: &ProjectId,
    bytes: &[u8],
) -> Result<PathBuf, RecoveryError> {
    for _ in 0..8 {
        let counter = RECOVERY_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temporary = directory.join(format!(
            ".{project_id}.{}.{counter}.tmp",
            std::process::id()
        ));
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
                    return Err(io_error(&temporary, error));
                }
                return Ok(temporary);
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(io_error(&temporary, error)),
        }
    }
    Err(RecoveryError::Io {
        path: directory.to_path_buf(),
        message: "could not allocate a unique recovery staging file".to_owned(),
    })
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, RecoveryError> {
    let file = File::open(path).map_err(|error| io_error(path, error))?;
    let mut bytes = Vec::new();
    file.take(MAX_RECOVERY_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| io_error(path, error))?;
    if bytes.len() as u64 > MAX_RECOVERY_FILE_BYTES {
        return Err(RecoveryError::TooLarge {
            size: bytes.len() as u64,
            limit: MAX_RECOVERY_FILE_BYTES,
        });
    }
    Ok(bytes)
}

fn io_error(path: &Path, error: std::io::Error) -> RecoveryError {
    RecoveryError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn unix_millis_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

#[cfg(unix)]
fn sync_parent_best_effort(parent: &Path) {
    if let Ok(directory) = File::open(parent) {
        let _ = directory.sync_all();
    }
}

#[cfg(not(unix))]
fn sync_parent_best_effort(_parent: &Path) {}
