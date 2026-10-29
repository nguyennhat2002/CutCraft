//! Editor composition: compact project bar, resizable three-column workspace,
//! expandable timeline, and the export modal.

use egui::{
    pos2, vec2, Align, Color32, CursorIcon, Frame, Layout, Margin, Panel, Sense, Stroke, Ui,
};

use crate::app_state::AppState;
use crate::config;
use crate::editor_state::EditorSession;
use crate::theme::Palette;
use crate::widgets::toolbar::TopBarAction;
use crate::widgets::{export_dialog, inspector, media_pool, preview, timeline, toolbar};
use crate::workspace_layout;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let context = ui.ctx().clone();

    if !state.has_session() {
        state.go_home();
        return;
    }

    let action = {
        let Some(session) = state.session_mut() else {
            return;
        };

        let action = toolbar::show(ui, session);

        if session.timeline_visible {
            let heights = workspace_layout::timeline_heights(ui.available_height());
            Panel::bottom("timeline_panel")
                .resizable(false)
                .exact_size(heights.default)
                .frame(
                    Frame::new()
                        .fill(Palette::PANEL_BG)
                        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
                        .inner_margin(Margin::symmetric(10, 6)),
                )
                .show(ui, |ui| timeline::show(ui, session));
        }

        show_workspace(ui, session);

        if session.export_open {
            export_dialog::show(&context, session);
        }

        if action == TopBarAction::Export {
            session.export_open = true;
        }

        action
    };

    if action == TopBarAction::Home {
        state.go_home();
    }
}

fn show_workspace(ui: &mut Ui, session: &mut EditorSession) {
    let show_left = session.tools_visible;
    let show_right = session.inspector_visible;
    let available = ui.available_size();
    let height = available.y.max(1.0);
    let splitter_count = usize::from(show_left) + usize::from(show_right);
    let content_width =
        (available.x - config::WORKSPACE_SPLITTER_WIDTH * splitter_count as f32).max(0.0);
    let widths = workspace_layout::workspace_widths(
        content_width,
        show_left,
        show_right,
        session.left_panel_fraction,
        session.right_panel_fraction,
    );

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;

        if show_left {
            workspace_column(
                ui,
                widths.left,
                height,
                Palette::PANEL_BG,
                Margin::ZERO,
                |ui| media_pool::show(ui, session),
            );
            splitter(
                ui,
                height,
                &mut session.left_panel_fraction,
                true,
                content_width,
            );
        }

        workspace_column(
            ui,
            widths.center,
            height,
            Palette::PANEL_BG,
            Margin::ZERO,
            |ui| preview::show(ui, session),
        );

        if show_right {
            splitter(
                ui,
                height,
                &mut session.right_panel_fraction,
                false,
                content_width,
            );
            workspace_column(
                ui,
                widths.right,
                height,
                Palette::PANEL_BG,
                Margin::ZERO,
                |ui| inspector::show(ui, session),
            );
        }
    });
}

fn workspace_column<R>(
    ui: &mut Ui,
    width: f32,
    height: f32,
    fill: Color32,
    margin: Margin,
    add_contents: impl FnOnce(&mut Ui) -> R,
) {
    ui.allocate_ui_with_layout(vec2(width, height), Layout::top_down(Align::Min), |ui| {
        Frame::new().fill(fill).inner_margin(margin).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            add_contents(ui);
        });
    });
}

fn splitter(ui: &mut Ui, height: f32, fraction: &mut f32, is_left: bool, content_width: f32) {
    let (rect, response) = ui.allocate_exact_size(
        vec2(config::WORKSPACE_SPLITTER_WIDTH, height),
        Sense::click_and_drag(),
    );
    let response = response.on_hover_cursor(CursorIcon::ResizeHorizontal);
    let active = response.hovered() || response.dragged();

    if active {
        ui.painter().rect_filled(rect, 0.0, Palette::ACCENT_BG);
    }
    ui.painter().line_segment(
        [
            pos2(rect.center().x, rect.top() + 2.0),
            pos2(rect.center().x, rect.bottom() - 2.0),
        ],
        Stroke::new(
            1.0,
            if active {
                Palette::ACCENT
            } else {
                Palette::BORDER
            },
        ),
    );

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
