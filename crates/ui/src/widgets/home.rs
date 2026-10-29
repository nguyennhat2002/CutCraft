//! Homepage, Projects, and Settings screens.
//!
//! These screens share a teal navigation sidebar and compact header. All
//! project data is temporary UI state; nothing is read from or written to disk.

use egui::{
    pos2, vec2, Align, Align2, Color32, CursorIcon, FontId, Frame, Layout, Margin, Mesh, Panel,
    Rect, Response, RichText, ScrollArea, Sense, Shape, Stroke, StrokeKind, TextEdit, Ui,
};

use crate::app_state::{AppState, Screen};
use crate::config;
use crate::theme::{Palette, Radii};
use crate::widgets::common::{self, focus_ring, focus_ring_colored};
use crate::widgets::icons::{self, Icon};

pub fn show(ui: &mut Ui, state: &mut AppState) {
    sidebar(ui, state);
    let screen = state.screen();
    if screen != Screen::Home {
        header(ui, state);
    }

    egui::CentralPanel::no_frame()
        .frame(
            Frame::new()
                .fill(Palette::HOME_BG)
                .inner_margin(Margin::symmetric(32, 18)),
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
                .fill(Palette::HOME_SIDEBAR)
                .inner_margin(Margin::symmetric(12, 16)),
        )
        .show(ui, |ui| {
            brand(ui);
            ui.add_space(12.0);
            sidebar_profile(ui);
            ui.add_space(12.0);

            let current = state.screen();
            if nav_row(ui, Icon::Home, "Home", current == Screen::Home).clicked() {
                state.go_home();
            }
            upcoming_row(ui, Icon::Grid, "Templates");
            if nav_row(ui, Icon::Settings, "Settings", current == Screen::Settings).clicked() {
                state.go_settings();
            }

            ui.add_space(18.0);
            common::section_heading(ui, "Management");
            ui.add_space(4.0);
            if nav_row(ui, Icon::Projects, "Library", current == Screen::Projects).clicked() {
                state.go_projects();
            }
            upcoming_row(ui, Icon::Stickers, "Spaces");

            let bottom_block_height = 140.0;
            ui.add_space((ui.available_height() - bottom_block_height).max(10.0));
            sidebar_promo(ui);
            ui.add_space(8.0);
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("About CutCraft")
                            .size(10.0)
                            .color(Palette::TEXT_MUTED),
                    )
                    .frame(false),
                )
                .clicked()
            {
                state.open_about();
            }
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("v{} \u{00b7} UI shell", config::APP_VERSION))
                    .size(8.5)
                    .color(Palette::TEXT_DISABLED),
            );
        });
}

