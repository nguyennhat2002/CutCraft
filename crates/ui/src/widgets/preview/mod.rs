//! Reference-style preview player: title bar, centered monitor, and transport.
//!
//! The loaded state is an explicit synthetic QA frame. No media is decoded,
//! imported, or mixed into the temporary project view.

use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Mesh, Painter, Rect, Response,
    Sense, Shape, Stroke, StrokeKind, Ui,
};

use crate::config;
use crate::editor_state::{EditorSession, PreviewAspect};
use crate::theme::{Palette, Radii};
use crate::timecode::format_timecode;
use crate::timeline_state::TimelineDisplayMode;
use crate::widgets::common::focus_ring;
use crate::widgets::icons::{self, Icon, IconColors, IconSize};

const HEADER_HEIGHT: f32 = 42.0;
const FOOTER_HEIGHT: f32 = 50.0;
const DEMO_DURATION_MILLIS: u64 = 18_767;
const DEMO_LABEL: &str = "DEMO FRAME";

pub fn show(ui: &mut Ui, session: &mut EditorSession) {
    header(ui);

    let monitor_height = (ui.available_height() - FOOTER_HEIGHT).max(40.0);
    let (monitor_area, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), monitor_height), Sense::hover());
    paint_monitor(ui, monitor_area, session);

    footer(ui, session);
}

fn header(ui: &mut Ui) {
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), HEADER_HEIGHT), Sense::hover());
    let menu_rect =
        Rect::from_center_size(pos2(rect.right() - 21.0, rect.center().y), vec2(28.0, 28.0));
    let menu = ui.interact(
        menu_rect,
        ui.scope_id().with("preview_menu"),
        Sense::hover(),
    );

    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, Palette::TOOLBAR_BG);
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, Palette::BORDER_SUBTLE),
    );
    painter.text(
        pos2(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        "Player-Timeline 01",
        FontId::proportional(10.5),
        Palette::TEXT_PRIMARY,
    );

    if menu.hovered() {
        painter.rect_filled(menu_rect, Radii::CONTROL, Palette::WIDGET_BG);
    }
    icons::paint_sized(
        painter,
        menu_rect,
        Icon::Menu,
        IconSize::Standard,
        if menu.hovered() {
            IconColors::HOVERED
        } else {
            IconColors::NORMAL
        },
    );
    menu.on_hover_cursor(CursorIcon::NotAllowed)
        .on_hover_text("Preview options are not implemented.");
}

