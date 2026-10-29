//! Modal export configuration dialog.
//!
//! Settings are stored for the temporary session, but encoding is not
//! implemented and the Export action is permanently disabled.

use egui::{
    vec2, Align, Align2, ComboBox, Context, CornerRadius, DragValue, Frame, Id, Label, Layout,
    Margin, RichText, ScrollArea, Sense, Stroke, StrokeKind, TextEdit, Ui,
};

use crate::config;
use crate::editor_state::EditorSession;
use crate::export_settings::{
    BitratePreset, ExportCodec, ExportFormat, ExportFrameRate, ExportResolution,
    CUSTOM_BITRATE_MAX_KBPS, CUSTOM_BITRATE_MIN_KBPS, MAX_FILENAME_CHARS,
};
use crate::theme::Palette;
use crate::widgets::common;
use crate::widgets::controls;
use crate::widgets::icons::Icon;
use crate::widgets::preview::fit_preview_rect;

pub fn show(context: &Context, session: &mut EditorSession) {
    if !session.export_open {
        return;
    }

    let content = context.content_rect();
    let dialog_width = (content.width() - 64.0).clamp(
        config::EXPORT_DIALOG_MIN_WIDTH,
        config::EXPORT_DIALOG_MAX_WIDTH,
    );
    let dialog_height = (content.height() - 56.0).clamp(360.0, config::EXPORT_DIALOG_MAX_HEIGHT);

    let mut close_requested = false;

    let modal = egui::Modal::new(Id::unique("cutcraft_export_dialog"))
        .backdrop_color(egui::Color32::from_black_alpha(170))
        .frame(
            Frame::new()
                .fill(Palette::PANEL_RAISED)
                .stroke(Stroke::new(1.0, Palette::BORDER))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(16)),
        )
        .show(context, |ui| {
            ui.set_width(dialog_width);
            ui.set_max_height(dialog_height);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Export")
                            .size(18.0)
                            .strong()
                            .color(Palette::TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new(
                            "Configure a future MP4 export. Settings are stored for this \
                             temporary session only.",
                        )
                        .size(9.5)
                        .color(Palette::TEXT_MUTED),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if common::icon_button(ui, Icon::Close, "Close export dialog").clicked() {
                        close_requested = true;
                    }
                    ui.add_space(4.0);
                    common::badge(ui, "Not available", Palette::WARNING);
                });
            });

            ui.add_space(7.0);
            ui.separator();
            ui.add_space(8.0);

            let body_height = (dialog_height - 130.0).max(260.0);
            ui.horizontal_top(|ui| {
                let available = ui.available_width();
                let left_width = (available * 0.40).max(250.0);
                let right_width = (available - left_width - 16.0).max(280.0);

                ui.allocate_ui_with_layout(
                    vec2(left_width, body_height),
                    Layout::top_down(Align::Min),
                    |ui| preview_panel(ui, session, body_height),
                );
                ui.add_space(16.0);
                ui.allocate_ui_with_layout(
                    vec2(right_width, body_height),
                    Layout::top_down(Align::Min),
                    |ui| {
                        let _ = ScrollArea::vertical()
                            .id_salt("export_form_scroll")
                            .auto_shrink([false, false])
                            .max_height(body_height)
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                            )
                            .show(ui, |ui| {
                                form(ui, session);
                            });
                    },
                );
            });

            ui.add_space(3.0);
            ui.separator();
            ui.add_space(7.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Encoding is not available in this milestone.")
                        .size(9.0)
                        .color(Palette::TEXT_DISABLED),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let export_button = ui.add_enabled(
                        false,
                        egui::Button::new(RichText::new("Export")).min_size(vec2(88.0, 28.0)),
                    );
                    let _ =
                        export_button.on_disabled_hover_text("An encoder is not implemented yet.");
                    ui.add_space(6.0);
                    if ui
                        .add(egui::Button::new("Cancel").min_size(vec2(78.0, 28.0)))
                        .clicked()
                    {
                        close_requested = true;
                    }
                });
            });
        });

    if close_requested || modal.should_close() {
        session.export_open = false;
    }
}

