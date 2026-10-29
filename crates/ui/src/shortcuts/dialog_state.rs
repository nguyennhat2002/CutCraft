//! Pending dialog state for the Shortcuts editor, independent of egui widgets.

use egui::Event;

use super::binding::{capture_outcome, CaptureOutcome, ShortcutBinding};
use super::manager::{ShortcutDraft, ShortcutManager};
use super::profile::ShortcutProfile;
use super::registry::{commands_for, CommandCategory, CommandId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictPrompt {
    pub command: CommandId,
    pub binding: ShortcutBinding,
    pub conflicts: Vec<CommandId>,
}

#[derive(Debug, Clone)]
pub struct ShortcutsDialogState {
    pub category: CommandCategory,
    pub draft: ShortcutDraft,
    pub capturing: Option<CommandId>,
    pub conflict: Option<ConflictPrompt>,
    pub reset_requested: bool,
    pub capture_warning: Option<String>,
    pub save_error: Option<String>,
}

impl ShortcutsDialogState {
    pub fn new(manager: &ShortcutManager) -> Self {
        Self {
            category: CommandCategory::Timeline,
            draft: ShortcutDraft::new(manager),
            capturing: None,
            conflict: None,
            reset_requested: false,
            capture_warning: None,
            save_error: None,
        }
    }

    pub fn commands(&self) -> &'static [CommandId] {
        commands_for(self.category)
    }

    pub fn profile(&self) -> ShortcutProfile {
        self.draft.profile
    }

    pub fn set_profile(&mut self, profile: ShortcutProfile) {
        self.draft.profile = profile;
        self.capturing = None;
        self.conflict = None;
        self.capture_warning = None;
    }

    pub fn start_capture(&mut self, command: CommandId) {
        self.capturing = Some(command);
        self.conflict = None;
        self.capture_warning = None;
        self.save_error = None;
    }

    pub fn cancel_capture(&mut self) {
        self.capturing = None;
        self.capture_warning = None;
    }

    pub fn clear_binding(&mut self, command: CommandId) {
        self.capturing = None;
        self.conflict = None;
        self.draft.clear_binding(command);
    }

    /// Feed one event while `capturing` is active. Returns true when the event
    /// was relevant to capture and should be swallowed by the dialog.
    pub fn capture_event(&mut self, event: &Event) -> bool {
        let Some(command) = self.capturing else {
            return false;
        };
        match capture_outcome(event) {
            CaptureOutcome::Ignored => false,
            CaptureOutcome::Cancelled => {
                self.cancel_capture();
                true
            }
            CaptureOutcome::Reserved => {
                self.capture_warning =
                    Some("That key combination is reserved by Windows.".to_owned());
                true
            }
            CaptureOutcome::Unsupported => {
                self.capture_warning = Some(
                    "Use letters, numbers, arrows, function keys or navigation keys.".to_owned(),
                );
                true
            }
            CaptureOutcome::Captured(binding) => {
                let conflicts = self.draft.conflicts_for(command, binding);
                self.capturing = None;
                self.capture_warning = None;
                if conflicts.is_empty() {
                    self.draft.set_binding(command, binding);
                } else {
                    self.conflict = Some(ConflictPrompt {
                        command,
                        binding,
                        conflicts,
                    });
                }
                true
            }
        }
    }

    pub fn reassign_conflict(&mut self) {
        if let Some(prompt) = self.conflict.take() {
            self.draft
                .reassign(prompt.command, prompt.binding, &prompt.conflicts);
            self.capture_warning = None;
        }
    }

    pub fn cancel_conflict(&mut self) {
        self.conflict = None;
    }

    pub fn request_reset(&mut self) {
        self.reset_requested = true;
        self.capturing = None;
        self.conflict = None;
    }

    pub fn cancel_reset(&mut self) {
        self.reset_requested = false;
    }

    pub fn confirm_reset(&mut self) {
        if self.reset_requested {
            self.draft.reset_to_defaults();
            self.reset_requested = false;
            self.save_error = None;
        }
    }

    pub fn is_dirty(&self, manager: &ShortcutManager) -> bool {
        self.draft.is_custom_dirty(manager)
    }

    pub fn save(&mut self, manager: &mut ShortcutManager) -> Result<(), String> {
        self.capturing = None;
        self.conflict = None;
        self.reset_requested = false;
        self.save_error = None;
        manager.save_draft(&self.draft).map_err(|error| {
            let message = error.to_string();
            self.save_error = Some(message.clone());
            message
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcuts::registry::spec;
    use egui::{Key, Modifiers};

    fn key_event(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn state() -> ShortcutsDialogState {
        ShortcutsDialogState::new(&ShortcutManager::default())
    }

    #[test]
    fn capture_starts_and_escape_cancels_without_dirtying() {
        let manager = ShortcutManager::default();
        let mut dialog = ShortcutsDialogState::new(&manager);
        dialog.start_capture(CommandId::ZoomIn);
        assert_eq!(dialog.capturing, Some(CommandId::ZoomIn));
        assert!(dialog.capture_event(&key_event(Key::Escape, Modifiers::NONE)));
        assert_eq!(dialog.capturing, None);
        assert!(!dialog.is_dirty(&manager));
    }

    #[test]
    fn captured_nonconflicting_binding_updates_pending_only() {
        let manager = ShortcutManager::default();
        let mut dialog = ShortcutsDialogState::new(&manager);
        let binding = ShortcutBinding::new(Key::Z, true, false, true);
        dialog.start_capture(CommandId::ZoomIn);
        assert!(dialog.capture_event(&key_event(
            Key::Z,
            Modifiers {
                ctrl: true,
                alt: true,
                ..Default::default()
            }
        )));
        assert_eq!(dialog.draft.binding(CommandId::ZoomIn), Some(binding));
        assert!(dialog.is_dirty(&manager));
        assert_eq!(
            manager.binding(CommandId::ZoomIn),
            spec(CommandId::ZoomIn).and_then(|spec| spec.default_binding)
        );
    }

    #[test]
    fn conflicting_capture_requires_explicit_reassignment() {
        let manager = ShortcutManager::default();
        let mut dialog = ShortcutsDialogState::new(&manager);
        dialog.start_capture(CommandId::ZoomIn);
        assert!(dialog.capture_event(&key_event(
            Key::E,
            Modifiers {
                ctrl: true,
                ..Default::default()
            }
        )));
        let prompt = dialog.conflict.clone().expect("conflict prompt");
        assert_eq!(prompt.command, CommandId::ZoomIn);
        assert!(prompt.conflicts.contains(&CommandId::Export));
        assert_eq!(
            dialog.draft.binding(CommandId::ZoomIn),
            spec(CommandId::ZoomIn).and_then(|spec| spec.default_binding),
            "the requested binding must not be applied before confirmation"
        );

        dialog.reassign_conflict();
        assert!(dialog.conflict.is_none());
        assert_eq!(
            dialog.draft.binding(CommandId::ZoomIn),
            Some(ShortcutBinding::new(Key::E, true, false, false))
        );
        assert_eq!(dialog.draft.binding(CommandId::Export), None);
    }

    #[test]
    fn cancel_conflict_keeps_the_previous_binding() {
        let mut dialog = state();
        let previous = dialog.draft.binding(CommandId::ZoomIn);
        dialog.start_capture(CommandId::ZoomIn);
        dialog.capture_event(&key_event(
            Key::E,
            Modifiers {
                ctrl: true,
                ..Default::default()
            },
        ));
        dialog.cancel_conflict();
        assert_eq!(dialog.draft.binding(CommandId::ZoomIn), previous);
    }

    #[test]
    fn clear_binding_switches_to_custom() {
        let manager = ShortcutManager::default();
        let mut dialog = ShortcutsDialogState::new(&manager);
        dialog.clear_binding(CommandId::SnappingToggle);
        assert_eq!(dialog.profile(), ShortcutProfile::Custom);
        assert_eq!(dialog.draft.binding(CommandId::SnappingToggle), None);
    }

    #[test]
    fn reset_requires_confirmation_then_restores_defaults() {
        let mut dialog = state();
        dialog.start_capture(CommandId::ZoomIn);
        dialog.capture_event(&key_event(
            Key::Z,
            Modifiers {
                ctrl: true,
                alt: true,
                ..Default::default()
            },
        ));
        assert!(dialog.draft.binding(CommandId::ZoomIn).is_some());

        dialog.request_reset();
        assert!(dialog.reset_requested);
        assert_ne!(
            dialog.draft.binding(CommandId::ZoomIn),
            spec(CommandId::ZoomIn).and_then(|spec| spec.default_binding)
        );
        dialog.confirm_reset();
        assert!(!dialog.reset_requested);
        assert_eq!(
            dialog.draft.binding(CommandId::ZoomIn),
            spec(CommandId::ZoomIn).and_then(|spec| spec.default_binding)
        );
    }

    #[test]
    fn switching_profile_keeps_pending_custom_bindings_available() {
        let mut dialog = state();
        dialog.clear_binding(CommandId::SnappingToggle);
        assert_eq!(dialog.profile(), ShortcutProfile::Custom);
        dialog.set_profile(ShortcutProfile::Default);
        assert_eq!(
            dialog.draft.binding(CommandId::SnappingToggle),
            spec(CommandId::SnappingToggle).and_then(|spec| spec.default_binding)
        );
        dialog.set_profile(ShortcutProfile::Custom);
        assert_eq!(dialog.draft.binding(CommandId::SnappingToggle), None);
    }

    #[test]
    fn save_and_cancel_semantics_are_explicit() {
        let mut manager = ShortcutManager::default();
        let mut dialog = ShortcutsDialogState::new(&manager);
        dialog.clear_binding(CommandId::ZoomIn);
        assert!(dialog.is_dirty(&manager));
        dialog.save(&mut manager).expect("save");
        assert_eq!(manager.binding(CommandId::ZoomIn), None);

        let mut cancelled = ShortcutsDialogState::new(&manager);
        cancelled.clear_binding(CommandId::ZoomOut);
        assert!(cancelled.is_dirty(&manager));
        drop(cancelled);
        assert!(manager.binding(CommandId::ZoomOut).is_some());
    }

    #[test]
    fn reserved_and_unsupported_keys_warn_without_assigning() {
        let manager = ShortcutManager::default();
        let mut dialog = ShortcutsDialogState::new(&manager);
        dialog.start_capture(CommandId::ZoomIn);
        assert!(dialog.capture_event(&key_event(
            Key::F4,
            Modifiers {
                alt: true,
                ..Default::default()
            }
        )));
        assert!(dialog.capture_warning.is_some());
        assert_eq!(dialog.capturing, Some(CommandId::ZoomIn));
        assert!(!dialog.is_dirty(&manager));

        dialog.start_capture(CommandId::ZoomIn);
        assert!(dialog.capture_event(&key_event(
            Key::Copy,
            Modifiers {
                ctrl: true,
                ..Default::default()
            }
        )));
        assert!(dialog.capture_warning.is_some());
        assert!(!dialog.is_dirty(&manager));
    }
}