fn brand(ui: &mut Ui) {
    ui.horizontal(|ui| {
        let (mark_rect, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
        let painter = ui.painter_at(mark_rect);
        painter.rect_filled(mark_rect, Radii::CARD, Palette::ACCENT);
        painter.add(Shape::convex_polygon(
            vec![
                pos2(mark_rect.center().x - 3.0, mark_rect.center().y - 5.5),
                pos2(mark_rect.center().x - 3.0, mark_rect.center().y + 5.5),
                pos2(mark_rect.center().x + 5.5, mark_rect.center().y),
            ],
            Palette::ON_ACCENT,
            Stroke::NONE,
        ));

        ui.add_space(2.0);
        ui.vertical(|ui| {
            ui.label(
                RichText::new(config::APP_NAME)
                    .size(16.0)
                    .strong()
                    .color(Palette::TEXT_PRIMARY),
            );
            ui.label(
                RichText::new("DESKTOP EDITOR")
                    .size(7.0)
                    .strong()
                    .color(Palette::ACCENT),
            );
        });
    });
}

fn nav_row(ui: &mut Ui, icon: Icon, label: &str, selected: bool) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    let hovered = response.hovered();

    if selected {
        ui.painter()
            .rect_filled(rect, Radii::CONTROL, Palette::ACCENT_BG);
    } else if response.is_pointer_button_down_on() {
        ui.painter()
            .rect_filled(rect, Radii::CONTROL, Palette::HOME_CARD_HOVER);
    } else if hovered {
        ui.painter()
            .rect_filled(rect, Radii::CONTROL, Palette::HOME_CARD);
    }
    focus_ring(ui, &response, rect.shrink(1.0), Radii::CONTROL);

    let icon_rect =
        Rect::from_center_size(pos2(rect.left() + 16.0, rect.center().y), vec2(15.0, 15.0));
    icons::paint(
        ui.painter(),
        icon_rect,
        icon,
        if selected {
            Palette::ACCENT_LIGHT
        } else {
            Palette::TEXT_MUTED
        },
    );

    ui.painter().text(
        pos2(rect.left() + 32.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.5),
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
        Rect::from_center_size(pos2(rect.left() + 16.0, rect.center().y), vec2(14.0, 14.0));
    icons::paint(ui.painter(), icon_rect, icon, Palette::TEXT_DISABLED);
    ui.painter().text(
        pos2(rect.left() + 32.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        Palette::TEXT_DISABLED,
    );
    ui.painter().text(
        pos2(rect.right() - 2.0, rect.center().y),
        Align2::RIGHT_CENTER,
        "SOON",
        FontId::proportional(7.0),
        Palette::TEXT_DISABLED,
    );
    let _ = response.on_hover_text("Planned for a later milestone; not available yet.");
}

fn sidebar_profile(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 42.0), Sense::hover());
    ui.painter().circle_filled(
        pos2(rect.left() + 15.0, rect.center().y),
        13.0,
        Palette::ACCENT_DIM,
    );
    ui.painter().text(
        pos2(rect.left() + 15.0, rect.center().y),
        Align2::CENTER_CENTER,
        "C",
        FontId::proportional(11.0),
        Palette::ACCENT_LIGHT,
    );
    ui.painter().text(
        pos2(rect.left() + 36.0, rect.center().y - 6.0),
        Align2::LEFT_CENTER,
        "Local workspace",
        FontId::proportional(10.5),
        Palette::TEXT_SECONDARY,
    );
    ui.painter().text(
        pos2(rect.left() + 36.0, rect.center().y + 8.0),
        Align2::LEFT_CENTER,
        "No account connected",
        FontId::proportional(8.5),
        Palette::TEXT_DISABLED,
    );
}

fn sidebar_promo(ui: &mut Ui) {
    Frame::new()
        .fill(Palette::HOME_CARD)
        .stroke(Stroke::new(1.0, Palette::HOME_BORDER))
        .corner_radius(Radii::CARD)
        .inner_margin(Margin::same(11))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("CutCraft preview")
                        .size(10.5)
                        .strong()
                        .color(Palette::TEXT_PRIMARY),
                );
                ui.add_space(2.0);
                ui.add(
                    egui::Label::new(
                        RichText::new("Media, playback, and export arrive in later milestones.")
                            .size(8.5)
                            .color(Palette::TEXT_MUTED),
                    )
                    .wrap(),
                );
            });
        });
}

fn header(ui: &mut Ui, state: &mut AppState) {
    let screen = state.screen();
    let has_session = state.has_session();

    Panel::top("home_header")
        .exact_size(config::HOME_HEADER_HEIGHT)
        .frame(
            Frame::new()
                .fill(Palette::HOME_BG)
                .inner_margin(Margin::symmetric(32, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(
                    RichText::new(screen.title())
                        .size(13.5)
                        .strong()
                        .color(Palette::TEXT_PRIMARY),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("CUTCRAFT \u{00b7} LOCAL WORKSPACE")
                        .size(7.5)
                        .color(Palette::TEXT_DISABLED),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("About").size(10.0).color(Palette::TEXT_MUTED),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        state.open_about();
                    }
                    if has_session {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Temporary session")
                                .size(9.0)
                                .color(Palette::WARNING),
                        );
                    }
                });
            });
        });
}

fn home_content(ui: &mut Ui, state: &mut AppState) {
    create_banner(ui, state);
    ui.add_space(18.0);
    feature_cards(ui);
    ui.add_space(20.0);
    creative_suite(ui);
    ui.add_space(20.0);
    projects_section(ui, state);
}