fn preview_panel(ui: &mut Ui, session: &EditorSession, body_height: f32) {
    Frame::new()
        .fill(Palette::CARD_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.set_min_height((body_height - 24.0).max(200.0));
            ui.horizontal(|ui| {
                common::section_heading(ui, "Output preview");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    common::badge(ui, "Placeholder", Palette::WARNING);
                });
            });
            ui.add_space(7.0);

            let canvas_height = (ui.available_height() - 68.0).max(100.0);
            let (area, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), canvas_height), Sense::hover());
            let canvas = fit_preview_rect(area.shrink(4.0), 16.0 / 9.0);
            let painter = ui.painter_at(area);

            if canvas.width() > 0.5 && canvas.height() > 0.5 {
                painter.rect_filled(canvas, CornerRadius::same(2), Palette::CANVAS);
                painter.rect_stroke(
                    canvas,
                    CornerRadius::same(2),
                    Stroke::new(1.0, Palette::BORDER),
                    StrokeKind::Inside,
                );
                if canvas.height() > 50.0 {
                    painter.text(
                        canvas.center() + vec2(0.0, -7.0),
                        Align2::CENTER_CENTER,
                        "No preview available",
                        egui::FontId::proportional(13.0),
                        Palette::TEXT_MUTED,
                    );
                }
                if canvas.height() > 76.0 {
                    painter.text(
                        canvas.center() + vec2(0.0, 11.0),
                        Align2::CENTER_CENTER,
                        "Rendered output preview is not implemented",
                        egui::FontId::proportional(9.0),
                        Palette::TEXT_DISABLED,
                    );
                }
            }

            ui.add_space(7.0);
            let settings = &session.export_settings;
            ui.label(
                RichText::new(&settings.filename)
                    .size(10.0)
                    .strong()
                    .color(Palette::TEXT_SECONDARY),
            );
            ui.label(
                RichText::new(format!(
                    "{} \u{00b7} {} \u{00b7} {} \u{00b7} {}",
                    settings.resolution.label(),
                    settings.frame_rate.label(),
                    settings.format_label(),
                    if settings.include_audio {
                        "audio on"
                    } else {
                        "video only"
                    }
                ))
                .size(9.0)
                .color(Palette::TEXT_MUTED),
            );
        });
}

