//! Application navigation and transient UI state.
//!
//! This module intentionally contains no engine project model, media handles,
//! or persistence. The editor session is a temporary in-memory UI session.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::auto_save_state::AutoSaveCoordinator;
use crate::editor_state::EditorSession;
use crate::media_library_state::{AssetAvailability, MediaDialogState, MediaLibraryState};
use crate::media_preview_state::{ClipPreviewStatus, MediaPreview};
use crate::native_project_state::{
    default_save_path, NativeDialogMode, NativeProjectDialog, PendingAction, UnsavedPrompt,
};
use crate::recovery_state::RecoveryCoordinator;
use crate::shortcuts::{CommandId, ShortcutManager, ShortcutRuntime, ShortcutsDialogState};
use capcut_core::time::FrameRate;
use capcut_engine::command::NativeEditor;
use capcut_engine::project::{Project, ProjectSnapshot};
use capcut_project::autosave::{SaveJob, SaveJobKind};
use capcut_project::{
    PreparedProject, ProjectDocumentMetadata, ProjectId, ProjectLibrary, ProjectLibraryEntry,
    RecoveryCandidateInfo, RecoveryCandidateStatus,
};
use capcut_renderer::PreviewRenderer;

use crate::workspace_preferences::{WorkspacePreferences, WorkspaceStore};

/// Top-level application screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Projects,
    Settings,
    Editor,
}

impl Screen {
    pub fn title(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Projects => "Projects",
            Self::Settings => "Settings",
            Self::Editor => "Editor",
        }
    }
}

/// Severity of a message shown in the status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeSeverity {
    Info,
    Warning,
    Error,
}

/// A short, single-line status message suitable for the status bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    severity: NoticeSeverity,
    message: String,
}

impl Notice {
    pub const MAX_MESSAGE_CHARS: usize = 280;

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            severity: NoticeSeverity::Info,
            message: sanitize(message.into()),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: NoticeSeverity::Warning,
            message: sanitize(message.into()),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: NoticeSeverity::Error,
            message: sanitize(message.into()),
        }
    }

    pub fn severity(&self) -> NoticeSeverity {
        self.severity
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug)]
enum PendingCreatePurpose {
    NewProject,
    RecoveredProject {
        candidate: Box<RecoveryCandidateInfo>,
    },
    ConflictCopy,
}

#[derive(Debug)]
struct PendingProjectCreation {
    prepared: PreparedProject,
    snapshot: ProjectSnapshot,
    purpose: PendingCreatePurpose,
}

#[derive(Debug, Clone)]
pub struct RecoveryPrompt {
    pub candidate: RecoveryCandidateInfo,
    pub error: Option<String>,
    pub busy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingDestination {
    NewProject,
    OpenProjectDialog,
    OpenLibrary(capcut_project::ProjectId),
    ReturnHome,
    Projects,
    Settings,
    Exit,
}

#[derive(Debug)]
struct PendingNavigation {
    destination: PendingDestination,
    required_generation: u64,
    problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryRenameDialog {
    pub project_id: capcut_project::ProjectId,
    pub name: String,
    pub error: Option<String>,
}

/// Navigation, native document/session, dialog, and status state for the application shell.
#[derive(Debug)]
pub struct AppState {
    screen: Screen,
    session: Option<EditorSession>,
    shortcuts: ShortcutManager,
    shortcuts_dialog: Option<ShortcutsDialogState>,
    workspace_preferences: WorkspacePreferences,
    workspace_store: Option<WorkspaceStore>,
    native_dialog: Option<NativeProjectDialog>,
    unsaved_prompt: Option<UnsavedPrompt>,
    native_error_dialog: Option<String>,
    media_preview: MediaPreview,
    media_library: MediaLibraryState,
    preview_renderer: Option<Arc<Mutex<PreviewRenderer>>>,
    preview_renderer_error: Option<String>,
    project_library: Option<ProjectLibrary>,
    project_library_error: Option<String>,
    project_library_entries: Vec<ProjectLibraryEntry>,
    auto_save: AutoSaveCoordinator,
    recovery: RecoveryCoordinator,
    recovery_candidates: Vec<RecoveryCandidateInfo>,
    recovery_prompt: Option<RecoveryPrompt>,
    recovery_prompt_suppressed: BTreeSet<ProjectId>,
    recovery_warning: Option<String>,
    pending_project_creation: Option<PendingProjectCreation>,
    pending_navigation: Option<PendingNavigation>,
    library_rename: Option<LibraryRenameDialog>,
    exit_requested: bool,
    about_open: bool,
    notice: Notice,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            screen: Screen::Home,
            session: None,
            shortcuts: ShortcutManager::default(),
            shortcuts_dialog: None,
            workspace_preferences: WorkspacePreferences::default(),
            workspace_store: None,
            native_dialog: None,
            unsaved_prompt: None,
            native_error_dialog: None,
            media_preview: MediaPreview::default(),
            media_library: MediaLibraryState::default(),
            preview_renderer: None,
            preview_renderer_error: None,
            project_library: None,
            project_library_error: None,
            project_library_entries: Vec::new(),
            auto_save: AutoSaveCoordinator::default(),
            recovery: RecoveryCoordinator::new(None),
            recovery_candidates: Vec::new(),
            recovery_prompt: None,
            recovery_prompt_suppressed: BTreeSet::new(),
            recovery_warning: None,
            pending_project_creation: None,
            pending_navigation: None,
            library_rename: None,
            exit_requested: false,
            about_open: false,
            notice: Notice::info(
                "Create a project to start editing. Projects and media references are saved automatically to the CutCraft project library.",
            ),
        }
    }
}

/// Milliseconds elapsed since `started`, or 0.0 when tracing is off.
fn elapsed_ms(started: Option<Instant>) -> f64 {
    started
        .map(|started| started.elapsed().as_secs_f64() * 1_000.0)
        .unwrap_or_default()
}

impl AppState {
    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn session(&self) -> Option<&EditorSession> {
        self.session.as_ref()
    }

    pub fn session_mut(&mut self) -> Option<&mut EditorSession> {
        self.session.as_mut()
    }

    pub fn install_project_library(&mut self, library: ProjectLibrary) {
        self.recovery.set_root(Some(library.root().to_path_buf()));
        self.project_library = Some(library);
        self.refresh_project_library();
        self.refresh_recovery_candidates();
        self.prompt_newest_recovery_candidate();
    }

    pub fn install_project_library_error(&mut self, error: impl Into<String>) {
        let error = error.into();
        self.project_library = None;
        self.project_library_error = Some(error.clone());
        self.notice = Notice::error(format!("Project library unavailable: {error}"));
    }

    pub fn refresh_project_library(&mut self) {
        let Some(library) = &self.project_library else {
            self.project_library_entries.clear();
            return;
        };
        match library.scan() {
            Ok(entries) => {
                self.project_library_entries = entries;
                self.project_library_error = None;
            }
            Err(error) => {
                self.project_library_error = Some(error.to_string());
                self.project_library_entries.clear();
            }
        }
    }

    pub fn project_library_entries(&self) -> &[ProjectLibraryEntry] {
        &self.project_library_entries
    }

    pub fn refresh_recovery_candidates(&mut self) {
        let Some(library) = &self.project_library else {
            self.recovery_candidates.clear();
            self.recovery_warning = None;
            return;
        };
        match library.scan_recovery_candidates(&self.project_library_entries) {
            Ok(candidates) => {
                self.recovery_warning = candidates
                    .iter()
                    .find_map(|candidate| match &candidate.status {
                        RecoveryCandidateStatus::Invalid(message) => Some(format!(
                            "Recovery data is invalid and was not used: {message}"
                        )),
                        _ => None,
                    })
                    .or_else(|| {
                        candidates
                            .iter()
                            .any(|candidate| candidate.status == RecoveryCandidateStatus::Orphan)
                            .then(|| {
                                "Recovery data for an unknown project was found and was not used."
                                    .to_owned()
                            })
                    });
                self.recovery_candidates = candidates;
            }
            Err(error) => {
                self.recovery_candidates.clear();
                self.recovery_warning =
                    Some(format!("Recovery data could not be scanned: {error}"));
            }
        }
    }

    pub fn recovery_candidates(&self) -> &[RecoveryCandidateInfo] {
        &self.recovery_candidates
    }

    pub fn recovery_warning(&self) -> Option<&str> {
        self.recovery_warning.as_deref()
    }

    pub fn recovery_prompt(&self) -> Option<&RecoveryPrompt> {
        self.recovery_prompt.as_ref()
    }

    pub fn set_recovery_prompt_error(&mut self, message: impl Into<String>) {
        if let Some(prompt) = &mut self.recovery_prompt {
            prompt.error = Some(message.into());
            prompt.busy = false;
        }
    }

    pub fn review_recovery_candidate(&mut self, path: &std::path::Path) -> Result<(), String> {
        let candidate = self
            .recovery_candidates
            .iter()
            .find(|candidate| candidate.path == path)
            .cloned()
            .ok_or_else(|| "The recovery candidate is no longer available.".to_owned())?;
        if candidate.status != RecoveryCandidateStatus::Newer {
            return Err("Only a validated newer recovery can be restored.".to_owned());
        }
        if let Some(project_id) = candidate.project_id.clone() {
            self.recovery_prompt_suppressed.remove(&project_id);
        }
        self.recovery_prompt = Some(RecoveryPrompt {
            candidate,
            error: None,
            busy: false,
        });
        Ok(())
    }

    pub fn dismiss_recovery_prompt(&mut self) {
        if let Some(prompt) = self.recovery_prompt.take() {
            if let Some(project_id) = prompt.candidate.project_id {
                self.recovery_prompt_suppressed.insert(project_id);
            }
        }
    }

    /// Prepare a recovered document and publish it as a new managed project.
    /// The original project and recovery artefact are never overwritten.
    pub fn restore_recovery_candidate(&mut self) -> Result<(), String> {
        if self.pending_project_creation.is_some() {
            return Err("Another project creation is already in progress.".to_owned());
        }
        let prompt = self
            .recovery_prompt
            .as_mut()
            .ok_or_else(|| "No recovery candidate is selected.".to_owned())?;
        let candidate = prompt.candidate.clone();
        if candidate.status != RecoveryCandidateStatus::Newer {
            prompt.error = Some("Only a validated newer recovery can be restored.".to_owned());
            return Err("Only a validated newer recovery can be restored.".to_owned());
        }
        let Some(library) = self.project_library.as_ref() else {
            prompt.error = Some("The project library is unavailable.".to_owned());
            return Err("The project library is unavailable.".to_owned());
        };
        let document = match library.load_recovery_document(&candidate) {
            Ok(document) => document,
            Err(error) => {
                let message = error.to_string();
                prompt.error = Some(message.clone());
                return Err(message);
            }
        };
        let current_path = candidate
            .project_id
            .as_ref()
            .and_then(|id| {
                self.project_library_entries
                    .iter()
                    .find(|entry| entry.project_id.as_ref() == Some(id))
            })
            .map(|entry| entry.path.clone());
        let (prepared, snapshot) =
            match library.prepare_recovered(&document, current_path.as_deref()) {
                Ok(prepared) => prepared,
                Err(error) => {
                    let message = error.to_string();
                    prompt.error = Some(message.clone());
                    return Err(message);
                }
            };
        let metadata = ProjectDocumentMetadata {
            project_id: prepared.project_id.clone(),
            created_at_unix_ms: prepared.created_at_unix_ms,
            save_generation: 0,
        };
        let job = SaveJob {
            kind: SaveJobKind::CreateProject,
            generation: 0,
            snapshot: snapshot.clone(),
            metadata,
            target: prepared.path.clone(),
            allowed_existing: None,
        };
        if let Err(error) = self.auto_save.submit_auxiliary(job) {
            library.discard_prepared(&prepared);
            let message = format!("Recovered project could not be created: {error}");
            prompt.error = Some(message.clone());
            return Err(message);
        }
        self.pending_project_creation = Some(PendingProjectCreation {
            prepared,
            snapshot,
            purpose: PendingCreatePurpose::RecoveredProject {
                candidate: Box::new(candidate),
            },
        });
        if let Some(prompt) = &mut self.recovery_prompt {
            prompt.busy = true;
            prompt.error = None;
        }
        self.notice = Notice::info("Creating recovered project...");
        Ok(())
    }

    pub fn open_last_saved_version(&mut self) -> Result<(), String> {
        let prompt = self
            .recovery_prompt
            .as_ref()
            .ok_or_else(|| "No recovery candidate is selected.".to_owned())?;
        let project_id = prompt
            .candidate
            .project_id
            .clone()
            .ok_or_else(|| "The recovery candidate has no project identity.".to_owned())?;
        self.dismiss_recovery_prompt();
        self.open_library_project(project_id);
        Ok(())
    }

    pub fn can_preserve_conflict_copy(&self) -> bool {
        self.pending_project_creation.is_none()
            && self.project_library.is_some()
            && self.session.as_ref().is_some_and(|session| {
                session
                    .native_document()
                    .is_some_and(|document| document.has_conflict() && document.path().is_some())
            })
    }

