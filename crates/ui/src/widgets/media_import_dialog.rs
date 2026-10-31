//! Media import and relink path dialog.

use egui::{vec2, Context, Frame, Id, Layout, Margin, RichText, Stroke};

use crate::app_state::AppState;
use crate::media_library_state::MediaDialogMode;
use crate::theme::{Palette, Radii};
use crate::widgets::common;

pub fn show(context: &Context, state: &mut AppState) {
    if state.media_import_dialog().is_none() {
        return;
    }
    let snapshot = state.media_import_dialog().expect("checked").clone();
    let (title, action_label, description) = match snapshot.mode {
        MediaDialogMode::Import => (
            "Import Media",
            "Import",
            "Probe a local H.264 video and register it as a project media asset. The file is referenced in place; media is never copied or modified.",
        ),
        MediaDialogMode::Relink(_) => (
            "Relink Media",
            "Relink",
            "Choose the replacement local file for this asset. Clip timing and source ranges stay unchanged, and the file must remain long enough for every clip that uses it.",
        ),
    };
    let mut path = snapshot.path.clone();
    let mut confirmed = false;
    let mut cancelled = false;
    let modal = egui::Modal::new(Id::unique("cutcraft_media_import"))
        .backdrop_color(egui::Color32::from_black_alpha(180))
        .frame(
            Frame::new()
                .fill(Palette::MODAL_BG)
                .stroke(Stroke::new(1.0, Palette::BORDER))
                .corner_radius(Radii::DIALOG)
                .inner_margin(Margin::ZERO),
        )
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
                        common::badge(ui, "Managed asset", Palette::ACCENT);
                        if let Some(name) = &snapshot.asset_name {
                            common::badge(ui, name, Palette::WARNING);
                        }
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if common::icon_button(ui, crate::widgets::icons::Icon::Close, "Close media dialog")
                                .clicked()
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
                    ui.label(
                        RichText::new(description)
                            .size(9.5)
                            .color(Palette::TEXT_SECONDARY),
                    );
                    ui.add_space(6.0);
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut path)
                            .hint_text("C:\\path\\to\\video.mp4")
                            .desired_width(ui.available_width()),
                    );
                    if response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter))
                    {
                        confirmed = true;
                    }
                    if let Some(error) = &snapshot.error {
                        ui.add_space(4.0);
                        ui.label(RichText::new(error).size(9.0).color(Palette::ERROR));
                    }
                    if let Some(message) = &snapshot.message {
                        ui.add_space(4.0);
                        ui.label(RichText::new(message).size(9.0).color(Palette::SUCCESS));
                    }
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "FFmpeg is discovered through CUTCRAFT_FFMPEG/CUTCRAFT_FFPROBE or PATH and is not bundled. AutoSave persists the asset metadata and locator, never media bytes.",
                        )
                        .size(8.2)
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
                                egui::Button::new(RichText::new("Close").size(10.5))
                                    .min_size(vec2(76.0, 28.0)),
                            )
                            .clicked()
                        {
                            cancelled = true;
                        }
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled(
                                    !snapshot.pending,
                                    egui::Button::new(
                                        RichText::new(if snapshot.pending {
                                            "Probing..."
                                        } else {
                                            action_label
                                        })
                                        .size(10.5)
                                        .color(Palette::ON_ACCENT),
                                    )
                                    .fill(if snapshot.pending {
                                        Palette::WIDGET_BG
                                    } else {
                                        Palette::ACCENT
                                    })
                                    .min_size(vec2(96.0, 28.0)),
                                )
                                .clicked()
                            {
                                confirmed = true;
                            }
                        });
                    });
                });
        });
    if let Some(dialog) = state.media_library_mut().dialog_mut() {
        dialog.path = path;
    }
    if confirmed {
        let _ = state.confirm_media_import();
        context.request_repaint();
    } else if cancelled || modal.should_close() {
        state.close_media_import_dialog();
        context.request_repaint();
    }
}
