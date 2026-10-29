# CutCraft — Architecture and Dependency Boundaries

> **Current state:** early scaffold with a working **M1 desktop UI shell** (Home, Projects, Settings, Editor, and Export settings dialog). This file combines **EXISTING** repository structure with **PROPOSED** architecture for the first working Windows editor. A planned interface is not an implemented API. Durable design changes require an ADR in `docs/ADR/`.

## 1. Product direction

CutCraft is an independent Rust desktop video editor. The first target is a short-form editing workflow: a single 4K/30 FPS video layer, subtitles and an audio layer, with responsive preview, seek, trim, undo/redo and export. Deliver a working vertical slice before extending multi-track effects, CapCut draft compatibility, collaboration or web support.

Platform priority: **Windows**. Cross-platform is a future goal only where the implementation and tests justify it. Rust is pinned to `1.97.1`, edition `2021` in the current workspace.

## 2. Existing crate graph

The following edges reflect the checked-in Cargo manifests, not idealized future relationships:

```text
L0  capcut-core
     ├── capcut-format (L1)
     ├── capcut-media  (L2)
     └── capcut-engine (L2; ALSO depends on capcut-format)

L3  capcut-renderer → capcut-core, capcut-engine, capcut-media
L4  capcut-ui       → capcut-core, capcut-engine, capcut-renderer, capcut-media
App apps/capcut-editor → capcut-ui, capcut-engine, capcut-renderer, capcut-media
Tool xtask (separate scaffold)
```

| Path | Existing responsibility | Implementation status |
|---|---|---|
| `crates/core` | Shared time, IDs, errors, math | Placeholder modules |
| `crates/capcut-format` | Draft schema/import/export/validation | Placeholder modules; compatibility unverified |
| `crates/engine` | Project model, commands, revision, selection, playback | Placeholder modules |
| `crates/media` | Probe, decoder, encoder, cache, audio resampling | Placeholder modules |
| `crates/renderer` | Context, pipelines, compositor, texture pool | Placeholder modules/shader stubs |
| `crates/ui` | egui screens, navigation/UI state, theme, panels, display-only timeline, export form | M1 Home/Editor/Export shell; editing/media/playback placeholders |
| `apps/capcut-editor` | Executable composition root | Launches the eframe/egui window |
| `xtask` | Automation | Prints scaffold-only message |

The project already contains `.github/workflows/ci.yml`, `docs/CAPCUT_FORMAT.md`, `docs/ADR/README.md`, and Rust tooling configuration. They are not proof that functional editor features exist.

### Boundary rules

1. `core` must not depend on GUI, GPU, media decoder or platform APIs.
2. `media` must not import `egui`, window types or renderer internals.
3. `engine` owns persistent project/timeline logic; UI must not mutate project state independently.
4. `renderer` owns presentation resources, color conversion/composition strategy and GPU handles.
5. `ui` translates user actions into engine commands and displays results/errors. egui/eframe dependencies are confined to `capcut-ui` and the application; UI state (screens, temporary sessions, panel visibility, export form values) is not engine state.
6. The application is the composition root; backend selection and cross-layer wiring happen here or in dedicated integration modules.
7. The current `engine → capcut-format` edge exists. A future format-agnostic engine is a **proposal** requiring an ADR and coordinated Cargo/code changes, not an assumption about today's implementation.

## 3. Proposed vertical-slice system

```text
                  ┌─────────────────────────┐
                  │ Desktop UI (egui/eframe)│
                  │ Media / Preview / Tracks│
                  └───────────┬─────────────┘
                              │ commands + view state
                  ┌───────────▼─────────────┐
                  │ Engine                  │
                  │ Project / Timeline      │
                  │ Commands / Undo-Redo    │
                  │ Playhead / Playback     │
                  └───────┬─────────┬───────┘
                          │         │ frame requests
                          │         ▼
                          │  ┌──────────────────┐
                          │  │ Media            │
                          │  │ Probe / Decoder  │
                          │  │ Owned frames     │
                          │  │ Cache / Audio    │
                          │  └─────────┬────────┘
                          │            │ frame + PTS
                          ▼            ▼
                  ┌─────────────────────────┐
                  │ Renderer                │
                  │ Texture / Compositor    │
                  │ Preview surface        │
                  └─────────────────────────┘
```

This is a **CUTCRAFT DESIGN PROPOSAL**. It is not a recovered internal CapCut implementation and is not functional in the repository today.

## 4. Proposed data contracts

### Project and timeline (engine)