fn form(ui: &mut Ui, session: &mut EditorSession) {
    let settings = &mut session.export_settings;

    form_section(ui, "File");
    common::field_label(ui, "Output name");
    ui.add(
        TextEdit::singleline(&mut settings.filename)
            .desired_width(ui.available_width())
            .char_limit(MAX_FILENAME_CHARS)
            .hint_text("cutcraft-export.mp4"),
    );
    ui.add_space(5.0);
    common::field_label(ui, "Destination");
    ui.horizontal(|ui| {
        let browse_width = 78.0;
        let field_width = (ui.available_width() - browse_width - 8.0).max(120.0);
        ui.add(
            TextEdit::singleline(&mut settings.directory)
                .desired_width(field_width)
                .hint_text("Select or type a folder"),
        );
        let browse = ui.add_enabled(
            false,
            egui::Button::new(RichText::new("Browse\u{2026}").size(10.0)),
        );
        let _ = browse.on_disabled_hover_text("A native folder picker is not implemented.");
    });

    ui.add_space(10.0);
    form_section(ui, "Format");

    controls::control_row(ui, "Format", |ui| {
        ComboBox::from_id_salt("export_format")
            .selected_text(settings.format.label())
            .width(140.0)
            .show_ui(ui, |ui| {
                for format in ExportFormat::ALL {
                    ui.selectable_value(&mut settings.format, format, format.label());
                }
            });
    });
    controls::control_row(ui, "Codec", |ui| {
        ComboBox::from_id_salt("export_codec")
            .selected_text(settings.codec.label())
            .width(140.0)
            .show_ui(ui, |ui| {
                for codec in ExportCodec::ALL {
                    ui.selectable_value(&mut settings.codec, codec, codec.label());
                }
            });
    });
    controls::control_row(ui, "Resolution", |ui| {
        ComboBox::from_id_salt("export_resolution")
            .selected_text(settings.resolution.label())
            .width(140.0)
            .show_ui(ui, |ui| {
                for resolution in ExportResolution::ALL {
                    ui.selectable_value(&mut settings.resolution, resolution, resolution.label());
                }
            });
    });
    controls::control_row(ui, "Frame rate", |ui| {
        ComboBox::from_id_salt("export_frame_rate")
            .selected_text(settings.frame_rate.label())
            .width(140.0)
            .show_ui(ui, |ui| {
                for frame_rate in ExportFrameRate::ALL {
                    ui.selectable_value(&mut settings.frame_rate, frame_rate, frame_rate.label());
                }
            });
    });
    controls::control_row(ui, "Color space", |ui| {
        let (width, height) = settings.resolution.dimensions();
        ui.label(
            RichText::new(format!(
                "{}  \u{00b7}  {width}\u{00d7}{height}",
                settings.color_space_label()
            ))
            .size(9.5)
            .color(Palette::TEXT_MUTED),
        );
    });

    ui.add_space(10.0);
    form_section(ui, "Video");
    controls::control_row(ui, "Bitrate", |ui| {
        ComboBox::from_id_salt("export_bitrate")
            .selected_text(settings.bitrate.label())
            .width(140.0)
            .show_ui(ui, |ui| {
                for preset in BitratePreset::ALL {
                    ui.selectable_value(&mut settings.bitrate, preset, preset.label());
                }
            });
    });
    if settings.bitrate == BitratePreset::Custom {
        controls::control_row(ui, "Custom", |ui| {
            ui.add(
                DragValue::new(&mut settings.custom_bitrate_kbps)
                    .range(CUSTOM_BITRATE_MIN_KBPS..=CUSTOM_BITRATE_MAX_KBPS)
                    .suffix(" kbps"),
            );
        });
    }

    ui.add_space(10.0);
    form_section(ui, "Audio");
    ui.checkbox(&mut settings.include_audio, "Include audio");

    ui.add_space(10.0);
    form_section(ui, "Validation");
    let issues = settings.issues();
    if issues.is_empty() {
        ui.label(
            RichText::new("The form is complete. Encoding is still unavailable.")
                .size(9.0)
                .color(Palette::TEXT_MUTED),
        );
    } else {
        for issue in issues {
            ui.label(
                RichText::new(format!("\u{2013} {}", issue.message()))
                    .size(9.0)
                    .color(Palette::WARNING),
            );
        }
    }

    ui.add_space(5.0);
    ui.add(
        Label::new(
            RichText::new(
                "H.264 in MP4 is the only format pair exposed by this UI. No file is \
                 created and no output size is estimated.",
            )
            .size(8.5)
            .color(Palette::TEXT_DISABLED),
        )
        .wrap(),
    );
}

fn form_section(ui: &mut Ui, title: &str) {
    common::section_heading(ui, title);
    ui.add_space(2.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Rect;

    fn input() -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1280.0, 800.0))),
            ..Default::default()
        }
    }

    #[test]
    fn export_dialog_renders_settings_without_enabling_export() {
        let context = Context::default();
        let mut session = EditorSession {
            export_open: true,
            ..EditorSession::default()
        };

        let mut text = Vec::new();
        for _ in 0..3 {
            let output = context.run_ui(input(), |ui| show(ui.ctx(), &mut session));
            for clipped in &output.shapes {
                if let egui::Shape::Text(shape) = &clipped.shape {
                    text.push(shape.galley.text().to_owned());
                }
            }
            output.drop_without_applying_deltas();
        }

        assert!(text.iter().any(|line| line == "Export"));
        assert!(text.iter().any(|line| line == "OUTPUT NAME"));
        assert!(text.iter().any(|line| line == "FORMAT"));
        assert!(text.iter().any(|line| line == "Codec"));
        assert!(text.iter().any(|line| line == "Cancel"));
        assert!(session.export_open);
    }

    #[test]
    fn closed_dialog_renders_nothing() {
        let context = Context::default();
        let mut session = EditorSession::default();
        let output = context.run_ui(input(), |ui| show(ui.ctx(), &mut session));

        assert!(output.shapes.is_empty());
        output.drop_without_applying_deltas();
    }
}
