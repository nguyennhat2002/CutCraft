//! Layout and placeholder constants for the CutCraft desktop shell.
//!
//! Values here describe UI geometry only. Sequence resolution, frame rate,
//! duration, and playback position are display placeholders and are not
//! authoritative project state.

/// Product name shown in the window title, headers, and About dialog.
pub const APP_NAME: &str = "CutCraft";
/// Package version, also shown in the About dialog and status bar.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initial logical window size in points, matching the primary reference viewport.
pub const WINDOW_DEFAULT_WIDTH: f32 = 1600.0;
pub const WINDOW_DEFAULT_HEIGHT: f32 = 900.0;
/// Smallest supported window size in points.
pub const WINDOW_MIN_WIDTH: f32 = 960.0;
pub const WINDOW_MIN_HEIGHT: f32 = 600.0;

/// Homepage navigation sidebar width.
pub const HOME_SIDEBAR_WIDTH: f32 = 216.0;
/// Height of the compact header on Home, Projects, and Settings.
pub const HOME_HEADER_HEIGHT: f32 = 40.0;
/// Height of the editor project/menu bar.
pub const EDITOR_TOP_BAR_HEIGHT: f32 = 40.0;
/// Height of the tool/preview/inspector header row inside the editor.
pub const EDITOR_COLUMN_HEADER_HEIGHT: f32 = 38.0;
/// Height of the global status bar.
pub const STATUS_BAR_HEIGHT: f32 = 24.0;

/// Gap and drag handle between the rounded editor panels.
pub const WORKSPACE_SPLITTER_WIDTH: f32 = 6.0;
/// Window-background margin around the editor panel group.
pub const PANEL_OUTER_MARGIN: f32 = 6.0;
/// Initial left library panel fraction of the workspace width.
pub const WORKSPACE_LEFT_FRACTION: f32 = 0.335;
/// Initial right inspector fraction of the workspace width.
pub const WORKSPACE_RIGHT_FRACTION: f32 = 0.376;
pub const WORKSPACE_LEFT_MIN_WIDTH: f32 = 240.0;
pub const WORKSPACE_RIGHT_MIN_WIDTH: f32 = 260.0;
pub const WORKSPACE_CENTER_MIN_WIDTH: f32 = 280.0;
pub const WORKSPACE_LEFT_MAX_FRACTION: f32 = 0.45;
pub const WORKSPACE_RIGHT_MAX_FRACTION: f32 = 0.50;

/// Timeline bottom panel bounds. The actual default height is proportional to
/// the available window height.
pub const TIMELINE_DEFAULT_HEIGHT: f32 = 400.0;
pub const TIMELINE_MIN_HEIGHT: f32 = 280.0;
pub const TIMELINE_MAX_HEIGHT: f32 = 460.0;

/// Maximum content width for the homepage workspace.
pub const HOME_CONTENT_MAX_WIDTH: f32 = 1320.0;

/// Export dialog target bounds.
pub const EXPORT_DIALOG_MAX_WIDTH: f32 = 760.0;
pub const EXPORT_DIALOG_MIN_WIDTH: f32 = 640.0;
pub const EXPORT_DIALOG_MAX_HEIGHT: f32 = 620.0;

/// Preview placeholder frame-rate and playhead values.
pub const PREVIEW_FPS_NUMERATOR: u32 = 30;
pub const PREVIEW_FPS_DENOMINATOR: u32 = 1;

/// Static placeholder playhead position and timeline view length.
pub const PLAYHEAD_MILLIS: u64 = 0;
pub const TIMELINE_DURATION_SECONDS: u64 = 180;
pub const TIMELINE_PIXELS_PER_SECOND: f32 = 30.0;
pub const TIMELINE_MIN_PIXELS_PER_SECOND: f32 = 4.0;
pub const TIMELINE_MAX_PIXELS_PER_SECOND: f32 = 80.0;
pub const TIMELINE_LEADING_PADDING: f32 = 14.0;

/// Timeline chrome/empty-canvas geometry in points, measured from the M1.5 reference.
pub const TIMELINE_TOOLBAR_HEIGHT: f32 = 32.0;
pub const TIMELINE_EMPTY_LEADING_PADDING: f32 = 78.0;
pub const TIMELINE_INVITATION_HEIGHT: f32 = 68.0;
/// Timeline row geometry in points.
pub const TIMELINE_RULER_HEIGHT: f32 = 26.0;
pub const TIMELINE_TRACK_HEIGHT: f32 = 40.0;
pub const TRACK_HEADER_WIDTH: f32 = 158.0;

/// Text shown for actions that intentionally have no implementation yet.
pub const UNIMPLEMENTED_HINT: &str =
    "Not available in this milestone. The UI shell does not edit projects yet.";
