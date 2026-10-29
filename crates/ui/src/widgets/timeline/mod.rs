//! Timeline presentation: pristine canvas, supplied rows/items, or explicit QA.
//! Import, drop, selection, and editing controls are deliberately disabled.

pub mod clip_view;
pub mod clip_visuals;
pub mod playhead;
pub mod scale;
pub mod toolbar;
pub mod track_view;

use egui::{
    pos2, vec2, Align2, CornerRadius, FontId, Painter, Pos2, Rect, ScrollArea, Sense, Shape,
    Stroke, Ui,
};

use std::collections::BTreeSet;

use crate::config;
use crate::editor_state::EditorSession;
use crate::shortcuts::ShortcutManager;
use crate::theme::{Palette, Radii};
use crate::timeline_state::{
    ThumbnailMode, TimelineContentState, TimelineDisplayMode, TimelineItemId, TimelineView,
};
use crate::widgets::icons::{self, Icon, IconColors, IconSize};

use self::clip_visuals::TimelineVisualCache;
use self::scale::{format_ruler_label, TimelineScale};

const INVITATION: &str = "Drag material here and start to create";
const DROP_UNAVAILABLE: &str =
    "Placeholder only: media import and drag-and-drop are not implemented.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimelineClick {
    Select(TimelineItemId, bool),
    Clear,
}

pub fn show(
    ui: &mut Ui,
    session: &mut EditorSession,
    shortcuts: &ShortcutManager,
    assets: &mut TimelineVisualCache,
) {
    session.prune_timeline_selection();
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        session.timeline_pixels_per_second =
            TimelineScale::new(session.timeline_pixels_per_second, 0.0).pixels_per_second();
        toolbar::show(ui, session, shortcuts);
        let demo = session.timeline_display_mode == TimelineDisplayMode::VisualDemo;
        assets.set_demo(demo);
        assets.begin_frame();

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
        let mut click = None;
        let output = ScrollArea::new([true, rows_height > body_height])
            .id_salt("timeline_scroll")
            .scroll_offset(vec2(session.timeline_scroll[0], session.timeline_scroll[1]))
            .auto_shrink([false, false])
            .max_height(available.y)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show_viewport(ui, |ui, viewport| {
                let (content, response) =
                    ui.allocate_exact_size(vec2(content_width, content_height), Sense::click());
                let visible = viewport
                    .translate(content.min.to_vec2())
                    .intersect(ui.clip_rect());
                let hovered = ui
                    .input(|input| input.pointer.hover_pos())
                    .and_then(|position| {
                        item_at(view, content, header_width, track_height, &scale, position)
                    });
                paint_content(
                    PaintContext {
                        ui,
                        area: PaintedArea { content, visible },
                        scale: &scale,
                        view,
                        track_height,
                        selection: session.timeline_selection(),
                        options: PaintOptions {
                            demo,
                            hovered,
                            show_thumbnails: session.timeline_show_thumbnails,
                            thumbnail_mode: session.timeline_thumbnail_mode,
                            show_waveforms: session.timeline_show_waveforms,
                        },
                    },
                    assets,
                );
                if response.clicked() {
                    if let Some(position) = response.interact_pointer_pos() {
                        let additive =
                            ui.input(|input| input.modifiers.command || input.modifiers.shift);
                        click = click_target(
                            view,
                            content,
                            header_width,
                            track_height,
                            &scale,
                            position,
                            additive,
                        );
                    }
                }
                if let Some(id) = hovered {
                    if let Some(label) = view
                        .tracks
                        .get(id.track)
                        .and_then(|track| track.items.get(id.item))
                        .map(|item| item.label().to_owned())
                    {
                        let _ = response
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text(label);
                    }
                }
            });
        let offset = output.state.clamped_offset();
        session.timeline_scroll = [offset.x, offset.y];
        match click {
            Some(TimelineClick::Select(id, additive)) => {
                session.select_timeline_item(id, additive);
            }
            Some(TimelineClick::Clear) => session.clear_timeline_selection(),
            None => {}
        }
    });
}

