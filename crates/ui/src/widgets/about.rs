//! About dialog for the shell.

use egui::{Align2, Context, Grid, RichText, Ui, Vec2};

use crate::app_state::AppState;
use crate::config;
use crate::theme::Palette;

pub fn show(context: &Context, state: &mut AppState) {
    if !state.about_open() {
        return;
    }

    let mut open = true;
    let mut close_clicked = false;

    let _ = egui::Window::new("About CutCraft")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .default_width(400.0)
        .show(context, |ui| {
            ui.label(
                RichText::new(config::APP_NAME)
                    .size(24.0)
                    .strong()
                    .color(Palette::TEXT_PRIMARY),
            );
            ui.label(
                RichText::new("Desktop video editor \u{2014} application shell")
                    .size(11.5)
                    .color(Palette::TEXT_MUTED),
            );

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            Grid::new("about_details")
                .num_columns(2)
                .spacing([18.0, 6.0])
                .show(ui, |ui| {
                    detail_row(ui, "Version", &format!("v{}", config::APP_VERSION));
                    detail_row(ui, "Milestone", "M1 \u{2014} Home, Editor, Export UI");
                    detail_row(ui, "UI framework", "egui / eframe 0.37 (wgpu)");
                });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            ui.add(
                egui::Label::new(
                    RichText::new(
                        "This build is a user-interface shell. Media import, decoding, \
                         preview rendering, playback, timeline editing, project persistence, \
                         and export are not implemented. Editing controls are disabled \
                         placeholders, and temporary sessions are never saved to disk.",
                    )
                    .size(11.0)
                    .color(Palette::TEXT_SECONDARY),
                )
                .wrap(),
            );

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    close_clicked = true;
                }
            });
        });

    if close_clicked || !open {
        state.close_about();
    }
}

fn detail_row(ui: &mut Ui, label: &str, value: &str) {
    ui.label(RichText::new(label).size(10.5).color(Palette::TEXT_MUTED));
    ui.label(
        RichText::new(value)
            .size(11.0)
            .color(Palette::TEXT_SECONDARY),
    );
    ui.end_row();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_dialog_renders_its_scope_disclaimer() {
        let context = Context::default();
        let mut state = AppState::default();
        state.open_about();
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(900.0, 700.0),
            )),
            ..Default::default()
        };

        let mut disclaimer_visible = false;
        for _ in 0..3 {
            let output = context.run_ui(input(), |ui| show(ui.ctx(), &mut state));
            for clipped in &output.shapes {
                if let egui::Shape::Text(text) = &clipped.shape {
                    disclaimer_visible |= text.galley.text().contains("Media import, decoding");
                }
            }
            output.drop_without_applying_deltas();
        }
        assert!(disclaimer_visible);
        assert!(state.about_open());
    }
}
