//! Runtime gating and single-dispatch resolution for shortcuts.

use egui::{Context, Event};

use super::binding::ShortcutBinding;
use super::manager::ShortcutManager;
use super::registry::{CommandId, CommandSpec, CommandStatus, COMMAND_SPECS};

/// Runtime facts that decide whether a registered command may run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShortcutRuntime {
    pub editor_open: bool,
    pub timeline_visible: bool,
    pub has_selection: bool,
    pub text_focus: bool,
    pub modal_open: bool,
    pub capturing: bool,
}

impl ShortcutRuntime {
    fn blocks_dispatch(self) -> bool {
        self.text_focus || self.modal_open || self.capturing
    }
}

impl CommandSpec {
    pub fn is_available(self, runtime: ShortcutRuntime) -> bool {
        self.status == CommandStatus::Implemented
            && self.context.active_in_editor(runtime.editor_open)
            && (!self.requires_timeline || runtime.timeline_visible)
            && (!self.requires_selection || runtime.has_selection)
    }
}

impl ShortcutManager {
    /// Consume at most one matching key event and return its command.
    ///
    /// Binding iteration follows registry order and events are removed, so a
    /// single key event can never execute two handlers. Commands that are
    /// planned, unavailable, or missing prerequisites are skipped entirely.
    pub fn dispatch(&self, context: &Context, runtime: ShortcutRuntime) -> Option<CommandId> {
        if runtime.blocks_dispatch() {
            return None;
        }
        let bindings = self.bindings();
        for spec in COMMAND_SPECS {
            if !spec.is_available(runtime) {
                continue;
            }
            let Some(binding) = bindings.get(&spec.id).copied().flatten() else {
                continue;
            };
            let consumed =
                context.input_mut(|input| take_matching_event(&mut input.events, binding));
            if consumed {
                return Some(spec.id);
            }
        }
        None
    }
}