/// Resolve a clip under the pointer. Track headers and the ruler return none.
fn item_at(
    view: &TimelineView,
    content: Rect,
    header_width: f32,
    track_height: f32,
    scale: &TimelineScale,
    position: Pos2,
) -> Option<TimelineItemId> {
    if position.x < content.left() + header_width
        || position.y < content.top() + config::TIMELINE_RULER_HEIGHT
    {
        return None;
    }
    for (track_index, track) in view.tracks.iter().enumerate() {
        let top = content.top() + config::TIMELINE_RULER_HEIGHT + track_height * track_index as f32;
        let row = Rect::from_min_size(
            pos2(content.left() + header_width, top),
            vec2(content.width() - header_width, track_height),
        );
        for (item_index, item) in track.items.iter().enumerate() {
            let clip_rect = clip_view::rect(row, scale, item);
            if clip_rect.width() >= 2.0 && clip_rect.contains(position) {
                return Some(TimelineItemId {
                    track: track_index,
                    item: item_index,
                });
            }
        }
    }
    None
}

/// Resolve a click inside the scrollable content. Track headers and the ruler
/// never select; an empty body area clears the timeline selection.
fn click_target(
    view: &TimelineView,
    content: Rect,
    header_width: f32,
    track_height: f32,
    scale: &TimelineScale,
    position: Pos2,
    additive: bool,
) -> Option<TimelineClick> {
    let body = Rect::from_min_max(
        pos2(
            content.left() + header_width,
            content.top() + config::TIMELINE_RULER_HEIGHT,
        ),
        content.max,
    );
    if !body.contains(position) {
        return None;
    }
    if let Some(id) = item_at(view, content, header_width, track_height, scale, position) {
        return Some(TimelineClick::Select(id, additive));
    }
    Some(TimelineClick::Clear)
}

#[derive(Debug, Clone, Copy)]
struct PaintedArea {
    content: Rect,
    visible: Rect,
}

#[derive(Debug, Clone, Copy)]
struct PaintOptions {
    demo: bool,
    hovered: Option<TimelineItemId>,
    show_thumbnails: bool,
    thumbnail_mode: ThumbnailMode,
    show_waveforms: bool,
}

struct PaintContext<'a> {
    ui: &'a Ui,
    area: PaintedArea,
    scale: &'a TimelineScale,
    view: &'a TimelineView,
    track_height: f32,
    selection: &'a BTreeSet<TimelineItemId>,
    options: PaintOptions,
}