fn paint_monitor(ui: &Ui, area: Rect, session: &EditorSession) {
    let painter = ui.painter_at(area);
    painter.rect_filled(area, CornerRadius::ZERO, Palette::PANEL_BG);

    let padded = area.shrink2(vec2(10.0, 2.0));
    let canvas = fit_preview_rect(padded, session.preview_aspect.ratio());
    if canvas.width() <= 1.0 || canvas.height() <= 1.0 {
        return;
    }

    painter.rect_filled(canvas, Radii::SMALL, Palette::CANVAS);
    if session.timeline_display_mode == TimelineDisplayMode::VisualDemo {
        paint_demo_frame(&painter, canvas);
    } else {
        paint_empty_frame(&painter, canvas);
    }
    painter.rect_stroke(
        canvas,
        Radii::SMALL,
        Stroke::new(1.0, Palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );
}

fn paint_empty_frame(painter: &Painter, canvas: Rect) {
    if canvas.height() < 90.0 {
        return;
    }
    let center = canvas.center();
    let icon_rect = Rect::from_center_size(
        center - vec2(0.0, 19.0),
        vec2(IconSize::EmptyState.points(), IconSize::EmptyState.points()),
    );
    icons::paint_sized(
        painter,
        icon_rect,
        Icon::CirclePlay,
        IconSize::EmptyState,
        IconColors::MUTED,
    );
    painter.text(
        center + vec2(0.0, 16.0),
        Align2::CENTER_CENTER,
        "No media loaded",
        FontId::proportional(12.0),
        Palette::TEXT_SECONDARY,
    );
    painter.text(
        center + vec2(0.0, 32.0),
        Align2::CENTER_CENTER,
        "Preview rendering is not implemented",
        FontId::proportional(8.5),
        Palette::TEXT_DISABLED,
    );
}

fn paint_demo_frame(painter: &Painter, canvas: Rect) {
    let frame = fit_preview_rect(canvas.shrink(1.0), 4.0 / 5.0);
    if frame.width() < 24.0 || frame.height() < 30.0 {
        return;
    }
    let painter = painter.with_clip_rect(frame.intersect(painter.clip_rect()));
    paint_vertical_gradient(
        &painter,
        frame,
        Color32::from_rgb(0xf2, 0xbe, 0x8c),
        Color32::from_rgb(0x9f, 0x43, 0x48),
    );

    let width = frame.width();
    let height = frame.height();
    painter.circle_filled(
        pos2(frame.left() + width * 0.66, frame.top() + height * 0.27),
        width * 0.13,
        Color32::from_rgba_unmultiplied(255, 232, 184, 225),
    );

    painter.add(Shape::convex_polygon(
        vec![
            pos2(frame.left(), frame.top() + height * 0.62),
            pos2(frame.left() + width * 0.24, frame.top() + height * 0.45),
            pos2(frame.left() + width * 0.48, frame.top() + height * 0.62),
            pos2(frame.left() + width * 0.73, frame.top() + height * 0.41),
            pos2(frame.right(), frame.top() + height * 0.58),
            pos2(frame.right(), frame.bottom()),
            pos2(frame.left(), frame.bottom()),
        ],
        Color32::from_rgb(0x64, 0x30, 0x38),
        Stroke::NONE,
    ));
    painter.add(Shape::convex_polygon(
        vec![
            pos2(frame.left(), frame.top() + height * 0.74),
            pos2(frame.left() + width * 0.32, frame.top() + height * 0.60),
            pos2(frame.left() + width * 0.58, frame.top() + height * 0.76),
            pos2(frame.left() + width * 0.82, frame.top() + height * 0.58),
            pos2(frame.right(), frame.top() + height * 0.71),
            pos2(frame.right(), frame.bottom()),
            pos2(frame.left(), frame.bottom()),
        ],
        Color32::from_rgb(0x30, 0x19, 0x22),
        Stroke::NONE,
    ));

    let trunk_x = frame.left() + width * 0.21;
    let ground_y = frame.top() + height * 0.77;
    painter.line_segment(
        [
            pos2(trunk_x, ground_y),
            pos2(trunk_x, frame.top() + height * 0.58),
        ],
        Stroke::new(1.5, Color32::from_rgb(0x18, 0x0e, 0x13)),
    );
    for (dx, dy) in [(-0.06, 0.03), (0.07, 0.05), (-0.03, 0.09)] {
        painter.line_segment(
            [
                pos2(trunk_x, frame.top() + height * (0.62 + dy)),
                pos2(trunk_x + width * dx, frame.top() + height * (0.55 + dy)),
            ],
            Stroke::new(1.2, Color32::from_rgb(0x18, 0x0e, 0x13)),
        );
    }
    for (dx, dy) in [(0.46, 0.35), (0.55, 0.32), (0.62, 0.37)] {
        let x = frame.left() + width * dx;
        let y = frame.top() + height * dy;
        painter.line_segment(
            [pos2(x - 4.0, y), pos2(x, y - 2.0)],
            Stroke::new(1.0, Color32::from_rgb(0x3a, 0x24, 0x2a)),
        );
        painter.line_segment(
            [pos2(x, y - 2.0), pos2(x + 4.0, y)],
            Stroke::new(1.0, Color32::from_rgb(0x3a, 0x24, 0x2a)),
        );
    }

    painter.rect_filled(
        Rect::from_min_max(
            pos2(frame.left(), frame.bottom() - height * 0.30),
            frame.right_bottom(),
        ),
        CornerRadius::ZERO,
        Color32::from_rgba_unmultiplied(0, 0, 0, 76),
    );
    painter.text(
        pos2(frame.left() + 10.0, frame.bottom() - 29.0),
        Align2::LEFT_CENTER,
        "Golden hour",
        FontId::proportional(12.0),
        Palette::TEXT_PRIMARY,
    );
    painter.text(
        pos2(frame.left() + 10.0, frame.bottom() - 14.0),
        Align2::LEFT_CENTER,
        "Synthetic preview",
        FontId::proportional(7.5),
        Palette::TEXT_SECONDARY,
    );

    let badge = Rect::from_min_size(frame.left_top() + vec2(7.0, 7.0), vec2(70.0, 16.0));
    painter.rect_filled(
        badge,
        Radii::SMALL,
        Color32::from_rgba_unmultiplied(6, 12, 14, 170),
    );
    painter.text(
        badge.center(),
        Align2::CENTER_CENTER,
        DEMO_LABEL,
        FontId::proportional(7.5),
        Palette::ACCENT_LIGHT,
    );
}

fn paint_vertical_gradient(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

fn footer(ui: &mut Ui, session: &mut EditorSession) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, FOOTER_HEIGHT), Sense::hover());
    let width = rect.width();
    let compact = width < 420.0;
    let very_compact = width < 320.0;
    let pad = if very_compact {
        6.0
    } else if compact {
        8.0
    } else {
        12.0
    };
    let time_font_size = if very_compact {
        7.4
    } else if compact {
        8.3
    } else {
        10.0
    };
    let play_size = if compact { 26.0 } else { 34.0 };
    let play_rect = Rect::from_center_size(rect.center(), vec2(play_size, 26.0));

    let (full_w, crop_w, ratio_w, fullscreen_w, gap) = if compact {
        (28.0, 20.0, 36.0, 20.0, 4.0)
    } else {
        (32.0, 24.0, 44.0, 24.0, 6.0)
    };
    let right_width = full_w + crop_w + ratio_w + fullscreen_w + gap * 3.0;
    let right_left = rect.right() - pad - right_width;
    let control_height = 22.0;
    let control_y = rect.center().y - control_height / 2.0;

    let full_rect = Rect::from_min_size(pos2(right_left, control_y), vec2(full_w, control_height));
    let crop_rect = Rect::from_min_size(
        pos2(full_rect.right() + gap, control_y),
        vec2(crop_w, control_height),
    );
    let ratio_rect = Rect::from_min_size(
        pos2(crop_rect.right() + gap, control_y),
        vec2(ratio_w, control_height),
    );
    let fullscreen_rect = Rect::from_min_size(
        pos2(ratio_rect.right() + gap, control_y),
        vec2(fullscreen_w, control_height),
    );

    let _ = disabled_text_control(
        ui,
        full_rect,
        "Full",
        "Full-screen fitting is not implemented.",
        "preview_full",
    );
    let _ = disabled_icon_control(
        ui,
        crop_rect,
        Icon::Crop,
        "Crop and frame controls are not implemented.",
        "preview_crop",
    );
    if ratio_control(ui, ratio_rect, session.preview_aspect, compact).clicked() {
        session.preview_aspect = match session.preview_aspect {
            PreviewAspect::Landscape => PreviewAspect::Portrait,
            PreviewAspect::Portrait => PreviewAspect::Landscape,
        };
    }
    let _ = disabled_icon_control(
        ui,
        fullscreen_rect,
        Icon::Fullscreen,
        "Preview fullscreen is not implemented.",
        "preview_fullscreen",
    );
    let _ = play_button(ui, play_rect, compact);

    let painter = ui.painter_at(rect);
    let font = FontId::monospace(time_font_size);
    let current = timecode_label(0);
    let total = timecode_label(preview_total_millis(session));
    let current_galley = painter.layout_no_wrap(current, font.clone(), Palette::ACCENT);
    let slash_galley = painter.layout_no_wrap("/".to_owned(), font.clone(), Palette::TEXT_MUTED);
    let total_galley = painter.layout_no_wrap(total, font, Palette::TEXT_SECONDARY);
    let grid_width = if compact { 0.0 } else { 20.0 };
    let full_left_width =
        current_galley.size().x + 4.0 + slash_galley.size().x + 4.0 + total_galley.size().x;
    let left_start = rect.left() + pad;
    let left_limit = play_rect.left() - 5.0;
    let show_total = left_start + full_left_width <= left_limit;
    let show_grid = !compact && left_start + full_left_width + grid_width <= left_limit;

    let mut x = left_start;
    let baseline = rect.center().y - current_galley.size().y / 2.0;
    painter.galley(pos2(x, baseline), current_galley.clone(), Palette::ACCENT);
    x += current_galley.size().x + 4.0;
    painter.galley(pos2(x, baseline), slash_galley.clone(), Palette::TEXT_MUTED);
    x += slash_galley.size().x + 4.0;
    if show_total {
        painter.galley(
            pos2(x, baseline),
            total_galley.clone(),
            Palette::TEXT_SECONDARY,
        );
        x += total_galley.size().x;
    }
    if show_grid {
        let icon_rect = Rect::from_center_size(pos2(x + 8.0, rect.center().y), vec2(14.0, 14.0));
        icons::paint_sized(
            &painter,
            icon_rect,
            Icon::Grid,
            IconSize::Small,
            IconColors::MUTED,
        );
        x += grid_width;
    }
    let time_rect = Rect::from_min_max(
        pos2(left_start, rect.top() + 10.0),
        pos2(x.max(left_start + 1.0), rect.bottom() - 10.0),
    );
    ui.interact(
        time_rect,
        ui.scope_id().with("preview_timecode"),
        Sense::hover(),
    )
    .on_hover_text("Static preview timecode; playback is not implemented.");
}

