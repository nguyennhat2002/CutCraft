//! Compact application menu for the editor top bar.

use egui::Ui;

use crate::config;
use crate::editor_state::EditorSession;
use crate::shortcuts::{CommandId, ShortcutManager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    BackHome,
    NewProject,
    OpenProject,
    RenameProject,
    DuplicateProject,
    SaveRecoveryCopy,
}

/// Render the editor menu. Returns the requested navigation or file action.
pub fn show_menus(
    ui: &mut Ui,
    session: &mut EditorSession,
    shortcuts: &ShortcutManager,
) -> Option<MenuAction> {
    let mut action = None;

    ui.scope(|ui| {
        egui::menu::menu_style(ui.style_mut());

        let _ = ui.menu_button("Menu", |ui| {
            ui.set_min_width(250.0);
            if ui.button("Back to Home").clicked() {
                action = Some(MenuAction::BackHome);
            }
            ui.separator();
            if ui
                .button(with_shortcut(
                    "New Native Project",
                    shortcuts,
                    CommandId::NewProject,
                ))
                .clicked()
            {
                action = Some(MenuAction::NewProject);
            }
            if ui
                .button(with_shortcut(
                    "Open CutCraft Project\u{2026}",
                    shortcuts,
                    CommandId::OpenProject,
                ))
                .clicked()
            {
                action = Some(MenuAction::OpenProject);
            }
            disabled_menu_item(
                ui,
                &with_shortcut("Import Media\u{2026}", shortcuts, CommandId::ImportMedia),
                config::UNIMPLEMENTED_HINT,
            );
            if session.native_document().is_some() {
                if ui.button("Rename Project\u{2026}").clicked() {
                    action = Some(MenuAction::RenameProject);
                }
                if ui.button("Duplicate Project\u{2026}").clicked() {
                    action = Some(MenuAction::DuplicateProject);
                }
                if session
                    .native_document()
                    .is_some_and(|document| document.has_conflict())
                    && ui.button("Save Recovery Copy\u{2026}").clicked()
                {
                    action = Some(MenuAction::SaveRecoveryCopy);
                }
            } else {
                disabled_menu_item(
                    ui,
                    "Rename Project\u{2026}",
                    "This session has no managed project.",
                );
                disabled_menu_item(
                    ui,
                    "Duplicate Project\u{2026}",
                    "This session has no managed project.",
                );
            }
            ui.separator();
            ui.checkbox(&mut session.tools_visible, "Library");
            ui.checkbox(&mut session.inspector_visible, "Inspector");
            ui.checkbox(&mut session.timeline_visible, "Timeline");
            if ui.button("Reset Layout").clicked() {
                session.reset_layout();
            }
            ui.separator();
            if session.native_document().is_some() {
                let undo = ui.add_enabled(
                    session.can_undo_native(),
                    egui::Button::new(with_shortcut("Undo", shortcuts, CommandId::Undo)),
                );
                if undo.clicked() {
                    let _ = session.undo_native();
                }
                let redo = ui.add_enabled(
                    session.can_redo_native(),
                    egui::Button::new(with_shortcut("Redo", shortcuts, CommandId::Redo)),
                );
                if redo.clicked() {
                    let _ = session.redo_native();
                }
            } else {
                disabled_menu_item(
                    ui,
                    &with_shortcut("Undo", shortcuts, CommandId::Undo),
                    config::UNIMPLEMENTED_HINT,
                );
                disabled_menu_item(
                    ui,
                    &with_shortcut("Redo", shortcuts, CommandId::Redo),
                    config::UNIMPLEMENTED_HINT,
                );
            }
            ui.separator();
            if ui.button("Exit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    });

    action
}

fn with_shortcut(label: &str, shortcuts: &ShortcutManager, command: CommandId) -> String {
    match shortcuts.shortcut_hint(command) {
        Some(shortcut) => format!("{label}    {shortcut}"),
        None => label.to_owned(),
    }
}

fn disabled_menu_item(ui: &mut Ui, label: &str, hint: &str) {
    let response = ui.add_enabled(false, egui::Button::new(label));
    let _ = response.on_disabled_hover_text(hint);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_labels_include_live_shortcut_bindings() {
        let manager = ShortcutManager::default();
        assert_eq!(
            with_shortcut("Undo", &manager, CommandId::Undo),
            "Undo    Ctrl+Z"
        );
        assert_eq!(
            with_shortcut("New Native Project", &manager, CommandId::NewProject),
            "New Native Project    Ctrl+N"
        );
        assert_eq!(
            with_shortcut("Add Track", &manager, CommandId::AddTrack),
            "Add Track",
            "unassigned commands keep a clean label"
        );
    }
}
