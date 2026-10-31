//! Editor composition: compact project bar, resizable three-column workspace,
//! expandable timeline, and the export modal.

use std::sync::{Arc, Mutex};

use capcut_renderer::PreviewRenderer;
use egui::{pos2, vec2, CornerRadius, CursorIcon, Rect, Sense, Stroke, Ui};

use crate::app_state::AppState;
use crate::config;
use crate::editor_state::EditorSession;
use crate::media_library_state::MediaLibraryState;
use crate::media_preview_state::MediaPreview;
use crate::theme::Palette;
use crate::widgets::media_pool::MediaPoolAction;
use crate::widgets::timeline::clip_visuals::TimelineVisualCache;
use crate::widgets::toolbar::TopBarAction;
use crate::widgets::{export_dialog, inspector, media_pool, panel, preview, timeline, toolbar};
use crate::workspace_layout;

pub fn show(ui: &mut Ui, state: &mut AppState, timeline_visuals: &mut TimelineVisualCache) {
    let context = ui.ctx().clone();

    if !state.has_session() {
        state.go_home();
        return;
    }

    let gpu_renderer = state.preview_renderer();
    let (action, media_pool_action) = {
        let Some((session, shortcuts, media, media_library)) = state.session_shortcuts_media_mut()
        else {
            return;
        };

        let action = toolbar::show(ui, session, shortcuts);
        let mut media_pool_action = None;

        paint_editor_background(ui);
        let available = ui.available_size();
        let (area, _) = ui.allocate_exact_size(available, Sense::hover());
        let margin = config::PANEL_OUTER_MARGIN;
        let gap = config::WORKSPACE_SPLITTER_WIDTH;
        let content = Rect::from_min_max(
            area.min + vec2(margin, margin),
            area.max - vec2(margin, margin),
        );
        if content.width() > 1.0 && content.height() > 1.0 {
            let initial_height = if session.timeline_visible {
                workspace_layout::timeline_height(
                    available.y,
                    content.height(),
                    session.timeline_height_fraction,
                )
            } else {
                0.0
            };
            let initial_workspace_height = (content.height()
                - initial_height
                - if session.timeline_visible { gap } else { 0.0 })
            .max(1.0);
            if session.timeline_visible && initial_height > 1.0 {
                let splitter_rect = Rect::from_min_size(
                    pos2(content.left(), content.top() + initial_workspace_height),
                    vec2(content.width(), gap),
                );
                timeline_splitter(ui, splitter_rect, session, available.y, content.height());
            }

            let timeline_height = if session.timeline_visible {
                workspace_layout::timeline_height(
                    available.y,
                    content.height(),
                    session.timeline_height_fraction,
                )
            } else {
                0.0
            };
            let workspace_height = (content.height()
                - timeline_height
                - if session.timeline_visible { gap } else { 0.0 })
            .max(1.0);
            let workspace_rect =
                Rect::from_min_size(content.min, vec2(content.width(), workspace_height));

            show_workspace(
                ui,
                workspace_rect,
                session,
                media,
                media_library,
                &mut media_pool_action,
                gpu_renderer.as_ref(),
            );

            if session.timeline_visible && timeline_height > 1.0 {
                let timeline_rect = Rect::from_min_size(
                    pos2(content.left(), workspace_rect.bottom() + gap),
                    vec2(content.width(), timeline_height),
                );
                panel::show(ui, timeline_rect, Palette::PANEL_BG, |ui| {
                    timeline::show(ui, session, shortcuts, timeline_visuals);
                });
            }
        }

        if session.export_open {
            export_dialog::show(&context, session);
        }

        if action == TopBarAction::Export {
            session.export_open = true;
        }

        (action, media_pool_action)
    };

    match media_pool_action {
        Some(MediaPoolAction::Import) => state.open_media_import_dialog(),
        Some(MediaPoolAction::Append(asset)) => {
            let _ = state.append_media_asset(asset);
            context.request_repaint();
        }
        Some(MediaPoolAction::Relink(asset)) => {
            state.open_media_relink_dialog(asset);
            context.request_repaint();
        }
        None => {}
    }

    match action {
        TopBarAction::Home => state.go_home(),
        TopBarAction::Shortcuts => {
            state.open_shortcuts();
            context.request_repaint();
        }
        TopBarAction::NewProject => state.request_new_project(),
        TopBarAction::OpenProject => state.request_open_native_project(),
        TopBarAction::RenameProject => state.begin_active_project_rename(),
        TopBarAction::DuplicateProject => {
            if let Some(project_id) = state
                .session()
                .and_then(EditorSession::native_document)
                .map(|document| document.project_id().clone())
            {
                let _ = state.duplicate_library_project(&project_id);
            }
        }
        TopBarAction::SaveRecoveryCopy => {
            if let Err(error) = state.preserve_conflict_copy() {
                state.report_error(error);
            }
        }
        TopBarAction::None | TopBarAction::Export => {}
    }
}

