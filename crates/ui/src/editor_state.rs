//! Transient editor-session state.
//!
//! A session exists only in memory. It contains UI preferences and form values
//! needed to keep the editor coherent while navigating; it is not an engine
//! project and is never written to disk.

use crate::config;
use crate::export_settings::ExportSettings;

/// Left-panel library categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTool {
    Media,
    Audio,
    Text,
    Stickers,
    Effects,
    Transitions,
    Captions,
    Filters,
    Adjustment,
}

impl EditorTool {
    pub const ALL: [Self; 9] = [
        Self::Media,
        Self::Audio,
        Self::Text,
        Self::Stickers,
        Self::Effects,
        Self::Transitions,
        Self::Captions,
        Self::Filters,
        Self::Adjustment,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Media => "Media",
            Self::Audio => "Audio",
            Self::Text => "Text",
            Self::Stickers => "Stickers",
            Self::Effects => "Effects",
            Self::Transitions => "Transitions",
            Self::Captions => "Captions",
            Self::Filters => "Filters",
            Self::Adjustment => "Adjustment",
        }
    }
}

/// Inspector property tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Video,
    Audio,
    Text,
    Animation,
    Basic,
}

impl InspectorTab {
    pub const ALL: [Self; 5] = [
        Self::Video,
        Self::Audio,
        Self::Text,
        Self::Animation,
        Self::Basic,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Video => "Video",
            Self::Audio => "Audio",
            Self::Text => "Text",
            Self::Animation => "Animation",
            Self::Basic => "Basic",
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
    pub inspector_tab: InspectorTab,
    pub preview_aspect: PreviewAspect,
    pub export_open: bool,
    pub export_settings: ExportSettings,
    pub tools_visible: bool,
    pub inspector_visible: bool,
    pub timeline_visible: bool,
    pub timeline_pixels_per_second: f32,
    pub left_panel_fraction: f32,
    pub right_panel_fraction: f32,
}

impl Default for EditorSession {
    fn default() -> Self {
        Self {
            project_name: "Untitled Project".to_owned(),
            active_tool: EditorTool::Media,
            inspector_tab: InspectorTab::Video,
            preview_aspect: PreviewAspect::Landscape,
            export_open: false,
            export_settings: ExportSettings::default(),
            tools_visible: true,
            inspector_visible: true,
            timeline_visible: true,
            timeline_pixels_per_second: config::TIMELINE_PIXELS_PER_SECOND,
            left_panel_fraction: config::WORKSPACE_LEFT_FRACTION,
            right_panel_fraction: config::WORKSPACE_RIGHT_FRACTION,
        }
    }
}

impl EditorSession {
    pub fn reset_layout(&mut self) {
        self.tools_visible = true;
        self.inspector_visible = true;
        self.timeline_visible = true;
        self.left_panel_fraction = config::WORKSPACE_LEFT_FRACTION;
        self.right_panel_fraction = config::WORKSPACE_RIGHT_FRACTION;
    }

    pub fn zoom_timeline_in(&mut self) {
        self.timeline_pixels_per_second = (self.timeline_pixels_per_second * 1.25).clamp(
            config::TIMELINE_MIN_PIXELS_PER_SECOND,
            config::TIMELINE_MAX_PIXELS_PER_SECOND,
        );
    }

    pub fn zoom_timeline_out(&mut self) {
        self.timeline_pixels_per_second = (self.timeline_pixels_per_second / 1.25).clamp(
            config::TIMELINE_MIN_PIXELS_PER_SECOND,
            config::TIMELINE_MAX_PIXELS_PER_SECOND,
        );
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
        assert_eq!(session.inspector_tab, InspectorTab::Video);
        assert_eq!(session.preview_aspect, PreviewAspect::Landscape);
        assert!(!session.export_open);
        assert!(session.tools_visible);
        assert!(session.inspector_visible);
        assert!(session.timeline_visible);
    }

    #[test]
    fn reset_layout_restores_hidden_panels_and_split_fractions() {
        let mut session = EditorSession {
            tools_visible: false,
            inspector_visible: false,
            timeline_visible: false,
            left_panel_fraction: 0.45,
            right_panel_fraction: 0.5,
            ..EditorSession::default()
        };

        session.reset_layout();

        assert!(session.tools_visible);
        assert!(session.inspector_visible);
        assert!(session.timeline_visible);
        assert_eq!(session.left_panel_fraction, config::WORKSPACE_LEFT_FRACTION);
        assert_eq!(
            session.right_panel_fraction,
            config::WORKSPACE_RIGHT_FRACTION
        );
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
    fn all_editor_tools_and_inspector_tabs_expose_labels() {
        assert_eq!(EditorTool::ALL.len(), 9);
        assert_eq!(EditorTool::Stickers.label(), "Stickers");
        assert_eq!(EditorTool::Adjustment.label(), "Adjustment");
        assert_eq!(InspectorTab::ALL.len(), 5);
        assert_eq!(InspectorTab::Animation.label(), "Animation");
    }

    #[test]
    fn timeline_zoom_is_clamped_to_supported_range() {
        let mut at_max = EditorSession {
            timeline_pixels_per_second: config::TIMELINE_MAX_PIXELS_PER_SECOND,
            ..EditorSession::default()
        };
        at_max.zoom_timeline_in();
        assert_eq!(
            at_max.timeline_pixels_per_second,
            config::TIMELINE_MAX_PIXELS_PER_SECOND
        );

        let mut at_min = EditorSession {
            timeline_pixels_per_second: config::TIMELINE_MIN_PIXELS_PER_SECOND,
            ..EditorSession::default()
        };
        at_min.zoom_timeline_out();
        assert_eq!(
            at_min.timeline_pixels_per_second,
            config::TIMELINE_MIN_PIXELS_PER_SECOND
        );
    }
}