    /// Publish the current conflicted editor snapshot as a new managed project.
    /// The externally modified primary and the current session are untouched.
    pub fn preserve_conflict_copy(&mut self) -> Result<(), String> {
        if self.pending_project_creation.is_some() {
            return Err("Another project creation is already in progress.".to_owned());
        }
        let Some(library) = self.project_library.as_ref() else {
            return Err("The project library is unavailable.".to_owned());
        };
        let Some(session) = self.session.as_ref() else {
            return Err("There is no active project.".to_owned());
        };
        let Some(document) = session.native_document() else {
            return Err("There is no managed project to preserve.".to_owned());
        };
        if !document.has_conflict() {
            return Err("A recovery copy is only available for an external conflict.".to_owned());
        }
        let Some(path) = document.path().cloned() else {
            return Err("There is no managed project to preserve.".to_owned());
        };
        let Some(native) = session.native_timeline.as_ref() else {
            return Err("There is no native timeline to preserve.".to_owned());
        };
        let snapshot = native.editor().snapshot();
        let name = conflict_copy_name(&session.project_name);
        let (prepared, snapshot) = library
            .prepare_copy_of_snapshot(snapshot, &path, &name)
            .map_err(|error| error.to_string())?;
        let metadata = ProjectDocumentMetadata {
            project_id: prepared.project_id.clone(),
            created_at_unix_ms: prepared.created_at_unix_ms,
            save_generation: 0,
        };
        let job = SaveJob {
            kind: SaveJobKind::CreateProject,
            generation: 0,
            snapshot: snapshot.clone(),
            metadata,
            target: prepared.path.clone(),
            allowed_existing: None,
        };
        if let Err(error) = self.auto_save.submit_auxiliary(job) {
            library.discard_prepared(&prepared);
            return Err(format!("Recovery copy could not be created: {error}"));
        }
        self.pending_project_creation = Some(PendingProjectCreation {
            prepared,
            snapshot,
            purpose: PendingCreatePurpose::ConflictCopy,
        });
        self.notice = Notice::info("Saving a recovery copy of the current edits...");
        Ok(())
    }

    fn prompt_newest_recovery_candidate(&mut self) {
        if self.recovery_prompt.is_some() || self.pending_project_creation.is_some() {
            return;
        }
        let candidate = self.recovery_candidates.iter().find(|candidate| {
            candidate.status == RecoveryCandidateStatus::Newer
                && candidate
                    .project_id
                    .as_ref()
                    .is_none_or(|id| !self.recovery_prompt_suppressed.contains(id))
        });
        if let Some(candidate) = candidate.cloned() {
            self.recovery_prompt = Some(RecoveryPrompt {
                candidate,
                error: None,
                busy: false,
            });
        }
    }

    fn prompt_recovery_for(&mut self, project_id: ProjectId) {
        if self.recovery_prompt.is_some()
            || self.pending_project_creation.is_some()
            || self.recovery_prompt_suppressed.contains(&project_id)
        {
            return;
        }
        let candidate = self.recovery_candidates.iter().find(|candidate| {
            candidate.status == RecoveryCandidateStatus::Newer
                && candidate.project_id.as_ref() == Some(&project_id)
        });
        if let Some(candidate) = candidate.cloned() {
            self.recovery_prompt = Some(RecoveryPrompt {
                candidate,
                error: None,
                busy: false,
            });
        }
    }

    pub fn library_rename_dialog(&self) -> Option<&LibraryRenameDialog> {
        self.library_rename.as_ref()
    }

    pub fn library_rename_dialog_mut(&mut self) -> Option<&mut LibraryRenameDialog> {
        self.library_rename.as_mut()
    }

    pub fn begin_active_project_rename(&mut self) {
        let Some(document) = self
            .session
            .as_ref()
            .and_then(EditorSession::native_document)
        else {
            self.notice = Notice::warning("There is no active managed project to rename.");
            return;
        };
        self.library_rename = Some(LibraryRenameDialog {
            project_id: document.project_id().clone(),
            name: self
                .session
                .as_ref()
                .map(|session| session.project_name.clone())
                .unwrap_or_default(),
            error: None,
        });
    }

    pub fn begin_library_rename(&mut self, project_id: capcut_project::ProjectId) {
        let Some(entry) = self
            .project_library_entries
            .iter()
            .find(|entry| entry.project_id.as_ref() == Some(&project_id))
        else {
            self.notice = Notice::warning("The selected project is no longer available.");
            return;
        };
        self.library_rename = Some(LibraryRenameDialog {
            project_id,
            name: entry.display_name().to_owned(),
            error: None,
        });
    }

    pub fn close_library_rename(&mut self) {
        self.library_rename = None;
    }

    pub fn confirm_library_rename(&mut self) -> Result<(), String> {
        let Some(dialog) = self.library_rename.as_ref() else {
            return Err("No rename dialog is open.".to_owned());
        };
        let project_id = dialog.project_id.clone();
        let name = dialog.name.trim().to_owned();
        if name.is_empty() {
            return self.set_library_rename_error("Enter a project name.");
        }
        let active_project = self
            .session
            .as_ref()
            .and_then(EditorSession::native_document)
            .map(|document| document.project_id().clone());
        if active_project.as_ref() == Some(&project_id) {
            let result = self
                .session
                .as_mut()
                .ok_or_else(|| "The active project is unavailable.".to_owned())
                .and_then(|session| session.rename_native_project(name.clone()));
            match result {
                Ok(_) => {
                    self.library_rename = None;
                    self.notice =
                        Notice::info("Project renamed. AutoSave will publish the new name.");
                    Ok(())
                }
                Err(error) => self.set_library_rename_error(error),
            }
        } else {
            let Some(entry) = self
                .project_library_entries
                .iter()
                .find(|entry| entry.project_id.as_ref() == Some(&project_id))
                .cloned()
            else {
                return self
                    .set_library_rename_error("The selected project is no longer available.");
            };
            let result = self
                .project_library
                .as_ref()
                .ok_or_else(|| "The project library is unavailable.".to_owned())
                .and_then(|library| {
                    library
                        .rename(&entry, &name)
                        .map_err(|error| error.to_string())
                });
            match result {
                Ok(_) => {
                    self.library_rename = None;
                    self.notice = Notice::info("Project renamed.");
                    self.refresh_project_library();
                    Ok(())
                }
                Err(error) => self.set_library_rename_error(error),
            }
        }
    }

    pub fn duplicate_library_project(
        &mut self,
        project_id: &capcut_project::ProjectId,
    ) -> Result<capcut_project::CreatedProject, String> {
        if self
            .session
            .as_ref()
            .is_some_and(|session| self.auto_save.is_pending(session))
        {
            let error =
                "Wait for AutoSave to finish before duplicating the active project.".to_owned();
            self.notice = Notice::warning(error.clone());
            return Err(error);
        }
        let Some(entry) = self
            .project_library_entries
            .iter()
            .find(|entry| entry.project_id.as_ref() == Some(project_id))
            .cloned()
        else {
            let error = "The selected project is no longer available.".to_owned();
            self.notice = Notice::warning(error.clone());
            return Err(error);
        };
        let result = self
            .project_library
            .as_ref()
            .ok_or_else(|| "The project library is unavailable.".to_owned())
            .and_then(|library| library.duplicate(&entry).map_err(|error| error.to_string()));
        match result {
            Ok(created) => {
                self.refresh_project_library();
                self.notice = Notice::info(format!("Project duplicated as \"{}\".", created.name));
                Ok(created)
            }
            Err(error) => {
                self.notice = Notice::error(error.clone());
                Err(error)
            }
        }
    }

    fn set_library_rename_error<T>(&mut self, error: impl Into<String>) -> Result<T, String> {
        let error = error.into();
        if let Some(dialog) = &mut self.library_rename {
            dialog.error = Some(error.clone());
        }
        self.notice = Notice::error(error.clone());
        Err(error)
    }

    pub fn project_library_error(&self) -> Option<&str> {
        self.project_library_error.as_deref()
    }

    pub fn project_library_root(&self) -> Option<&std::path::Path> {
        self.project_library.as_ref().map(ProjectLibrary::root)
    }

    pub fn auto_save(&self) -> &AutoSaveCoordinator {
        &self.auto_save
    }

    pub fn is_creating_project(&self) -> bool {
        self.pending_project_creation.is_some()
    }

    /// Start managed project creation on the persistence worker. The active
    /// session is not replaced until the initial document is verified on disk.
    pub fn start_new_managed_project(&mut self) -> Result<(), String> {
        if self.pending_project_creation.is_some() {
            return Err("A project is already being created.".to_owned());
        }
        let Some(library) = self.project_library.as_ref() else {
            let error = self
                .project_library_error
                .clone()
                .unwrap_or_else(|| "The project library is unavailable.".to_owned());
            self.notice = Notice::error(error.clone());
            return Err(error);
        };
        let name = default_project_name(unix_millis_now());
        let fps = FrameRate::new(30, 1).map_err(|error| error.to_string())?;
        let project = Project::named(fps, name.clone()).map_err(|error| error.to_string())?;
        let editor = NativeEditor::new(project).map_err(|error| error.to_string())?;
        let mut snapshot = editor.snapshot();
        let prepared = library
            .prepare(&mut snapshot, &name)
            .map_err(|error| error.to_string())?;
        let metadata = ProjectDocumentMetadata {
            project_id: prepared.project_id.clone(),
            created_at_unix_ms: prepared.created_at_unix_ms,
            save_generation: 0,
        };
        let job = SaveJob {
            kind: SaveJobKind::CreateProject,
            generation: 0,
            snapshot: snapshot.clone(),
            metadata,
            target: prepared.path.clone(),
            allowed_existing: None,
        };
        if let Err(error) = self.auto_save.submit_auxiliary(job) {
            library.discard_prepared(&prepared);
            self.notice = Notice::error(format!("Project could not be created: {error}"));
            return Err(error);
        }
        self.pending_project_creation = Some(PendingProjectCreation {
            prepared,
            snapshot,
            purpose: PendingCreatePurpose::NewProject,
        });
        self.notice = Notice::info("Creating project...");
        Ok(())
    }

    /// Called from the application draw loop. Never performs file IO on the UI
    /// thread; it polls the bounded persistence worker and schedules repaints.
    pub fn update_auto_save(&mut self, context: &egui::Context) {
        self.update_auto_save_at(context, unix_millis_now());
    }

    pub fn update_auto_save_at(&mut self, context: &egui::Context, now_ms: u64) {
        let mut changed = self.auto_save.tick(self.session.as_mut(), now_ms);
        changed |= self.recovery.tick(self.session.as_ref(), now_ms);
        self.handle_project_creation_outcomes(now_ms);
        self.check_pending_navigation(now_ms);
        if changed {
            context.request_repaint();
        }
        let creating = self.pending_project_creation.is_some();
        let saving = self
            .session
            .as_ref()
            .and_then(EditorSession::auto_save_status)
            .is_some_and(|status| {
                status.kind() == crate::native_document::AutoSaveStatusKind::Saving
            });
        let in_flight = self.auto_save.metrics().in_flight;
        if creating || saving || in_flight || self.recovery.is_pending() {
            context.request_repaint_after(std::time::Duration::from_millis(16));
            return;
        }
        let auto_save_deadline = self.auto_save.next_deadline_ms();
        let recovery_deadline = self.recovery.next_deadline_ms();
        if let Some(deadline) = [auto_save_deadline, recovery_deadline]
            .into_iter()
            .flatten()
            .min()
        {
            let delay = deadline.saturating_sub(now_ms).max(16);
            context.request_repaint_after(std::time::Duration::from_millis(delay));
        }
    }

    pub fn pending_destination(&self) -> Option<&PendingDestination> {
        self.pending_navigation
            .as_ref()
            .map(|pending| &pending.destination)
    }

    pub fn navigation_wait_visible(&self) -> bool {
        self.pending_navigation
            .as_ref()
            .is_some_and(|pending| pending.problem.is_none())
    }

    pub fn navigation_problem(&self) -> Option<&str> {
        self.pending_navigation
            .as_ref()
            .and_then(|pending| pending.problem.as_deref())
    }

    pub fn retry_pending_navigation(&mut self) {
        let now = unix_millis_now();
        let Some(session) = self.session.as_mut() else {
            self.pending_navigation = None;
            return;
        };
        if !self.auto_save.retry_after_problem(session, now) {
            let _ = self.auto_save.request_immediate_save(session, now);
        }
        if let Some(pending) = &mut self.pending_navigation {
            pending.problem = None;
        }
    }

    pub fn cancel_pending_navigation(&mut self) {
        self.pending_navigation = None;
    }

    pub fn open_library_project(&mut self, project_id: capcut_project::ProjectId) {
        self.navigate_or_flush(PendingDestination::OpenLibrary(project_id));
    }

    fn navigate_or_flush(&mut self, destination: PendingDestination) {
        let Some(session) = self.session.as_ref() else {
            self.execute_destination(destination);
            return;
        };
        let managed = session
            .native_document
            .as_ref()
            .is_some_and(|document| document.path().is_some());
        if !managed {
            self.execute_destination(destination);
            return;
        }
        if self.auto_save.is_fully_saved(session) {
            self.execute_destination(destination);
            return;
        }
        let required_generation = session.persistent_generation().unwrap_or(0);
        self.pending_navigation = Some(PendingNavigation {
            destination,
            required_generation,
            problem: None,
        });
        let now = unix_millis_now();
        if let Some(session) = self.session.as_mut() {
            if let Err(error) = self.auto_save.request_immediate_save(session, now) {
                if let Some(pending) = &mut self.pending_navigation {
                    pending.problem = Some(error);
                }
            }
        }
    }

