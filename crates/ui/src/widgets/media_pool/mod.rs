//! Left library panel with ten tool categories.
//!
//! Every category has a distinct UI shell. No media is read or generated and
//! all editing actions are disabled.

use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Layout, Margin,
    Rect, Response, RichText, ScrollArea, Sense, Stroke, StrokeKind, TextEdit, Ui,
};

use crate::editor_state::{EditorSession, EditorTool};
use crate::theme::Palette;
use crate::widgets::common;
use crate::widgets::controls;
use crate::widgets::icons::{self, Icon};

pub fn show(ui: &mut Ui, session: &mut EditorSession) {
    Frame::new()
        .fill(Palette::TOOLBAR_BG)
        .inner_margin(Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                ui.horizontal_wrapped(|ui| {
                    for tool in EditorTool::ALL {
                        if category_tab(ui, tool, session.active_tool == tool).clicked() {
                            session.active_tool = tool;
                        }
                    }
                });
            });
        });
    ui.separator();

    let footer_height = if session.active_tool == EditorTool::Captions {
        42.0
    } else {
        0.0
    };
    let body_height = (ui.available_height() - footer_height).max(0.0);

    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), body_height),
        Layout::top_down(Align::Min),
        |ui| {
            let _ = ScrollArea::vertical()
                .id_salt("library_scroll")
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    Frame::new()
                        .inner_margin(Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            match session.active_tool {
                    EditorTool::Media => media_body(ui),
                    EditorTool::Audio => audio_body(ui),
                    EditorTool::Text => text_body(ui),
                    EditorTool::Stickers => locked_body(
                        ui,
                        Icon::Stickers,
                        "Sticker library",
                        "Sticker import and rendering are not implemented in this build.",
                        &["Stickers", "Shapes", "Emoji", "Frames", "Masks", "Decals"],
                    ),
                    EditorTool::Effects => locked_body(
                        ui,
                        Icon::Effects,
                        "Effects library",
                        "Video effects and filters are not implemented in this build.",
                        &["Effects", "Glow", "Blur", "Shake", "Particles", "Distort"],
                    ),
                    EditorTool::Transitions => locked_body(
                        ui,
                        Icon::Transitions,
                        "Transition library",
                        "Clip transitions are not implemented in this build.",
                        &["Dissolve", "Wipe", "Slide", "Zoom", "Spin", "Fade"],
                    ),
                    EditorTool::Captions => captions_body(ui),
                    EditorTool::Filters => locked_body(
                        ui,
                        Icon::Filters,
                        "Filter library",
                        "Color filters are not implemented in this build.",
                        &["Filters", "Warm", "Cool", "Mono", "Vivid", "Soft"],
                    ),
                    EditorTool::Adjustment => adjustment_body(ui),
                    EditorTool::Templates => locked_body(
                        ui,
                        Icon::Grid,
                        "Template library",
                        "Project and text templates are not implemented in this build.",
                        &["Templates", "Vertical", "Square", "Text", "Intro", "Outro"],
                    ),
                            }
                        });
                });
        },
    );

    if session.active_tool == EditorTool::Captions {
        Frame::new()
            .inner_margin(Margin::symmetric(10, 4))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                caption_footer(ui);
            });
    }
}

fn media_body(ui: &mut Ui) {
    subtab_row(ui, &["Local", "Stock", "Cloud"]);
    ui.add_space(6.0);
    disabled_search(ui, "Search media");
    ui.add_space(5.0);
    let _ =
        common::disabled_action_button(ui, "+  Import Media", "Media import is not implemented.");
    ui.add_space(8.0);
    media_grid_empty(ui);
    ui.add_space(6.0);
    ui.add(
        egui::Label::new(
            RichText::new("No files are read and no video is decoded in this build.")
                .size(9.5)
                .color(Palette::TEXT_DISABLED),
        )
        .wrap(),
    );
}

fn audio_body(ui: &mut Ui) {
    subtab_row(ui, &["Music", "Sound effects", "Voiceover"]);
    ui.add_space(6.0);
    disabled_search(ui, "Search audio");
    ui.add_space(5.0);
    let _ =
        common::disabled_action_button(ui, "+  Import Audio", "Audio import is not implemented.");
    ui.add_space(8.0);
    common::compact_empty_state(
        ui,
        Icon::Audio,
        "No audio imported",
        "Music, sound effects, and voiceover clips will appear here once media support exists.",
    );
    ui.add_space(6.0);
    waveform_placeholder(ui);
}

