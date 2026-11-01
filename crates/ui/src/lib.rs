//! egui-based desktop shell for CutCraft.
//!
//! This crate owns presentation state and widgets only. It must not contain
//! media decoding, GPU video rendering, or authoritative timeline state.

pub mod app;
pub mod app_state;
pub mod auto_save_state;
pub mod config;
pub mod editor_state;
pub mod export_settings;
pub mod media_library_state;
pub mod media_preview_state;
pub mod native_document;
pub mod native_project_state;
pub mod native_timeline;
pub mod recovery_state;
pub mod shortcuts;
pub mod theme;
pub mod timecode;
pub mod timeline_mapping;
pub mod timeline_state;
pub mod widgets;
pub mod workspace_layout;
pub mod workspace_preferences;

pub use app::{native_options, CutCraftApp};
