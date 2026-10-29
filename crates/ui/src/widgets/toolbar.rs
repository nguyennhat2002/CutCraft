//! Editor project bar: Home navigation, project identity, menus, undo/redo
//! placeholders, and the Export entry point.

use egui::{
    pos2, vec2, Align, Align2, CursorIcon, FontId, Frame, Label, Layout, Margin, Panel, Rect,
    Response, RichText, Sense, Stroke, Ui,
};

use crate::config;
use crate::editor_state::EditorSession;
use crate::theme::{Palette, Radii};
use crate::widgets::common;
use crate::widgets::common::focus_ring;
use crate::widgets::icons::{self, Icon};
use crate::widgets::menu_bar;

/// Result of interacting with the editor top bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TopBarAction {
    #[default]
    None,
    Home,
    Export,
}

pub fn show(ui: &mut Ui, session: &mut EditorSession) -> TopBarAction {
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
                ui.allocate_ui_with_layout(
                    vec2(68.0, 22.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        let _ = common::badge(ui, "Not saved", Palette::WARNING).on_hover_text(
                            "Temporary in-memory session; nothing is saved to disk.",
                        );
                    },
                );

                ui.add_space(4.0);
                if menu_bar::show_menus(ui, session) {
                    action = TopBarAction::Home;
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if common::primary_button(ui, "Export", 70.0).clicked() {
                        action = TopBarAction::Export;
                    }
                    ui.add_space(6.0);
                    let _ = common::disabled_icon_button(
                        ui,
                        Icon::Redo,
                        "Redo history requires editor commands and is not implemented.",
                    );
                    let _ = common::disabled_icon_button(
                        ui,
                        Icon::Undo,
                        "Undo history requires editor commands and is not implemented.",
                    );
                });
            });
        });

    action
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
        Rect::from_center_size(pos2(rect.left() + 12.0, rect.center().y), vec2(13.0, 13.0));
    icons::paint(
        ui.painter(),
        icon_rect,
        Icon::Home,
        if hovered {
            Palette::TEXT_PRIMARY
        } else {
            Palette::TEXT_SECONDARY
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
        .on_hover_text("Return to the homepage. Your temporary session stays in memory.")
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
