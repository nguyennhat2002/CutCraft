//! Homepage, Projects, and Settings screens.
//!
//! These screens share a navigation sidebar and top header. All project data is
//! temporary UI state; nothing is read from or written to disk.

use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Layout, Margin,
    Mesh, Panel, Rect, Response, RichText, ScrollArea, Sense, Shape, Stroke, StrokeKind, TextEdit,
    Ui, UiBuilder,
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
                .inner_margin(Margin::symmetric(28, 22)),
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
            ui.add_space(22.0);
            common::section_heading(ui, "Workspace");
            ui.add_space(4.0);

            let current = state.screen();
            if nav_row(ui, Icon::Home, "Home", current == Screen::Home).clicked() {
                state.go_home();
            }
            if nav_row(ui, Icon::Projects, "Projects", current == Screen::Projects).clicked() {
                state.go_projects();
            }
            upcoming_row(ui, Icon::Effects, "Templates");
            if nav_row(ui, Icon::Settings, "Settings", current == Screen::Settings).clicked() {
                state.go_settings();
            }

            ui.add_space(18.0);
            common::section_heading(ui, "Coming later");
            ui.add_space(4.0);
            upcoming_row(ui, Icon::Transitions, "Cloud sync");
            upcoming_row(ui, Icon::Stickers, "Team spaces");

            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.label(
                    RichText::new(format!("v{} \u{00b7} UI shell", config::APP_VERSION))
                        .size(9.0)
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
        painter.rect_filled(mark_rect, CornerRadius::same(7), Palette::ACCENT);
        painter.add(Shape::convex_polygon(
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
                    .size(17.0)
                    .strong()
                    .color(Palette::TEXT_PRIMARY),
            );
            ui.label(
                RichText::new("DESKTOP EDITOR")
                    .size(7.5)
                    .strong()
                    .color(Palette::TEXT_DISABLED),
            );
        });
    });
}

fn nav_row(ui: &mut Ui, icon: Icon, label: &str, selected: bool) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, 34.0), Sense::click());
    let hovered = response.hovered();

    if selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), Palette::ACCENT_BG);
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, vec2(2.0, rect.height())),
            CornerRadius::same(1),
            Palette::ACCENT,
        );
    } else if hovered {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), Palette::WIDGET_BG);
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
        FontId::proportional(12.0),
        if selected || hovered {
            Palette::TEXT_PRIMARY
        } else {
            Palette::TEXT_SECONDARY
        },
    );

    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn upcoming_row(ui: &mut Ui, icon: Icon, label: &str) {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
    let icon_rect =
        Rect::from_center_size(pos2(rect.left() + 20.0, rect.center().y), vec2(15.0, 15.0));
    icons::paint(ui.painter(), icon_rect, icon, Palette::TEXT_DISABLED);
    ui.painter().text(
        pos2(rect.left() + 38.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.5),
        Palette::TEXT_DISABLED,
    );
    ui.painter().text(
        pos2(rect.right() - 2.0, rect.center().y),
        Align2::RIGHT_CENTER,
        "SOON",
        FontId::proportional(7.5),
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
                .inner_margin(Margin::symmetric(22, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(screen.title())
                            .size(17.0)
                            .strong()
                            .color(Palette::TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new("CUTCRAFT \u{00b7} WORKSPACE")
                            .size(7.5)
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
                        vec2(98.0, 20.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            common::badge(ui, "Local UI shell", Palette::ACCENT);
                        },
                    );
                    if has_session {
                        ui.add_space(4.0);
                        ui.allocate_ui_with_layout(
                            vec2(112.0, 20.0),
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
    create_banner(ui, state);
    ui.add_space(20.0);
    feature_cards(ui);
    ui.add_space(22.0);
    recent_projects(ui, state);
}

fn create_banner(ui: &mut Ui, state: &mut AppState) {
    let width = ui.available_width();
    let wide = width >= 900.0;
    let height = if wide { 244.0 } else { 224.0 };
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    paint_banner_background(ui, rect);

    let inner = rect.shrink2(vec2(28.0, 24.0));
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::top_down(Align::Min)),
        |ui| {
            if wide {
                let gap = 26.0;
                let left_width = (inner.width() * 0.56).max(360.0);
                let right_width = (inner.width() - left_width - gap).max(240.0);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        vec2(left_width, inner.height()),
                        Layout::top_down(Align::Min),
                        |ui| banner_copy(ui, state, false),
                    );
                    ui.add_space(gap);
                    ui.allocate_ui_with_layout(
                        vec2(right_width, inner.height()),
                        Layout::top_down(Align::Min),
                        banner_artwork,
                    );
                });
            } else {
                banner_copy(ui, state, true);
            }
        },
    );
}

