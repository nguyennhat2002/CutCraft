# CutCraft

An independent Rust desktop video editor in early development.

Current state: a native editor slice with desktop shell, `.cutcraft` persistence (M3-A), authoritative playhead (M3-B), a real clip-aware H.264 Preview (M3-C), managed projects with persistent media assets and AutoSave (M3-D), bounded recovery plus external-conflict preservation (M3-D.1), and a renderer-owned single-layer wgpu Preview with egui fallback (M4-A) provides:

- a charcoal/cyan homepage with real Recent Projects (open/rename/duplicate),
  Create Project, Open CutCraft Project, and separate Open CapCut Draft actions;
- a resizable editor workspace (approximately 33/29/38 library/preview/inspector),
  nine tool categories, a tabbed property Inspector, a 16:9/9:16 monitor, and a
  native timeline with zoom, track controls, Magnet, Delete, Undo/Redo,
  authoritative tick playhead, ruler scrubbing, frame navigation and
  Split-at-Playhead;
- managed native projects created automatically in a CutCraft project library,
  discovered on Home, and persisted without manual Save/Save As;
- generation-safe AutoSave with visible Saving/Saved/Unsaved changes/Save failed/
  Conflict states, nonblocking navigation/exit flush, and no silent overwrite of
  a file changed outside the application;
- checksum-validated one-slot recovery snapshots that can be restored as a new
  managed project after forced termination, without overwriting the primary,
  plus an explicit Save Recovery Copy route for external conflicts;
- a renderer-owned wgpu Preview pipeline that reuses eframe's existing device
  and queue, reuses/reallocates bounded textures, preserves clip/PTS identity,
  and falls back to the existing egui texture path when GPU preview is unavailable;
- a real Media Pool: import a local H.264 video as a persistent project asset,
  pick its card, append it to the main track, and relink offline/changed media
  while preserving Clip/Asset IDs;
- `.cutcraft` schema v2 with stable media asset IDs, clip references, exact
  integer tick timing, project identity/timestamps/save generation, atomic
  staged replacement, and in-memory v1 migration that does not rewrite v1 files
  on open;
- a clip-aware Preview that resolves the active Timeline clip through its asset,
  decodes asynchronously, preserves media aspect/orientation, shows actual
  presentation timestamps and Clip/Asset IDs, rejects stale results, and
  reports gaps, missing/changed media, and multiple visual layers explicitly;
- a modal export form with output file/folder, format, codec, resolution, frame
  rate, color space, bitrate, audio, and validation (encoding not implemented).

Audio, continuous video playback, GPU compositing, linked-media movement, nested editing,
hardware decoding, AutoRecovery journals, CapCut draft compatibility, and
export encoding are **not implemented**. M4-A is single-frame GPU presentation only; GPU completion/present timing is not observed. Recovery is a bounded snapshot, not a
journal and not a power-loss guarantee. There is no OS-level multi-process
project lock; CutCraft assumes one writer per project. Playhead scrubbing is timeline editing
state, not video playback. Import, relink, duplicate, and AutoSave reference
media in place and never copy media bytes into the project. Real Preview
requires a locally installed FFmpeg CLI (discovered through
`CUTCRAFT_FFMPEG`/`CUTCRAFT_FFPROBE` or `PATH`); no FFmpeg binary is bundled or
linked, and distribution licensing remains an explicit future decision
(ADR-0005). CapCut draft folders remain a separate read-only path and are never
written by the native AutoSave path.

## Workspace

- `apps/capcut-editor`: native desktop entry point
- `crates/core`: shared time, IDs, errors, and math
- `crates/capcut-format`: CapCut draft schema, import, export, and validation
- `crates/engine`: native project model, commands, history, selection, Magnet, and playback placeholders
- `crates/project`: strict `.cutcraft` schema v2, managed project library, generation-safe AutoSave, bounded recovery snapshots, and safe persistence
- `crates/media`: FFmpeg CLI probing/frame decoding, exact PTS mapping, bounded async engine (encoding/cache/audio still placeholders)
- `crates/renderer`: renderer-owned wgpu Preview pipeline, texture reuse, bounded validation, and metrics (full compositor still planned)
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

See `ROADMAP.md`, `docs/ARCHITECTURE.md`, `docs/TESTING.md`, `docs/MEDIA_IDENTITY.md`, and
`docs/ADR/0001-desktop-ui-shell.md`, `docs/ADR/0002-native-timeline-timebase.md`,
`docs/ADR/0003-native-project-persistence.md`, `docs/ADR/0004-authoritative-playhead.md`,
`docs/ADR/0005-ffmpeg-cli-preview-backend.md`,
`docs/ADR/0006-managed-autosave-media-assets.md`,
`docs/ADR/0007-native-recovery-conflict-copy.md`, and
`docs/ADR/0008-wgpu-preview-renderer.md`
for the verified scope and design decisions. Set `CUTCRAFT_MEDIA_LOG=1` to emit
probe/seek/decode/timing lines to stderr. Set `CUTCRAFT_PROJECTS_DIR` to isolate
the managed project library for QA or tests. Set `CUTCRAFT_REQUIRE_MEDIA=1` to
make the media integration gate fail instead of skipping when FFmpeg is absent.
Set `CUTCRAFT_PREVIEW_RENDERER=egui` to force the safe Preview fallback for QA;
`CUTCRAFT_RENDER_LOG=1` emits renderer upload/backend lines.
