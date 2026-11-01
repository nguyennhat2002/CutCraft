//! eframe application entry and native window configuration.

use std::time::Instant;

use crate::shortcuts::{ShortcutManager, ShortcutStore};
use crate::widgets::timeline::clip_visuals::TimelineVisualCache;
use crate::workspace_preferences::{WorkspacePreferences, WorkspaceStore};
use crate::{app_state::AppState, config, theme, widgets};
use capcut_renderer::PreviewRenderer;

/// The CutCraft desktop application.
#[derive(Default)]
pub struct CutCraftApp {
    state: AppState,
    timeline_visuals: TimelineVisualCache,
    render_log: bool,
}

impl CutCraftApp {
    /// Create the application, install the dark editor theme once, and load the
    /// user's shortcut profile. Corrupt configuration degrades to defaults.
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        theme::configure(&creation_context.egui_ctx);
        let mut app = Self {
            render_log: std::env::var_os("CUTCRAFT_RENDER_LOG").is_some(),
            ..Self::default()
        };
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

        let workspace_store = WorkspaceStore::user_default();
        let (workspace_preferences, workspace_warning) = match &workspace_store {
            Some(store) => match store.load() {
                Ok(Some(preferences)) => (preferences, None),
                Ok(None) => (WorkspacePreferences::default(), None),
                Err(error) => (
                    WorkspacePreferences::default(),
                    Some(format!("Workspace layout could not be loaded: {error}")),
                ),
            },
            None => (WorkspacePreferences::default(), None),
        };
        app.state
            .install_workspace_preferences(workspace_preferences, workspace_store);
        if let Some(warning) = workspace_warning {
            app.state.report_warning(warning);
        }
        match capcut_project::ProjectLibrary::user_default() {
            Ok(library) => app.state.install_project_library(library),
            Err(error) => app.state.install_project_library_error(error.to_string()),
        }
        let requested_renderer = std::env::var("CUTCRAFT_PREVIEW_RENDERER").unwrap_or_default();
        if requested_renderer.eq_ignore_ascii_case("egui") {
            eprintln!("[cutcraft-renderer] forced egui fallback by CUTCRAFT_PREVIEW_RENDERER");
        } else if requested_renderer.eq_ignore_ascii_case("init-failure") {
            // QA seam: exercises the exact branch the app takes when GPU Preview
            // initialization fails, without needing a broken driver.
            eprintln!("[cutcraft-renderer] simulated initialization failure requested");
            app.state.report_warning(
                "GPU preview unavailable; using egui fallback: simulated initialization failure",
            );
        } else if let Some(render_state) = creation_context.wgpu_render_state.as_ref() {
            let adapter = render_state.adapter.get_info();
            eprintln!(
                "[cutcraft-renderer] reusing eframe wgpu backend={:?} adapter={:?} driver={:?} target={:?}",
                adapter.backend, adapter.name, adapter.driver, render_state.target_format
            );
            match PreviewRenderer::new(
                &render_state.device,
                &render_state.queue,
                render_state.target_format,
            ) {
                Ok(renderer) => app.state.install_preview_renderer(renderer),
                Err(error) => {
                    eprintln!("[cutcraft-renderer] initialization failed: {error}");
                    app.state.report_warning(format!(
                        "GPU preview unavailable; using egui fallback: {error}"
                    ));
                }
            }
        } else {
            eprintln!("[cutcraft-renderer] no wgpu render state; using egui fallback");
            if requested_renderer.eq_ignore_ascii_case("gpu") {
                app.state.report_warning(
                    "GPU preview was requested but eframe did not expose a wgpu render state.",
                );
            }
        }
        app
    }

    /// Draw one frame of the active screen.
    ///
    /// Kept separate from the `eframe::App` impl so headless tests can drive it
    /// without creating a native window.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        // Diagnostic-only tick timing; disabled unless CUTCRAFT_RENDER_LOG is set
        // so normal runs pay nothing for it.
        let render_log = self.render_log;
        let tick_started = render_log.then(Instant::now);
        let mut lap = tick_started;
        let mut state_ms = 0.0_f64;
        let mut shell_ms = 0.0_f64;
        let uploads_before = self
            .state
            .preview_renderer_metrics()
            .map(|metrics| metrics.upload_count);
        let context = ui.ctx().clone();
        if context.input(|input| input.viewport().close_requested())
            && !self.state.request_app_close()
        {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        let shortcuts_were_open = self.state.shortcuts_open();
        self.state.handle_shortcuts(&context);
        if !shortcuts_were_open && self.state.shortcuts_open() {
            // egui lays out a newly created modal area on the first pass; request
            // one follow-up frame so it becomes visible without another input.
            context.request_repaint();
        }
        self.state.update_auto_save(&context);
        self.state.update_media_library(&context);
        self.state.update_media_preview(&context);
        if let Some(previous) = lap.take() {
            state_ms = previous.elapsed().as_secs_f64() * 1_000.0;
        }
        if render_log {
            lap = Some(Instant::now());
        }
        widgets::layout::show_shell(ui, &mut self.state, &mut self.timeline_visuals);
        if let Some(previous) = lap.take() {
            shell_ms = previous.elapsed().as_secs_f64() * 1_000.0;
        }
        self.state.persist_workspace_layout_if_requested();
        if self.state.take_exit_request() {
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if let Some(tick_started) = tick_started {
            let draw_ms = tick_started.elapsed().as_secs_f64() * 1_000.0;
            match self.state.preview_renderer_metrics() {
                Some(metrics) => {
                    let milliseconds = |value: Option<f64>| match value {
                        Some(value) => format!("{value:.3}"),
                        None => "n/a".to_owned(),
                    };
                    eprintln!(
                        "[cutcraft-ui] draw_ms={draw_ms:.3} state_ms={state_ms:.3} shell_ms={shell_ms:.3} gpu=true tick_uploads={} upload_cpu_ms={} allocation_cpu_ms={} submit_cpu_ms={} gpu_completion_ms={} present_ms={} gpu_texture_bytes={} staging_bytes={} peak_gpu_bytes={} peak_staging_bytes={} reallocs={} reuses={} skipped={} rejections={}",
                        uploads_before
                            .map(|before| metrics.upload_count.saturating_sub(before))
                            .unwrap_or(0),
                        milliseconds(metrics.frame_upload_cpu_ms),
                        milliseconds(metrics.texture_allocation_cpu_ms),
                        milliseconds(metrics.renderer_submit_ms),
                        milliseconds(metrics.gpu_upload_completion_ms),
                        milliseconds(metrics.preview_present_ms),
                        metrics.gpu_texture_bytes,
                        metrics.cpu_staging_bytes,
                        metrics.peak_gpu_texture_bytes,
                        metrics.peak_cpu_staging_bytes,
                        metrics.reallocation_count,
                        metrics.texture_reuse_count,
                        metrics.uploads_skipped,
                        metrics.upload_rejections,
                    );
                }
                None => {
                    eprintln!(
                        "[cutcraft-ui] draw_ms={draw_ms:.3} state_ms={state_ms:.3} shell_ms={shell_ms:.3} gpu=false tick_uploads=0"
                    );
                }
            }
        }
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
    fn draw_persists_a_requested_timeline_height() {
        let dir = std::env::temp_dir().join(format!("cutcraft-app-persist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp");
        let store = crate::workspace_preferences::WorkspaceStore::new(dir.join("workspace.json"));
        let context = Context::default();
        theme::configure(&context);
        let mut app = CutCraftApp::default();
        app.state.install_workspace_preferences(
            crate::workspace_preferences::WorkspacePreferences::default(),
            Some(store.clone()),
        );
        app.state.create_temporary_project();
        app.state
            .session_mut()
            .expect("session")
            .set_timeline_height_fraction(Some(0.66));
        app.state
            .session_mut()
            .expect("session")
            .request_layout_save();

        let output = context.run_ui(frame_input(1600.0, 900.0, 0.0), |ui| app.draw(ui));
        output.drop_without_applying_deltas();

        assert_eq!(
            store.load().expect("load").expect("preferences"),
            crate::workspace_preferences::WorkspacePreferences {
                timeline_height_fraction: Some(0.66)
            }
        );
        let _ = std::fs::remove_dir_all(dir);
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
