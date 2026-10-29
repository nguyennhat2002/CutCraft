//! Timeline toolbar components.
//!
//! Editing commands are unavailable, so edit controls are visibly disabled.
//! Snapping and linked-selection are explicit visual preferences: their state
//! persists in the session, but no editing behavior is claimed or applied.

use std::sync::LazyLock;

use egui::{pos2, vec2, Align, Color32, CursorIcon, Layout, Rect, RichText, Sense, Stroke, Ui};

use crate::config;
use crate::editor_state::EditorSession;
use crate::shortcuts::{CommandId, ShortcutManager};
use crate::theme::{Palette, Radii};
use crate::timeline_state::{ThumbnailMode, TimelineDisplayMode, TimelineSelectionKind};
use crate::widgets::common::{self, focus_ring, focus_ring_colored};
use crate::widgets::icons::{self, Icon, IconColors, IconSize};

const BUTTON_SIZE: f32 = 27.0;
const TOGGLE_WIDTH: f32 = 30.0;
const SEPARATOR_WIDTH: f32 = 9.0;
const CLIP_TOOLS: [Tool; 9] = [
    Tool::Split,
    Tool::TrimIn,
    Tool::TrimOut,
    Tool::Delete,
    Tool::Lock,
    Tool::Crop,
    Tool::DetachAudio,
    Tool::Freeze,
    Tool::Reverse,
];
const MEDIA_TOOLS: [Tool; 6] = [
    Tool::Captions,
    Tool::Adjust,
    Tool::AudioTools,
    Tool::TextTools,
    Tool::Filters,
    Tool::Transitions,
];
const COMPACT_CONTEXTUAL_TOOLS: [Tool; 2] = [Tool::Split, Tool::Delete];
static QA_ENABLED: LazyLock<bool> =
    LazyLock::new(|| std::env::var("CUTCRAFT_UI_DEMO").ok().as_deref() == Some("1"));

/// Toolbar density chosen from the available width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Density {
    Full,
    Compact,
}

impl Density {
    pub const FULL_MIN_WIDTH: f32 = 1360.0;

    pub fn for_width(width: f32) -> Self {
        if width >= Self::FULL_MIN_WIDTH {
            Self::Full
        } else {
            Self::Compact
        }
    }

    #[cfg(test)]
    pub fn shows_clip_tools(self) -> bool {
        self == Self::Full
    }
}

/// Honest implementation state for a toolbar control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolStatus {
    Implemented,
    Disabled,
    Planned,
    /// State persists in the session, but no editing behavior is applied.
    UiPreference,
}

/// Toolbar inventory used by both rendering and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    AddImport,
    Select,
    Undo,
    Redo,
    Split,
    TrimIn,
    TrimOut,
    Delete,
    Lock,
    Crop,
    DetachAudio,
    Freeze,
    Reverse,
    Captions,
    Adjust,
    AudioTools,
    TextTools,
    Filters,
    Transitions,
    Voiceover,
    Options,
    FitTimeline,
    Snapping,
    LinkedSelection,
    More,
    ZoomIn,
    ZoomOut,
}

