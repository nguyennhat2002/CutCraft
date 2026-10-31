//! Managed project rename dialog.

use egui::{vec2, Context, Frame, Id, Layout, Margin, RichText, Stroke};

use crate::app_state::AppState;
use crate::theme::{Palette, Radii};
use crate::widgets::common;

pub fn show(context: &Context, state: &mut AppState) {
    if state.library_rename_dialog().is_some() {
        show_rename(context, state);
    }
}

fn modal_frame() -> Frame {
    Frame::new()
        .fill(Palette::MODAL_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER))
        .corner_radius(Radii::DIALOG)
        .inner_margin(Margin::ZERO)
}

fn show_rename(context: &Context, state: &mut AppState) {
    let Some(dialog) = state.library_rename_dialog() else {
        return;
    };
    let mut name = dialog.name.clone();
    let error = dialog.error.clone();
    let mut rename = false;
    let mut cancel = false;
    let modal = egui::Modal::new(Id::unique("cutcraft_project_rename"))
        .backdrop_color(egui::Color32::from_black_alpha(180))
        .frame(modal_frame())
        .show(context, |ui| {
            ui.set_width(480.0);
            Frame::new()
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Rename Project")
                                .size(15.0)
                                .strong()
                                .color(Palette::TEXT_PRIMARY),
                        );
                        common::badge(ui, "Identity unchanged", Palette::ACCENT);
                    });
                });
            ui.separator();
            Frame::new()
                .inner_margin(Margin::symmetric(18, 12))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(
                        RichText::new(
                            "The project name is display metadata. Its Project ID, managed path and media references do not change.",
                        )
                        .size(9.5)
                        .color(Palette::TEXT_SECONDARY),
                    );
                    ui.add_space(6.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut name)
                            .hint_text("Project name")
                            .desired_width(ui.available_width()),
                    );
                    if let Some(error) = &error {
                        ui.add_space(4.0);
                        ui.label(RichText::new(error).size(9.0).color(Palette::ERROR));
                    }
                });
            ui.separator();
            Frame::new()
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Cancel").size(10.5))
                                    .min_size(vec2(76.0, 28.0)),
                            )
                            .clicked()
                        {
                            cancel = true;
                        }
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Rename")
                                            .size(10.5)
                                            .color(Palette::ON_ACCENT),
                                    )
                                    .fill(Palette::ACCENT)
                                    .min_size(vec2(88.0, 28.0)),
                                )
                                .clicked()
                            {
                                rename = true;
                            }
                        });
                    });
                });
        });
    if let Some(dialog) = state.library_rename_dialog_mut() {
        dialog.name = name;
    }
    if rename {
        let _ = state.confirm_library_rename();
        context.request_repaint();
    } else if cancel || modal.should_close() {
        state.close_library_rename();
        context.request_repaint();
    }
}
