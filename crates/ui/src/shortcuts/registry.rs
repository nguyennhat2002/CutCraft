//! Central command registry for keyboard shortcuts.
//!
//! Metadata and default bindings live here; execution lives in `AppState`.
//! Planned commands remain visible in the manager but can never execute, so
//! adding a binding does not imply that the feature exists.

use std::collections::BTreeMap;

use egui::Key;

use super::binding::ShortcutBinding;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandId {
    SelectionTool,
    SplitAtPlayhead,
    SplitAll,
    DeselectAll,
    DeleteSelected,
    DeleteLeft,
    DeleteRight,
    Undo,
    Redo,
    SnappingToggle,
    LinkedSelection,
    ZoomIn,
    ZoomOut,
    FitTimeline,
    AddTrack,
    FreezeFrame,
    DetachAudio,
    AddKeyframe,
    PlayPause,
    PreviousFrame,
    NextFrame,
    SeekBackward,
    SeekForward,
    JumpToBeginning,
    JumpToEnd,
    FullscreenPreview,
    NewProject,
    OpenProject,
    SaveProject,
    ImportMedia,
    Export,
    Preferences,
    ShortcutsDialog,
    TogglePanels,
    InspectorVisibility,
    TimelineFocus,
    PreviewFocus,
    WorkspaceNavigation,
}

impl CommandId {
    /// Stable identifier used by the persisted configuration.
    pub const fn id(self) -> &'static str {
        match self {
            Self::SelectionTool => "timeline.selection_tool",
            Self::SplitAtPlayhead => "timeline.split_at_playhead",
            Self::SplitAll => "timeline.split_all",
            Self::DeselectAll => "timeline.deselect_all",
            Self::DeleteSelected => "timeline.delete_selected",
            Self::DeleteLeft => "timeline.delete_left",
            Self::DeleteRight => "timeline.delete_right",
            Self::Undo => "editor.undo",
            Self::Redo => "editor.redo",
            Self::SnappingToggle => "timeline.snapping_toggle",
            Self::LinkedSelection => "timeline.linked_selection",
            Self::ZoomIn => "timeline.zoom_in",
            Self::ZoomOut => "timeline.zoom_out",
            Self::FitTimeline => "timeline.fit",
            Self::AddTrack => "timeline.add_track",
            Self::FreezeFrame => "timeline.freeze_frame",
            Self::DetachAudio => "timeline.detach_audio",
            Self::AddKeyframe => "timeline.add_keyframe",
            Self::PlayPause => "player.play_pause",
            Self::PreviousFrame => "player.previous_frame",
            Self::NextFrame => "player.next_frame",
            Self::SeekBackward => "player.seek_backward",
            Self::SeekForward => "player.seek_forward",
            Self::JumpToBeginning => "player.jump_to_beginning",
            Self::JumpToEnd => "player.jump_to_end",
            Self::FullscreenPreview => "player.fullscreen_preview",
            Self::NewProject => "basic.new_project",
            Self::OpenProject => "basic.open_project",
            Self::SaveProject => "basic.save_project",
            Self::ImportMedia => "basic.import_media",
            Self::Export => "basic.export",
            Self::Preferences => "basic.preferences",
            Self::ShortcutsDialog => "basic.shortcuts_dialog",
            Self::TogglePanels => "other.toggle_panels",
            Self::InspectorVisibility => "other.inspector_visibility",
            Self::TimelineFocus => "other.timeline_focus",
            Self::PreviewFocus => "other.preview_focus",
            Self::WorkspaceNavigation => "other.workspace_navigation",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::SelectionTool => "Selection Tool",
            Self::SplitAtPlayhead => "Split at Playhead",
            Self::SplitAll => "Split All",
            Self::DeselectAll => "Deselect All",
            Self::DeleteSelected => "Delete Selected",
            Self::DeleteLeft => "Delete Left",
            Self::DeleteRight => "Delete Right",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::SnappingToggle => "Snapping Toggle",
            Self::LinkedSelection => "Linked Selection",
            Self::ZoomIn => "Zoom In",
            Self::ZoomOut => "Zoom Out",
            Self::FitTimeline => "Fit Timeline",
            Self::AddTrack => "Add Track",
            Self::FreezeFrame => "Freeze Frame",
            Self::DetachAudio => "Detach Audio",
            Self::AddKeyframe => "Add Keyframe",
            Self::PlayPause => "Play / Pause",
            Self::PreviousFrame => "Previous Frame",
            Self::NextFrame => "Next Frame",
            Self::SeekBackward => "Seek Backward",
            Self::SeekForward => "Seek Forward",
            Self::JumpToBeginning => "Jump to Beginning",
            Self::JumpToEnd => "Jump to End",
            Self::FullscreenPreview => "Fullscreen Preview",
            Self::NewProject => "New Project",
            Self::OpenProject => "Open Project",
            Self::SaveProject => "Save Project",
            Self::ImportMedia => "Import Media",
            Self::Export => "Export",
            Self::Preferences => "Preferences",
            Self::ShortcutsDialog => "Shortcuts Dialog",
            Self::TogglePanels => "Toggle Panels",
            Self::InspectorVisibility => "Inspector Visibility",
            Self::TimelineFocus => "Timeline Focus",
            Self::PreviewFocus => "Preview Focus",
            Self::WorkspaceNavigation => "Workspace Navigation",
        }
    }

    pub fn from_stable_id(raw: &str) -> Option<Self> {
        COMMAND_SPECS
            .iter()
            .find(|spec| spec.id.id() == raw)
            .map(|spec| spec.id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandCategory {
    Timeline,
    Player,
    Basic,
    Other,
}

impl CommandCategory {
    pub const ALL: [Self; 4] = [Self::Timeline, Self::Player, Self::Basic, Self::Other];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Timeline => "Timeline",
            Self::Player => "Player",
            Self::Basic => "Basic",
            Self::Other => "Other",
        }
    }
}

