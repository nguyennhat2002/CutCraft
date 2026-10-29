# UI reconstruction plan — M1.3

## Objective

Rebuild the Homepage, Editor, and Export dialog of the CutCraft desktop shell so
their composition, density, color hierarchy, and control placement follow the
supplied reference screenshots as closely as the current UI-only milestone
allows, without copying proprietary artwork or implying missing engine features.

## Constraints

- Rust 1.97.1, egui/eframe 0.37.0, existing workspace boundaries.
- No video decoding, playback, timeline editing, persistence, or encoding.
- Every unimplemented action is disabled or explicitly labelled; no fabricated
  export success, file size, progress, or media preview.
- No commit or push without user authorization.
- Preserve unrelated engine/media/renderer code.
- Visual verification is mandatory and must use actual application captures.

## Workstreams

The milestone called for Reference Analyst, UI Architect, Editor Engineer,
Homepage/Export Engineer, and Visual QA roles. This environment ran as one
implementation stream; the responsibilities were combined and executed in that
order. No independent subagent review was performed.

1. **Reference Analyst** — measured screenshot dimensions, region geometry,
   sampled colors, typography, control inventory, and interaction states.
   Output: `REFERENCE_MEASUREMENTS.md`.
2. **UI Architect** — mapped measurements to `config.rs`, `theme.rs`, workspace
   layout, session state, and widget boundaries; identified deliberate
   differences required by missing engine features. Output: this plan.
3. **Editor Engineer** — reworked the top chrome, tool categories, Captions
   library, preview header/stage/transport, six-tab inspector with caption
   controls, adaptive timeline tracks, ruler, and scrolling.
4. **Homepage/Export Engineer** — rebuilt the teal homepage shell, bright
   create-project banner, feature and creative-suite rows, Projects empty state,
   and the export modal cover/settings/footer.
5. **Visual QA** — launched the desktop build, drove Create/Export/Cancel/Home
   through Windows UI Automation, captured the window with `PrintWindow`, compared
   against references, fixed the highest-impact differences, and re-captured.
   Output: `VISUAL_QA_REPORT.md` and `screenshots/`.

## Implementation phases

### 1. Tokens and geometry

- Reference palette: app bar `#141414`, column headers `#303030`, content panels
  `#262626`, controls `#343437`, accent `#00C1CD`, canvas black, home teal surfaces.
- Editor defaults: 1600×900, 33.5% / 28.5% / 37.6% upper columns, 40 px app bar,
  38 px column headers, six tracks, 10 px/s default timeline scale.
- Caption demo state added to `EditorSession`: font size, B/U/I, case, color,
  spacing, alignment, preset index, and apply-to-all. This state is transient and
  never touches a project.

### 2. Editor chrome

- Single compact top bar with Home, brand, project identity, "Not saved", Menu,
  undo/redo placeholders, and Export.
- Ten tool categories in the left panel: Media, Audio, Text, Stickers, Effects,
  Transitions, Captions, Filters, Adjustment, Templates.
- Captions library with secondary navigation, language/bilingual fields, filler
  words, pinned delete/generate footer.
- Preview with 38 px "Player / Timeline 01" header, 9:16 default canvas,
  16:9/9:16 switch, static timecode, disabled transport, and zoom placeholder.
- Inspector with six tabs and a caption Basic panel: font, size, pattern, case,
  color, character/line spacing, alignment, and original "Aa" preset tiles. Other
  tabs use clearly disabled property controls.
- Timeline with six adaptive tracks, track headers, 10-second major labels,
  second-level minor ticks, playhead, horizontal scrolling, zoom, and three
  clearly labelled demo strips.

### 3. Homepage and Export

- Homepage: dark teal canvas, local profile, navigation and management groups,
  bright cyan create-project banner, featured-tool cards with the first wide/teal,
  creative-suite cards, and a Projects section with search/view controls and an
  empty state.
- Export: centered modal, left 9:16 cover placeholder with disabled Edit cover,
  right grouped form (Export timeline, Name, Export to, Video settings, Audio),
  validation messaging, disabled Export, working Cancel/close.

### 4. Verification loop

1. Build and launch `target/debug/capcut-editor.exe`.
2. Capture Home, Editor, Export, and post-Cancel Editor at 1600×900 content size.
3. Resize to 1080×720 content size and repeat.
4. Maximize and capture Editor and Export.
5. Compare against references; log differences; fix the highest-impact issues;
   repeat until remaining differences are deliberate or documented.

## File map

| Area | Files |
|---|---|
| Tokens | `crates/ui/src/theme.rs`, `config.rs` |
| Session demo state | `crates/ui/src/editor_state.rs` |
| Workspace split and timeline height | `crates/ui/src/workspace_layout.rs`, `widgets/editor.rs` |
| Top bar | `widgets/toolbar.rs`, `widgets/menu_bar.rs` |
| Library and captions | `widgets/media_pool/mod.rs` |
| Preview | `widgets/preview/mod.rs` |
| Inspector | `widgets/inspector/mod.rs` |
| Timeline | `widgets/timeline/{mod,scale,track_view}.rs` |
| Homepage | `widgets/home.rs` |
| Export | `widgets/export_dialog.rs`, `export_settings.rs` |
| Measurement and QA docs | `docs/ui/*` |

## Non-goals

- No media import, decode, render, playback, edit commands, persistence, export
  encoding, account system, or template marketplace.
- No copied CapCut code, images, icons, fonts, or proprietary layouts beyond the
  measured proportions and color hierarchy.
- No new dependencies and no changes to lower-level crates.

## Acceptance checklist

- [x] Reference measurements captured and documented.
- [x] Homepage, Editor, and Export rebuilt against the reference hierarchy.
- [x] Ten tool categories and six-track timeline with zoom/scroll.
- [x] Six-tab inspector with caption controls and preset tiles.
- [x] Export groups, cover placeholder, footer, working Cancel, disabled Export.
- [x] No fabricated size/progress/export result.
- [x] `cargo fmt`, `clippy -D warnings`, `cargo test`, `cargo check` pass.
- [x] Desktop app launched and screenshots captured at all required sizes.
- [x] Remaining differences documented honestly.
- [ ] Commit/push — intentionally not performed; requires user authorization.
