//! Editor screen composition: project bar, library, preview, inspector,
//! timeline, and the export modal.

use egui::{CentralPanel, Frame, Margin, Panel, Ui};

use crate::app_state::AppState;
use crate::config;
use crate::theme::Palette;
use crate::widgets::toolbar::TopBarAction;
use crate::widgets::{export_dialog, inspector, media_pool, preview, timeline, toolbar};

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
            Panel::bottom("timeline_panel")
                .resizable(true)
                .default_size(config::TIMELINE_DEFAULT_HEIGHT)
                .min_size(config::TIMELINE_MIN_HEIGHT)
                .max_size(config::TIMELINE_MAX_HEIGHT)
                .frame(panel_frame())
                .show(ui, timeline::show);
        }

        if session.tools_visible {
            Panel::left("tools_panel")
                .resizable(true)
                .default_size(config::TOOLS_DEFAULT_WIDTH)
                .min_size(config::TOOLS_MIN_WIDTH)
                .max_size(config::TOOLS_MAX_WIDTH)
                .frame(panel_frame())
                .show(ui, |ui| media_pool::show(ui, session));
        }

        if session.inspector_visible {
            Panel::right("inspector_panel")
                .resizable(true)
                .default_size(config::INSPECTOR_DEFAULT_WIDTH)
                .min_size(config::INSPECTOR_MIN_WIDTH)
                .max_size(config::INSPECTOR_MAX_WIDTH)
                .frame(panel_frame())
                .show(ui, inspector::show);
        }

        CentralPanel::no_frame()
            .frame(
                Frame::new()
                    .fill(Palette::APP_BG)
                    .inner_margin(Margin::same(12)),
            )
            .show(ui, |ui| preview::show(ui, session));

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

fn panel_frame() -> Frame {
    Frame::new()
        .fill(Palette::PANEL_BG)
        .inner_margin(Margin::symmetric(12, 10))
}
