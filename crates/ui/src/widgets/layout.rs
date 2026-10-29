//! Top-level application router and global chrome.

use egui::Ui;

use crate::app_state::{AppState, Screen};
use crate::widgets::shortcuts_dialog::{self, ShortcutDialogOutcome};
use crate::widgets::{about, editor, home, status_bar};

/// Draw the active CutCraft screen and global status/About/dialog chrome.
pub fn show_shell(ui: &mut Ui, state: &mut AppState) {
    let context = ui.ctx().clone();

    status_bar::show(ui, state);

    match state.screen() {
        Screen::Editor => editor::show(ui, state),
        Screen::Home | Screen::Projects | Screen::Settings => home::show(ui, state),
    }

    about::show(&context, state);

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
