//! Top-level application router and global chrome.

use egui::Ui;

use crate::app_state::{AppState, Screen};
use crate::widgets::shortcuts_dialog::{self, ShortcutDialogOutcome};
use crate::widgets::timeline::clip_visuals::TimelineVisualCache;
use crate::widgets::{
    about, auto_save_dialogs, capcut_draft, capcut_draft_dialog, editor, home, media_import_dialog,
    native_project_dialogs, project_library_dialogs, recovery_dialogs, status_bar,
};

/// Draw the active CutCraft screen and global status/About/dialog chrome.
pub fn show_shell(ui: &mut Ui, state: &mut AppState, timeline_visuals: &mut TimelineVisualCache) {
    let context = ui.ctx().clone();

    status_bar::show(ui, state);

    match state.screen() {
        Screen::Editor => editor::show(ui, state, timeline_visuals),
        Screen::CapCutDraft => capcut_draft::show(ui, state),
        Screen::Home | Screen::Projects | Screen::Settings => home::show(ui, state),
    }

    about::show(&context, state);

    capcut_draft_dialog::show(&context, state);
    native_project_dialogs::show(&context, state);
    media_import_dialog::show(&context, state);
    project_library_dialogs::show(&context, state);
    auto_save_dialogs::show(&context, state);
    recovery_dialogs::show(&context, state);

    if state.shortcuts_open() {
        let outcome = match state.shortcuts_dialog_parts_mut() {
            Some((manager, dialog)) => shortcuts_dialog::show(&context, manager, dialog),
            None => ShortcutDialogOutcome::Closed,
        };
        match outcome {
            ShortcutDialogOutcome::Saved => state.finish_shortcuts_saved(),
            ShortcutDialogOutcome::Closed => state.close_shortcuts(),
            ShortcutDialogOutcome::Open => {}
        }
    }
}