fn preview_total_millis(session: &EditorSession) -> u64 {
    if session.timeline_display_mode == TimelineDisplayMode::VisualDemo {
        DEMO_DURATION_MILLIS
    } else {
        0
    }
}

fn timecode_label(millis: u64) -> String {
    format_timecode(
        millis,
        config::PREVIEW_FPS_NUMERATOR,
        config::PREVIEW_FPS_DENOMINATOR,
    )
    .unwrap_or_else(|| "--:--:--:--".to_owned())
}

fn disabled_text_control(
    ui: &mut Ui,
    rect: Rect,
    label: &str,
    tooltip: &str,
    id: &'static str,
) -> Response {
    let response = ui.interact(rect, ui.scope_id().with(id), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, Radii::CONTROL, Palette::WIDGET_BG);
    painter.rect_stroke(
        rect,
        Radii::CONTROL,
        Stroke::new(1.0, Palette::BORDER),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(9.0),
        Palette::TEXT_SECONDARY,
    );
    response
        .on_hover_cursor(CursorIcon::NotAllowed)
        .on_hover_text(tooltip)
}

fn disabled_icon_control(
    ui: &mut Ui,
    rect: Rect,
    icon: Icon,
    tooltip: &str,
    id: &'static str,
) -> Response {
    let response = ui.interact(rect, ui.scope_id().with(id), Sense::hover());
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect, Radii::CONTROL, Palette::WIDGET_BG);
    }
    icons::paint_sized(
        painter,
        Rect::from_center_size(rect.center(), vec2(16.0, 16.0)),
        icon,
        IconSize::Small,
        if response.hovered() {
            IconColors::MUTED
        } else {
            IconColors::DISABLED
        },
    );
    response
        .on_hover_cursor(CursorIcon::NotAllowed)
        .on_hover_text(tooltip)
}

