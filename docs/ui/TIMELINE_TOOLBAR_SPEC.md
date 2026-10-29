# Timeline Toolbar Specification

## Source

- Reference: user-supplied CapCut timeline toolbar screenshot (external, not stored in the repository).
- Size: **1857 × 35 px**
- The reference is external and is **not** copied into this repository.
- Measurements below are approximate pixel readings from the original image.
- Icons whose function cannot be established from the screenshot are recorded as
  **UNRESOLVED**; no authoritative CapCut name is invented for them.

## Reference measurements

| Property | Value |
|---|---|
| Toolbar height | 35 px |
| Background | `#262626` |
| Outer edge | `#141414` |
| Typical icon glyph | 12–19 px |
| Primary hit target | ≈26–28 px square |
| Separator | 1 px vertical line, ≈17 px tall |
| Active toggle | solid cyan `#00C1CD` with dark glyph |
| Disabled glyph | ≈`#7E7E7E` |
| Left group end | ≈x 760 |
| Empty center | ≈x 760–1417 |
| Right group start | ≈x 1417 |
| Right edge inset | ≈14 px |

### Left side (x ≈ 0–760)

| Approx. x | Observed control | Reliability |
|---|---|---|
| 9–20 | Add / import (`+`) | high |
| 35 | Separator | high |
| 50–61 + 75–81 | Cursor/selection tool + dropdown chevron | high |
| 96 | Separator | high |
| 119–130 | Undo | high |
| 155–166 | Redo | high |
| 191–202 | Split/bracket family icon | **UNRESOLVED variant** |
| 227–238 | Split/bracket family icon | **UNRESOLVED variant** |
| 263–274 | Split/bracket family icon | **UNRESOLVED variant** |
| 298–311 | Delete/trash | high |
| 335–346 | Shield/protection-like icon | medium |
| 369–384 | Crop/transform-like icon | medium |
| 404–422 | Scissors + sparkle-like icon | **UNRESOLVED** |
| 437 | Separator | high |
| 455–759 | Dense media/enhancement cluster | mostly **UNRESOLVED** |

The media cluster contains shapes suggesting audio, person/subject, sparkle,
HD, video, and waveform tools. The screenshot alone is insufficient to claim
their exact operations, so CutCraft uses documented planned categories instead
of assigning unsupported AI names.

### Right side (x ≈ 1417–1857)

| Approx. x | Observed control | Reliability |
|---|---|---|
| 1417–1428 | Microphone / voice-over | high |
| 1464–1479 | Bracket-arrow-like icon | **UNRESOLVED** |
| 1491–1553 | Two cyan active toggle blocks | medium; likely editing preferences |
| 1560–1567 | Toggle dropdown chevron | high |
| 1585–1617 | List/cursor-like control + chevron | **UNRESOLVED** |
| 1648–1665 | Fit/frame-like tool | medium |
| 1686–1699 | Zoom out (magnifier −) | high |
| 1717–1812 | Zoom slider | high |
| 1830–1843 | Zoom in (magnifier +) | high |

## CutCraft implementation inventory

The toolbar is defined in
`crates/ui/src/widgets/timeline/toolbar.rs`. The control list is intentionally
honest: editing commands are not implemented in the engine, so edit controls do
not pretend to work.

| Group | Control | Icon | Status | Shortcut | Actual behavior |
|---|---|---|---|---|---|
| A | Add / import | `+` | PLANNED | none | Tooltip; disabled |
| A | Selection tool | cursor | DISABLED | none | Timeline clip clicks select items; pointer-mode switching is disabled |
| A | Undo | curved arrow | DISABLED | none | Tooltip; no history exists |
| A | Redo | curved arrow | DISABLED | none | Tooltip; no history exists |
| A | Split at playhead | split | DISABLED | none | Contextual only; tooltip; no command path |
| A | Trim start | trim-in | PLANNED | none | Contextual only; disabled |
| A | Trim end | trim-out | PLANNED | none | Contextual only; disabled |
| A | Delete selected items | trash | DISABLED | none | Contextual only; disabled |
| B | Clip protection | lock | PLANNED | none | Disabled |
| B | Crop / transform | crop | PLANNED | none | Disabled |
| B | Detach audio | detached waveform | PLANNED | none | Disabled |
| B | Freeze frame | snowflake | PLANNED | none | Disabled |
| B | Reverse clip | reverse arrows | PLANNED | none | Disabled |
| C | Auto captions | captions | PLANNED | none | Disabled |
| C | Video adjustment | sliders | PLANNED | none | Disabled |
| C | Audio tools | audio | PLANNED | none | Disabled |
| C | Text tools | text | PLANNED | none | Disabled |
| C | Filters | circles | PLANNED | none | Disabled |
| C | Transitions | transition glyph | PLANNED | none | Disabled |
| D | Voice-over | microphone | PLANNED | none | Disabled |
| D | Snapping | snap | **UI PREFERENCE** | none | Toggles session preference; editing behavior is not implemented |
| D | Linked selection | link | **UI PREFERENCE** | none | Toggles session preference; editing behavior is not implemented |
| D | Timeline display | list | **IMPLEMENTED** | none | Opens thumbnail/waveform display preferences |
| E | Fit timeline | fit | PLANNED | none | Disabled |
| E | Zoom out | magnifier − | **IMPLEMENTED** | none | Changes timeline display scale |
| E | Zoom slider | slider | **IMPLEMENTED** | none | Changes timeline display scale |
| E | Zoom in | magnifier + | **IMPLEMENTED** | none | Changes timeline display scale |
| — | More timeline tools | ellipsis | **IMPLEMENTED** | none | Opens compact overflow menu |

### Toolbar component helpers