fn banner_copy(ui: &mut Ui, state: &mut AppState, compact: bool) {
    ui.add_space(if compact { 0.0 } else { 8.0 });
    ui.label(
        RichText::new("CUTCRAFT WORKSPACE")
            .size(8.5)
            .strong()
            .color(Palette::ACCENT),
    );
    ui.add_space(5.0);
    ui.add(
        egui::Label::new(
            RichText::new("Create something worth sharing.")
                .size(if compact { 25.0 } else { 31.0 })
                .strong()
                .color(Palette::TEXT_PRIMARY),
        )
        .wrap(),
    );
    ui.add_space(8.0);
    ui.add(
        egui::Label::new(
            RichText::new(
                "Open a temporary CutCraft session and explore the editing workspace. \
                 Projects stay in memory until persistence exists.",
            )
            .size(11.5)
            .color(Palette::TEXT_SECONDARY),
        )
        .wrap(),
    );
    ui.add_space(16.0);

    ui.horizontal(|ui| {
        if common::primary_button(ui, "Create Project", 156.0).clicked() {
            state.create_temporary_project();
        }
        ui.add_space(4.0);
        if state.has_session() && ui.button("Resume editing").clicked() {
            state.resume_editor();
        }
    });

    ui.add_space(8.0);
    ui.label(
        RichText::new("Temporary session \u{2014} nothing is saved to disk.")
            .size(9.0)
            .color(Palette::TEXT_MUTED),
    );
}

fn banner_artwork(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), ui.available_height().max(150.0)),
        Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    let card = rect.shrink2(vec2(8.0, 12.0));
    painter.rect_filled(
        card,
        CornerRadius::same(8),
        Color32::from_rgba_unmultiplied(0, 0, 0, 55),
    );
    painter.rect_stroke(
        card,
        CornerRadius::same(8),
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 26)),
        StrokeKind::Inside,
    );

    let stage = Rect::from_min_size(
        card.min + vec2(14.0, 14.0),
        vec2(card.width() - 28.0, card.height() * 0.58),
    );
    painter.rect_filled(stage, CornerRadius::same(5), Palette::CANVAS);
    painter.rect_stroke(
        stage,
        CornerRadius::same(5),
        Stroke::new(1.0, Palette::BORDER),
        StrokeKind::Inside,
    );
    painter.circle_filled(
        stage.center(),
        18.0,
        Color32::from_rgba_unmultiplied(24, 197, 207, 36),
    );
    painter.add(Shape::convex_polygon(
        vec![
            pos2(stage.center().x - 4.0, stage.center().y - 8.0),
            pos2(stage.center().x - 4.0, stage.center().y + 8.0),
            pos2(stage.center().x + 8.0, stage.center().y),
        ],
        Palette::ACCENT,
        Stroke::NONE,
    ));

    let timeline_top = stage.bottom() + 12.0;
    let timeline = Rect::from_min_max(
        pos2(card.left() + 14.0, timeline_top),
        pos2(card.right() - 14.0, card.bottom() - 14.0),
    );
    painter.rect_filled(timeline, CornerRadius::same(4), Palette::RULER_BG);
    let header_width = 42.0;
    for row in 0..2 {
        let y = timeline.top() + 7.0 + row as f32 * 20.0;
        painter.rect_filled(
            Rect::from_min_size(
                pos2(timeline.left() + header_width, y),
                vec2(timeline.width() - header_width - 8.0, 13.0),
            ),
            CornerRadius::same(2),
            if row == 0 {
                Palette::TRACK_BG
            } else {
                Palette::TRACK_ALT_BG
            },
        );
        painter.text(
            pos2(timeline.left() + 9.0, y + 6.0),
            Align2::LEFT_CENTER,
            if row == 0 { "V1" } else { "A1" },
            FontId::monospace(7.0),
            Palette::TEXT_DISABLED,
        );
    }
    let playhead_x = timeline.left() + header_width + 28.0;
    painter.line_segment(
        [
            pos2(playhead_x, timeline.top()),
            pos2(playhead_x, timeline.bottom()),
        ],
        Stroke::new(1.4, Palette::PLAYHEAD),
    );
}

