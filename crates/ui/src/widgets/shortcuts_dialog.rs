//! Reference-matched Shortcuts configuration dialog.
//!
//! The dialog only edits a pending draft. Save is the single commit point;
//! backdrop, X, Cancel and Escape discard uncommitted changes.

use egui::{
    pos2, vec2, Align, Align2, Color32, Context, CursorIcon, FontId, Frame, Id, Key, Layout,
    Margin, Painter, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui,
};

use crate::config;
use crate::shortcuts::{
    spec, CommandCategory, CommandId, CommandStatus, ShortcutManager, ShortcutProfile,
    ShortcutsDialogState,
};
use crate::theme::{Palette, Radii};
use crate::widgets::common;
use crate::widgets::icons::{self, Icon, IconColors, IconSize};

const ROW_HEIGHT: f32 = 34.0;
const FIELD_WIDTH: f32 = 172.0;
const CLEAR_WIDTH: f32 = 18.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutDialogOutcome {
    Open,
    Saved,
    Closed,
}

pub fn show(
    context: &Context,
    manager: &mut ShortcutManager,
    state: &mut ShortcutsDialogState,
) -> ShortcutDialogOutcome {
    if state.capturing.is_some() {
        process_capture(context, state);
    } else if state.conflict.is_some() {
        if consume_escape(context) {
            state.cancel_conflict();
        }
    } else if state.reset_requested && consume_escape(context) {
        state.cancel_reset();
    }

    let content = context.content_rect();
    let dialog_width = (content.width() - 80.0).clamp(
        config::SHORTCUTS_DIALOG_MIN_WIDTH,
        config::SHORTCUTS_DIALOG_MAX_WIDTH,
    );
    let dialog_height = (content.height() - 64.0).clamp(
        config::SHORTCUTS_DIALOG_MIN_HEIGHT,
        config::SHORTCUTS_DIALOG_MAX_HEIGHT,
    );
    let mut outcome = ShortcutDialogOutcome::Open;
    let mut close_requested = false;
    let mut save_requested = false;

    let modal_id = Id::unique("cutcraft_shortcuts_dialog");
    let modal = egui::Modal::new(modal_id)
        .area(egui::Modal::default_area(modal_id).fade_in(false))
        .backdrop_color(Color32::from_black_alpha(180))
        .frame(
            Frame::new()
                .fill(Palette::MODAL_BG)
                .stroke(Stroke::new(1.0, Palette::BORDER))
                .corner_radius(Radii::DIALOG)
                .inner_margin(Margin::ZERO),
        )
        .show(context, |ui| {
            ui.set_width(dialog_width);
            ui.set_height(dialog_height);

            header(ui, &mut close_requested);
            ui.separator();
            controls(ui, state);
            ui.separator();

            let body_height = (ui.available_height() - 58.0).max(160.0);
            Frame::new()
                .inner_margin(Margin::symmetric(18, 4))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.set_min_height(body_height);
                    body(ui, state, body_height);
                });

            ui.separator();
            footer(
                ui,
                manager,
                state,
                &mut save_requested,
                &mut close_requested,
            );
        });

    if save_requested {
        match state.save(manager) {
            Ok(()) => outcome = ShortcutDialogOutcome::Saved,
            Err(message) => state.save_error = Some(message),
        }
    } else if close_requested || modal.should_close() {
        outcome = ShortcutDialogOutcome::Closed;
    }

    outcome
}

fn process_capture(context: &Context, state: &mut ShortcutsDialogState) {
    let events = context.input(|input| input.events.clone());
    for event in &events {
        if matches!(event, egui::Event::Key { .. }) {
            let _ = state.capture_event(event);
        }
    }
    // Capture owns the keyboard until it resolves; nothing else may react to a
    // Tab, Space, arrow or Enter pressed while recording.
    context.input_mut(|input| {
        input
            .events
            .retain(|event| !matches!(event, egui::Event::Key { .. } | egui::Event::Text(_)));
    });
}

