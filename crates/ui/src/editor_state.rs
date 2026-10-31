//! Transient editor-session state.
//!
//! A session exists only in memory. It contains UI preferences and form values
//! needed to keep the editor coherent while navigating; it is not an engine
//! project and is never written to disk.

use std::collections::BTreeSet;

use capcut_core::time::{FrameRate, TimelineTicks};
use capcut_engine::asset::{
    MediaAssetId, MediaAssetMetadata, MediaAssetRegistration, MediaLocator,
};
use capcut_engine::command::{Command, CommandOutcome};
use capcut_engine::project::{ClipId, Project, ProjectSnapshot};
use capcut_project::{LoadedProject, PreparedProject, ProjectId, SavedDocument};

use crate::config;
use crate::export_settings::ExportSettings;
use crate::native_document::{AutoSaveStatus, NativeDocumentState};
use crate::native_timeline::NativeTimeline;
use crate::shortcuts::CommandId;
use crate::timeline_state::{
    self, ThumbnailMode, TimelineDisplayMode, TimelineItemId, TimelineSelectionContext,
    TimelineView,
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
    /// Timeline display preferences; they never mutate project media.
    pub timeline_show_thumbnails: bool,
    pub timeline_thumbnail_mode: ThumbnailMode,
    pub timeline_show_waveforms: bool,
    pub left_panel_fraction: f32,
    pub right_panel_fraction: f32,
    /// `None` uses the responsive default; `Some` preserves the user's height.
    pub timeline_height_fraction: Option<f32>,
    pub(crate) layout_save_requested: bool,
    /// Present only when a native editor is active.
    pub native_timeline: Option<NativeTimeline>,
    /// File identity, path and dirty state; never part of engine history.
    pub native_document: Option<NativeDocumentState>,
    /// Session-only authoritative playhead. Never persisted or included in history.
    pub playhead_ticks: TimelineTicks,
    /// Transient ruler-drag state; never project content.
    pub timeline_scrubbing: bool,
    /// Session preference; independent from Snapping.
    pub main_track_magnet_enabled: bool,
    pub native_error: Option<String>,
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
            timeline_show_thumbnails: true,
            timeline_thumbnail_mode: ThumbnailMode::Continuous,
            timeline_show_waveforms: true,
            left_panel_fraction: config::WORKSPACE_LEFT_FRACTION,
            right_panel_fraction: config::WORKSPACE_RIGHT_FRACTION,
            timeline_height_fraction: None,
            layout_save_requested: false,
            native_timeline: None,
            native_document: None,
            playhead_ticks: TimelineTicks::ZERO,
            timeline_scrubbing: false,
            main_track_magnet_enabled: false,
            native_error: None,
        }
    }
}

impl EditorSession {
    /// Construct the explicitly labeled native QA project. Ordinary sessions
    /// never call this path.
    pub fn with_native_qa() -> Result<Self, String> {
        let native = NativeTimeline::qa_project()?;
        let view = native.view();
        let project_name = native.project().name().to_owned();
        let magnet = native.magnet_enabled();
        Ok(Self {
            project_name,
            timeline_view: view,
            native_timeline: Some(native),
            native_document: Some(NativeDocumentState::unsaved(ProjectId::generate(), true)),
            main_track_magnet_enabled: magnet,
            ..Self::default()
        })
    }

    /// Start an empty production-native project with no synthetic content.
    pub fn new_native() -> Result<Self, String> {
        let fps = FrameRate::new(30, 1).map_err(|error| error.to_string())?;
        let project = Project::named(fps, "Untitled Project").map_err(|error| error.to_string())?;
        let native = NativeTimeline::from_project(project)?;
        let view = native.view();
        let magnet = native.magnet_enabled();
        Ok(Self {
            project_name: "Untitled Project".to_owned(),
            timeline_view: view,
            native_timeline: Some(native),
            native_document: Some(NativeDocumentState::unsaved(ProjectId::generate(), false)),
            main_track_magnet_enabled: magnet,
            ..Self::default()
        })
    }

