//! Bottom status bar with a visible notice area.

use egui::{Align, Frame, Layout, Margin, RichText, Ui};

use crate::app_state::{AppState, NoticeSeverity, Screen};
use crate::config;
use crate::theme::Palette;
use crate::widgets::common;

pub fn show(ui: &mut Ui, state: &AppState) {
    let notice = state.notice();
    let (dot_color, text_color) = match notice.severity() {
        NoticeSeverity::Info => (Palette::INFO, Palette::TEXT_MUTED),
        NoticeSeverity::Warning => (Palette::WARNING, Palette::WARNING),
        NoticeSeverity::Error => (Palette::ERROR, Palette::ERROR),
    };

    let context_label = match state.screen() {
        Screen::Editor => "Temporary session \u{00b7} Not saved",
        Screen::Home | Screen::Projects | Screen::Settings => "No saved projects",
    };

    egui::Panel::bottom("status_bar")
        .exact_size(config::STATUS_BAR_HEIGHT)
        .frame(
            Frame::new()
                .fill(Palette::STATUS_BG)
                .inner_margin(Margin::symmetric(12, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                common::status_dot(ui, dot_color);
                ui.label(RichText::new(notice.message()).size(10.5).color(text_color));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("v{} \u{00b7} UI SHELL", config::APP_VERSION))
                            .size(9.5)
                            .color(Palette::TEXT_DISABLED),
                    );
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(context_label)
                            .size(9.5)
                            .color(Palette::TEXT_DISABLED),
                    );
                });
            });
        });
}
