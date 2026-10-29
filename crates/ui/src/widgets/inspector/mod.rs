//! Inspector panel with reference-style caption styling and clip properties.
//!
//! Caption controls edit only the transient session demo state. They do not
//! touch project media, which does not exist yet. Clip property views remain
//! disabled placeholders.

use egui::{
    pos2, vec2, Align2, CornerRadius, CursorIcon, FontFamily, FontId, Frame, Margin, Rect,
    RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui,
};

use crate::config;
use crate::editor_state::{CaptionAlignment, CaptionCase, CaptionTab, EditorSession, InspectorTab};
use crate::theme::{Palette, Radii};
use crate::widgets::common;
use crate::widgets::common::focus_ring;
use crate::widgets::controls;
use crate::widgets::icons::{self, Icon};

pub fn show(ui: &mut Ui, session: &mut EditorSession) {
    tab_strip(ui, session);
    preview_note(ui);

    let _ = ScrollArea::vertical()
        .id_salt("inspector_scroll")
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .show(ui, |ui| {
            Frame::new()
                .inner_margin(Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    match session.inspector_tab {
                        InspectorTab::Captions => captions_properties(ui, session),
                        InspectorTab::Text => text_properties(ui),
                        InspectorTab::Animation => animation_properties(ui),
                        InspectorTab::Video => video_properties(ui),
                        InspectorTab::Audio => audio_properties(ui),
                        InspectorTab::Basic => basic_properties(ui),
                    }
                    ui.add_space(10.0);
                });
        });
}

fn tab_strip(ui: &mut Ui, session: &mut EditorSession) {
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), config::EDITOR_COLUMN_HEADER_HEIGHT),
        Sense::hover(),
    );
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, Palette::TOOLBAR_BG);

    let count = InspectorTab::ALL.len() as f32;
    let tab_width = rect.width() / count;
    for (index, tab) in InspectorTab::ALL.iter().enumerate() {
        let tab_rect = Rect::from_min_size(
            pos2(rect.left() + index as f32 * tab_width, rect.top()),
            vec2(tab_width, rect.height()),
        );
        let response = ui.interact(
            tab_rect,
            ui.scope_id().with(("inspector_tab", index)),
            Sense::click(),
        );
        let selected = session.inspector_tab == *tab;

        if response.is_pointer_button_down_on() && !selected {
            ui.painter().rect_filled(
                tab_rect.shrink2(vec2(2.0, 5.0)),
                Radii::SMALL,
                Palette::WIDGET_ACTIVE,
            );
        } else if response.hovered() && !selected {
            ui.painter().rect_filled(
                tab_rect.shrink2(vec2(2.0, 5.0)),
                Radii::SMALL,
                Palette::WIDGET_BG,
            );
        }
        focus_ring(
            ui,
            &response,
            tab_rect.shrink2(vec2(2.0, 4.0)),
            Radii::SMALL,
        );
        ui.painter().text(
            tab_rect.center() - vec2(0.0, 2.0),
            Align2::CENTER_CENTER,
            tab.label(),
            FontId::proportional(10.0),
            if selected {
                Palette::TEXT_PRIMARY
            } else {
                Palette::TEXT_SECONDARY
            },
        );
        if selected {
            ui.painter().rect_filled(
                Rect::from_min_size(
                    pos2(tab_rect.left() + 10.0, tab_rect.bottom() - 2.0),
                    vec2((tab_rect.width() - 20.0).max(8.0), 2.0),
                ),
                CornerRadius::ZERO,
                Palette::ACCENT,
            );
        }
        if response.clicked() {
            session.inspector_tab = *tab;
        }
        response.on_hover_cursor(CursorIcon::PointingHand);
    }
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, Palette::BORDER_SUBTLE),
    );
}

fn preview_note(ui: &mut Ui) {
    Frame::new()
        .fill(Palette::PANEL_BG)
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                common::badge(ui, "UI preview", Palette::ACCENT);
                ui.add_space(6.0);
                ui.label(
                    RichText::new("No clip selected · changes stay in this session")
                        .size(9.5)
                        .color(Palette::TEXT_MUTED),
                );
            });
        });
}