fn consume_escape(context: &Context) -> bool {
    let mut found = false;
    context.input_mut(|input| {
        input.events.retain(|event| {
            let is_escape = matches!(
                event,
                egui::Event::Key {
                    key: Key::Escape,
                    pressed: true,
                    ..
                }
            );
            found |= is_escape;
            !is_escape
        });
    });
    found
}

fn header(ui: &mut Ui, close_requested: &mut bool) {
    Frame::new()
        .inner_margin(Margin::symmetric(18, 6))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Shortcuts")
                        .size(15.0)
                        .strong()
                        .color(Palette::TEXT_PRIMARY),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if common::icon_button(ui, Icon::Close, "Close shortcuts dialog").clicked() {
                        *close_requested = true;
                    }
                });
            });
        });
}

fn controls(ui: &mut Ui, state: &mut ShortcutsDialogState) {
    Frame::new()
        .inner_margin(Margin::symmetric(18, 5))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for category in CommandCategory::ALL {
                    if category_tab(ui, category, state.category == category).clicked() {
                        state.category = category;
                        state.capturing = None;
                        state.conflict = None;
                        state.capture_warning = None;
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("shortcuts_profile")
                        .width(160.0)
                        .selected_text(
                            RichText::new(state.profile().label())
                                .size(10.5)
                                .color(Palette::TEXT_PRIMARY),
                        )
                        .show_ui(ui, |ui| {
                            for profile in ShortcutProfile::ALL {
                                if ui
                                    .selectable_label(state.profile() == profile, profile.label())
                                    .clicked()
                                {
                                    state.set_profile(profile);
                                }
                            }
                        });
                });
            });
        });
}

fn category_tab(ui: &mut Ui, category: CommandCategory, selected: bool) -> egui::Response {
    let label = category.label();
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        FontId::proportional(10.5),
        Palette::TEXT_PRIMARY,
    );
    let (rect, response) =
        ui.allocate_exact_size(vec2(galley.size().x + 24.0, 26.0), Sense::click());
    let hovered = response.hovered();
    if selected {
        ui.painter()
            .rect_filled(rect, Radii::CONTROL, Palette::WIDGET_ACTIVE);
    } else if hovered {
        ui.painter()
            .rect_filled(rect, Radii::CONTROL, Palette::WIDGET_BG);
    }
    ui.painter().galley(
        rect.center() - galley.size() / 2.0,
        galley,
        if selected || hovered {
            Palette::TEXT_PRIMARY
        } else {
            Palette::TEXT_MUTED
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::accesskit::Role::Tab, true, selected, label)
    });
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(format!("Show {} shortcuts", label))
}

fn body(ui: &mut Ui, state: &mut ShortcutsDialogState, body_height: f32) {
    let mut banners = 0.0;
    if let Some(command) = state.capturing {
        capture_banner(ui, command);
        banners += 27.0;
    }
    if let Some(prompt) = state.conflict.clone() {
        conflict_banner(ui, state, &prompt);
        banners += 62.0;
    }
    if let Some(warning) = &state.capture_warning {
        warning_line(ui, warning, Palette::WARNING);
        banners += 20.0;
    }
    if let Some(error) = &state.save_error {
        warning_line(ui, error, Palette::ERROR);
        banners += 20.0;
    }

    let list_height = (body_height - banners - ui.spacing().item_spacing.y * 4.0).max(120.0);
    Frame::new()
        .fill(Palette::PANEL_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
        .corner_radius(Radii::CARD)
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ScrollArea::vertical()
                .id_salt("shortcuts_list_scroll")
                .auto_shrink([false, false])
                .max_height(list_height)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    let commands = state.commands();
                    if ui.available_width() >= 560.0 {
                        let split = commands.len().div_ceil(2);
                        ui.columns(2, |columns| {
                            for command in &commands[..split] {
                                command_row(&mut columns[0], state, *command);
                            }
                            for command in &commands[split..] {
                                command_row(&mut columns[1], state, *command);
                            }
                        });
                    } else {
                        for command in commands {
                            command_row(ui, state, *command);
                        }
                    }
                });
        });
}