fn paint_editor_background(ui: &Ui) {
    let rect = ui.available_rect_before_wrap();
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, Palette::APP_BG);
}

fn show_workspace(
    ui: &mut Ui,
    rect: Rect,
    session: &mut EditorSession,
    media: &MediaPreview,
    media_library: &mut MediaLibraryState,
    media_pool_action: &mut Option<MediaPoolAction>,
    gpu_renderer: Option<&Arc<Mutex<PreviewRenderer>>>,
) {
    let show_left = session.tools_visible;
    let show_right = session.inspector_visible;
    let gap = config::WORKSPACE_SPLITTER_WIDTH;
    let splitter_count = usize::from(show_left) + usize::from(show_right);
    let content_width = (rect.width() - gap * splitter_count as f32).max(0.0);
    let widths = workspace_layout::workspace_widths(
        content_width,
        show_left,
        show_right,
        session.left_panel_fraction,
        session.right_panel_fraction,
    );

    let mut x = rect.left();
    if show_left {
        add_panel(
            ui,
            Rect::from_min_size(pos2(x, rect.top()), vec2(widths.left, rect.height())),
            |ui| *media_pool_action = media_pool::show(ui, session, media_library),
        );
        x += widths.left;
        splitter(
            ui,
            Rect::from_min_size(pos2(x, rect.top()), vec2(gap, rect.height())),
            &mut session.left_panel_fraction,
            true,
            content_width,
        );
        x += gap;
    }

    add_panel(
        ui,
        Rect::from_min_size(pos2(x, rect.top()), vec2(widths.center, rect.height())),
        |ui| preview::show(ui, session, media, gpu_renderer),
    );
    x += widths.center;

    if show_right {
        splitter(
            ui,
            Rect::from_min_size(pos2(x, rect.top()), vec2(gap, rect.height())),
            &mut session.right_panel_fraction,
            false,
            content_width,
        );
        x += gap;
        add_panel(
            ui,
            Rect::from_min_size(pos2(x, rect.top()), vec2(widths.right, rect.height())),
            |ui| inspector::show(ui, session),
        );
    }
}

fn add_panel(ui: &mut Ui, rect: Rect, add_contents: impl FnOnce(&mut Ui)) {
    panel::show(ui, rect, Palette::PANEL_BG, add_contents);
}

fn timeline_splitter(
    ui: &mut Ui,
    rect: Rect,
    session: &mut EditorSession,
    available_height: f32,
    content_height: f32,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let response = ui
        .interact(
            rect,
            ui.scope_id().with("timeline_splitter"),
            Sense::click_and_drag(),
        )
        .on_hover_cursor(CursorIcon::ResizeVertical);

    if response.hovered() || response.dragged() {
        ui.painter().line_segment(
            [
                pos2(rect.left() + 8.0, rect.center().y),
                pos2(rect.right() - 8.0, rect.center().y),
            ],
            Stroke::new(2.0, Palette::ACCENT),
        );
    }

    if response.double_clicked() {
        session.set_timeline_height_fraction(None);
        session.request_layout_save();
        return;
    }

    if response.dragged() {
        let delta = -response.drag_delta().y;
        if delta.is_finite() && delta != 0.0 && available_height > 1.0 {
            let heights = workspace_layout::timeline_heights(available_height, content_height);
            let current = workspace_layout::timeline_height(
                available_height,
                content_height,
                session.timeline_height_fraction,
            );
            let next = (current + delta).clamp(heights.min, heights.max);
            if (next - current).abs() > f32::EPSILON {
                session.set_timeline_height_fraction(Some(next / available_height));
            }
        }
    }

    if response.drag_stopped() {
        session.request_layout_save();
    }
}

