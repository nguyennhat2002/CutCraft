//! Left library panel with Media, Audio, Text, Effects, and Transitions tabs.
//!
//! All tabs show empty states. No media is read, decoded, or generated.

use egui::{
    pos2, vec2, Align, Align2, CornerRadius, CursorIcon, FontId, Layout, Rect, Response, RichText,
    ScrollArea, Sense, Ui,
};

use crate::editor_state::{EditorSession, EditorTool};
use crate::theme::Palette;
use crate::widgets::common;
use crate::widgets::icons::{self, Icon};

pub fn show(ui: &mut Ui, session: &mut EditorSession) {
    ui.horizontal(|ui| {
        common::section_heading(ui, "Library");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            common::badge(ui, "0 assets", Palette::TEXT_MUTED);
        });
    });

    ui.add_space(6.0);

    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        for tool in EditorTool::ALL {
            if tool_row(ui, tool, session.active_tool == tool).clicked() {
                session.active_tool = tool;
            }
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(10.0);

    let _ = ScrollArea::vertical()
        .id_salt("library_scroll")
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .show(ui, |ui| {
            let (action, empty_title, empty_body, note) = tool_content(session.active_tool);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(session.active_tool.label())
                        .size(14.0)
                        .strong()
                        .color(Palette::TEXT_PRIMARY),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    common::badge(ui, "Empty", Palette::TEXT_MUTED);
                });
            });
            ui.add_space(8.0);

            common::disabled_action_button(ui, action, "This action is not implemented yet.");
            ui.add_space(10.0);
            common::compact_empty_state(
                ui,
                tool_icon(session.active_tool),
                empty_title,
                empty_body,
            );
            ui.add_space(8.0);
            ui.add(
                egui::Label::new(RichText::new(note).size(9.5).color(Palette::TEXT_DISABLED))
                    .wrap(),
            );
        });
}

fn tool_row(ui: &mut Ui, tool: EditorTool, selected: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
    let hovered = response.hovered();

    if selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), Palette::ACCENT_BG);
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, vec2(3.0, rect.height())),
            CornerRadius::same(2),
            Palette::ACCENT,
        );
    } else if hovered {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), Palette::WIDGET_BG);
    }

    let icon_rect =
        Rect::from_center_size(pos2(rect.left() + 18.0, rect.center().y), vec2(15.0, 15.0));
    icons::paint(
        ui.painter(),
        icon_rect,
        tool_icon(tool),
        if selected {
            Palette::ACCENT
        } else {
            Palette::TEXT_MUTED
        },
    );
    ui.painter().text(
        pos2(rect.left() + 34.0, rect.center().y),
        Align2::LEFT_CENTER,
        tool.label(),
        FontId::proportional(12.0),
        if selected || hovered {
            Palette::TEXT_PRIMARY
        } else {
            Palette::TEXT_SECONDARY
        },
    );

    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn tool_icon(tool: EditorTool) -> Icon {
    match tool {
        EditorTool::Media => Icon::Media,
        EditorTool::Audio => Icon::Audio,
        EditorTool::Text => Icon::Text,
        EditorTool::Effects => Icon::Effects,
        EditorTool::Transitions => Icon::Transitions,
    }
}

fn tool_content(tool: EditorTool) -> (&'static str, &'static str, &'static str, &'static str) {
    match tool {
        EditorTool::Media => (
            "+  Import Media",
            "No media in this session",
            "Imported clips will appear here once media import and decoding are implemented.",
            "No files are read and no video is decoded in this build.",
        ),
        EditorTool::Audio => (
            "+  Import Audio",
            "No audio in this session",
            "Imported audio and waveforms will appear here once audio support is implemented.",
            "No audio files are read and no audio is decoded in this build.",
        ),
        EditorTool::Text => (
            "+  Add Text",
            "No text yet",
            "Titles and subtitles will appear here once text editing is implemented.",
            "Text editing is a placeholder in this build.",
        ),
        EditorTool::Effects => (
            "Browse Effects",
            "No effects available",
            "Video effects and filters are not implemented yet.",
            "Effect processing is a placeholder in this build.",
        ),
        EditorTool::Transitions => (
            "Browse Transitions",
            "No transitions available",
            "Transitions between clips are not implemented yet.",
            "Transition processing is a placeholder in this build.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_library_tab_renders_without_media() {
        let context = egui::Context::default();

        for tool in EditorTool::ALL {
            let mut session = EditorSession {
                active_tool: tool,
                ..EditorSession::default()
            };
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(300.0, 700.0))),
                ..Default::default()
            };
            let output = context.run_ui(input, |ui| show(ui, &mut session));
            assert!(!output.shapes.is_empty());
            output.drop_without_applying_deltas();
        }
    }
}