    /// Build the editor session for a newly published managed project.
    pub fn from_created_project(
        prepared: &PreparedProject,
        snapshot: ProjectSnapshot,
        saved: &SavedDocument,
    ) -> Result<Self, String> {
        let magnet = snapshot.magnet_enabled;
        let editor = capcut_engine::command::NativeEditor::from_restored(
            snapshot.project,
            snapshot.magnet_enabled,
        )
        .map_err(|error| error.to_string())?;
        let native = NativeTimeline::from_editor(editor)?;
        let view = native.view();
        Ok(Self {
            project_name: prepared.name.clone(),
            timeline_view: view,
            native_timeline: Some(native),
            native_document: Some(NativeDocumentState::created(
                prepared.project_id.clone(),
                prepared.path.clone(),
                saved.snapshot.clone(),
                prepared.created_at_unix_ms,
                saved.last_saved_unix_ms,
            )),
            main_track_magnet_enabled: magnet,
            ..Self::default()
        })
    }

    /// Build a session from a fully validated saved document. History starts
    /// empty and selection starts unselected.
    pub fn with_loaded_project(loaded: LoadedProject) -> Result<Self, String> {
        let LoadedProject {
            project_id,
            editor,
            snapshot,
            created_at_unix_ms,
            last_saved_unix_ms,
            save_generation,
            ..
        } = loaded;
        let project_name = editor.project().name().to_owned();
        let magnet = editor.magnet_enabled();
        let native = NativeTimeline::from_editor(editor)?;
        let view = native.view();
        Ok(Self {
            project_name,
            timeline_view: view,
            native_timeline: Some(native),
            native_document: Some(NativeDocumentState::loaded(
                project_id,
                snapshot,
                created_at_unix_ms,
                last_saved_unix_ms,
                save_generation,
                false,
            )),
            main_track_magnet_enabled: magnet,
            ..Self::default()
        })
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
        self.sync_native_selection();
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
        self.sync_native_selection();
    }

    pub fn main_track_magnet_enabled(&self) -> bool {
        self.native_timeline.as_ref().map_or(
            self.main_track_magnet_enabled,
            NativeTimeline::magnet_enabled,
        )
    }

    pub fn native_document(&self) -> Option<&NativeDocumentState> {
        self.native_document.as_ref()
    }

    pub fn native_frame_rate(&self) -> Option<FrameRate> {
        self.native_timeline
            .as_ref()
            .map(|native| native.project().fps())
    }

    pub fn native_content_end(&self) -> Option<TimelineTicks> {
        self.native_timeline
            .as_ref()
            .map(|native| native.project().content_end())
    }

    pub fn playhead_ticks(&self) -> TimelineTicks {
        self.playhead_ticks
    }

    /// Clamp an authoritative tick to the native project range. Playhead moves
    /// are session state, so they never dirty the document or create history.
    pub fn set_playhead_ticks(&mut self, ticks: TimelineTicks) -> bool {
        let Some(end) = self.native_content_end() else {
            return false;
        };
        let clamped = TimelineTicks::new(ticks.get().clamp(0, end.get()));
        if clamped == self.playhead_ticks {
            return false;
        }
        self.playhead_ticks = clamped;
        true
    }

    pub fn previous_frame(&mut self) -> Result<bool, String> {
        let Some(fps) = self.native_frame_rate() else {
            return Ok(false);
        };
        let Some(end) = self.native_content_end() else {
            return Ok(false);
        };
        if end.get() == 0 {
            return Ok(self.set_playhead_ticks(TimelineTicks::ZERO));
        }
        let current = self.playhead_ticks;
        let frame = fps
            .ticks_to_frame_floor(current)
            .map_err(|error| error.to_string())?;
        let frame_start = fps
            .frame_to_ticks(frame)
            .map_err(|error| error.to_string())?;
        let target_frame = if current.get() > frame_start.get() {
            frame
        } else {
            frame.checked_sub(1).unwrap_or(0)
        };
        let target = fps
            .frame_to_ticks(target_frame)
            .map_err(|error| error.to_string())?;
        Ok(self.set_playhead_ticks(target))
    }

