//! Homepage, Projects, and Settings screens.
//!
//! These screens share a navigation sidebar and top header. All project data is
//! temporary UI state; nothing is read from or written to disk.

use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Layout, Margin,
    Panel, Rect, Response, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui,
};

use crate::app_state::{AppState, Screen};
use crate::config;
use crate::theme::Palette;
use crate::widgets::common;
use crate::widgets::icons::{self, Icon};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    sidebar(ui, state);
    header(ui, state);

    let screen = state.screen();
    egui::CentralPanel::no_frame()
        .frame(
            Frame::new()
                .fill(Palette::APP_BG)
                .inner_margin(Margin::symmetric(28, 24)),
        )
        .show(ui, |ui| {
            let available = ui.available_width();
            let content_width = available.min(config::HOME_CONTENT_MAX_WIDTH);
            let side_padding = ((available - content_width) * 0.5).max(0.0);

            let _ = ScrollArea::vertical()
                .id_salt("home_content_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.add_space(side_padding);
                        ui.vertical(|ui| {
                            ui.set_width(content_width);
                            match screen {
                                Screen::Home => home_content(ui, state),
                                Screen::Projects => projects_content(ui, state),
                                Screen::Settings => settings_content(ui),
                                Screen::Editor => {}
                            }
                        });
                    });
                });
        });
}

fn sidebar(ui: &mut Ui, state: &mut AppState) {
    Panel::left("home_sidebar")
        .resizable(false)
        .exact_size(config::HOME_SIDEBAR_WIDTH)
        .frame(
            Frame::new()
                .fill(Palette::MENU_BG)
                .inner_margin(Margin::symmetric(12, 16)),
        )
        .show(ui, |ui| {
            brand(ui);
            ui.add_space(26.0);
            common::section_heading(ui, "Workspace");
            ui.add_space(4.0);

            let current = state.screen();
            if nav_row(ui, Icon::Home, "Home", current == Screen::Home).clicked() {
                state.go_home();
            }
            if nav_row(ui, Icon::Projects, "Projects", current == Screen::Projects).clicked() {
                state.go_projects();
            }
            if nav_row(ui, Icon::Settings, "Settings", current == Screen::Settings).clicked() {
                state.go_settings();
            }

            ui.add_space(20.0);
            common::section_heading(ui, "Coming later");
            ui.add_space(4.0);
            upcoming_row(ui, Icon::Effects, "Templates");
            upcoming_row(ui, Icon::Transitions, "Cloud sync");

            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.label(
                    RichText::new(format!("v{} \u{00b7} UI shell", config::APP_VERSION))
                        .size(9.5)
                        .color(Palette::TEXT_DISABLED),
                );
                ui.add_space(6.0);
                if ui.button("About CutCraft").clicked() {
                    state.open_about();
                }
                ui.add_space(8.0);
                ui.separator();
            });
        });
}

fn brand(ui: &mut Ui) {
    ui.horizontal(|ui| {
        let (mark_rect, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
        let painter = ui.painter_at(mark_rect);
        painter.rect_filled(mark_rect, CornerRadius::same(8), Palette::ACCENT);
        painter.add(egui::Shape::convex_polygon(
            vec![
                pos2(mark_rect.center().x - 3.0, mark_rect.center().y - 6.0),
                pos2(mark_rect.center().x - 3.0, mark_rect.center().y + 6.0),
                pos2(mark_rect.center().x + 6.0, mark_rect.center().y),
            ],
            Palette::ON_ACCENT,
            Stroke::NONE,
        ));

        ui.add_space(2.0);
        ui.vertical(|ui| {
            ui.label(
                RichText::new(config::APP_NAME)
                    .size(18.0)
                    .strong()
                    .color(Palette::TEXT_PRIMARY),
            );
            ui.label(
                RichText::new("DESKTOP EDITOR")
                    .size(8.0)
                    .strong()
                    .color(Palette::TEXT_MUTED),
            );
        });
    });
}

fn nav_row(ui: &mut Ui, icon: Icon, label: &str, selected: bool) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, 36.0), Sense::click());
    let hovered = response.hovered();

    if selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(7), Palette::ACCENT_BG);
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, vec2(3.0, rect.height())),
            CornerRadius::same(2),
            Palette::ACCENT,
        );
    } else if hovered {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(7), Palette::WIDGET_BG);
    }

    let icon_rect =
        Rect::from_center_size(pos2(rect.left() + 20.0, rect.center().y), vec2(16.0, 16.0));
    icons::paint(
        ui.painter(),
        icon_rect,
        icon,
        if selected {
            Palette::ACCENT
        } else {
            Palette::TEXT_MUTED
        },
    );

    ui.painter().text(
        pos2(rect.left() + 38.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.5),
        if selected || hovered {
            Palette::TEXT_PRIMARY
        } else {
            Palette::TEXT_SECONDARY
        },
    );

    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn upcoming_row(ui: &mut Ui, icon: Icon, label: &str) {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
    let icon_rect =
        Rect::from_center_size(pos2(rect.left() + 20.0, rect.center().y), vec2(15.0, 15.0));
    icons::paint(ui.painter(), icon_rect, icon, Palette::TEXT_DISABLED);
    ui.painter().text(
        pos2(rect.left() + 38.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.0),
        Palette::TEXT_DISABLED,
    );
    ui.painter().text(
        pos2(rect.right() - 2.0, rect.center().y),
        Align2::RIGHT_CENTER,
        "SOON",
        FontId::proportional(8.0),
        Palette::TEXT_DISABLED,
    );
    let _ = response.on_hover_text("Planned for a later milestone; not available yet.");
}