    fn check_pending_navigation(&mut self, now_ms: u64) {
        let Some(pending) = &self.pending_navigation else {
            return;
        };
        if pending.problem.is_some() {
            return;
        }
        let Some(session) = self.session.as_ref() else {
            self.pending_navigation = None;
            return;
        };
        if self.auto_save.is_fully_saved(session) {
            let required = pending.required_generation;
            let saved = session.persisted_generation().unwrap_or(0);
            if saved >= required && session.persistent_generation() == Some(saved) {
                let destination = self
                    .pending_navigation
                    .take()
                    .map(|pending| pending.destination)
                    .unwrap();
                self.execute_destination(destination);
                return;
            }
        }
        if let Some(status) = session.auto_save_status() {
            if matches!(
                status.kind(),
                crate::native_document::AutoSaveStatusKind::Failed
                    | crate::native_document::AutoSaveStatusKind::Conflict
            ) && !self.auto_save.metrics().in_flight
            {
                let message = status
                    .detail()
                    .unwrap_or("The project could not be saved.")
                    .to_owned();
                if let Some(pending) = &mut self.pending_navigation {
                    pending.problem = Some(message);
                }
            }
        }
        let _ = now_ms;
    }

    fn execute_destination(&mut self, destination: PendingDestination) {
        match destination {
            PendingDestination::NewProject => {
                let _ = self.start_new_managed_project();
            }
            PendingDestination::OpenProjectDialog => self.open_native_project_dialog(),
            PendingDestination::OpenLibrary(project_id) => {
                let _ = self.open_managed_project(&project_id);
            }
            PendingDestination::ReturnHome => self.finish_return_home(false),
            PendingDestination::Projects => {
                self.leave_editor();
                self.screen = Screen::Projects;
                self.refresh_project_library();
            }
            PendingDestination::Settings => {
                self.leave_editor();
                self.screen = Screen::Settings;
            }
            PendingDestination::Exit => {
                self.exit_requested = true;
            }
        }
    }

    fn open_managed_project(
        &mut self,
        project_id: &capcut_project::ProjectId,
    ) -> Result<(), String> {
        let Some(entry) = self
            .project_library_entries
            .iter()
            .find(|entry| entry.project_id.as_ref() == Some(project_id))
            .cloned()
        else {
            let error = "The selected project is no longer in the library.".to_owned();
            self.notice = Notice::error(error.clone());
            return Err(error);
        };
        if !entry.is_valid() {
            let error = match &entry.status {
                capcut_project::ProjectEntryStatus::Invalid(message) => {
                    format!("Invalid project entry: {message}")
                }
                capcut_project::ProjectEntryStatus::Valid => "Invalid project entry.".to_owned(),
            };
            self.notice = Notice::error(error.clone());
            return Err(error);
        }
        let Some(library) = &self.project_library else {
            return Err("The project library is unavailable.".to_owned());
        };
        let loaded = library
            .open_entry(&entry)
            .map_err(|error| error.to_string());
        match loaded {
            Ok(loaded) => {
                let mut session = match EditorSession::with_loaded_project(loaded) {
                    Ok(session) => session,
                    Err(error) => {
                        self.notice = Notice::error(error.clone());
                        return Err(error);
                    }
                };
                carry_layout(&mut session, self.session.as_ref());
                session.set_timeline_height_fraction(
                    self.workspace_preferences.timeline_height_fraction,
                );
                self.session = Some(session);
                self.screen = Screen::Editor;
                self.native_dialog = None;
                self.unsaved_prompt = None;
                self.native_error_dialog = None;
                self.media_preview.clear();
                self.clear_preview_renderer();
                self.media_library.reset();
                self.refresh_project_library();
                self.refresh_recovery_candidates();
                self.prompt_recovery_for(project_id.clone());
                self.notice = Notice::info(format!("Project opened: {}.", entry.display_name()));
                Ok(())
            }
            Err(error) => {
                self.notice = Notice::error(error.clone());
                Err(error)
            }
        }
    }

