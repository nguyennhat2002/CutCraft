//! Reusable editor controls for the Inspector and Adjustment library.
//!
//! Controls are intentionally disabled until clip editing exists, but they use
//! consistent dimensions and chrome so the panels read as a professional tool.

use egui::{
    pos2, vec2, Color32, CursorIcon, FontId, Frame, Label, Margin, Response, RichText, Sense,
    Stroke, StrokeKind, Ui,
};

use crate::theme::{Palette, Radii};
use crate::widgets::common;

/// Disabled dropdown-shaped control used in place of a real ComboBox.
pub fn disabled_select(ui: &mut Ui, value: &str, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
    paint_field(ui, rect);
    ui.painter().text(
        pos2(rect.left() + 8.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        value,
        FontId::proportional(11.0),
        Palette::TEXT_MUTED,
    );
    let chevron_x = rect.right() - 12.0;
    let chevron_y = rect.center().y - 1.0;
    let stroke = Stroke::new(1.2, Palette::TEXT_DISABLED);
    ui.painter().line_segment(
        [
            pos2(chevron_x - 3.0, chevron_y),
            pos2(chevron_x, chevron_y + 3.0),
        ],
        stroke,
    );
    ui.painter().line_segment(
        [
            pos2(chevron_x, chevron_y + 3.0),
            pos2(chevron_x + 3.0, chevron_y),
        ],
        stroke,
    );

    disabled_hover(response)
}

/// Disabled numeric or read-only value field.
pub fn disabled_value(ui: &mut Ui, value: &str, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
    paint_field(ui, rect);
    ui.painter().text(
        pos2(rect.right() - 8.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        value,
        FontId::monospace(10.5),
        Palette::TEXT_MUTED,
    );
    disabled_hover(response)
}

/// Disabled slider with a visible value.
pub fn disabled_slider(
    ui: &mut Ui,
    label: &str,
    value: f32,
    range: std::ops::RangeInclusive<f32>,
) -> Response {
    let mut value = value;
    let response = ui.add_enabled(
        false,
        egui::Slider::new(&mut value, range)
            .text(label)
            .show_value(true),
    );
    response.on_disabled_hover_text("Clip property editing is not implemented yet.")
}

/// Disabled checkbox/toggle.
pub fn disabled_toggle(ui: &mut Ui, label: &str, checked: bool) -> Response {
    let mut checked = checked;
    ui.add_enabled(false, egui::Checkbox::new(&mut checked, label))
        .on_disabled_hover_text("Clip property editing is not implemented yet.")
}

/// Small 3×3 alignment picker, rendered disabled.
pub fn alignment_grid(ui: &mut Ui, selected: usize) -> Response {
    let cell = 18.0;
    let gap = 2.0;
    let size = cell * 3.0 + gap * 2.0;
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let painter = ui.painter_at(rect);

    for index in 0..9 {
        let column = index % 3;
        let row = index / 3;
        let min = pos2(
            rect.left() + column as f32 * (cell + gap),
            rect.top() + row as f32 * (cell + gap),
        );
        let cell_rect = egui::Rect::from_min_size(min, vec2(cell, cell));
        painter.rect_filled(
            cell_rect,
            Radii::SMALL,
            if index == selected {
                Palette::ACCENT_DIM
            } else {
                Palette::WIDGET_BG
            },
        );
        painter.rect_stroke(
            cell_rect,
            Radii::SMALL,
            Stroke::new(1.0, Palette::BORDER_SUBTLE),
            StrokeKind::Inside,
        );
        let dot = cell_rect.center();
        painter.circle_filled(
            dot,
            1.3,
            if index == selected {
                Palette::ACCENT
            } else {
                Palette::TEXT_DISABLED
            },
        );
    }

    disabled_hover(response)
}

/// Disabled color swatch with a neutral placeholder value.
pub fn color_swatch(ui: &mut Ui, color: Color32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
    ui.painter().rect_filled(rect, Radii::SMALL, color);
    ui.painter().rect_stroke(
        rect,
        Radii::SMALL,
        Stroke::new(1.0, Palette::BORDER),
        StrokeKind::Inside,
    );
    disabled_hover(response)
}

/// Label and control row with aligned label column.
pub fn control_row(ui: &mut Ui, label: &str, add_control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            vec2(88.0, 22.0),
            Label::new(RichText::new(label).size(10.5).color(Palette::TEXT_MUTED)).truncate(),
        );
        add_control(ui);
    });
}

/// Bordered inspector section with a compact heading.
pub fn section_card(ui: &mut Ui, title: &str, add_contents: impl FnOnce(&mut Ui)) {
    Frame::new()
        .fill(Palette::CARD_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
        .corner_radius(Radii::CARD)
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            common::section_heading(ui, title);
            ui.add_space(3.0);
            add_contents(ui);
        });
}

fn paint_field(ui: &Ui, rect: egui::Rect) {
    ui.painter()
        .rect_filled(rect, Radii::CONTROL, Palette::WIDGET_BG);
    ui.painter().rect_stroke(
        rect,
        Radii::CONTROL,
        Stroke::new(1.0, Palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );
}

fn disabled_hover(response: Response) -> Response {
    response
        .on_hover_cursor(CursorIcon::NotAllowed)
        .on_hover_text("Clip property editing is not implemented yet.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_controls_render_headlessly() {
        let context = egui::Context::default();
        let output = context.run_ui(egui::RawInput::default(), |ui| {
            section_card(ui, "Transform", |ui| {
                control_row(ui, "Position", |ui| {
                    let _ = disabled_value(ui, "0.0", 80.0);
                });
                let _ = disabled_slider(ui, "Scale", 100.0, 0.0..=400.0);
                let _ = disabled_toggle(ui, "Lock", true);
                let _ = alignment_grid(ui, 4);
                let _ = color_swatch(ui, Palette::WIDGET_BG);
                let _ = disabled_select(ui, "Normal", 100.0);
            });
        });
        assert!(!output.shapes.is_empty());
        output.drop_without_applying_deltas();
    }
}