fn paint_banner_background(ui: &Ui, rect: Rect) {
    let painter = ui.painter_at(rect);
    let start = Color32::from_rgb(0x0a, 0x22, 0x26);
    let end = Color32::from_rgb(0x0e, 0x52, 0x59);
    let radius = 9.0_f32;

    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.center(), mix_color(start, end, 0.55));

    let mut perimeter = Vec::with_capacity(24);
    let corners = [
        (
            pos2(rect.left() + radius, rect.top() + radius),
            std::f32::consts::PI,
            1.5 * std::f32::consts::PI,
        ),
        (
            pos2(rect.right() - radius, rect.top() + radius),
            1.5 * std::f32::consts::PI,
            2.0 * std::f32::consts::PI,
        ),
        (
            pos2(rect.right() - radius, rect.bottom() - radius),
            0.0,
            0.5 * std::f32::consts::PI,
        ),
        (
            pos2(rect.left() + radius, rect.bottom() - radius),
            0.5 * std::f32::consts::PI,
            std::f32::consts::PI,
        ),
    ];
    for (center, start_angle, end_angle) in corners {
        for step in 0..=4 {
            let angle = start_angle + (end_angle - start_angle) * step as f32 / 4.0;
            perimeter.push(center + vec2(angle.cos() * radius, angle.sin() * radius));
        }
    }
    for point in &perimeter {
        let t = (((point.x - rect.left()) / rect.width()) * 0.75
            + (1.0 - (point.y - rect.top()) / rect.height()) * 0.25)
            .clamp(0.0, 1.0);
        mesh.colored_vertex(*point, mix_color(start, end, t));
    }
    for index in 0..perimeter.len() {
        mesh.add_triangle(
            0,
            (index + 1) as u32,
            (((index + 1) % perimeter.len()) + 1) as u32,
        );
    }
    painter.add(Shape::mesh(mesh));

    painter.circle_filled(
        pos2(rect.right() - 120.0, rect.top() + 24.0),
        120.0,
        Color32::from_rgba_unmultiplied(24, 197, 207, 18),
    );
    painter.circle_filled(
        pos2(rect.right() - 260.0, rect.bottom() - 20.0),
        150.0,
        Color32::from_rgba_unmultiplied(24, 197, 207, 8),
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(9),
        Stroke::new(1.0, Palette::ACCENT_DIM),
        StrokeKind::Inside,
    );
}

fn mix_color(from: Color32, to: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color32::from_rgb(
        mix(from.r(), to.r()),
        mix(from.g(), to.g()),
        mix(from.b(), to.b()),
    )
}

fn feature_cards(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Explore CutCraft")
                .size(16.0)
                .strong()
                .color(Palette::TEXT_PRIMARY),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new("UI PREVIEWS \u{00b7} PLANNED")
                    .size(7.5)
                    .color(Palette::TEXT_DISABLED),
            );
        });
    });
    ui.add_space(8.0);

    let available = ui.available_width();
    let gap = 10.0;
    let columns = if available >= 1040.0 { 4 } else { 2 };
    let card_width = ((available - gap * (columns - 1) as f32) / columns as f32).max(170.0);

    let cards = [
        (
            Icon::Media,
            "Media library",
            "Import and organize source clips.",
        ),
        (
            Icon::Text,
            "Text & captions",
            "Titles, subtitles, and captions.",
        ),
        (
            Icon::Effects,
            "Effects & filters",
            "Color and motion adjustments.",
        ),
        (
            Icon::Export,
            "Export presets",
            "MP4 setup for future encoding.",
        ),
    ];

    for row in cards.chunks(columns) {
        ui.horizontal_top(|ui| {
            for (icon, title, body) in row {
                feature_card(ui, card_width, *icon, title, body);
                ui.add_space(gap);
            }
        });
        ui.add_space(4.0);
    }
}