fn captions_properties(ui: &mut Ui, session: &mut EditorSession) {
    caption_tabs(ui, session);
    ui.add_space(7.0);
    checkbox_row(ui, "Apply to all", &mut session.caption_apply_all);
    ui.add_space(5.0);

    match session.caption_tab {
        CaptionTab::Basic => caption_basic(ui, session),
        CaptionTab::Templates => caption_placeholder(
            ui,
            Icon::Captions,
            "Caption templates",
            "Template thumbnails are not implemented in this build.",
        ),
        CaptionTab::Bubble => caption_placeholder(
            ui,
            Icon::Text,
            "Bubble styles",
            "Speech-bubble styling is not implemented in this build.",
        ),
        CaptionTab::Effects => caption_placeholder(
            ui,
            Icon::Effects,
            "Caption effects",
            "Animated caption effects are not implemented in this build.",
        ),
    }
}

fn caption_tabs(ui: &mut Ui, session: &mut EditorSession) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, Radii::CONTROL, Palette::TRACK_BG);
    let count = CaptionTab::ALL.len() as f32;
    let width = rect.width() / count;
    for (index, tab) in CaptionTab::ALL.iter().enumerate() {
        let seg_rect = Rect::from_min_size(
            pos2(rect.left() + index as f32 * width, rect.top()),
            vec2(width, rect.height()),
        );
        let response = ui.interact(
            seg_rect,
            ui.scope_id().with(("caption_tab", index)),
            Sense::click(),
        );
        let selected = session.caption_tab == *tab;
        if response.is_pointer_button_down_on() {
            ui.painter()
                .rect_filled(seg_rect.shrink(2.0), Radii::SMALL, Palette::WIDGET_ACTIVE);
        } else if selected {
            ui.painter()
                .rect_filled(seg_rect.shrink(2.0), Radii::SMALL, Palette::WIDGET_BG);
        } else if response.hovered() {
            ui.painter()
                .rect_filled(seg_rect.shrink(2.0), Radii::SMALL, Palette::CARD_HOVER);
        }
        focus_ring(ui, &response, seg_rect.shrink(1.0), Radii::SMALL);
        ui.painter().text(
            seg_rect.center(),
            Align2::CENTER_CENTER,
            tab.label(),
            FontId::proportional(9.5),
            if selected {
                Palette::TEXT_PRIMARY
            } else {
                Palette::TEXT_MUTED
            },
        );
        if response.clicked() {
            session.caption_tab = *tab;
        }
        response.on_hover_cursor(CursorIcon::PointingHand);
    }
}

