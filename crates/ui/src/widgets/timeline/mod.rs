//! Timeline presentation: pristine canvas, supplied rows/items, or explicit QA.
//! Import, drop, selection, and editing controls are deliberately disabled.

pub mod clip_view;
pub mod playhead;
pub mod scale;
pub mod track_view;

use std::sync::LazyLock;

use egui::{
    pos2, vec2, Align, Align2, CornerRadius, FontId, Layout, Painter, Rect, ScrollArea, Sense,
    Shape, Stroke, Ui,
};

use crate::config;
use crate::editor_state::EditorSession;
use crate::theme::{Palette, Radii};
use crate::timeline_state::{TimelineContentState, TimelineDisplayMode, TimelineView};
use crate::widgets::common;
use crate::widgets::icons::{self, Icon};

use self::scale::{format_ruler_label, TimelineScale};

const INVITATION: &str = "Drag material here and start to create";
const DROP_UNAVAILABLE: &str =
    "Placeholder only: media import and drag-and-drop are not implemented.";

/// The visual demo and its toolbar control are opt-in for QA sessions only.
static QA_CONTROLS_ENABLED: LazyLock<bool> =
    LazyLock::new(|| std::env::var("CUTCRAFT_UI_DEMO").ok().as_deref() == Some("1"));

pub fn show(ui: &mut Ui, session: &mut EditorSession) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        session.timeline_pixels_per_second =
            TimelineScale::new(session.timeline_pixels_per_second, 0.0).pixels_per_second();
        toolbar(ui, session);

        let view = session.displayed_timeline();
        let pristine = view.content_state() == TimelineContentState::PristineEmpty;
        let header_width = if pristine {
            0.0
        } else {
            config::TRACK_HEADER_WIDTH
        };
        let scale = TimelineScale::new(
            session.timeline_pixels_per_second,
            if pristine {
                config::TIMELINE_EMPTY_LEADING_PADDING
            } else {
                config::TIMELINE_LEADING_PADDING
            },
        );
        let available = ui.available_size().max(vec2(1.0, 1.0));
        let body_height = (available.y - 14.0).max(config::TIMELINE_RULER_HEIGHT);
        // No row height calculation divides by zero for a pristine project.
        let track_height = if pristine {
            config::TIMELINE_TRACK_HEIGHT
        } else {
            ((body_height - config::TIMELINE_RULER_HEIGHT) / view.tracks.len() as f32)
                .clamp(24.0, 58.0)
        };
        let rows_height = config::TIMELINE_RULER_HEIGHT + track_height * view.tracks.len() as f32;
        let content_height = body_height.max(rows_height);
        // A navigable view span is NOT a project duration. Even zero tracks fill
        // the viewport, and the ruler continues over its complete visible range.
        let display_end = view
            .display_end_seconds()
            .max(config::TIMELINE_DURATION_SECONDS as f64);
        let content_width =
            (header_width + scale.x_for_seconds(display_end) + scale.leading_padding())
                .min(1_000_000.0)
                .max(available.x);
        let demo = session.timeline_display_mode == TimelineDisplayMode::VisualDemo;
        let output = ScrollArea::new([true, rows_height > body_height])
            .id_salt("timeline_scroll")
            .scroll_offset(vec2(session.timeline_scroll[0], session.timeline_scroll[1]))
            .auto_shrink([false, false])
            .max_height(available.y)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show_viewport(ui, |ui, viewport| {
                let (content, _) =
                    ui.allocate_exact_size(vec2(content_width, content_height), Sense::hover());
                let visible = viewport
                    .translate(content.min.to_vec2())
                    .intersect(ui.clip_rect());
                paint_content(ui, content, visible, &scale, view, track_height, demo);
            });
        let offset = output.state.clamped_offset();
        session.timeline_scroll = [offset.x, offset.y];
    });
}