fn text_body(ui: &mut Ui) {
    subtab_row(ui, &["Basic", "Templates"]);
    ui.add_space(6.0);
    let _ = common::disabled_action_button(ui, "+  Add Text", "Text editing is not implemented.");
    ui.add_space(8.0);
    Frame::new()
        .fill(Palette::CARD_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Aa")
                        .size(34.0)
                        .strong()
                        .color(Palette::TEXT_MUTED),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Text and subtitles")
                        .size(12.0)
                        .strong()
                        .color(Palette::TEXT_SECONDARY),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "Titles, captions, and text styling are not implemented. \
                             This panel is a layout placeholder.",
                        )
                        .size(9.5)
                        .color(Palette::TEXT_MUTED),
                    )
                    .wrap(),
                );
            });
        });
}

fn captions_body(ui: &mut Ui) {
    let nav_width = (ui.available_width() * 0.22).clamp(82.0, 108.0);
    let body_height = ui.available_height().max(180.0);
    ui.horizontal_top(|ui| {
        Frame::new()
            .fill(Palette::PANEL_BG)
            .inner_margin(Margin::symmetric(4, 4))
            .show(ui, |ui| {
                ui.set_min_height((body_height - 8.0).max(0.0));
                ui.allocate_ui_with_layout(
                    vec2((nav_width - 8.0).max(60.0), (body_height - 8.0).max(0.0)),
                    Layout::top_down(Align::Min),
                    caption_nav,
                );
            });
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.label(
                RichText::new("Spoken language")
                    .size(10.0)
                    .color(Palette::TEXT_SECONDARY),
            );
            ui.add_space(3.0);
            let width = ui.available_width().max(120.0);
            let _ = controls::disabled_select(ui, "English", width);
            ui.add_space(8.0);

            ui.label(
                RichText::new("Bilingual captions")
                    .size(10.0)
                    .color(Palette::TEXT_SECONDARY),
            );
            ui.add_space(3.0);
            let width = ui.available_width().max(120.0);
            let _ = controls::disabled_select(ui, "None", width);
            ui.add_space(10.0);

            let _ = controls::disabled_toggle(ui, "Identify filler words", false);
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "Caption generation is a visual placeholder. No speech recognition or \
                         media processing is available in this build.",
                    )
                    .size(9.0)
                    .color(Palette::TEXT_DISABLED),
                )
                .wrap(),
            );
        });
    });
}

fn caption_nav(ui: &mut Ui) {
    ui.set_width(ui.available_width());
    let items = ["Auto captions", "Templates", "Auto lyrics", "Add captions"];
    for (index, label) in items.iter().enumerate() {
        let selected = index == 0;
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
        if selected {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(3), Palette::ACCENT_BG);
            ui.painter().rect_filled(
                Rect::from_min_size(rect.min, vec2(2.0, rect.height())),
                CornerRadius::same(1),
                Palette::ACCENT,
            );
        }
        ui.painter().text(
            pos2(rect.left() + 10.0, rect.center().y),
            Align2::LEFT_CENTER,
            *label,
            FontId::proportional(10.0),
            if selected {
                Palette::ACCENT
            } else {
                Palette::TEXT_MUTED
            },
        );
        let _ = response.on_hover_text("Caption options are not implemented.");
    }
}

fn caption_footer(ui: &mut Ui) {
    ui.separator();
    ui.horizontal_centered(|ui| {
        let _ =
            common::disabled_icon_button(ui, Icon::Close, "Deleting captions is not implemented.");
        ui.label(
            RichText::new("Delete current captions")
                .size(9.0)
                .color(Palette::TEXT_DISABLED),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let generate = ui.add_enabled(
                false,
                egui::Button::new(
                    RichText::new("Generate")
                        .size(10.0)
                        .color(Palette::TEXT_DISABLED),
                )
                .fill(Palette::ACCENT_BG)
                .min_size(vec2(74.0, 24.0)),
            );
            let _ = generate.on_disabled_hover_text("Caption generation is not implemented.");
        });
    });
}