impl Tool {
    #[cfg(test)]
    pub const ALL: [Self; 27] = [
        Self::AddImport,
        Self::Select,
        Self::Undo,
        Self::Redo,
        Self::Split,
        Self::TrimIn,
        Self::TrimOut,
        Self::Delete,
        Self::Lock,
        Self::Crop,
        Self::DetachAudio,
        Self::Freeze,
        Self::Reverse,
        Self::Captions,
        Self::Adjust,
        Self::AudioTools,
        Self::TextTools,
        Self::Filters,
        Self::Transitions,
        Self::Voiceover,
        Self::Options,
        Self::FitTimeline,
        Self::Snapping,
        Self::LinkedSelection,
        Self::More,
        Self::ZoomIn,
        Self::ZoomOut,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::AddImport => "Add / import",
            Self::Select => "Selection tool",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::Split => "Split at playhead",
            Self::TrimIn => "Trim start",
            Self::TrimOut => "Trim end",
            Self::Delete => "Delete selected items",
            Self::Lock => "Clip protection",
            Self::Crop => "Crop / transform",
            Self::DetachAudio => "Detach audio",
            Self::Freeze => "Freeze frame",
            Self::Reverse => "Reverse clip",
            Self::Captions => "Auto captions",
            Self::Adjust => "Video adjustment",
            Self::AudioTools => "Audio tools",
            Self::TextTools => "Text tools",
            Self::Filters => "Filters",
            Self::Transitions => "Transitions",
            Self::Voiceover => "Voice-over",
            Self::Options => "Timeline settings",
            Self::FitTimeline => "Fit timeline",
            Self::Snapping => "Snapping",
            Self::LinkedSelection => "Linked selection",
            Self::More => "More timeline tools",
            Self::ZoomIn => "Zoom in",
            Self::ZoomOut => "Zoom out",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::AddImport => "Media import is not implemented.",
            Self::Select => "Timeline item selection works; selection modes are not implemented.",
            Self::Undo | Self::Redo => "Undo and redo require editor commands.",
            Self::Split => "Splitting requires timeline commands.",
            Self::TrimIn | Self::TrimOut => "Trimming requires timeline commands.",
            Self::Delete => "Deleting items requires a timeline command.",
            Self::Lock => "Clip protection is planned.",
            Self::Crop => "Crop and transform are planned.",
            Self::DetachAudio => "Detaching audio is planned.",
            Self::Freeze => "Freeze frame is planned.",
            Self::Reverse => "Reverse clip is planned.",
            Self::Captions => "Caption tooling is not implemented.",
            Self::Adjust => "Video adjustment is not implemented.",
            Self::AudioTools => "Audio tools are planned.",
            Self::TextTools => "Text tools are planned.",
            Self::Filters => "Filters are planned.",
            Self::Transitions => "Transitions are planned.",
            Self::Voiceover => "Voice-over recording is not implemented.",
            Self::Options => "Timeline display preferences.",
            Self::FitTimeline => "Fit-to-window is not implemented.",
            Self::Snapping => "Visual preference only. Snapping behavior is not implemented.",
            Self::LinkedSelection => {
                "Visual preference only. Linked-selection behavior is not implemented."
            }
            Self::More => "Open the planned timeline tools menu.",
            Self::ZoomIn | Self::ZoomOut => "Change the timeline display scale.",
        }
    }

    pub fn status(self) -> ToolStatus {
        match self {
            Self::Snapping | Self::LinkedSelection => ToolStatus::UiPreference,
            Self::More | Self::Options | Self::ZoomIn | Self::ZoomOut => ToolStatus::Implemented,
            Self::Undo | Self::Redo | Self::Split | Self::Delete | Self::Select => {
                ToolStatus::Disabled
            }
            Self::AddImport
            | Self::TrimIn
            | Self::TrimOut
            | Self::Lock
            | Self::Crop
            | Self::DetachAudio
            | Self::Freeze
            | Self::Reverse
            | Self::Captions
            | Self::Adjust
            | Self::AudioTools
            | Self::TextTools
            | Self::Filters
            | Self::Transitions
            | Self::Voiceover
            | Self::FitTimeline => ToolStatus::Planned,
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Self::AddImport => Icon::Plus,
            Self::Select => Icon::Select,
            Self::Undo => Icon::Undo,
            Self::Redo => Icon::Redo,
            Self::Split => Icon::Split,
            Self::TrimIn => Icon::TrimIn,
            Self::TrimOut => Icon::TrimOut,
            Self::Delete => Icon::Delete,
            Self::Lock => Icon::Lock,
            Self::Crop => Icon::Crop,
            Self::DetachAudio => Icon::DetachAudio,
            Self::Freeze => Icon::Freeze,
            Self::Reverse => Icon::Reverse,
            Self::Captions => Icon::Captions,
            Self::Adjust => Icon::Adjust,
            Self::AudioTools => Icon::Audio,
            Self::TextTools => Icon::Text,
            Self::Filters => Icon::Filters,
            Self::Transitions => Icon::Transitions,
            Self::Voiceover => Icon::Voiceover,
            Self::Options => Icon::List,
            Self::FitTimeline => Icon::Fit,
            Self::Snapping => Icon::Snap,
            Self::LinkedSelection => Icon::Link,
            Self::More => Icon::More,
            Self::ZoomIn => Icon::ZoomIn,
            Self::ZoomOut => Icon::ZoomOut,
        }
    }

    /// Registered command represented by this control, when one exists.
    pub(crate) fn command_id(self) -> Option<CommandId> {
        match self {
            Self::AddImport => Some(CommandId::ImportMedia),
            Self::Select => Some(CommandId::SelectionTool),
            Self::Undo => Some(CommandId::Undo),
            Self::Redo => Some(CommandId::Redo),
            Self::Split => Some(CommandId::SplitAtPlayhead),
            Self::Delete => Some(CommandId::DeleteSelected),
            Self::DetachAudio => Some(CommandId::DetachAudio),
            Self::Freeze => Some(CommandId::FreezeFrame),
            Self::Snapping => Some(CommandId::SnappingToggle),
            Self::LinkedSelection => Some(CommandId::LinkedSelection),
            Self::FitTimeline => Some(CommandId::FitTimeline),
            Self::ZoomIn => Some(CommandId::ZoomIn),
            Self::ZoomOut => Some(CommandId::ZoomOut),
            _ => None,
        }
    }

    /// Clip tools common to both media kinds stay visible for mixed selections;
    /// kind-specific tools never imply applicability to the other media type.
    pub(crate) fn applies_to(self, kind: TimelineSelectionKind) -> bool {
        match self {
            Self::Split
            | Self::TrimIn
            | Self::TrimOut
            | Self::Delete
            | Self::Lock
            | Self::Reverse => true,
            Self::Crop
            | Self::DetachAudio
            | Self::Freeze
            | Self::Captions
            | Self::Adjust
            | Self::TextTools
            | Self::Filters
            | Self::Transitions => kind == TimelineSelectionKind::VideoOnly,
            Self::AudioTools => kind == TimelineSelectionKind::AudioOnly,
            _ => false,
        }
    }
}