fn capture_banner(ui: &mut Ui, command: CommandId) {
    Frame::new()
        .fill(Palette::ACCENT_BG)
        .stroke(Stroke::new(1.0, Palette::ACCENT_DIM))
        .corner_radius(Radii::CONTROL)
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new(format!(
                    "Recording shortcut for {} · press a key combination · Esc to cancel",
                    command.label()
                ))
                .size(9.5)
                .color(Palette::ACCENT_LIGHT),
            );
        });
}

fn conflict_banner(
    ui: &mut Ui,
    state: &mut ShortcutsDialogState,
    prompt: &crate::shortcuts::ConflictPrompt,
) {
    let names = prompt
        .conflicts
        .iter()
        .map(|command| command.label())
        .collect::<Vec<_>>()
        .join(", ");
    Frame::new()
        .fill(Color32::from_rgb(0x35, 0x2b, 0x1c))
        .stroke(Stroke::new(1.0, Palette::WARNING.gamma_multiply(0.6)))
        .corner_radius(Radii::CONTROL)
        .inner_margin(Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} is already used by {names}. Reassign?",
                        prompt.binding.label()
                    ))
                    .size(9.5)
                    .color(Palette::WARNING),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(RichText::new("Reassign").size(9.5)))
                        .clicked()
                    {
                        state.reassign_conflict();
                    }
                    ui.add_space(4.0);
                    if ui
                        .add(egui::Button::new(RichText::new("Cancel").size(9.5)))
                        .clicked()
                    {
                        state.cancel_conflict();
                    }
                });
            });
        });
}

fn warning_line(ui: &mut Ui, text: &str, color: Color32) {
    ui.label(RichText::new(text).size(9.0).color(color));
}