fn adjustment_body(ui: &mut Ui) {
    controls::section_card(ui, "Light", |ui| {
        let _ = controls::disabled_slider(ui, "Brightness", 0.0, -100.0..=100.0);
        let _ = controls::disabled_slider(ui, "Contrast", 0.0, -100.0..=100.0);
        let _ = controls::disabled_slider(ui, "Exposure", 0.0, -100.0..=100.0);
    });
    ui.add_space(6.0);
    controls::section_card(ui, "Color", |ui| {
        let _ = controls::disabled_slider(ui, "Saturation", 0.0, -100.0..=100.0);
        let _ = controls::disabled_slider(ui, "Temperature", 0.0, -100.0..=100.0);
        let _ = controls::disabled_slider(ui, "Tint", 0.0, -100.0..=100.0);
    });
    ui.add_space(6.0);
    let reset = ui.add_enabled(false, egui::Button::new("Reset adjustments"));
    let _ = reset.on_disabled_hover_text("Adjustment editing is not implemented.");
}

fn locked_body(ui: &mut Ui, icon: Icon, title: &str, body: &str, tile_labels: &[&str]) {
    disabled_search(ui, "Search library");
    ui.add_space(6.0);
    let _ = common::disabled_action_button(ui, "Browse library", body);
    ui.add_space(8.0);

    let available = ui.available_width();
    let columns = if available < 320.0 { 2 } else { 3 };
    let gap = 6.0;
    let tile_width = ((available - gap * (columns - 1) as f32) / columns as f32).max(60.0);

    for row in tile_labels.chunks(columns) {
        ui.horizontal(|ui| {
            for label in row {
                locked_tile(ui, tile_width, icon, label);
                ui.add_space(gap);
            }
        });
        ui.add_space(4.0);
    }

    ui.add_space(4.0);
    common::compact_empty_state(ui, icon, title, body);
}

fn locked_tile(ui: &mut Ui, width: f32, icon: Icon, label: &str) {
    Frame::new()
        .fill(Palette::CARD_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::same(8))
        .show(ui, |ui| {
            ui.set_min_size(vec2(width - 16.0, 64.0));
            let (icon_rect, _) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
            icons::paint(ui.painter(), icon_rect, icon, Palette::TEXT_DISABLED);
            ui.label(RichText::new(label).size(9.5).color(Palette::TEXT_DISABLED));
            ui.label(
                RichText::new("Unavailable")
                    .size(8.0)
                    .color(Palette::TEXT_DISABLED),
            );
        });
}

fn category_tab(ui: &mut Ui, tool: EditorTool, selected: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(50.0, 38.0), Sense::click());
    let hovered = response.hovered();
    let painter = ui.painter_at(rect);

    if selected {
        painter.rect_filled(
            Rect::from_min_max(
                pos2(rect.left() + 9.0, rect.bottom() - 2.0),
                pos2(rect.right() - 9.0, rect.bottom()),
            ),
            CornerRadius::same(1),
            Palette::ACCENT,
        );
    } else if hovered {
        painter.rect_filled(rect, CornerRadius::same(3), Palette::WIDGET_BG);
    }

    let icon_rect =
        Rect::from_center_size(pos2(rect.center().x, rect.top() + 12.0), vec2(14.0, 14.0));
    icons::paint(
        &painter,
        icon_rect,
        tool_icon(tool),
        if selected {
            Palette::ACCENT
        } else if hovered {
            Palette::TEXT_PRIMARY
        } else {
            Palette::TEXT_MUTED
        },
    );
    painter.text(
        pos2(rect.center().x, rect.top() + 28.0),
        Align2::CENTER_CENTER,
        tool.label(),
        FontId::proportional(9.0),
        if selected {
            Palette::ACCENT
        } else {
            Palette::TEXT_MUTED
        },
    );

    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tool.label())
}

fn tool_icon(tool: EditorTool) -> Icon {
    match tool {
        EditorTool::Media => Icon::Media,
        EditorTool::Audio => Icon::Audio,
        EditorTool::Text => Icon::Text,
        EditorTool::Stickers => Icon::Stickers,
        EditorTool::Effects => Icon::Effects,
        EditorTool::Transitions => Icon::Transitions,
        EditorTool::Captions => Icon::Captions,
        EditorTool::Filters => Icon::Filters,
        EditorTool::Adjustment => Icon::Settings,
        EditorTool::Templates => Icon::Grid,
    }
}

