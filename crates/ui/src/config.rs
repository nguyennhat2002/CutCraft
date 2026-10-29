//! Layout and placeholder constants for the CutCraft desktop shell.
//!
//! Values here describe UI navigation and geometry only. Sequence resolution,
//! frame rate, duration, and playback position are display placeholders and
//! are not authoritative project state.

/// Product name shown in the window title, headers, and About dialog.
pub const APP_NAME: &str = "CutCraft";
/// Package version, also shown in the About dialog and status bar.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initial logical window size in points.
pub const WINDOW_DEFAULT_WIDTH: f32 = 1440.0;
pub const WINDOW_DEFAULT_HEIGHT: f32 = 900.0;
/// Smallest supported window size in points.
pub const WINDOW_MIN_WIDTH: f32 = 960.0;
pub const WINDOW_MIN_HEIGHT: f32 = 600.0;

/// Homepage navigation sidebar width.
pub const HOME_SIDEBAR_WIDTH: f32 = 224.0;
/// Height of the page header on Home, Projects, and Settings.
pub const HOME_HEADER_HEIGHT: f32 = 56.0;
/// Height of the editor project/menu bar.
pub const EDITOR_TOP_BAR_HEIGHT: f32 = 52.0;
/// Height of the global status bar.
pub const STATUS_BAR_HEIGHT: f32 = 26.0;

/// Left library panel bounds in the editor, including panel margins.
pub const TOOLS_DEFAULT_WIDTH: f32 = 292.0;
pub const TOOLS_MIN_WIDTH: f32 = 224.0;
pub const TOOLS_MAX_WIDTH: f32 = 360.0;

/// Inspector side panel bounds, including panel margins.
pub const INSPECTOR_DEFAULT_WIDTH: f32 = 300.0;
pub const INSPECTOR_MIN_WIDTH: f32 = 224.0;
pub const INSPECTOR_MAX_WIDTH: f32 = 360.0;

/// Timeline bottom panel bounds, including panel margins.
pub const TIMELINE_DEFAULT_HEIGHT: f32 = 252.0;
pub const TIMELINE_MIN_HEIGHT: f32 = 220.0;
pub const TIMELINE_MAX_HEIGHT: f32 = 380.0;

/// Maximum content width for the homepage workspace.
pub const HOME_CONTENT_MAX_WIDTH: f32 = 1040.0;

/// Export dialog target bounds at common window sizes.
pub const EXPORT_DIALOG_MAX_WIDTH: f32 = 940.0;
pub const EXPORT_DIALOG_MIN_WIDTH: f32 = 640.0;
pub const EXPORT_DIALOG_MAX_HEIGHT: f32 = 560.0;

/// Preview placeholder frame-rate and playhead values.
pub const PREVIEW_FPS_NUMERATOR: u32 = 30;
pub const PREVIEW_FPS_DENOMINATOR: u32 = 1;

/// Static placeholder playhead position and timeline view length.
pub const PLAYHEAD_MILLIS: u64 = 0;
pub const TIMELINE_DURATION_SECONDS: u64 = 60;
pub const TIMELINE_PIXELS_PER_SECOND: f32 = 80.0;
pub const TIMELINE_LEADING_PADDING: f32 = 14.0;

/// Timeline row geometry in points.
pub const TIMELINE_RULER_HEIGHT: f32 = 26.0;
pub const TIMELINE_TRACK_HEIGHT: f32 = 56.0;
pub const TRACK_HEADER_WIDTH: f32 = 136.0;

/// Text shown for actions that intentionally have no implementation yet.
pub const UNIMPLEMENTED_HINT: &str =
    "Not available in this milestone. The UI shell does not edit projects yet.";