fn create_banner(ui: &mut Ui, state: &mut AppState) {
    let width = ui.available_width();
    let height = if width < 760.0 { 132.0 } else { 146.0 };
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    paint_banner_background(ui, rect, response.hovered());
    if response.is_pointer_button_down_on() {
        ui.painter().rect_filled(
            rect,
            Radii::LARGE_CARD,
            Color32::from_rgba_unmultiplied(0, 0, 0, 18),
        );
    }
    focus_ring_colored(
        ui,
        &response,
        rect.shrink(1.0),
        Radii::LARGE_CARD,
        Palette::ON_ACCENT,
    );

    let painter = ui.painter_at(rect);
    let font = FontId::proportional(22.0);
    let galley = painter.layout_no_wrap(
        "Create project".to_owned(),
        font,
        Color32::from_rgb(0x07, 0x14, 0x16),
    );
    let icon_size = 27.0;
    let gap = 12.0;
    let group_width = icon_size + gap + galley.size().x;
    let start_x = rect.center().x - group_width / 2.0;
    let center_y = rect.center().y;
    let icon_rect = Rect::from_center_size(
        pos2(start_x + icon_size / 2.0, center_y),
        vec2(icon_size, icon_size),
    );
    painter.rect_filled(
        icon_rect,
        Radii::CONTROL,
        Color32::from_rgb(0x08, 0x11, 0x13),
    );
    let plus = Stroke::new(1.6, Palette::ACCENT_LIGHT);
    painter.line_segment(
        [
            pos2(icon_rect.center().x - 5.0, icon_rect.center().y),
            pos2(icon_rect.center().x + 5.0, icon_rect.center().y),
        ],
        plus,
    );
    painter.line_segment(
        [
            pos2(icon_rect.center().x, icon_rect.center().y - 5.0),
            pos2(icon_rect.center().x, icon_rect.center().y + 5.0),
        ],
        plus,
    );
    painter.galley(
        pos2(start_x + icon_size + gap, center_y - galley.size().y / 2.0),
        galley,
        Color32::from_rgb(0x07, 0x14, 0x16),
    );
    if state.has_session() {
        painter.text(
            pos2(rect.left() + 16.0, rect.bottom() - 15.0),
            Align2::LEFT_CENTER,
            "Temporary session \u{00b7} not saved to disk",
            FontId::proportional(9.0),
            Color32::from_rgba_unmultiplied(7, 20, 22, 170),
        );
    }

    let mut resume_clicked = false;
    if state.has_session() {
        let resume_rect = Rect::from_min_size(
            pos2(rect.right() - 152.0, rect.bottom() - 40.0),
            vec2(136.0, 26.0),
        );
        let chip = ui.interact(
            resume_rect,
            ui.scope_id().with("resume_banner"),
            Sense::click(),
        );
        ui.painter().rect_filled(
            resume_rect,
            Radii::CONTROL,
            Color32::from_rgba_unmultiplied(7, 18, 20, 210),
        );
        ui.painter().text(
            resume_rect.center(),
            Align2::CENTER_CENTER,
            "Resume editing",
            FontId::proportional(10.5),
            Palette::ACCENT_LIGHT,
        );
        chip.widget_info(|| {
            egui::WidgetInfo::labeled(egui::accesskit::Role::Button, true, "Resume editing")
        });
        resume_clicked = chip.clicked();
        if chip.is_pointer_button_down_on() {
            ui.painter().rect_filled(
                resume_rect,
                Radii::CONTROL,
                Color32::from_rgba_unmultiplied(16, 58, 64, 230),
            );
        } else if chip.hovered() {
            ui.painter().rect_filled(
                resume_rect,
                Radii::CONTROL,
                Color32::from_rgba_unmultiplied(30, 80, 86, 220),
            );
        }
        focus_ring(ui, &chip, resume_rect.shrink(1.0), Radii::CONTROL);
        chip.on_hover_cursor(CursorIcon::PointingHand);
    }

    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::accesskit::Role::Button, true, "Create Project")
    });
    let create_clicked = response.clicked();
    response.on_hover_cursor(CursorIcon::PointingHand);
    if resume_clicked {
        state.resume_editor();
    } else if create_clicked {
        state.create_temporary_project();
    }
}