fn header(ui: &mut Ui, state: &mut AppState) {
    let screen = state.screen();
    let has_session = state.has_session();

    Panel::top("home_header")
        .exact_size(config::HOME_HEADER_HEIGHT)
        .frame(
            Frame::new()
                .fill(Palette::PANEL_BG)
                .inner_margin(Margin::symmetric(24, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(screen.title())
                            .size(18.0)
                            .strong()
                            .color(Palette::TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new("CUTCRAFT \u{00b7} WORKSPACE")
                            .size(8.5)
                            .strong()
                            .color(Palette::TEXT_DISABLED),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("About").clicked() {
                        state.open_about();
                    }
                    ui.add_space(4.0);
                    ui.allocate_ui_with_layout(
                        vec2(102.0, 22.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            common::badge(ui, "Local UI shell", Palette::ACCENT);
                        },
                    );
                    if has_session {
                        ui.add_space(4.0);
                        ui.allocate_ui_with_layout(
                            vec2(116.0, 22.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                common::badge(ui, "Unsaved session", Palette::WARNING);
                            },
                        );
                    }
                });
            });
        });
}

fn home_content(ui: &mut Ui, state: &mut AppState) {
    hero(ui, state);
    ui.add_space(28.0);

    if let Some(project_name) = state.session().map(|session| session.project_name.clone()) {
        current_session_card(ui, state, &project_name);
        ui.add_space(24.0);
    }

    recent_projects(ui);
    ui.add_space(26.0);
    upcoming_features(ui);
}

fn hero(ui: &mut Ui, state: &mut AppState) {
    let width = ui.available_width();
    if width >= 820.0 {
        ui.horizontal_top(|ui| {
            let gap = 28.0;
            let left_width = width * 0.56;
            let right_width = (width - left_width - gap).max(280.0);
            ui.allocate_ui_with_layout(
                vec2(left_width, 244.0),
                Layout::top_down(Align::Min),
                |ui| hero_text(ui, state),
            );
            ui.add_space(gap);
            ui.allocate_ui_with_layout(
                vec2(right_width, 244.0),
                Layout::top_down(Align::Min),
                hero_illustration,
            );
        });
    } else {
        hero_text(ui, state);
    }
}

fn hero_text(ui: &mut Ui, state: &mut AppState) {
    ui.add_space(6.0);
    ui.label(
        RichText::new("NEW SESSION")
            .size(9.5)
            .strong()
            .color(Palette::ACCENT),
    );
    ui.add_space(6.0);
    ui.add(
        egui::Label::new(
            RichText::new("Your next cut starts here.")
                .size(34.0)
                .strong()
                .color(Palette::TEXT_PRIMARY),
        )
        .wrap(),
    );
    ui.add_space(10.0);
    ui.add(
        egui::Label::new(
            RichText::new(
                "Open a temporary CutCraft editing session. Explore the layout, \
                 preview aspect ratios, timeline, and export settings.",
            )
            .size(12.5)
            .color(Palette::TEXT_SECONDARY),
        )
        .wrap(),
    );
    ui.add_space(18.0);

    ui.horizontal(|ui| {
        if common::primary_button(ui, "Create Project", 168.0).clicked() {
            state.create_temporary_project();
        }
        ui.add_space(4.0);
        if state.has_session() && ui.button("Resume session").clicked() {
            state.resume_editor();
        }
    });

    ui.add_space(8.0);
    ui.label(
        RichText::new("Temporary in-memory session \u{2014} nothing is saved to disk.")
            .size(9.5)
            .color(Palette::TEXT_DISABLED),
    );
}

