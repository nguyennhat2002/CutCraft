//! Versioned native CutCraft project persistence.
//!
//! This crate owns the `.cutcraft` schema, document identity, bounded input
//! validation, and staged atomic file replacement. It never reads or writes
//! CapCut draft documents.

pub mod autosave;
pub mod recovery;

mod error;
mod id;
mod library;
mod schema;
mod store;

pub use autosave::{
    AutoSavePolicy, AutoSaveScheduler, PersistenceMetrics, PersistenceWorker, SaveExecutor,
    SaveJob, SaveJobKind, SaveOutcome,
};
pub use error::{ProjectOpenError, ProjectSaveError};
pub use id::{ProjectId, ProjectIdError, MAX_PROJECT_ID_BYTES};
pub use library::{
    locator_for_import, paths_equivalent, resolve_media_locator, CreatedProject, PreparedProject,
    ProjectEntryStatus, ProjectLibrary, ProjectLibraryEntry, ProjectLibraryError,
    RecoveryCandidateInfo, RecoveryCandidateStatus,
};
pub use recovery::{
    publish_recovery_snapshot, read_recovery_document, recovery_path, FilesystemRecoveryExecutor,
    RecoveryDocument, RecoveryError, RecoveryExecutor, RecoveryJob, RecoveryMetrics,
    RecoveryOutcome, RecoveryWorker, RecoveryWritten, MAX_RECOVERY_CANDIDATES,
    MAX_RECOVERY_FILE_BYTES, RECOVERY_DEBOUNCE_MS, RECOVERY_DIRECTORY, RECOVERY_FORMAT,
    RECOVERY_RETRY_MS, RECOVERY_VERSION,
};
pub use schema::{
    ProjectDocumentMetadata, FORMAT_MARKER, MAX_JSON_NESTING_DEPTH, MAX_PROJECT_FILE_BYTES,
    MIN_SCHEMA_VERSION, SCHEMA_VERSION,
};
pub use store::{
    open, save, save_snapshot, serialize_snapshot, FileSnapshot, LoadedProject, SavedDocument,
};
