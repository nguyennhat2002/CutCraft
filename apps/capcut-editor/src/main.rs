//! CutCraft native desktop entry point.
//!
//! The application is currently a UI shell: media import, decoding, preview
//! rendering, playback, timeline editing, persistence, and export are not
//! implemented. See `docs/ADR/0001-desktop-ui-shell.md`.

fn main() -> eframe::Result {
    eframe::run_native(
        capcut_ui::config::APP_NAME,
        capcut_ui::native_options(),
        Box::new(|creation_context| Ok(Box::new(capcut_ui::CutCraftApp::new(creation_context)))),
    )
}
