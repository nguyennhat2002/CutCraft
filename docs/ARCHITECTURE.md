# CutCraft — Architecture and Dependency Boundaries

> **Current state:** early-stage native editor with a working desktop shell, native `.cutcraft` schema v2 persistence (M3-A/M3-D), an authoritative UI-session tick playhead with scrubbing and Split-at-Playhead (M3-B), a bounded H.264 probe/decode/clip Preview pipeline (M3-C/M3-D), managed project AutoSave with persistent media assets (M3-D), bounded recovery snapshots plus external-conflict preservation (M3-D.1), and a renderer-owned single-layer wgpu Preview with egui fallback (M4-A). This file combines **EXISTING** repository structure with **PROPOSED** architecture for the first working Windows editor. A planned interface is not an implemented API. Durable design changes require an ADR in `docs/ADR/`.

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

L2  capcut-project → capcut-core, capcut-engine
L3  capcut-renderer → capcut-core, capcut-engine, capcut-media, wgpu 30.0.1
L4  capcut-ui       → capcut-core, capcut-engine, capcut-project, capcut-renderer, capcut-media
App apps/capcut-editor → capcut-ui, capcut-engine, capcut-renderer, capcut-media
Tool xtask (separate scaffold)
```

| Path | Existing responsibility | Implementation status |
|---|---|---|
| `crates/core` | Shared time, IDs, errors, math | Exact 120k tick time and rational frame-rate conversion implemented; remaining helpers placeholders |
| `crates/capcut-format` | Draft schema/import/export/validation | Placeholder modules; compatibility unverified |
| `crates/engine` | Project model, commands, revision, selection, playback | Native project/timeline model with a bounded media asset registry and clip references; selection; transactional Delete/TrimEnd/Split/NormalizeMainTrack/AddMediaAsset/RelinkMediaAsset/AppendMediaClip/RenameProject history; engine-owned Magnet and validated restore APIs; playhead is UI session state; playback placeholders |
| `crates/project` | Native `.cutcraft` schema, managed library, AutoSave, recovery, document identity, bounded validation, safe save/open | Strict schema v2 with in-memory v1 migration, staged atomic save, external-change detection, managed root/scan/rename/duplicate, generation-safe scheduler and one-worker persistence worker (ADR-0006), checksum-validated one-slot recovery snapshots and worker (ADR-0007), abandoned staging cleanup |
| `crates/media` | Probe, decoder, encoder, cache, audio resampling | H.264 FFprobe/FFmpeg CLI adapter, exact metadata, rational source-to-PTS conversion, one-worker latest-request frame decoder with owned RGBA frames and bounded errors, bounded import/validation probe queue; encoder/cache/audio remain placeholders |
| `crates/renderer` | GPU Preview resources, pipelines, texture ownership, compositor contract | Renderer-owned single-layer `PreviewRenderer` (wgpu 30.0.1, target-color-space texture, reusable texture/sampler/pipeline, bounded validation/metrics); full compositor, text/effects passes remain placeholders |
| `crates/ui` | egui screens, navigation/UI state, design tokens, nine-category library, tabbed inspector, monitor/timeline UI, export form | Native timeline projection, Magnet/Delete/Undo/Redo, managed project library Home, AutoSave status/navigation flush dialogs, recovery detection/restore and conflict-copy dialogs, Media Pool import/relink/append, authoritative tick playhead and scrubbing, Split-at-Playhead, clip-aware asynchronous real frame Preview with a reused egui texture; playback/compositing placeholders |
| `apps/capcut-editor` | Executable composition root | Launches the eframe/egui window |
| `xtask` | Automation | Prints scaffold-only message |

The project already contains `.github/workflows/ci.yml`, `docs/ADR/README.md`, and Rust tooling configuration. They are not proof that functional editor features exist.

### Boundary rules

1. `core` must not depend on GUI, GPU, media decoder or platform APIs.
2. `media` must not import `egui`, window types or renderer internals.
3. `engine` owns persistent project/timeline logic; UI must not mutate project state independently.
4. `renderer` owns presentation resources, color conversion/composition strategy and GPU handles.
5. `ui` translates user actions into engine commands and displays results/errors. egui/eframe dependencies are confined to `capcut-ui` and the application; UI state (screens, temporary sessions, panel visibility, export form values) is not engine state.
6. The application is the composition root; backend selection and cross-layer wiring happen here or in dedicated integration modules.
7. `project` owns the native `.cutcraft` schema, serde and file IO; `engine` must not import serde or filesystem APIs. `capcut-format` remains exclusively the CapCut draft compatibility path.
8. The current `engine → capcut-format` edge exists. A future format-agnostic engine is a **proposal** requiring an ADR and coordinated Cargo/code changes, not an assumption about today's implementation.

### Actual vs planned (2026-10-31)

| Area | Actual in this tree | Planned / NOT implemented |
|---|---|---|
| Project model | `capcut-engine` owns typed project/track/clip/media-asset state, exact ticks, allocators, revision, selection and snapshot history | Multi-sequence/arbitrary composition model; full insert/move/trim-start/linked/nested edits |
| Playhead | UI-session `TimelineTicks` in `EditorSession`; not persisted and never dirties the project (ADR-0004) | Engine-owned playback clock; playback controller; play/pause scheduler |
| History | In-memory snapshot transactions, 8 MiB/100 entries; IDs remain monotonic across Undo/Redo; not persisted | Persistent undo history; collaborative branching |
| Persistence | `capcut-project` strict schema v2, staged atomic save, SHA-256 external-change detection, managed library, generation-safe AutoSave and one-slot validated recovery (ADR-0003/0006/0007) | AutoRecovery journal; OS-level multi-process lock; cross-platform parent-directory flush guarantees; power-loss guarantees |
| Media | Runtime FFmpeg CLI probe/decode, one bounded decoder worker, bounded probe queue, process timeouts on the direct child, owned RGBA frames, PTS selection | FFmpeg library/Media Foundation backend; hardware decode; playback decode scheduling |
| Preview | Renderer-owned single-layer wgpu pipeline reusing eframe device/queue, with egui texture fallback; clip-aware active-source resolution; no compositor | Multi-layer GPU compositor; subtitles/effects compositing; playback clock; device-loss recovery |
| CapCut | `capcut-format` read-only reader and verified filesystem no-op copy path; separate research ledger | CapCut draft writer; CapCut-to-native conversion; verified reopened-edited round trip |
| Renderer (GPU) | **Implemented, limited scope:** `PreviewRenderer` owns one texture, sampler, bind group and pipeline on eframe's existing device/queue, uploads RGBA8 frames, and draws through an egui-wgpu callback; device-backed tests prove colour, stride and lifetime behaviour | Multi-layer GPU compositor, text/effect/subtitle passes, YUV shaders, HDR/colour management, device-loss recovery, GPU completion/present timing |
| Audio / export | Not implemented; the crates contain no audio or encoder pipeline | Audio output and clock, export encoder, container muxing |

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

These names describe responsibilities; the current engine implements the subset listed in the table above. Planned sequence/composition types must not be assumed to exist.

### Time domain (core)

Keep separate `TimelineTicks`, `SourceTicks`, decoder PTS, frame index, rational `FrameRate` and monotonic playback clock. ADR-0002 fixes the native timeline at 120,000 ticks/second; use checked `i128` intermediates for rescaling. Float seconds remain display/pointer-only, including at 30000/1001 FPS.

### Media result (media)

A frame result should carry stream/asset identity, dimensions, format, per-plane strides, color/rotation metadata, source PTS/timebase, and an owned buffer or a clearly documented GPU handle. An asynchronous request carries an ID/generation and cancellation behavior so stale seek results cannot replace newer frames.

M3-C/M3-D implement this contract: `MediaProbe` keeps rational timebase/start/rotation/rate hints; `DecodedFrame` owns normalized RGBA8 bytes plus actual PTS, source/generation/clip/asset tags, dimensions, and requested target; `PreviewEngine` has one bounded latest-request worker; `ProbeQueue` bounds import/validation probes. FFmpeg process execution has bounded output plus a hard timeout, and the adapter normalizes formats/applies rotation before the UI, so no borrowed decoder buffer reaches egui.

### GPU presentation (renderer)

Keep CPU decode buffers, GPU staging buffers, reusable textures and swapchain/surface ownership separate. Define conversion responsibilities (e.g. YUV → RGB, range/transfer) explicitly. The renderer may consume a stable engine snapshot for overlays, but should not own edit commands.

## 5. Playback control and scheduling

**Planned**; M4-A provides single-frame GPU presentation but no play/pause clock, frame scheduler, frame cache, or audio master.

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

The native `.cutcraft` representation now round-trips exact project content through `capcut-project` (see ADR-0003, ADR-0006 and ADR-0007). Schema v2 adds a persistent media asset registry, clip `media_asset_id` references, project identity/timestamps, and a save generation. v1 files are migrated in memory and are not rewritten merely by opening. Third-party project-format interoperability is not implemented and is not a native design goal.

If the engine is made independent from `capcut-format`, record an ADR detailing ownership, dependency changes, migration and tests. Do not represent external draft field names as engine invariants.

## 7. Dependency and backend choices still open

- **UI:** egui/eframe **0.37.0** is pinned for the desktop shell (ADR-0001). The UI renders through eframe's default wgpu backend, but that choice applies to UI painting only; it does not establish the video compositor or preview pipeline. Homepage navigation, temporary sessions, and export form values are in-memory UI state only.
- **Media:** M3-C selects a runtime-discovered local FFmpeg CLI adapter for H.264 probing and single-frame decode (ADR-0005). FFmpeg is not bundled or linked; `CUTCRAFT_FFMPEG`/`CUTCRAFT_FFPROBE` or `PATH` provides the tools. FFmpeg libraries, Media Foundation, hardware decode, audio, and licensed distribution remain future decisions. The repository's development machine uses a GPL FFmpeg build; no redistribution claim is made.
- **GPU:** ADR-0008 reuses eframe's existing wgpu device/queue for a renderer-owned single-layer Preview pipeline. Texture ownership, upload, fallback, and metrics are implemented; a full compositor, device-loss recovery, GPU completion timing, and future layer passes remain open. Correctness is checked in two tiers: `crates/renderer/tests/preview_gpu.rs` renders real fixtures on a device and reads the presented pixels back (skipped, not passed, when no GPU is available), while `capcut-renderer` unit tests cover the CPU-side validation and identity rules that do not need a device.
- **Audio:** output backend and synchronization master are undecided.
- **Persistence/AutoSave/Recovery:** `.cutcraft` schema v2, the managed project library, deterministic locator policy and generation-safe AutoSave are decided in ADR-0006. Bounded one-slot recovery snapshots and conflict-copy preservation are decided in ADR-0007. There is no AutoRecovery journal; AutoSave is not crash-proof. OS-level multi-process locking, Windows parent-directory flush and full-volume guarantees remain unresolved; export/encoder/container profiles remain undecided.

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

Milestone ordering lives in `../ROADMAP.md`: (M0) baseline → (M1) desktop shell → (M2) core timeline and commands → (M3) real decoder, managed persistence/AutoSave, recovery → (M4) GPU preview → (M5) playback/seek → (M6) edits → (M7) subtitles/audio/export → (M8) compatibility/acceleration.

The M1 shell exists and has been launched/resized/closed on Windows. The M1.2 visual pass adds a neutral charcoal/cyan design system, a 33/29/38 resizable workspace, an expanded timeline, nine category shells, a tabbed inspector, and a grouped export form.

A native M2.2 slice exists: exact tick time, rational frame rates, native project/track/clip IDs, transactional DeleteClip/TrimClipEnd/SplitClip/NormalizeMainTrack commands, snapshot undo/redo, and engine-owned Main Track Magnet. M3-A adds `capcut-project` with a versioned `.cutcraft` JSON format, validated engine restore APIs, atomic staged save with external-change detection, exact Magnet-state persistence, and a historical New/Open/Save/Save As UI (manual Save is superseded by M3-D). M3-B adds a session-only `TimelineTicks` playhead, checked tick↔pointer mapping, ruler click/drag scrubbing, rational-FPS frame navigation, real Split-at-Playhead, and preview timecode. M3-C adds real H.264 probing and PTS-based frame decode behind a bounded asynchronous worker and real frames painted in the existing Preview. M3-D replaces the session-only binding with a persistent media asset registry (schema v2), a managed project library, generation-safe AutoSave, Media Pool import/append/relink, and clip-aware Preview. M3-D.1 adds checksum-validated one-slot recovery snapshots, explicit restore as a new managed project, external-conflict recovery copies, FFmpeg process timeouts, abandoned-staging cleanup, and worker panic containment. Media bytes are never copied into the project. History, playhead, playback/audio, compositing, linked-media movement, nested editing, GPU/hardware decoding, and export encoding remain unimplemented. CapCut draft folders remain a separate read-only pathway and are never AutoSaved by the native writer.

Update this architecture only when code or an accepted design decision changes. Never mark a proposed box as implemented merely because a new empty source file appeared.