/// Where a command can execute. Contexts are deliberately coarse: Timeline and
/// Player are sub-surfaces of the Editor and overlap for conflict detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandContext {
    Global,
    Homepage,
    Editor,
    Timeline,
    Player,
}

impl CommandContext {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Global => "Global",
            Self::Homepage => "Homepage",
            Self::Editor => "Editor",
            Self::Timeline => "Timeline",
            Self::Player => "Player",
        }
    }

    pub fn active_in_editor(self, editor_open: bool) -> bool {
        match self {
            Self::Global => true,
            Self::Homepage => !editor_open,
            Self::Editor | Self::Timeline | Self::Player => editor_open,
        }
    }
}

/// Two commands can share a binding only if their contexts cannot be active at
/// the same time. Editor sub-surfaces all overlap because they are on screen
/// together.
pub fn contexts_overlap(left: CommandContext, right: CommandContext) -> bool {
    match (left, right) {
        (CommandContext::Global, _) | (_, CommandContext::Global) => true,
        (CommandContext::Homepage, CommandContext::Homepage) => true,
        (CommandContext::Homepage, _) | (_, CommandContext::Homepage) => false,
        _ => true,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStatus {
    Implemented,
    Planned,
}

impl CommandStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Implemented => "Available",
            Self::Planned => "Unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CommandSpec {
    pub id: CommandId,
    pub context: CommandContext,
    pub default_binding: Option<ShortcutBinding>,
    pub status: CommandStatus,
    pub requires_selection: bool,
    pub requires_timeline: bool,
}

const fn binding(key: Key, ctrl: bool, shift: bool, alt: bool) -> Option<ShortcutBinding> {
    Some(ShortcutBinding::new(key, ctrl, shift, alt))
}

pub const COMMAND_SPECS: [CommandSpec; 38] = [
    CommandSpec {
        id: CommandId::SelectionTool,
        context: CommandContext::Timeline,
        default_binding: binding(Key::V, false, false, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::SplitAtPlayhead,
        context: CommandContext::Timeline,
        default_binding: binding(Key::S, false, false, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::SplitAll,
        context: CommandContext::Timeline,
        default_binding: binding(Key::K, true, true, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::DeselectAll,
        context: CommandContext::Timeline,
        default_binding: binding(Key::A, true, true, false),
        status: CommandStatus::Implemented,
        requires_selection: true,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::DeleteSelected,
        context: CommandContext::Timeline,
        default_binding: binding(Key::Delete, false, false, false),
        status: CommandStatus::Planned,
        requires_selection: true,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::DeleteLeft,
        context: CommandContext::Timeline,
        default_binding: binding(Key::Q, false, false, false),
        status: CommandStatus::Planned,
        requires_selection: true,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::DeleteRight,
        context: CommandContext::Timeline,
        default_binding: binding(Key::W, false, false, false),
        status: CommandStatus::Planned,
        requires_selection: true,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::Undo,
        context: CommandContext::Editor,
        default_binding: binding(Key::Z, true, false, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::Redo,
        context: CommandContext::Editor,
        default_binding: binding(Key::Z, true, true, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::SnappingToggle,
        context: CommandContext::Timeline,
        default_binding: binding(Key::S, false, true, false),
        status: CommandStatus::Implemented,
        requires_selection: false,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::LinkedSelection,
        context: CommandContext::Timeline,
        default_binding: binding(Key::L, true, false, false),
        status: CommandStatus::Implemented,
        requires_selection: false,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::ZoomIn,
        context: CommandContext::Timeline,
        default_binding: binding(Key::Equals, true, false, false),
        status: CommandStatus::Implemented,
        requires_selection: false,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::ZoomOut,
        context: CommandContext::Timeline,
        default_binding: binding(Key::Minus, true, false, false),
        status: CommandStatus::Implemented,
        requires_selection: false,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::FitTimeline,
        context: CommandContext::Timeline,
        default_binding: binding(Key::Num0, true, false, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::AddTrack,
        context: CommandContext::Timeline,
        default_binding: None,
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::FreezeFrame,
        context: CommandContext::Timeline,
        default_binding: None,
        status: CommandStatus::Planned,
        requires_selection: true,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::DetachAudio,
        context: CommandContext::Timeline,
        default_binding: None,
        status: CommandStatus::Planned,
        requires_selection: true,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::AddKeyframe,
        context: CommandContext::Timeline,
        default_binding: None,
        status: CommandStatus::Planned,
        requires_selection: true,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::PlayPause,
        context: CommandContext::Player,
        default_binding: binding(Key::Space, false, false, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::PreviousFrame,
        context: CommandContext::Player,
        default_binding: binding(Key::ArrowLeft, false, false, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::NextFrame,
        context: CommandContext::Player,
        default_binding: binding(Key::ArrowRight, false, false, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::SeekBackward,
        context: CommandContext::Player,
        default_binding: binding(Key::ArrowLeft, false, true, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::SeekForward,
        context: CommandContext::Player,
        default_binding: binding(Key::ArrowRight, false, true, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::JumpToBeginning,
        context: CommandContext::Player,
        default_binding: binding(Key::Home, false, false, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::JumpToEnd,
        context: CommandContext::Player,
        default_binding: binding(Key::End, false, false, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::FullscreenPreview,
        context: CommandContext::Player,
        default_binding: binding(Key::F11, false, false, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::NewProject,
        context: CommandContext::Homepage,
        default_binding: binding(Key::N, true, false, false),
        status: CommandStatus::Implemented,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::OpenProject,
        context: CommandContext::Editor,
        default_binding: binding(Key::O, true, false, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::SaveProject,
        context: CommandContext::Editor,
        default_binding: binding(Key::S, true, false, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::ImportMedia,
        context: CommandContext::Editor,
        default_binding: binding(Key::I, true, false, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::Export,
        context: CommandContext::Editor,
        default_binding: binding(Key::E, true, false, false),
        status: CommandStatus::Implemented,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::Preferences,
        context: CommandContext::Homepage,
        default_binding: binding(Key::Comma, true, false, false),
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::ShortcutsDialog,
        context: CommandContext::Global,
        default_binding: binding(Key::Slash, true, false, false),
        status: CommandStatus::Implemented,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::TogglePanels,
        context: CommandContext::Editor,
        default_binding: None,
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::InspectorVisibility,
        context: CommandContext::Editor,
        default_binding: None,
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::TimelineFocus,
        context: CommandContext::Timeline,
        default_binding: None,
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: true,
    },
    CommandSpec {
        id: CommandId::PreviewFocus,
        context: CommandContext::Player,
        default_binding: None,
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
    CommandSpec {
        id: CommandId::WorkspaceNavigation,
        context: CommandContext::Editor,
        default_binding: None,
        status: CommandStatus::Planned,
        requires_selection: false,
        requires_timeline: false,
    },
];

pub const TIMELINE_COMMANDS: [CommandId; 18] = [
    CommandId::SelectionTool,
    CommandId::SplitAtPlayhead,
    CommandId::SplitAll,
    CommandId::DeselectAll,
    CommandId::DeleteSelected,
    CommandId::DeleteLeft,
    CommandId::DeleteRight,
    CommandId::Undo,
    CommandId::Redo,
    CommandId::SnappingToggle,
    CommandId::LinkedSelection,
    CommandId::ZoomIn,
    CommandId::ZoomOut,
    CommandId::FitTimeline,
    CommandId::AddTrack,
    CommandId::FreezeFrame,
    CommandId::DetachAudio,
    CommandId::AddKeyframe,
];

pub const PLAYER_COMMANDS: [CommandId; 8] = [
    CommandId::PlayPause,
    CommandId::PreviousFrame,
    CommandId::NextFrame,
    CommandId::SeekBackward,
    CommandId::SeekForward,
    CommandId::JumpToBeginning,
    CommandId::JumpToEnd,
    CommandId::FullscreenPreview,
];

pub const BASIC_COMMANDS: [CommandId; 9] = [
    CommandId::NewProject,
    CommandId::OpenProject,
    CommandId::SaveProject,
    CommandId::ImportMedia,
    CommandId::Export,
    CommandId::Undo,
    CommandId::Redo,
    CommandId::Preferences,
    CommandId::ShortcutsDialog,
];

pub const OTHER_COMMANDS: [CommandId; 5] = [
    CommandId::TogglePanels,
    CommandId::InspectorVisibility,
    CommandId::TimelineFocus,
    CommandId::PreviewFocus,
    CommandId::WorkspaceNavigation,
];

pub fn commands_for(category: CommandCategory) -> &'static [CommandId] {
    match category {
        CommandCategory::Timeline => &TIMELINE_COMMANDS,
        CommandCategory::Player => &PLAYER_COMMANDS,
        CommandCategory::Basic => &BASIC_COMMANDS,
        CommandCategory::Other => &OTHER_COMMANDS,
    }
}

pub fn spec(id: CommandId) -> Option<&'static CommandSpec> {
    COMMAND_SPECS.iter().find(|spec| spec.id == id)
}

pub fn default_bindings() -> BTreeMap<CommandId, Option<ShortcutBinding>> {
    COMMAND_SPECS
        .iter()
        .map(|spec| (spec.id, spec.default_binding))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_ids_and_labels_are_unique_and_stable() {
        let mut ids = std::collections::BTreeSet::new();
        let mut labels = std::collections::BTreeSet::new();
        for spec in COMMAND_SPECS {
            assert!(ids.insert(spec.id.id()), "duplicate id {}", spec.id.id());
            assert!(
                labels.insert(spec.id.label()),
                "duplicate label {}",
                spec.id.label()
            );
            assert_eq!(CommandId::from_stable_id(spec.id.id()), Some(spec.id));
        }
    }

    #[test]
    fn every_category_is_populated_without_duplicate_rows() {
        for category in CommandCategory::ALL {
            let commands = commands_for(category);
            assert!(!commands.is_empty(), "{} is empty", category.label());
            let unique: std::collections::BTreeSet<_> = commands.iter().collect();
            assert_eq!(
                unique.len(),
                commands.len(),
                "{} duplicates",
                category.label()
            );
            for command in commands {
                assert!(spec(*command).is_some(), "missing spec for {command:?}");
            }
        }
    }

    #[test]
    fn default_bindings_do_not_conflict_in_overlapping_contexts() {
        let defaults = default_bindings();
        let bound: Vec<_> = defaults
            .iter()
            .filter_map(|(id, binding)| binding.map(|binding| (*id, binding)))
            .collect();
        for (index, (left_id, left_binding)) in bound.iter().enumerate() {
            let left = spec(*left_id).expect("registered command");
            for (right_id, right_binding) in bound.iter().skip(index + 1) {
                let right = spec(*right_id).expect("registered command");
                if contexts_overlap(left.context, right.context) {
                    assert_ne!(
                        left_binding,
                        right_binding,
                        "{} and {} share {}",
                        left_id.label(),
                        right_id.label(),
                        left_binding.label()
                    );
                }
            }
        }
    }

    #[test]
    fn context_overlap_rules_match_screen_exclusivity() {
        use CommandContext::*;
        assert!(contexts_overlap(Global, Homepage));
        assert!(contexts_overlap(Global, Timeline));
        assert!(contexts_overlap(Editor, Timeline));
        assert!(contexts_overlap(Editor, Player));
        assert!(contexts_overlap(Timeline, Player));
        assert!(!contexts_overlap(Homepage, Editor));
        assert!(!contexts_overlap(Homepage, Timeline));
        assert!(!contexts_overlap(Homepage, Player));
    }

    #[test]
    fn only_registered_commands_claim_implementation() {
        let implemented: Vec<_> = COMMAND_SPECS
            .iter()
            .filter(|spec| spec.status == CommandStatus::Implemented)
            .map(|spec| spec.id)
            .collect();
        assert_eq!(
            implemented,
            vec![
                CommandId::DeselectAll,
                CommandId::SnappingToggle,
                CommandId::LinkedSelection,
                CommandId::ZoomIn,
                CommandId::ZoomOut,
                CommandId::NewProject,
                CommandId::Export,
                CommandId::ShortcutsDialog,
            ]
        );
    }
}
