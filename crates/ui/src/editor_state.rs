//! Transient editor-session state.
//!
//! A session exists only in memory. It contains UI preferences and form values
//! needed to keep the editor coherent while navigating; it is not an engine
//! project and is never written to disk.

use std::collections::BTreeSet;

use crate::config;
use crate::export_settings::ExportSettings;
use crate::shortcuts::CommandId;
use crate::timeline_state::{
    self, TimelineDisplayMode, TimelineItemId, TimelineSelectionContext, TimelineView,
};

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
    Templates,
}

impl EditorTool {
    pub const ALL: [Self; 10] = [
        Self::Media,
        Self::Audio,
        Self::Text,
        Self::Stickers,
        Self::Effects,
        Self::Transitions,
        Self::Captions,
        Self::Filters,
        Self::Adjustment,
        Self::Templates,
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
            Self::Templates => "Templates",
        }
    }
}

/// Inspector property tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Captions,
    Text,
    Animation,
    Video,
    Audio,
    Basic,
}

impl InspectorTab {
    pub const ALL: [Self; 6] = [
        Self::Captions,
        Self::Text,
        Self::Animation,
        Self::Video,
        Self::Audio,
        Self::Basic,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Captions => "Captions",
            Self::Text => "Text",
            Self::Animation => "Animation",
            Self::Video => "Video",
            Self::Audio => "Audio",
            Self::Basic => "Basic",
        }
    }
}

/// Secondary tabs for caption/text styling in the Inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptionTab {
    Basic,
    Templates,
    Bubble,
    Effects,
}

impl CaptionTab {
    pub const ALL: [Self; 4] = [Self::Basic, Self::Templates, Self::Bubble, Self::Effects];

    pub fn label(self) -> &'static str {
        match self {
            Self::Basic => "Basic",
            Self::Templates => "Templates",
            Self::Bubble => "Bubble",
            Self::Effects => "Effects",
        }
    }
}

/// Alignment demo state for the Captions inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptionAlignment {
    Left,
    Center,
    Right,
    Justify,
}

impl CaptionAlignment {
    pub const ALL: [Self; 4] = [Self::Left, Self::Center, Self::Right, Self::Justify];
}

/// Case demo state for the Captions inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptionCase {
    Upper,
    Lower,
    Title,
}

impl CaptionCase {
    pub const ALL: [Self; 3] = [Self::Upper, Self::Lower, Self::Title];

