//! Modal export configuration dialog.
//!
//! Settings are stored for the temporary session, but encoding is not
//! implemented and the Export action is permanently disabled.

use egui::{
    vec2, Align, Align2, ComboBox, Context, DragValue, Frame, Id, Label, Layout, Margin, RichText,
    ScrollArea, Sense, Stroke, StrokeKind, TextEdit, Ui,
};

use crate::config;
use crate::editor_state::EditorSession;
use crate::export_settings::{
    BitratePreset, ExportCodec, ExportFormat, ExportFrameRate, ExportResolution,
    CUSTOM_BITRATE_MAX_KBPS, CUSTOM_BITRATE_MIN_KBPS, MAX_FILENAME_CHARS,
};
use crate::theme::{Palette, Radii};
use crate::widgets::common;
use crate::widgets::controls;
use crate::widgets::icons::Icon;

pub fn show(context: &Context, session: &mut EditorSession) {
    if !session.export_open {
        return;
    }

    let content = context.content_rect();
    let dialog_width = (content.width() - 80.0).clamp(
        config::EXPORT_DIALOG_MIN_WIDTH,
        config::EXPORT_DIALOG_MAX_WIDTH,
    );
    let dialog_height = (content.height() - 64.0).clamp(400.0, config::EXPORT_DIALOG_MAX_HEIGHT);

    let mut close_requested = false;

    let modal = egui::Modal::new(Id::unique("cutcraft_export_dialog"))
        .backdrop_color(egui::Color32::from_black_alpha(170))
        .frame(
            Frame::new()
                .fill(Palette::MODAL_BG)
                .stroke(Stroke::new(1.0, Palette::BORDER))
                .corner_radius(Radii::DIALOG)
                .inner_margin(Margin::ZERO),
        )
        .show(context, |ui| {
            ui.set_width(dialog_width);
            ui.set_max_height(dialog_height);

            Frame::new()
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Export")
                                .size(15.0)
                                .strong()
                                .color(Palette::TEXT_PRIMARY),
                        );
                        ui.add_space(4.0);
                        common::badge(ui, "Encoder unavailable", Palette::WARNING);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if common::icon_button(ui, Icon::Close, "Close export dialog").clicked()
                            {
                                close_requested = true;
                            }
                        });
                    });
                });
            ui.separator();

            let body_height = (dialog_height - 112.0).max(260.0);
            Frame::new()
                .inner_margin(Margin::symmetric(18, 12))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal_top(|ui| {
                        ui.allocate_ui_with_layout(
                            vec2(190.0, body_height),
                            Layout::top_down(Align::Min),
                            |ui| cover_panel(ui, session, body_height),
                        );
                        ui.add_space(20.0);
                        ui.allocate_ui_with_layout(
                            vec2(ui.available_width(), body_height),
                            Layout::top_down(Align::Min),
                            |ui| {
                                let _ = ScrollArea::vertical()
                                    .id_salt("export_form_scroll")
                                    .auto_shrink([false, false])
                                    .max_height(body_height)
                                    .scroll_bar_visibility(
                                        egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                    )
                                    .show(ui, |ui| form(ui, session));
                            },
                        );
                    });
                });

            ui.separator();
            Frame::new()
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(
                                "Export is unavailable: no encoder is implemented in this build.",
                            )
                            .size(8.5)
                            .color(Palette::TEXT_DISABLED),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let export_button = ui.add_enabled(
                                false,
                                egui::Button::new(
                                    RichText::new("Export")
                                        .size(11.0)
                                        .color(Palette::TEXT_DISABLED),
                                )
                                .fill(Palette::ACCENT_BG)
                                .min_size(vec2(84.0, 28.0)),
                            );
                            let _ = export_button
                                .on_disabled_hover_text("An encoder is not implemented yet.");
                            ui.add_space(6.0);
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("Cancel").size(11.0))
                                        .min_size(vec2(76.0, 28.0)),
                                )
                                .clicked()
                            {
                                close_requested = true;
                            }
                        });
                    });
                });
        });

    if close_requested || modal.should_close() {
        session.export_open = false;
    }
}

