# CutCraft

An independent Rust desktop video editor, built natively with [egui](https://github.com/emilk/egui)/eframe and a
renderer-owned wgpu preview — currently in early development, and honest about which parts actually work.

![CutCraft editor showing a real decoded frame on the GPU preview path, with the native timeline, media pool and inspector](assets/screenshots/editor-gpu-preview.png)

*The badge inside the preview reads `GPU PREVIEW` with the decoded clip's asset, dimensions, frame rate and
presentation timestamp. The clips shown are synthetic H.264 test fixtures generated for this project — see
[screenshot provenance](assets/screenshots/README.md).*

---

## What is CutCraft?

CutCraft is a native Windows video editor written from scratch in Rust. The goal is an offline editor whose
project format is stable and inspectable, whose preview path is real GPU rendering rather than a web view,
and whose timing is exact integers instead of floating-point seconds — eventually reaching reliable
short-form editing (one 4K/30 video layer, subtitles and audio) without a browser anywhere in the stack.

It is an independent implementation. It is not a fork, wrapper, or companion of any existing editor.

## What works today

Verified on Windows against the current build — not aspirational:

- **Native desktop shell** in Rust with egui/eframe 0.37: Home, project library, Settings, Editor, and a
  modal Export *settings* form.
- **Native `.cutcraft` projects**, schema v2: exact integer tick timing (120,000 ticks/s), stable
  project/track/clip/asset IDs, atomic staged saves, and in-memory migration of v1 files without rewriting them.
- **Managed project library**: Create Project produces a real managed project that is discovered on Home and
  can be opened, renamed and duplicated.
- **Generation-safe AutoSave**: visible `Saving` / `Saved` / `Unsaved changes` / `Save failed` / `Conflict`
  states, flush on navigation and exit, and refusal to overwrite a file that changed outside the app.
- **Bounded recovery snapshots**: checksum-validated one-slot snapshots that can be restored *as a new
  project* after abnormal termination, without ever silently replacing the primary file, plus an explicit
  *Save Recovery Copy* route for external conflicts.
- **Persistent media assets**: import a local H.264 video and keep it in place (media bytes are never copied
  into the project), see its state, and relink it when it goes offline while preserving Clip/Asset IDs.
- **Native timeline editing subset**: append to the main track, Split at Playhead, Delete, Magnet, Undo/Redo,
  ruler scrubbing, frame navigation, zoom, and an authoritative tick playhead.
- **Real decoded-frame preview on a GPU path**: a renderer-owned single-layer wgpu pipeline that reuses
  eframe's existing device and queue, preserves clip/asset/source/generation/PTS identity, reuses or
  reallocates bounded textures, reports actual presentation timestamps, and falls back to the existing egui
  texture path when GPU preview is unavailable. Colour, stride and identity behaviour is covered by
  device-backed pixel-readback tests.
- **H.264 single-frame decoding** through a locally installed FFmpeg CLI, with hard timeouts on spawned
  processes and bounded, asynchronous probe/decode workers that keep decoding off the UI thread.

## Not implemented yet

These are on the roadmap and are **not** available, so please don't file them as bugs:

- Continuous video playback and a playback clock — the preview shows one frame per requested position.
- Audio output, mixing or A/V synchronisation.
- Export encoding — the Export dialog collects settings only; no encoder exists.
- A multi-layer video compositor, subtitle/text/effect rendering, transitions, or colour management.
- Hardware decoding, and any GPU completion/present timing (those metrics are unobservable in this build).
- Full non-linear-editing parity: nested sequences, linked-media movement, and multi-select ripple tooling.
- Device-loss recovery, and crash protection for edits made before a recovery snapshot was published.

A single decoded 4K frame rendering correctly on the GPU is a correctness result, not a 4K/30 playback claim.

## Requirements

- **Platform:** Windows is the primary development and target platform; everything below was verified on
  Windows. CI is configured to also build on Ubuntu and macOS, but those platforms are not the focus and are
  not claimed as verified here.
- **Rust:** pinned to **1.97.1** by [`rust-toolchain.toml`](rust-toolchain.toml), with `rustfmt` and `clippy`.
- **FFmpeg (optional — needed for real media preview):** `ffmpeg` and `ffprobe` are discovered at runtime from
  `PATH`, or explicitly via `CUTCRAFT_FFMPEG` / `CUTCRAFT_FFPROBE`. **They are not bundled or linked.** Without
  them the app still runs and everything except real frame decoding works.
- **Distribution:** there are no installers or published release binaries yet — you build from source.

## Build and run

```powershell
git clone https://github.com/nguyennhat2002/CutCraft.git
cd CutCraft
cargo run -p capcut-editor
```

Click **Create Project**, then **Import Media** in the Media pool to register a local H.264 file, and
**Append** it to the main track. Click the timeline ruler to scrub: the preview reports the decoded frame's
PTS and whether it was drawn on the GPU path or the egui fallback.

Checks contributors run locally — and what CI is **configured** to run on Ubuntu, Windows and macOS
([`.github/workflows/ci.yml`](.github/workflows/ci.yml)); a workflow file is configuration, not a claim that
every run is green, so this README deliberately carries no status badges:

```powershell
cargo fmt --all -- --check --config-path .config/rustfmt.toml
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace

# strict media gate: fails instead of skipping when FFmpeg or fixtures are absent
$env:CUTCRAFT_REQUIRE_MEDIA = "1"
cargo test -p capcut-media --test media_pipeline -- --nocapture
Remove-Item Env:CUTCRAFT_REQUIRE_MEDIA
```

Useful diagnostics: `CUTCRAFT_MEDIA_LOG=1` (probe/seek/decode timing), `CUTCRAFT_RENDER_LOG=1` (GPU backend,
texture upload and UI tick timing — the log writes are themselves measurable, so tick numbers taken with it
enabled are not production latency), `CUTCRAFT_PROJECTS_DIR=<path>` (isolate the project library), and
`CUTCRAFT_PREVIEW_RENDERER=egui` (force the fallback path).

## Native project format and recovery limits

- `.cutcraft` is CutCraft's own format: schema v2, strict validation, stable IDs, exact tick timing, and
  atomic staged replacement. It is not an industry interchange format and makes no compatibility claims.
- AutoSave writes managed projects automatically and refuses to clobber a file changed outside the app;
  conflicts surface as an explicit choice rather than a silent overwrite.
- Recovery is a **bounded, checksum-validated snapshot**, applied only when you ask, always as a new project.
  There is **no AutoRecovery journal**, no power-loss guarantee, and no recovery for edits made before a
  snapshot was published.
- There is **no OS-level multi-process lock**. CutCraft assumes one writer per project; opening the same
  project in two processes at once is unsupported.
- Media is referenced in place, so moving source files remains your responsibility.

Details in [`docs/TESTING.md`](docs/TESTING.md) and [`docs/MEDIA_IDENTITY.md`](docs/MEDIA_IDENTITY.md).

## Architecture

Native code is split into focused crates with a strict upward-only dependency order — `core` → `engine` →
`project` / `media` → `renderer` → `ui` → the `capcut-editor` binary. UI toolkits never leak into lower
layers, GPU resources are owned only by `capcut-renderer`, and no project or session state holds a GPU handle.

| Crate | Responsibility |
|---|---|
| `crates/core` | shared time, IDs, math, errors |
| `crates/engine` | native project/track/clip/asset state, commands, history, selection |
| `crates/project` | `.cutcraft` schema v2, managed library, AutoSave, recovery snapshots |
| `crates/media` | FFmpeg CLI probe/decode, exact PTS mapping, bounded async workers |
| `crates/renderer` | wgpu preview resources, texture ownership, upload, metrics |
| `crates/ui` | egui screens, panels, interaction, shortcuts |
| `apps/capcut-editor` | composition root and desktop executable |
| `xtask` | build/test/release automation scaffold (not yet a working quality gate) |

Details in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md); the reasoning behind each major choice is in
[`docs/ADR/`](docs/ADR/), notably [ADR-0005](docs/ADR/0005-ffmpeg-cli-preview-backend.md) on the FFmpeg CLI
adapter and [ADR-0008](docs/ADR/0008-wgpu-preview-renderer.md) on the wgpu preview.