/// Contextual inventory in toolbar order. Empty without a real selection.
pub(crate) fn contextual_tools(kind: Option<TimelineSelectionKind>) -> Vec<Tool> {
    let Some(kind) = kind else {
        return Vec::new();
    };
    CLIP_TOOLS
        .into_iter()
        .chain(MEDIA_TOOLS)
        .filter(|tool| tool.applies_to(kind))
        .collect()
}

pub fn show(ui: &mut Ui, session: &mut EditorSession, shortcuts: &ShortcutManager) {
    let density = Density::for_width(ui.available_width());
    let selection_kind = session.timeline_selection_context().kind();
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), config::TIMELINE_TOOLBAR_HEIGHT),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.add_space(4.0);
            left_groups(ui, density, selection_kind, shortcuts);

            if session.timeline_display_mode == TimelineDisplayMode::VisualDemo
                && density == Density::Full
            {
                ui.add_space(6.0);
                common::badge(ui, "Visual demo", Palette::WARNING)
                    .on_hover_text("Synthetic timeline for visual QA. Not project data.");
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(4.0);
                if *QA_ENABLED || session.timeline_display_mode == TimelineDisplayMode::VisualDemo {
                    qa_menu(ui, session);
                }
                zoom_controls(ui, session, density, shortcuts);
                if density == Density::Full {
                    let _ = disabled_tool(ui, Tool::FitTimeline, shortcuts);
                }
                separator(ui);
                if density == Density::Compact {
                    more_menu(ui, session, selection_kind, shortcuts);
                } else {
                    display_menu(ui, session);
                }
                separator(ui);
                if toggle_tool(
                    ui,
                    Tool::LinkedSelection,
                    session.timeline_linked_selection,
                    shortcuts,
                )
                .clicked()
                {
                    session.execute_shortcut_command(CommandId::LinkedSelection);
                }
                if toggle_tool(ui, Tool::Snapping, session.timeline_snapping, shortcuts).clicked() {
                    session.execute_shortcut_command(CommandId::SnappingToggle);
                }
                if density == Density::Full {
                    separator(ui);
                    let _ = disabled_tool(ui, Tool::Voiceover, shortcuts);
                }
            });
        },
    );
}

fn left_groups(
    ui: &mut Ui,
    density: Density,
    selection: Option<TimelineSelectionKind>,
    shortcuts: &ShortcutManager,
) {
    let _ = disabled_tool(ui, Tool::AddImport, shortcuts);
    separator(ui);
    dropdown_tool(ui, Tool::Select, shortcuts);
    separator(ui);
    let _ = disabled_tool(ui, Tool::Undo, shortcuts);
    let _ = disabled_tool(ui, Tool::Redo, shortcuts);

    let Some(selection) = selection else {
        return;
    };
    separator(ui);
    if density == Density::Compact {
        for tool in COMPACT_CONTEXTUAL_TOOLS {
            let _ = disabled_tool(ui, tool, shortcuts);
        }
        return;
    }
    for tool in CLIP_TOOLS
        .into_iter()
        .filter(|tool| tool.applies_to(selection))
    {
        let _ = disabled_tool(ui, tool, shortcuts);
    }
    let media = MEDIA_TOOLS
        .into_iter()
        .filter(|tool| tool.applies_to(selection))
        .collect::<Vec<_>>();
    if !media.is_empty() {
        separator(ui);
        for tool in media {
            let _ = disabled_tool(ui, tool, shortcuts);
        }
    }
}