fn feature_card(ui: &mut Ui, width: f32, icon: Icon, title: &str, body: &str) {
    ui.allocate_ui_with_layout(vec2(width, 138.0), Layout::top_down(Align::Min), |ui| {
        Frame::new()
            .fill(Palette::CARD_BG)
            .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(7))
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.set_min_width((width - 28.0).max(80.0));
                ui.horizontal(|ui| {
                    let (icon_rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                    ui.painter()
                        .rect_filled(icon_rect, CornerRadius::same(6), Palette::ACCENT_BG);
                    icons::paint(ui.painter(), icon_rect.shrink(8.0), icon, Palette::ACCENT);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        common::badge(ui, "Planned", Palette::WARNING);
                    });
                });
                ui.add_space(8.0);
                ui.label(
                    RichText::new(title)
                        .size(12.0)
                        .strong()
                        .color(Palette::TEXT_SECONDARY),
                );
                ui.add(
                    egui::Label::new(RichText::new(body).size(9.5).color(Palette::TEXT_MUTED))
                        .wrap(),
                );
            });
    });
}

fn recent_projects(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Recent projects")
                .size(17.0)
                .strong()
                .color(Palette::TEXT_PRIMARY),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(20.0);
            common::badge(ui, "0 saved", Palette::TEXT_MUTED);
        });
    });
    ui.add_space(3.0);

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Search and project views require saved projects.")
                .size(8.5)
                .color(Palette::TEXT_DISABLED),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(20.0);
            let _ = common::disabled_icon_button(
                ui,
                Icon::List,
                "Project views require saved projects.",
            );
            let _ = common::disabled_icon_button(
                ui,
                Icon::Grid,
                "Project views require saved projects.",
            );
            let mut query = String::new();
            let search = ui.add_enabled(
                false,
                TextEdit::singleline(&mut query)
                    .hint_text("Search projects")
                    .desired_width(170.0),
            );
            let _ = search.on_disabled_hover_text("Search requires saved projects.");
        });
    });
    ui.add_space(8.0);

    if let Some(project_name) = state.session().map(|session| session.project_name.clone()) {
        common::section_heading(ui, "Current session");
        ui.add_space(4.0);
        current_session_card(ui, state, &project_name);
        ui.add_space(12.0);
    }

    common::card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            let (icon_rect, _) = ui.allocate_exact_size(vec2(42.0, 42.0), Sense::hover());
            ui.painter()
                .rect_filled(icon_rect, CornerRadius::same(8), Palette::WIDGET_BG);
            icons::paint(
                ui.painter(),
                icon_rect.shrink(12.0),
                Icon::Folder,
                Palette::TEXT_MUTED,
            );

            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("No saved projects yet")
                        .size(12.5)
                        .strong()
                        .color(Palette::TEXT_SECONDARY),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "Project persistence is not implemented. Recent projects will \
                             appear here after storage exists.",
                        )
                        .size(10.0)
                        .color(Palette::TEXT_MUTED),
                    )
                    .wrap(),
                );
            });
        });
    });
}

