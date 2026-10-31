//! Recovery decision modal.

use egui::{vec2, Context, Frame, Id, Layout, Margin, RichText, Stroke};

use crate::app_state::{AppState, RecoveryPrompt};
use crate::theme::{Palette, Radii};
use crate::widgets::common;

pub fn show(context: &Context, state: &mut AppState) {
    let Some(snapshot) = state.recovery_prompt().cloned() else {
        return;
    };
    show_prompt(context, state, &snapshot);
}

fn modal_frame() -> Frame {
    Frame::new()
        .fill(Palette::MODAL_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER))
        .corner_radius(Radii::DIALOG)
        .inner_margin(Margin::ZERO)
}

fn show_prompt(context: &Context, state: &mut AppState, prompt: &RecoveryPrompt) {
    let mut restore = false;
    let mut open_last_saved = false;
    let mut keep_both = false;
    let candidate = &prompt.candidate;
    let name = candidate.name.as_deref().unwrap_or("Recovered project");
    let path = candidate
        .original_project_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "unknown project path".to_owned());
    let _ = egui::Modal::new(Id::unique("cutcraft_recovery"))
        .backdrop_color(egui::Color32::from_black_alpha(180))
        .frame(modal_frame())
        .show(context, |ui| {
            ui.set_width(560.0);
            Frame::new()
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Recovered work available")
                                .size(15.0)
                                .strong()
                                .color(Palette::TEXT_PRIMARY),
                        );
                        common::badge(ui, "Recovery snapshot", Palette::WARNING);
                    });
                });
            ui.separator();
            Frame::new()
                .inner_margin(Margin::symmetric(18, 12))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(
                        RichText::new(format!(
                            "\"{name}\" has a validated recovery snapshot newer than the last saved project file."
                        ))
                        .size(9.5)
                        .color(Palette::TEXT_SECONDARY),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!(
                            "Recovery generation {}  \u{00b7}  last saved generation {}  \u{00b7}  {}",
                            candidate.document_generation.unwrap_or(0),
                            candidate.last_saved_generation.unwrap_or(0),
                            candidate
                                .written_at_unix_ms
                                .map(format_unix_ms)
                                .unwrap_or_else(|| "unknown time".to_owned())
                        ))
                        .size(8.5)
                        .color(Palette::TEXT_MUTED),
                    );
                    ui.label(RichText::new(path).size(7.8).color(Palette::TEXT_DISABLED));
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(
                            "Restore creates a new managed project with a new Project ID. The original project and the recovery snapshot are not overwritten; clip, asset, tick, and Magnet identity are preserved.",
                        )
                        .size(8.5)
                        .color(Palette::TEXT_MUTED),
                    );
                    if let Some(error) = &prompt.error {
                        ui.add_space(6.0);
                        ui.label(RichText::new(error).size(9.0).color(Palette::ERROR));
                    }
                    if prompt.busy {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Publishing recovered project...")
                                .size(9.0)
                                .color(Palette::ACCENT_LIGHT),
                        );
                    }
                });
            ui.separator();
            Frame::new()
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !prompt.busy,
                                egui::Button::new(RichText::new("Keep Both").size(10.5))
                                    .min_size(vec2(92.0, 28.0)),
                            )
                            .clicked()
                        {
                            keep_both = true;
                        }
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled(
                                    !prompt.busy,
                                    egui::Button::new(
                                        RichText::new("Restore as New Project")
                                            .size(10.5)
                                            .color(Palette::ON_ACCENT),
                                    )
                                    .fill(if prompt.busy {
                                        Palette::WIDGET_BG
                                    } else {
                                        Palette::ACCENT
                                    })
                                    .min_size(vec2(156.0, 28.0)),
                                )
                                .clicked()
                            {
                                restore = true;
                            }
                            if ui
                                .add_enabled(
                                    !prompt.busy,
                                    egui::Button::new(
                                        RichText::new("Open Last Saved").size(10.5),
                                    )
                                    .min_size(vec2(112.0, 28.0)),
                                )
                                .clicked()
                            {
                                open_last_saved = true;
                            }
                        });
                    });
                });
        });
    if restore {
        if let Err(error) = state.restore_recovery_candidate() {
            state.set_recovery_prompt_error(error);
        }
        context.request_repaint();
    } else if open_last_saved {
        if let Err(error) = state.open_last_saved_version() {
            state.set_recovery_prompt_error(error);
        }
        context.request_repaint();
    } else if keep_both {
        state.dismiss_recovery_prompt();
        context.request_repaint();
    }
}

fn format_unix_ms(value: u64) -> String {
    if value == 0 {
        return "unknown time".to_owned();
    }
    let seconds = (value / 1000) as i64;
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400) as u32;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        seconds_of_day / 3_600,
        (seconds_of_day % 3_600) / 60
    )
}
