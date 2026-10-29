//! Compact application menu for the editor top bar.

use egui::Ui;

use crate::config;
use crate::editor_state::EditorSession;
use crate::shortcuts::{CommandId, ShortcutManager};

/// Render the editor menu. Returns `true` when Back to Home was chosen.
pub fn show_menus(ui: &mut Ui, session: &mut EditorSession, shortcuts: &ShortcutManager) -> bool {
    let mut back_home = false;

    ui.scope(|ui| {
        egui::menu::menu_style(ui.style_mut());

        let _ = ui.menu_button("Menu", |ui| {
            ui.set_min_width(240.0);
            if ui.button("Back to Home").clicked() {
                back_home = true;
            }
            ui.separator();
            disabled_menu_item(
                ui,
                &with_shortcut("New Project", shortcuts, CommandId::NewProject),
                config::UNIMPLEMENTED_HINT,
            );
            disabled_menu_item(
                ui,
                &with_shortcut("Open Project\u{2026}", shortcuts, CommandId::OpenProject),
                config::UNIMPLEMENTED_HINT,
            );
            disabled_menu_item(
                ui,
                &with_shortcut("Import Media\u{2026}", shortcuts, CommandId::ImportMedia),
                config::UNIMPLEMENTED_HINT,
            );
            disabled_menu_item(
                ui,
                &with_shortcut("Save", shortcuts, CommandId::SaveProject),
                config::UNIMPLEMENTED_HINT,
            );
            ui.separator();
            ui.checkbox(&mut session.tools_visible, "Library");
            ui.checkbox(&mut session.inspector_visible, "Inspector");
            ui.checkbox(&mut session.timeline_visible, "Timeline");
            if ui.button("Reset Layout").clicked() {
                session.reset_layout();
            }
            ui.separator();
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
            ui.separator();
            if ui.button("Exit").clicked() {
                ui.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    });

    back_home
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
            with_shortcut("New Project", &manager, CommandId::NewProject),
            "New Project    Ctrl+N"
        );
        assert_eq!(
            with_shortcut("Add Track", &manager, CommandId::AddTrack),
            "Add Track",
            "unassigned commands keep a clean label"
        );
    }
}
