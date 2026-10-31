//! Native project Open/Save As, unsaved-change and save-error dialogs.

use egui::{vec2, Context, Frame, Id, Layout, Margin, RichText, Stroke};

use crate::app_state::AppState;
use crate::native_project_state::NativeDialogMode;
use crate::theme::{Palette, Radii};
use crate::widgets::{common, icons::Icon};

pub fn show(context: &Context, state: &mut AppState) {
    if state.native_error_dialog().is_some() {
        show_error(context, state);
        return;
    }
    if state.native_project_dialog().is_some() {
        show_file_dialog(context, state);
        return;
    }
    if state.unsaved_prompt().is_some() {
        show_unsaved(context, state);
    }
}

fn modal_frame() -> Frame {
    Frame::new()
        .fill(Palette::MODAL_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER))
        .corner_radius(Radii::DIALOG)
        .inner_margin(Margin::ZERO)
}

fn show_file_dialog(context: &Context, state: &mut AppState) {
    let Some(dialog) = state.native_project_dialog() else {
        return;
    };
    let mode = dialog.mode();
    let title = match mode {
        NativeDialogMode::Open => "Open CutCraft Project",
        NativeDialogMode::SaveAs => "Save Project As",
    };
    let action_label = match mode {
        NativeDialogMode::Open => "Open",
        NativeDialogMode::SaveAs => "Save",
    };
    let title_id = match mode {
        NativeDialogMode::Open => "cutcraft_open_native_project",
        NativeDialogMode::SaveAs => "cutcraft_save_native_project",
    };
    let mut confirmed = false;
    let mut cancelled = false;

    let modal = egui::Modal::new(Id::unique(title_id))
        .backdrop_color(egui::Color32::from_black_alpha(180))
        .frame(modal_frame())
        .show(context, |ui| {
            ui.set_width(620.0);
            Frame::new()
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(title)
                                .size(15.0)
                                .strong()
                                .color(Palette::TEXT_PRIMARY),
                        );
                        common::badge(ui, "Native .cutcraft", Palette::ACCENT);
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if common::icon_button(ui, Icon::Close, "Close project dialog").clicked()
                            {
                                cancelled = true;
                            }
                        });
                    });
                });
            ui.separator();
            Frame::new()
                .inner_margin(Margin::symmetric(18, 12))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    let description = match mode {
                        NativeDialogMode::Open => {
                            "Open a native CutCraft .cutcraft project. This is separate from CapCut draft folders."
                        }
                        NativeDialogMode::SaveAs => {
                            "Choose a .cutcraft file. Saving to a different file creates a new project identity."
                        }
                    };
                    ui.label(
                        RichText::new(description)
                            .size(9.5)
                            .color(Palette::TEXT_SECONDARY),
                    );
                    ui.add_space(5.0);
                    if let Some(dialog) = state.native_project_dialog_mut() {
                        let hint = match mode {
                            NativeDialogMode::Open => "C:\\path\\to\\project.cutcraft",
                            NativeDialogMode::SaveAs => "C:\\path\\to\\project.cutcraft",
                        };
                        ui.add(
                            egui::TextEdit::singleline(dialog.path_mut())
                                .hint_text(hint)
                                .desired_width(ui.available_width()),
                        );
                        if let Some(error) = dialog.error() {
                            ui.label(RichText::new(error).size(9.0).color(Palette::ERROR));
                        }
                    }
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(match mode {
                            NativeDialogMode::Open => {
                                "Validation failures leave the current project untouched."
                            }
                            NativeDialogMode::SaveAs => {
                                "Existing different files are never overwritten."
                            }
                        })
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
                                egui::Button::new(RichText::new("Cancel").size(10.5))
                                    .min_size(vec2(76.0, 28.0)),
                            )
                            .clicked()
                        {
                            cancelled = true;
                        }
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(action_label)
                                            .size(10.5)
                                            .color(Palette::ON_ACCENT),
                                    )
                                    .fill(Palette::ACCENT)
                                    .min_size(vec2(88.0, 28.0)),
                                )
                                .clicked()
                            {
                                confirmed = true;
                            }
                        });
                    });
                });
        });

    if confirmed {
        let _ = match mode {
            NativeDialogMode::Open => state.confirm_native_open(),
            NativeDialogMode::SaveAs => state.confirm_native_save_as(),
        };
        context.request_repaint();
    } else if cancelled || modal.should_close() {
        state.close_native_project_dialog();
        context.request_repaint();
    }
}

fn show_unsaved(context: &Context, state: &mut AppState) {
    let Some(prompt) = state.unsaved_prompt() else {
        return;
    };
    let description = prompt.action().description();
    let error = prompt.error().map(str::to_owned);
    let mut save = false;
    let mut discard = false;
    let mut cancel = false;

    let modal = egui::Modal::new(Id::unique("cutcraft_unsaved_changes"))
        .backdrop_color(egui::Color32::from_black_alpha(180))
        .frame(modal_frame())
        .show(context, |ui| {
            ui.set_width(520.0);
            Frame::new()
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Unsaved changes")
                                .size(15.0)
                                .strong()
                                .color(Palette::TEXT_PRIMARY),
                        );
                        common::badge(ui, "Project", Palette::WARNING);
                    });
                });
            ui.separator();
            Frame::new()
                .inner_margin(Margin::symmetric(18, 12))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(
                        RichText::new(format!(
                            "Save the current project before you {description}?"
                        ))
                        .size(10.0)
                        .color(Palette::TEXT_SECONDARY),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "Save must complete successfully before the pending action continues. Cancel keeps the current project and its unsaved changes.",
                        )
                        .size(8.5)
                        .color(Palette::TEXT_MUTED),
                    );
                    if let Some(error) = error {
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
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Discard").size(10.5).color(Palette::ERROR),
                                )
                                .min_size(vec2(84.0, 28.0)),
                            )
                            .clicked()
                        {
                            discard = true;
                        }
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Save").size(10.5).color(Palette::ON_ACCENT),
                                    )
                                    .fill(Palette::ACCENT)
                                    .min_size(vec2(88.0, 28.0)),
                                )
                                .clicked()
                            {
                                save = true;
                            }
                        });
                    });
                });
        });

    if save {
        state.resolve_unsaved_save();
        context.request_repaint();
    } else if discard {
        state.resolve_unsaved_discard();
        context.request_repaint();
    } else if cancel || modal.should_close() {
        state.cancel_unsaved_prompt();
        context.request_repaint();
    }
}

fn show_error(context: &Context, state: &mut AppState) {
    let Some(message) = state.native_error_dialog().map(str::to_owned) else {
        return;
    };
    let mut close = false;
    let modal = egui::Modal::new(Id::unique("cutcraft_native_save_error"))
        .backdrop_color(egui::Color32::from_black_alpha(180))
        .frame(modal_frame())
        .show(context, |ui| {
            ui.set_width(540.0);
            Frame::new()
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Project Save Failed")
                                .size(15.0)
                                .strong()
                                .color(Palette::TEXT_PRIMARY),
                        );
                        common::badge(ui, "Not saved", Palette::ERROR);
                    });
                });
            ui.separator();
            Frame::new()
                .inner_margin(Margin::symmetric(18, 12))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(RichText::new(message).size(10.0).color(Palette::ERROR));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "The active project and the existing file were left unchanged. Resolve the conflict and save again or use Save As.",
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
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Close").size(10.5).color(Palette::ON_ACCENT),
                                )
                                .fill(Palette::ACCENT)
                                .min_size(vec2(88.0, 28.0)),
                            )
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
        });

    if close || modal.should_close() {
        state.close_native_error_dialog();
        context.request_repaint();
    }
}