fn command_row(ui: &mut Ui, state: &mut ShortcutsDialogState, command: CommandId) {
    let Some(spec) = spec(command) else {
        return;
    };
    let binding = state.draft.binding(command);
    let capturing = state.capturing == Some(command);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());

    let clear_space = if binding.is_some() && !capturing {
        CLEAR_WIDTH
    } else {
        0.0
    };
    let field_rect = Rect::from_min_size(
        pos2(
            rect.right() - FIELD_WIDTH - clear_space,
            rect.center().y - 11.0,
        ),
        vec2(FIELD_WIDTH, 22.0),
    );
    let label_rect = Rect::from_min_max(
        rect.min,
        pos2((field_rect.left() - 8.0).max(rect.left()), rect.bottom()),
    );
    let painter = ui.painter_at(label_rect);
    let label_color = if spec.status == CommandStatus::Implemented {
        Palette::TEXT_SECONDARY
    } else {
        Palette::TEXT_MUTED
    };
    if spec.status == CommandStatus::Implemented {
        painter.text(
            pos2(label_rect.left() + 4.0, rect.center().y),
            Align2::LEFT_CENTER,
            command.label(),
            FontId::proportional(10.5),
            label_color,
        );
    } else {
        painter.text(
            pos2(label_rect.left() + 4.0, rect.center().y - 4.0),
            Align2::LEFT_CENTER,
            command.label(),
            FontId::proportional(10.5),
            label_color,
        );
        painter.text(
            pos2(label_rect.left() + 4.0, rect.center().y + 8.0),
            Align2::LEFT_CENTER,
            "Unavailable",
            FontId::proportional(8.0),
            Palette::TEXT_DISABLED,
        );
    }

    let field_id = ui.scope_id().with(("shortcut_field", command.id()));
    let field_response = ui.interact(field_rect, field_id, Sense::click());
    let field_hovered = field_response.hovered();
    let field_clicked = field_response.clicked();
    let field_fill = if capturing {
        Palette::ACCENT_BG
    } else if field_hovered {
        Palette::WIDGET_HOVER
    } else {
        Color32::TRANSPARENT
    };
    if field_fill != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(field_rect, Radii::CONTROL, field_fill);
    }
    if capturing || field_hovered {
        ui.painter().rect_stroke(
            field_rect,
            Radii::CONTROL,
            Stroke::new(
                1.0,
                if capturing {
                    Palette::ACCENT
                } else {
                    Palette::BORDER_SUBTLE
                },
            ),
            StrokeKind::Inside,
        );
    }
    if capturing {
        ui.painter().text(
            pos2(field_rect.left() + 8.0, field_rect.center().y),
            Align2::LEFT_CENTER,
            "Press shortcut…",
            FontId::proportional(9.5),
            Palette::ACCENT_LIGHT,
        );
    } else if let Some(binding) = binding {
        paint_keycaps(
            ui.painter(),
            field_rect.shrink2(vec2(6.0, 2.0)),
            &binding.keycaps(),
            spec.status == CommandStatus::Implemented,
        );
    } else {
        ui.painter().text(
            pos2(field_rect.right() - 7.0, field_rect.center().y),
            Align2::RIGHT_CENTER,
            "Unassigned",
            FontId::proportional(9.0),
            Palette::TEXT_DISABLED,
        );
    }
    field_response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::accesskit::Role::Button,
            true,
            format!("Change shortcut for {}", command.label()),
        )
    });
    let _ = field_response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(format!(
            "{}: click to record a new shortcut{}",
            command.label(),
            if spec.status == CommandStatus::Planned {
                " (command is not implemented)"
            } else {
                ""
            }
        ));
    if field_clicked {
        state.start_capture(command);
    }

    if binding.is_some() && !capturing {
        let clear_rect = Rect::from_center_size(
            pos2(
                field_rect.right() + CLEAR_WIDTH / 2.0,
                field_rect.center().y,
            ),
            vec2(16.0, 16.0),
        );
        let clear = ui.interact(
            clear_rect,
            ui.scope_id().with(("shortcut_clear", command.id())),
            Sense::click(),
        );
        let clear_clicked = clear.clicked();
        icons::paint_sized(
            ui.painter(),
            clear_rect,
            Icon::Close,
            IconSize::Small,
            if clear.hovered() {
                IconColors::NORMAL
            } else {
                IconColors::DISABLED
            },
        );
        clear.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::accesskit::Role::Button,
                true,
                format!("Clear shortcut for {}", command.label()),
            )
        });
        let _ = clear
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(format!("Clear binding for {}", command.label()));
        if clear_clicked {
            state.clear_binding(command);
        }
    }
}

fn paint_keycaps(painter: &Painter, rect: Rect, caps: &[String], available: bool) {
    let color = if available {
        Palette::TEXT_PRIMARY
    } else {
        Palette::TEXT_MUTED
    };
    let galley_font = FontId::proportional(9.0);
    let mut widths = Vec::with_capacity(caps.len());
    let mut total = 0.0;
    for cap in caps {
        let width = painter
            .layout_no_wrap(cap.clone(), galley_font.clone(), color)
            .size()
            .x
            + 10.0;
        total += width;
        widths.push(width);
    }
    total += 3.0 * caps.len().saturating_sub(1) as f32;
    let mut x = rect.right() - total;
    for (cap, width) in caps.iter().zip(widths) {
        let cap_rect = Rect::from_min_size(pos2(x, rect.center().y - 7.0), vec2(width, 14.0));
        painter.rect_filled(cap_rect, Radii::SMALL, Palette::WIDGET_BG);
        painter.rect_stroke(
            cap_rect,
            Radii::SMALL,
            Stroke::new(1.0, Palette::BORDER),
            StrokeKind::Inside,
        );
        let galley = painter.layout_no_wrap(cap.clone(), galley_font.clone(), color);
        painter.galley(cap_rect.center() - galley.size() / 2.0, galley, color);
        x += width + 3.0;
    }
}

