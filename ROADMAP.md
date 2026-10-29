# CutCraft Roadmap

> **Project stage:** scaffold / pre-MVP. **Platform priority:** Windows desktop. **Primary workflow:** one 4K/30 FPS video clip, subtitle text, and audio on a short TikTok-style timeline.
>
> Status is evidence-based. A directory, comment, trait declaration, or placeholder is **not** an implemented feature. Update status only after meeting the milestone's acceptance criteria.

## 1. Outcome and non-goals

**North star:** an independent, robust Rust video editor capable of loading a real project, editing a timeline, previewing accurately, and exporting a correct result. Start with one video track and subtitle/audio tracks; layer complexity comes later.

**Near-term non-goals:** exact CapCut UI cloning, full CapCut draft compatibility, custom codecs from scratch, multi-camera editing, effects marketplace, online collaboration, AI functions, and immediate 4K/30 FPS parity on every machine.

### Current verified-in-repository facts

- [x] Cargo workspace defines `apps/capcut-editor`, six library crates, and `xtask`.
- [x] Rust toolchain declared as `1.97.1`; edition `2021`.
- [x] Scaffold module boundaries and project documents exist.
- [x] GitHub Actions workflow for format, Clippy, and tests exists **as configuration**.
- [ ] A passing CI run has been verified for this roadmap revision. Local Windows format, Clippy, test, and check gates have passed (2026-10-29).
- [x] A graphical desktop application exists with Home, Projects, Settings, a temporary Editor shell, and an Export settings dialog; it was launched, resized, and closed normally on Windows (2026-10-29).
- [ ] A real clip can be imported, decoded, or displayed.

`docs/ARCHITECTURE.md` distinguishes existing code from planned structures. Do not convert roadmap items to `[x]` until evidence exists.

## 2. Prioritized implementation milestones

### M0 — Establish a reliable baseline

**Goal:** ensure the empty workspace builds cleanly and the agent workflow has guardrails.

- [x] Run and record `cargo check --workspace`, format, Clippy, and tests on Windows (2026-10-29).
- [ ] Add first unit tests for `capcut-core` and a smoke test for the application executable.
- [ ] Agree on timebase/PTS representation in an ADR before implementing clip math.
- [ ] Agree on media decoding integration and license/deployment plan in an ADR.
- [ ] Decide whether to keep or later remove `capcut-engine → capcut-format` coupling; do not change blindly.

**Acceptance:** all documented checks pass or remaining platform blockers are explicitly captured; no feature is misrepresented as implemented.

### M1 — Launchable desktop shell

**Goal:** a native CutCraft window that stays responsive.

- [x] Select/pin `egui`/`eframe` **0.37.0** dependencies consistent with the pinned Rust 1.97.1 toolchain (ADR-0001).
- [x] Launch a window on Windows via `cargo run -p capcut-editor`.
- [x] Implement initial Media Pool, Preview, Timeline, Inspector, and status/error areas.
- [x] Implement split-panel layout and reasonable resizing/minimum dimensions (960×600 client minimum).
- [x] Add visible error reporting (status-bar notice area), a clean close path, and a headless/UI smoke-test procedure.
- [x] Implement Home, Projects, and Settings navigation with a clearly temporary in-memory editor session.
- [x] Implement the Editor shell: project bar, tabbed library, 16:9/9:16 preview, inspector sections, and timeline toolbar.
- [x] Implement the modal Export settings form (filename, folder, resolution, frame rate, H.264/MP4, bitrate preset, audio) with validation; Export remains disabled.
- [x] Verify Home → Create Project → Editor, Editor → Home → Resume, Editor → Export → Cancel, and portrait preview selection on Windows.

**M1.2 visual-fidelity iteration:**

