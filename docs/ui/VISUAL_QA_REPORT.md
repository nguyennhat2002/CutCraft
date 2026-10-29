# Visual QA report — M1.3

Date: 2026-10-30
Platform: Windows 10.0.26300, x64
Build: debug (`cargo build -p capcut-editor`)
Window content sizes exercised: 1600×900, 1080×720, maximized
Reference images: supplied by the user, kept outside the repository

## Method

The interactive desktop session was locked, so QA used two native mechanisms:

1. **Windows UI Automation** to invoke real controls by name: Create Project,
   Export, Cancel, and Home.
2. **`PrintWindow` with `PW_RENDERFULLCONTENT`** to capture the live window into
   PNG without relying on screen compositing.

Captures include the native window frame. Content viewports were:

| Capture | Image size | Content viewport |
|---|---|---|
| 1600×900 set | 1616 × 939 | 1600 × 900 |
| 1080×720 set | 1096 × 759 | 1080 × 720 |
| Maximized | 1888 × 1096 | maximized on 1920×1080 display |

This is qualitative visual comparison against the references. No pixel-diff or
similarity score is claimed.

## Evidence

All images are in `docs/ui/screenshots/`.

| Screen | 1600×900 | 1080×720 | Maximized |
|---|---|---|---|
| Homepage | `home-1600x900.png` | `home-1080x720.png` | — |
| Editor | `editor-1600x900.png` | `editor-1080x720.png` | `editor-maximized.png` |
| Export | `export-1600x900.png` | `export-1080x720.png` | `export-maximized.png` |
| Cancel result | `editor-after-export-cancel.png` | — | — |

Interaction evidence:

- Create Project enters the editor with an in-memory session.
- Export opens the modal.
- Cancel closes the modal and returns to the editor (captured).
- Home returns to the homepage while preserving the session; the banner shows a
  Resume editing chip and the temporary-session notice.
- Export remains disabled; no file, size estimate, or progress is shown.
- The app exited with code 0 after `cargo run -p capcut-editor`.

## Gate results

| Gate | Command | Result |
|---|---|---|
| Formatting | `cargo fmt --all -- --check --config-path .config/rustfmt.toml` | pass (exit 0) |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` | pass (exit 0) |
| Tests | `cargo test --workspace` | pass: 72 UI tests, other crates 0 tests |
| Compile | `cargo check --workspace` | pass (exit 0) |
| Launch | `cargo run -p capcut-editor` | launched; closed; exit code 0 |

## Comparison

### Homepage

Matched:

- Dark teal canvas with a left navigation sidebar and a compact top strip.
- Wide bright-cyan create-project banner with a centered dark plus glyph and
  "Create project", replacing the earlier dark copy/CTA card.
- Content gutter, profile placement, and first-wide feature card row.
- Four creative-suite cards, Projects heading with search and grid/list controls,
  and a centered empty state.

Differences remaining:

- CutCraft branding, feature names, icons, and copy differ by design.
- Sidebar contains a local profile and preview notice instead of account, Pro,
  invite, and sale cards.
- Feature and creative-suite artwork is original abstract geometry, not the
  reference photography.
- The Projects empty state includes a Create Project button; the reference only
  shows copy.
- Native titlebar is visible in captures.

### Editor

Matched:

- 33.5 / 28.5 / 37.6 upper-column proportions.
- Dark content surfaces `#262626`, header strips `#303030`, app bar `#141414`,
  cyan accent, black preview canvas.
- Ten-category strip above the library, Captions selected by default, secondary
  captions navigation, language fields, and pinned Generate footer.
- "Player / Timeline 01" header, 9:16 preview canvas, timecode/transport footer.
- Six-tab inspector with Basic/Templates/Bubble/Effects, Apply to all, font,
  size, B/U/I, case, color, spacing, alignment, and 8 preset tiles.