fn separator(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(
        vec2(SEPARATOR_WIDTH, config::TIMELINE_TOOLBAR_HEIGHT),
        Sense::hover(),
    );
    ui.painter().line_segment(
        [
            pos2(rect.center().x, rect.center().y - 8.0),
            pos2(rect.center().x, rect.center().y + 8.0),
        ],
        Stroke::new(1.0, Palette::BORDER),
    );
}

fn disabled_tool(ui: &mut Ui, tool: Tool, shortcuts: &ShortcutManager) -> egui::Response {
    let width = if tool == Tool::Select {
        BUTTON_SIZE + 9.0
    } else {
        BUTTON_SIZE
    };
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, config::TIMELINE_TOOLBAR_HEIGHT), Sense::hover());
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect.shrink(1.0), Radii::CONTROL, Palette::WIDGET_BG);
    }
    let icon_rect = Rect::from_center_size(
        pos2(rect.left() + BUTTON_SIZE / 2.0, rect.center().y),
        vec2(18.0, 18.0),
    );
    icons::paint_sized(
        painter,
        icon_rect,
        tool.icon(),
        IconSize::Standard,
        IconColors::DISABLED,
    );
    if tool == Tool::Select {
        icons::paint_sized(
            painter,
            Rect::from_center_size(
                pos2(rect.right() - 9.0, rect.center().y + 5.0),
                vec2(16.0, 16.0),
            ),
            Icon::ChevronDown,
            IconSize::Small,
            IconColors::DISABLED,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::accesskit::Role::Button, false, tool.label())
    });
    response
        .on_hover_cursor(CursorIcon::NotAllowed)
        .on_hover_text(format!(
            "{} — {} · {}{}",
            tool.label(),
            status_label(tool.status()),
            tool.hint(),
            shortcut_suffix(shortcuts, tool)
        ))
}

fn dropdown_tool(ui: &mut Ui, tool: Tool, shortcuts: &ShortcutManager) -> egui::Response {
    disabled_tool(ui, tool, shortcuts)
}

fn enabled_icon_button(ui: &mut Ui, icon: Icon, label: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        vec2(BUTTON_SIZE, config::TIMELINE_TOOLBAR_HEIGHT),
        Sense::click(),
    );
    let hovered = response.hovered();
    let pressed = response.is_pointer_button_down_on();
    let painter = ui.painter();
    if pressed {
        painter.rect_filled(rect.shrink(1.0), Radii::CONTROL, Palette::WIDGET_ACTIVE);
    } else if hovered {
        painter.rect_filled(rect.shrink(1.0), Radii::CONTROL, Palette::WIDGET_BG);
    }
    icons::paint_sized(
        painter,
        rect,
        icon,
        IconSize::Standard,
        if hovered {
            IconColors::HOVERED
        } else {
            IconColors::NORMAL
        },
    );
    focus_ring(ui, &response, rect.shrink(1.0), Radii::CONTROL);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Button, true, label));
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(label)
}

fn status_label(status: ToolStatus) -> &'static str {
    match status {
        ToolStatus::Implemented => "Available",
        ToolStatus::Disabled => "Unavailable",
        ToolStatus::Planned => "Planned",
        ToolStatus::UiPreference => "UI preference",
    }
}

fn shortcut_suffix(shortcuts: &ShortcutManager, tool: Tool) -> String {
    tool.command_id()
        .and_then(|command| shortcuts.shortcut_hint(command))
        .map(|shortcut| format!(" · {shortcut}"))
        .unwrap_or_default()
}

fn toggle_tool(
    ui: &mut Ui,
    tool: Tool,
    active: bool,
    shortcuts: &ShortcutManager,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        vec2(TOGGLE_WIDTH, config::TIMELINE_TOOLBAR_HEIGHT),
        Sense::click(),
    );
    let hovered = response.hovered();
    let pressed = response.is_pointer_button_down_on();
    let fill = if active {
        if hovered {
            Palette::ACCENT_LIGHT
        } else {
            Palette::ACCENT
        }
    } else if pressed {
        Palette::WIDGET_ACTIVE
    } else if hovered {
        Palette::WIDGET_BG
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter();
    if fill != Color32::TRANSPARENT {
        painter.rect_filled(rect.shrink(1.0), Radii::CONTROL, fill);
    }
    icons::paint_sized(
        painter,
        rect,
        tool.icon(),
        IconSize::Standard,
        if active {
            IconColors::ON_ACCENT
        } else if hovered {
            IconColors::HOVERED
        } else {
            IconColors::MUTED
        },
    );
    if active {
        focus_ring_colored(
            ui,
            &response,
            rect.shrink(1.0),
            Radii::CONTROL,
            Palette::ON_ACCENT,
        );
    } else {
        focus_ring(ui, &response, rect.shrink(1.0), Radii::CONTROL);
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::accesskit::Role::CheckBox, true, active, tool.label())
    });
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(format!(
            "{} — {}{}",
            tool.label(),
            tool.hint(),
            shortcut_suffix(shortcuts, tool)
        ))
}