fn current_session_card(ui: &mut Ui, state: &mut AppState, project_name: &str) {
    Frame::new()
        .fill(Palette::CARD_BG)
        .stroke(Stroke::new(1.0, Palette::ACCENT_DIM))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                ui.painter()
                    .rect_filled(icon_rect, CornerRadius::same(6), Palette::ACCENT_BG);
                icons::paint(
                    ui.painter(),
                    icon_rect.shrink(8.0),
                    Icon::Media,
                    Palette::ACCENT,
                );

                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(project_name)
                            .size(13.0)
                            .strong()
                            .color(Palette::TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new("Temporary session \u{00b7} not saved to disk")
                            .size(9.0)
                            .color(Palette::TEXT_MUTED),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if common::primary_button(ui, "Resume editing", 118.0).clicked() {
                        state.resume_editor();
                    }
                });
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
    ui.add_space(22.0);

    if let Some(project_name) = state.session().map(|session| session.project_name.clone()) {
        common::section_heading(ui, "Current session");
        ui.add_space(4.0);
        current_session_card(ui, state, &project_name);
        ui.add_space(20.0);
    }

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Saved projects")
                .size(17.0)
                .strong()
                .color(Palette::TEXT_PRIMARY),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            common::badge(ui, "0 saved", Palette::TEXT_MUTED);
        });
    });
    ui.add_space(8.0);

    common::card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.vertical_centered(|ui| {
            let (icon_rect, _) = ui.allocate_exact_size(vec2(42.0, 42.0), Sense::hover());
            icons::paint(
                ui.painter(),
                icon_rect,
                Icon::Folder,
                Palette::TEXT_DISABLED,
            );
            ui.add_space(5.0);
            ui.label(
                RichText::new("No saved projects")
                    .size(12.5)
                    .strong()
                    .color(Palette::TEXT_SECONDARY),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "This build does not read or write project files. Saved projects \
                         will appear here after persistence is implemented.",
                    )
                    .size(10.0)
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
    ui.add_space(18.0);

    settings_card(
        ui,
        "Appearance",
        &[
            ("Theme", "Dark (fixed)"),
            ("Accent", "Cyan"),
            ("Density", "Compact"),
        ],
        "Theme switching and customization are not implemented.",
    );
    ui.add_space(10.0);
    settings_card(
        ui,
        "Project storage",
        &[
            ("Persistence", "Not implemented"),
            ("Autosave", "Not implemented"),
        ],
        "Projects exist only in memory and disappear when the app closes.",
    );
    ui.add_space(10.0);
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
    Frame::new()
        .fill(Palette::CARD_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new(title)
                    .size(13.0)
                    .strong()
                    .color(Palette::TEXT_PRIMARY),
            );
            ui.add_space(5.0);
            for (label, value) in rows {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(*label)
                            .size(10.5)
                            .color(Palette::TEXT_SECONDARY),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(*value)
                                .size(10.5)
                                .color(Palette::TEXT_DISABLED),
                        );
                    });
                });
            }
            ui.add_space(4.0);
            ui.add(
                egui::Label::new(RichText::new(note).size(9.5).color(Palette::TEXT_MUTED)).wrap(),
            );
        });
}

fn page_intro(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(
        RichText::new(title)
            .size(26.0)
            .strong()
            .color(Palette::TEXT_PRIMARY),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new(subtitle)
            .size(11.5)
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
    fn homepage_renders_create_banner_features_and_empty_recent_projects() {
        let context = egui::Context::default();
        let mut state = AppState::default();
        let output = context.run_ui(test_input(1600.0, 900.0), |ui| show(ui, &mut state));
        let text = collect_text(&output);

        assert!(text.iter().any(|line| line == "Create Project"));
        assert!(text.iter().any(|line| line == "Explore CutCraft"));
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
        let output = context.run_ui(test_input(1080.0, 720.0), |ui| show(ui, &mut state));
        assert!(!output.shapes.is_empty());
        output.drop_without_applying_deltas();

        state.go_settings();
        let output = context.run_ui(test_input(1080.0, 720.0), |ui| show(ui, &mut state));
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
        let output = context.run_ui(test_input(1600.0, 900.0), |ui| show(ui, &mut state));
        let text = collect_text(&output);

        assert!(text.iter().any(|line| line.contains("Resume editing")));
        assert!(text.iter().any(|line| line.contains("Temporary session")));
        output.drop_without_applying_deltas();
    }
}
