//! Inspector panel with tabbed, control-dense clip properties.
//!
//! Every property control is disabled until clip selection and timeline
//! editing exist. The layout is a visual preview, not a functional editor.

use egui::{
    vec2, Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke, Ui,
};

use crate::editor_state::{EditorSession, InspectorTab};
use crate::theme::Palette;
use crate::widgets::common;
use crate::widgets::controls;
use crate::widgets::icons::{self, Icon};

pub fn show(ui: &mut Ui, session: &mut EditorSession) {
    ui.horizontal(|ui| {
        common::section_heading(ui, "Inspector");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            common::badge(ui, "No selection", Palette::TEXT_MUTED);
        });
    });

    ui.add_space(4.0);
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.horizontal_wrapped(|ui| {
            for tab in InspectorTab::ALL {
                let selected = session.inspector_tab == tab;
                let response = ui.add(
                    egui::Button::new(RichText::new(tab.label()).size(10.0).color(if selected {
                        Palette::ACCENT
                    } else {
                        Palette::TEXT_MUTED
                    }))
                    .selected(selected)
                    .min_size(vec2(0.0, 24.0)),
                );
                if response.clicked() {
                    session.inspector_tab = tab;
                }
            }
        });
    });
    ui.add_space(5.0);
    ui.separator();
    ui.add_space(6.0);

    let _ = ScrollArea::vertical()
        .id_salt("inspector_scroll")
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .show(ui, |ui| {
            empty_selection_banner(ui);
            ui.add_space(8.0);

            match session.inspector_tab {
                InspectorTab::Video => video_properties(ui),
                InspectorTab::Audio => audio_properties(ui),
                InspectorTab::Text => text_properties(ui),
                InspectorTab::Animation => animation_properties(ui),
                InspectorTab::Basic => basic_properties(ui),
            }

            ui.add_space(8.0);
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "Property controls are previews. Editing arrives with the timeline milestone.",
                    )
                    .size(9.0)
                    .color(Palette::TEXT_DISABLED),
                )
                .wrap(),
            );
        });
}

fn empty_selection_banner(ui: &mut Ui) {
    Frame::new()
        .fill(Palette::TRACK_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
                icons::paint(
                    ui.painter(),
                    icon_rect,
                    Icon::Settings,
                    Palette::TEXT_DISABLED,
                );
                ui.add_space(2.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Nothing selected")
                            .size(11.5)
                            .strong()
                            .color(Palette::TEXT_SECONDARY),
                    );
                    ui.label(
                        RichText::new("Select a clip to inspect its properties.")
                            .size(9.5)
                            .color(Palette::TEXT_MUTED),
                    );
                });
            });
        });
}

fn video_properties(ui: &mut Ui) {
    controls::section_card(ui, "Transform", |ui| {
        controls::control_row(ui, "Position", |ui| {
            let field_width = (ui.available_width() * 0.36).clamp(64.0, 132.0);
            let _ = controls::disabled_value(ui, "X 0.0", field_width);
            let _ = controls::disabled_value(ui, "Y 0.0", field_width);
        });
        controls::control_row(ui, "Scale", |ui| {
            let _ = controls::disabled_value(ui, "100%", 76.0);
        });
        controls::control_row(ui, "Rotation", |ui| {
            let _ = controls::disabled_value(ui, "0.0°", 76.0);
        });
        controls::control_row(ui, "Alignment", |ui| {
            let _ = controls::alignment_grid(ui, 4);
        });
        let _ = controls::disabled_slider(ui, "Opacity", 100.0, 0.0..=100.0);
    });
    ui.add_space(6.0);
    controls::section_card(ui, "Video", |ui| {
        controls::control_row(ui, "Blend mode", |ui| {
            let width = ui.available_width().clamp(90.0, 180.0);
            let _ = controls::disabled_select(ui, "Normal", width);
        });
        controls::control_row(ui, "Speed", |ui| {
            let _ = controls::disabled_value(ui, "1.00x", 76.0);
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
            let _ = controls::disabled_value(ui, "0.0s", 70.0);
        });
        controls::control_row(ui, "Fade out", |ui| {
            let _ = controls::disabled_value(ui, "0.0s", 70.0);
        });
        let _ = controls::disabled_toggle(ui, "Noise reduction", false);
    });
    ui.add_space(6.0);
    controls::section_card(ui, "Channels", |ui| {
        controls::control_row(ui, "Output", |ui| {
            let width = ui.available_width().clamp(100.0, 200.0);
            let _ = controls::disabled_select(ui, "Stereo", width);
        });
        controls::control_row(ui, "Mix", |ui| {
            let width = ui.available_width().clamp(100.0, 200.0);
            let _ = controls::disabled_select(ui, "Original", width);
        });
    });
}

fn text_properties(ui: &mut Ui) {
    controls::section_card(ui, "Text", |ui| {
        controls::control_row(ui, "Font", |ui| {
            let width = ui.available_width().clamp(100.0, 200.0);
            let _ = controls::disabled_select(ui, "Proportional", width);
        });
        controls::control_row(ui, "Size", |ui| {
            let _ = controls::disabled_value(ui, "24 pt", 70.0);
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
            let width = ui.available_width().clamp(100.0, 200.0);
            let _ = controls::disabled_select(ui, "None", width);
        });
        controls::control_row(ui, "Out", |ui| {
            let width = ui.available_width().clamp(100.0, 200.0);
            let _ = controls::disabled_select(ui, "None", width);
        });
        controls::control_row(ui, "Duration", |ui| {
            let _ = controls::disabled_value(ui, "0.5s", 70.0);
        });
        let _ = controls::disabled_toggle(ui, "Loop", false);
    });
}

fn basic_properties(ui: &mut Ui) {
    controls::section_card(ui, "Clip", |ui| {
        controls::control_row(ui, "Name", |ui| {
            let _ = controls::disabled_value(ui, "No clip", 130.0);
        });
        controls::control_row(ui, "Type", |ui| {
            let width = ui.available_width().clamp(100.0, 180.0);
            let _ = controls::disabled_select(ui, "Video", width);
        });
        controls::control_row(ui, "Start", |ui| {
            let _ = controls::disabled_value(ui, "00:00:00:00", 100.0);
        });
        controls::control_row(ui, "Duration", |ui| {
            let _ = controls::disabled_value(ui, "00:00:00:00", 100.0);
        });
    });
    ui.add_space(6.0);
    controls::section_card(ui, "Source", |ui| {
        controls::control_row(ui, "Media", |ui| {
            let _ = controls::disabled_value(ui, "None", 130.0);
        });
        controls::control_row(ui, "Resolution", |ui| {
            let _ = controls::disabled_value(ui, "—", 100.0);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_inspector_tab_renders_headlessly() {
        let context = egui::Context::default();
        for tab in InspectorTab::ALL {
            let mut session = EditorSession {
                inspector_tab: tab,
                ..EditorSession::default()
            };
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(620.0, 520.0),
                )),
                ..Default::default()
            };
            let output = context.run_ui(input, |ui| show(ui, &mut session));
            assert!(!output.shapes.is_empty());
            output.drop_without_applying_deltas();
        }
    }
}