- `Project`: stable project identity, asset catalog, one or more sequences, settings and revision.
- `Sequence`: rational frame rate, dimensions, duration, ordered track collection.
- `Track`: stable identity, kind (video/audio/text), enabled/visibility state, ordered clip references.
- `Clip`: asset reference, timeline `[start,end)` range, source in/out and rate/speed mapping.
- `Command`: validated edit, reversible change or an immutable-state transaction; undo/redo history belongs in engine.
- `Selection`: transient editor state, separate from file persistence unless deliberately saved.

These names describe intended responsibilities, **not current public Rust types**.

### Time domain (core)

Keep separate `TimelineTime`, `MediaPts`, `FrameIndex`, rational `FrameRate` and monotonic playback clock. Decide the concrete canonical time representation in an ADR; use checked `i128` intermediate operations for rescaling. Avoid float seconds as authoritative edit values, including at 30000/1001 FPS.

### Media result (media)

A frame result should carry stream/asset identity, dimensions, format, per-plane strides, color/rotation metadata, source PTS/timebase, and an owned buffer or a clearly documented GPU handle. An asynchronous request carries an ID/generation and cancellation behavior so stale seek results cannot replace newer frames.

### GPU presentation (renderer)

Keep CPU decode buffers, GPU staging buffers, reusable textures and swapchain/surface ownership separate. Define conversion responsibilities (e.g. YUV → RGB, range/transfer) explicitly. The renderer may consume a stable engine snapshot for overlays, but should not own edit commands.

## 5. Playback control and scheduling

**Planned**, not implemented:

```text
UI Play/Pause/Seek
      ↓
PlaybackController state + monotonic clock
      ↓
TimelineTime → source MediaPts mapping
      ↓
Frame request (generation, cancellation)
      ↓
Decoder / bounded cache
      ↓
PTS-aware scheduler → preview renderer
```

The scheduler chooses frames by intended presentation time, not decode completion order. A queue is bounded; old seeks are cancelled/dropped; presentation and decode metrics are distinct. Audio-master versus video-master clock must be decided in an ADR when audio playback is implemented. A frame drop policy for preview must never silently carry over to export.

## 6. File formats and compatibility

The first deliverable should be a native CutCraft project representation that round-trips safely when persistence is implemented. `capcut-format` is a distinct interoperability surface: import/validate/map supported drafts into CutCraft project data, and export explicitly supported fields only. `docs/CAPCUT_FORMAT.md` records the still-unverified compatibility target.

If the engine is made independent from `capcut-format`, record an ADR detailing ownership, dependency changes, migration and tests. Do not represent external draft field names as engine invariants.

## 7. Dependency and backend choices still open

- **UI:** egui/eframe **0.37.0** is pinned for the desktop shell (ADR-0001). The UI renders through eframe's default wgpu backend, but that choice applies to UI painting only; it does not establish the video compositor or preview pipeline. Homepage navigation, temporary sessions, and export form values are in-memory UI state only.
- **Media:** FFmpeg/native platform decoders are candidates, not selected or integrated. Assess licensing, Windows deployment, pixel formats and test fixtures first.
- **GPU:** wgpu is the intended direction, not an existing working renderer. Decide device/surface ownership, texture types and failure handling before code, including whether a future preview can share eframe's device.
- **Audio:** output backend and synchronization master are undecided.
- **Persistence/export:** native project format, autosave and encoder/container profiles are undecided.

An accepted ADR should describe the chosen option, alternatives, consequences, rollback and proof.

## 8. Cross-cutting safety and testing

- All inputs are untrusted; validate sizes and timestamps, cap allocations, prevent arithmetic overflow.
- UI threads do not block on decode, I/O, GPU readback or export.
- Do not lose edits on media failures; background tasks observe stable state.
- Use typed errors, bounded queues, and explicit resource lifetimes.
- Test timeline correctness headlessly. Use synthetic/owned media for decoder integration and image comparisons for rendering.
- Validate on real Windows hardware and keep performance claims tied to a fixture/hardware report.

Read `TESTING.md` and `PERFORMANCE.md` for concrete gates and benchmark methodology.

## 9. Evolution plan

Milestone ordering lives in `../ROADMAP.md`: (M0) baseline → (M1) desktop shell → (M2) core timeline and commands → (M3) real decoder → (M4) GPU preview → (M5) playback/seek → (M6) edits → (M7) subtitles/audio/export → (M8) compatibility/acceleration.

The M1 shell exists and has been launched/resized/closed on Windows with placeholder screens and panels only. It contains no engine commands, project persistence, media access, playback, or encoding. The Home → temporary Editor → Export → Cancel flow and Home/Resume navigation are UI-only and do not save a project.

Update this architecture only when code or an accepted design decision changes. Never mark a proposed box as implemented merely because a new empty source file appeared.