fn footer(
    ui: &mut Ui,
    manager: &ShortcutManager,
    state: &mut ShortcutsDialogState,
    save_requested: &mut bool,
    close_requested: &mut bool,
) {
    Frame::new()
        .inner_margin(Margin::symmetric(18, 10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            if state.reset_requested {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Reset all custom shortcuts to defaults?")
                            .size(10.0)
                            .color(Palette::WARNING),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if footer_cancel(ui, "Cancel").clicked() {
                            state.cancel_reset();
                        }
                        ui.add_space(6.0);
                        if reset_confirm_button(ui).clicked() {
                            state.confirm_reset();
                        }
                    });
                });
                return;
            }

            ui.horizontal(|ui| {
                if footer_secondary(ui, "Reset").clicked() {
                    state.request_reset();
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Bindings use the active keyboard layout.")
                        .size(8.5)
                        .color(Palette::TEXT_DISABLED),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if footer_cancel(ui, "Cancel").clicked() {
                        *close_requested = true;
                    }
                    ui.add_space(6.0);
                    let can_save = state.is_dirty(manager)
                        && state.capturing.is_none()
                        && state.conflict.is_none();
                    let save = ui.add_enabled(
                        can_save,
                        egui::Button::new(RichText::new("Save").size(11.0).color(if can_save {
                            Palette::ON_ACCENT
                        } else {
                            Palette::TEXT_DISABLED
                        }))
                        .fill(if can_save {
                            Palette::ACCENT
                        } else {
                            Palette::ACCENT_BG
                        })
                        .corner_radius(Radii::CONTROL)
                        .min_size(vec2(78.0, 28.0)),
                    );
                    if save.clicked() {
                        *save_requested = true;
                    }
                    let _ = save.on_hover_text(if can_save {
                        "Save shortcut changes".to_owned()
                    } else {
                        "No pending shortcut changes.".to_owned()
                    });
                });
            });
        });
}

fn footer_secondary(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(10.5)
                .color(Palette::TEXT_SECONDARY),
        )
        .fill(Palette::WIDGET_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
        .corner_radius(Radii::CONTROL)
        .min_size(vec2(66.0, 28.0)),
    )
}

fn footer_cancel(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).size(10.5).color(Palette::TEXT_PRIMARY))
            .fill(Palette::WIDGET_BG)
            .stroke(Stroke::new(1.0, Palette::BORDER))
            .corner_radius(Radii::CONTROL)
            .min_size(vec2(76.0, 28.0)),
    )
}