fn hero_illustration(ui: &mut Ui) {
    let size = vec2(ui.available_width(), 234.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, CornerRadius::same(12), Palette::CARD_BG);
    painter.rect_stroke(
        rect,
        CornerRadius::same(12),
        Stroke::new(1.0, Palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );

    let glow_center = pos2(rect.right() - 54.0, rect.top() + 46.0);
    painter.circle_filled(
        glow_center,
        64.0,
        Color32::from_rgba_unmultiplied(45, 212, 191, 12),
    );
    painter.circle_filled(
        glow_center,
        38.0,
        Color32::from_rgba_unmultiplied(45, 212, 191, 14),
    );

    for index in 0..3 {
        painter.circle_filled(
            pos2(rect.left() + 18.0 + index as f32 * 10.0, rect.top() + 17.0),
            2.5,
            Palette::BORDER,
        );
    }
    painter.text(
        pos2(rect.right() - 16.0, rect.top() + 16.0),
        Align2::RIGHT_CENTER,
        "LAYOUT PREVIEW",
        FontId::proportional(8.0),
        Palette::TEXT_DISABLED,
    );

    let inner = rect.shrink(18.0);
    let preview_height = inner.height() * 0.6;
    let preview = Rect::from_min_size(inner.min, vec2(inner.width(), preview_height));
    painter.rect_filled(preview, CornerRadius::same(6), Palette::CANVAS);
    painter.rect_stroke(
        preview,
        CornerRadius::same(6),
        Stroke::new(1.0, Palette::BORDER),
        StrokeKind::Inside,
    );
    painter.circle_filled(
        preview.center(),
        20.0,
        Color32::from_rgba_unmultiplied(45, 212, 191, 32),
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            pos2(preview.center().x - 5.0, preview.center().y - 9.0),
            pos2(preview.center().x - 5.0, preview.center().y + 9.0),
            pos2(preview.center().x + 9.0, preview.center().y),
        ],
        Palette::ACCENT,
        Stroke::NONE,
    ));

    let timeline_top = preview.bottom() + 14.0;
    let timeline = Rect::from_min_max(
        pos2(inner.left(), timeline_top),
        pos2(inner.right(), inner.bottom()),
    );
    painter.rect_filled(timeline, CornerRadius::same(5), Palette::RULER_BG);

    let header_width = 46.0;
    let rows = 2;
    let row_height = (timeline.height() - 8.0) / rows as f32;
    for row in 0..rows {
        let y = timeline.top() + 4.0 + row as f32 * row_height;
        painter.rect_filled(
            Rect::from_min_size(
                pos2(timeline.left() + header_width, y),
                vec2(timeline.width() - header_width, row_height - 3.0),
            ),
            CornerRadius::same(2),
            if row == 0 {
                Palette::TRACK_BG
            } else {
                Palette::TRACK_ALT_BG
            },
        );
        painter.text(
            pos2(timeline.left() + 9.0, y + row_height * 0.4),
            Align2::LEFT_CENTER,
            if row == 0 { "V1" } else { "A1" },
            FontId::monospace(8.0),
            Palette::TEXT_DISABLED,
        );
    }

    let playhead_x = timeline.left() + header_width + 38.0;
    painter.line_segment(
        [
            pos2(playhead_x, timeline.top()),
            pos2(playhead_x, timeline.bottom()),
        ],
        Stroke::new(1.5, Palette::PLAYHEAD),
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            pos2(playhead_x - 4.0, timeline.top()),
            pos2(playhead_x + 4.0, timeline.top()),
            pos2(playhead_x, timeline.top() + 6.0),
        ],
        Palette::PLAYHEAD,
        Stroke::NONE,
    ));
}

