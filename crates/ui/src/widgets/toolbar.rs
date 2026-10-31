//! Editor project bar: Home navigation, project identity, menus, undo/redo
//! placeholders, and the Export entry point.

use egui::{
    pos2, vec2, Align, Align2, CursorIcon, FontId, Frame, Label, Layout, Margin, Panel, Rect,
    Response, RichText, Sense, Stroke, Ui,
};

use crate::config;
use crate::editor_state::EditorSession;
use crate::native_document::AutoSaveStatus;
use crate::shortcuts::{CommandId, ShortcutManager};
use crate::theme::{Palette, Radii};
use crate::widgets::common;
use crate::widgets::common::focus_ring;
use crate::widgets::icons::{self, Icon, IconColors, IconSize};
use crate::widgets::menu_bar;

/// Result of interacting with the editor top bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TopBarAction {
    #[default]
    None,
    Home,
    Export,
    Shortcuts,
    NewProject,
    OpenProject,
    RenameProject,
    DuplicateProject,
    SaveRecoveryCopy,
}

pub fn show(ui: &mut Ui, session: &mut EditorSession, shortcuts: &ShortcutManager) -> TopBarAction {
    let mut action = TopBarAction::None;

    Panel::top("editor_top_bar")
        .exact_size(config::EDITOR_TOP_BAR_HEIGHT)
        .frame(
            Frame::new()
                .fill(Palette::MENU_BG)
                .inner_margin(Margin::symmetric(10, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                if home_button(ui).clicked() {
                    action = TopBarAction::Home;
                }

                toolbar_separator(ui);
                brand_mark(ui);
                toolbar_separator(ui);

                ui.allocate_ui_with_layout(
                    vec2((ui.available_width() * 0.24).clamp(110.0, 190.0), 22.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.add(
                            Label::new(
                                RichText::new(&session.project_name)
                                    .size(12.5)
                                    .strong()
                                    .color(Palette::TEXT_PRIMARY),
                            )
                            .truncate(),
                        )
                        .on_hover_text(&session.project_name);
                    },
                );
                let (save_text, save_color, save_hint) = match session.auto_save_status() {
                    Some(AutoSaveStatus::Saved {
                        last_saved_unix_ms,
                    }) => (
                        "Saved",
                        Palette::SUCCESS,
                        match session.native_document().and_then(|document| document.path()) {
                            Some(path) => format!(
                                "AutoSave completed at {} ms.\n{}",
                                last_saved_unix_ms,
                                path.display()
                            ),
                            None => "Project content is saved.".to_owned(),
                        },
                    ),
                    Some(AutoSaveStatus::Saving) => (
                        "Saving...",
                        Palette::ACCENT_LIGHT,
                        "AutoSave is writing the current project generation.".to_owned(),
                    ),
                    Some(AutoSaveStatus::UnsavedChanges) => (
                        "Unsaved changes",
                        Palette::WARNING,
                        "AutoSave will persist this project after the debounce interval.".to_owned(),
                    ),
                    Some(AutoSaveStatus::Failed(message)) => (
                        "Save failed",
                        Palette::ERROR,
                        format!("AutoSave failed. The project remains dirty.\n{message}"),
                    ),
                    Some(AutoSaveStatus::Conflict(message)) => (
                        "Conflict detected",
                        Palette::ERROR,
                        format!("AutoSave is paused because the project file changed externally.\n{message}"),
                    ),
                    None => (
                        "Not saved",
                        Palette::WARNING,
                        "This session has no managed project document.".to_owned(),
                    ),
                };
                let badge_width = match save_text {
                    "Unsaved changes" => 108.0,
                    "Saving..." | "Save failed" => 88.0,
                    "Conflict detected" => 118.0,
                    _ => 68.0,
                };
                ui.allocate_ui_with_layout(
                    vec2(badge_width, 22.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        let _ = common::badge(ui, save_text, save_color).on_hover_text(save_hint);
                    },
                );

                ui.add_space(4.0);
                if let Some(menu_action) = menu_bar::show_menus(ui, session, shortcuts) {
                    action = match menu_action {
                        menu_bar::MenuAction::BackHome => TopBarAction::Home,
                        menu_bar::MenuAction::NewProject => TopBarAction::NewProject,
                        menu_bar::MenuAction::OpenProject => TopBarAction::OpenProject,
                        menu_bar::MenuAction::RenameProject => TopBarAction::RenameProject,
                        menu_bar::MenuAction::DuplicateProject => {
                            TopBarAction::DuplicateProject
                        }
                        menu_bar::MenuAction::SaveRecoveryCopy => {
                            TopBarAction::SaveRecoveryCopy
                        }
                    };
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let export_shortcut = shortcuts.shortcut_hint(CommandId::Export);
                    let export = common::primary_button(ui, "Export", 70.0);
                    if export.clicked() {
                        action = TopBarAction::Export;
                    }
                    let _ = export.on_hover_text(match export_shortcut {
                        Some(shortcut) => format!("Open export settings ({shortcut})"),
                        None => "Open export settings".to_owned(),
                    });
                    ui.add_space(6.0);
                    if shortcuts_button(ui, shortcuts.shortcut_hint(CommandId::ShortcutsDialog))
                        .clicked()
                    {
                        action = TopBarAction::Shortcuts;
                    }
                    ui.add_space(2.0);
                    let native_active = session.native_document().is_some();
                    if native_active {
                        if session.can_redo_native() {
                            if common::icon_button(ui, Icon::Redo, "Redo").clicked() {
                                let _ = session.redo_native();
                            }
                        } else {
                            let _ = common::disabled_icon_button(ui, Icon::Redo, "Nothing to redo.");
                        }
                        if session.can_undo_native() {
                            if common::icon_button(ui, Icon::Undo, "Undo").clicked() {
                                let _ = session.undo_native();
                            }
                        } else {
                            let _ = common::disabled_icon_button(ui, Icon::Undo, "Nothing to undo.");
                        }
                    } else {
                        let redo_shortcut = shortcuts.shortcut_hint(CommandId::Redo);
                        let redo_hint = match redo_shortcut {
                            Some(shortcut) => format!(
                                "Redo history requires editor commands and is not implemented. ({shortcut})"
                            ),
                            None => {
                                "Redo history requires editor commands and is not implemented."
                                    .to_owned()
                            }
                        };
                        let _ = common::disabled_icon_button(ui, Icon::Redo, &redo_hint);
                        let undo_shortcut = shortcuts.shortcut_hint(CommandId::Undo);
                        let undo_hint = match undo_shortcut {
                            Some(shortcut) => format!(
                                "Undo history requires editor commands and is not implemented. ({shortcut})"
                            ),
                            None => {
                                "Undo history requires editor commands and is not implemented."
                                    .to_owned()
                            }
                        };
                        let _ = common::disabled_icon_button(ui, Icon::Undo, &undo_hint);
                    }
                });
            });
        });

    action
}