    pub fn next_frame(&mut self) -> Result<bool, String> {
        let Some(fps) = self.native_frame_rate() else {
            return Ok(false);
        };
        let Some(end) = self.native_content_end() else {
            return Ok(false);
        };
        if end.get() == 0 {
            return Ok(self.set_playhead_ticks(TimelineTicks::ZERO));
        }
        let frame = fps
            .ticks_to_frame_floor(self.playhead_ticks)
            .map_err(|error| error.to_string())?;
        let target_frame = frame.checked_add(1).ok_or("playhead frame overflow")?;
        let target = fps
            .frame_to_ticks(target_frame)
            .map_err(|error| error.to_string())?;
        Ok(self.set_playhead_ticks(target))
    }

    pub fn jump_playhead_to_start(&mut self) -> bool {
        self.set_playhead_ticks(TimelineTicks::ZERO)
    }

    pub fn jump_playhead_to_end(&mut self) -> bool {
        let Some(end) = self.native_content_end() else {
            return false;
        };
        self.set_playhead_ticks(end)
    }

    fn clamp_playhead_to_native_end(&mut self) {
        if let Some(end) = self.native_content_end() {
            if self.playhead_ticks.get() > end.get() || self.playhead_ticks.get() < 0 {
                self.set_playhead_ticks(end);
            }
        }
    }

    pub fn can_split_at_playhead(&self) -> bool {
        if self.timeline_display_mode != TimelineDisplayMode::Project
            || self.timeline_selection.len() != 1
        {
            return false;
        }
        let Some(native) = &self.native_timeline else {
            return false;
        };
        let clips = native.selected_clips(&self.timeline_selection);
        clips.len() == 1 && native.can_split_clip(clips[0], self.playhead_ticks)
    }