fn paint_banner_background(ui: &Ui, rect: Rect, hovered: bool) {
    let painter = ui.painter_at(rect);
    let start = Color32::from_rgb(0x68, 0xd9, 0xe4);
    let end = Color32::from_rgb(0x17, 0xb1, 0xd0);
    let radius = 8.0_f32;

    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.center(), mix_color(start, end, 0.45));

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
        let t = ((point.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
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
        pos2(rect.left() + rect.width() * 0.22, rect.center().y + 30.0),
        rect.height() * 0.9,
        Color32::from_rgba_unmultiplied(255, 255, 255, 12),
    );
    painter.circle_filled(
        pos2(rect.right() - rect.width() * 0.12, rect.top() - 10.0),
        rect.height() * 0.8,
        Color32::from_rgba_unmultiplied(6, 92, 110, 24),
    );
    if hovered {
        painter.rect_filled(
            rect,
            Radii::LARGE_CARD,
            Color32::from_rgba_unmultiplied(255, 255, 255, 16),
        );
    }
    painter.rect_stroke(
        rect,
        Radii::LARGE_CARD,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(7, 30, 34, 60)),
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

fn section_header(ui: &mut Ui, title: &str, note: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .size(14.0)
                .strong()
                .color(Palette::TEXT_PRIMARY),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(note).size(8.0).color(Palette::TEXT_DISABLED));
        });
    });
}

fn feature_cards(ui: &mut Ui) {
    section_header(ui, "Featured tools", "UI PREVIEWS \u{00b7} NOT FUNCTIONAL");
    ui.add_space(8.0);

    let available = ui.available_width();
    let gap = 10.0;
    let compact = available < 900.0;
    let weights: [f32; 4] = if compact {
        [1.0, 1.0, 1.0, 1.0]
    } else {
        [1.45, 1.0, 1.0, 1.0]
    };
    let total_weight: f32 = weights.iter().sum();
    let usable = available - gap * 3.0;
    let heights = if compact { 118.0 } else { 128.0 };

    let features = [
        (
            Icon::Captions,
            "Auto captions",
            "Generate and style subtitles in one pass.",
            true,
        ),
        (
            Icon::Text,
            "Text to speech",
            "Turn scripts into voice tracks.",
            false,
        ),
        (
            Icon::Effects,
            "Smart effects",
            "Apply reusable looks and motion.",
            false,
        ),
        (
            Icon::Export,
            "Export presets",
            "Prepare social-ready deliverables.",
            false,
        ),
    ];

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (index, (icon, title, body, featured)) in features.iter().enumerate() {
            let width = usable * weights[index] / total_weight;
            feature_card(ui, width, heights, *icon, title, body, *featured);
        }
    });
}

fn feature_card(
    ui: &mut Ui,
    width: f32,
    height: f32,
    icon: Icon,
    title: &str,
    body: &str,
    featured: bool,
) {
    let fill = if featured {
        Color32::from_rgb(0x0e, 0x46, 0x4c)
    } else {
        Palette::HOME_CARD
    };
    let border = if featured {
        Palette::ACCENT_DIM
    } else {
        Palette::HOME_BORDER
    };
    ui.allocate_ui_with_layout(vec2(width, height), Layout::top_down(Align::Min), |ui| {
        Frame::new()
            .fill(fill)
            .stroke(Stroke::new(1.0, border))
            .corner_radius(Radii::CARD)
            .inner_margin(Margin::same(13))
            .show(ui, |ui| {
                ui.set_min_size(vec2((width - 26.0).max(60.0), height - 26.0));
                let (icon_rect, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
                ui.painter().rect_filled(
                    icon_rect,
                    Radii::CONTROL,
                    if featured {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 24)
                    } else {
                        Palette::ACCENT_BG
                    },
                );
                icons::paint(
                    ui.painter(),
                    icon_rect.shrink(7.0),
                    icon,
                    if featured {
                        Palette::ACCENT_LIGHT
                    } else {
                        Palette::ACCENT
                    },
                );
                ui.add_space(10.0);
                ui.label(RichText::new(title).size(12.0).strong().color(if featured {
                    Palette::TEXT_PRIMARY
                } else {
                    Palette::TEXT_SECONDARY
                }));
                ui.add_space(2.0);
                ui.add(
                    egui::Label::new(RichText::new(body).size(9.0).color(if featured {
                        Palette::ACCENT_LIGHT
                    } else {
                        Palette::TEXT_MUTED
                    }))
                    .wrap(),
                );
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    ui.label(RichText::new("UI PREVIEW").size(7.0).color(if featured {
                        Palette::ACCENT
                    } else {
                        Palette::TEXT_DISABLED
                    }));
                });
            });
    });
}

