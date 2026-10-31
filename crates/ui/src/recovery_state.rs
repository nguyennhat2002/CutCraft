//! UI-side generation-safe recovery publication.

use std::path::PathBuf;

use capcut_project::recovery::{RecoveryJob, RecoveryMetrics, RecoveryWorker};
use capcut_project::{ProjectId, RECOVERY_DEBOUNCE_MS, RECOVERY_RETRY_MS};

use crate::editor_state::EditorSession;

#[derive(Debug, Clone)]
pub struct RecoveryScheduler {
    current_generation: u64,
    saved_generation: u64,
    published_generation: u64,
    pending_generation: Option<u64>,
    pending_since_ms: Option<u64>,
    in_flight_generation: Option<u64>,
    retry_at_ms: Option<u64>,
}

impl RecoveryScheduler {
    pub fn new() -> Self {
        Self {
            current_generation: 0,
            saved_generation: 0,
            published_generation: 0,
            pending_generation: None,
            pending_since_ms: None,
            in_flight_generation: None,
            retry_at_ms: None,
        }
    }

    /// A new/reset document starts with the primary file as the watermark;
    /// already-saved generations do not need a recovery snapshot.
    pub fn reset(&mut self, saved_generation: u64) {
        self.current_generation = saved_generation;
        self.saved_generation = saved_generation;
        self.published_generation = saved_generation;
        self.pending_generation = None;
        self.pending_since_ms = None;
        self.in_flight_generation = None;
        self.retry_at_ms = None;
    }

    pub fn note_generation(&mut self, now_ms: u64, generation: u64, saved_generation: u64) {
        self.current_generation = generation;
        self.saved_generation = saved_generation;
        if generation <= saved_generation {
            self.pending_generation = None;
            self.pending_since_ms = None;
            return;
        }
        if self.in_flight_generation == Some(generation) || generation <= self.published_generation
        {
            if self.pending_generation == Some(generation) {
                self.pending_generation = None;
                self.pending_since_ms = None;
            }
            return;
        }
        if self
            .pending_generation
            .is_none_or(|pending| pending < generation)
        {
            self.pending_generation = Some(generation);
            self.pending_since_ms = Some(now_ms);
        }
    }

    pub fn should_publish(&self, now_ms: u64) -> bool {
        if self.in_flight_generation.is_some() {
            return false;
        }
        let Some(pending) = self.pending_generation else {
            return false;
        };
        if pending <= self.saved_generation || pending <= self.published_generation {
            return false;
        }
        if self.retry_at_ms.is_some_and(|retry_at| now_ms < retry_at) {
            return false;
        }
        self.pending_since_ms
            .is_some_and(|since| now_ms >= since.saturating_add(RECOVERY_DEBOUNCE_MS))
    }

    pub fn next_deadline_ms(&self) -> Option<u64> {
        if self.in_flight_generation.is_some() {
            return None;
        }
        let pending = self.pending_generation?;
        if pending <= self.saved_generation || pending <= self.published_generation {
            return None;
        }
        let since = self.pending_since_ms?;
        let debounce = since.saturating_add(RECOVERY_DEBOUNCE_MS);
        Some(match self.retry_at_ms {
            Some(retry_at) => debounce.max(retry_at),
            None => debounce,
        })
    }

    pub fn note_started(&mut self, generation: u64) {
        self.in_flight_generation = Some(generation);
        if self.pending_generation == Some(generation) {
            self.pending_generation = None;
            self.pending_since_ms = None;
        }
        self.retry_at_ms = None;
    }

    pub fn note_finished(&mut self, now_ms: u64, generation: u64, success: bool) {
        self.in_flight_generation = None;
        if success {
            self.published_generation = self.published_generation.max(generation);
            if self
                .pending_generation
                .is_some_and(|pending| pending <= self.published_generation)
            {
                self.pending_generation = None;
                self.pending_since_ms = None;
            } else if let Some(pending) = self.pending_generation {
                self.pending_since_ms = Some(now_ms);
                if pending <= self.saved_generation {
                    self.pending_generation = None;
                    self.pending_since_ms = None;
                }
            }
            self.retry_at_ms = None;
        } else {
            self.retry_at_ms = Some(now_ms.saturating_add(RECOVERY_RETRY_MS));
            if self.pending_generation.is_none()
                && generation > self.saved_generation
                && generation > self.published_generation
            {
                self.pending_generation = Some(generation);
                self.pending_since_ms = Some(now_ms);
            }
        }
    }

    pub fn pending_generation(&self) -> Option<u64> {
        self.pending_generation
    }

    pub fn published_generation(&self) -> u64 {
        self.published_generation
    }

    pub fn in_flight_generation(&self) -> Option<u64> {
        self.in_flight_generation
    }

    pub fn is_pending(&self) -> bool {
        self.pending_generation.is_some() || self.in_flight_generation.is_some()
    }
}

impl Default for RecoveryScheduler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RecoveryCoordinator {
    worker: RecoveryWorker,
    scheduler: RecoveryScheduler,
    root: Option<PathBuf>,
    project_key: Option<(ProjectId, PathBuf)>,
    last_error: Option<String>,
}

impl RecoveryCoordinator {
    pub fn new(root: Option<PathBuf>) -> Self {
        Self::with_worker(root, RecoveryWorker::spawn())
    }

    pub fn with_worker(root: Option<PathBuf>, worker: RecoveryWorker) -> Self {
        Self {
            worker,
            scheduler: RecoveryScheduler::new(),
            root,
            project_key: None,
            last_error: None,
        }
    }

    pub fn set_root(&mut self, root: Option<PathBuf>) {
        self.root = root;
        self.project_key = None;
        self.scheduler.reset(0);
        self.last_error = None;
    }