## Roadmap

Milestones, acceptance criteria and — importantly — what has *evidence* rather than just code, are tracked in
[`ROADMAP.md`](ROADMAP.md). The next major step is interactive playback (M5): a playback clock, frame
scheduling and A/V sync, which the current single-frame preview is designed to grow into.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the toolchain, the exact checks CI runs, how to report bugs and
propose features, and the rule that keeps this project honest: **a claim needs evidence, and unimplemented
behaviour stays labelled as unimplemented.**

## License

CutCraft is licensed under either of, at your option:

- **MIT license** ([LICENSE-MIT](LICENSE-MIT))
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))

SPDX expression: `MIT OR Apache-2.0`, declared in the workspace `Cargo.toml` and inherited by every crate.
Contributions are accepted on the same terms. Third-party dependencies, the fonts bundled with egui and the
Lucide icon set keep their own licences — see [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) and
[`docs/licenses/`](docs/licenses/).

## Disclaimer

CutCraft is an independent project and is not affiliated with, sponsored by, or endorsed by any other video
editor vendor. All third-party names, trademarks and technologies mentioned or used are the property of their
respective owners, and naming a technology indicates interoperability or dependency only.

This software is provided "as is", without warranty of any kind. Licensing CutCraft's code grants nothing
over third-party assets, codecs, trademarks, or your own media. FFmpeg is a separate project with its own
LGPL/GPL obligations that depend on which build you use and how you distribute it; CutCraft currently invokes
a locally installed FFmpeg CLI and bundles nothing. Keep your own backups — AutoSave and recovery reduce the
chance of losing work but are not a durability guarantee.