fn cover_panel(ui: &mut Ui, session: &EditorSession, body_height: f32) {
    ui.set_min_height(body_height);
    let width = ui.available_width();
    let cover_height = (width * 16.0 / 9.0).min(body_height - 92.0).max(160.0);
    let (cover, _) = ui.allocate_exact_size(vec2(width, cover_height), Sense::hover());
    let painter = ui.painter_at(cover);
    painter.rect_filled(cover, Radii::CONTROL, Palette::CANVAS);
    painter.rect_stroke(
        cover,
        Radii::CONTROL,
        Stroke::new(1.0, Palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );

    let (width_px, height_px) = session.preview_aspect.dimensions();
    painter.text(
        cover.left_top() + vec2(9.0, 7.0),
        Align2::LEFT_TOP,
        "COVER",
        egui::FontId::proportional(7.5),
        Palette::TEXT_DISABLED,
    );
    painter.text(
        cover.right_top() + vec2(-9.0, 7.0),
        Align2::RIGHT_TOP,
        format!("{width_px}\u{00d7}{height_px}"),
        egui::FontId::monospace(7.5),
        Palette::TEXT_DISABLED,
    );
    painter.text(
        cover.center() - vec2(0.0, 7.0),
        Align2::CENTER_CENTER,
        "No cover preview",
        egui::FontId::proportional(10.5),
        Palette::TEXT_MUTED,
    );
    painter.text(
        cover.center() + vec2(0.0, 9.0),
        Align2::CENTER_CENTER,
        "Rendered frames are not implemented",
        egui::FontId::proportional(8.0),
        Palette::TEXT_DISABLED,
    );

    ui.add_space(8.0);
    ui.label(
        RichText::new("Cover")
            .size(10.0)
            .strong()
            .color(Palette::TEXT_SECONDARY),
    );
    ui.label(
        RichText::new("Placeholder \u{00b7} no rendered frame")
            .size(8.0)
            .color(Palette::TEXT_DISABLED),
    );
    ui.add_space(6.0);
    let edit = ui.add_enabled(
        false,
        egui::Button::new(RichText::new("Edit cover").size(10.0))
            .min_size(vec2(ui.available_width(), 26.0)),
    );
    let _ = edit.on_disabled_hover_text("Cover editing is not implemented.");
    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
        ui.add(
            Label::new(
                RichText::new("Cover and file-size estimates are not fabricated.")
                    .size(8.0)
                    .color(Palette::TEXT_DISABLED),
            )
            .wrap(),
        );
    });
}