    pub fn label(self) -> &'static str {
        match self {
            Self::Upper => "TT",
            Self::Lower => "tt",
            Self::Title => "Tt",
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
    pub caption_tab: CaptionTab,
    pub caption_apply_all: bool,
    pub caption_font_size: f32,
    pub caption_bold: bool,
    pub caption_underline: bool,
    pub caption_italic: bool,
    pub caption_case: CaptionCase,
    pub caption_color: [u8; 3],
    pub caption_character_spacing: f32,
    pub caption_line_spacing: f32,
    pub caption_alignment: CaptionAlignment,
    pub caption_style: usize,
    pub preview_aspect: PreviewAspect,
    pub export_open: bool,
    pub export_settings: ExportSettings,
    pub tools_visible: bool,
    pub inspector_visible: bool,
    pub timeline_visible: bool,
    pub timeline_pixels_per_second: f32,
    /// Supplied, non-authoritative presentation data; never seeded from demos.
    pub timeline_view: TimelineView,
    pub timeline_display_mode: TimelineDisplayMode,
    /// Session-owned scroll offsets prevent egui memory leaking into new projects.
    pub timeline_scroll: [f32; 2],
    /// Presentation selection over the displayed timeline; never media-pool state.
    pub(crate) timeline_selection: BTreeSet<TimelineItemId>,
    /// Nonfunctional toolbar preferences; editing behavior is not implemented.
    pub timeline_snapping: bool,
    pub timeline_linked_selection: bool,
    pub left_panel_fraction: f32,
    pub right_panel_fraction: f32,
}

impl Default for EditorSession {
    fn default() -> Self {
        Self {
            project_name: "Untitled Project".to_owned(),
            active_tool: EditorTool::Captions,
            inspector_tab: InspectorTab::Captions,
            caption_tab: CaptionTab::Basic,
            caption_apply_all: true,
            caption_font_size: 10.0,
            caption_bold: false,
            caption_underline: false,
            caption_italic: false,
            caption_case: CaptionCase::Title,
            caption_color: [255, 255, 255],
            caption_character_spacing: 0.0,
            caption_line_spacing: 0.0,
            caption_alignment: CaptionAlignment::Center,
            caption_style: 0,
            preview_aspect: PreviewAspect::Portrait,
            export_open: false,
            export_settings: ExportSettings::default(),
            tools_visible: true,
            inspector_visible: true,
            timeline_visible: true,
            timeline_pixels_per_second: config::TIMELINE_PIXELS_PER_SECOND,
            timeline_view: TimelineView::default(),
            timeline_display_mode: TimelineDisplayMode::Project,
            timeline_scroll: [0.0; 2],
            timeline_selection: BTreeSet::new(),
            timeline_snapping: true,
            timeline_linked_selection: true,
            left_panel_fraction: config::WORKSPACE_LEFT_FRACTION,
            right_panel_fraction: config::WORKSPACE_RIGHT_FRACTION,
        }
    }
}

impl EditorSession {
    /// Select a render source without ever modifying supplied timeline data.
    pub fn displayed_timeline(&self) -> &TimelineView {
        match self.timeline_display_mode {
            TimelineDisplayMode::Project => &self.timeline_view,
            TimelineDisplayMode::VisualDemo => timeline_state::visual_demo(),
        }
    }

    /// Opt into/out of QA. Each switch starts the selected view at zero.
    pub fn set_timeline_demo(&mut self, enabled: bool) {
        self.timeline_display_mode = if enabled {
            TimelineDisplayMode::VisualDemo
        } else {
            TimelineDisplayMode::Project
        };
        self.timeline_scroll = [0.0; 2];
        self.timeline_selection.clear();
    }

    pub fn timeline_selection(&self) -> &BTreeSet<TimelineItemId> {
        &self.timeline_selection
    }

    pub fn is_timeline_item_selected(&self, id: TimelineItemId) -> bool {
        self.timeline_selection.contains(&id)
    }

    /// Counts only ids that still resolve in the displayed view, so a supplied
    /// snapshot refresh that removes items cannot keep contextual tools visible.
    pub fn timeline_selection_context(&self) -> TimelineSelectionContext {
        TimelineSelectionContext::classify(self.displayed_timeline(), &self.timeline_selection)
    }

    pub fn clear_timeline_selection(&mut self) {
        self.timeline_selection.clear();
    }

    /// Select an existing displayed item. `additive` toggles one item; otherwise
    /// the click replaces the current selection. Unknown ids are ignored.
    pub fn select_timeline_item(&mut self, id: TimelineItemId, additive: bool) -> bool {
        let exists = self
            .displayed_timeline()
            .tracks
            .get(id.track)
            .and_then(|track| track.items.get(id.item))
            .is_some();
        if !exists {
            return false;
        }
        if additive {
            if !self.timeline_selection.remove(&id) {
                self.timeline_selection.insert(id);
            }
        } else {
            self.timeline_selection.clear();
            self.timeline_selection.insert(id);
        }
        true
    }

    /// Drop ids that no longer exist after supplied content changes.
    pub fn prune_timeline_selection(&mut self) {
        let stale: Vec<TimelineItemId> = {
            let view = self.displayed_timeline();
            self.timeline_selection
                .iter()
                .copied()
                .filter(|id| {
                    view.tracks
                        .get(id.track)
                        .and_then(|track| track.items.get(id.item))
                        .is_none()
                })
                .collect()
        };
        for id in stale {
            self.timeline_selection.remove(&id);
        }
    }

    /// UI preference only; no snapping behavior is applied yet.
    pub fn toggle_timeline_snapping(&mut self) {
        self.timeline_snapping = !self.timeline_snapping;
    }