- `disabled_tool` — disabled/planned icon button with status tooltip and
  accessible disabled Button role.
- `dropdown_tool` — disabled selection control with explicit chevron.
- `toggle_tool` — enabled toggle with active, hover, pressed and focus states;
  accessible CheckBox role and selected state.
- `enabled_icon_button` — enabled icon action with hover/pressed/focus states.
- `separator` — 1 px neutral separator.
- `zoom_controls` — real zoom-out/slider/zoom-in group.
- `more_menu` — compact overflow popup with honestly disabled planned tools.

## Toggle semantics

Snapping and linked selection are explicitly **UI preferences**:

- The state is stored in `EditorSession` and survives Home/Resume within the
  temporary session.
- The active state is visually distinct (cyan fill).
- Hover, pressed and keyboard-focus rings are implemented.
- Tooltips explicitly say that no snapping or linked-selection behavior is
  applied because timeline editing is not implemented.
- A new project resets both preferences to their defaults (enabled).

## Zoom behavior

The zoom controls are connected to the existing timeline presentation scale:

- zoom range is clamped to `4..=80 px/s`;
- ruler major ticks and minor ticks are generated from the active scale;
- clip x positions and widths use the same scale;
- horizontal scroll state is kept in the session;
- the playhead remains at time zero and is derived from the same scale;
- no percentage-only zoom control exists.

## Contextual selection policy (M1.7.1)

The clip/media group is driven by the session's real timeline selection, never
by track count or media-library state.

| State | Clip/media group | Applicable tools |
|---|---|---|
| Empty project | Hidden, including separators and spacing | none |
| Populated, no selection | Hidden | none |
| Video item(s) selected | Visible | split, trim, delete, lock, crop, detach audio, freeze, reverse, captions, adjust, text, filters, transitions |
| Audio item(s) selected | Visible | split, trim, delete, lock, reverse, audio tools |
| Mixed selection | Visible | only tools common to both kinds |
| Unknown/removed selection ids | Hidden | none |

- Clicking a clip selects it; Ctrl/Cmd- or Shift-click toggles additive
  selection. Clicking empty timeline body clears selection.
- Track headers, the ruler, media-pool content and toolbar buttons never select
  clips. Selecting a media asset therefore leaves the contextual group hidden.
- Format switches and supplied-view refreshes prune stale ids. New projects
  start with no selection.
- The toolbar height is fixed. Appearing/disappearing contextual controls do not
  move the ruler, canvas, zoom slider, snapping or linked-selection toggles.
- Group separators are emitted only when the group is visible; no placeholder
  icons or reserved spacing remain.

### Contextual responsive behavior

| Width | Density | Contextual tools when no selection | Contextual tools when selected |
|---|---|---|---|
| 1857 px | Full | Hidden | Split/trim/delete/clip/media tools inline |
| 1600 px | Full | Hidden | Split/trim/delete/clip/media tools inline |
| 1210 px | Compact | Hidden | Split and delete inline; remaining applicable tools in More |
| 1080 px | Compact | Hidden | Split and delete inline; remaining applicable tools in More |

The More button remains present in compact density regardless of selection, so
the right-aligned global controls never shift.

## Responsive policy

`Density::for_width` switches to compact below **1360 px**:

| Width | Density | Secondary clip/media tools |
|---|---|---|
| 1857 px | Full | Visible only with selection |
| 1600 px | Full | Visible only with selection |
| 1210 px | Compact | In More menu, only with selection |
| 1080 px | Compact | In More menu, only with selection |

Add/import, selection, undo/redo, zoom and both preferences remain visible in
one row; overflow items are disabled planned tools, never silent replacements
for working actions.

## Shortcuts

No editing shortcut is assigned because no editing command is implemented.
Tooltips therefore do not advertise `Ctrl+Z`, `Delete`, or other bindings for
unavailable actions. Zoom controls remain pointer-driven.

## Empty vs populated timeline

- Empty timeline: no contextual group. Add/import, selection, undo/redo, zoom
  and preferences remain visible; no tracks or clips are created by the toolbar.
- Populated timeline with no selection: identical to the empty state for clip
  tools; the contextual group appears only after a real item is selected.
- Selected timeline item: applicable clip/media tools appear disabled; editing
  commands remain unimplemented and no engine command path is claimed.
- Visual demo mode uses the same selection behavior for QA, and switching the
  display mode clears selection so ids never carry over.

## M1.7.1 visual evidence

Real application captures at 1600×900 client size in explicit visual-demo mode
(synthetic demo content, never project data):

- `docs/ui/screenshots/timeline-toolbar-no-selection.png`
- `docs/ui/screenshots/timeline-toolbar-video-selected.png`
- `docs/ui/screenshots/timeline-toolbar-audio-selected.png`
- Full-window versions: `m20-editor-1600-*.png`

Measured cyan toggle columns are identical at x = 1207–1234 and 1239–1266 in
all three captures, confirming snapping/linked selection do not shift when the
contextual group appears or changes media kind.

## Remaining differences

- The reference uses active white editing glyphs. CutCraft renders unavailable
  editing glyphs in a low-contrast disabled state.
- Three split-family icons, the media/AI cluster, and several right-side icons
  have unresolved identities; CutCraft uses original equivalent glyphs for
  documented planned categories instead.
- The reference shows a dropdown chevron on one active toggle; CutCraft has no
  toggle menu because no additional behavior exists.
- The reference’s extra AI/cloud/paid tools are intentionally omitted.
- The toolbar is inset by the rounded-panel margin, so its first and last
  controls sit slightly farther from the window edge than the cropped reference.
- The reference includes additional tool slots; CutCraft retains a flexible
  empty center and moves secondary tools into the compact overflow at narrow
  widths.
