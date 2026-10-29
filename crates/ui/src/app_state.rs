//! Application navigation and transient UI state.
//!
//! This module intentionally contains no engine project model, media handles,
//! or persistence. The editor session is a temporary in-memory UI session.

use crate::editor_state::EditorSession;

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

/// Navigation, temporary session, and status state for the application shell.
#[derive(Debug, Clone)]
pub struct AppState {
    screen: Screen,
    session: Option<EditorSession>,
    about_open: bool,
    notice: Notice,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            screen: Screen::Home,
            session: None,
            about_open: false,
            notice: Notice::info(
                "UI shell only \u{2014} editing, playback, and export are not implemented.",
            ),
        }
    }
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

    pub fn has_session(&self) -> bool {
        self.session.is_some()
    }

    /// Start a new temporary project and enter the editor.
    ///
    /// Any previous temporary session is replaced. No data is written to disk.
    pub fn create_temporary_project(&mut self) {
        self.session = Some(EditorSession::default());
        self.screen = Screen::Editor;
        self.notice =
            Notice::info("Temporary project created in memory. Nothing is saved to disk.");
    }

    /// Return to an existing temporary session.
    pub fn resume_editor(&mut self) -> bool {
        if self.session.is_some() {
            self.screen = Screen::Editor;
            true
        } else {
            false
        }
    }

    pub fn go_home(&mut self) {
        self.leave_editor();
        self.screen = Screen::Home;
    }

    pub fn go_projects(&mut self) {
        self.leave_editor();
        self.screen = Screen::Projects;
    }

    pub fn go_settings(&mut self) {
        self.leave_editor();
        self.screen = Screen::Settings;
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

    pub fn clear_notice(&mut self) {
        self.notice = Notice::info(
            "UI shell only \u{2014} editing, playback, and export are not implemented.",
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
        TimelineContentState, TimelineDisplayMode, TimelineItemView, TimelineTrackView, TrackKind,
    };

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
                vec![TimelineItemView::new("Supplied item", 1.0, 7.0)
                    .expect("valid display range")],
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
        assert!(state.notice().message().contains("not implemented"));
    }

    #[test]
    fn creating_a_project_enters_a_temporary_editor_session() {
        let state = editor_state();

        assert_eq!(state.screen(), Screen::Editor);
        assert!(state.session().is_some());
        assert!(!state.session().is_some_and(|session| session.export_open));
        assert!(state.notice().message().contains("Temporary"));
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
}