fn reset_confirm_button(ui: &mut Ui) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new("Reset").size(10.5).color(Palette::ON_ACCENT))
            .fill(Palette::WARNING)
            .corner_radius(Radii::CONTROL)
            .min_size(vec2(66.0, 28.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcuts::ShortcutBinding;
    use egui::{Event, Modifiers, RawInput};

    fn input(events: Vec<Event>, width: f32, height: f32) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(width, height))),
            events,
            ..Default::default()
        }
    }

    fn texts(output: &egui::FullOutput) -> Vec<String> {
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    }

    fn render(
        manager: &mut ShortcutManager,
        state: &mut ShortcutsDialogState,
        events: Vec<Event>,
        width: f32,
        height: f32,
    ) -> egui::FullOutput {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let first = context.run_ui(input(events, width, height), |ui| {
            let _ = show(ui.ctx(), manager, state);
        });
        first.drop_without_applying_deltas();
        context.run_ui(input(Vec::new(), width, height), |ui| {
            let _ = show(ui.ctx(), manager, state);
        })
    }

    fn key_event(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn dialog_renders_reference_structure_at_two_sizes() {
        for (width, height) in [(1600.0, 900.0), (960.0, 600.0)] {
            let mut manager = ShortcutManager::default();
            let mut state = ShortcutsDialogState::new(&manager);
            let output = render(&mut manager, &mut state, Vec::new(), width, height);
            let labels = texts(&output);
            output.drop_without_applying_deltas();
            for expected in [
                "Shortcuts",
                "Timeline",
                "Player",
                "Basic",
                "Other",
                "CutCraft Default",
                "Selection Tool",
                "Zoom In",
                "Reset",
                "Save",
                "Cancel",
            ] {
                assert!(
                    labels.iter().any(|label| label == expected),
                    "missing '{expected}' at {width}x{height}: {labels:?}"
                );
            }
        }
    }

    #[test]
    fn category_tabs_change_the_visible_command_set() {
        let mut manager = ShortcutManager::default();
        let mut state = ShortcutsDialogState::new(&manager);
        state.category = CommandCategory::Player;
        let output = render(&mut manager, &mut state, Vec::new(), 1600.0, 900.0);
        let labels = texts(&output);
        output.drop_without_applying_deltas();
        assert!(labels.iter().any(|label| label == "Play / Pause"));
        assert!(!labels.iter().any(|label| label == "Split at Playhead"));

        state.category = CommandCategory::Basic;
        let output = render(&mut manager, &mut state, Vec::new(), 1600.0, 900.0);
        let labels = texts(&output);
        output.drop_without_applying_deltas();
        assert!(labels.iter().any(|label| label == "New Project"));
        assert!(labels.iter().any(|label| label == "Shortcuts Dialog"));
    }

    #[test]
    fn capture_through_the_dialog_records_modifiers_and_cancels_with_escape() {
        let mut manager = ShortcutManager::default();
        let mut state = ShortcutsDialogState::new(&manager);
        state.start_capture(CommandId::ZoomIn);
        let output = render(
            &mut manager,
            &mut state,
            vec![key_event(
                Key::K,
                Modifiers {
                    ctrl: true,
                    alt: true,
                    ..Default::default()
                },
            )],
            1600.0,
            900.0,
        );
        output.drop_without_applying_deltas();
        assert_eq!(
            state.draft.binding(CommandId::ZoomIn),
            Some(ShortcutBinding::new(Key::K, true, false, true))
        );

        state.start_capture(CommandId::ZoomOut);
        let output = render(
            &mut manager,
            &mut state,
            vec![key_event(Key::Escape, Modifiers::NONE)],
            1600.0,
            900.0,
        );
        output.drop_without_applying_deltas();
        assert_eq!(state.capturing, None);
        assert_eq!(
            state.draft.binding(CommandId::ZoomOut),
            Some(ShortcutBinding::new(Key::Minus, true, false, false))
        );
    }

    #[test]
    fn conflict_warning_is_rendered_and_reassign_is_explicit() {
        let mut manager = ShortcutManager::default();
        let mut state = ShortcutsDialogState::new(&manager);
        state.start_capture(CommandId::ZoomIn);
        let output = render(
            &mut manager,
            &mut state,
            vec![key_event(
                Key::E,
                Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
            )],
            1600.0,
            900.0,
        );
        let labels = texts(&output);
        output.drop_without_applying_deltas();
        assert!(state.conflict.is_some());
        assert!(
            labels
                .iter()
                .any(|label| label.contains("is already used by Export")),
            "conflict text missing: {labels:?}"
        );
        assert!(labels.iter().any(|label| label == "Reassign"));

        let output = render(&mut manager, &mut state, Vec::new(), 1600.0, 900.0);
        output.drop_without_applying_deltas();
        state.reassign_conflict();
        assert_eq!(state.draft.binding(CommandId::Export), None);
    }

    #[test]
    fn reset_confirmation_is_explicit() {
        let mut manager = ShortcutManager::default();
        let mut state = ShortcutsDialogState::new(&manager);
        state.clear_binding(CommandId::ZoomIn);
        let output = render(&mut manager, &mut state, Vec::new(), 1600.0, 900.0);
        let labels = texts(&output);
        output.drop_without_applying_deltas();
        assert!(labels.iter().any(|label| label == "Save"));

        state.request_reset();
        let output = render(&mut manager, &mut state, Vec::new(), 1600.0, 900.0);
        let labels = texts(&output);
        output.drop_without_applying_deltas();
        assert!(labels
            .iter()
            .any(|label| label.contains("Reset all custom shortcuts")));
        assert!(labels.iter().any(|label| label == "Reset"));
    }
}