/// Remove the first exact press of `binding` from the event queue.
pub(crate) fn take_matching_event(events: &mut Vec<Event>, binding: ShortcutBinding) -> bool {
    let Some(index) = events.iter().position(|event| match event {
        Event::Key {
            key,
            pressed: true,
            repeat: false,
            modifiers,
            ..
        } => binding.matches(*key, modifiers),
        _ => false,
    }) else {
        return false;
    };
    events.remove(index);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcuts::binding::ShortcutBinding;
    use crate::shortcuts::manager::ShortcutDraft;
    use crate::shortcuts::profile::ShortcutProfile;
    use crate::shortcuts::registry::CommandCategory;
    use egui::{Key, Modifiers, Pos2, Rect, Vec2};

    fn runtime() -> ShortcutRuntime {
        ShortcutRuntime {
            editor_open: true,
            timeline_visible: true,
            has_selection: true,
            ..Default::default()
        }
    }

    fn key_event(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn dispatch_event(
        manager: &ShortcutManager,
        runtime: ShortcutRuntime,
        event: Event,
    ) -> Option<CommandId> {
        let context = Context::default();
        let mut result = None;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events: vec![event],
            ..Default::default()
        };
        let output = context.run_ui(input, |_ui| {
            result = manager.dispatch(&context, runtime);
        });
        output.drop_without_applying_deltas();
        result
    }

    #[test]
    fn default_binding_dispatches_implemented_command() {
        let manager = ShortcutManager::default();
        assert_eq!(
            dispatch_event(
                &manager,
                runtime(),
                key_event(
                    Key::Equals,
                    Modifiers {
                        ctrl: true,
                        ..Default::default()
                    }
                )
            ),
            Some(CommandId::ZoomIn)
        );
    }

    #[test]
    fn custom_binding_is_active_immediately_after_save() {
        let mut manager = ShortcutManager::default();
        let mut draft = ShortcutDraft::new(&manager);
        draft.set_binding(
            CommandId::ZoomIn,
            ShortcutBinding::new(Key::Z, true, false, true),
        );
        manager.save_draft(&draft).expect("save custom binding");
        assert_eq!(manager.active_profile(), ShortcutProfile::Custom);
        assert_eq!(
            dispatch_event(
                &manager,
                runtime(),
                key_event(
                    Key::Z,
                    Modifiers {
                        ctrl: true,
                        alt: true,
                        ..Default::default()
                    }
                )
            ),
            Some(CommandId::ZoomIn)
        );
        assert_eq!(
            dispatch_event(
                &manager,
                runtime(),
                key_event(
                    Key::Equals,
                    Modifiers {
                        ctrl: true,
                        ..Default::default()
                    }
                )
            ),
            None,
            "the old binding must no longer run"
        );
    }

    #[test]
    fn text_focus_modal_and_capture_block_all_shortcuts() {
        let manager = ShortcutManager::default();
        for runtime in [
            ShortcutRuntime {
                text_focus: true,
                ..runtime()
            },
            ShortcutRuntime {
                modal_open: true,
                ..runtime()
            },
            ShortcutRuntime {
                capturing: true,
                ..runtime()
            },
        ] {
            assert_eq!(
                dispatch_event(
                    &manager,
                    runtime,
                    key_event(
                        Key::Equals,
                        Modifiers {
                            ctrl: true,
                            ..Default::default()
                        }
                    )
                ),
                None
            );
        }
    }

    #[test]
    fn planned_commands_never_dispatch() {
        let manager = ShortcutManager::default();
        assert_eq!(
            dispatch_event(&manager, runtime(), key_event(Key::S, Modifiers::NONE)),
            None,
            "Split at Playhead is planned"
        );
        assert_eq!(
            dispatch_event(&manager, runtime(), key_event(Key::Space, Modifiers::NONE)),
            None,
            "Play/Pause is planned"
        );
    }

    #[test]
    fn selection_and_timeline_availability_are_enforced() {
        let manager = ShortcutManager::default();
        assert_eq!(
            dispatch_event(
                &manager,
                ShortcutRuntime {
                    has_selection: false,
                    ..runtime()
                },
                key_event(
                    Key::A,
                    Modifiers {
                        ctrl: true,
                        shift: true,
                        ..Default::default()
                    }
                )
            ),
            None,
            "Deselect All requires a selection"
        );
        assert_eq!(
            dispatch_event(
                &manager,
                runtime(),
                key_event(
                    Key::A,
                    Modifiers {
                        ctrl: true,
                        shift: true,
                        ..Default::default()
                    }
                )
            ),
            Some(CommandId::DeselectAll)
        );
        assert_eq!(
            dispatch_event(
                &manager,
                ShortcutRuntime {
                    timeline_visible: false,
                    ..runtime()
                },
                key_event(
                    Key::Equals,
                    Modifiers {
                        ctrl: true,
                        ..Default::default()
                    }
                )
            ),
            None,
            "timeline shortcuts require a visible timeline"
        );
    }

    #[test]
    fn global_shortcut_works_on_the_homepage() {
        let manager = ShortcutManager::default();
        assert_eq!(
            dispatch_event(
                &manager,
                ShortcutRuntime::default(),
                key_event(
                    Key::Slash,
                    Modifiers {
                        ctrl: true,
                        ..Default::default()
                    }
                )
            ),
            Some(CommandId::ShortcutsDialog)
        );
    }

    #[test]
    fn duplicate_bindings_in_nonoverlapping_contexts_resolve_by_screen() {
        let mut manager = ShortcutManager::default();
        let mut draft = ShortcutDraft::new(&manager);
        let shared = ShortcutBinding::new(Key::E, true, false, false);
        draft.set_binding(CommandId::NewProject, shared);
        manager
            .save_draft(&draft)
            .expect("nonoverlapping duplicate");
        assert_eq!(
            dispatch_event(
                &manager,
                runtime(),
                key_event(
                    Key::E,
                    Modifiers {
                        ctrl: true,
                        ..Default::default()
                    }
                )
            ),
            Some(CommandId::Export)
        );
        assert_eq!(
            dispatch_event(
                &manager,
                ShortcutRuntime::default(),
                key_event(
                    Key::E,
                    Modifiers {
                        ctrl: true,
                        ..Default::default()
                    }
                )
            ),
            Some(CommandId::NewProject)
        );
    }

    #[test]
    fn one_event_is_consumed_by_at_most_one_command() {
        let manager = ShortcutManager::default();
        let context = Context::default();
        let mut results = Vec::new();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events: vec![key_event(
                Key::Slash,
                Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
            )],
            ..Default::default()
        };
        let output = context.run_ui(input, |_ui| {
            results.push(
                manager
                    .dispatch(&context, ShortcutRuntime::default())
                    .map(|id| id.label()),
            );
            results.push(
                manager
                    .dispatch(&context, ShortcutRuntime::default())
                    .map(|id| id.label()),
            );
        });
        output.drop_without_applying_deltas();
        assert_eq!(results, vec![Some("Shortcuts Dialog"), None]);
    }

    #[test]
    fn categories_still_map_to_registered_commands() {
        for category in CommandCategory::ALL {
            for command in super::super::registry::commands_for(category) {
                assert!(super::super::registry::spec(*command).is_some());
            }
        }
    }
}