fn splitter(ui: &mut Ui, rect: Rect, fraction: &mut f32, is_left: bool, content_width: f32) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let response = ui.interact(
        rect,
        ui.scope_id().with(("workspace_splitter", is_left)),
        Sense::click_and_drag(),
    );
    let response = response.on_hover_cursor(CursorIcon::ResizeHorizontal);
    let active = response.hovered() || response.dragged();

    if active {
        ui.painter().line_segment(
            [
                pos2(rect.center().x, rect.top() + 8.0),
                pos2(rect.center().x, rect.bottom() - 8.0),
            ],
            Stroke::new(2.0, Palette::ACCENT),
        );
    }

    if response.dragged() && content_width > 1.0 {
        let delta = response.drag_delta().x;
        if delta.is_finite() && delta != 0.0 {
            let change = delta / content_width;
            if is_left {
                *fraction = (*fraction + change).clamp(0.18, config::WORKSPACE_LEFT_MAX_FRACTION);
            } else {
                *fraction = (*fraction - change).clamp(0.18, config::WORKSPACE_RIGHT_MAX_FRACTION);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline_state::{TimelineItemId, TimelineItemView, TimelineTrackView, TrackKind};
    use egui::{Pos2, Rect};

    fn frame_input(width: f32, height: f32, time: f64, events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, height))),
            time: Some(time),
            events,
            ..Default::default()
        }
    }

    fn run_editor_frame(
        context: &egui::Context,
        state: &mut AppState,
        input: egui::RawInput,
    ) -> egui::FullOutput {
        context.run_ui(input, |ui| {
            show(ui, state, &mut TimelineVisualCache::default())
        })
    }

    fn splitter_position(output: &egui::FullOutput, width: f32) -> Pos2 {
        let panels = rounded_panel_rects(output);
        let timeline = *panels
            .iter()
            .max_by(|a, b| a.top().total_cmp(&b.top()))
            .expect("timeline panel");
        let upper_bottom = panels
            .iter()
            .filter(|rect| **rect != timeline)
            .map(Rect::bottom)
            .fold(0.0, f32::max);
        pos2(width * 0.5, (upper_bottom + timeline.top()) * 0.5)
    }

    fn selectable_session() -> AppState {
        let mut state = AppState::default();
        state.create_temporary_project();
        let session = state.session_mut().expect("session");
        session.timeline_pixels_per_second = 42.0;
        session.timeline_view.tracks = vec![TimelineTrackView {
            label: "V1".to_owned(),
            name: "Video".to_owned(),
            kind: TrackKind::Video,
            items: vec![TimelineItemView::new("Clip", 1.0, 4.0).expect("item")],
        }];
        assert!(session.select_timeline_item(TimelineItemId { track: 0, item: 0 }, false));
        state
    }

    #[test]
    fn dragging_splitter_resizes_without_touching_selection_or_zoom() {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let mut state = selectable_session();
        let (width, height) = (1600.0, 900.0);

        let first = run_editor_frame(
            &context,
            &mut state,
            frame_input(width, height, 0.0, vec![]),
        );
        let splitter = splitter_position(&first, width);
        let initial_height = rounded_panel_rects(&first)
            .iter()
            .max_by(|a, b| a.top().total_cmp(&b.top()))
            .expect("timeline")
            .height();
        first.drop_without_applying_deltas();

        let pressed = splitter;
        let dragged = pos2(splitter.x, splitter.y - 90.0);
        run_editor_frame(
            &context,
            &mut state,
            frame_input(width, height, 0.1, vec![egui::Event::PointerMoved(pressed)]),
        )
        .drop_without_applying_deltas();
        run_editor_frame(
            &context,
            &mut state,
            frame_input(
                width,
                height,
                0.2,
                vec![egui::Event::PointerButton {
                    pos: pressed,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }],
            ),
        )
        .drop_without_applying_deltas();
        run_editor_frame(
            &context,
            &mut state,
            frame_input(width, height, 0.3, vec![egui::Event::PointerMoved(dragged)]),
        )
        .drop_without_applying_deltas();
        let released = run_editor_frame(
            &context,
            &mut state,
            frame_input(
                width,
                height,
                0.4,
                vec![egui::Event::PointerButton {
                    pos: dragged,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            ),
        );

        let panels = rounded_panel_rects(&released);
        let final_height = panels
            .iter()
            .max_by(|a, b| a.top().total_cmp(&b.top()))
            .expect("timeline")
            .height();
        released.drop_without_applying_deltas();

        assert!(
            final_height > initial_height + 60.0,
            "timeline should grow after dragging up: {initial_height} -> {final_height}"
        );
        let session = state.session().expect("session");
        assert_eq!(session.timeline_pixels_per_second, 42.0);
        assert!(session.timeline_height_fraction.is_some());
        assert!(session.is_timeline_item_selected(TimelineItemId { track: 0, item: 0 }));
        assert!(
            state
                .session_mut()
                .expect("session")
                .take_layout_save_request(),
            "drag release must request persistence"
        );
    }

    #[test]
    fn splitter_drags_down_to_restore_a_smaller_timeline() {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let mut state = selectable_session();
        state
            .session_mut()
            .expect("session")
            .set_timeline_height_fraction(Some(0.70));
        let (width, height) = (1600.0, 900.0);

        let first = run_editor_frame(
            &context,
            &mut state,
            frame_input(width, height, 0.0, vec![]),
        );
        let splitter = splitter_position(&first, width);
        let initial_height = rounded_panel_rects(&first)
            .iter()
            .max_by(|a, b| a.top().total_cmp(&b.top()))
            .expect("timeline")
            .height();
        first.drop_without_applying_deltas();

        let pressed = splitter;
        let dragged = pos2(splitter.x, splitter.y + 60.0);
        run_editor_frame(
            &context,
            &mut state,
            frame_input(width, height, 0.1, vec![egui::Event::PointerMoved(pressed)]),
        )
        .drop_without_applying_deltas();
        run_editor_frame(
            &context,
            &mut state,
            frame_input(
                width,
                height,
                0.2,
                vec![egui::Event::PointerButton {
                    pos: pressed,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }],
            ),
        )
        .drop_without_applying_deltas();
        run_editor_frame(
            &context,
            &mut state,
            frame_input(width, height, 0.3, vec![egui::Event::PointerMoved(dragged)]),
        )
        .drop_without_applying_deltas();
        let released = run_editor_frame(
            &context,
            &mut state,
            frame_input(
                width,
                height,
                0.4,
                vec![egui::Event::PointerButton {
                    pos: dragged,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            ),
        );

        let final_height = rounded_panel_rects(&released)
            .iter()
            .max_by(|a, b| a.top().total_cmp(&b.top()))
            .expect("timeline")
            .height();
        released.drop_without_applying_deltas();

        assert!(
            final_height < initial_height - 40.0,
            "timeline should shrink after dragging down: {initial_height} -> {final_height}"
        );
    }

    #[test]
    fn double_clicking_splitter_restores_the_default_height() {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let mut state = selectable_session();
        state
            .session_mut()
            .expect("session")
            .set_timeline_height_fraction(Some(0.70));
        let (width, height) = (1600.0, 900.0);

        let first = run_editor_frame(
            &context,
            &mut state,
            frame_input(width, height, 0.0, vec![]),
        );
        let splitter = splitter_position(&first, width);
        first.drop_without_applying_deltas();

        for (index, time) in [0.2, 0.35].into_iter().enumerate() {
            let events = vec![
                egui::Event::PointerMoved(splitter),
                egui::Event::PointerButton {
                    pos: splitter,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos: splitter,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ];
            run_editor_frame(
                &context,
                &mut state,
                frame_input(width, height, 0.1 + time + index as f64 * 0.01, events),
            )
            .drop_without_applying_deltas();
        }

        assert_eq!(
            state.session().expect("session").timeline_height_fraction,
            None,
            "double-click must restore the responsive default"
        );
        assert!(state
            .session_mut()
            .expect("session")
            .take_layout_save_request());
    }

    fn timeline_and_upper_rects(output: &egui::FullOutput) -> (Rect, Vec<Rect>) {
        let panels = rounded_panel_rects(output);
        let timeline = *panels
            .iter()
            .max_by(|a, b| a.top().total_cmp(&b.top()))
            .expect("timeline");
        let mut upper: Vec<Rect> = panels
            .iter()
            .copied()
            .filter(|rect| *rect != timeline)
            .collect();
        upper.sort_by(|a, b| a.left().total_cmp(&b.left()));
        (timeline, upper)
    }

    #[test]
    fn maximum_timeline_height_keeps_the_upper_workspace_visible() {
        for (width, height) in [(960.0, 600.0), (1600.0, 900.0)] {
            let context = egui::Context::default();
            crate::theme::configure(&context);
            let mut state = selectable_session();
            state
                .session_mut()
                .expect("session")
                .set_timeline_height_fraction(Some(0.99));

            let output = run_editor_frame(
                &context,
                &mut state,
                frame_input(width, height, 0.0, vec![]),
            );
            let (timeline, upper) = timeline_and_upper_rects(&output);
            output.drop_without_applying_deltas();

            assert!(timeline.top() >= 0.0 && timeline.bottom() <= height);
            assert!(timeline.height() > 0.0);
            assert_eq!(upper.len(), 3);
            for panel in &upper {
                assert!(
                    panel.height() >= 179.5,
                    "upper workspace collapsed at {width}x{height}: {panel:?}"
                );
                assert!(panel.top() >= 0.0 && panel.bottom() <= height);
            }
            assert!(
                (5.0..=7.0).contains(&(timeline.top() - upper[0].bottom())),
                "splitter gap must remain visible at maximum height"
            );
        }
    }

    #[test]
    fn timeline_height_reflows_when_the_window_is_resized() {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let mut state = selectable_session();
        state
            .session_mut()
            .expect("session")
            .set_timeline_height_fraction(Some(0.70));

        let reference = run_editor_frame(
            &context,
            &mut state,
            frame_input(1600.0, 900.0, 0.0, vec![]),
        );
        let reference_height = timeline_and_upper_rects(&reference).0.height();
        reference.drop_without_applying_deltas();

        let compact =
            run_editor_frame(&context, &mut state, frame_input(960.0, 600.0, 0.1, vec![]));
        let (compact_timeline, compact_upper) = timeline_and_upper_rects(&compact);
        compact.drop_without_applying_deltas();
        assert!(compact_timeline.height() < reference_height);
        assert!(compact_upper[0].height() >= 179.5);

        let restored = run_editor_frame(
            &context,
            &mut state,
            frame_input(1600.0, 900.0, 0.2, vec![]),
        );
        let restored_height = timeline_and_upper_rects(&restored).0.height();
        restored.drop_without_applying_deltas();
        assert!(
            (restored_height - reference_height).abs() < 0.5,
            "the same preference must restore the same height: {restored_height} vs {reference_height}"
        );
    }

    fn rounded_panel_rects(output: &egui::FullOutput) -> Vec<Rect> {
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if shape.fill == Palette::PANEL_BG && shape.corner_radius.nw >= 8 =>
                {
                    Some(shape.rect)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn editor_renders_four_gapped_rounded_panels() {
        for (width, height) in [(960.0, 600.0), (1080.0, 720.0), (1600.0, 900.0)] {
            let context = egui::Context::default();
            crate::theme::configure(&context);
            let mut state = AppState::default();
            state.create_temporary_project();

            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, height))),
                ..Default::default()
            };
            let output = context.run_ui(input, |ui| {
                show(ui, &mut state, &mut TimelineVisualCache::default())
            });
            let panels = rounded_panel_rects(&output);
            output.drop_without_applying_deltas();

            assert_eq!(
                panels.len(),
                4,
                "expected four large rounded panel surfaces at {width}x{height}, found {panels:?}"
            );

            let timeline = *panels
                .iter()
                .max_by(|a, b| a.top().total_cmp(&b.top()))
                .expect("timeline panel");
            let mut upper: Vec<Rect> = panels
                .iter()
                .copied()
                .filter(|rect| *rect != timeline)
                .collect();
            upper.sort_by(|a, b| a.left().total_cmp(&b.left()));
            assert_eq!(upper.len(), 3);

            for pair in upper.windows(2) {
                let gap = pair[1].left() - pair[0].right();
                assert!(
                    (5.0..=7.0).contains(&gap),
                    "expected a visible 5-7 px panel gap at {width}x{height}, found {gap}"
                );
            }
            let vertical_gap = timeline.top() - upper[0].bottom();
            assert!(
                (5.0..=7.0).contains(&vertical_gap),
                "expected a visible workspace/timeline gap at {width}x{height}, found {vertical_gap}"
            );
            assert!(timeline.left() > 0.0 && timeline.right() < width);
            assert!((timeline.left() - upper[0].left()).abs() < 0.5);
            assert!((timeline.right() - upper[2].right()).abs() < 0.5);
            assert!(upper[0].left() > 0.0);
            assert!(upper[2].right() < width);
            for panel in panels {
                assert!(
                    panel.width() > 40.0 && panel.height() > 40.0,
                    "panel collapsed at {width}x{height}: {panel:?}"
                );
            }
        }
    }
}
