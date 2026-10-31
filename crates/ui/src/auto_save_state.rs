//! UI-side generation-safe AutoSave coordination.

use std::path::PathBuf;

use capcut_engine::project::ProjectSnapshot;
use capcut_project::autosave::{
    AutoSavePolicy, AutoSaveScheduler, PersistenceWorker, SaveJob, SaveJobKind, SaveOutcome,
};
use capcut_project::{ProjectDocumentMetadata, ProjectId, ProjectSaveError};

use crate::editor_state::EditorSession;
use crate::native_document::AutoSaveStatus;

pub struct AutoSaveCoordinator {
    worker: PersistenceWorker,
    scheduler: AutoSaveScheduler,
    document_key: Option<(ProjectId, PathBuf)>,
    conflict_paused: bool,
    auxiliary_outcomes: Vec<SaveOutcome>,
}

impl Default for AutoSaveCoordinator {
    fn default() -> Self {
        Self::new(AutoSavePolicy::default())
    }
}

impl AutoSaveCoordinator {
    pub fn new(policy: AutoSavePolicy) -> Self {
        Self::with_worker(policy, PersistenceWorker::spawn())
    }

    pub fn with_worker(policy: AutoSavePolicy, worker: PersistenceWorker) -> Self {
        Self {
            worker,
            scheduler: AutoSaveScheduler::new(policy),
            document_key: None,
            conflict_paused: false,
            auxiliary_outcomes: Vec::new(),
        }
    }

    /// Poll persistence results and schedule the next AutoSave.
    pub fn tick(&mut self, mut session: Option<&mut EditorSession>, now_ms: u64) -> bool {
        let mut changed = self.poll_outcomes(session.as_deref_mut(), now_ms);
        let Some(session) = session else {
            return changed;
        };
        let Some(document) = session.native_document.as_ref() else {
            return changed;
        };
        let Some(path) = document.path().cloned() else {
            return changed;
        };
        let key = (document.project_id().clone(), path);
        if self.document_key.as_ref() != Some(&key) {
            self.document_key = Some(key.clone());
            self.conflict_paused = false;
            self.scheduler
                .reset_saved_generation(document.saved_generation());
            changed = true;
        }
        let generation = document.generation();
        if self.scheduler.current_generation() != generation {
            self.scheduler.note_change(now_ms, generation);
            changed = true;
        }
        if self.conflict_paused || document.has_conflict() {
            return changed;
        }
        if self.scheduler.should_save(now_ms) {
            match self.submit_auto_save(session, now_ms) {
                Ok(true) => changed = true,
                Ok(false) => {}
                Err(error) => {
                    let message = error.to_string();
                    if let Some(document) = session.native_document.as_mut() {
                        document.note_save_failed(message);
                    }
                    self.scheduler.note_save_finished(
                        now_ms,
                        self.scheduler.current_generation(),
                        false,
                    );
                    changed = true;
                }
            }
        }
        changed
    }