fn subtab_row(ui: &mut Ui, labels: &[&str]) {
    ui.horizontal_wrapped(|ui| {
        for (index, label) in labels.iter().enumerate() {
            let selected = index == 0;
            let response = ui.add_enabled(
                false,
                egui::Button::new(RichText::new(*label).size(9.5).color(if selected {
                    Palette::ACCENT
                } else {
                    Palette::TEXT_MUTED
                }))
                .selected(selected)
                .min_size(vec2(0.0, 22.0)),
            );
            let _ = response.on_disabled_hover_text("Library filtering is not implemented yet.");
        }
    });
}

fn disabled_search(ui: &mut Ui, hint: &str) {
    let mut search = String::new();
    let response = ui.add_enabled(
        false,
        TextEdit::singleline(&mut search)
            .hint_text(hint)
            .desired_width(ui.available_width()),
    );
    let _ = response.on_disabled_hover_text("Search is not implemented in this build.");
}

fn media_grid_empty(ui: &mut Ui) {
    Frame::new()
        .fill(Palette::APP_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let height = ui.available_height().clamp(180.0, 300.0);
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            let painter = ui.painter_at(rect);

            let gap = 8.0;
            let columns = 3;
            let rows = 2;
            let tile_width = (rect.width() - gap * (columns as f32 - 1.0)) / columns as f32;
            let tile_height =
                (rect.height() - gap * (rows as f32 - 1.0) - 24.0) / rows as f32 - 6.0;

            for row in 0..rows {
                for column in 0..columns {
                    let tile = Rect::from_min_size(
                        pos2(
                            rect.left() + column as f32 * (tile_width + gap),
                            rect.top() + row as f32 * (tile_height + gap),
                        ),
                        vec2(tile_width, tile_height),
                    );
                    painter.rect_filled(tile, CornerRadius::same(4), Palette::CARD_BG);
                    painter.rect_stroke(
                        tile,
                        CornerRadius::same(4),
                        Stroke::new(1.0, Palette::BORDER_SUBTLE),
                        StrokeKind::Inside,
                    );
                    let icon_center = pos2(tile.center().x, tile.center().y - 4.0);
                    let icon_rect = Rect::from_center_size(icon_center, vec2(18.0, 18.0));
                    icons::paint(&painter, icon_rect, Icon::Media, Palette::TEXT_DISABLED);
                }
            }

            let overlay =
                Rect::from_center_size(rect.center(), vec2((rect.width() - 32.0).min(280.0), 76.0));
            painter.rect_filled(
                overlay,
                CornerRadius::same(5),
                Color32::from_rgba_unmultiplied(24, 24, 25, 240),
            );
            painter.rect_stroke(
                overlay,
                CornerRadius::same(5),
                Stroke::new(1.0, Palette::BORDER),
                StrokeKind::Inside,
            );
            painter.text(
                overlay.center() - vec2(0.0, 10.0),
                Align2::CENTER_CENTER,
                "No media imported",
                FontId::proportional(12.0),
                Palette::TEXT_SECONDARY,
            );
            painter.text(
                overlay.center() + vec2(0.0, 10.0),
                Align2::CENTER_CENTER,
                "Import is not available in this build",
                FontId::proportional(9.0),
                Palette::TEXT_DISABLED,
            );
        });
}

fn waveform_placeholder(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 76.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(4), Palette::APP_BG);
    painter.rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, Palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );
    let bars = 26;
    let step = rect.width() / bars as f32;
    for index in 0..bars {
        let height = ((index * 37 % 23) as f32 / 23.0).max(0.12);
        let x = rect.left() + index as f32 * step + step * 0.35;
        let half = rect.height() * height * 0.32;
        painter.line_segment(
            [
                pos2(x, rect.center().y - half),
                pos2(x, rect.center().y + half),
            ],
            Stroke::new(1.0, Palette::WIDGET_HOVER),
        );
    }
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        "No audio",
        FontId::proportional(10.0),
        Palette::TEXT_DISABLED,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_library_category_renders_without_media() {
        let context = egui::Context::default();

        for tool in EditorTool::ALL {
            let mut session = EditorSession {
                active_tool: tool,
                ..EditorSession::default()
            };
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(520.0, 620.0))),
                ..Default::default()
            };
            let output = context.run_ui(input, |ui| show(ui, &mut session));
            assert!(!output.shapes.is_empty());
            output.drop_without_applying_deltas();
        }
    }
}