fn toolbar(ui: &mut Ui, session: &mut EditorSession) {
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), config::TIMELINE_TOOLBAR_HEIGHT),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.add_space(4.0);
            disabled_tool(ui, Icon::Plus, "Add / import", DROP_UNAVAILABLE);
            ui.separator();
            disabled_tool(
                ui,
                Icon::Select,
                "Selection tool",
                config::UNIMPLEMENTED_HINT,
            );
            ui.separator();
            disabled_tool(ui, Icon::Undo, "Undo", config::UNIMPLEMENTED_HINT);
            disabled_tool(ui, Icon::Redo, "Redo", config::UNIMPLEMENTED_HINT);
            ui.separator();
            disabled_tool(ui, Icon::Split, "Split", config::UNIMPLEMENTED_HINT);
            disabled_tool(ui, Icon::Delete, "Delete", config::UNIMPLEMENTED_HINT);
            if session.timeline_display_mode == TimelineDisplayMode::VisualDemo {
                common::badge(ui, "Visual demo · not project data", Palette::WARNING);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(5.0);
                if *QA_CONTROLS_ENABLED
                    || session.timeline_display_mode == TimelineDisplayMode::VisualDemo
                {
                    ui.menu_button("QA", |ui| {
                        let mut enabled =
                            session.timeline_display_mode == TimelineDisplayMode::VisualDemo;
                        if ui
                            .checkbox(&mut enabled, "Visual demo (not project data)")
                            .changed()
                        {
                            session.set_timeline_demo(enabled);
                        }
                        ui.label("Synthetic layout for visual QA only. No media is loaded.");
                    });
                }
                if zoom_button(ui, "+", "Zoom timeline in").clicked() {
                    session.zoom_timeline_in();
                }
                ui.scope(|ui| {
                    ui.spacing_mut().slider_width = 92.0;
                    let zoom = ui.add(
                        egui::Slider::new(
                            &mut session.timeline_pixels_per_second,
                            config::TIMELINE_MIN_PIXELS_PER_SECOND
                                ..=config::TIMELINE_MAX_PIXELS_PER_SECOND,
                        )
                        .logarithmic(true)
                        .show_value(false),
                    );
                    let _ = zoom.on_hover_text("Timeline zoom");
                });
                if zoom_button(ui, "−", "Zoom timeline out").clicked() {
                    session.zoom_timeline_out();
                }
                ui.separator();
                disabled_tool(
                    ui,
                    Icon::Link,
                    "Linked selection",
                    config::UNIMPLEMENTED_HINT,
                );
                disabled_tool(ui, Icon::Snap, "Snapping", config::UNIMPLEMENTED_HINT);
            });
        },
    );
}

fn disabled_tool(ui: &mut Ui, icon: Icon, label: &str, hint: &str) {
    let response = ui.add_enabled(
        false,
        egui::Button::new("")
            .frame(false)
            .corner_radius(Radii::CONTROL)
            .min_size(vec2(26.0, 24.0)),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Button, false, label));
    icons::paint(
        ui.painter(),
        response.rect.shrink(5.0),
        icon,
        Palette::TEXT_DISABLED,
    );
    response.on_disabled_hover_text(format!("{label} — {hint}"));
}

fn zoom_button(ui: &mut Ui, glyph: &str, label: &str) -> egui::Response {
    let response = ui.add(
        egui::Button::new(glyph)
            .frame(false)
            .corner_radius(Radii::CONTROL)
            .min_size(vec2(24.0, 24.0)),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Button, true, label));
    response.on_hover_text(label)
}