    /// Poll recovery results and publish a debounced snapshot when needed.
    /// This never waits for disk IO and never raises the authoritative AutoSave
    /// saved generation.
    pub fn tick(&mut self, session: Option<&EditorSession>, now_ms: u64) -> bool {
        let mut changed = self.poll_outcomes(now_ms);
        let Some(session) = session else {
            return changed;
        };
        let Some(root) = self.root.clone() else {
            return changed;
        };
        let Some(document) = session.native_document.as_ref() else {
            return changed;
        };
        let Some(path) = document.path().cloned() else {
            return changed;
        };
        let key = (document.project_id().clone(), path.clone());
        if self.project_key.as_ref() != Some(&key) {
            self.project_key = Some(key.clone());
            self.scheduler.reset(document.saved_generation());
            self.last_error = None;
            changed = true;
        }
        self.scheduler
            .note_generation(now_ms, document.generation(), document.saved_generation());
        if self.scheduler.should_publish(now_ms) {
            let Some(generation) = self.scheduler.pending_generation() else {
                return changed;
            };
            let Some(native) = session.native_timeline.as_ref() else {
                return changed;
            };
            let job = RecoveryJob {
                snapshot: native.editor().snapshot(),
                project_id: key.0.clone(),
                original_project_path: path,
                document_generation: generation,
                last_saved_generation: document.saved_generation(),
                last_saved_unix_ms: document.last_saved_unix_ms(),
                created_at_unix_ms: document.created_at_unix_ms(),
                library_root: root,
            };
            match self.worker.submit(job) {
                Ok(()) => {
                    self.scheduler.note_started(generation);
                    self.last_error = None;
                    changed = true;
                }
                Err(error) => {
                    self.scheduler.note_finished(now_ms, generation, false);
                    self.last_error = Some(error.to_string());
                    changed = true;
                }
            }
        }
        changed
    }

    pub fn published_generation(&self) -> u64 {
        self.scheduler.published_generation()
    }

    pub fn is_pending(&self) -> bool {
        self.scheduler.is_pending()
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn next_deadline_ms(&self) -> Option<u64> {
        self.scheduler.next_deadline_ms()
    }

    pub fn metrics(&self) -> RecoveryMetrics {
        self.worker.metrics()
    }

    fn poll_outcomes(&mut self, now_ms: u64) -> bool {
        let mut changed = false;
        while let Some(outcome) = self.worker.poll() {
            if self
                .project_key
                .as_ref()
                .is_none_or(|(project_id, _)| project_id != &outcome.project_id)
            {
                continue;
            }
            match outcome.result {
                Ok(written) => {
                    self.scheduler.note_finished(
                        now_ms,
                        outcome.generation.max(written.generation),
                        true,
                    );
                    self.last_error = None;
                }
                Err(error) => {
                    self.scheduler
                        .note_finished(now_ms, outcome.generation, false);
                    self.last_error = Some(error.to_string());
                }
            }
            changed = true;
        }
        changed
    }
}

impl std::fmt::Debug for RecoveryCoordinator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecoveryCoordinator")
            .field("project_key", &self.project_key)
            .field("scheduler", &self.scheduler)
            .field("last_error", &self.last_error)
            .field("metrics", &self.worker.metrics())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_debounces_and_ignores_saved_generations() {
        let mut scheduler = RecoveryScheduler::new();
        scheduler.reset(5);
        scheduler.note_generation(1_000, 5, 5);
        assert!(!scheduler.should_publish(1_000));
        scheduler.note_generation(1_000, 6, 5);
        assert_eq!(scheduler.pending_generation(), Some(6));
        assert!(!scheduler.should_publish(1_299));
        assert!(scheduler.should_publish(1_300));
        scheduler.note_started(6);
        assert_eq!(scheduler.in_flight_generation(), Some(6));
        scheduler.note_finished(1_400, 6, true);
        assert_eq!(scheduler.published_generation(), 6);
        assert!(!scheduler.is_pending());
    }

    #[test]
    fn edit_during_publication_keeps_newer_generation_pending() {
        let mut scheduler = RecoveryScheduler::new();
        scheduler.reset(30);
        scheduler.note_generation(0, 31, 30);
        assert!(scheduler.should_publish(300));
        scheduler.note_started(31);
        scheduler.note_generation(400, 32, 30);
        assert_eq!(scheduler.pending_generation(), Some(32));
        scheduler.note_finished(500, 31, true);
        assert_eq!(scheduler.published_generation(), 31);
        assert_eq!(scheduler.pending_generation(), Some(32));
        assert!(!scheduler.should_publish(500));
        assert!(scheduler.should_publish(800));
        scheduler.note_started(32);
        scheduler.note_finished(900, 32, true);
        assert_eq!(scheduler.published_generation(), 32);
        assert!(!scheduler.is_pending());
    }

    #[test]
    fn failure_retries_without_claiming_publication() {
        let mut scheduler = RecoveryScheduler::new();
        scheduler.reset(5);
        scheduler.note_generation(0, 6, 5);
        assert!(scheduler.should_publish(300));
        scheduler.note_started(6);
        scheduler.note_finished(400, 6, false);
        assert_eq!(scheduler.published_generation(), 5);
        assert!(!scheduler.should_publish(400));
        assert!(scheduler.should_publish(5_400));
    }

    #[test]
    fn saved_generation_supersedes_pending_recovery() {
        let mut scheduler = RecoveryScheduler::new();
        scheduler.reset(5);
        scheduler.note_generation(0, 6, 5);
        assert!(scheduler.should_publish(300));
        scheduler.note_generation(350, 6, 6);
        assert!(!scheduler.is_pending());
        assert!(!scheduler.should_publish(400));
    }
}