fn creative_suite(ui: &mut Ui) {
    section_header(ui, "Creative suite", "PLANNED \u{00b7} NOT IMPLEMENTED");
    ui.add_space(8.0);

    let available = ui.available_width();
    let gap = 10.0;
    let card_width = ((available - gap * 3.0) / 4.0).max(120.0);
    let cards = [
        (
            "Templates",
            Color32::from_rgb(0x0f, 0x3a, 0x40),
            Color32::from_rgb(0x1d, 0x6d, 0x75),
        ),
        (
            "Effects",
            Color32::from_rgb(0x18, 0x2c, 0x42),
            Color32::from_rgb(0x2b, 0x56, 0x7d),
        ),
        (
            "Transitions",
            Color32::from_rgb(0x3a, 0x2c, 0x1a),
            Color32::from_rgb(0x7a, 0x5a, 0x2c),
        ),
        (
            "Filters",
            Color32::from_rgb(0x2a, 0x20, 0x3e),
            Color32::from_rgb(0x53, 0x3d, 0x78),
        ),
    ];

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (index, (title, from, to)) in cards.iter().enumerate() {
            let (rect, _) = ui.allocate_exact_size(vec2(card_width, 112.0), Sense::hover());
            paint_suite_card(ui, rect, title, *from, *to, index);
        }
    });
}

fn paint_suite_card(ui: &Ui, rect: Rect, title: &str, from: Color32, to: Color32, index: usize) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Radii::LARGE_CARD, from);
    painter.rect_filled(
        Rect::from_min_size(
            pos2(rect.left(), rect.bottom() - 46.0),
            vec2(rect.width(), 46.0),
        ),
        Radii::LARGE_CARD,
        Color32::from_rgba_unmultiplied(8, 16, 18, 190),
    );
    painter.circle_filled(
        pos2(rect.right() - 24.0, rect.top() + 24.0),
        34.0 + index as f32 * 3.0,
        Color32::from_rgba_unmultiplied(to.r(), to.g(), to.b(), 70),
    );
    painter.circle_stroke(
        pos2(rect.left() + 26.0, rect.top() + 26.0),
        18.0,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 30)),
    );
    for step in 0..3 {
        let y = rect.top() + 52.0 + step as f32 * 9.0;
        painter.line_segment(
            [
                pos2(rect.left() + 14.0, y),
                pos2(rect.left() + 50.0 + step as f32 * 12.0, y),
            ],
            Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 18)),
        );
    }
    painter.text(
        pos2(rect.left() + 12.0, rect.bottom() - 27.0),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(11.5),
        Palette::TEXT_PRIMARY,
    );
    painter.text(
        pos2(rect.left() + 12.0, rect.bottom() - 12.0),
        Align2::LEFT_CENTER,
        "Planned",
        FontId::proportional(8.0),
        Palette::TEXT_MUTED,
    );
    painter.rect_stroke(
        rect,
        Radii::LARGE_CARD,
        Stroke::new(1.0, Palette::HOME_BORDER),
        StrokeKind::Inside,
    );
}