fn paint_content(context: PaintContext<'_>, assets: &mut TimelineVisualCache) {
    let PaintContext {
        ui,
        area,
        scale,
        view,
        track_height,
        selection,
        options,
    } = context;
    let PaintedArea { content, visible } = area;
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
            for (item_index, item) in track.items.iter().enumerate() {
                let id = TimelineItemId {
                    track: index,
                    item: item_index,
                };
                clip_view::paint(
                    &body_painter,
                    row,
                    clip_view::ClipPaint {
                        scale,
                        item,
                        track_kind: track.kind,
                        demo: options.demo,
                        selected: selection.contains(&id),
                        hovered: options.hovered == Some(id),
                        show_thumbnails: options.show_thumbnails,
                        thumbnail_mode: options.thumbnail_mode,
                        show_waveforms: options.show_waveforms,
                    },
                    assets,
                );
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
    icons::paint_sized(
        &painter,
        Rect::from_center_size(pos2(rect.left() + 31.0, rect.center().y), vec2(18.0, 18.0)),
        Icon::Media,
        IconSize::Standard,
        IconColors::MUTED,
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
            let output = context.run_ui(input(width, height), |ui| {
                show(
                    ui,
                    &mut session,
                    &ShortcutManager::default(),
                    &mut TimelineVisualCache::default(),
                )
            });
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
        let output = context.run_ui(input(1000.0, 360.0), |ui| {
            show(
                ui,
                &mut session,
                &ShortcutManager::default(),
                &mut TimelineVisualCache::default(),
            )
        });
        assert!(labels(&output).iter().any(|text| text == "V7"));
        assert!(!labels(&output).iter().any(|text| text == INVITATION));
        output.drop_without_applying_deltas();
        session.timeline_view.tracks.push(track("V8"));
        session.timeline_view.tracks[1].items.push(
            TimelineItemView::new("Supplied item", 1.0, 8.0).expect("valid display geometry"),
        );
        let before = session.timeline_view.clone();
        let output = context.run_ui(input(1000.0, 360.0), |ui| {
            show(
                ui,
                &mut session,
                &ShortcutManager::default(),
                &mut TimelineVisualCache::default(),
            )
        });
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
        let output = context.run_ui(input(1000.0, 360.0), |ui| {
            show(
                ui,
                &mut session,
                &ShortcutManager::default(),
                &mut TimelineVisualCache::default(),
            )
        });
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
        let output = context.run_ui(dropped, |ui| {
            show(
                ui,
                &mut session,
                &ShortcutManager::default(),
                &mut TimelineVisualCache::default(),
            )
        });
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
        let output = context.run_ui(input(1000.0, 360.0), |ui| {
            show(
                ui,
                &mut session,
                &ShortcutManager::default(),
                &mut TimelineVisualCache::default(),
            )
        });
        output.drop_without_applying_deltas();
        let mut fresh = EditorSession::default();
        let output = context.run_ui(input(1000.0, 360.0), |ui| {
            show(
                ui,
                &mut fresh,
                &ShortcutManager::default(),
                &mut TimelineVisualCache::default(),
            )
        });
        assert_eq!(fresh.timeline_scroll, [0.0; 2]);
        assert!(labels(&output).iter().any(|text| text == "00:00"));
        output.drop_without_applying_deltas();
    }

    #[test]
    fn selected_rich_demo_clip_paints_a_high_contrast_outline() {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let mut session = EditorSession::default();
        session.set_timeline_demo(true);
        assert!(session.select_timeline_item(TimelineItemId { track: 2, item: 1 }, false));
        let output = context.run_ui(input(1200.0, 420.0), |ui| {
            show(
                ui,
                &mut session,
                &ShortcutManager::default(),
                &mut TimelineVisualCache::default(),
            )
        });
        let outlined = output.shapes.iter().any(|clipped| match &clipped.shape {
            Shape::Rect(rect) => {
                rect.stroke.color == Palette::TEXT_PRIMARY && rect.stroke.width >= 2.0
            }
            _ => false,
        });
        output.drop_without_applying_deltas();
        assert!(
            outlined,
            "selected clip should paint a high-contrast outline"
        );
    }

    #[test]
    fn zero_playhead_and_invitation_align_at_the_canvas_leading_padding() {
        let context = egui::Context::default();
        let mut session = EditorSession::default();
        let output = context.run_ui(input(1000.0, 360.0), |ui| {
            show(
                ui,
                &mut session,
                &ShortcutManager::default(),
                &mut TimelineVisualCache::default(),
            )
        });
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

    fn item_view() -> TimelineView {
        TimelineView {
            tracks: vec![TimelineTrackView {
                label: "V1".to_owned(),
                name: "Video".to_owned(),
                kind: TrackKind::Video,
                items: vec![TimelineItemView::new("Supplied clip", 1.0, 8.0).expect("valid")],
            }],
        }
    }

    #[test]
    fn click_target_selects_clips_and_clears_only_empty_body_area() {
        let view = item_view();
        let content = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 400.0));
        let scale = TimelineScale::new(30.0, config::TIMELINE_LEADING_PADDING);
        let clip_rect = clip_view::rect(
            Rect::from_min_size(
                pos2(config::TRACK_HEADER_WIDTH, config::TIMELINE_RULER_HEIGHT),
                vec2(1000.0 - config::TRACK_HEADER_WIDTH, 58.0),
            ),
            &scale,
            &view.tracks[0].items[0],
        );
        let id = TimelineItemId { track: 0, item: 0 };

        assert_eq!(
            click_target(
                &view,
                content,
                config::TRACK_HEADER_WIDTH,
                58.0,
                &scale,
                clip_rect.center(),
                false,
            ),
            Some(TimelineClick::Select(id, false))
        );
        assert_eq!(
            click_target(
                &view,
                content,
                config::TRACK_HEADER_WIDTH,
                58.0,
                &scale,
                clip_rect.center(),
                true,
            ),
            Some(TimelineClick::Select(id, true))
        );
        assert_eq!(
            click_target(
                &view,
                content,
                config::TRACK_HEADER_WIDTH,
                58.0,
                &scale,
                pos2(clip_rect.right() + 40.0, clip_rect.center().y),
                false,
            ),
            Some(TimelineClick::Clear)
        );
        assert_eq!(
            click_target(
                &view,
                content,
                config::TRACK_HEADER_WIDTH,
                58.0,
                &scale,
                pos2(40.0, clip_rect.center().y),
                false,
            ),
            None,
            "track headers must not select or clear"
        );
        assert_eq!(
            click_target(
                &view,
                content,
                config::TRACK_HEADER_WIDTH,
                58.0,
                &scale,
                pos2(clip_rect.center().x, 10.0),
                false,
            ),
            None,
            "the ruler must not select or clear"
        );
    }

    fn click_at(context: &egui::Context, position: egui::Pos2, session: &mut EditorSession) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 360.0))),
            events: vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        let output = context.run_ui(input, |ui| {
            show(
                ui,
                session,
                &ShortcutManager::default(),
                &mut TimelineVisualCache::default(),
            )
        });
        output.drop_without_applying_deltas();
    }

    fn render_session(context: &egui::Context, session: &mut EditorSession) -> egui::FullOutput {
        context.run_ui(input(1000.0, 360.0), |ui| {
            show(
                ui,
                session,
                &ShortcutManager::default(),
                &mut TimelineVisualCache::default(),
            )
        })
    }

    fn video_clip_rect(output: &egui::FullOutput) -> Rect {
        output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                Shape::Rect(rect)
                    if rect.fill == Palette::ACCENT_BG
                        && rect.rect.width() > 50.0
                        && rect.rect.height() > 20.0 =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .expect("supplied video clip should paint a block")
    }

    #[test]
    fn clicking_a_clip_selects_it_and_empty_canvas_clears_it_through_show() {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let mut session = EditorSession {
            timeline_view: item_view(),
            ..EditorSession::default()
        };

        let first = render_session(&context, &mut session);
        let clip = video_clip_rect(&first);
        first.drop_without_applying_deltas();
        assert!(session.timeline_selection().is_empty());

        click_at(&context, clip.center(), &mut session);
        assert_eq!(
            session.timeline_selection_context().kind(),
            Some(crate::timeline_state::TimelineSelectionKind::VideoOnly)
        );

        let empty = pos2(clip.right() + 60.0, clip.center().y);
        let frame = render_session(&context, &mut session);
        frame.drop_without_applying_deltas();
        click_at(&context, empty, &mut session);
        assert!(session.timeline_selection().is_empty());
        assert_eq!(session.timeline_selection_context().kind(), None);
    }

    #[test]
    fn clicking_a_track_header_does_not_select_or_clear_a_clip() {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let mut session = EditorSession {
            timeline_view: item_view(),
            ..EditorSession::default()
        };

        let first = render_session(&context, &mut session);
        let clip = video_clip_rect(&first);
        first.drop_without_applying_deltas();
        click_at(&context, clip.center(), &mut session);
        assert_eq!(session.timeline_selection().len(), 1);

        click_at(
            &context,
            pos2(clip.left() - 120.0, clip.center().y),
            &mut session,
        );
        assert_eq!(session.timeline_selection().len(), 1);
    }
}
