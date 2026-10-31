//! Nonblocking AutoSave wait and save-problem dialogs.

use egui::{vec2, Context, Frame, Id, Layout, Margin, RichText, Stroke};

use crate::app_state::{AppState, PendingDestination};
use crate::theme::{Palette, Radii};
use crate::widgets::common;

pub fn show(context: &Context, state: &mut AppState) {
    if state.navigation_problem().is_some() {
        show_problem(context, state);
    } else if state.navigation_wait_visible() {
        show_wait(context, state);
    }
}

fn modal_frame() -> Frame {
    Frame::new()
        .fill(Palette::MODAL_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER))
        .corner_radius(Radii::DIALOG)
        .inner_margin(Margin::ZERO)
}

fn show_wait(context: &Context, state: &mut AppState) {
    let destination = state
        .pending_destination()
        .map(describe_destination)
        .unwrap_or("continue");
    let mut stay = false;
    let _ = egui::Modal::new(Id::unique("cutcraft_autosave_wait"))
        .backdrop_color(egui::Color32::from_black_alpha(150))
        .frame(modal_frame())
        .show(context, |ui| {
            ui.set_width(440.0);
            Frame::new()
                .inner_margin(Margin::symmetric(18, 16))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(
                        RichText::new("Saving project...")
                            .size(15.0)
                            .strong()
                            .color(Palette::TEXT_PRIMARY),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!(
                            "CutCraft is waiting for the current project generation to be written before you {destination}."
                        ))
                        .size(9.5)
                        .color(Palette::TEXT_SECONDARY),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(
                            "The UI remains responsive. No success is reported until the file has been validated and published.",
                        )
                        .size(8.5)
                        .color(Palette::TEXT_MUTED),
                    );
                    ui.add_space(10.0);
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Stay in Editor").size(10.5))
                                .min_size(vec2(116.0, 28.0)),
                        )
                        .clicked()
                    {
                        stay = true;
                    }
                });
        });
    if stay {
        state.cancel_pending_navigation();
        context.request_repaint();
    }
}

fn show_problem(context: &Context, state: &mut AppState) {
    let Some(problem) = state.navigation_problem().map(str::to_owned) else {
        return;
    };
    let destination = state
        .pending_destination()
        .map(describe_destination)
        .unwrap_or("continue");
    let mut retry = false;
    let mut stay = false;
    let mut preserve_copy = false;
    let can_preserve_copy = state.can_preserve_conflict_copy();
    let _ = egui::Modal::new(Id::unique("cutcraft_autosave_problem"))
        .backdrop_color(egui::Color32::from_black_alpha(180))
        .frame(modal_frame())
        .show(context, |ui| {
            ui.set_width(520.0);
            Frame::new()
                .inner_margin(Margin::symmetric(18, 12))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Project was not saved")
                                .size(15.0)
                                .strong()
                                .color(Palette::TEXT_PRIMARY),
                        );
                        common::badge(ui, "AutoSave", Palette::ERROR);
                    });
                });
            ui.separator();
            Frame::new()
                .inner_margin(Margin::symmetric(18, 12))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(
                        RichText::new(format!(
                            "CutCraft cannot {destination} because the latest project generation was not persisted."
                        ))
                        .size(9.5)
                        .color(Palette::TEXT_SECONDARY),
                    );
                    ui.add_space(6.0);
                    ui.label(RichText::new(problem).size(9.0).color(Palette::ERROR));
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(
                            "The in-memory project and the last valid file are preserved. Resolve the filesystem or external-file issue and retry, or stay in the Editor. Nothing is discarded automatically.",
                        )
                        .size(8.5)
                        .color(Palette::TEXT_MUTED),
                    );
                });
            ui.separator();
            Frame::new()
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Stay in Editor").size(10.5))
                                    .min_size(vec2(112.0, 28.0)),
                            )
                            .clicked()
                        {
                            stay = true;
                        }
                        if can_preserve_copy
                            && ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Save Recovery Copy").size(10.5),
                                    )
                                    .min_size(vec2(138.0, 28.0)),
                                )
                                .on_hover_text(
                                    "Create a new managed project from the current edits. The externally modified project is not overwritten.",
                                )
                                .clicked()
                        {
                            preserve_copy = true;
                        }
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Retry Save")
                                            .size(10.5)
                                            .color(Palette::ON_ACCENT),
                                    )
                                    .fill(Palette::ACCENT)
                                    .min_size(vec2(100.0, 28.0)),
                                )
                                .clicked()
                            {
                                retry = true;
                            }
                        });
                    });
                });
        });
    if retry {
        state.retry_pending_navigation();
        context.request_repaint();
    } else if preserve_copy {
        if let Err(error) = state.preserve_conflict_copy() {
            state.report_error(error);
        }
        context.request_repaint();
    } else if stay {
        state.cancel_pending_navigation();
        context.request_repaint();
    }
}

fn describe_destination(destination: &PendingDestination) -> &'static str {
    match destination {
        PendingDestination::NewProject => "create a new project",
        PendingDestination::OpenProjectDialog => "open another project",
        PendingDestination::OpenLibrary(_) => "open the selected project",
        PendingDestination::ReturnHome => "return home",
        PendingDestination::Projects => "open Projects",
        PendingDestination::Settings => "open Settings",
        PendingDestination::Exit => "exit CutCraft",
    }
}