fn zoom_controls(
    ui: &mut Ui,
    session: &mut EditorSession,
    density: Density,
    shortcuts: &ShortcutManager,
) {
    let zoom_in = format!(
        "{}{}",
        Tool::ZoomIn.label(),
        shortcut_suffix(shortcuts, Tool::ZoomIn)
    );
    if enabled_icon_button(ui, Icon::ZoomIn, &zoom_in).clicked() {
        session.execute_shortcut_command(CommandId::ZoomIn);
    }
    let slider_width = if density == Density::Compact {
        78.0
    } else {
        148.0
    };
    ui.scope(|ui| {
        ui.spacing_mut().slider_width = slider_width;
        let response = ui.add(
            egui::Slider::new(
                &mut session.timeline_pixels_per_second,
                config::TIMELINE_MIN_PIXELS_PER_SECOND..=config::TIMELINE_MAX_PIXELS_PER_SECOND,
            )
            .logarithmic(true)
            .show_value(false),
        );
        let _ = response.on_hover_text(format!(
            "Timeline zoom · {:.0} px/s",
            session.timeline_pixels_per_second
        ));
    });
    let zoom_out = format!(
        "{}{}",
        Tool::ZoomOut.label(),
        shortcut_suffix(shortcuts, Tool::ZoomOut)
    );
    if enabled_icon_button(ui, Icon::ZoomOut, &zoom_out).clicked() {
        session.execute_shortcut_command(CommandId::ZoomOut);
    }
}

fn display_menu(ui: &mut Ui, session: &mut EditorSession) {
    let response = enabled_icon_button(ui, Icon::List, Tool::Options.label());
    let _ = egui::Popup::menu(&response)
        .id(ui.scope_id().with("timeline_display_menu"))
        .show(|ui| {
            ui.set_min_width(236.0);
            ui.spacing_mut().button_padding = vec2(8.0, 2.0);
            ui.spacing_mut().interact_size.y = 20.0;
            display_options(ui, session);
        });
}

fn display_options(ui: &mut Ui, session: &mut EditorSession) {
    menu_heading(ui, "Timeline display");
    ui.checkbox(
        &mut session.timeline_show_thumbnails,
        "Show video thumbnails",
    );
    ui.add_enabled_ui(session.timeline_show_thumbnails, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Thumbnail mode")
                    .size(10.0)
                    .color(Palette::TEXT_SECONDARY),
            );
            egui::ComboBox::from_id_salt("timeline_thumbnail_mode")
                .width(118.0)
                .selected_text(
                    RichText::new(session.timeline_thumbnail_mode.label())
                        .size(9.5)
                        .color(Palette::TEXT_PRIMARY),
                )
                .show_ui(ui, |ui| {
                    for mode in ThumbnailMode::ALL {
                        ui.selectable_value(
                            &mut session.timeline_thumbnail_mode,
                            mode,
                            mode.label(),
                        );
                    }
                });
        });
    });
    ui.checkbox(&mut session.timeline_show_waveforms, "Show audio waveforms");
    ui.label(
        RichText::new("Synthetic content appears only in Visual Demo mode.")
            .size(8.5)
            .color(Palette::TEXT_DISABLED),
    );
}

