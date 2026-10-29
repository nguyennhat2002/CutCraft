//! Transient editor-session state.
//!
//! A session exists only in memory. It contains UI preferences and form values
//! needed to keep the editor coherent while navigating; it is not an engine
//! project and is never written to disk.

use crate::export_settings::ExportSettings;

/// Left-panel library tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTool {
    Media,
    Audio,
    Text,
    Effects,
    Transitions,
}

impl EditorTool {
    pub const ALL: [Self; 5] = [
        Self::Media,
        Self::Audio,
        Self::Text,
        Self::Effects,
        Self::Transitions,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Media => "Media",
            Self::Audio => "Audio",
            Self::Text => "Text",
            Self::Effects => "Effects",
            Self::Transitions => "Transitions",
        }
    }
}

/// Preview canvas aspect ratio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewAspect {
    Landscape,
    Portrait,
}

impl PreviewAspect {
    pub const ALL: [Self; 2] = [Self::Landscape, Self::Portrait];

    pub fn label(self) -> &'static str {
        match self {
            Self::Landscape => "16:9",
            Self::Portrait => "9:16",
        }
    }

    pub fn ratio(self) -> f32 {
        match self {
            Self::Landscape => 16.0 / 9.0,
            Self::Portrait => 9.0 / 16.0,
        }
    }

    pub fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Landscape => (1920, 1080),
            Self::Portrait => (1080, 1920),
        }
    }
}

/// One temporary editor session, created from the homepage.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorSession {
    pub project_name: String,
    pub active_tool: EditorTool,
    pub preview_aspect: PreviewAspect,
    pub export_open: bool,
    pub export_settings: ExportSettings,
    pub tools_visible: bool,
    pub inspector_visible: bool,
    pub timeline_visible: bool,
}

impl Default for EditorSession {
    fn default() -> Self {
        Self {
            project_name: "Untitled Project".to_owned(),
            active_tool: EditorTool::Media,
            preview_aspect: PreviewAspect::Landscape,
            export_open: false,
            export_settings: ExportSettings::default(),
            tools_visible: true,
            inspector_visible: true,
            timeline_visible: true,
        }
    }
}

impl EditorSession {
    pub fn reset_layout(&mut self) {
        self.tools_visible = true;
        self.inspector_visible = true;
        self.timeline_visible = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_is_unsaved_and_empty() {
        let session = EditorSession::default();

        assert_eq!(session.project_name, "Untitled Project");
        assert_eq!(session.active_tool, EditorTool::Media);
        assert_eq!(session.preview_aspect, PreviewAspect::Landscape);
        assert!(!session.export_open);
        assert!(session.tools_visible);
        assert!(session.inspector_visible);
        assert!(session.timeline_visible);
    }

    #[test]
    fn reset_layout_restores_hidden_panels() {
        let mut session = EditorSession {
            tools_visible: false,
            inspector_visible: false,
            timeline_visible: false,
            ..EditorSession::default()
        };

        session.reset_layout();

        assert!(session.tools_visible);
        assert!(session.inspector_visible);
        assert!(session.timeline_visible);
    }

    #[test]
    fn preview_aspects_have_distinct_dimensions_and_ratios() {
        assert_eq!(PreviewAspect::Landscape.label(), "16:9");
        assert_eq!(PreviewAspect::Landscape.dimensions(), (1920, 1080));
        assert!((PreviewAspect::Landscape.ratio() - 16.0 / 9.0).abs() < f32::EPSILON);

        assert_eq!(PreviewAspect::Portrait.label(), "9:16");
        assert_eq!(PreviewAspect::Portrait.dimensions(), (1080, 1920));
        assert!((PreviewAspect::Portrait.ratio() - 9.0 / 16.0).abs() < f32::EPSILON);
    }

    #[test]
    fn all_editor_tools_expose_labels() {
        assert_eq!(EditorTool::ALL.len(), 5);
        assert_eq!(EditorTool::Audio.label(), "Audio");
        assert_eq!(EditorTool::Transitions.label(), "Transitions");
    }
}
