//! Small shared shell widgets: headings, badges, empty states, and rows.

use egui::{
    vec2, Color32, CornerRadius, CursorIcon, FontId, Frame, Margin, Rect, Response, RichText,
    Sense, Stroke, StrokeKind, Ui,
};

use crate::theme::{Palette, Radii};
use crate::widgets::icons::{self, Icon, IconColors, IconSize};

/// Uppercase muted section label used at the top of each panel.
pub fn section_heading(ui: &mut Ui, text: &str) -> Response {
    ui.label(
        RichText::new(text.to_ascii_uppercase())
            .size(10.0)
            .strong()
            .color(Palette::TEXT_MUTED),
    )
}

/// Compact outlined badge, e.g. `PLACEHOLDER` or `EMPTY`.
///
/// Painted manually so layout contexts cannot stretch the pill vertically.
pub fn badge(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_ascii_uppercase(), FontId::proportional(8.5), color);
    let size = galley.size() + vec2(10.0, 5.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter()
        .rect_filled(rect, Radii::SMALL, Palette::WIDGET_BG);
    ui.painter().rect_stroke(
        rect,
        Radii::SMALL,
        Stroke::new(1.0, color.gamma_multiply(0.55)),
        StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, color);
    response
}

/// Small status indicator dot.
pub fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.0, color);
}

/// Full-width disabled action, used by empty states.
pub fn disabled_action_button(ui: &mut Ui, label: &str, hint: &str) -> Response {
    let width = ui.available_width();
    let button = egui::Button::new(RichText::new(label).size(12.0)).min_size(vec2(width, 28.0));

    ui.add_enabled(false, button).on_disabled_hover_text(hint)
}

/// Teal primary action button used on the homepage and editor top bar.
pub fn primary_button(ui: &mut Ui, label: &str, min_width: f32) -> Response {
    let response = ui
        .add(
            egui::Button::new(
                RichText::new(label)
                    .size(13.0)
                    .strong()
                    .color(Palette::ON_ACCENT),
            )
            .fill(Palette::ACCENT)
            .stroke(Stroke::NONE)
            .corner_radius(Radii::CONTROL)
            .min_size(vec2(min_width, 34.0)),
        )
        .on_hover_cursor(CursorIcon::PointingHand);
    let rect = response.rect;
    if response.is_pointer_button_down_on() {
        ui.painter()
            .rect_filled(rect, Radii::CONTROL, Color32::from_black_alpha(20));
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, Radii::CONTROL, Color32::from_white_alpha(12));
    }
    focus_ring_colored(ui, &response, rect, Radii::CONTROL, Palette::ON_ACCENT);
    response
}

/// Compact bordered empty state for narrow side panels.
pub fn compact_empty_state(ui: &mut Ui, icon: Icon, title: &str, body: &str) {
    Frame::new()
        .fill(Palette::TRACK_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
        .corner_radius(Radii::CARD)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical_centered(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                icons::paint_sized(
                    ui.painter(),
                    icon_rect,
                    icon,
                    IconSize::EmptyState,
                    IconColors::MUTED,
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(title)
                        .size(12.0)
                        .strong()
                        .color(Palette::TEXT_SECONDARY),
                );
                ui.add_space(2.0);
                ui.add(
                    egui::Label::new(RichText::new(body).size(10.5).color(Palette::TEXT_MUTED))
                        .wrap(),
                );
            });
        });
}

/// Small disabled icon-only button with an explanation tooltip.
pub fn disabled_icon_button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(26.0, 24.0), Sense::hover());
    let hovered = response.hovered();
    if hovered {
        ui.painter()
            .rect_filled(rect, Radii::CONTROL, Palette::WIDGET_BG);
    }
    icons::paint_sized(
        ui.painter(),
        rect,
        icon,
        IconSize::Standard,
        if hovered {
            IconColors::MUTED
        } else {
            IconColors::DISABLED
        },
    );

    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Button, false, tooltip));
    response
        .on_hover_cursor(CursorIcon::NotAllowed)
        .on_hover_text(tooltip)
}

/// Small square icon-only button with a tooltip.
pub fn icon_button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
    let hovered = response.hovered();
    if response.is_pointer_button_down_on() {
        ui.painter()
            .rect_filled(rect, Radii::CONTROL, Palette::WIDGET_ACTIVE);
    } else if hovered {
        ui.painter()
            .rect_filled(rect, Radii::CONTROL, Palette::WIDGET_BG);
    }
    focus_ring(ui, &response, rect, Radii::CONTROL);
    icons::paint_sized(
        ui.painter(),
        rect,
        icon,
        IconSize::Standard,
        if hovered {
            IconColors::HOVERED
        } else {
            IconColors::NORMAL
        },
    );

    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Button, true, tooltip));
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

/// Draw a subtle keyboard-focus ring for custom-painted controls.
pub fn focus_ring(ui: &Ui, response: &Response, rect: Rect, radius: CornerRadius) {
    focus_ring_colored(ui, response, rect, radius, Palette::ACCENT_LIGHT);
}

/// Focus ring variant for filled primary surfaces.
pub fn focus_ring_colored(
    ui: &Ui,
    response: &Response,
    rect: Rect,
    radius: CornerRadius,
    color: Color32,
) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            radius,
            Stroke::new(1.0, color),
            StrokeKind::Inside,
        );
    }
}