fn more_menu(
    ui: &mut Ui,
    session: &mut EditorSession,
    selection: Option<TimelineSelectionKind>,
    shortcuts: &ShortcutManager,
) {
    let response = enabled_icon_button(ui, Icon::More, Tool::More.label());
    let _ = egui::Popup::menu(&response)
        .id(ui.scope_id().with("timeline_tool_overflow"))
        .show(|ui| {
            ui.set_min_width(228.0);
            ui.spacing_mut().button_padding = vec2(8.0, 2.0);
            ui.spacing_mut().interact_size.y = 20.0;
            ui.spacing_mut().item_spacing.y = 2.0;
            let contextual = contextual_tools(selection)
                .into_iter()
                .filter(|tool| !COMPACT_CONTEXTUAL_TOOLS.contains(tool))
                .collect::<Vec<_>>();
            if !contextual.is_empty() {
                menu_heading(ui, "Selected clip tools");
                for tool in contextual {
                    menu_tool(ui, tool, shortcuts);
                }
                ui.separator();
            }
            display_options(ui, session);
            ui.separator();
            menu_heading(ui, "Planned timeline tools");
            for tool in [Tool::FitTimeline, Tool::Voiceover] {
                menu_tool(ui, tool, shortcuts);
            }
            if *QA_ENABLED || session.timeline_display_mode == TimelineDisplayMode::VisualDemo {
                ui.separator();
                let mut enabled = session.timeline_display_mode == TimelineDisplayMode::VisualDemo;
                if ui
                    .checkbox(&mut enabled, "Visual demo (not project data)")
                    .changed()
                {
                    session.set_timeline_demo(enabled);
                }
            }
        });
}

fn menu_heading(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(10.0)
            .strong()
            .color(Palette::TEXT_MUTED),
    );
    ui.add_space(2.0);
}

fn menu_tool(ui: &mut Ui, tool: Tool, shortcuts: &ShortcutManager) {
    let label = format!("{}{}", tool.label(), shortcut_suffix(shortcuts, tool));
    let item = ui.add_enabled(false, egui::Button::new(label));
    let _ = item.on_disabled_hover_text(tool.hint());
}