    /// UI preference only; no linked-selection behavior is applied yet.
    pub fn toggle_timeline_linked_selection(&mut self) {
        self.timeline_linked_selection = !self.timeline_linked_selection;
    }

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

    /// Shared execution path for editor-scoped commands invoked by the toolbar
    /// or keyboard. Returns `false` for commands owned by the application shell
    /// or not yet implemented.
    pub fn execute_shortcut_command(&mut self, command: CommandId) -> bool {
        match command {
            CommandId::SnappingToggle => self.toggle_timeline_snapping(),
            CommandId::LinkedSelection => self.toggle_timeline_linked_selection(),
            CommandId::ZoomIn => self.zoom_timeline_in(),
            CommandId::ZoomOut => self.zoom_timeline_out(),
            CommandId::DeselectAll => self.clear_timeline_selection(),
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline_state::{
        TimelineItemView, TimelineSelectionKind, TimelineTrackView, TrackKind,
    };

    fn populated_session() -> EditorSession {
        let mut session = EditorSession::default();
        session.timeline_view.tracks = vec![
            TimelineTrackView {
                label: "V1".to_owned(),
                name: "Video".to_owned(),
                kind: TrackKind::Video,
                items: vec![TimelineItemView::new("Video clip", 1.0, 4.0).expect("valid")],
            },
            TimelineTrackView {
                label: "A1".to_owned(),
                name: "Audio".to_owned(),
                kind: TrackKind::Audio,
                items: vec![TimelineItemView::new("Audio clip", 1.0, 4.0).expect("valid")],
            },
        ];
        session
    }

    const VIDEO: TimelineItemId = TimelineItemId { track: 0, item: 0 };
    const AUDIO: TimelineItemId = TimelineItemId { track: 1, item: 0 };

    #[test]
    fn new_session_is_unsaved_and_starts_on_captions_preview() {
        let session = EditorSession::default();

        assert_eq!(session.project_name, "Untitled Project");
        assert_eq!(session.active_tool, EditorTool::Captions);
        assert_eq!(session.inspector_tab, InspectorTab::Captions);
        assert_eq!(session.preview_aspect, PreviewAspect::Portrait);
        assert_eq!(session.caption_tab, CaptionTab::Basic);
        assert_eq!(session.caption_font_size, 10.0);
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
        assert_eq!(EditorTool::ALL.len(), 10);
        assert_eq!(EditorTool::Stickers.label(), "Stickers");
        assert_eq!(EditorTool::Templates.label(), "Templates");
        assert_eq!(InspectorTab::ALL.len(), 6);
        assert_eq!(InspectorTab::Captions.label(), "Captions");
        assert_eq!(CaptionTab::ALL.len(), 4);
        assert_eq!(CaptionTab::Bubble.label(), "Bubble");
    }

    #[test]
    fn timeline_toolbar_preferences_toggle_without_editing_behavior() {
        let mut session = EditorSession::default();
        assert!(session.timeline_snapping);
        assert!(session.timeline_linked_selection);

        session.toggle_timeline_snapping();
        session.toggle_timeline_linked_selection();
        assert!(!session.timeline_snapping);
        assert!(!session.timeline_linked_selection);

        session.toggle_timeline_snapping();
        session.toggle_timeline_linked_selection();
        assert!(session.timeline_snapping);
        assert!(session.timeline_linked_selection);
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

    #[test]
    fn empty_and_unselected_populated_timelines_have_no_clip_context() {
        let empty = EditorSession::default();
        assert!(empty.timeline_selection().is_empty());
        assert!(empty.timeline_selection_context().is_empty());
        assert_eq!(empty.timeline_selection_context().kind(), None);

        let populated = populated_session();
        assert!(populated.timeline_selection().is_empty());
        assert_eq!(populated.timeline_selection_context().kind(), None);
    }

    #[test]
    fn video_audio_and_mixed_selections_are_classified_from_real_items() {
        let mut session = populated_session();

        assert!(session.select_timeline_item(VIDEO, false));
        assert_eq!(
            session.timeline_selection_context().kind(),
            Some(TimelineSelectionKind::VideoOnly)
        );

        assert!(session.select_timeline_item(AUDIO, false));
        assert_eq!(
            session.timeline_selection_context().kind(),
            Some(TimelineSelectionKind::AudioOnly)
        );
        assert_eq!(session.timeline_selection().len(), 1);

        assert!(session.select_timeline_item(VIDEO, true));
        let mixed = session.timeline_selection_context();
        assert_eq!(mixed.kind(), Some(TimelineSelectionKind::Mixed));
        assert_eq!(mixed.selected, 2);
        assert_eq!((mixed.video, mixed.audio), (1, 1));

        assert!(session.select_timeline_item(VIDEO, true));
        assert_eq!(
            session.timeline_selection_context().kind(),
            Some(TimelineSelectionKind::AudioOnly)
        );
    }

    #[test]
    fn selecting_unknown_or_removed_items_never_creates_a_selection() {
        let mut session = populated_session();
        let unknown = TimelineItemId { track: 9, item: 0 };
        assert!(!session.select_timeline_item(unknown, false));
        assert!(session.timeline_selection().is_empty());

        assert!(session.select_timeline_item(VIDEO, false));
        session.timeline_view.tracks[0].items.clear();
        assert_eq!(session.timeline_selection_context().kind(), None);
        assert_eq!(session.timeline_selection().len(), 1);

        session.prune_timeline_selection();
        assert!(session.timeline_selection().is_empty());
    }

    #[test]
    fn removing_the_last_selected_item_hides_contextual_tools() {
        let mut session = populated_session();
        assert!(session.select_timeline_item(AUDIO, false));
        assert!(session.timeline_selection_context().kind().is_some());

        session.timeline_view.tracks.clear();
        session.prune_timeline_selection();

        assert!(session.timeline_selection().is_empty());
        assert_eq!(session.timeline_selection_context().kind(), None);
    }

    #[test]
    fn clearing_or_switching_the_view_removes_selection() {
        let mut session = populated_session();
        assert!(session.select_timeline_item(VIDEO, false));
        session.clear_timeline_selection();
        assert_eq!(session.timeline_selection_context().kind(), None);

        assert!(session.select_timeline_item(VIDEO, false));
        session.set_timeline_demo(true);
        assert!(session.timeline_selection().is_empty());
        assert_eq!(session.timeline_selection_context().kind(), None);
    }

    #[test]
    fn selection_changes_never_modify_zoom_or_toolbar_preferences() {
        let mut session = populated_session();
        let zoom = session.timeline_pixels_per_second;
        let snapping = session.timeline_snapping;
        let linked = session.timeline_linked_selection;

        assert!(session.select_timeline_item(VIDEO, false));
        assert!(session.select_timeline_item(AUDIO, true));
        session.clear_timeline_selection();

        assert_eq!(session.timeline_pixels_per_second, zoom);
        assert_eq!(session.timeline_snapping, snapping);
        assert_eq!(session.timeline_linked_selection, linked);
        assert_eq!(session.displayed_timeline(), &session.timeline_view);
    }

    #[test]
    fn toolbar_and_keyboard_share_the_same_editor_command_path() {
        let mut session = populated_session();
        assert!(session.select_timeline_item(VIDEO, false));
        let zoom = session.timeline_pixels_per_second;

        assert!(session.execute_shortcut_command(CommandId::SnappingToggle));
        assert!(!session.timeline_snapping);
        assert!(session.execute_shortcut_command(CommandId::LinkedSelection));
        assert!(!session.timeline_linked_selection);
        assert!(session.execute_shortcut_command(CommandId::ZoomIn));
        assert!(session.timeline_pixels_per_second > zoom);
        assert!(session.execute_shortcut_command(CommandId::DeselectAll));
        assert!(session.timeline_selection().is_empty());
        assert!(!session.execute_shortcut_command(CommandId::SplitAtPlayhead));
    }
}