fn paint_content(
    ui: &Ui,
    content: Rect,
    visible: Rect,
    scale: &TimelineScale,
    view: &TimelineView,
    track_height: f32,
    demo: bool,
) {
    let painter = ui.painter_at(visible);
    painter.rect_filled(visible, CornerRadius::ZERO, Palette::TRACK_BG);
    let pristine = view.content_state() == TimelineContentState::PristineEmpty;
    let header_width = if pristine {
        0.0
    } else {
        config::TRACK_HEADER_WIDTH
    };
    let time_viewport = Rect::from_min_max(visible.min + vec2(header_width, 0.0), visible.max);
    let ruler = Rect::from_min_size(
        time_viewport.min,
        vec2(time_viewport.width(), config::TIMELINE_RULER_HEIGHT),
    );
    let body = Rect::from_min_max(
        pos2(time_viewport.left(), ruler.bottom()),
        time_viewport.max,
    );
    let body_painter = painter.with_clip_rect(body);
    let time_origin_x = content.left() + header_width;

    if pristine {
        paint_invitation(ui, body, scale.leading_padding());
    } else {
        let header_clip = Rect::from_min_max(
            pos2(visible.left(), body.top()),
            pos2(time_viewport.left(), body.bottom()),
        );
        let header_painter = painter.with_clip_rect(header_clip);
        for (index, track) in view.tracks.iter().enumerate() {
            let top = content.top() + config::TIMELINE_RULER_HEIGHT + track_height * index as f32;
            let row = Rect::from_min_size(
                pos2(time_origin_x, top),
                vec2(content.width() - header_width, track_height),
            );
            if !row.intersects(body) {
                continue;
            }
            if index % 2 == 1 {
                body_painter.rect_filled(row, CornerRadius::ZERO, Palette::TRACK_ALT_BG);
            }
            body_painter.line_segment(
                [pos2(body.left(), top), pos2(body.right(), top)],
                Stroke::new(1.0, Palette::BORDER_SUBTLE),
            );
            track_view::paint_header(
                &header_painter,
                Rect::from_min_size(pos2(visible.left(), top), vec2(header_width, track_height)),
                track,
            );
            for item in &track.items {
                clip_view::paint(&body_painter, row, scale, item, track.kind, demo);
            }
        }
        painter.line_segment(
            [
                pos2(time_viewport.left(), visible.top()),
                pos2(time_viewport.left(), visible.bottom()),
            ],
            Stroke::new(1.0, Palette::BORDER),
        );
    }

    paint_ruler(&painter.with_clip_rect(ruler), ruler, scale, time_origin_x);
    playhead::paint(
        &painter.with_clip_rect(time_viewport),
        ruler.top(),
        body.bottom(),
        time_origin_x + playhead::x_for_millis(scale, config::PLAYHEAD_MILLIS),
    );
}

fn paint_ruler(painter: &Painter, rect: Rect, scale: &TimelineScale, origin_x: f32) {
    painter.rect_filled(rect, CornerRadius::ZERO, Palette::RULER_BG);
    let interval = scale.major_tick_interval() as f64;
    for second in scale.ticks_in_view(rect.left() - origin_x, rect.right() - origin_x) {
        let x = origin_x + scale.x_for_seconds(second as f64);
        painter.line_segment(
            [pos2(x, rect.top() + 3.0), pos2(x, rect.top() + 13.0)],
            Stroke::new(1.0, Palette::TEXT_DISABLED),
        );
        // Minor ticks are generated even when their preceding major tick is offscreen.
        for division in 1..10 {
            let minor_x = origin_x
                + scale.x_for_seconds(second as f64 + interval * f64::from(division) / 10.0);
            painter.line_segment(
                [
                    pos2(minor_x, rect.top() + 4.0),
                    pos2(minor_x, rect.top() + 9.0),
                ],
                Stroke::new(1.0, Palette::BORDER),
            );
        }
        if x >= rect.left() && x + 40.0 < rect.right() {
            painter.text(
                pos2(x + 4.0, rect.top() + 9.0),
                Align2::LEFT_CENTER,
                format_ruler_label(second),
                FontId::proportional(10.0),
                Palette::TEXT_MUTED,
            );
        }
    }
}

fn invitation_rect(body: Rect, leading_padding: f32) -> Rect {
    let height = config::TIMELINE_INVITATION_HEIGHT.min(body.height().max(0.0));
    Rect::from_min_max(
        pos2(
            (body.left() + leading_padding).min(body.right()),
            body.center().y - height / 2.0,
        ),
        pos2(
            (body.right() - 10.0).max(body.left() + leading_padding),
            body.center().y + height / 2.0,
        ),
    )
}