fn caption_basic(ui: &mut Ui, session: &mut EditorSession) {
    row(ui, "Font", |ui| {
        let width = ui.available_width().max(80.0);
        let _ = controls::disabled_select(ui, "System", width);
    });
    row(ui, "Size", |ui| {
        ui.spacing_mut().slider_width = (ui.available_width() - 52.0).max(56.0);
        let slider = egui::Slider::new(&mut session.caption_font_size, 6.0..=72.0)
            .show_value(false)
            .trailing_fill(true);
        let _ = ui.add(slider);
        let _ = ui.add(
            egui::DragValue::new(&mut session.caption_font_size)
                .speed(0.5)
                .range(6.0..=72.0)
                .fixed_decimals(0),
        );
    });
    row(ui, "Pattern", |ui| {
        if style_toggle(ui, RichText::new("B").strong(), session.caption_bold).clicked() {
            session.caption_bold = !session.caption_bold;
        }
        if style_toggle(
            ui,
            RichText::new("U").underline(),
            session.caption_underline,
        )
        .clicked()
        {
            session.caption_underline = !session.caption_underline;
        }
        if style_toggle(ui, RichText::new("I").italics(), session.caption_italic).clicked() {
            session.caption_italic = !session.caption_italic;
        }
    });
    row(ui, "Case", |ui| {
        for case in CaptionCase::ALL {
            if case_toggle(ui, case, session.caption_case == case).clicked() {
                session.caption_case = case;
            }
        }
    });
    row(ui, "Color", |ui| {
        let mut rgb = session.caption_color;
        if egui::color_picker::color_edit_button_srgb(ui, &mut rgb).changed() {
            session.caption_color = rgb;
        }
        ui.label(
            RichText::new(format!(
                "#{:02X}{:02X}{:02X}",
                session.caption_color[0], session.caption_color[1], session.caption_color[2]
            ))
            .monospace()
            .size(9.5)
            .color(Palette::TEXT_MUTED),
        );
    });
    row(ui, "Character", |ui| {
        let _ = ui.add(
            egui::DragValue::new(&mut session.caption_character_spacing)
                .speed(0.1)
                .range(-20.0..=50.0)
                .fixed_decimals(1),
        );
        ui.label(RichText::new("px").size(9.5).color(Palette::TEXT_MUTED));
    });
    row(ui, "Line", |ui| {
        let _ = ui.add(
            egui::DragValue::new(&mut session.caption_line_spacing)
                .speed(0.1)
                .range(-20.0..=50.0)
                .fixed_decimals(1),
        );
        ui.label(RichText::new("px").size(9.5).color(Palette::TEXT_MUTED));
    });
    row(ui, "Alignment", |ui| {
        for alignment in CaptionAlignment::ALL {
            if alignment_button(ui, alignment, session.caption_alignment == alignment).clicked() {
                session.caption_alignment = alignment;
            }
        }
    });

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Preset style")
                .size(10.5)
                .strong()
                .color(Palette::TEXT_SECONDARY),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let _ = ui
                .add_enabled(
                    false,
                    egui::Button::new(
                        RichText::new("Save as preset")
                            .size(9.5)
                            .color(Palette::TEXT_MUTED),
                    )
                    .frame(false),
                )
                .on_disabled_hover_text(config::UNIMPLEMENTED_HINT);
        });
    });
    ui.add_space(4.0);
    style_tiles(ui, session);
}

fn style_tiles(ui: &mut Ui, session: &mut EditorSession) {
    let gap = 4.0;
    let columns = 4.0;
    let tile_width = ((ui.available_width() - gap * (columns - 1.0)) / columns).max(32.0);
    let selected = session.caption_style;
    for row_index in 0..2 {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for column in 0..4 {
                let index = row_index * 4 + column;
                let (rect, response) =
                    ui.allocate_exact_size(vec2(tile_width, 42.0), Sense::click());
                let hovered = response.hovered();
                let pressed = response.is_pointer_button_down_on();
                let active = index == selected;
                let painter = ui.painter();
                painter.rect_filled(
                    rect,
                    Radii::CONTROL,
                    if active {
                        Palette::ACCENT_BG
                    } else if pressed {
                        Palette::WIDGET_ACTIVE
                    } else if hovered {
                        Palette::CARD_HOVER
                    } else {
                        Palette::WIDGET_BG
                    },
                );
                painter.rect_stroke(
                    rect,
                    Radii::CONTROL,
                    Stroke::new(
                        1.0,
                        if active {
                            Palette::ACCENT
                        } else if hovered {
                            Palette::BORDER
                        } else {
                            Palette::BORDER_SUBTLE
                        },
                    ),
                    StrokeKind::Inside,
                );
                focus_ring(ui, &response, rect.shrink(1.0), Radii::CONTROL);
                let family = if index >= 4 {
                    FontFamily::Monospace
                } else {
                    FontFamily::Proportional
                };
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "Aa",
                    FontId::new(17.0, family),
                    if active {
                        Palette::ACCENT_LIGHT
                    } else {
                        Palette::TEXT_SECONDARY
                    },
                );
                if response.clicked() {
                    session.caption_style = index;
                }
                response
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .on_hover_text("Original placeholder style · preview only");
            }
        });
        if row_index == 0 {
            ui.add_space(gap);
        }
    }
}

