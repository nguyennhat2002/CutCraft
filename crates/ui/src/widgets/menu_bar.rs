//! File, Edit, and View menus for the editor top bar.

use egui::Ui;

use crate::config;
use crate::editor_state::EditorSession;

/// Render the editor menu buttons. Returns `true` when Back to Home was chosen.
pub fn show_menus(ui: &mut Ui, session: &mut EditorSession) -> bool {
    let mut back_home = false;

    ui.scope(|ui| {
        egui::menu::menu_style(ui.style_mut());

        let _ = ui.menu_button("File", |ui| {
            ui.set_min_width(232.0);
            if ui.button("Back to Home").clicked() {
                back_home = true;
            }
            ui.separator();
            disabled_menu_item(ui, "New Project", config::UNIMPLEMENTED_HINT);
            disabled_menu_item(ui, "Open Project\u{2026}", config::UNIMPLEMENTED_HINT);
            disabled_menu_item(ui, "Import Media\u{2026}", config::UNIMPLEMENTED_HINT);
            ui.separator();
            disabled_menu_item(ui, "Save", config::UNIMPLEMENTED_HINT);
            disabled_menu_item(ui, "Save As\u{2026}", config::UNIMPLEMENTED_HINT);
            ui.separator();
            if ui.button("Exit").clicked() {
                ui.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });

        let _ = ui.menu_button("Edit", |ui| {
            ui.set_min_width(232.0);
            disabled_menu_item(ui, "Undo", config::UNIMPLEMENTED_HINT);
            disabled_menu_item(ui, "Redo", config::UNIMPLEMENTED_HINT);
            ui.separator();
            disabled_menu_item(ui, "Cut", config::UNIMPLEMENTED_HINT);
            disabled_menu_item(ui, "Copy", config::UNIMPLEMENTED_HINT);
            disabled_menu_item(ui, "Paste", config::UNIMPLEMENTED_HINT);
            disabled_menu_item(ui, "Delete", config::UNIMPLEMENTED_HINT);
        });

        let _ = ui.menu_button("View", |ui| {
            ui.set_min_width(210.0);
            ui.checkbox(&mut session.tools_visible, "Library");
            ui.checkbox(&mut session.inspector_visible, "Inspector");
            ui.checkbox(&mut session.timeline_visible, "Timeline");
            ui.separator();
            if ui.button("Reset Layout").clicked() {
                session.reset_layout();
            }
        });
    });

    back_home
}

fn disabled_menu_item(ui: &mut Ui, label: &str, hint: &str) {
    let response = ui.add_enabled(false, egui::Button::new(label));
    let _ = response.on_disabled_hover_text(hint);
}