- [x] Rebalance the editor workspace to approximately 33% library / 29% preview / 38% inspector at 1600×900, with draggable splitters.
- [x] Increase the timeline to roughly 40–45% of the window height, with four track rows, track-control icons, ruler hierarchy, horizontal scrolling, and working zoom controls.
- [x] Add nine tool categories with category-specific library shells; keep unimplemented actions visibly disabled.
- [x] Add a tabbed, control-dense Inspector (Video, Audio, Text, Animation, Basic) using reusable disabled controls.
- [x] Redesign the homepage around a teal/cyan Create Project banner, feature cards, and a project section with search/view placeholders.
- [x] Redesign the Export modal with grouped file/format/video/audio settings, format/codec selectors, color-space display, and disabled Export.
- [x] Verify responsive rendering and navigation at 1600×900 and 1080×720 on Windows.
- [ ] Compare captures directly with the four supplied reference screenshots. **BLOCKED:** the reference images were not available in the workspace or session.

**Acceptance:** user can launch, resize, and close the app; panels remain usable; no blocking media operations or misleading playback controls.

**M1 evidence (2026-10-29, Windows):** Home is the startup screen with no saved projects. Create Project enters a temporary Editor session; File → Back to Home and Resume editing retain session UI state. At 1600×900 the live editor rendered the target three-column proportions, a substantially expanded timeline, nine category tabs, four empty tracks, the tabbed inspector, and the redesigned Export modal; the same paths were checked at 1080×720. The preview switched between 16:9 and 9:16 (1080×1920). Cancel returned from Export to the Editor, and both window close and `cargo run -p capcut-editor` shutdown returned exit code 0. Automated `capcut-ui` tests (70 passing) cover navigation, temporary-session preservation, export validation/persistence, workspace geometry, aspect fitting, zero-time playhead, headless screen/modal rendering, and idle repaint behavior. The reference screenshots were unavailable, so reference similarity was not measured and no percentage-match claim is made. Media import, decoding, preview rendering, playback, timeline editing, persistence, and export encoding remain unimplemented.

### M2 — Native project and timeline model

**Goal:** independent project state and deterministic edit math.

- [ ] Define typed project, asset references, sequence, tracks, clips, and duration in `capcut-engine`.
- [ ] Implement rational FPS and exact timeline/media conversions in `capcut-core`.
- [ ] Add track insertion, clip insertion, selection, and playhead position.
- [ ] Implement validated editing commands and undo/redo before hooking up editable widgets.
- [ ] Unit-test zero/negative/overflow times, split boundaries, trim constraints, and command reversibility.

**Acceptance:** a project can be built and edited in headless tests without a UI, decoder, or GPU.

### M3 — Import, probe, and decode a real video

**Goal:** retrieve correct decoded frames from common MP4 media.

- [ ] Choose backend: FFmpeg integration or platform media decoder, behind a Rust trait.
- [ ] Probe duration, codec, dimensions, frame rate, pixel format, rotation, and stream timebase.
- [ ] Decode the first frame from a test MP4.
- [ ] Request a frame near a specified timeline position using source PTS and a documented seek policy.
- [ ] Return owned frame buffers or well-defined shared/GPU handles with explicit lifetimes.
- [ ] Verify failure cases for missing files, bad timestamps, empty streams, and corrupt input.

**Acceptance:** headless decoder tests can open known fixtures, return expected dimensions and approximate time/PTS, and release resources without leaks or crashes. Unsupported formats return clear errors.

### M4 — GPU preview vertical slice

**Goal:** show the decoded video frame in the Preview panel.

- [ ] Select/pin `wgpu` integration and connect it to the application/window lifecycle.
- [ ] Implement frame upload, texture ownership, color conversion strategy, and aspect-fit output.
- [ ] Display a decoded frame without blocking the UI thread.
- [ ] Handle resizing, device errors, empty frames, and source rotation.
- [ ] Instrument upload, compose, and render/present timing separately.

**Acceptance:** a real MP4's first/selected frame is visibly correct in the app and the timing path can be measured. No frame is silently interpreted with the wrong stride/pixel format.

### M5 — Interactive playback and seek

