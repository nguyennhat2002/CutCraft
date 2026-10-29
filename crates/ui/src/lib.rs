//! egui-based desktop shell for CutCraft.
//!
//! This crate owns presentation state and widgets only. It must not contain
//! media decoding, GPU video rendering, or authoritative timeline state.

pub mod app;
pub mod app_state;
pub mod config;
pub mod editor_state;
pub mod export_settings;
pub mod shortcuts;
pub mod theme;
pub mod timecode;
pub mod timeline_state;
pub mod widgets;
pub mod workspace_layout;

pub use app::{native_options, CutCraftApp};