fn qa_menu(ui: &mut Ui, session: &mut EditorSession) {
    let _ = ui.menu_button("QA", |ui| {
        ui.set_min_width(230.0);
        let mut enabled = session.timeline_display_mode == TimelineDisplayMode::VisualDemo;
        if ui
            .checkbox(&mut enabled, "Visual demo (not project data)")
            .changed()
        {
            session.set_timeline_demo(enabled);
        }
        ui.label("Synthetic layout for visual QA only. No media is loaded.");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline_state::{
        TimelineItemId, TimelineItemView, TimelineSelectionKind, TimelineTrackView, TrackKind,
    };
    use egui::{Pos2, Rect, Shape};

    fn populated_session() -> EditorSession {
        let mut session = EditorSession::default();
        session.timeline_view.tracks = vec![
            TimelineTrackView {
                label: "V1".to_owned(),
                name: "Video".to_owned(),
                kind: TrackKind::Video,
                items: vec![TimelineItemView::new("Video clip", 1.0, 4.0).expect("valid")],
            },
            TimelineTrackView {
                label: "A1".to_owned(),
                name: "Audio".to_owned(),
                kind: TrackKind::Audio,
                items: vec![TimelineItemView::new("Audio clip", 1.0, 4.0).expect("valid")],
            },
        ];
        session
    }

    const VIDEO: TimelineItemId = TimelineItemId { track: 0, item: 0 };
    const AUDIO: TimelineItemId = TimelineItemId { track: 1, item: 0 };

    fn render(width: f32, session: &mut EditorSession) -> egui::FullOutput {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                vec2(width, config::TIMELINE_TOOLBAR_HEIGHT),
            )),
            ..Default::default()
        };
        context.run_ui(input, |ui| show(ui, session, &ShortcutManager::default()))
    }

    fn accent_rects(output: &egui::FullOutput) -> Vec<Rect> {
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Rect(rect) if rect.fill == Palette::ACCENT => Some(rect.rect),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn density_switches_at_the_compact_threshold() {
        assert_eq!(Density::for_width(1857.0), Density::Full);
        assert_eq!(Density::for_width(1600.0), Density::Full);
        assert_eq!(Density::for_width(1210.0), Density::Compact);
        assert_eq!(Density::for_width(1080.0), Density::Compact);
        assert!(!Density::Compact.shows_clip_tools());
        assert!(Density::Full.shows_clip_tools());
    }

    #[test]
    fn no_edit_tool_claims_to_be_implemented() {
        for tool in Tool::ALL {
            if matches!(
                tool,
                Tool::Snapping
                    | Tool::LinkedSelection
                    | Tool::More
                    | Tool::Options
                    | Tool::ZoomIn
                    | Tool::ZoomOut
            ) {
                continue;
            }
            assert_ne!(
                tool.status(),
                ToolStatus::Implemented,
                "{} must not claim implemented editing behavior",
                tool.label()
            );
        }
        for tool in [
            Tool::Undo,
            Tool::Redo,
            Tool::Split,
            Tool::Delete,
            Tool::Select,
        ] {
            assert_eq!(tool.status(), ToolStatus::Disabled);
        }
        assert_eq!(Tool::AddImport.status(), ToolStatus::Planned);
        assert_eq!(Tool::Snapping.status(), ToolStatus::UiPreference);
        assert_eq!(Tool::LinkedSelection.status(), ToolStatus::UiPreference);
        assert_eq!(Tool::ZoomIn.status(), ToolStatus::Implemented);
        assert_eq!(Tool::More.status(), ToolStatus::Implemented);
        assert_eq!(Tool::Options.status(), ToolStatus::Implemented);
    }

    #[test]
    fn compact_overflow_hides_secondary_tools_without_renaming_primary_tools() {
        assert_eq!(
            COMPACT_CONTEXTUAL_TOOLS,
            [Tool::Split, Tool::Delete],
            "selected clips keep only the primary contextual actions inline"
        );
        for tool in [
            Tool::AddImport,
            Tool::Select,
            Tool::Undo,
            Tool::Redo,
            Tool::Snapping,
            Tool::LinkedSelection,
            Tool::ZoomIn,
            Tool::ZoomOut,
        ] {
            assert!(
                Tool::ALL.contains(&tool),
                "{} must remain a global control",
                tool.label()
            );
        }
        assert!(Tool::ALL
            .iter()
            .any(|tool| tool.label() == "Crop / transform"));
        assert!(Tool::ALL
            .iter()
            .any(|tool| tool.label() == "Linked selection"));
    }

    #[test]
    fn toolbar_renders_in_one_row_at_required_widths() {
        for width in [1857.0, 1600.0, 1210.0, 1080.0] {
            let context = egui::Context::default();
            crate::theme::configure(&context);
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(width, config::TIMELINE_TOOLBAR_HEIGHT),
                )),
                ..Default::default()
            };
            let mut session = EditorSession::default();
            let output = context.run_ui(input, |ui| {
                show(ui, &mut session, &ShortcutManager::default())
            });
            assert!(
                !output.shapes.is_empty(),
                "toolbar did not render at {width}"
            );
            output.drop_without_applying_deltas();
        }
    }

    #[test]
    fn no_selection_produces_no_clip_or_media_tools() {
        let complete = contextual_tools(Some(TimelineSelectionKind::VideoOnly));
        assert!(!complete.is_empty());

        assert!(contextual_tools(None).is_empty());
        let empty = EditorSession::default();
        assert!(contextual_tools(empty.timeline_selection_context().kind()).is_empty());
        let populated = populated_session();
        assert!(contextual_tools(populated.timeline_selection_context().kind()).is_empty());
    }

    #[test]
    fn video_selection_offers_video_tools_and_hides_audio_only_tools() {
        let mut session = populated_session();
        assert!(session.select_timeline_item(VIDEO, false));
        let tools = contextual_tools(session.timeline_selection_context().kind());
        assert_eq!(
            tools,
            vec![
                Tool::Split,
                Tool::TrimIn,
                Tool::TrimOut,
                Tool::Delete,
                Tool::Lock,
                Tool::Crop,
                Tool::DetachAudio,
                Tool::Freeze,
                Tool::Reverse,
                Tool::Captions,
                Tool::Adjust,
                Tool::TextTools,
                Tool::Filters,
                Tool::Transitions,
            ]
        );
        assert!(!tools.contains(&Tool::AudioTools));
    }

    #[test]
    fn audio_selection_offers_audio_tools_and_hides_video_only_tools() {
        let mut session = populated_session();
        assert!(session.select_timeline_item(AUDIO, false));
        let tools = contextual_tools(session.timeline_selection_context().kind());
        assert_eq!(
            tools,
            vec![
                Tool::Split,
                Tool::TrimIn,
                Tool::TrimOut,
                Tool::Delete,
                Tool::Lock,
                Tool::Reverse,
                Tool::AudioTools,
            ]
        );
        for video_only in [
            Tool::Crop,
            Tool::DetachAudio,
            Tool::Freeze,
            Tool::Captions,
            Tool::Adjust,
            Tool::TextTools,
            Tool::Filters,
            Tool::Transitions,
        ] {
            assert!(!tools.contains(&video_only));
        }
    }

    #[test]
    fn mixed_selection_keeps_only_tools_common_to_both_media_kinds() {
        let mut session = populated_session();
        assert!(session.select_timeline_item(VIDEO, false));
        assert!(session.select_timeline_item(AUDIO, true));
        assert_eq!(
            session.timeline_selection_context().kind(),
            Some(TimelineSelectionKind::Mixed)
        );
        assert_eq!(
            contextual_tools(session.timeline_selection_context().kind()),
            vec![
                Tool::Split,
                Tool::TrimIn,
                Tool::TrimOut,
                Tool::Delete,
                Tool::Lock,
                Tool::Reverse,
            ]
        );
    }

    #[test]
    fn compact_selection_keeps_primary_context_and_overflows_the_rest() {
        let mut session = populated_session();
        assert!(session.select_timeline_item(VIDEO, false));
        let kind = session.timeline_selection_context().kind();
        for tool in COMPACT_CONTEXTUAL_TOOLS {
            assert!(contextual_tools(kind).contains(&tool));
        }
        let overflow = contextual_tools(kind)
            .into_iter()
            .filter(|tool| !COMPACT_CONTEXTUAL_TOOLS.contains(tool))
            .collect::<Vec<_>>();
        assert!(overflow.contains(&Tool::Crop));
        assert!(overflow.contains(&Tool::TrimIn));
    }

    #[test]
    fn selection_never_shifts_snapping_linked_selection_or_zoom() {
        for width in [1600.0, 1080.0] {
            let mut unselected = populated_session();
            let base = render(width, &mut unselected);
            let base_toggles = accent_rects(&base);
            assert_eq!(
                base_toggles.len(),
                2,
                "expected snapping and linked-selection highlights at {width}"
            );
            base.drop_without_applying_deltas();

            let mut selected = populated_session();
            assert!(selected.select_timeline_item(VIDEO, false));
            let with_selection = render(width, &mut selected);
            assert_eq!(accent_rects(&with_selection), base_toggles);
            with_selection.drop_without_applying_deltas();
        }
    }

    #[test]
    fn undo_and_redo_remain_disabled_and_outside_the_contextual_group() {
        let mut session = populated_session();
        assert_eq!(Tool::Undo.status(), ToolStatus::Disabled);
        assert_eq!(Tool::Redo.status(), ToolStatus::Disabled);

        assert!(session.select_timeline_item(VIDEO, false));
        let tools = contextual_tools(session.timeline_selection_context().kind());
        assert!(!tools.contains(&Tool::Undo));
        assert!(!tools.contains(&Tool::Redo));
        assert_eq!(Tool::Undo.status(), ToolStatus::Disabled);
        assert_eq!(Tool::Redo.status(), ToolStatus::Disabled);
    }

    #[test]
    fn toolbar_shortcut_hints_follow_live_profile_changes() {
        use crate::shortcuts::{ShortcutBinding, ShortcutDraft};
        use egui::Key;

        let mut manager = ShortcutManager::default();
        assert_eq!(shortcut_suffix(&manager, Tool::ZoomIn), " · Ctrl+=");
        assert_eq!(
            Tool::ZoomIn.command_id(),
            Some(CommandId::ZoomIn),
            "the toolbar must dispatch through the central registry"
        );

        let mut draft = ShortcutDraft::new(&manager);
        draft.set_binding(
            CommandId::ZoomIn,
            ShortcutBinding::new(Key::Z, true, false, true),
        );
        manager.save_draft(&draft).expect("save custom binding");
        assert_eq!(shortcut_suffix(&manager, Tool::ZoomIn), " · Ctrl+Alt+Z");
    }

    #[test]
    fn toolbar_clicks_never_change_timeline_selection() {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let mut session = populated_session();
        assert!(session.select_timeline_item(VIDEO, false));
        let selection_before = session.timeline_selection().clone();

        let first = render(1600.0, &mut session);
        let toggles = accent_rects(&first);
        first.drop_without_applying_deltas();
        let position = toggles[0].center();
        let screen = Some(Rect::from_min_size(
            Pos2::ZERO,
            vec2(1600.0, config::TIMELINE_TOOLBAR_HEIGHT),
        ));
        let mut frame = |events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: screen,
                events,
                ..Default::default()
            };
            let output = context.run_ui(input, |ui| {
                show(ui, &mut session, &ShortcutManager::default())
            });
            output.drop_without_applying_deltas();
        };
        frame(vec![egui::Event::PointerMoved(position)]);
        frame(vec![egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }]);
        frame(vec![egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);

        assert_eq!(session.timeline_selection(), &selection_before);
        assert_eq!(
            session.timeline_selection_context().kind(),
            Some(TimelineSelectionKind::VideoOnly)
        );
        assert!(
            !session.timeline_snapping || !session.timeline_linked_selection,
            "the clicked preference should have toggled"
        );
    }
}