fn current_session_card(ui: &mut Ui, state: &mut AppState, project_name: &str) {
    Frame::new()
        .fill(Palette::ACCENT_BG)
        .stroke(Stroke::new(1.0, Palette::ACCENT_DIM))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
                ui.painter()
                    .rect_filled(icon_rect, CornerRadius::same(8), Palette::ACCENT);
                icons::paint(
                    ui.painter(),
                    icon_rect.shrink(9.0),
                    Icon::Media,
                    Palette::ON_ACCENT,
                );

                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(project_name)
                            .size(14.0)
                            .strong()
                            .color(Palette::TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new("Current temporary session \u{00b7} not saved to disk")
                            .size(10.0)
                            .color(Palette::TEXT_SECONDARY),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if common::primary_button(ui, "Resume editing", 124.0).clicked() {
                        state.resume_editor();
                    }
                });
            });
        });
}

fn recent_projects(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Recent projects")
                .size(18.0)
                .strong()
                .color(Palette::TEXT_PRIMARY),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            common::badge(ui, "0 saved", Palette::TEXT_MUTED);
        });
    });
    ui.add_space(10.0);

    common::card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            let (icon_rect, _) = ui.allocate_exact_size(vec2(46.0, 46.0), Sense::hover());
            ui.painter()
                .rect_filled(icon_rect, CornerRadius::same(10), Palette::WIDGET_BG);
            icons::paint(
                ui.painter(),
                icon_rect.shrink(13.0),
                Icon::Folder,
                Palette::TEXT_MUTED,
            );

            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("No saved projects yet")
                        .size(13.5)
                        .strong()
                        .color(Palette::TEXT_SECONDARY),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "Project persistence is not implemented. Recent projects will \
                             appear here after project storage exists.",
                        )
                        .size(10.5)
                        .color(Palette::TEXT_MUTED),
                    )
                    .wrap(),
                );
            });
        });
    });
}

fn upcoming_features(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("What's coming next")
                .size(16.0)
                .strong()
                .color(Palette::TEXT_PRIMARY),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new("PLANNED \u{00b7} NOT AVAILABLE YET")
                    .size(8.5)
                    .color(Palette::TEXT_DISABLED),
            );
        });
    });
    ui.add_space(10.0);

    let available = ui.available_width();
    let gap = 12.0;
    let card_width = ((available - gap * 2.0) / 3.0).max(170.0);
    ui.horizontal_top(|ui| {
        feature_card(
            ui,
            card_width,
            Icon::Media,
            "Media import",
            "Decode local video and audio into the media library.",
        );
        ui.add_space(gap);
        feature_card(
            ui,
            card_width,
            Icon::Audio,
            "Timeline editing",
            "Arrange clips, trim, split, and undo changes.",
        );
        ui.add_space(gap);
        feature_card(
            ui,
            card_width,
            Icon::Export,
            "Export encoding",
            "Render an MP4 with the configured settings.",
        );
    });
}

fn feature_card(ui: &mut Ui, width: f32, icon: Icon, title: &str, body: &str) {
    ui.allocate_ui_with_layout(vec2(width, 150.0), Layout::top_down(Align::Min), |ui| {
        Frame::new()
            .fill(Palette::CARD_BG)
            .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.set_min_width(width - 28.0);
                let (icon_rect, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
                ui.painter()
                    .rect_filled(icon_rect, CornerRadius::same(8), Palette::ACCENT_BG);
                icons::paint(ui.painter(), icon_rect.shrink(8.0), icon, Palette::ACCENT);
                ui.add_space(6.0);
                ui.label(
                    RichText::new(title)
                        .size(12.5)
                        .strong()
                        .color(Palette::TEXT_SECONDARY),
                );
                ui.add(
                    egui::Label::new(RichText::new(body).size(10.0).color(Palette::TEXT_MUTED))
                        .wrap(),
                );
                ui.add_space(4.0);
                common::badge(ui, "Planned", Palette::WARNING);
            });
    });
}

fn projects_content(ui: &mut Ui, state: &mut AppState) {
    page_intro(
        ui,
        "Your projects",
        "Start a temporary edit or return to the session held in memory.",
    );
    ui.add_space(12.0);
    if common::primary_button(ui, "Create Project", 150.0).clicked() {
        state.create_temporary_project();
    }
    ui.add_space(24.0);

    if let Some(project_name) = state.session().map(|session| session.project_name.clone()) {
        common::section_heading(ui, "Current session");
        ui.add_space(6.0);
        current_session_card(ui, state, &project_name);
        ui.add_space(24.0);
    }

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Saved projects")
                .size(18.0)
                .strong()
                .color(Palette::TEXT_PRIMARY),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            common::badge(ui, "0 saved", Palette::TEXT_MUTED);
        });
    });
    ui.add_space(10.0);

    common::card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.vertical_centered(|ui| {
            let (icon_rect, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
            icons::paint(
                ui.painter(),
                icon_rect,
                Icon::Folder,
                Palette::TEXT_DISABLED,
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new("No saved projects")
                    .size(13.5)
                    .strong()
                    .color(Palette::TEXT_SECONDARY),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "This build does not read or write project files. Saved projects \
                         will appear here after persistence is implemented.",
                    )
                    .size(10.5)
                    .color(Palette::TEXT_MUTED),
                )
                .wrap(),
            );
        });
    });
}