- Six-track timeline with adaptive row heights, track headers, 10-second ruler
  labels, minor ticks, playhead, horizontal scrolling, zoom, and demo strips.

Differences remaining:

- Top chrome is one 40 px row with brand/project/Menu/Export, while the reference
  uses a thinner app bar plus category header row.
- Inspector tabs are Captions / Text / Animation / Video / Audio / Basic rather
  than the reference's assistant-oriented tab set.
- Preview is an honest empty canvas; no decoded frame exists.
- Timeline demo strips replace real media clips and are labelled `DEMO`.
- At 1080×720 the inspector and library scroll; the preset tile rows and lower
  caption controls are not fully visible without scrolling.
- Default system fonts are used; the reference uses different typographic
  families.
- Category and many painted controls do not yet expose full accessibility roles.

### Export

Matched:

- Centered dark modal with a 9:16 cover column and grouped right-hand settings.
- Groups for Export timeline, Name, Export to, Video settings, and Audio.
- Footer status with Cancel and Export actions.
- Cancel and close work; Export is visibly disabled.

Differences remaining:

- Dialog is 760×620 versus the reference's 721×663; the reference cover contains
  an image, while CutCraft shows an explicit placeholder.
- Audio codec/sample-rate rows are labelled `planned`; no encoder settings are
  functional.
- The reference's Export button appears enabled; CutCraft intentionally keeps it
  disabled to avoid implying encoding exists.

## Iteration log

### Pass 1 — initial reconstruction

Findings:

- Panel content was too light (`#303030`) compared with the reference's
  `#262626`; headers and content lacked separation.
- At 1080×720 the six-track timeline overflowed and clipped the A3 row.
- The inspector disclosure badge touched its text and the preset tiles were cut
  mid-row at 900 px height.
- The export Export button used a bright fill and looked enabled.
- The homepage banner was dark and copy-heavy instead of the bright reference
  create surface.
- At 1080×720 the homepage sidebar's bottom cluster overlapped management rows.

Fixes:

- Split tokens into `#141414` app bar, `#303030` headers, `#262626` panels, and
  `#343437` controls; added header fills and matched secondary-nav surfaces.
- Added adaptive timeline track heights (20–58 px) so all six tracks fit at 720.
- Added spacing to the inspector notice; reduced preset tile height.
- Styled disabled Export with an accent-dim fill and disabled text.
- Rebuilt the banner as a whole-surface bright-cyan create button with a centered
  plus glyph and session disclosure.
- Replaced the sidebar's bottom-up layout with top-down flow and a computed
  spacer; moved the profile under the brand.

### Pass 2 — second capture

Findings:

- Palette and track fit were corrected.
- The new bright banner matched the reference hierarchy; the Resume chip and
  session disclosure were visible only with an active session.
- Remaining cut-off content at 1080 is inside scrollable panels.

Actions:

- Re-captured all screens, confirmed Cancel returns to the editor, and confirmed
  Home navigation with a session.
- Confirmed `cargo run` exit code 0.

## Honest limitations

- The desktop session was locked; visual verification used `PrintWindow`, not
  direct observation of animation or hover states.
- Screenshots include the native window frame.
- No media pipeline exists, so preview, timeline clip content, export output, and
  file-size estimates cannot be verified — and are not fabricated.
- Only qualitative comparison was performed; no numerical similarity score.
- 1080×720 content is functional but dense: library and inspector content scroll.
- The export dialog's 9:16 cover placeholder is not a rendered frame.

## Follow-up polish

1. Add AccessKit roles/labels for painted category tabs and custom controls.
2. Make inspector scroll affordance more visible at 900 px height.
3. Tighten editor top chrome to a thinner app bar plus header row if the project
   identity can move elsewhere.
4. Replace homepage abstract artwork with original CutCraft illustrations when
   asset production is authorized.
5. Populate the timeline from real project state once engine commands exist.
6. Revisit export dialog proportions against a higher-resolution reference crop.