    /// Bypass debounce for a navigation/exit boundary.
    pub fn request_immediate_save(
        &mut self,
        session: &mut EditorSession,
        now_ms: u64,
    ) -> Result<(), String> {
        let Some(document) = session.native_document.as_ref() else {
            return Ok(());
        };
        if document.path().is_none() {
            return Ok(());
        }
        if document.generation() == document.saved_generation()
            && !self.scheduler.is_save_in_flight()
        {
            return Ok(());
        }
        self.conflict_paused = false;
        self.scheduler.note_change(now_ms, document.generation());
        if self.scheduler.is_save_in_flight() {
            return Ok(());
        }
        self.submit_auto_save(session, now_ms)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn is_fully_saved(&self, session: &EditorSession) -> bool {
        let Some(document) = session.native_document.as_ref() else {
            return true;
        };
        document.path().is_none()
            || (!self.conflict_paused
                && !self.scheduler.is_save_in_flight()
                && document.generation() == document.saved_generation()
                && matches!(document.auto_save_status(), AutoSaveStatus::Saved { .. }))
    }

    pub fn is_pending(&self, session: &EditorSession) -> bool {
        let Some(document) = session.native_document.as_ref() else {
            return false;
        };
        document.path().is_some()
            && (self.scheduler.is_dirty()
                || self.scheduler.is_save_in_flight()
                || document.generation() != document.saved_generation())
    }

    pub fn next_deadline_ms(&self) -> Option<u64> {
        if self.scheduler.is_save_in_flight() {
            Some(0)
        } else {
            self.scheduler.next_deadline_ms()
        }
    }

    pub fn has_conflict(&self) -> bool {
        self.conflict_paused
    }

    pub fn submit_auxiliary(&self, job: SaveJob) -> Result<(), String> {
        self.worker.submit(job).map_err(|error| error.to_string())
    }

    pub fn take_auxiliary_outcomes(&mut self) -> Vec<SaveOutcome> {
        std::mem::take(&mut self.auxiliary_outcomes)
    }

    pub fn retry_after_problem(&mut self, session: &mut EditorSession, now_ms: u64) -> bool {
        let Some(document) = session.native_document.as_mut() else {
            return false;
        };
        if !matches!(
            document.auto_save_status(),
            AutoSaveStatus::Failed(_) | AutoSaveStatus::Conflict(_)
        ) {
            return false;
        }
        document.clear_failure();
        self.conflict_paused = false;
        let generation = session.persistent_generation().unwrap_or(0);
        self.scheduler.note_change(now_ms, generation);
        self.scheduler
            .reset_saved_generation(session.persisted_generation().unwrap_or(0));
        self.scheduler.note_change(now_ms, generation);
        self.request_immediate_save(session, now_ms)
            .map(|_| true)
            .unwrap_or(false)
    }

    pub fn pause_conflict(&mut self, session: &mut EditorSession, message: impl Into<String>) {
        self.conflict_paused = true;
        if let Some(document) = session.native_document.as_mut() {
            document.note_conflict(message);
        }
    }

    pub fn metrics(&self) -> capcut_project::PersistenceMetrics {
        self.worker.metrics()
    }

    fn poll_outcomes(&mut self, mut session: Option<&mut EditorSession>, now_ms: u64) -> bool {
        let mut changed = false;
        while let Some(outcome) = self.worker.poll() {
            if outcome.kind != SaveJobKind::AutoSave {
                self.auxiliary_outcomes.push(outcome);
                changed = true;
                continue;
            }
            let Some(session) = session.as_deref_mut() else {
                continue;
            };
            let Some(document) = session.native_document.as_mut() else {
                continue;
            };
            if document.project_id() != &outcome.project_id {
                continue;
            }
            match outcome.result {
                Ok(saved) => {
                    document.note_saved(&saved);
                    self.conflict_paused = false;
                    self.scheduler
                        .note_save_finished(now_ms, outcome.generation, true);
                }
                Err(error) => {
                    let conflict = matches!(
                        error,
                        ProjectSaveError::ExternalModified(_)
                            | ProjectSaveError::ExternalMissing(_)
                            | ProjectSaveError::TargetExists(_)
                    );
                    if conflict {
                        self.conflict_paused = true;
                        document.note_conflict(error.to_string());
                    } else {
                        document.note_save_failed(error.to_string());
                    }
                    self.scheduler
                        .note_save_finished(now_ms, outcome.generation, false);
                }
            }
            changed = true;
        }
        changed
    }

    fn submit_auto_save(
        &mut self,
        session: &mut EditorSession,
        now_ms: u64,
    ) -> Result<bool, String> {
        let Some(document) = session.native_document.as_ref() else {
            return Ok(false);
        };
        let Some(path) = document.path().cloned() else {
            return Ok(false);
        };
        let Some(native) = session.native_timeline.as_ref() else {
            return Ok(false);
        };
        let generation = document.generation();
        let snapshot: ProjectSnapshot = native.editor().snapshot();
        let metadata = ProjectDocumentMetadata {
            project_id: document.project_id().clone(),
            created_at_unix_ms: document.created_at_unix_ms(),
            save_generation: generation,
        };
        let job = SaveJob {
            kind: SaveJobKind::AutoSave,
            generation,
            snapshot,
            metadata,
            target: path,
            allowed_existing: document.snapshot().cloned(),
        };
        self.worker.submit(job).map_err(|error| error.to_string())?;
        self.scheduler.note_save_started(now_ms, generation);
        if let Some(document) = session.native_document.as_mut() {
            document.note_save_started();
        }
        Ok(true)
    }
}

impl std::fmt::Debug for AutoSaveCoordinator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AutoSaveCoordinator")
            .field("document_key", &self.document_key)
            .field("conflict_paused", &self.conflict_paused)
            .field("scheduler", &self.scheduler)
            .field("metrics", &self.worker.metrics())
            .finish()
    }
}