fn projects_section(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Projects")
                .size(15.0)
                .strong()
                .color(Palette::TEXT_PRIMARY),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let _ =
                common::disabled_icon_button(ui, Icon::List, "List view requires saved projects.");
            let _ =
                common::disabled_icon_button(ui, Icon::Grid, "Grid view requires saved projects.");
            let mut query = String::new();
            let search = ui.add_enabled(
                false,
                TextEdit::singleline(&mut query)
                    .hint_text("Search projects")
                    .desired_width(180.0),
            );
            let _ = search.on_disabled_hover_text("Search requires saved projects.");
        });
    });
    ui.add_space(8.0);

    Frame::new()
        .fill(Palette::HOME_CARD)
        .stroke(Stroke::new(1.0, Palette::HOME_BORDER))
        .corner_radius(Radii::CARD)
        .inner_margin(Margin::same(22))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.set_min_height(170.0);
            ui.vertical_centered(|ui| {
                ui.add_space(14.0);
                let (icon_rect, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
                icons::paint(
                    ui.painter(),
                    icon_rect,
                    Icon::Folder,
                    Palette::TEXT_DISABLED,
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("No projects yet")
                        .size(12.5)
                        .strong()
                        .color(Palette::TEXT_SECONDARY),
                );
                ui.add_space(2.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "Create a project to open the editor. Projects are not saved to disk \
                             in this build.",
                        )
                        .size(9.5)
                        .color(Palette::TEXT_MUTED),
                    )
                    .wrap(),
                );
                ui.add_space(10.0);
                if common::primary_button(ui, "Create Project", 140.0).clicked() {
                    state.create_temporary_project();
                }
            });
        });
}

fn current_session_card(ui: &mut Ui, state: &mut AppState, project_name: &str) {
    Frame::new()
        .fill(Palette::HOME_CARD)
        .stroke(Stroke::new(1.0, Palette::ACCENT_DIM))
        .corner_radius(Radii::CARD)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                ui.painter()
                    .rect_filled(icon_rect, Radii::CONTROL, Palette::ACCENT_BG);
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
        "Projects",
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
                .size(15.0)
                .strong()
                .color(Palette::TEXT_PRIMARY),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new("0 saved")
                    .size(8.5)
                    .color(Palette::TEXT_DISABLED),
            );
        });
    });
    ui.add_space(8.0);

    Frame::new()
        .fill(Palette::HOME_CARD)
        .stroke(Stroke::new(1.0, Palette::HOME_BORDER))
        .corner_radius(Radii::CARD)
        .inner_margin(Margin::same(24))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical_centered(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(vec2(42.0, 42.0), Sense::hover());
                icons::paint(
                    ui.painter(),
                    icon_rect,
                    Icon::Folder,
                    Palette::TEXT_DISABLED,
                );
                ui.add_space(6.0);
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
                        .size(9.5)
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
        .fill(Palette::HOME_CARD)
        .stroke(Stroke::new(1.0, Palette::HOME_BORDER))
        .corner_radius(Radii::CARD)
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new(title)
                    .size(12.5)
                    .strong()
                    .color(Palette::TEXT_PRIMARY),
            );
            ui.add_space(5.0);
            for (label, value) in rows {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(*label)
                            .size(10.0)
                            .color(Palette::TEXT_SECONDARY),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(*value)
                                .size(10.0)
                                .color(Palette::TEXT_DISABLED),
                        );
                    });
                });
            }
            ui.add_space(4.0);
            ui.add(
                egui::Label::new(RichText::new(note).size(9.0).color(Palette::TEXT_MUTED)).wrap(),
            );
        });
}

fn page_intro(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(
        RichText::new(title)
            .size(24.0)
            .strong()
            .color(Palette::TEXT_PRIMARY),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new(subtitle)
            .size(10.5)
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
    fn homepage_renders_banner_features_and_projects_empty_state() {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let mut state = AppState::default();
        let output = context.run_ui(test_input(1600.0, 900.0), |ui| show(ui, &mut state));
        let text = collect_text(&output);

        assert!(text.iter().any(|line| line == "Create Project"));
        assert!(text.iter().any(|line| line == "Featured tools"));
        assert!(text.iter().any(|line| line == "Creative suite"));
        assert!(text.iter().any(|line| line == "Projects"));
        assert!(text.iter().any(|line| line == "No projects yet"));
        output.drop_without_applying_deltas();
    }

    #[test]
    fn projects_and_settings_screens_render_without_a_session() {
        let context = egui::Context::default();
        crate::theme::configure(&context);
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
    fn homepage_shows_resume_action_for_a_temporary_session() {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let mut state = AppState::default();
        state.create_temporary_project();
        state.go_home();
        let output = context.run_ui(test_input(1600.0, 900.0), |ui| show(ui, &mut state));
        let text = collect_text(&output);
        output.drop_without_applying_deltas();

        assert!(text.iter().any(|line| line.contains("Resume editing")));
        assert!(text.iter().any(|line| line.contains("Temporary session")));
    }
}
