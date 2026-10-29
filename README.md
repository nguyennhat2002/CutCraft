# CutCraft

An independent Rust desktop video editor in early development.

Current state: the **M1.2 desktop application shell** (egui/eframe 0.37) provides:

- a charcoal/cyan homepage with a Create Project banner, feature cards,
  Recent Projects empty state, search/view placeholders, and honest temporary-session messaging;
- a resizable editor workspace (approximately 33/29/38 library/preview/inspector),
  nine tool categories, a tabbed property Inspector, a 16:9/9:16 monitor, and a
  four-track timeline with zoom, track controls, and horizontal scrolling;
- a modal export form with output file/folder, format, codec, resolution, frame
  rate, color space, bitrate, audio, and validation.

Media import, decoding, preview rendering, playback, timeline editing, project
persistence, CapCut draft compatibility, and export encoding are **not
implemented**. Their controls are visibly disabled; no project is saved to disk.

## Workspace

- `apps/capcut-editor`: native desktop entry point
- `crates/core`: shared time, IDs, errors, and math
- `crates/capcut-format`: CapCut draft schema, import, export, and validation
- `crates/engine`: project model, commands, history, selection, and playback
- `crates/media`: decoding, encoding, probing, frame cache, and audio resampling
- `crates/renderer`: GPU context, pipelines, compositor, and shaders
- `crates/ui`: egui screens, theme, navigation state, and panels
- `xtask`: project build, test, and packaging automation (scaffold)

## Run

```powershell
cargo run -p capcut-editor
```

## Verify

```powershell
cargo fmt --all -- --check --config-path .config/rustfmt.toml
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace
```

See `ROADMAP.md`, `docs/ARCHITECTURE.md`, `docs/TESTING.md`, and
`docs/ADR/0001-desktop-ui-shell.md` for the verified scope and design decisions.
