//! Configurable keyboard shortcuts.
//!
//! The registry is the single source of truth for command identity, category,
//! default binding, execution context and availability. Keyboard dispatch and
//! the Shortcuts dialog both consume this module; neither owns a parallel
//! shortcut table. Actual handlers stay in `AppState`, which delegates to the
//! existing session and navigation operations.

mod binding;
mod dialog_state;
mod dispatch;
mod manager;
mod persistence;
mod profile;
mod registry;

pub use binding::{
    capture_outcome, is_supported_key, BindingParseError, CaptureOutcome, ShortcutBinding,
    SUPPORTED_KEYS,
};
pub use dialog_state::{ConflictPrompt, ShortcutsDialogState};
pub use dispatch::ShortcutRuntime;
pub use manager::{ShortcutDraft, ShortcutManager};
pub use persistence::{
    PersistedShortcuts, ShortcutLoadError, ShortcutSaveError, ShortcutStore, FORMAT_VERSION,
};
pub use profile::ShortcutProfile;
pub use registry::{
    commands_for, contexts_overlap, default_bindings, spec, CommandCategory, CommandContext,
    CommandId, CommandSpec, CommandStatus, BASIC_COMMANDS, COMMAND_SPECS, OTHER_COMMANDS,
    PLAYER_COMMANDS, TIMELINE_COMMANDS,
};
