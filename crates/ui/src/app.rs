//! eframe application entry and native window configuration.

use crate::shortcuts::{ShortcutManager, ShortcutStore};
use crate::widgets::timeline::clip_visuals::TimelineVisualCache;
use crate::{app_state::AppState, config, theme, widgets};

/// The CutCraft desktop application.
#[derive(Default)]
pub struct CutCraftApp {
    state: AppState,
    timeline_visuals: TimelineVisualCache,
}

impl CutCraftApp {
    /// Create the application, install the dark editor theme once, and load the
    /// user's shortcut profile. Corrupt configuration degrades to defaults.
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        theme::configure(&creation_context.egui_ctx);
        let mut app = Self::default();
        let store = ShortcutStore::user_default();
        let persistence_unavailable = store.is_none();
        let (shortcuts, warning) = ShortcutManager::load(store);
        app.state.install_shortcuts(shortcuts);
        if let Some(warning) = warning {
            app.state.report_warning(warning);
        } else if persistence_unavailable {
            app.state.report_warning(
                "Shortcut settings can be changed for this session but cannot be saved on this system.",
            );
        }
        app
    }

    /// Draw one frame of the active screen.
    ///
    /// Kept separate from the `eframe::App` impl so headless tests can drive it
    /// without creating a native window.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        let context = ui.ctx().clone();
        let shortcuts_were_open = self.state.shortcuts_open();
        self.state.handle_shortcuts(&context);
        if !shortcuts_were_open && self.state.shortcuts_open() {
            // egui lays out a newly created modal area on the first pass; request
            // one follow-up frame so it becomes visible without another input.
            context.request_repaint();
        }
        widgets::layout::show_shell(ui, &mut self.state, &mut self.timeline_visuals);
    }
}

impl eframe::App for CutCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}