fn form(ui: &mut Ui, session: &mut EditorSession) {
    let settings = &mut session.export_settings;

    form_row(ui, "Export timeline", |ui| {
        let width = ui.available_width().min(190.0);
        let _ = controls::disabled_select(ui, "Timeline 01", width);
    });

    ui.horizontal(|ui| {
        ui.add_sized(
            vec2(92.0, 24.0),
            Label::new(
                RichText::new("Name")
                    .size(10.0)
                    .color(Palette::TEXT_SECONDARY),
            )
            .truncate(),
        );
        ui.add(
            TextEdit::singleline(&mut settings.filename)
                .desired_width(ui.available_width())
                .char_limit(MAX_FILENAME_CHARS)
                .hint_text("cutcraft-export.mp4"),
        );
    });

    ui.horizontal(|ui| {
        ui.add_sized(
            vec2(92.0, 24.0),
            Label::new(
                RichText::new("Export to")
                    .size(10.0)
                    .color(Palette::TEXT_SECONDARY),
            )
            .truncate(),
        );
        let browse_width = 70.0;
        let field_width = (ui.available_width() - browse_width - 6.0).max(100.0);
        ui.add(
            TextEdit::singleline(&mut settings.directory)
                .desired_width(field_width)
                .hint_text("Select a folder"),
        );
        let browse = ui.add_enabled(
            false,
            egui::Button::new(RichText::new("Browse").size(9.5)).min_size(vec2(browse_width, 22.0)),
        );
        let _ = browse.on_disabled_hover_text("A native folder picker is not implemented.");
    });

    form_section(ui, "Video settings");

    setting_row(ui, "Resolution", |ui| {
        ComboBox::from_id_salt("export_resolution")
            .selected_text(settings.resolution.label())
            .width(150.0)
            .show_ui(ui, |ui| {
                for resolution in ExportResolution::ALL {
                    ui.selectable_value(&mut settings.resolution, resolution, resolution.label());
                }
            });
    });
    setting_row(ui, "Bitrate", |ui| {
        ComboBox::from_id_salt("export_bitrate")
            .selected_text(settings.bitrate.label())
            .width(150.0)
            .show_ui(ui, |ui| {
                for preset in BitratePreset::ALL {
                    ui.selectable_value(&mut settings.bitrate, preset, preset.label());
                }
            });
    });
    if settings.bitrate == BitratePreset::Custom {
        setting_row(ui, "Custom bitrate", |ui| {
            ui.add(
                DragValue::new(&mut settings.custom_bitrate_kbps)
                    .range(CUSTOM_BITRATE_MIN_KBPS..=CUSTOM_BITRATE_MAX_KBPS)
                    .suffix(" kbps"),
            );
        });
    }
    setting_row(ui, "Codec", |ui| {
        ComboBox::from_id_salt("export_codec")
            .selected_text(settings.codec.label())
            .width(150.0)
            .show_ui(ui, |ui| {
                for codec in ExportCodec::ALL {
                    ui.selectable_value(&mut settings.codec, codec, codec.label());
                }
            });
    });
    setting_row(ui, "Format", |ui| {
        ComboBox::from_id_salt("export_format")
            .selected_text(settings.format.label())
            .width(150.0)
            .show_ui(ui, |ui| {
                for format in ExportFormat::ALL {
                    ui.selectable_value(&mut settings.format, format, format.label());
                }
            });
    });
    setting_row(ui, "Frame rate", |ui| {
        ComboBox::from_id_salt("export_frame_rate")
            .selected_text(settings.frame_rate.label())
            .width(150.0)
            .show_ui(ui, |ui| {
                for frame_rate in ExportFrameRate::ALL {
                    ui.selectable_value(&mut settings.frame_rate, frame_rate, frame_rate.label());
                }
            });
    });
    setting_row(ui, "Color space", |ui| {
        ui.label(
            RichText::new(settings.color_space_label())
                .size(10.0)
                .color(Palette::TEXT_MUTED),
        );
    });

    form_section(ui, "Audio");
    ui.checkbox(&mut settings.include_audio, "Include audio");
    setting_row(ui, "Audio codec", |ui| {
        ui.label(
            RichText::new("AAC \u{00b7} planned")
                .size(10.0)
                .color(Palette::TEXT_DISABLED),
        );
    });
    setting_row(ui, "Sample rate", |ui| {
        ui.label(
            RichText::new("48 kHz \u{00b7} planned")
                .size(10.0)
                .color(Palette::TEXT_DISABLED),
        );
    });
    ui.add_space(4.0);

    ui.add_space(8.0);
    let issues = settings.issues();
    if issues.is_empty() {
        ui.label(
            RichText::new("Form values are complete. Encoding is still unavailable.")
                .size(8.5)
                .color(Palette::TEXT_MUTED),
        );
    } else {
        for issue in issues {
            ui.label(
                RichText::new(format!("\u{2013} {}", issue.message()))
                    .size(8.5)
                    .color(Palette::WARNING),
            );
        }
    }
    ui.add_space(4.0);
    ui.add(
        Label::new(
            RichText::new(
                "H.264 in MP4 is the only format pair exposed. No file is created and no \
                 output size is estimated.",
            )
            .size(8.0)
            .color(Palette::TEXT_DISABLED),
        )
        .wrap(),
    );
}

fn setting_row(ui: &mut Ui, label: &str, add_control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            vec2(92.0, 24.0),
            Label::new(
                RichText::new(label)
                    .size(10.0)
                    .color(Palette::TEXT_SECONDARY),
            )
            .truncate(),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            add_control(ui);
        });
    });
}

fn form_row(ui: &mut Ui, label: &str, add_control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            vec2(92.0, 24.0),
            Label::new(
                RichText::new(label)
                    .size(10.0)
                    .color(Palette::TEXT_SECONDARY),
            )
            .truncate(),
        );
        add_control(ui);
    });
}

fn form_section(ui: &mut Ui, title: &str) {
    ui.add_space(10.0);
    ui.label(
        RichText::new(title)
            .size(10.5)
            .strong()
            .color(Palette::TEXT_PRIMARY),
    );
    ui.add_space(3.0);
    ui.separator();
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
        crate::theme::configure(&context);
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
        assert!(text.iter().any(|line| line == "Name"));
        assert!(text.iter().any(|line| line == "Video settings"));
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
