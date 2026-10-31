//! Document identity, generations and AutoSave status for the active project.
//!
//! This is session state, not project content: it is never stored in engine
//! history snapshot or in the `.cutcraft` file.

use std::path::PathBuf;

use capcut_project::{FileSnapshot, ProjectId, SavedDocument};

/// Legacy low-level save indicator used by explicit recovery/copy APIs.
/// The primary Editor UI uses `AutoSaveStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveStatus {
    NotSaved,
    Saved,
    UnsavedChanges,
}

impl SaveStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotSaved => "Not saved",
            Self::Saved => "Saved",
            Self::UnsavedChanges => "Unsaved changes",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoSaveStatus {
    Saved { last_saved_unix_ms: u64 },
    UnsavedChanges,
    Saving,
    Failed(String),
    Conflict(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoSaveStatusKind {
    Saved,
    UnsavedChanges,
    Saving,
    Failed,
    Conflict,
}

impl AutoSaveStatus {
    pub fn kind(&self) -> AutoSaveStatusKind {
        match self {
            Self::Saved { .. } => AutoSaveStatusKind::Saved,
            Self::UnsavedChanges => AutoSaveStatusKind::UnsavedChanges,
            Self::Saving => AutoSaveStatusKind::Saving,
            Self::Failed(_) => AutoSaveStatusKind::Failed,
            Self::Conflict(_) => AutoSaveStatusKind::Conflict,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Saved { .. } => "Saved",
            Self::UnsavedChanges => "Unsaved changes",
            Self::Saving => "Saving...",
            Self::Failed(_) => "Save failed",
            Self::Conflict(_) => "Conflict detected",
        }
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Failed(message) | Self::Conflict(message) => Some(message),
            _ => None,
        }
    }

    pub fn last_saved_unix_ms(&self) -> Option<u64> {
        match self {
            Self::Saved { last_saved_unix_ms } => Some(*last_saved_unix_ms),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDocumentState {
    project_id: ProjectId,
    path: Option<PathBuf>,
    snapshot: Option<FileSnapshot>,
    created_at_unix_ms: u64,
    last_saved_unix_ms: u64,
    generation: u64,
    saved_generation: u64,
    auto_save_status: AutoSaveStatus,
    synthetic_qa: bool,
}

impl NativeDocumentState {
    pub fn unsaved(project_id: ProjectId, synthetic_qa: bool) -> Self {
        Self {
            project_id,
            path: None,
            snapshot: None,
            created_at_unix_ms: 0,
            last_saved_unix_ms: 0,
            generation: 0,
            saved_generation: 0,
            auto_save_status: AutoSaveStatus::UnsavedChanges,
            synthetic_qa,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn loaded(
        project_id: ProjectId,
        snapshot: FileSnapshot,
        created_at_unix_ms: u64,
        last_saved_unix_ms: u64,
        save_generation: u64,
        synthetic_qa: bool,
    ) -> Self {
        Self {
            project_id,
            path: Some(snapshot.path().to_path_buf()),
            snapshot: Some(snapshot),
            created_at_unix_ms,
            last_saved_unix_ms,
            generation: save_generation,
            saved_generation: save_generation,
            auto_save_status: AutoSaveStatus::Saved { last_saved_unix_ms },
            synthetic_qa,
        }
    }

    pub fn created(
        project_id: ProjectId,
        path: PathBuf,
        snapshot: FileSnapshot,
        created_at_unix_ms: u64,
        last_saved_unix_ms: u64,
    ) -> Self {
        Self {
            project_id,
            path: Some(path),
            snapshot: Some(snapshot),
            created_at_unix_ms,
            last_saved_unix_ms,
            generation: 0,
            saved_generation: 0,
            auto_save_status: AutoSaveStatus::Saved { last_saved_unix_ms },
            synthetic_qa: false,
        }
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn snapshot(&self) -> Option<&FileSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn created_at_unix_ms(&self) -> u64 {
        self.created_at_unix_ms
    }

    pub fn last_saved_unix_ms(&self) -> u64 {
        self.last_saved_unix_ms
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn saved_generation(&self) -> u64 {
        self.saved_generation
    }

    pub fn is_dirty(&self) -> bool {
        self.generation != self.saved_generation
    }

    pub fn synthetic_qa(&self) -> bool {
        self.synthetic_qa
    }

    pub fn auto_save_status(&self) -> &AutoSaveStatus {
        &self.auto_save_status
    }

    pub fn set_auto_save_status(&mut self, status: AutoSaveStatus) {
        self.auto_save_status = status;
    }

    /// Advance the persisted-state generation. Transient UI state must never
    /// call this method.
    pub fn mark_persistent_change(&mut self) -> u64 {
        self.generation = self.generation.saturating_add(1);
        if !matches!(self.auto_save_status, AutoSaveStatus::Conflict(_)) {
            self.auto_save_status = AutoSaveStatus::UnsavedChanges;
        }
        self.generation
    }

    pub fn note_save_started(&mut self) {
        self.auto_save_status = AutoSaveStatus::Saving;
    }

    pub fn note_saved(&mut self, saved: &SavedDocument) {
        self.snapshot = Some(saved.snapshot.clone());
        self.path = Some(saved.snapshot.path().to_path_buf());
        self.last_saved_unix_ms = saved.last_saved_unix_ms;
        self.saved_generation = self.saved_generation.max(saved.save_generation);
        self.auto_save_status = if self.generation <= self.saved_generation {
            AutoSaveStatus::Saved {
                last_saved_unix_ms: self.last_saved_unix_ms,
            }
        } else {
            AutoSaveStatus::UnsavedChanges
        };
    }

    pub fn note_save_failed(&mut self, message: impl Into<String>) {
        self.auto_save_status = AutoSaveStatus::Failed(message.into());
    }

    pub fn note_conflict(&mut self, message: impl Into<String>) {
        self.auto_save_status = AutoSaveStatus::Conflict(message.into());
    }

    pub fn clear_failure(&mut self) {
        if matches!(
            self.auto_save_status,
            AutoSaveStatus::Failed(_) | AutoSaveStatus::Conflict(_)
        ) {
            self.auto_save_status = if self.is_dirty() {
                AutoSaveStatus::UnsavedChanges
            } else {
                AutoSaveStatus::Saved {
                    last_saved_unix_ms: self.last_saved_unix_ms,
                }
            };
        }
    }

    pub fn has_conflict(&self) -> bool {
        matches!(self.auto_save_status, AutoSaveStatus::Conflict(_))
    }

    /// Explicit recovery/copy save: marks the current generation published.
    pub fn commit_saved(&mut self, project_id: ProjectId, snapshot: FileSnapshot) {
        self.project_id = project_id;
        self.path = Some(snapshot.path().to_path_buf());
        self.snapshot = Some(snapshot);
        self.saved_generation = self.generation;
        self.auto_save_status = AutoSaveStatus::Saved {
            last_saved_unix_ms: self.last_saved_unix_ms,
        };
    }

    pub fn save_status(&self) -> SaveStatus {
        if self.path.is_none() {
            SaveStatus::NotSaved
        } else if self.generation == self.saved_generation
            && matches!(self.auto_save_status, AutoSaveStatus::Saved { .. })
        {
            SaveStatus::Saved
        } else {
            SaveStatus::UnsavedChanges
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_kinds_and_labels_are_stable() {
        assert_eq!(AutoSaveStatus::Saving.kind(), AutoSaveStatusKind::Saving);
        assert_eq!(AutoSaveStatus::Saving.label(), "Saving...");
        assert_eq!(
            AutoSaveStatus::Conflict("x".to_owned()).kind(),
            AutoSaveStatusKind::Conflict
        );
        assert_eq!(AutoSaveStatus::Failed("x".to_owned()).detail(), Some("x"));
    }
}