/// Native window configuration for the desktop application.
///
/// The minimum size keeps the library, preview, inspector, and timeline usable
/// at 960×600. The UI is event-driven; nothing requests a periodic repaint.
pub fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(config::APP_NAME)
            .with_inner_size([config::WINDOW_DEFAULT_WIDTH, config::WINDOW_DEFAULT_HEIGHT])
            .with_min_inner_size([config::WINDOW_MIN_WIDTH, config::WINDOW_MIN_HEIGHT])
            .with_resizable(true),
        centered: true,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use egui::{Context, Pos2, Rect, Vec2, ViewportId};

    use super::*;
    use crate::app_state::Screen;

    fn frame_input(width: f32, height: f32, seconds: f64) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, height))),
            time: Some(seconds),
            ..Default::default()
        }
    }

    fn last_repaint_delay(
        context: &Context,
        app: &mut CutCraftApp,
        seconds: f64,
        width: f32,
        height: f32,
    ) -> Duration {
        let output = context.run_ui(frame_input(width, height, seconds), |ui| app.draw(ui));
        let delay = output
            .viewport_output
            .get(&ViewportId::ROOT)
            .map_or(Duration::ZERO, |viewport| viewport.repaint_delay);
        output.drop_without_applying_deltas();
        delay
    }

    #[test]
    fn native_options_define_title_minimums_and_resizability() {
        let options = native_options();

        assert_eq!(options.viewport.title.as_deref(), Some("CutCraft"));
        assert_eq!(
            options.viewport.inner_size,
            Some(Vec2::new(
                config::WINDOW_DEFAULT_WIDTH,
                config::WINDOW_DEFAULT_HEIGHT
            ))
        );
        assert_eq!(
            options.viewport.min_inner_size,
            Some(Vec2::new(
                config::WINDOW_MIN_WIDTH,
                config::WINDOW_MIN_HEIGHT
            ))
        );
        assert_eq!(options.viewport.resizable, Some(true));
        assert!(options.centered);
    }

    #[test]
    fn minimum_window_leaves_room_for_the_workspace_and_timeline() {
        let workspace_min = config::WORKSPACE_LEFT_MIN_WIDTH
            + config::WORKSPACE_CENTER_MIN_WIDTH
            + config::WORKSPACE_RIGHT_MIN_WIDTH
            + config::WORKSPACE_SPLITTER_WIDTH * 2.0;
        assert!(config::WINDOW_MIN_WIDTH >= workspace_min);

        let vertical_chrome = config::EDITOR_TOP_BAR_HEIGHT + config::STATUS_BAR_HEIGHT;
        assert!(config::WINDOW_MIN_HEIGHT - vertical_chrome - config::TIMELINE_MIN_HEIGHT >= 180.0);
    }

    #[test]
    fn application_starts_on_home_and_renders_every_screen_headlessly() {
        let context = Context::default();
        theme::configure(&context);
        let mut app = CutCraftApp::default();

        for (width, height) in [(960.0, 600.0), (1080.0, 720.0), (1600.0, 900.0)] {
            app.state.go_home();
            let output = context.run_ui(frame_input(width, height, 0.0), |ui| app.draw(ui));
            assert!(!output.shapes.is_empty(), "home produced no shapes");
            output.drop_without_applying_deltas();

            app.state.create_temporary_project();
            assert_eq!(app.state.screen(), Screen::Editor);
            let output = context.run_ui(frame_input(width, height, 0.1), |ui| app.draw(ui));
            assert!(!output.shapes.is_empty(), "editor produced no shapes");
            output.drop_without_applying_deltas();

            app.state.open_export();
            for step in 0..3 {
                let output = context
                    .run_ui(frame_input(width, height, 0.2 + step as f64 * 0.1), |ui| {
                        app.draw(ui)
                    });
                assert!(
                    !output.shapes.is_empty(),
                    "export dialog produced no shapes"
                );
                output.drop_without_applying_deltas();
            }
            app.state.close_export();
        }
    }

    #[test]
    fn stable_home_and_editor_do_not_request_continuous_repaints() {
        let context = Context::default();
        theme::configure(&context);
        let mut app = CutCraftApp::default();
        for step in 0..8 {
            let output = context.run_ui(frame_input(1600.0, 900.0, step as f64 / 60.0), |ui| {
                app.draw(ui)
            });
            output.drop_without_applying_deltas();
        }
        assert_eq!(
            last_repaint_delay(&context, &mut app, 8.0 / 60.0, 1600.0, 900.0),
            Duration::MAX,
            "idle homepage must not run a continuous repaint loop"
        );

        app.state.create_temporary_project();
        for step in 0..12 {
            let output = context
                .run_ui(frame_input(1600.0, 900.0, 1.0 + step as f64 / 60.0), |ui| {
                    app.draw(ui)
                });
            output.drop_without_applying_deltas();
        }
        assert_eq!(
            last_repaint_delay(&context, &mut app, 1.2, 1600.0, 900.0),
            Duration::MAX,
            "idle editor must not run a continuous repaint loop"
        );

        app.state.open_export();
        for step in 0..30 {
            let output = context
                .run_ui(frame_input(1600.0, 900.0, 2.0 + step as f64 / 60.0), |ui| {
                    app.draw(ui)
                });
            output.drop_without_applying_deltas();
        }
        assert_eq!(
            last_repaint_delay(&context, &mut app, 3.0, 1600.0, 900.0),
            Duration::MAX,
            "idle export dialog must not run a continuous repaint loop"
        );
    }

    #[test]
    fn shortcuts_dialog_is_rendered_with_registry_rows_and_footer() {
        let context = Context::default();
        theme::configure(&context);
        let mut app = CutCraftApp::default();
        app.state.create_temporary_project();
        app.state.open_shortcuts();

        let mut labels = Vec::new();
        for step in 0..3 {
            let output = context.run_ui(frame_input(1600.0, 900.0, step as f64 / 60.0), |ui| {
                app.draw(ui)
            });
            labels = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                    _ => None,
                })
                .collect();
            output.drop_without_applying_deltas();
        }

        for expected in [
            "Shortcuts",
            "Timeline",
            "Player",
            "Basic",
            "Other",
            "CutCraft Default",
            "Selection Tool",
            "Reset",
            "Save",
            "Cancel",
        ] {
            assert!(
                labels.iter().any(|label| label == expected),
                "missing shortcut-dialog text '{expected}'"
            );
        }
    }

    #[test]
    fn idle_shortcuts_dialog_does_not_request_continuous_repaints() {
        let context = Context::default();
        theme::configure(&context);
        let mut app = CutCraftApp::default();
        app.state.create_temporary_project();
        app.state.open_shortcuts();
        for step in 0..12 {
            let output = context.run_ui(frame_input(1600.0, 900.0, step as f64 / 60.0), |ui| {
                app.draw(ui)
            });
            output.drop_without_applying_deltas();
        }
        assert_eq!(
            last_repaint_delay(&context, &mut app, 0.3, 1600.0, 900.0),
            Duration::MAX,
            "an idle shortcuts dialog must not repaint continuously"
        );
    }
}