fn paint_invitation(ui: &Ui, body: Rect, leading_padding: f32) {
    let rect = invitation_rect(body, leading_padding);
    if rect.width() < 100.0 || rect.height() < 30.0 {
        return;
    }
    let painter = ui.painter_at(body);
    painter.rect_filled(rect, Radii::SMALL, Palette::TIMELINE_DROP_BG);
    // Original dotted outline; the reference treats this as a canvas region,
    // so only a small corner radius separates it from the surrounding timeline.
    let radius = f32::from(Radii::SMALL.nw).min(rect.height() / 2.0);
    let mut path = Vec::with_capacity(37);
    for (center, start_angle) in [
        (
            pos2(rect.right() - radius, rect.top() + radius),
            -std::f32::consts::FRAC_PI_2,
        ),
        (pos2(rect.right() - radius, rect.bottom() - radius), 0.0),
        (
            pos2(rect.left() + radius, rect.bottom() - radius),
            std::f32::consts::FRAC_PI_2,
        ),
        (
            pos2(rect.left() + radius, rect.top() + radius),
            std::f32::consts::PI,
        ),
    ] {
        for step in 0..=8 {
            let angle = start_angle + step as f32 / 8.0 * std::f32::consts::FRAC_PI_2;
            path.push(center + radius * vec2(angle.cos(), angle.sin()));
        }
    }
    if let Some(first) = path.first().copied() {
        path.push(first);
    }
    painter.extend(Shape::dotted_line(
        &path,
        Palette::TIMELINE_DROP_BORDER,
        4.0,
        0.6,
    ));
    icons::paint(
        &painter,
        Rect::from_center_size(pos2(rect.left() + 31.0, rect.center().y), vec2(18.0, 18.0)),
        Icon::Media,
        Palette::TEXT_SECONDARY,
    );
    painter.text(
        pos2(rect.left() + 55.0, rect.center().y),
        Align2::LEFT_CENTER,
        INVITATION,
        FontId::proportional(12.0),
        Palette::TEXT_SECONDARY,
    );
    ui.interact(
        rect,
        ui.scope_id().with("timeline_drop_placeholder"),
        Sense::hover(),
    )
    .on_hover_cursor(egui::CursorIcon::NotAllowed)
    .on_hover_text(DROP_UNAVAILABLE);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline_state::{TimelineItemView, TimelineTrackView, TrackKind};

    fn input(width: f32, height: f32) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(width, height))),
            ..Default::default()
        }
    }

    fn labels(output: &egui::FullOutput) -> Vec<String> {
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    }

    fn track(label: &str) -> TimelineTrackView {
        TimelineTrackView {
            label: label.to_owned(),
            name: "Supplied track".to_owned(),
            kind: TrackKind::Video,
            items: Vec::new(),
        }
    }

    #[test]
    fn pristine_canvas_has_one_invitation_and_no_rows_or_demo_content() {
        for (width, height) in [(960.0, 280.0), (1600.0, 400.0), (1861.0, 486.0)] {
            let context = egui::Context::default();
            crate::theme::configure(&context);
            let mut session = EditorSession::default();
            let output = context.run_ui(input(width, height), |ui| show(ui, &mut session));
            let text = labels(&output);
            assert_eq!(
                text.iter()
                    .filter(|text| text.as_str() == INVITATION)
                    .count(),
                1
            );
            assert!(text.iter().any(|text| text == "00:00"));
            assert!(!text.iter().any(|text| text.contains("Demo")
                || text.contains("DEMO")
                || text.contains("No clips")));
            for label in ["V1", "V2", "V3", "A1", "A2", "A3", "TRACKS"] {
                assert!(!text.iter().any(|text| text == label));
            }
            output.drop_without_applying_deltas();
        }
    }

    #[test]
    fn supplied_empty_and_populated_tracks_render_without_the_invitation() {
        let context = egui::Context::default();
        let mut session = EditorSession::default();
        session.timeline_view.tracks.push(track("V7"));
        let output = context.run_ui(input(1000.0, 360.0), |ui| show(ui, &mut session));
        assert!(labels(&output).iter().any(|text| text == "V7"));
        assert!(!labels(&output).iter().any(|text| text == INVITATION));
        output.drop_without_applying_deltas();
        session.timeline_view.tracks.push(track("V8"));
        session.timeline_view.tracks[1].items.push(
            TimelineItemView::new("Supplied item", 1.0, 8.0).expect("valid display geometry"),
        );
        let before = session.timeline_view.clone();
        let output = context.run_ui(input(1000.0, 360.0), |ui| show(ui, &mut session));
        let text = labels(&output);
        for expected in ["V7", "V8", "Supplied item"] {
            assert!(text.iter().any(|text| text == expected));
        }
        assert!(!text.iter().any(|text| text == "DEMO" || text == INVITATION));
        assert_eq!(session.timeline_view, before);
        output.drop_without_applying_deltas();
    }

    #[test]
    fn rendering_demo_never_changes_supplied_content() {
        let context = egui::Context::default();
        let mut session = EditorSession::default();
        session.timeline_view.tracks.push(track("V7"));
        let before = session.timeline_view.clone();
        session.set_timeline_demo(true);
        let output = context.run_ui(input(1000.0, 360.0), |ui| show(ui, &mut session));
        assert!(labels(&output).iter().any(|text| text == "Demo title"));
        assert_eq!(session.timeline_view, before);
        output.drop_without_applying_deltas();
        session.set_timeline_demo(false);
        assert_eq!(session.displayed_timeline(), &before);
    }

    #[test]
    fn drop_events_do_not_import_or_synthesize_items() {
        let context = egui::Context::default();
        let mut session = EditorSession::default();
        #[derive(Debug)]
        struct DroppedFile(std::path::PathBuf);
        impl egui::DroppedFile for DroppedFile {
            fn path(&self) -> &std::path::Path {
                &self.0
            }
            fn bytes(&self) -> Result<Vec<u8>, String> {
                Ok(Vec::new())
            }
        }

        let mut dropped = input(1000.0, 360.0);
        dropped
            .dropped_files
            .push(std::sync::Arc::new(DroppedFile(std::path::PathBuf::from(
                "not-imported.mp4",
            ))));
        let output = context.run_ui(dropped, |ui| show(ui, &mut session));
        assert!(session.timeline_view.tracks.is_empty());
        assert_eq!(session.timeline_display_mode, TimelineDisplayMode::Project);
        output.drop_without_applying_deltas();
    }

    #[test]
    fn fresh_session_resets_scroll_even_after_demo_used_the_same_widget() {
        let context = egui::Context::default();
        let mut session = EditorSession::default();
        session.set_timeline_demo(true);
        session.timeline_scroll[0] = 900.0;
        let output = context.run_ui(input(1000.0, 360.0), |ui| show(ui, &mut session));
        output.drop_without_applying_deltas();
        let mut fresh = EditorSession::default();
        let output = context.run_ui(input(1000.0, 360.0), |ui| show(ui, &mut fresh));
        assert_eq!(fresh.timeline_scroll, [0.0; 2]);
        assert!(labels(&output).iter().any(|text| text == "00:00"));
        output.drop_without_applying_deltas();
    }

    #[test]
    fn zero_playhead_and_invitation_align_at_the_canvas_leading_padding() {
        let context = egui::Context::default();
        let mut session = EditorSession::default();
        let output = context.run_ui(input(1000.0, 360.0), |ui| show(ui, &mut session));
        let playhead = output.shapes.iter().find_map(|clipped| {
            if let Shape::LineSegment { points, stroke } = &clipped.shape {
                (stroke.color == Palette::PLAYHEAD && points[0].x == points[1].x)
                    .then_some((points[0].x, clipped.clip_rect))
            } else {
                None
            }
        });
        let (x, clip) = playhead.expect("time-zero playhead");
        assert!((x - clip.left() - config::TIMELINE_EMPTY_LEADING_PADDING).abs() < 0.01);
        assert!(clip.contains(pos2(x, clip.center().y)));
        output.drop_without_applying_deltas();
    }

    #[test]
    fn invitation_fills_canvas_width_and_keeps_reference_height_on_resize() {
        for width in [960.0, 1600.0, 1861.0] {
            let body = Rect::from_min_size(pos2(0.0, 58.0), vec2(width, 400.0));
            let invitation = invitation_rect(body, config::TIMELINE_EMPTY_LEADING_PADDING);
            assert_eq!(invitation.left(), 78.0);
            assert_eq!(invitation.right(), width - 10.0);
            assert_eq!(invitation.height(), 68.0);
            assert_eq!(invitation.center().y, body.center().y);
            assert!(body.contains_rect(invitation));
        }
    }
}