fn ratio_control(ui: &mut Ui, rect: Rect, aspect: PreviewAspect, compact: bool) -> Response {
    let response = ui.interact(rect, ui.scope_id().with("preview_ratio"), Sense::click());
    let hovered = response.hovered();
    let pressed = response.is_pointer_button_down_on();
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Radii::CONTROL,
        if pressed {
            Palette::WIDGET_ACTIVE
        } else if hovered {
            Palette::WIDGET_HOVER
        } else {
            Palette::WIDGET_BG
        },
    );
    painter.rect_stroke(
        rect,
        Radii::CONTROL,
        Stroke::new(
            1.0,
            if hovered {
                Palette::BORDER
            } else {
                Palette::BORDER_SUBTLE
            },
        ),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        aspect.label(),
        FontId::monospace(if compact { 8.0 } else { 9.5 }),
        Palette::TEXT_SECONDARY,
    );
    focus_ring(ui, &response, rect.shrink(1.0), Radii::CONTROL);
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(format!(
            "{} · click to switch preview aspect (display only)",
            resolution_label(aspect)
        ))
}

fn play_button(ui: &mut Ui, rect: Rect, compact: bool) -> Response {
    let response = ui.interact(rect, ui.scope_id().with("preview_play"), Sense::hover());
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect.shrink(2.0), Radii::CONTROL, Palette::WIDGET_BG);
    }
    icons::paint_sized(
        painter,
        rect,
        Icon::Play,
        if compact {
            IconSize::Small
        } else {
            IconSize::Standard
        },
        IconColors::NORMAL,
    );
    response
        .on_hover_cursor(CursorIcon::NotAllowed)
        .on_hover_text("Playback is not implemented in this milestone.")
}

fn resolution_label(aspect: PreviewAspect) -> String {
    let (width, height) = aspect.dimensions();
    format!(
        "{width} \u{00d7} {height}  \u{00b7}  {}",
        rate_label(
            config::PREVIEW_FPS_NUMERATOR,
            config::PREVIEW_FPS_DENOMINATOR
        )
    )
}

fn rate_label(numerator: u32, denominator: u32) -> String {
    if denominator == 0 {
        return "unknown fps".to_owned();
    }
    if denominator == 1 {
        format!("{numerator} fps")
    } else {
        format!("{:.2} fps", f64::from(numerator) / f64::from(denominator))
    }
}