    fn handle_project_creation_outcomes(&mut self, now_ms: u64) {
        for outcome in self.auto_save.take_auxiliary_outcomes() {
            if outcome.kind != SaveJobKind::CreateProject {
                continue;
            }
            let Some(pending) = self.pending_project_creation.take() else {
                continue;
            };
            if pending.prepared.project_id != outcome.project_id {
                self.pending_project_creation = Some(pending);
                continue;
            }
            let PendingProjectCreation {
                prepared,
                snapshot,
                purpose,
            } = pending;
            match outcome.result {
                Ok(saved) => match purpose {
                    PendingCreatePurpose::ConflictCopy => {
                        self.notice = Notice::info(format!(
                            "Recovery copy saved as \"{}\" in the managed project library.",
                            prepared.name
                        ));
                        self.refresh_project_library();
                        self.refresh_recovery_candidates();
                    }
                    PendingCreatePurpose::NewProject
                    | PendingCreatePurpose::RecoveredProject { .. } => {
                        let mut session = match EditorSession::from_created_project(
                            &prepared, snapshot, &saved,
                        ) {
                            Ok(session) => session,
                            Err(error) => {
                                if let Some(library) = &self.project_library {
                                    library.discard_prepared(&prepared);
                                }
                                self.notice =
                                    Notice::error(format!("Project could not be opened: {error}"));
                                continue;
                            }
                        };
                        carry_layout(&mut session, self.session.as_ref());
                        let height = self
                            .session
                            .as_ref()
                            .and_then(|current| current.timeline_height_fraction)
                            .or(self.workspace_preferences.timeline_height_fraction);
                        session.set_timeline_height_fraction(height);
                        self.session = Some(session);
                        self.screen = Screen::Editor;
                        self.native_dialog = None;
                        self.unsaved_prompt = None;
                        self.native_error_dialog = None;
                        self.media_preview.clear();
                        self.clear_preview_renderer();
                        self.media_library.reset();
                        if let PendingCreatePurpose::RecoveredProject { candidate } = purpose {
                            if let Some(library) = &self.project_library {
                                let _ = library.discard_recovery(&candidate);
                            }
                            self.recovery_prompt = None;
                            self.refresh_project_library();
                            self.refresh_recovery_candidates();
                            self.notice = Notice::info(format!(
                                "Recovered project created and saved as \"{}\".",
                                prepared.name
                            ));
                        } else {
                            self.refresh_project_library();
                            self.notice = Notice::info(format!(
                                "Project created and saved automatically (generation {} at {} ms).",
                                saved.save_generation, now_ms
                            ));
                        }
                    }
                },
                Err(error) => {
                    if let Some(library) = &self.project_library {
                        library.discard_prepared(&prepared);
                    }
                    match purpose {
                        PendingCreatePurpose::NewProject => {
                            self.notice =
                                Notice::error(format!("Project creation failed: {error}"));
                        }
                        PendingCreatePurpose::RecoveredProject { candidate } => {
                            if let Some(prompt) = &mut self.recovery_prompt {
                                prompt.busy = false;
                                prompt.error = Some(error.to_string());
                            } else {
                                self.recovery_prompt = Some(RecoveryPrompt {
                                    candidate: *candidate,
                                    error: Some(error.to_string()),
                                    busy: false,
                                });
                            }
                            self.notice = Notice::error(format!(
                                "Recovered project creation failed; recovery data was kept: {error}"
                            ));
                        }
                        PendingCreatePurpose::ConflictCopy => {
                            self.notice = Notice::error(format!(
                                "Recovery copy failed; the current session and external file were preserved: {error}"
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Split borrows for editor chrome, shortcuts, and the Preview media state.
    pub fn session_shortcuts_media_mut(
        &mut self,
    ) -> Option<(
        &mut EditorSession,
        &ShortcutManager,
        &mut MediaPreview,
        &mut MediaLibraryState,
    )> {
        let session = self.session.as_mut()?;
        Some((
            session,
            &self.shortcuts,
            &mut self.media_preview,
            &mut self.media_library,
        ))
    }

    pub fn native_project_dialog(&self) -> Option<&NativeProjectDialog> {
        self.native_dialog.as_ref()
    }

    pub fn native_project_dialog_mut(&mut self) -> Option<&mut NativeProjectDialog> {
        self.native_dialog.as_mut()
    }

    pub fn close_native_project_dialog(&mut self) {
        self.native_dialog = None;
    }

    pub fn unsaved_prompt(&self) -> Option<&UnsavedPrompt> {
        self.unsaved_prompt.as_ref()
    }

    pub fn native_error_dialog(&self) -> Option<&str> {
        self.native_error_dialog.as_deref()
    }

    pub fn close_native_error_dialog(&mut self) {
        self.native_error_dialog = None;
    }

    pub fn media_library(&self) -> &MediaLibraryState {
        &self.media_library
    }

    pub fn media_library_mut(&mut self) -> &mut MediaLibraryState {
        &mut self.media_library
    }

    pub fn media_import_dialog(&self) -> Option<&MediaDialogState> {
        self.media_library.dialog()
    }

    pub fn open_media_import_dialog(&mut self) {
        if self.screen != Screen::Editor || self.session.is_none() {
            self.notice = Notice::warning("Open a native project before importing media.");
            return;
        }
        self.media_library.open_import_dialog();
    }

    pub fn open_media_relink_dialog(&mut self, asset: capcut_engine::asset::MediaAssetId) {
        let Some(session) = self.session.as_ref() else {
            self.notice = Notice::warning("Open a native project before relinking media.");
            return;
        };
        self.media_library.open_relink_dialog(session, asset);
    }

    pub fn close_media_import_dialog(&mut self) {
        self.media_library.close_dialog();
    }

    pub fn confirm_media_import(&mut self) -> Result<(), String> {
        let Some(dialog) = self.media_library.dialog() else {
            return Err("No media dialog is open.".to_owned());
        };
        let path = dialog.path.clone();
        let Some(session) = self.session.as_ref() else {
            return Err("There is no active project.".to_owned());
        };
        match self.media_library.start_dialog_probe(session, &path) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.notice = Notice::error(error.clone());
                Err(error)
            }
        }
    }

    pub fn media_availability(
        &self,
        asset: capcut_engine::asset::MediaAssetId,
    ) -> Option<&AssetAvailability> {
        self.media_library.availability(asset)
    }

    pub fn append_media_asset(
        &mut self,
        asset: capcut_engine::asset::MediaAssetId,
    ) -> Result<(), String> {
        let result = self
            .session
            .as_mut()
            .ok_or_else(|| "There is no active project.".to_owned())
            .and_then(|session| session.append_media_asset(asset));
        match result {
            Ok(_) => {
                self.notice =
                    Notice::info("Media clip appended to the main track. AutoSave scheduled.");
                Ok(())
            }
            Err(error) => {
                self.notice = Notice::error(error.clone());
                Err(error)
            }
        }
    }

    pub fn update_media_library(&mut self, context: &egui::Context) {
        let changed = if let Some(session) = self.session.as_mut() {
            self.media_library.update(session)
        } else {
            false
        };
        if let Some(message) = self.media_library.take_notice() {
            self.notice = Notice::info(message);
        }
        if changed {
            context.request_repaint();
        }
        if self.media_library.is_pending() {
            context.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    pub fn media_preview(&self) -> &MediaPreview {
        &self.media_preview
    }

    pub fn media_preview_mut(&mut self) -> &mut MediaPreview {
        &mut self.media_preview
    }

    pub fn install_preview_renderer(&mut self, renderer: PreviewRenderer) {
        self.preview_renderer = Some(Arc::new(Mutex::new(renderer)));
        self.preview_renderer_error = None;
    }

    pub fn preview_renderer(&self) -> Option<Arc<Mutex<PreviewRenderer>>> {
        self.preview_renderer.clone()
    }

    pub fn preview_renderer_error(&self) -> Option<&str> {
        self.preview_renderer_error.as_deref()
    }

    pub fn preview_renderer_metrics(&self) -> Option<capcut_renderer::PreviewRendererMetrics> {
        self.preview_renderer
            .as_ref()
            .and_then(|renderer| renderer.lock().ok().map(|renderer| renderer.metrics()))
    }

    pub fn clear_preview_renderer(&mut self) {
        if let Some(renderer) = &self.preview_renderer {
            if let Ok(mut renderer) = renderer.lock() {
                renderer.clear();
            }
        }
    }

    /// Poll the decoder worker, resolve the active clip, and request its frame.
    pub fn update_media_preview(&mut self, context: &egui::Context) {
        // Sub-phase timers so a slow UI tick can be attributed instead of guessed
        // at; they only cost an Instant::now when CUTCRAFT_RENDER_LOG is set.
        let trace = std::env::var_os("CUTCRAFT_RENDER_LOG").is_some();
        let poll_started = trace.then(Instant::now);
        let polled = self.media_preview.poll(&mut self.media_library);
        let poll_ms = elapsed_ms(poll_started);
        let sync_started = trace.then(Instant::now);
        if self.screen == Screen::Editor {
            if let Some(session) = self.session.as_ref() {
                self.media_preview.sync(session, &self.media_library);
            }
        }
        let sync_ms = elapsed_ms(sync_started);
        if let ClipPreviewStatus::Failed(message) = self.media_preview.status() {
            if polled {
                self.notice = Notice::error(message.clone());
            }
        }
        let mut gpu_ready = false;
        let gpu_started = trace.then(Instant::now);
        if let Some(renderer) = self.preview_renderer.clone() {
            if let Some(frame) = self.media_preview.frame() {
                match renderer.lock() {
                    Ok(mut renderer) => match renderer.upload_frame(frame) {
                        Ok(upload) => {
                            gpu_ready = true;
                            self.preview_renderer_error = None;
                            if matches!(upload, capcut_renderer::PreviewUpload::Uploaded)
                                && std::env::var_os("CUTCRAFT_RENDER_LOG").is_some()
                            {
                                eprintln!(
                                    "[cutcraft-renderer] upload identity={:?} metrics={:?}",
                                    renderer.identity(),
                                    renderer.metrics()
                                );
                            }
                        }
                        Err(error) => {
                            renderer.clear();
                            let message = error.to_string();
                            eprintln!("[cutcraft-renderer] upload failed: {message}");
                            self.preview_renderer_error = Some(message);
                        }
                    },
                    Err(_) => {
                        self.preview_renderer_error =
                            Some("preview renderer lock was poisoned".to_owned());
                    }
                }
            }
        }
        let gpu_ms = elapsed_ms(gpu_started);
        let fallback_started = trace.then(Instant::now);
        let uploaded = if gpu_ready {
            false
        } else {
            self.media_preview.ensure_texture(context)
        };
        let fallback_ms = elapsed_ms(fallback_started);
        if trace && polled {
            eprintln!(
                "[cutcraft-ui.media] poll_ms={:.3} sync_ms={:.3} gpu_ms={:.3} fallback_ms={:.3} gpu={gpu_ready}",
                poll_ms, sync_ms, gpu_ms, fallback_ms
            );
        }
        let pending = self.media_preview.is_pending();
        if polled || uploaded {
            context.request_repaint();
        }
        if pending {
            context.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    pub fn take_exit_request(&mut self) -> bool {
        std::mem::take(&mut self.exit_requested)
    }

    pub fn request_new_project(&mut self) {
        self.navigate_or_flush(PendingDestination::NewProject);
    }

    pub fn request_open_native_project(&mut self) {
        self.navigate_or_flush(PendingDestination::OpenProjectDialog);
    }

    pub fn open_native_project_dialog(&mut self) {
        self.native_dialog = Some(NativeProjectDialog::open());
        self.unsaved_prompt = None;
        self.native_error_dialog = None;
    }

    pub fn save_native_project(&mut self) {
        let Some(session) = self.session.as_ref() else {
            self.notice = Notice::warning("There is no active project to save.");
            return;
        };
        let Some(document) = session.native_document() else {
            self.notice = Notice::warning("This session has no native project to save.");
            return;
        };
        if document.path().is_none() {
            self.open_save_as_dialog_with_pending(None);
            return;
        }
        if let Err(error) = self.save_to_current_path() {
            self.report_native_save_error(error);
        }
    }

    pub fn open_save_as_dialog(&mut self) {
        self.open_save_as_dialog_with_pending(None);
    }

    pub fn confirm_native_save_as(&mut self) -> Result<(), String> {
        let Some(dialog) = self.native_dialog.as_ref() else {
            return Err("No save dialog is open.".to_owned());
        };
        if dialog.mode() != NativeDialogMode::SaveAs {
            return Err("No save dialog is open.".to_owned());
        }
        let raw = dialog.path().trim().to_owned();
        if raw.is_empty() {
            return self.set_native_dialog_error("Enter a .cutcraft file path.");
        }
        let target = match normalize_save_target(&raw) {
            Ok(target) => target,
            Err(error) => return self.set_native_dialog_error(error),
        };
        let pending = dialog.pending_after_save();
        if let Some(dialog) = &mut self.native_dialog {
            *dialog.path_mut() = target.to_string_lossy().into_owned();
            dialog.clear_error();
        }

        let Some(session) = self.session.as_ref() else {
            return self.set_native_dialog_error("There is no active project to save.");
        };
        let Some(document) = session.native_document() else {
            return self.set_native_dialog_error("This session has no native project to save.");
        };
        let Some(native) = session.native_timeline.as_ref() else {
            return self.set_native_dialog_error("This session has no native project to save.");
        };
        let current_id = document.project_id().clone();
        let existing = document.snapshot().cloned();
        let same_file = existing
            .as_ref()
            .is_some_and(|snapshot| snapshot.matches_path(&target));
        let candidate_id = if existing.is_some() && !same_file {
            capcut_project::ProjectId::generate()
        } else {
            current_id
        };
        let allowed_existing = if same_file { existing.as_ref() } else { None };
        match capcut_project::save(native.editor(), &candidate_id, &target, allowed_existing) {
            Ok(snapshot) => {
                if let Some(document) = self
                    .session
                    .as_mut()
                    .and_then(EditorSession::native_document_mut)
                {
                    document.commit_saved(candidate_id, snapshot.clone());
                }
                self.native_dialog = None;
                self.notice =
                    Notice::info(format!("Project saved to {}.", snapshot.path().display()));
                if let Some(action) = pending {
                    self.run_pending_action(action, false);
                }
                Ok(())
            }
            Err(error) => {
                let message = error.to_string();
                if let Some(dialog) = &mut self.native_dialog {
                    dialog.set_error(message.clone());
                    dialog.clear_pending_after_save();
                }
                self.notice = Notice::error(message.clone());
                Err(message)
            }
        }
    }

    pub fn confirm_native_open(&mut self) -> Result<(), String> {
        let Some(dialog) = self.native_dialog.as_ref() else {
            return Err("No open dialog is open.".to_owned());
        };
        if dialog.mode() != NativeDialogMode::Open {
            return Err("No open dialog is open.".to_owned());
        }
        let raw = dialog.path().trim().to_owned();
        if raw.is_empty() {
            return self.set_native_dialog_error("Enter a .cutcraft project path.");
        }
        let target = match normalize_open_target(&raw) {
            Ok(target) => target,
            Err(error) => return self.set_native_dialog_error(error),
        };
        if let Some(dialog) = &mut self.native_dialog {
            *dialog.path_mut() = target.to_string_lossy().into_owned();
            dialog.clear_error();
        }
        match capcut_project::open(&target) {
            Ok(loaded) => {
                let mut session = match EditorSession::with_loaded_project(loaded) {
                    Ok(session) => session,
                    Err(error) => return self.set_native_dialog_error(error),
                };
                carry_layout(&mut session, self.session.as_ref());
                let height = self
                    .session
                    .as_ref()
                    .and_then(|current| current.timeline_height_fraction)
                    .or(self.workspace_preferences.timeline_height_fraction);
                session.set_timeline_height_fraction(height);
                self.session = Some(session);
                self.screen = Screen::Editor;
                self.native_dialog = None;
                self.unsaved_prompt = None;
                self.native_error_dialog = None;
                self.media_preview.clear();
                self.clear_preview_renderer();
                self.media_library.reset();
                self.notice = Notice::info(format!("Project opened: {}.", target.display()));
                Ok(())
            }
            Err(error) => {
                let message = error.to_string();
                if let Some(dialog) = &mut self.native_dialog {
                    dialog.set_error(message.clone());
                }
                self.notice = Notice::error(message.clone());
                Err(message)
            }
        }
    }

    pub fn resolve_unsaved_save(&mut self) {
        let Some(prompt) = self.unsaved_prompt.take() else {
            return;
        };
        let action = prompt.action();
        let has_path = self
            .session
            .as_ref()
            .and_then(EditorSession::native_document)
            .is_some_and(|document| document.path().is_some());
        if !has_path {
            self.open_save_as_dialog_with_pending(Some(action));
            return;
        }
        match self.save_to_current_path() {
            Ok(()) => self.run_pending_action(action, false),
            Err(error) => {
                self.report_native_save_error(error);
            }
        }
    }

    pub fn resolve_unsaved_discard(&mut self) {
        let Some(prompt) = self.unsaved_prompt.take() else {
            return;
        };
        self.run_pending_action(prompt.action(), true);
    }

    pub fn cancel_unsaved_prompt(&mut self) {
        self.unsaved_prompt = None;
    }

    fn set_native_dialog_error<T>(&mut self, error: impl Into<String>) -> Result<T, String> {
        let error = error.into();
        if let Some(dialog) = &mut self.native_dialog {
            dialog.set_error(error.clone());
        }
        self.notice = Notice::error(error.clone());
        Err(error)
    }

    fn open_save_as_dialog_with_pending(&mut self, pending: Option<PendingAction>) {
        let Some(session) = self.session.as_ref() else {
            self.notice = Notice::warning("There is no active project to save.");
            return;
        };
        let Some(document) = session.native_document() else {
            self.notice = Notice::warning("This session has no native project to save.");
            return;
        };
        let default_path = document
            .path()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| default_save_path(&session.project_name));
        self.native_dialog = Some(NativeProjectDialog::save_as(default_path, pending));
        self.unsaved_prompt = None;
        self.native_error_dialog = None;
    }

    fn save_to_current_path(&mut self) -> Result<(), String> {
        let Some(session) = self.session.as_ref() else {
            return Err("There is no active project to save.".to_owned());
        };
        let Some(document) = session.native_document() else {
            return Err("This session has no native project to save.".to_owned());
        };
        let Some(native) = session.native_timeline.as_ref() else {
            return Err("This session has no native project to save.".to_owned());
        };
        let Some(path) = document.path().cloned() else {
            return Err("Choose a .cutcraft file path before saving.".to_owned());
        };
        let Some(existing) = document.snapshot().cloned() else {
            return Err("The active project has no saved file yet.".to_owned());
        };
        let project_id = document.project_id().clone();
        match capcut_project::save(native.editor(), &project_id, &path, Some(&existing)) {
            Ok(snapshot) => {
                if let Some(document) = self
                    .session
                    .as_mut()
                    .and_then(EditorSession::native_document_mut)
                {
                    document.commit_saved(project_id, snapshot.clone());
                }
                self.notice =
                    Notice::info(format!("Project saved to {}.", snapshot.path().display()));
                Ok(())
            }
            Err(error) => Err(error.to_string()),
        }
    }

    fn report_native_save_error(&mut self, error: String) {
        self.native_error_dialog = Some(error.clone());
        self.notice = Notice::error(error);
    }

    fn run_pending_action(&mut self, action: PendingAction, discard_session: bool) {
        self.unsaved_prompt = None;
        match action {
            PendingAction::NewProject => {
                let _ = self.start_new_managed_project();
            }
            PendingAction::OpenProject => self.open_native_project_dialog(),
            PendingAction::ReturnHome => self.finish_return_home(discard_session),
            PendingAction::Exit => {
                if discard_session {
                    self.session = None;
                }
                self.exit_requested = true;
            }
        }
    }

    fn finish_return_home(&mut self, discard_session: bool) {
        if discard_session {
            self.session = None;
            self.media_preview.clear();
            self.clear_preview_renderer();
            self.media_library.reset();
        } else {
            self.leave_editor();
        }
        self.screen = Screen::Home;
        self.native_dialog = None;
        self.unsaved_prompt = None;
        self.native_error_dialog = None;
        self.refresh_project_library();
        self.refresh_recovery_candidates();
    }

    /// Returns `false` when closing must wait for a cancelled copy to clean up
    /// or for the unsaved-changes decision.
    pub fn request_app_close(&mut self) -> bool {
        if self.native_dialog.is_some()
            || self.unsaved_prompt.is_some()
            || self.native_error_dialog.is_some()
            || self.pending_navigation.is_some()
            || self.pending_project_creation.is_some()
        {
            return false;
        }
        if let Some(session) = self.session.as_ref() {
            let managed = session
                .native_document
                .as_ref()
                .is_some_and(|document| document.path().is_some());
            if managed {
                if self.auto_save.is_fully_saved(session) {
                    return true;
                }
                self.navigate_or_flush(PendingDestination::Exit);
                return false;
            }
            if session.native_is_dirty() {
                self.unsaved_prompt = Some(UnsavedPrompt::new(PendingAction::Exit));
                return false;
            }
        }
        true
    }

    pub fn shortcuts(&self) -> &ShortcutManager {
        &self.shortcuts
    }

    pub fn install_shortcuts(&mut self, shortcuts: ShortcutManager) {
        self.shortcuts = shortcuts;
        self.shortcuts_dialog = None;
    }

    pub fn install_workspace_preferences(
        &mut self,
        preferences: WorkspacePreferences,
        store: Option<WorkspaceStore>,
    ) {
        self.workspace_preferences = preferences;
        self.workspace_store = store;
        if let Some(session) = &mut self.session {
            session.set_timeline_height_fraction(preferences.timeline_height_fraction);
        }
    }

    pub fn workspace_preferences(&self) -> WorkspacePreferences {
        self.workspace_preferences
    }

    /// Save only when the editor requested it after a drag, reset or layout command.
    pub fn persist_workspace_layout_if_requested(&mut self) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        if !session.take_layout_save_request() {
            return false;
        }
        let preferences = WorkspacePreferences {
            timeline_height_fraction: session.timeline_height_fraction,
        };
        self.workspace_preferences = preferences;
        if let Some(store) = &self.workspace_store {
            if let Err(error) = store.save(preferences) {
                self.notice =
                    Notice::warning(format!("Workspace layout could not be saved: {error}"));
            }
        }
        true
    }

    pub fn shortcuts_open(&self) -> bool {
        self.shortcuts_dialog.is_some()
    }

    pub fn open_shortcuts(&mut self) {
        self.shortcuts_dialog = Some(ShortcutsDialogState::new(&self.shortcuts));
    }

    pub fn close_shortcuts(&mut self) {
        self.shortcuts_dialog = None;
    }

    pub fn finish_shortcuts_saved(&mut self) {
        self.shortcuts_dialog = None;
        self.notice = Notice::info("Keyboard shortcuts saved.");
    }

    /// Split borrows for the modal renderer: manager for labels/save, dialog for
    /// pending edits and capture state.
    pub fn shortcuts_dialog_parts_mut(
        &mut self,
    ) -> Option<(&mut ShortcutManager, &mut ShortcutsDialogState)> {
        let dialog = self.shortcuts_dialog.as_mut()?;
        Some((&mut self.shortcuts, dialog))
    }

    /// Consume configured shortcuts and execute their real handlers.
    ///
    /// Text focus, open modals and capture mode block dispatch before any event
    /// is consumed, so typing and dialog navigation remain isolated. Multiple
    /// distinct key events in one frame are supported, but dispatch stops as
    /// soon as an action opens a modal or hides the timeline.
    pub fn handle_shortcuts(&mut self, context: &egui::Context) {
        if self.export_open()
            || self.shortcuts_open()
            || self.about_open
            || self.native_dialog.is_some()
            || self.unsaved_prompt.is_some()
            || self.native_error_dialog.is_some()
            || self.pending_navigation.is_some()
        {
            return;
        }
        for _ in 0..8 {
            let runtime = self.shortcut_runtime(context);
            let Some(command) = self.shortcuts.dispatch(context, runtime) else {
                break;
            };
            self.execute_shortcut_command(command);
        }
    }

    fn shortcut_runtime(&self, context: &egui::Context) -> ShortcutRuntime {
        let editor_open = self.screen == Screen::Editor && self.session.is_some();
        ShortcutRuntime {
            editor_open,
            timeline_visible: editor_open
                && self
                    .session
                    .as_ref()
                    .is_some_and(|session| session.timeline_visible),
            has_selection: self
                .session
                .as_ref()
                .is_some_and(|session| !session.timeline_selection_context().is_empty()),
            text_focus: context.text_edit_focused(),
            modal_open: self.export_open()
                || self.shortcuts_open()
                || self.about_open
                || self.native_dialog.is_some()
                || self.unsaved_prompt.is_some()
                || self.native_error_dialog.is_some()
                || self.pending_navigation.is_some(),
            capturing: self
                .shortcuts_dialog
                .as_ref()
                .is_some_and(|dialog| dialog.capturing.is_some()),
        }
    }

    fn execute_shortcut_command(&mut self, command: CommandId) -> bool {
        match command {
            CommandId::NewProject => {
                self.request_new_project();
                true
            }
            CommandId::OpenProject => {
                if self.screen != Screen::Editor {
                    return false;
                }
                self.request_open_native_project();
                true
            }
            CommandId::SaveProject => {
                if self.screen != Screen::Editor {
                    return false;
                }
                let now = unix_millis_now();
                if let Some(session) = self.session.as_mut() {
                    if let Err(error) = self.auto_save.request_immediate_save(session, now) {
                        self.notice = Notice::error(error);
                    } else {
                        self.notice = Notice::info("Saving project...");
                    }
                }
                true
            }
            CommandId::ShortcutsDialog => {
                self.open_shortcuts();
                true
            }
            CommandId::Export => self.open_export(),
            CommandId::SnappingToggle
            | CommandId::LinkedSelection
            | CommandId::ZoomIn
            | CommandId::ZoomOut
            | CommandId::DeselectAll
            | CommandId::PreviousFrame
            | CommandId::NextFrame
            | CommandId::JumpToBeginning
            | CommandId::JumpToEnd
            | CommandId::SplitAtPlayhead => {
                if self.screen != Screen::Editor {
                    return false;
                }
                let Some(session) = self.session.as_mut() else {
                    return false;
                };
                session.execute_shortcut_command(command)
            }
            _ => false,
        }
    }

    fn export_open(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.export_open)
    }

    pub fn has_session(&self) -> bool {
        self.session.is_some()
    }

    /// Start a new native project and enter the editor. Production sessions
    /// are empty; only the explicit QA flag seeds synthetic clips.
    pub fn create_temporary_project(&mut self) {
        let native_qa = cfg!(debug_assertions)
            && std::env::var("CUTCRAFT_NATIVE_QA").ok().as_deref() == Some("1");
        let mut creation_error = None;
        let mut session = if native_qa {
            match EditorSession::with_native_qa() {
                Ok(session) => session,
                Err(error) => {
                    creation_error = Some(error);
                    EditorSession::default()
                }
            }
        } else {
            match EditorSession::new_native() {
                Ok(session) => session,
                Err(error) => {
                    creation_error = Some(error);
                    EditorSession::default()
                }
            }
        };
        session.set_timeline_height_fraction(self.workspace_preferences.timeline_height_fraction);
        self.session = Some(session);
        self.native_dialog = None;
        self.unsaved_prompt = None;
        self.native_error_dialog = None;
        self.media_preview.clear();
        self.clear_preview_renderer();
        self.media_library.reset();
        self.screen = Screen::Editor;
        self.notice = match creation_error {
            Some(error) => Notice::error(format!("Native project could not be created: {error}")),
            None if native_qa => Notice::info(
                "Native QA project loaded. This synthetic project is not production data.",
            ),
            None => {
                Notice::info("New native project created. Save it to a .cutcraft file when ready.")
            }
        };
    }

    /// Return to the existing native session.
    pub fn resume_editor(&mut self) -> bool {
        if self.session.is_some() {
            self.screen = Screen::Editor;
            true
        } else {
            false
        }
    }

    pub fn go_home(&mut self) {
        self.navigate_or_flush(PendingDestination::ReturnHome);
    }

    pub fn go_projects(&mut self) {
        self.navigate_or_flush(PendingDestination::Projects);
    }

    pub fn go_settings(&mut self) {
        self.navigate_or_flush(PendingDestination::Settings);
    }

    /// Open the export modal for the current editor session.
    pub fn open_export(&mut self) -> bool {
        if self.screen != Screen::Editor {
            return false;
        }

        if let Some(session) = &mut self.session {
            session.export_open = true;
            true
        } else {
            false
        }
    }

    pub fn close_export(&mut self) {
        if let Some(session) = &mut self.session {
            session.export_open = false;
        }
    }

    /// Show every editor panel again.
    pub fn reset_layout(&mut self) {
        if let Some(session) = &mut self.session {
            session.reset_layout();
        }
    }

    pub fn notice(&self) -> &Notice {
        &self.notice
    }

    pub fn report_error(&mut self, message: impl Into<String>) {
        self.notice = Notice::error(message);
    }

    pub fn report_warning(&mut self, message: impl Into<String>) {
        self.notice = Notice::warning(message);
    }

    pub fn clear_notice(&mut self) {
        self.notice = Notice::info(
            "Native .cutcraft projects can be created, saved, opened, and edited. Real H.264 frames can be previewed from an explicitly bound local video; media import, playback, audio, and export remain unavailable.",
        );
    }

    pub fn about_open(&self) -> bool {
        self.about_open
    }

    pub fn open_about(&mut self) {
        self.about_open = true;
    }

    pub fn close_about(&mut self) {
        self.about_open = false;
    }

    fn leave_editor(&mut self) {
        if let Some(session) = &mut self.session {
            session.export_open = false;
        }
    }
}

fn unix_millis_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

fn conflict_copy_name(original: &str) -> String {
    let suffix = " (Recovery Copy)";
    let limit = 256usize.saturating_sub(suffix.len());
    let mut base = String::new();
    for character in original.chars() {
        if character.is_control() {
            continue;
        }
        if base.len() + character.len_utf8() > limit {
            break;
        }
        base.push(character);
    }
    let name = format!("{base}{suffix}");
    if base.trim().is_empty() {
        "Project (Recovery Copy)".to_owned()
    } else {
        name
    }
}

fn default_project_name(now_ms: u64) -> String {
    let seconds = (now_ms / 1000) as i64;
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400) as u32;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    format!("Project {year:04}-{month:02}-{day:02} {hour:02}:{minute:02}")
}

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year, month as u32, day as u32)
}

fn normalize_save_target(raw: &str) -> Result<PathBuf, String> {
    let mut path = PathBuf::from(raw.trim());
    if path.file_name().is_none() {
        return Err("Enter a file path ending in .cutcraft.".to_owned());
    }
    match path.extension().and_then(|extension| extension.to_str()) {
        None => {
            path.set_extension("cutcraft");
        }
        Some(extension) if extension.eq_ignore_ascii_case("cutcraft") => {}
        Some(_) => return Err("Use a .cutcraft file name.".to_owned()),
    }
    absolutize(path)
}

fn normalize_open_target(raw: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw.trim());
    if path.as_os_str().is_empty() {
        return Err("Enter a .cutcraft project path.".to_owned());
    }
    absolutize(path)
}

fn absolutize(path: PathBuf) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path);
    }
    std::env::current_dir()
        .map(|current| current.join(path))
        .map_err(|error| format!("Could not resolve the project path: {error}"))
}