**Goal:** responsive playback with one video track.

- [ ] Implement playback states, monotonic clock, and media PTS/timebase conversions.
- [ ] Add Play, Pause, Stop, seek, and latest-request-wins cancellation.
- [ ] Add bounded decode queue/frame cache and backpressure; never block the UI on decode.
- [ ] Define frame selection and late-frame policy; separate UI draw rate from media rate.
- [ ] Track dropped/late frames, seek latency, queue depth, and memory budget.
- [ ] Run reproducible 1080p/30 and 4K/30 benchmarks on documented Windows hardware.

**Acceptance:** user can play/pause and scrub a real single-track video without hanging the interface; measured performance and failures are recorded. Do not mark 4K/30 FPS achieved without an actual test.

### M6 — Useful timeline editing

**Goal:** basic non-linear edit workflow.

- [ ] Drag video into a timeline track; render clip/track/playhead views.
- [ ] Trim, split, move and delete clips using validated commands and undo/redo.
- [ ] Map timeline time to source PTS correctly across trim and speed=1.
- [ ] Support selection, zoom, snapping policy, and keyboard shortcuts.
- [ ] Preserve editor state when a decode or media operation fails.

**Acceptance:** a real file can be imported and edited, undo/redo restores project state, and seek uses the edited mapping rather than raw asset time.

### M7 — Subtitles, audio and export MVP

**Goal:** finish the initial TikTok-style editing workflow.

- [ ] Add a text/subtitle track with basic font, position, and timing.
- [ ] Decode/play one audio track; choose/document an A/V clock strategy.
- [ ] Add volume and simple audio handling.
- [ ] Implement export for a deliberately small codec/container profile chosen in an ADR.
- [ ] Validate exported duration, resolution, audio sync, subtitles, and timestamp continuity.
- [ ] Test save/reopen and interruption behavior once native project persistence exists.

**Acceptance:** import → edit video + captions + audio → preview → export passes end-to-end integration checks on Windows.

### M8 — Compatibility, polish and acceleration

- [ ] Validate CapCut draft schema using user-authorized, sanitized fixtures; implement only supported fields.
- [ ] Add optional hardware decode/zero-copy paths with measured CPU fallback.
- [ ] Optimize cache, proxy playback, text rendering and color pipeline.
- [ ] Improve accessibility, localization, packaging, crash recovery and telemetry privacy.
- [ ] Expand to multi-layer timeline and effects only after the simpler workflow is stable.

## 3. Performance targets versus evidence

**Targets to benchmark, not established capabilities:**

| Scenario | Desired user experience | Proof required |
|---|---|---|
| 1080p/30 single clip | Responsive play/scrub | Frame presentation trace and measured latency |
| 4K/30, one video + subtitles + audio | Smooth practical preview on suitable hardware | Hardware profile, preview scale, dropped-frame rate, CPU/GPU utilization, memory |
| Seek/scrub | Latest requested frame wins promptly | p50/p95 latency, repeated back-and-forth seek test |
| Export | Deterministic output with A/V sync | Validated file and automated fixture comparison |

See `docs/PERFORMANCE.md` for benchmark methodology. Performance numbers without hardware and test settings are not accepted.

## 4. Priority rules for agents

1. Fix data loss, crashes, hangs, and broken builds before optional features.
2. Finish the **current milestone** before expanding scope.
3. Prefer one end-to-end slice to many speculative abstractions.
4. Update only verified checkboxes. Keep known failures visible.
5. Add ADRs for durable choices: timeline representation, media backend, graphics backend, project serialization, and hardware acceleration.
6. Research about CapCut architecture is supplementary, not a prerequisite for implementing a native editor.

## 5. Roadmap update protocol

When completing a milestone task, provide: the commit/PR if one was authorized, changed paths, tests and their actual results, user-visible proof, any platform limitations, and the next unchecked task. Mark an item `[x]` only after its acceptance criteria pass. Do not estimate completion percentages from file counts.