fn shortcuts_button(ui: &mut Ui, shortcut: Option<String>) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
    let hovered = response.hovered();
    if response.is_pointer_button_down_on() {
        ui.painter()
            .rect_filled(rect, Radii::CONTROL, Palette::WIDGET_ACTIVE);
    } else if hovered {
        ui.painter()
            .rect_filled(rect, Radii::CONTROL, Palette::WIDGET_BG);
    }
    focus_ring(ui, &response, rect.shrink(1.0), Radii::CONTROL);
    icons::paint_sized(
        ui.painter(),
        rect,
        Icon::Keyboard,
        IconSize::Standard,
        if hovered {
            IconColors::HOVERED
        } else {
            IconColors::MUTED
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::accesskit::Role::Button, true, "Keyboard Shortcuts")
    });
    let tooltip = match shortcut {
        Some(shortcut) => format!("Keyboard Shortcuts\n{shortcut}"),
        None => "Keyboard Shortcuts".to_owned(),
    };
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

fn home_button(ui: &mut Ui) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(58.0, 28.0), Sense::click());
    let hovered = response.hovered();
    if response.is_pointer_button_down_on() {
        ui.painter()
            .rect_filled(rect, Radii::CONTROL, Palette::WIDGET_ACTIVE);
    } else if hovered {
        ui.painter()
            .rect_filled(rect, Radii::CONTROL, Palette::WIDGET_BG);
    }
    focus_ring(ui, &response, rect.shrink(1.0), Radii::CONTROL);

    let icon_rect =
        Rect::from_center_size(pos2(rect.left() + 13.0, rect.center().y), vec2(20.0, 20.0));
    icons::paint_sized(
        ui.painter(),
        icon_rect,
        Icon::Home,
        IconSize::Navigation,
        if hovered {
            IconColors::HOVERED
        } else {
            IconColors::MUTED
        },
    );
    ui.painter().text(
        pos2(rect.left() + 23.0, rect.center().y),
        Align2::LEFT_CENTER,
        "Home",
        FontId::proportional(11.0),
        if hovered {
            Palette::TEXT_PRIMARY
        } else {
            Palette::TEXT_SECONDARY
        },
    );

    response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Button, true, "Home"));

    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text("Return to the homepage. The native project stays in memory; unsaved changes are protected.")
}

fn brand_mark(ui: &mut Ui) {
    let (mark_rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
    let painter = ui.painter_at(mark_rect);
    painter.rect_filled(mark_rect, Radii::CONTROL, Palette::ACCENT);
    painter.add(egui::Shape::convex_polygon(
        vec![
            pos2(mark_rect.center().x - 2.0, mark_rect.center().y - 4.0),
            pos2(mark_rect.center().x - 2.0, mark_rect.center().y + 4.0),
            pos2(mark_rect.center().x + 4.0, mark_rect.center().y),
        ],
        Palette::ON_ACCENT,
        Stroke::NONE,
    ));
    ui.label(
        RichText::new(config::APP_NAME)
            .size(13.0)
            .strong()
            .color(Palette::TEXT_PRIMARY),
    );
}

fn toolbar_separator(ui: &mut Ui) {
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);
}