fn carry_layout(target: &mut EditorSession, source: Option<&EditorSession>) {
    let Some(source) = source else {
        return;
    };
    target.tools_visible = source.tools_visible;
    target.inspector_visible = source.inspector_visible;
    target.timeline_visible = source.timeline_visible;
    target.left_panel_fraction = source.left_panel_fraction;
    target.right_panel_fraction = source.right_panel_fraction;
    target.timeline_pixels_per_second = source.timeline_pixels_per_second;
    target.timeline_snapping = source.timeline_snapping;
    target.timeline_linked_selection = source.timeline_linked_selection;
    target.timeline_show_thumbnails = source.timeline_show_thumbnails;
    target.timeline_thumbnail_mode = source.timeline_thumbnail_mode;
    target.timeline_show_waveforms = source.timeline_show_waveforms;
    target.active_tool = source.active_tool;
    target.inspector_tab = source.inspector_tab;
    target.caption_tab = source.caption_tab;
}

fn sanitize(message: String) -> String {
    let mut sanitized: String = message
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(Notice::MAX_MESSAGE_CHARS)
        .collect();

    if sanitized.trim().is_empty() {
        sanitized = "Unknown UI error.".to_owned();
    }

    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_state::{EditorTool, PreviewAspect};
    use crate::export_settings::{BitratePreset, ExportResolution};
    use crate::timeline_state::{
        TimelineContentState, TimelineDisplayMode, TimelineItemId, TimelineItemView,
        TimelineTrackView, TrackKind,
    };
    use crate::workspace_preferences::{WorkspacePreferences, WorkspaceStore};

    fn editor_state() -> AppState {
        let mut state = AppState::default();
        state.create_temporary_project();
        state
    }

    fn supplied_track(label: &str, items: Vec<TimelineItemView>) -> TimelineTrackView {
        TimelineTrackView {
            label: label.to_owned(),
            name: "Supplied track".to_owned(),
            kind: TrackKind::Video,
            items,
        }
    }

    #[test]
    fn new_project_starts_with_a_pristine_timeline() {
        let state = editor_state();
        let session = state.session().expect("new session should exist");

        assert_eq!(
            session.timeline_view.content_state(),
            TimelineContentState::PristineEmpty
        );
        assert!(session.timeline_view.tracks.is_empty());
        assert_eq!(session.timeline_display_mode, TimelineDisplayMode::Project);
        assert_eq!(session.timeline_scroll, [0.0; 2]);
    }

    #[test]
    fn supplied_track_and_navigation_preserve_empty_track_state() {
        let mut state = editor_state();
        {
            let session = state.session_mut().expect("editor session should exist");
            session
                .timeline_view
                .tracks
                .push(supplied_track("V1", Vec::new()));
            session.timeline_scroll = [120.0, 0.0];
        }

        assert_eq!(
            state
                .session()
                .expect("session should exist")
                .timeline_view
                .content_state(),
            TimelineContentState::EmptyTracks
        );

        state.go_home();
        assert!(state.resume_editor());

        let session = state.session().expect("session should survive navigation");
        assert_eq!(
            session.timeline_view.content_state(),
            TimelineContentState::EmptyTracks
        );
        assert_eq!(session.timeline_view.tracks.len(), 1);
        assert!(session.timeline_view.tracks[0].items.is_empty());
        assert_eq!(session.timeline_scroll, [120.0, 0.0]);
    }

    #[test]
    fn populated_timeline_survives_home_and_resume() {
        let mut state = editor_state();
        {
            let session = state.session_mut().expect("editor session should exist");
            session.timeline_view.tracks.push(supplied_track(
                "V1",
                vec![
                    TimelineItemView::new("Supplied item", 1.0, 7.0).expect("valid display range"),
                ],
            ));
        }

        state.go_home();
        assert!(state.resume_editor());

        let session = state.session().expect("session should survive navigation");
        assert_eq!(
            session.timeline_view.content_state(),
            TimelineContentState::Populated
        );
        assert_eq!(session.timeline_view.tracks[0].items.len(), 1);
        assert_eq!(
            session.timeline_view.tracks[0].items[0].label(),
            "Supplied item"
        );
    }

    #[test]
    fn visual_demo_is_explicit_and_never_replaces_supplied_timeline_data() {
        let mut state = editor_state();
        let before = {
            let session = state.session_mut().expect("editor session should exist");
            session
                .timeline_view
                .tracks
                .push(supplied_track("V1", Vec::new()));
            session.timeline_view.clone()
        };

        {
            let session = state.session_mut().expect("editor session should exist");
            session.set_timeline_demo(true);
            assert_eq!(
                session.timeline_display_mode,
                TimelineDisplayMode::VisualDemo
            );
            assert_eq!(session.timeline_view, before);
            assert!(!session.displayed_timeline().tracks.is_empty());

            session.set_timeline_demo(false);
            assert_eq!(session.timeline_display_mode, TimelineDisplayMode::Project);
            assert_eq!(session.displayed_timeline(), &before);
            assert_eq!(session.timeline_view, before);
        }

        state.create_temporary_project();
        let session = state.session().expect("new session should exist");
        assert_eq!(session.timeline_display_mode, TimelineDisplayMode::Project);
        assert_eq!(
            session.timeline_view.content_state(),
            TimelineContentState::PristineEmpty
        );
    }

    #[test]
    fn application_starts_on_home_without_a_session() {
        let state = AppState::default();

        assert_eq!(state.screen(), Screen::Home);
        assert!(!state.has_session());
        assert!(!state.about_open());
        assert_eq!(state.notice().severity(), NoticeSeverity::Info);
        assert!(state.notice().message().contains("Create a project"));
    }

    #[test]
    fn creating_a_project_enters_a_native_editor_session() {
        let state = editor_state();

        assert_eq!(state.screen(), Screen::Editor);
        assert!(state.session().is_some());
        assert!(!state.session().is_some_and(|session| session.export_open));
        assert!(state.notice().message().contains("native project"));
    }

    #[test]
    fn navigation_between_workspace_pages_preserves_the_session() {
        let mut state = editor_state();
        {
            let session = state.session_mut().expect("editor session should exist");
            session.active_tool = EditorTool::Effects;
            session.preview_aspect = PreviewAspect::Portrait;
            session.export_settings.resolution = ExportResolution::P2160;
            session.export_settings.bitrate = BitratePreset::Higher;
            session.timeline_snapping = false;
            session.timeline_linked_selection = false;
            session.timeline_show_thumbnails = false;
            session.timeline_show_waveforms = false;
            session.timeline_thumbnail_mode = crate::timeline_state::ThumbnailMode::FirstAndLast;
        }

        state.go_home();
        assert_eq!(state.screen(), Screen::Home);
        state.go_projects();
        assert_eq!(state.screen(), Screen::Projects);
        state.go_settings();
        assert_eq!(state.screen(), Screen::Settings);

        assert!(state.resume_editor());
        assert_eq!(state.screen(), Screen::Editor);
        let session = state.session().expect("session should survive navigation");
        assert_eq!(session.active_tool, EditorTool::Effects);
        assert_eq!(session.preview_aspect, PreviewAspect::Portrait);
        assert_eq!(session.export_settings.resolution, ExportResolution::P2160);
        assert_eq!(session.export_settings.bitrate, BitratePreset::Higher);
        assert!(!session.timeline_snapping);
        assert!(!session.timeline_linked_selection);
        assert!(!session.timeline_show_thumbnails);
        assert!(!session.timeline_show_waveforms);
        assert_eq!(
            session.timeline_thumbnail_mode,
            crate::timeline_state::ThumbnailMode::FirstAndLast
        );
    }

    #[test]
    fn resume_requires_an_existing_session() {
        let mut state = AppState::default();

        assert!(!state.resume_editor());
        assert_eq!(state.screen(), Screen::Home);
    }

    #[test]
    fn export_opens_only_from_the_editor_and_closes_explicitly() {
        let mut state = AppState::default();
        assert!(!state.open_export());

        state.create_temporary_project();
        assert!(state.open_export());
        assert!(state.session().is_some_and(|session| session.export_open));

        state.close_export();
        assert!(!state.session().is_some_and(|session| session.export_open));
    }

    #[test]
    fn export_settings_survive_dialog_close_and_reopen() {
        let mut state = editor_state();
        {
            let session = state
                .session_mut()
                .expect("temporary editor session should exist");
            session.export_settings.resolution = ExportResolution::P2160;
            session.export_settings.filename = "my-export.mp4".to_owned();
        }

        assert!(state.open_export());
        state.close_export();
        assert!(state.open_export());

        let session = state.session().expect("session should still exist");
        assert_eq!(session.export_settings.resolution, ExportResolution::P2160);
        assert_eq!(session.export_settings.filename, "my-export.mp4");
    }

    #[test]
    fn leaving_the_editor_closes_the_export_dialog() {
        let mut state = editor_state();
        assert!(state.open_export());

        state.go_home();

        assert_eq!(state.screen(), Screen::Home);
        assert!(!state.session().is_some_and(|session| session.export_open));
    }

    #[test]
    fn creating_another_project_replaces_the_previous_session() {
        let mut state = editor_state();
        {
            let session = state
                .session_mut()
                .expect("temporary editor session should exist");
            session.active_tool = EditorTool::Text;
            session.export_open = true;
            session
                .timeline_view
                .tracks
                .push(supplied_track("V1", Vec::new()));
            session.set_timeline_demo(true);
        }

        state.create_temporary_project();

        let session = state.session().expect("new session should exist");
        assert_eq!(session.active_tool, EditorTool::Captions);
        assert!(!session.export_open);
        assert_eq!(state.screen(), Screen::Editor);
        assert_eq!(session.timeline_display_mode, TimelineDisplayMode::Project);
        assert_eq!(
            session.timeline_view.content_state(),
            TimelineContentState::PristineEmpty
        );
        assert!(session.timeline_view.tracks.is_empty());
        assert!(session.timeline_snapping);
        assert!(session.timeline_linked_selection);
    }

    #[test]
    fn creating_another_project_recomputes_an_empty_timeline_selection() {
        let mut state = editor_state();
        {
            let session = state
                .session_mut()
                .expect("temporary editor session should exist");
            session.timeline_view.tracks.push(supplied_track(
                "V1",
                vec![
                    TimelineItemView::new("Supplied item", 1.0, 7.0).expect("valid display range"),
                ],
            ));
            assert!(session.select_timeline_item(TimelineItemId { track: 0, item: 0 }, false));
            assert!(session.timeline_selection_context().kind().is_some());
        }

        state.create_temporary_project();

        let session = state.session().expect("new session should exist");
        assert!(session.timeline_selection().is_empty());
        assert_eq!(session.timeline_selection_context().kind(), None);
    }

    #[test]
    fn reset_layout_restores_editor_panels() {
        let mut state = editor_state();
        {
            let session = state.session_mut().expect("editor session should exist");
            session.tools_visible = false;
            session.inspector_visible = false;
            session.timeline_visible = false;
        }

        state.reset_layout();

        let session = state.session().expect("editor session should exist");
        assert!(session.tools_visible);
        assert!(session.inspector_visible);
        assert!(session.timeline_visible);
    }

    fn workspace_store(name: &str) -> (WorkspaceStore, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "cutcraft-app-workspace-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("workspace temp");
        (WorkspaceStore::new(dir.join("workspace.json")), dir)
    }

    #[test]
    fn timeline_height_is_shared_across_projects_and_persisted() {
        let (store, dir) = workspace_store("shared-height");
        let mut state = AppState::default();
        state.install_workspace_preferences(
            WorkspacePreferences {
                timeline_height_fraction: Some(0.61),
            },
            Some(store.clone()),
        );

        state.create_temporary_project();
        assert_eq!(
            state.session().expect("session").timeline_height_fraction,
            Some(0.61)
        );

        state
            .session_mut()
            .expect("session")
            .set_timeline_height_fraction(Some(0.73));
        state.session_mut().expect("session").request_layout_save();
        assert!(state.persist_workspace_layout_if_requested());
        assert!(!state.persist_workspace_layout_if_requested());
        assert_eq!(
            store.load().expect("load").expect("preferences"),
            WorkspacePreferences {
                timeline_height_fraction: Some(0.73)
            }
        );

        state.create_temporary_project();
        assert_eq!(
            state
                .session()
                .expect("new session")
                .timeline_height_fraction,
            Some(0.73),
            "a new project must keep the shared timeline height"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn reset_layout_persists_automatic_timeline_height() {
        let (store, dir) = workspace_store("reset-height");
        let mut state = AppState::default();
        state.install_workspace_preferences(
            WorkspacePreferences {
                timeline_height_fraction: Some(0.64),
            },
            Some(store.clone()),
        );
        state.create_temporary_project();
        state.reset_layout();

        assert!(state.persist_workspace_layout_if_requested());
        assert_eq!(
            store.load().expect("load").expect("preferences"),
            WorkspacePreferences {
                timeline_height_fraction: None
            }
        );

        state.create_temporary_project();
        assert_eq!(
            state
                .session()
                .expect("new session")
                .timeline_height_fraction,
            None
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn workspace_save_failure_is_reported_without_losing_the_session() {
        let dir = std::env::temp_dir().join(format!(
            "cutcraft-app-workspace-failure-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("workspace temp");
        let parent_file = dir.join("not-a-directory");
        std::fs::write(&parent_file, b"file").expect("parent file");
        let store = WorkspaceStore::new(parent_file.join("workspace.json"));

        let mut state = AppState::default();
        state.install_workspace_preferences(WorkspacePreferences::default(), Some(store));
        state.create_temporary_project();
        state
            .session_mut()
            .expect("session")
            .set_timeline_height_fraction(Some(0.70));
        state.session_mut().expect("session").request_layout_save();

        assert!(state.persist_workspace_layout_if_requested());
        assert_eq!(state.notice().severity(), NoticeSeverity::Warning);
        assert!(state.notice().message().contains("workspace layout"));
        assert_eq!(
            state.session().expect("session").timeline_height_fraction,
            Some(0.70)
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn about_dialog_state_can_open_and_close() {
        let mut state = AppState::default();

        state.open_about();
        assert!(state.about_open());
        state.close_about();
        assert!(!state.about_open());
    }

    #[test]
    fn report_error_sets_error_severity_and_clear_restores_info() {
        let mut state = AppState::default();
        state.report_error("Preview device was lost.");

        assert_eq!(state.notice().severity(), NoticeSeverity::Error);
        assert_eq!(state.notice().message(), "Preview device was lost.");

        state.clear_notice();
        assert_eq!(state.notice().severity(), NoticeSeverity::Info);
    }

    #[test]
    fn notice_sanitizes_control_characters_and_caps_length() {
        let notice = Notice::warning(format!("bad\nmessage{}", "x".repeat(400)));

        assert!(!notice.message().contains('\n'));
        assert_eq!(notice.message().chars().count(), Notice::MAX_MESSAGE_CHARS);
    }

    #[test]
    fn empty_error_message_is_replaced_with_an_explicit_fallback() {
        let notice = Notice::error("   ");

        assert_eq!(notice.severity(), NoticeSeverity::Error);
        assert_eq!(notice.message(), "Unknown UI error.");
    }

    fn key_event(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn run_shortcuts(state: &mut AppState, events: Vec<egui::Event>) {
        let context = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 900.0),
            )),
            events,
            ..Default::default()
        };
        let output = context.run_ui(input, |_ui| state.handle_shortcuts(&context));
        output.drop_without_applying_deltas();
    }

    #[test]
    fn keyboard_frame_navigation_and_split_dispatch_through_the_app() {
        let mut state = app_with_project(project_abc_for_app());
        run_shortcuts(
            &mut state,
            vec![key_event(egui::Key::ArrowRight, egui::Modifiers::NONE)],
        );
        assert_eq!(
            state.session().expect("session").playhead_ticks().get(),
            4_000
        );
        run_shortcuts(
            &mut state,
            vec![key_event(egui::Key::End, egui::Modifiers::NONE)],
        );
        assert_eq!(
            state.session().expect("session").playhead_ticks().get(),
            25 * 120_000
        );
        run_shortcuts(
            &mut state,
            vec![key_event(egui::Key::Home, egui::Modifiers::NONE)],
        );
        assert_eq!(
            state.session().expect("session").playhead_ticks(),
            capcut_core::time::TimelineTicks::ZERO
        );

        assert!(state.session_mut().expect("session").select_timeline_item(
            crate::timeline_state::TimelineItemId { track: 0, item: 1 },
            false
        ));
        assert!(state
            .session_mut()
            .expect("session")
            .set_playhead_ticks(capcut_core::time::TimelineTicks::new(12 * 120_000)));
        run_shortcuts(
            &mut state,
            vec![key_event(egui::Key::S, egui::Modifiers::NONE)],
        );
        assert_eq!(
            state
                .session()
                .expect("session")
                .native_timeline
                .as_ref()
                .expect("native")
                .project()
                .main_track()
                .expect("main")
                .clips()
                .len(),
            4
        );
    }

    #[test]
    fn keyboard_navigation_and_shortcuts_dialog_are_wired_to_real_actions() {
        let state = editor_state();
        assert_eq!(state.screen(), Screen::Editor);
        assert!(state.has_session());

        let mut state = AppState::default();
        run_shortcuts(
            &mut state,
            vec![key_event(
                egui::Key::Slash,
                egui::Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
            )],
        );
        assert!(state.shortcuts_open());
    }

    #[test]
    fn timeline_shortcuts_update_session_preferences_and_zoom() {
        let mut state = editor_state();
        let before = state.session().expect("session").timeline_pixels_per_second;
        run_shortcuts(
            &mut state,
            vec![
                key_event(
                    egui::Key::S,
                    egui::Modifiers {
                        shift: true,
                        ..Default::default()
                    },
                ),
                key_event(
                    egui::Key::Equals,
                    egui::Modifiers {
                        ctrl: true,
                        ..Default::default()
                    },
                ),
            ],
        );
        let session = state.session().expect("session");
        assert!(!session.timeline_snapping);
        assert!(session.timeline_pixels_per_second > before);

        run_shortcuts(
            &mut state,
            vec![key_event(
                egui::Key::L,
                egui::Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
            )],
        );
        assert!(!state.session().expect("session").timeline_linked_selection);
    }

    #[test]
    fn deselect_shortcut_requires_and_clears_a_real_selection() {
        let mut state = editor_state();
        {
            let session = state.session_mut().expect("session");
            session.timeline_view.tracks.push(supplied_track(
                "V1",
                vec![TimelineItemView::new("Clip", 1.0, 4.0).expect("valid")],
            ));
            assert!(session.select_timeline_item(TimelineItemId { track: 0, item: 0 }, false));
        }
        run_shortcuts(
            &mut state,
            vec![key_event(
                egui::Key::A,
                egui::Modifiers {
                    ctrl: true,
                    shift: true,
                    ..Default::default()
                },
            )],
        );
        assert!(state
            .session()
            .expect("session")
            .timeline_selection()
            .is_empty());
    }

    #[test]
    fn open_modals_block_underlying_shortcuts() {
        let mut state = editor_state();
        let zoom = state.session().expect("session").timeline_pixels_per_second;
        assert!(state.open_export());
        run_shortcuts(
            &mut state,
            vec![key_event(
                egui::Key::Equals,
                egui::Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
            )],
        );
        assert_eq!(
            state.session().expect("session").timeline_pixels_per_second,
            zoom
        );

        state.close_export();
        state.open_shortcuts();
        run_shortcuts(
            &mut state,
            vec![key_event(
                egui::Key::Equals,
                egui::Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
            )],
        );
        assert_eq!(
            state.session().expect("session").timeline_pixels_per_second,
            zoom
        );

        state.close_shortcuts();
    }

    #[test]
    fn planned_shortcuts_never_execute_or_modify_project_state() {
        let mut state = editor_state();
        let before = state.session().expect("session").clone();
        run_shortcuts(
            &mut state,
            vec![
                key_event(egui::Key::S, egui::Modifiers::NONE),
                key_event(egui::Key::Space, egui::Modifiers::NONE),
                key_event(
                    egui::Key::Z,
                    egui::Modifiers {
                        ctrl: true,
                        ..Default::default()
                    },
                ),
            ],
        );
        assert_eq!(state.session().expect("session"), &before);
    }

    #[test]
    fn focused_text_editing_protects_normal_typing_from_timeline_shortcuts() {
        let context = egui::Context::default();
        let mut text = String::new();
        let mut state = editor_state();
        let zoom = state.session().expect("session").timeline_pixels_per_second;

        let focus_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 900.0),
            )),
            ..Default::default()
        };
        let output = context.run_ui(focus_input, |ui| {
            let response = ui.add(egui::TextEdit::singleline(&mut text));
            response.request_focus();
        });
        output.drop_without_applying_deltas();
        assert!(context.text_edit_focused());

        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 900.0),
            )),
            events: vec![key_event(
                egui::Key::Equals,
                egui::Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
            )],
            ..Default::default()
        };
        let output = context.run_ui(input, |_ui| state.handle_shortcuts(&context));
        output.drop_without_applying_deltas();
        assert_eq!(
            state.session().expect("session").timeline_pixels_per_second,
            zoom,
            "shortcuts must not execute while a text field owns the keyboard"
        );
    }

    #[test]
    fn homepage_new_project_shortcut_does_not_replace_an_editor_session() {
        let mut state = editor_state();
        state.session_mut().expect("session").project_name = "Keep me".to_owned();
        run_shortcuts(
            &mut state,
            vec![key_event(
                egui::Key::N,
                egui::Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
            )],
        );
        assert_eq!(
            state.session().expect("session").project_name,
            "Keep me",
            "the Editor context must not run a Homepage-only command"
        );
    }

    #[test]
    fn warning_notices_are_visible_and_explicit() {
        let mut state = AppState::default();
        state.report_warning("Shortcut configuration could not be loaded.");
        assert_eq!(state.notice().severity(), NoticeSeverity::Warning);
        assert!(state.notice().message().contains("could not be loaded"));
    }

    use crate::native_document::{NativeDocumentState, SaveStatus};
    use crate::native_project_state::NativeDialogMode;
    use crate::native_timeline::NativeTimeline;
    use capcut_core::time::{
        FrameRate, SourceRange as CoreSourceRange, SourceTicks as CoreSourceTicks, Timebase,
        TimelineRange, TimelineTicks,
    };
    use capcut_engine::command::NativeEditor;
    use capcut_engine::project::{
        ClipKind, ClipSpec, Project as EngineProject, SourceMapping, TrackKind as EngineTrackKind,
    };
    use capcut_project::ProjectId;
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};

    static APP_TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct AppTempDir(std::path::PathBuf);

    impl AppTempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "cutcraft-app-m3a-{}-{}",
                std::process::id(),
                APP_TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).expect("temp dir");
            Self(path)
        }

        fn path(&self, name: &str) -> std::path::PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for AppTempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn app_rate() -> FrameRate {
        FrameRate::new(30, 1).expect("fps")
    }

    fn app_range(start: i64, end: i64) -> TimelineRange {
        TimelineRange::new(TimelineTicks::new(start), TimelineTicks::new(end)).expect("range")
    }

    fn app_source(duration: i64) -> SourceMapping {
        SourceMapping::new(
            CoreSourceRange::new(CoreSourceTicks::ZERO, CoreSourceTicks::new(duration))
                .expect("available"),
            CoreSourceRange::new(CoreSourceTicks::ZERO, CoreSourceTicks::new(duration))
                .expect("used"),
            Timebase::new(120_000).expect("timebase"),
            1,
            1,
        )
        .expect("source")
    }

    fn app_spec(name: &str, start: i64, end: i64) -> ClipSpec {
        ClipSpec {
            name: name.to_owned(),
            kind: ClipKind::Video,
            range: app_range(start, end),
            source: Some(app_source(end - start)),
            media_asset_id: None,
            linked_clip_ids: Default::default(),
            nested: false,
        }
    }

    fn project_abc_for_app() -> EngineProject {
        let mut project = EngineProject::named(app_rate(), "Project ABC").expect("project");
        let track = project
            .add_track("V1", EngineTrackKind::Video, false)
            .expect("track");
        project.assign_main_track(track).expect("main");
        project
            .add_clip(track, app_spec("A", 0, 600_000))
            .expect("A");
        project
            .add_clip(track, app_spec("B", 1_200_000, 1_800_000))
            .expect("B");
        project
            .add_clip(track, app_spec("C", 2_400_000, 3_000_000))
            .expect("C");
        project
    }

    fn project_contiguous_for_app() -> EngineProject {
        let mut project = EngineProject::named(app_rate(), "Contiguous").expect("project");
        let track = project
            .add_track("V1", EngineTrackKind::Video, false)
            .expect("track");
        project.assign_main_track(track).expect("main");
        project
            .add_clip(track, app_spec("A", 0, 600_000))
            .expect("A");
        project
            .add_clip(track, app_spec("B", 600_000, 1_200_000))
            .expect("B");
        project
    }

    fn app_with_project(project: EngineProject) -> AppState {
        let native = NativeTimeline::from_project(project).expect("native");
        let name = native.project().name().to_owned();
        let view = native.view();
        let session = EditorSession {
            project_name: name,
            timeline_view: view,
            native_timeline: Some(native),
            native_document: Some(NativeDocumentState::unsaved(ProjectId::generate(), false)),
            ..EditorSession::default()
        };
        AppState {
            session: Some(session),
            screen: Screen::Editor,
            ..AppState::default()
        }
    }

    fn set_dialog_path(state: &mut AppState, path: &Path) {
        let dialog = state.native_project_dialog_mut().expect("dialog");
        *dialog.path_mut() = path.to_string_lossy().into_owned();
    }

    fn save_first_time(state: &mut AppState, path: &Path) {
        state.save_native_project();
        set_dialog_path(state, path);
        state.confirm_native_save_as().expect("save as");
    }

    fn document_id(state: &AppState) -> ProjectId {
        state
            .session()
            .and_then(|session| session.native_document())
            .expect("document")
            .project_id()
            .clone()
    }

    fn document_path(state: &AppState) -> Option<std::path::PathBuf> {
        state
            .session()
            .and_then(|session| session.native_document())
            .and_then(|document| document.path().cloned())
    }

    fn document_dirty(state: &AppState) -> bool {
        state
            .session()
            .and_then(|session| session.native_document())
            .is_some_and(|document| document.is_dirty())
    }

    fn project_clip_ids(state: &AppState) -> Vec<u64> {
        state
            .session()
            .and_then(|session| session.native_timeline.as_ref())
            .expect("native")
            .project()
            .tracks()
            .iter()
            .flat_map(|track| track.clips())
            .map(|clip| clip.id().get())
            .collect()
    }

    #[test]
    fn save_edit_save_as_commits_new_identity_only_after_success() {
        let dir = AppTempDir::new();
        let first = dir.path("first.cutcraft");
        let second = dir.path("second.cutcraft");
        let mut state = app_with_project(project_abc_for_app());
        let original_id = document_id(&state);
        assert_eq!(
            state.session().and_then(EditorSession::save_status),
            Some(SaveStatus::NotSaved)
        );

        save_first_time(&mut state, &first);
        assert_eq!(
            document_id(&state),
            original_id,
            "first save keeps identity"
        );
        assert_eq!(document_path(&state).as_deref(), Some(first.as_path()));
        assert_eq!(
            state.session().and_then(EditorSession::save_status),
            Some(SaveStatus::Saved)
        );

        let clip_ids = project_clip_ids(&state);
        state
            .session_mut()
            .expect("session")
            .toggle_main_track_magnet();
        assert!(document_dirty(&state));
        state.open_save_as_dialog();
        set_dialog_path(&mut state, &second);
        state
            .confirm_native_save_as()
            .expect("save as new identity");

        let new_id = document_id(&state);
        assert_ne!(new_id, original_id);
        assert_eq!(document_path(&state).as_deref(), Some(second.as_path()));
        assert!(!document_dirty(&state));
        assert_eq!(
            project_clip_ids(&state),
            clip_ids,
            "track/clip IDs preserved"
        );
        let first_file = capcut_project::open(&first).expect("first file");
        assert_eq!(first_file.project_id, original_id);
        assert!(!first_file.editor.magnet_enabled());
        let second_file = capcut_project::open(&second).expect("second file");
        assert_eq!(second_file.project_id, new_id);
        assert!(second_file.editor.magnet_enabled());
    }

    #[test]
    fn failed_save_as_leaves_identity_path_dirty_and_files_unchanged() {
        let dir = AppTempDir::new();
        let first = dir.path("first.cutcraft");
        let other = dir.path("other.cutcraft");
        let other_editor = NativeEditor::new(project_abc_for_app()).expect("other editor");
        capcut_project::save(&other_editor, &ProjectId::generate(), &other, None)
            .expect("other file");
        let other_bytes = std::fs::read(&other).expect("other bytes");

        let mut state = app_with_project(project_abc_for_app());
        save_first_time(&mut state, &first);
        let first_bytes = std::fs::read(&first).expect("first bytes");
        let original_id = document_id(&state);
        {
            let session = state.session_mut().expect("session");
            session.select_timeline_item(
                crate::timeline_state::TimelineItemId { track: 0, item: 1 },
                false,
            );
            session.delete_selected_native().expect("delete");
        }
        assert!(document_dirty(&state));

        state.open_save_as_dialog();
        set_dialog_path(&mut state, &other);
        assert!(state.confirm_native_save_as().is_err());
        assert_eq!(document_id(&state), original_id);
        assert_eq!(document_path(&state).as_deref(), Some(first.as_path()));
        assert!(document_dirty(&state));
        assert!(
            state.native_project_dialog().is_some(),
            "dialog keeps the error"
        );
        assert_eq!(
            state
                .native_project_dialog()
                .and_then(|dialog| dialog.pending_after_save()),
            None,
            "a failed save cancels its pending navigation"
        );
        assert_eq!(std::fs::read(&first).expect("first bytes"), first_bytes);
        assert_eq!(std::fs::read(&other).expect("other bytes"), other_bytes);
    }

    #[test]
    fn failed_open_preserves_session_and_success_replaces_it() {
        let dir = AppTempDir::new();
        let first = dir.path("first.cutcraft");
        let second = dir.path("second.cutcraft");
        let invalid = dir.path("invalid.cutcraft");
        std::fs::write(&invalid, b"not a project").expect("invalid");
        let mut state = app_with_project(project_abc_for_app());
        save_first_time(&mut state, &first);
        let current_id = document_id(&state);
        let current_clips = project_clip_ids(&state);

        state.request_open_native_project();
        assert_eq!(
            state.native_project_dialog().map(|dialog| dialog.mode()),
            Some(NativeDialogMode::Open)
        );
        set_dialog_path(&mut state, &invalid);
        assert!(state.confirm_native_open().is_err());
        assert_eq!(state.screen(), Screen::Editor);
        assert_eq!(document_id(&state), current_id);
        assert_eq!(project_clip_ids(&state), current_clips);
        assert!(state.native_project_dialog().is_some());

        state.close_native_project_dialog();
        let second_project = EngineProject::named(app_rate(), "Second Project").expect("named");
        let second_editor = NativeEditor::new(second_project).expect("editor");
        let second_id = ProjectId::generate();
        capcut_project::save(&second_editor, &second_id, &second, None).expect("second save");

        state.request_open_native_project();
        set_dialog_path(&mut state, &second);
        state.confirm_native_open().expect("open second");
        assert_eq!(state.screen(), Screen::Editor);
        assert_eq!(document_id(&state), second_id);
        assert_eq!(document_path(&state).as_deref(), Some(second.as_path()));
        assert!(!document_dirty(&state));
        assert_eq!(
            state.session().expect("session").project_name.as_str(),
            "Second Project"
        );
        assert!(!state.session().expect("session").can_undo_native());
    }

    #[test]
    fn direct_save_conflict_is_visible_and_preserves_dirty_state_and_file() {
        let dir = AppTempDir::new();
        let path = dir.path("conflict.cutcraft");
        let mut state = app_with_project(project_abc_for_app());
        save_first_time(&mut state, &path);
        let id = document_id(&state);
        state
            .session_mut()
            .expect("session")
            .toggle_main_track_magnet();
        std::fs::write(&path, b"external bytes").expect("external");
        state.save_native_project();
        assert!(state.native_error_dialog().is_some());
        assert!(state
            .native_error_dialog()
            .is_some_and(|message| message.contains("outside CutCraft")));
        assert_eq!(document_id(&state), id);
        assert_eq!(document_path(&state).as_deref(), Some(path.as_path()));
        assert!(document_dirty(&state));
        assert_eq!(std::fs::read(&path).expect("file"), b"external bytes");
        state.close_native_error_dialog();
        assert!(state.native_error_dialog().is_none());
    }

    #[test]
    fn native_save_as_dialog_renders_native_identity_and_actions() {
        let mut state = app_with_project(project_abc_for_app());
        state.save_native_project();
        let text = render_native_dialog_text(&mut state);
        assert!(text.iter().any(|line| line == "Save Project As"));
        assert!(text.iter().any(|line| line.contains("NATIVE .CUTCRAFT")));
        assert!(text.iter().any(|line| line == "Cancel"));
        assert!(text.iter().any(|line| line == "Save"));
        assert!(text.iter().any(|line| line.contains(".cutcraft")));
    }

    #[test]
    fn save_error_dialog_reports_conflict_and_preservation() {
        let mut state = app_with_project(project_abc_for_app());
        state.report_native_save_error(
            "C:\\project.cutcraft changed outside CutCraft; save was not applied".to_owned(),
        );
        let text = render_native_dialog_text(&mut state);
        assert!(text.iter().any(|line| line == "Project Save Failed"));
        assert!(text.iter().any(|line| line.contains("outside CutCraft")));
        assert!(text
            .iter()
            .any(|line| line.contains("active project and the existing file were left unchanged")));
    }

    fn render_native_dialog_text(state: &mut AppState) -> Vec<String> {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(1600.0, 900.0));
        let mut text = Vec::new();
        for pass in 0..2 {
            let output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    let context = ui.ctx().clone();
                    crate::widgets::native_project_dialogs::show(&context, state);
                },
            );
            if pass == 1 {
                text = output
                    .shapes
                    .iter()
                    .filter_map(|clipped| match &clipped.shape {
                        egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                        _ => None,
                    })
                    .collect();
            }
            output.drop_without_applying_deltas();
        }
        text
    }

    #[test]
    fn preference_only_magnet_toggle_dirties_and_round_trips() {
        let dir = AppTempDir::new();
        let path = dir.path("preference.cutcraft");
        let mut state = app_with_project(project_contiguous_for_app());
        save_first_time(&mut state, &path);
        assert_eq!(
            state.session().and_then(EditorSession::save_status),
            Some(SaveStatus::Saved)
        );
        state
            .session_mut()
            .expect("session")
            .toggle_main_track_magnet();
        assert!(state
            .session()
            .expect("session")
            .main_track_magnet_enabled());
        assert_eq!(
            state.session().and_then(EditorSession::save_status),
            Some(SaveStatus::UnsavedChanges),
            "preference-only Magnet changes are persisted and must dirty the document"
        );
        state.save_native_project();
        let opened = capcut_project::open(&path).expect("open");
        assert!(opened.editor.magnet_enabled());
        let starts = opened
            .editor
            .project()
            .main_track()
            .expect("main")
            .clips()
            .iter()
            .map(|clip| clip.range().start().get())
            .collect::<Vec<_>>();
        assert_eq!(starts, vec![0, 600_000]);
    }

    #[test]
    fn unsupported_schema_reports_error_without_replacing_the_session() {
        let dir = AppTempDir::new();
        let future = dir.path("future.cutcraft");
        std::fs::write(
            &future,
            br#"{"format":"cutcraft-project","schema_version":3}"#,
        )
        .expect("write");
        let mut state = app_with_project(project_abc_for_app());
        let id = document_id(&state);
        let clips = project_clip_ids(&state);
        state.request_open_native_project();
        set_dialog_path(&mut state, &future);
        assert!(state.confirm_native_open().is_err());
        assert_eq!(state.screen(), Screen::Editor);
        assert_eq!(document_id(&state), id);
        assert_eq!(project_clip_ids(&state), clips);
        assert!(state
            .native_project_dialog()
            .and_then(|dialog| dialog.error())
            .is_some_and(|error| error.contains("unsupported project schema version 3")));
    }

    #[test]
    fn split_save_reopen_preserves_exact_ranges_ids_and_allocation() {
        let dir = AppTempDir::new();
        let path = dir.path("split.cutcraft");
        let mut state = app_with_project(project_abc_for_app());
        assert!(state.session_mut().expect("session").select_timeline_item(
            crate::timeline_state::TimelineItemId { track: 0, item: 0 },
            false
        ));
        assert!(state
            .session_mut()
            .expect("session")
            .set_playhead_ticks(capcut_core::time::TimelineTicks::new(2 * 120_000)));
        assert!(state.session().expect("session").can_split_at_playhead());
        state
            .session_mut()
            .expect("session")
            .split_at_playhead()
            .expect("split A");
        assert!(state.session().expect("session").native_is_dirty());
        {
            let session = state.session().expect("session");
            let main = session
                .native_timeline
                .as_ref()
                .expect("native")
                .project()
                .main_track()
                .expect("main");
            assert_eq!(main.clips().len(), 4);
            assert_eq!(main.clips()[0].id().get(), 1);
            assert_eq!(main.clips()[0].range().start().get(), 0);
            assert_eq!(main.clips()[0].range().end().get(), 240_000);
            assert_eq!(main.clips()[1].id().get(), 4);
            assert_eq!(main.clips()[1].range().start().get(), 240_000);
            assert_eq!(main.clips()[1].range().end().get(), 600_000);
            assert_eq!(
                session.playhead_ticks(),
                capcut_core::time::TimelineTicks::new(240_000)
            );
        }

        save_first_time(&mut state, &path);
        assert_eq!(
            state.session().and_then(EditorSession::save_status),
            Some(SaveStatus::Saved)
        );
        state.request_open_native_project();
        set_dialog_path(&mut state, &path);
        state.confirm_native_open().expect("reopen");
        {
            let session = state.session().expect("session");
            assert_eq!(
                session.playhead_ticks(),
                capcut_core::time::TimelineTicks::ZERO
            );
            assert!(!session.can_undo_native());
            assert_eq!(session.save_status(), Some(SaveStatus::Saved));
            let main = session
                .native_timeline
                .as_ref()
                .expect("native")
                .project()
                .main_track()
                .expect("main");
            assert_eq!(main.clips().len(), 4);
            assert_eq!(main.clips()[0].id().get(), 1);
            assert_eq!(main.clips()[0].range().end().get(), 240_000);
            assert_eq!(main.clips()[1].id().get(), 4);
            assert_eq!(main.clips()[1].range().start().get(), 240_000);
            assert_eq!(
                session
                    .native_timeline
                    .as_ref()
                    .expect("native")
                    .project()
                    .next_clip_id(),
                5
            );
        }

        assert!(state
            .session_mut()
            .expect("session")
            .set_playhead_ticks(capcut_core::time::TimelineTicks::new(3 * 120_000)));
        assert_eq!(
            state.session().and_then(EditorSession::save_status),
            Some(SaveStatus::Saved),
            "playhead navigation must not dirty the project"
        );
        assert!(state.session_mut().expect("session").select_timeline_item(
            crate::timeline_state::TimelineItemId { track: 0, item: 2 },
            false
        ));
        assert!(state
            .session_mut()
            .expect("session")
            .set_playhead_ticks(capcut_core::time::TimelineTicks::new(12 * 120_000)));
        assert!(state.session().expect("session").can_split_at_playhead());
        state
            .session_mut()
            .expect("session")
            .split_at_playhead()
            .expect("split after reopen");
        let session = state.session().expect("session");
        let main = session
            .native_timeline
            .as_ref()
            .expect("native")
            .project()
            .main_track()
            .expect("main");
        assert_eq!(main.clips().len(), 5);
        assert_eq!(main.clips()[2].id().get(), 2);
        assert_eq!(main.clips()[3].id().get(), 5);
        assert_eq!(
            session
                .native_timeline
                .as_ref()
                .expect("native")
                .project()
                .next_clip_id(),
            6
        );
        assert!(session.native_is_dirty());
    }

    #[test]
    fn split_with_magnet_on_survives_save_and_reopen() {
        let dir = AppTempDir::new();
        let path = dir.path("split-magnet.cutcraft");
        let mut state = app_with_project(project_abc_for_app());
        state
            .session_mut()
            .expect("session")
            .toggle_main_track_magnet();
        assert!(state
            .session()
            .expect("session")
            .main_track_magnet_enabled());
        assert!(state.session_mut().expect("session").select_timeline_item(
            crate::timeline_state::TimelineItemId { track: 0, item: 1 },
            false
        ));
        assert!(state
            .session_mut()
            .expect("session")
            .set_playhead_ticks(capcut_core::time::TimelineTicks::new(7 * 120_000)));
        state
            .session_mut()
            .expect("session")
            .split_at_playhead()
            .expect("split on");
        save_first_time(&mut state, &path);
        state.request_open_native_project();
        set_dialog_path(&mut state, &path);
        state.confirm_native_open().expect("reopen");
        let session = state.session().expect("session");
        assert!(session.main_track_magnet_enabled());
        assert_eq!(
            session.playhead_ticks(),
            capcut_core::time::TimelineTicks::ZERO
        );
        let main = session
            .native_timeline
            .as_ref()
            .expect("native")
            .project()
            .main_track()
            .expect("main");
        assert_eq!(main.clips().len(), 4);
        assert_eq!(
            main.clips()
                .iter()
                .map(|clip| (
                    clip.id().get(),
                    clip.range().start().get(),
                    clip.range().end().get()
                ))
                .collect::<Vec<_>>(),
            vec![
                (1, 0, 600_000),
                (2, 600_000, 840_000),
                (4, 840_000, 1_200_000),
                (3, 1_200_000, 1_800_000)
            ]
        );
        assert_eq!(
            session
                .native_timeline
                .as_ref()
                .expect("native")
                .project()
                .next_clip_id(),
            5
        );
    }

    #[test]
    fn dirty_state_tracks_magnet_undo_redo_delete_and_selection() {
        let dir = AppTempDir::new();
        let path = dir.path("dirty.cutcraft");
        let mut state = app_with_project(project_abc_for_app());
        save_first_time(&mut state, &path);
        assert_eq!(
            state.session().and_then(EditorSession::save_status),
            Some(SaveStatus::Saved)
        );

        state.session_mut().expect("session").select_timeline_item(
            crate::timeline_state::TimelineItemId { track: 0, item: 1 },
            false,
        );
        assert!(!document_dirty(&state), "selection is not persisted");

        state
            .session_mut()
            .expect("session")
            .toggle_main_track_magnet();
        assert_eq!(
            state.session().and_then(EditorSession::save_status),
            Some(SaveStatus::UnsavedChanges)
        );
        state
            .session_mut()
            .expect("session")
            .undo_native()
            .expect("undo");
        assert!(document_dirty(&state), "Undo marks the document dirty");
        state
            .session_mut()
            .expect("session")
            .redo_native()
            .expect("redo");
        assert!(document_dirty(&state), "Redo marks the document dirty");

        state
            .session_mut()
            .expect("session")
            .delete_selected_native()
            .expect("delete");
        assert!(document_dirty(&state));
        state.save_native_project();
        assert_eq!(
            state.session().and_then(EditorSession::save_status),
            Some(SaveStatus::Saved)
        );
    }
}

#[cfg(test)]
#[path = "app_state_autosave_tests.rs"]
mod managed_autosave_tests;

#[cfg(test)]
#[path = "app_state_media_tests.rs"]
mod managed_media_tests;

#[cfg(test)]
#[path = "app_state_recovery_tests.rs"]
mod recovery_tests;