fn caption_placeholder(ui: &mut Ui, icon: Icon, title: &str, body: &str) {
    ui.add_space(2.0);
    Frame::new()
        .fill(Palette::TRACK_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
        .corner_radius(Radii::CARD)
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical_centered(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::hover());
                icons::paint(ui.painter(), icon_rect, icon, Palette::TEXT_DISABLED);
                ui.add_space(6.0);
                ui.label(
                    RichText::new(title)
                        .size(11.5)
                        .strong()
                        .color(Palette::TEXT_SECONDARY),
                );
                ui.add_space(2.0);
                ui.add(
                    egui::Label::new(RichText::new(body).size(9.5).color(Palette::TEXT_MUTED))
                        .wrap(),
                );
            });
        });
}

fn row(ui: &mut Ui, label: &str, add_control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            vec2(62.0, 22.0),
            egui::Label::new(
                RichText::new(label)
                    .size(10.0)
                    .color(Palette::TEXT_SECONDARY),
            )
            .truncate(),
        );
        add_control(ui);
    });
}

fn checkbox_row(ui: &mut Ui, label: &str, checked: &mut bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::click());
    let box_rect =
        Rect::from_center_size(pos2(rect.left() + 8.0, rect.center().y), vec2(13.0, 13.0));
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(
            rect.shrink(1.0),
            Radii::SMALL,
            if response.is_pointer_button_down_on() {
                Palette::WIDGET_ACTIVE
            } else {
                Palette::WIDGET_BG
            },
        );
    }
    focus_ring(ui, &response, rect.shrink(1.0), Radii::SMALL);
    painter.rect_filled(
        box_rect,
        Radii::SMALL,
        if *checked {
            Palette::ACCENT
        } else {
            Palette::WIDGET_BG
        },
    );
    painter.rect_stroke(
        box_rect,
        Radii::SMALL,
        Stroke::new(
            1.0,
            if *checked {
                Palette::ACCENT
            } else {
                Palette::BORDER
            },
        ),
        StrokeKind::Inside,
    );
    if *checked {
        let stroke = Stroke::new(1.4, Palette::ON_ACCENT);
        painter.line_segment(
            [
                pos2(box_rect.left() + 3.0, box_rect.center().y),
                pos2(box_rect.center().x - 1.0, box_rect.bottom() - 3.5),
            ],
            stroke,
        );
        painter.line_segment(
            [
                pos2(box_rect.center().x - 1.0, box_rect.bottom() - 3.5),
                pos2(box_rect.right() - 2.5, box_rect.top() + 4.0),
            ],
            stroke,
        );
    }
    painter.text(
        pos2(box_rect.right() + 7.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(10.0),
        Palette::TEXT_SECONDARY,
    );
    if response.clicked() {
        *checked = !*checked;
    }
    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn style_toggle(ui: &mut Ui, text: RichText, selected: bool) -> egui::Response {
    let button = egui::Button::new(text.size(11.5))
        .selected(selected)
        .corner_radius(Radii::SMALL)
        .min_size(vec2(28.0, 22.0));
    ui.add(button).on_hover_cursor(CursorIcon::PointingHand)
}

fn case_toggle(ui: &mut Ui, case: CaptionCase, selected: bool) -> egui::Response {
    let button = egui::Button::new(RichText::new(case.label()).size(10.5))
        .selected(selected)
        .corner_radius(Radii::SMALL)
        .min_size(vec2(30.0, 22.0));
    ui.add(button).on_hover_cursor(CursorIcon::PointingHand)
}

fn alignment_button(ui: &mut Ui, alignment: CaptionAlignment, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(32.0, 22.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Radii::SMALL,
        if selected {
            Palette::ACCENT_BG
        } else if response.is_pointer_button_down_on() {
            Palette::WIDGET_ACTIVE
        } else if response.hovered() {
            Palette::WIDGET_HOVER
        } else {
            Palette::WIDGET_BG
        },
    );
    painter.rect_stroke(
        rect,
        Radii::SMALL,
        Stroke::new(
            1.0,
            if selected {
                Palette::ACCENT
            } else if response.hovered() {
                Palette::BORDER
            } else {
                Palette::BORDER_SUBTLE
            },
        ),
        StrokeKind::Inside,
    );
    focus_ring(ui, &response, rect.shrink(1.0), Radii::SMALL);
    let color = if selected {
        Palette::ACCENT_LIGHT
    } else {
        Palette::TEXT_SECONDARY
    };
    let stroke = Stroke::new(1.0, color);
    let left = rect.left() + 7.0;
    let right = rect.right() - 7.0;
    let widths = [13.0, 8.0, 11.0];
    for (index, width) in widths.iter().enumerate() {
        let y = rect.center().y - 4.0 + index as f32 * 4.0;
        let (x0, x1) = match alignment {
            CaptionAlignment::Left => (left, left + width),
            CaptionAlignment::Center => {
                let center = rect.center().x;
                (center - width / 2.0, center + width / 2.0)
            }
            CaptionAlignment::Right => (right - width, right),
            CaptionAlignment::Justify => {
                let segment = if index == 2 { *width } else { right - left };
                (left, left + segment)
            }
        };
        painter.line_segment([pos2(x0, y), pos2(x1, y)], stroke);
    }
    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn video_properties(ui: &mut Ui) {
    controls::section_card(ui, "Transform", |ui| {
        controls::control_row(ui, "Position", |ui| {
            let field_width = (ui.available_width() * 0.36).clamp(56.0, 120.0);
            let _ = controls::disabled_value(ui, "X 0.0", field_width);
            let _ = controls::disabled_value(ui, "Y 0.0", field_width);
        });
        controls::control_row(ui, "Scale", |ui| {
            let _ = controls::disabled_value(ui, "100%", 70.0);
        });
        controls::control_row(ui, "Rotation", |ui| {
            let _ = controls::disabled_value(ui, "0.0°", 70.0);
        });
        controls::control_row(ui, "Alignment", |ui| {
            let _ = controls::alignment_grid(ui, 4);
        });
        let _ = controls::disabled_slider(ui, "Opacity", 100.0, 0.0..=100.0);
    });
    ui.add_space(6.0);
    controls::section_card(ui, "Video", |ui| {
        controls::control_row(ui, "Blend mode", |ui| {
            let width = ui.available_width().clamp(80.0, 180.0);
            let _ = controls::disabled_select(ui, "Normal", width);
        });
        controls::control_row(ui, "Speed", |ui| {
            let _ = controls::disabled_value(ui, "1.00x", 70.0);
        });
        controls::control_row(ui, "Color", |ui| {
            let _ = controls::color_swatch(ui, Palette::WIDGET_HOVER);
            ui.label(
                RichText::new("#FFFFFF")
                    .monospace()
                    .size(9.5)
                    .color(Palette::TEXT_DISABLED),
            );
        });
    });
}

fn audio_properties(ui: &mut Ui) {
    controls::section_card(ui, "Audio", |ui| {
        let _ = controls::disabled_slider(ui, "Volume", 100.0, 0.0..=200.0);
        controls::control_row(ui, "Fade in", |ui| {
            let _ = controls::disabled_value(ui, "0.0s", 64.0);
        });
        controls::control_row(ui, "Fade out", |ui| {
            let _ = controls::disabled_value(ui, "0.0s", 64.0);
        });
        let _ = controls::disabled_toggle(ui, "Noise reduction", false);
    });
    ui.add_space(6.0);
    controls::section_card(ui, "Channels", |ui| {
        controls::control_row(ui, "Output", |ui| {
            let width = ui.available_width().clamp(90.0, 200.0);
            let _ = controls::disabled_select(ui, "Stereo", width);
        });
        controls::control_row(ui, "Mix", |ui| {
            let width = ui.available_width().clamp(90.0, 200.0);
            let _ = controls::disabled_select(ui, "Original", width);
        });
    });
}

fn text_properties(ui: &mut Ui) {
    controls::section_card(ui, "Text", |ui| {
        controls::control_row(ui, "Font", |ui| {
            let width = ui.available_width().clamp(90.0, 200.0);
            let _ = controls::disabled_select(ui, "Proportional", width);
        });
        controls::control_row(ui, "Size", |ui| {
            let _ = controls::disabled_value(ui, "24 pt", 64.0);
        });
        controls::control_row(ui, "Color", |ui| {
            let _ = controls::color_swatch(ui, Palette::TEXT_PRIMARY);
            ui.label(
                RichText::new("#FFFFFF")
                    .monospace()
                    .size(9.5)
                    .color(Palette::TEXT_DISABLED),
            );
        });
        controls::control_row(ui, "Alignment", |ui| {
            let _ = controls::alignment_grid(ui, 4);
        });
    });
    ui.add_space(6.0);
    controls::section_card(ui, "Spacing", |ui| {
        let _ = controls::disabled_slider(ui, "Letter spacing", 0.0, -10.0..=20.0);
        let _ = controls::disabled_slider(ui, "Line height", 100.0, 50.0..=200.0);
    });
}

fn animation_properties(ui: &mut Ui) {
    controls::section_card(ui, "Animation", |ui| {
        controls::control_row(ui, "In", |ui| {
            let width = ui.available_width().clamp(90.0, 200.0);
            let _ = controls::disabled_select(ui, "None", width);
        });
        controls::control_row(ui, "Out", |ui| {
            let width = ui.available_width().clamp(90.0, 200.0);
            let _ = controls::disabled_select(ui, "None", width);
        });
        controls::control_row(ui, "Duration", |ui| {
            let _ = controls::disabled_value(ui, "0.5s", 64.0);
        });
        let _ = controls::disabled_toggle(ui, "Loop", false);
    });
}

fn basic_properties(ui: &mut Ui) {
    controls::section_card(ui, "Clip", |ui| {
        controls::control_row(ui, "Name", |ui| {
            let _ = controls::disabled_value(ui, "No clip", 120.0);
        });
        controls::control_row(ui, "Type", |ui| {
            let width = ui.available_width().clamp(90.0, 180.0);
            let _ = controls::disabled_select(ui, "Video", width);
        });
        controls::control_row(ui, "Start", |ui| {
            let _ = controls::disabled_value(ui, "00:00:00:00", 96.0);
        });
        controls::control_row(ui, "Duration", |ui| {
            let _ = controls::disabled_value(ui, "00:00:00:00", 96.0);
        });
    });
    ui.add_space(6.0);
    controls::section_card(ui, "Source", |ui| {
        controls::control_row(ui, "Media", |ui| {
            let _ = controls::disabled_value(ui, "None", 120.0);
        });
        controls::control_row(ui, "Resolution", |ui| {
            let _ = controls::disabled_value(ui, "—", 96.0);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_tab(tab: InspectorTab, caption_tab: CaptionTab, width: f32) {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let mut session = EditorSession {
            inspector_tab: tab,
            caption_tab,
            ..EditorSession::default()
        };
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(width, 560.0))),
            ..Default::default()
        };
        let output = context.run_ui(input, |ui| show(ui, &mut session));
        assert!(!output.shapes.is_empty());
        output.drop_without_applying_deltas();
    }

    #[test]
    fn every_inspector_tab_renders_at_reference_width() {
        for tab in InspectorTab::ALL {
            render_tab(tab, CaptionTab::Basic, 600.0);
        }
    }

    #[test]
    fn caption_secondary_tabs_render_at_narrow_width() {
        for caption_tab in CaptionTab::ALL {
            render_tab(InspectorTab::Captions, caption_tab, 260.0);
        }
    }

    #[test]
    fn caption_style_tiles_fit_inside_narrow_panel() {
        let context = egui::Context::default();
        let mut session = EditorSession {
            inspector_tab: InspectorTab::Captions,
            caption_tab: CaptionTab::Basic,
            ..EditorSession::default()
        };
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(260.0, 560.0))),
            ..Default::default()
        };
        let output = context.run_ui(input, |ui| {
            Frame::new()
                .inner_margin(Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    style_tiles(ui, &mut session);
                });
        });
        let content = output
            .shapes
            .iter()
            .map(|shape| shape.shape.visual_bounding_rect());
        let min = content
            .clone()
            .fold(Rect::NOTHING, |acc, rect| acc.union(rect));
        assert!(min.max.x <= 260.0 + 0.5, "style tiles overflow: {min:?}");
        output.drop_without_applying_deltas();
    }
}
