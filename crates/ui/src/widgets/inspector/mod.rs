//! Inspector panel.
//!
//! Property groups are reserved and disabled until clip selection and
//! timeline editing exist.

use egui::{
    vec2, Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke, Ui,
};

use crate::theme::Palette;
use crate::widgets::common;
use crate::widgets::icons::{self, Icon};

pub fn show(ui: &mut Ui) {
    ui.horizontal(|ui| {
        common::section_heading(ui, "Inspector");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            common::badge(ui, "No selection", Palette::TEXT_MUTED);
        });
    });

    ui.add_space(8.0);

    let _ = ScrollArea::vertical()
        .id_salt("inspector_scroll")
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .show(ui, |ui| {
            empty_selection_card(ui);
            ui.add_space(12.0);

            property_section(
                ui,
                "Transform",
                &[("Position", "Not available"), ("Scale", "Not available")],
            );
            ui.add_space(6.0);
            property_section(
                ui,
                "Video",
                &[("Opacity", "Not available"), ("Speed", "Not available")],
            );
            ui.add_space(6.0);
            property_section(
                ui,
                "Audio",
                &[("Volume", "Not available"), ("Fade", "Not available")],
            );
            ui.add_space(6.0);
            property_section(
                ui,
                "Text",
                &[("Font", "Not available"), ("Size", "Not available")],
            );

            ui.add_space(10.0);
            ui.add(
                egui::Label::new(
                    RichText::new("Property editing arrives with the timeline milestone.")
                        .size(9.5)
                        .color(Palette::TEXT_DISABLED),
                )
                .wrap(),
            );
        });
}

fn empty_selection_card(ui: &mut Ui) {
    Frame::new()
        .fill(Palette::TRACK_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.add_space(2.0);
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
                        RichText::new("Clip selection is not implemented.")
                            .size(9.5)
                            .color(Palette::TEXT_MUTED),
                    );
                });
            });
        });
}

fn property_section(ui: &mut Ui, title: &str, rows: &[(&str, &str)]) {
    Frame::new()
        .fill(Palette::CARD_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            common::section_heading(ui, title);
            ui.add_space(4.0);
            for (label, value) in rows {
                common::property_row(ui, label, value);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspector_renders_its_reserved_property_sections() {
        let context = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(300.0, 700.0),
            )),
            ..Default::default()
        };
        let output = context.run_ui(input, show);

        let text: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        assert!(text.iter().any(|line| line == "TRANSFORM"));
        assert!(text.iter().any(|line| line == "VIDEO"));
        assert!(text.iter().any(|line| line == "AUDIO"));
        assert!(text.iter().any(|line| line == "TEXT"));
        output.drop_without_applying_deltas();
    }
}