    pub fn split_at_playhead(&mut self) -> Result<(), String> {
        if self.timeline_display_mode != TimelineDisplayMode::Project {
            return Err("Switch to the native project view before splitting.".to_owned());
        }
        if self.timeline_selection.len() != 1 {
            return Err("Split requires exactly one selected clip.".to_owned());
        }
        let clip = {
            let Some(native) = &self.native_timeline else {
                return Err("No native timeline is active.".to_owned());
            };
            let clips = native.selected_clips(&self.timeline_selection);
            if clips.len() != 1 {
                return Err("Split requires exactly one selected native clip.".to_owned());
            }
            clips[0]
        };
        let result = {
            let Some(native) = self.native_timeline.as_mut() else {
                return Err("No native timeline is active.".to_owned());
            };
            native
                .split_clip(clip, self.playhead_ticks)
                .map_err(|error| error.to_string())
        };
        match result {
            Ok(()) => {
                self.native_error = None;
                self.refresh_native_projection();
                self.clamp_playhead_to_native_end();
                self.mark_persistent_change();
                Ok(())
            }
            Err(error) => {
                self.native_error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn native_document_mut(&mut self) -> Option<&mut NativeDocumentState> {
        self.native_document.as_mut()
    }

    pub fn add_media_asset(
        &mut self,
        registration: MediaAssetRegistration,
    ) -> Result<MediaAssetId, String> {
        let outcome = self.execute_native(Command::AddMediaAsset {
            asset: registration,
        })?;
        if outcome.changed {
            self.mark_persistent_change();
        }
        outcome
            .created_asset
            .ok_or_else(|| "media asset command did not return an ID".to_owned())
    }

    pub fn relink_media_asset(
        &mut self,
        asset: MediaAssetId,
        locator: MediaLocator,
        metadata: MediaAssetMetadata,
        file_size_bytes: Option<u64>,
    ) -> Result<bool, String> {
        let outcome = self.execute_native(Command::RelinkMediaAsset {
            asset,
            locator,
            metadata,
            file_size_bytes,
        })?;
        if outcome.changed {
            self.mark_persistent_change();
        }
        Ok(outcome.changed)
    }

    pub fn append_media_asset(&mut self, asset: MediaAssetId) -> Result<Option<ClipId>, String> {
        let outcome = self.execute_native(Command::AppendMediaClip { asset })?;
        let Some(clip) = outcome.affected.first().copied() else {
            return Ok(None);
        };
        self.refresh_native_projection();
        if let Some(native) = &self.native_timeline {
            if let Some(item) = native.item_for_clip(clip) {
                self.select_timeline_item(item, false);
            }
        }
        if let Some(native) = &self.native_timeline {
            if let Some(native_clip) = native.project().clip(clip) {
                self.set_playhead_ticks(native_clip.range().start());
            }
        }
        self.mark_persistent_change();
        Ok(Some(clip))
    }

    pub fn rename_native_project(&mut self, name: impl Into<String>) -> Result<bool, String> {
        let name = name.into();
        let outcome = self.execute_native(Command::RenameProject { name: name.clone() })?;
        if outcome.changed {
            self.project_name = name;
            self.mark_persistent_change();
        }
        Ok(outcome.changed)
    }

    fn execute_native(&mut self, command: Command) -> Result<CommandOutcome, String> {
        let outcome = {
            let native = self
                .native_timeline
                .as_mut()
                .ok_or_else(|| "No native timeline is active.".to_owned())?;
            native.execute(command).map_err(|error| error.to_string())?
        };
        if outcome.changed {
            self.native_error = None;
            self.refresh_native_projection();
        }
        Ok(outcome)
    }

    pub fn auto_save_status(&self) -> Option<&AutoSaveStatus> {
        self.native_document
            .as_ref()
            .map(NativeDocumentState::auto_save_status)
    }

    pub fn save_status(&self) -> Option<crate::native_document::SaveStatus> {
        self.native_document
            .as_ref()
            .map(NativeDocumentState::save_status)
    }

    pub fn persistent_generation(&self) -> Option<u64> {
        self.native_document
            .as_ref()
            .map(NativeDocumentState::generation)
    }

    pub fn persisted_generation(&self) -> Option<u64> {
        self.native_document
            .as_ref()
            .map(NativeDocumentState::saved_generation)
    }

    pub fn native_is_dirty(&self) -> bool {
        self.native_document
            .as_ref()
            .is_some_and(NativeDocumentState::is_dirty)
    }

    fn mark_persistent_change(&mut self) {
        if let Some(document) = &mut self.native_document {
            document.mark_persistent_change();
        }
    }

    /// Activation compacts the main track in one engine transaction. Without a
    /// native timeline this remains a UI-only preference.
    pub fn toggle_main_track_magnet(&mut self) {
        let Some(native) = self.native_timeline.as_mut() else {
            self.main_track_magnet_enabled = !self.main_track_magnet_enabled;
            return;
        };
        let result = if native.magnet_enabled() {
            native.disable_magnet();
            Ok(())
        } else {
            native.enable_magnet()
        };
        match result {
            Ok(()) => {
                self.sync_magnet_from_native();
                self.native_error = None;
                self.refresh_native_projection();
                self.clamp_playhead_to_native_end();
                self.mark_persistent_change();
            }
            Err(error) => self.native_error = Some(error),
        }
    }

    fn sync_magnet_from_native(&mut self) {
        if let Some(native) = &self.native_timeline {
            self.main_track_magnet_enabled = native.magnet_enabled();
        }
    }

    pub fn native_timeline_active(&self) -> bool {
        self.native_timeline.is_some()
    }

    pub fn can_delete_native(&self) -> bool {
        if self.timeline_display_mode != TimelineDisplayMode::Project
            || self.timeline_selection.len() != 1
        {
            return false;
        }
        self.native_timeline
            .as_ref()
            .is_some_and(|native| native.can_delete_selected(&self.timeline_selection))
    }

    pub fn can_undo_native(&self) -> bool {
        self.native_timeline
            .as_ref()
            .is_some_and(NativeTimeline::can_undo)
    }

    pub fn can_redo_native(&self) -> bool {
        self.native_timeline
            .as_ref()
            .is_some_and(NativeTimeline::can_redo)
    }

    pub fn delete_selected_native(&mut self) -> Result<(), String> {
        if self.timeline_display_mode != TimelineDisplayMode::Project {
            return Err("Switch to the native project view before deleting.".to_owned());
        }
        let selected = self.timeline_selection.clone();
        let Some(native) = self.native_timeline.as_mut() else {
            return Err("No native timeline is active.".to_owned());
        };
        match native.delete_selected(&selected) {
            Ok(()) => {
                self.native_error = None;
                self.refresh_native_projection();
                self.clamp_playhead_to_native_end();
                self.mark_persistent_change();
                Ok(())
            }
            Err(error) => {
                self.native_error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn undo_native(&mut self) -> Result<(), String> {
        let Some(native) = self.native_timeline.as_mut() else {
            return Err("No native timeline is active.".to_owned());
        };
        match native.undo() {
            Ok(()) => {
                self.native_error = None;
                self.refresh_native_projection();
                self.clamp_playhead_to_native_end();
                self.mark_persistent_change();
                Ok(())
            }
            Err(error) => {
                self.native_error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn redo_native(&mut self) -> Result<(), String> {
        let Some(native) = self.native_timeline.as_mut() else {
            return Err("No native timeline is active.".to_owned());
        };
        match native.redo() {
            Ok(()) => {
                self.native_error = None;
                self.refresh_native_projection();
                self.clamp_playhead_to_native_end();
                self.mark_persistent_change();
                Ok(())
            }
            Err(error) => {
                self.native_error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn refresh_native_projection(&mut self) {
        let Some(native) = &self.native_timeline else {
            return;
        };
        let view = native.view();
        let selected = native.selected_items();
        self.main_track_magnet_enabled = native.magnet_enabled();
        self.timeline_view = view;
        self.timeline_selection = selected;
    }

    fn sync_native_selection(&mut self) {
        if self.timeline_display_mode != TimelineDisplayMode::Project {
            return;
        }
        let Some(native) = self.native_timeline.as_mut() else {
            return;
        };
        native.sync_selection(&self.timeline_selection);
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
        self.sync_native_selection();
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

    pub fn set_timeline_height_fraction(&mut self, fraction: Option<f32>) {
        self.timeline_height_fraction = fraction
            .map(|value| value.clamp(0.0, 1.0))
            .filter(|value| value.is_finite());
    }

    pub fn request_layout_save(&mut self) {
        self.layout_save_requested = true;
    }

    pub fn take_layout_save_request(&mut self) -> bool {
        std::mem::take(&mut self.layout_save_requested)
    }

    pub fn reset_layout(&mut self) {
        self.tools_visible = true;
        self.inspector_visible = true;
        self.timeline_visible = true;
        self.left_panel_fraction = config::WORKSPACE_LEFT_FRACTION;
        self.right_panel_fraction = config::WORKSPACE_RIGHT_FRACTION;
        self.timeline_height_fraction = None;
        self.request_layout_save();
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
            CommandId::Undo if self.native_timeline.is_some() => {
                let _ = self.undo_native();
            }
            CommandId::Redo if self.native_timeline.is_some() => {
                let _ = self.redo_native();
            }
            CommandId::DeleteSelected if self.native_timeline.is_some() => {
                let _ = self.delete_selected_native();
            }
            CommandId::PreviousFrame if self.native_timeline.is_some() => {
                if let Err(error) = self.previous_frame() {
                    self.native_error = Some(error);
                }
            }
            CommandId::NextFrame if self.native_timeline.is_some() => {
                if let Err(error) = self.next_frame() {
                    self.native_error = Some(error);
                }
            }
            CommandId::JumpToBeginning if self.native_timeline.is_some() => {
                self.jump_playhead_to_start();
            }
            CommandId::JumpToEnd if self.native_timeline.is_some() => {
                self.jump_playhead_to_end();
            }
            CommandId::SplitAtPlayhead if self.can_split_at_playhead() => {
                let _ = self.split_at_playhead();
            }
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
    fn timeline_height_defaults_to_auto_and_reset_restores_it() {
        let mut session = EditorSession::default();
        assert_eq!(session.timeline_height_fraction, None);

        session.set_timeline_height_fraction(Some(0.62));
        assert_eq!(session.timeline_height_fraction, Some(0.62));

        session.reset_layout();

        assert_eq!(session.timeline_height_fraction, None);
        assert!(
            session.take_layout_save_request(),
            "reset must persist the restored height"
        );
        assert!(!session.take_layout_save_request());
    }

    #[test]
    fn invalid_timeline_height_fraction_falls_back_to_auto() {
        let mut session = EditorSession::default();
        session.set_timeline_height_fraction(Some(f32::NAN));
        assert_eq!(session.timeline_height_fraction, None);

        session.set_timeline_height_fraction(Some(1.5));
        assert_eq!(session.timeline_height_fraction, Some(1.0));
    }

    #[test]
    fn timeline_height_changes_do_not_touch_zoom_or_selection() {
        let mut session = populated_session();
        session.timeline_pixels_per_second = 42.0;
        session.select_timeline_item(VIDEO, false);

        session.set_timeline_height_fraction(Some(0.70));
        session.reset_layout();

        assert_eq!(session.timeline_pixels_per_second, 42.0);
        assert!(session.is_timeline_item_selected(VIDEO));
        assert_eq!(session.timeline_height_fraction, None);
    }

    fn main_starts(session: &EditorSession) -> Vec<f64> {
        session.displayed_timeline().tracks[1]
            .items
            .iter()
            .map(TimelineItemView::start_seconds)
            .collect()
    }

    #[test]
    fn native_qa_session_supports_one_selection_delete_and_history() {
        let mut session = EditorSession::with_native_qa().expect("qa session");
        assert_eq!(session.displayed_timeline().tracks[0].items.len(), 1);
        assert_eq!(session.displayed_timeline().tracks[1].items.len(), 4);
        assert!(!session.can_delete_native());

        let middle = TimelineItemId { track: 1, item: 1 };
        assert!(session.select_timeline_item(middle, false));
        assert!(session.can_delete_native());
        let revision = session
            .native_timeline
            .as_ref()
            .expect("native")
            .project()
            .revision();
        let snapping_before = session.timeline_snapping;
        session.toggle_main_track_magnet();
        assert!(session.main_track_magnet_enabled());
        assert_eq!(session.timeline_snapping, snapping_before);
        assert_eq!(main_starts(&session), vec![0.0, 5.0, 10.0, 15.0]);
        assert_eq!(
            session.displayed_timeline().tracks[0].items[0].start_seconds(),
            12.0,
            "unlinked text must not move with the main track"
        );
        assert!(
            session
                .native_timeline
                .as_ref()
                .expect("native")
                .project()
                .revision()
                > revision,
            "Magnet activation that closes gaps is a real transaction"
        );

        session.delete_selected_native().expect("delete");
        assert_eq!(session.displayed_timeline().tracks[1].items.len(), 3);
        assert_eq!(main_starts(&session), vec![0.0, 5.0, 10.0]);
        assert!(session.can_undo_native());
        assert!(!session.can_delete_native());
        session.undo_native().expect("undo");
        assert_eq!(session.displayed_timeline().tracks[1].items.len(), 4);
        assert_eq!(main_starts(&session), vec![0.0, 5.0, 10.0, 15.0]);
        assert!(session.can_delete_native());
        session.redo_native().expect("redo");
        assert_eq!(session.displayed_timeline().tracks[1].items.len(), 3);
    }

    #[test]
    fn native_magnet_compacts_on_activation_and_off_preserves_gaps() {
        let mut session = EditorSession::with_native_qa().expect("qa session");
        let middle = TimelineItemId { track: 1, item: 1 };
        assert!(session.select_timeline_item(middle, false));
        session.toggle_main_track_magnet();
        assert_eq!(main_starts(&session), vec![0.0, 5.0, 10.0, 15.0]);
        session.delete_selected_native().expect("delete on");
        assert_eq!(main_starts(&session), vec![0.0, 5.0, 10.0]);

        session.undo_native().expect("undo delete");
        session.undo_native().expect("undo activation");
        assert!(!session.main_track_magnet_enabled());
        assert_eq!(main_starts(&session), vec![0.0, 10.0, 20.0, 30.0]);

        assert!(session.select_timeline_item(middle, false));
        session.delete_selected_native().expect("delete off");
        assert_eq!(main_starts(&session), vec![0.0, 20.0, 30.0]);
    }

    #[test]
    fn redo_restores_recorded_magnet_state_after_preference_toggle() {
        let mut session = EditorSession::with_native_qa().expect("qa session");
        let middle = TimelineItemId { track: 1, item: 1 };
        assert!(session.select_timeline_item(middle, false));
        session.toggle_main_track_magnet();
        session.delete_selected_native().expect("delete on");
        assert_eq!(main_starts(&session), vec![0.0, 5.0, 10.0]);

        session.undo_native().expect("undo");
        assert_eq!(main_starts(&session), vec![0.0, 5.0, 10.0, 15.0]);
        assert!(session.main_track_magnet_enabled());

        session.toggle_main_track_magnet();
        assert!(!session.main_track_magnet_enabled());
        assert_eq!(main_starts(&session), vec![0.0, 5.0, 10.0, 15.0]);

        session.redo_native().expect("redo");
        assert_eq!(main_starts(&session), vec![0.0, 5.0, 10.0]);
        assert!(
            session.main_track_magnet_enabled(),
            "redo restores the Magnet state recorded with the command"
        );
    }

    #[test]
    fn magnet_and_snapping_preferences_are_independent() {
        let mut session = EditorSession::with_native_qa().expect("qa session");
        session.toggle_timeline_snapping();
        assert!(!session.timeline_snapping);
        assert!(!session.main_track_magnet_enabled());
        assert_eq!(main_starts(&session), vec![0.0, 10.0, 20.0, 30.0]);

        session.toggle_main_track_magnet();
        assert!(!session.timeline_snapping);
        assert!(session.main_track_magnet_enabled());
        assert_eq!(main_starts(&session), vec![0.0, 5.0, 10.0, 15.0]);

        session.toggle_timeline_snapping();
        assert!(session.timeline_snapping);
        assert!(session.main_track_magnet_enabled());
        assert_eq!(main_starts(&session), vec![0.0, 5.0, 10.0, 15.0]);
    }

    #[test]
    fn native_zoom_does_not_change_authoritative_time() {
        let mut session = EditorSession::with_native_qa().expect("qa session");
        let before = main_starts(&session);
        let revision = session
            .native_timeline
            .as_ref()
            .expect("native")
            .project()
            .revision();
        session.zoom_timeline_in();
        session.zoom_timeline_out();
        assert_eq!(
            session
                .native_timeline
                .as_ref()
                .expect("native")
                .project()
                .revision(),
            revision
        );
        assert_eq!(main_starts(&session), before);
    }

    #[test]
    fn multi_selection_disables_native_delete() {
        let mut session = EditorSession::with_native_qa().expect("qa session");
        assert!(session.select_timeline_item(TimelineItemId { track: 1, item: 0 }, false));
        assert!(session.select_timeline_item(TimelineItemId { track: 1, item: 1 }, true));
        assert_eq!(session.timeline_selection().len(), 2);
        assert!(!session.can_delete_native());
        assert!(session.delete_selected_native().is_err());
        assert_eq!(session.displayed_timeline().tracks[1].items.len(), 4);
    }

    #[test]
    fn ordinary_session_has_no_native_timeline() {
        let mut session = EditorSession::default();
        assert!(session.native_timeline.is_none());
        assert!(!session.can_delete_native());
        assert!(session.displayed_timeline().tracks.is_empty());
        session.toggle_main_track_magnet();
        assert!(session.main_track_magnet_enabled());
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
    fn timeline_display_preferences_default_to_rich_rendering() {
        let mut session = EditorSession::default();
        assert!(session.timeline_show_thumbnails);
        assert!(session.timeline_show_waveforms);
        assert_eq!(session.timeline_thumbnail_mode, ThumbnailMode::Continuous);

        session.timeline_show_thumbnails = false;
        session.timeline_show_waveforms = false;
        session.timeline_thumbnail_mode = ThumbnailMode::FirstAndLast;
        assert!(!session.timeline_show_thumbnails);
        assert!(!session.timeline_show_waveforms);
        assert_eq!(session.timeline_thumbnail_mode, ThumbnailMode::FirstAndLast);
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

#[cfg(test)]
#[path = "editor_state_m3b_tests.rs"]
mod m3b_tests;
