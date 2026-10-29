//! Editor composition: compact project bar, resizable three-column workspace,
//! expandable timeline, and the export modal.

use egui::{pos2, vec2, CornerRadius, CursorIcon, Rect, Sense, Stroke, Ui};

use crate::app_state::AppState;
use crate::config;
use crate::editor_state::EditorSession;
use crate::theme::Palette;
use crate::widgets::toolbar::TopBarAction;
use crate::widgets::{export_dialog, inspector, media_pool, panel, preview, timeline, toolbar};
use crate::workspace_layout;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let context = ui.ctx().clone();

    if !state.has_session() {
        state.go_home();
        return;
    }

    let action = {
        let Some((session, shortcuts)) = state.session_and_shortcuts_mut() else {
            return;
        };

        let action = toolbar::show(ui, session, shortcuts);

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
            let heights = workspace_layout::timeline_heights(available.y);
            let timeline_height = if session.timeline_visible {
                heights
                    .default
                    .min((content.height() - gap - 80.0).max(0.0))
            } else {
                0.0
            };
            let workspace_height = (content.height()
                - timeline_height
                - if session.timeline_visible { gap } else { 0.0 })
            .max(1.0);
            let workspace_rect =
                Rect::from_min_size(content.min, vec2(content.width(), workspace_height));

            show_workspace(ui, workspace_rect, session);

            if session.timeline_visible && timeline_height > 1.0 {
                let timeline_rect = Rect::from_min_size(
                    pos2(content.left(), workspace_rect.bottom() + gap),
                    vec2(content.width(), timeline_height),
                );
                panel::show(ui, timeline_rect, Palette::PANEL_BG, |ui| {
                    timeline::show(ui, session, shortcuts);
                });
            }
        }

        if session.export_open {
            export_dialog::show(&context, session);
        }

        if action == TopBarAction::Export {
            session.export_open = true;
        }

        action
    };

    match action {
        TopBarAction::Home => state.go_home(),
        TopBarAction::Shortcuts => {
            state.open_shortcuts();
            context.request_repaint();
        }
        TopBarAction::None | TopBarAction::Export => {}
    }
}

fn paint_editor_background(ui: &Ui) {
    let rect = ui.available_rect_before_wrap();
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, Palette::APP_BG);
}

fn show_workspace(ui: &mut Ui, rect: Rect, session: &mut EditorSession) {
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
            |ui| media_pool::show(ui, session),
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
        |ui| preview::show(ui, session),
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
    use egui::{Pos2, Rect};

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
            let output = context.run_ui(input, |ui| show(ui, &mut state));
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