/// Fit a rectangle with `aspect` (width / height) inside `area`, centered.
///
/// Returns an empty rectangle at `area.min` for non-positive or non-finite
/// inputs so painting code cannot produce invalid geometry.
pub fn fit_preview_rect(area: Rect, aspect: f32) -> Rect {
    let size = area.size();
    if !aspect.is_finite()
        || aspect <= 0.0
        || !size.x.is_finite()
        || !size.y.is_finite()
        || size.x <= 0.0
        || size.y <= 0.0
    {
        return Rect::from_min_size(area.min, vec2(0.0, 0.0));
    }

    let (width, height) = if size.x / size.y > aspect {
        (size.y * aspect, size.y)
    } else {
        (size.x, size.x / aspect)
    };

    Rect::from_center_size(area.center(), vec2(width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_wide_area_by_height_and_centers() {
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
        let fitted = fit_preview_rect(area, PreviewAspect::Landscape.ratio());

        assert!((fitted.width() - 800.0).abs() < 0.01);
        assert!((fitted.height() - 450.0).abs() < 0.01);
        assert!((fitted.center().y - area.center().y).abs() < 0.01);
    }

    #[test]
    fn fits_tall_area_by_width_and_centers() {
        let area = Rect::from_min_size(pos2(10.0, 20.0), vec2(400.0, 600.0));
        let fitted = fit_preview_rect(area, 16.0 / 9.0);

        assert!((fitted.width() - 400.0).abs() < 0.01);
        assert!((fitted.height() - 225.0).abs() < 0.01);
        assert!((fitted.center().x - area.center().x).abs() < 0.01);
        assert!((fitted.center().y - area.center().y).abs() < 0.01);
    }

    #[test]
    fn portrait_aspect_fits_inside_a_wide_area() {
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
        let fitted = fit_preview_rect(area, PreviewAspect::Portrait.ratio());

        assert!((fitted.width() - 337.5).abs() < 0.01);
        assert!((fitted.height() - 600.0).abs() < 0.01);
        assert!((fitted.center().x - area.center().x).abs() < 0.01);
    }

    #[test]
    fn invalid_geometry_returns_an_empty_rect() {
        let area = Rect::from_min_size(pos2(3.0, 4.0), vec2(0.0, 100.0));
        assert_eq!(fit_preview_rect(area, 16.0 / 9.0).size(), vec2(0.0, 0.0));

        let area = Rect::from_min_size(pos2(3.0, 4.0), vec2(100.0, 100.0));
        assert_eq!(fit_preview_rect(area, f32::NAN).size(), vec2(0.0, 0.0));
        assert_eq!(fit_preview_rect(area, 0.0).size(), vec2(0.0, 0.0));
    }

    #[test]
    fn rate_label_distinguishes_integer_and_rational_rates() {
        assert_eq!(rate_label(30, 1), "30 fps");
        assert_eq!(rate_label(30_000, 1_001), "29.97 fps");
        assert_eq!(rate_label(30, 0), "unknown fps");
    }

    fn labels(session: &mut EditorSession, width: f32, height: f32) -> Vec<String> {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(width, height))),
            ..Default::default()
        };
        let output = context.run_ui(input, |ui| show(ui, session));
        let text = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        output.drop_without_applying_deltas();
        text
    }

    #[test]
    fn empty_preview_has_reference_header_timecodes_and_footer_controls() {
        let mut session = EditorSession::default();
        let text = labels(&mut session, 520.0, 520.0);

        assert!(text.iter().any(|line| line == "Player-Timeline 01"));
        assert!(
            text.iter()
                .filter(|line| line.as_str() == "00:00:00:00")
                .count()
                >= 2,
            "current and total timecodes should both be visible: {text:?}"
        );
        assert!(text.iter().any(|line| line == "Full"));
        assert!(!text.iter().any(|line| line.contains("DEMO FRAME")));
    }

    #[test]
    fn loaded_demo_preview_is_explicit_and_does_not_touch_timeline_data() {
        let mut session = EditorSession::default();
        let before = session.timeline_view.clone();
        session.set_timeline_demo(true);

        let text = labels(&mut session, 520.0, 520.0);

        assert!(
            text.iter().any(|line| line.contains("DEMO FRAME")),
            "loaded demo state should be explicitly labelled: {text:?}"
        );
        assert!(text.iter().any(|line| line == "00:00:18:23"));
        assert!(!text.iter().any(|line| line == "No media loaded"));
        assert_eq!(session.timeline_view, before);
    }

    #[test]
    fn preview_renders_headlessly_and_aspect_selection_is_stored() {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 500.0))),
            ..Default::default()
        };
        let mut session = EditorSession {
            preview_aspect: PreviewAspect::Portrait,
            ..EditorSession::default()
        };
        let output = context.run_ui(input, |ui| show(ui, &mut session));

        assert!(!output.shapes.is_empty());
        assert_eq!(session.preview_aspect, PreviewAspect::Portrait);
        output.drop_without_applying_deltas();
    }
}