fn settings_content(ui: &mut Ui) {
    page_intro(
        ui,
        "Settings",
        "Application preferences are placeholders in this UI milestone.",
    );
    ui.add_space(20.0);

    settings_card(
        ui,
        "Appearance",
        &[
            ("Theme", "Dark (fixed)"),
            ("Accent", "Teal"),
            ("Density", "Compact"),
        ],
        "Theme switching and customization are not implemented.",
    );
    ui.add_space(12.0);
    settings_card(
        ui,
        "Project storage",
        &[
            ("Persistence", "Not implemented"),
            ("Autosave", "Not implemented"),
        ],
        "Projects exist only in memory and disappear when the app closes.",
    );
    ui.add_space(12.0);
    settings_card(
        ui,
        "Playback and export",
        &[
            ("Preview engine", "Not implemented"),
            ("Encoder", "Not implemented"),
        ],
        "Playback, decoding, and export controls remain disabled.",
    );
}

fn settings_card(ui: &mut Ui, title: &str, rows: &[(&str, &str)], note: &str) {
    common::card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            RichText::new(title)
                .size(14.0)
                .strong()
                .color(Palette::TEXT_PRIMARY),
        );
        ui.add_space(6.0);
        for (label, value) in rows {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(*label)
                        .size(11.0)
                        .color(Palette::TEXT_SECONDARY),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(*value)
                            .size(11.0)
                            .color(Palette::TEXT_DISABLED),
                    );
                });
            });
        }
        ui.add_space(4.0);
        ui.add(egui::Label::new(RichText::new(note).size(10.0).color(Palette::TEXT_MUTED)).wrap());
    });
}

fn page_intro(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(
        RichText::new(title)
            .size(28.0)
            .strong()
            .color(Palette::TEXT_PRIMARY),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new(subtitle)
            .size(12.0)
            .color(Palette::TEXT_MUTED),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_input(width: f32, height: f32) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(width, height))),
            ..Default::default()
        }
    }

    fn collect_text(output: &egui::FullOutput) -> Vec<String> {
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn homepage_renders_create_action_and_empty_recent_projects() {
        let context = egui::Context::default();
        let mut state = AppState::default();
        let output = context.run_ui(test_input(1440.0, 900.0), |ui| show(ui, &mut state));
        let text = collect_text(&output);

        assert!(text.iter().any(|line| line == "Create Project"));
        assert!(text.iter().any(|line| line == "Recent projects"));
        assert!(text
            .iter()
            .any(|line| line.contains("No saved projects yet")));
        output.drop_without_applying_deltas();
    }

    #[test]
    fn projects_and_settings_screens_render_without_a_session() {
        let context = egui::Context::default();
        let mut state = AppState::default();
        state.go_projects();
        let output = context.run_ui(test_input(960.0, 600.0), |ui| show(ui, &mut state));
        assert!(!output.shapes.is_empty());
        output.drop_without_applying_deltas();

        state.go_settings();
        let output = context.run_ui(test_input(960.0, 600.0), |ui| show(ui, &mut state));
        let text = collect_text(&output);
        assert!(text.iter().any(|line| line == "Settings"));
        output.drop_without_applying_deltas();
    }

    #[test]
    fn homepage_shows_resume_card_for_a_temporary_session() {
        let context = egui::Context::default();
        let mut state = AppState::default();
        state.create_temporary_project();
        state.go_home();
        let output = context.run_ui(test_input(1280.0, 800.0), |ui| show(ui, &mut state));
        let text = collect_text(&output);

        assert!(text.iter().any(|line| line.contains("Resume editing")));
        assert!(text
            .iter()
            .any(|line| line.contains("Current temporary session")));
        output.drop_without_applying_deltas();
    }
}
