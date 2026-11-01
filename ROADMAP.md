# CutCraft Roadmap

> **Project stage:** scaffold / pre-MVP. **Platform priority:** Windows desktop. **Primary workflow:** one 4K/30 FPS video clip, subtitle text, and audio on a short TikTok-style timeline.
>
> Status is evidence-based. A directory, comment, trait declaration, or placeholder is **not** an implemented feature. Update status only after meeting the milestone's acceptance criteria.

**Status vocabulary:** **IMPLEMENTED** (automated and/or real QA evidence), **IMPLEMENTED — LIMITED SCOPE** (works with documented limits), **SUPERSEDED** (historical behavior replaced by a later milestone), **PLANNED**, **BLOCKED**, **UNVERIFIED**. Historical milestone evidence is retained even when a later milestone supersedes a workflow.

### Workstream

**Native CutCraft Editor:** M3-D.1 recovery/data integrity → GPU renderer foundation → playback scheduling → audio → subtitle/compositing → native export.

## 1. Outcome and non-goals

**North star:** an independent, robust Rust video editor capable of loading a real project, editing a timeline, previewing accurately, and exporting a correct result. Start with one video track and subtitle/audio tracks; layer complexity comes later.

**Near-term non-goals:** cloning another editor's interface, custom codecs from scratch, multi-camera editing, effects marketplace, online collaboration, AI functions, and immediate 4K/30 FPS parity on every machine.

### Current verified-in-repository facts

- [x] Cargo workspace defines `apps/capcut-editor`, seven library crates, and `xtask`.
- [x] Rust toolchain declared as `1.97.1`; edition `2021`.
- [x] Scaffold module boundaries and project documents exist.
- [x] GitHub Actions workflow for format, Clippy, and tests exists **as configuration**.
- [ ] A passing CI run has been verified for this roadmap revision. Local Windows format, Clippy, test, and check gates passed on 2026-11-01 on the integrated working tree (497 tests, 2 ignored) together with the strict media gate; CI is not claimed.
- [x] A graphical desktop application exists with Home, Projects, Settings, a native managed-project Editor, and an Export settings dialog; it was launched, resized, and closed normally on Windows (M1–M3-D.1 QA).
- [x] A real clip can be imported into the timeline. (M3-D Media Pool import registers a persistent media asset and appends an asset-backed clip with exact ticks; see the M3-D evidence below.)
- [x] A managed project can be AutoSaved and recovered after supported abnormal termination. (M3-D.1 bounded recovery snapshots; no journal/power-loss guarantee; see the M3-D.1 evidence below.)
- [x] Decoded frames can be presented through a renderer-owned GPU path. (M4-A single-layer wgpu Preview with egui fallback; continuous playback is not implemented.)

`docs/ARCHITECTURE.md` distinguishes existing code from planned structures. Do not convert roadmap items to `[x]` until evidence exists.

## 2. Prioritized implementation milestones

### M0 — Establish a reliable baseline

**Goal:** ensure the empty workspace builds cleanly and the agent workflow has guardrails.

- [x] Run and record `cargo check --workspace`, format, Clippy, and tests on Windows (2026-10-29).
- [x] Add first unit tests for `capcut-core` and a smoke test for the application executable. (Implemented; workspace suite covers core/engine/project/media/UI.)
- [x] Agree on timebase/PTS representation in an ADR before implementing clip math. (ADR-0002; implemented and tested.)
- [x] Agree on media decoding integration and license/deployment plan in an ADR. (ADR-0005 runtime FFmpeg CLI; not bundled/linked; media gate documented.)

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

**Status:** IMPLEMENTED — LIMITED SCOPE. Typed project/timeline state, exact time, transactional edits, selection, playhead and history exist; Insert/Move/TrimStart/nested/linked movement remain PLANNED.

**Goal:** independent project state and deterministic edit math.

- [x] Define typed project, asset references, sequence, tracks, clips, and duration in `capcut-engine`. (M2 plus M3-D media assets.)
- [x] Implement rational FPS and exact timeline/media conversions in `capcut-core`. (ADR-0002; tested across supported rates.)
- [x] Add track insertion, clip insertion, selection, and playhead position. (Track/clip construction and selection are implemented; the interactive playhead is UI-session state per ADR-0004. Arbitrary UI clip insertion/move is not.)
- [x] Implement validated editing commands and undo/redo before hooking up editable widgets. (DeleteClip, TrimClipEnd, SplitClip, NormalizeMainTrack, asset/clip commands; snapshot Undo/Redo; limited command set.)
- [x] Unit-test zero/negative/overflow times, split boundaries, trim constraints, and command reversibility. (Implemented.)

**Acceptance:** a project can be built and edited in headless tests without a UI, decoder, or GPU. **Met for the implemented command subset.**

**M2 partial evidence (2026-10-30; Magnet corrected 2026-10-31):** exact 120,000-tick timebase, rational supported frame rates, stable IDs, an optional main video track, transactional DeleteClip/TrimClipEnd/SplitClip, snapshot undo/redo, and engine-owned Main Track Magnet are implemented and tested. M2.1 hardening adds monotonic non-reused IDs and an 8 MiB metadata-only history budget with eviction/oversized-transaction rejection. M2.2 corrects Magnet semantics: activation executes `NormalizeMainTrack` in one transaction (A `[0,5)`, B `[10,15)`, C `[20,25)` → A `[0,5)`, B `[5,10)`, C `[10,15)`), reversing the earlier duration-only ripple model; Undo/Redo carry the exact Magnet state with the positions, and linked/nested content rejects activation and edits atomically. The UI exposes this only through `CUTCRAFT_NATIVE_QA=1` with four main clips plus an unlinked synthetic caption; ordinary sessions remain empty. Insert/move, trim start/extension, verified linked-media movement, nested editing, playback, and the remaining M2 items are still open. This is not a complete M2 milestone.

### M3-A — Native project persistence

**Status:** Core persistence IMPLEMENTED; manual Save/Save As as the primary UI is SUPERSEDED by M3-D managed AutoSave. Low-level save/open APIs remain for tests, duplication, and recovery.

**Goal:** create → edit → save → close → reopen → edit without losing exact timeline state.

- [x] Add the L2 `capcut-project` crate with versioned `.cutcraft` JSON v1 and strict unknown-field/version rejection.
- [x] Add validated `Project::from_parts` and `NativeEditor::from_restored` restore APIs without adding serde/filesystem dependencies to the engine.
- [x] Persist project identity/name, rational FPS, timebase, tracks/clips, exact ticks, source mappings, locks/links/nested flags, allocator high-water marks and engine-owned Magnet state.
- [x] Implement bounded open validation, staged atomic save, external-change detection, and target-overwrite protection.
- [x] Add New/Open/Save/Save As UI, save status and Save/Discard/Cancel unsaved-change protection.
- [x] Cover Magnet OFF gaps, ON continuity, Save after Undo/Redo, reopen-and-edit, failure atomicity, malformed/oversized/unsupported files and toolbar stability.
- [x] Autosave, recent-project listing, and project duplication. (Implemented by M3-D managed AutoSave/library; supersedes the manual primary Save flow. Selection and playhead are intentionally session-only and not persisted.)
- [x] Recovery after ordinary save failures and supported abnormal termination. (M3-D.1 bounded recovery snapshots; see M3-D.1. This is not a general power-loss guarantee.)
- [ ] Remaining M2 editing operations (insert/move, trim start/extension, verified linked movement, nested editing). (Still PLANNED.)
- [ ] Persist selection/playhead. (Deliberately not implemented; ADR-0004 keeps them session-only.)

**M3-A evidence (historical; manual Save flow superseded by M3-D, 2026-10-31, Windows):** real app saved a gapped QA project (Magnet OFF), activated Magnet (compacted), saved and reopened it with Magnet ON and exact starts `0/5/10/15 s`, disabled Magnet, saved/reopened the compacted OFF state, then deleted a clip and used Undo/Redo in the reopened session. External file modification produced a visible conflict error without overwriting; closing dirty opened Save/Discard/Cancel. Timeline toolbar UIA geometry checks at 1920×1080, 1600×900 and 1210×710 reported zero shifts for 9/9/6 unrelated controls across OFF→ON→Undo→Redo. Full workspace gates pass; details are in the M3-A spec and QA logs.

### M3-B — Authoritative playhead, scrubbing and Split-at-Playhead

**Goal:** make the Timeline playhead an authoritative editing position and connect the existing split transaction to the UI.

- [x] Session-only `TimelineTicks` playhead with checked frame snapping, clamping and zero-duration handling.
- [x] Reusable tick↔viewport mapping covering zoom, scroll, header width and pointer validation.
- [x] Ruler click/drag scrubbing, handle dragging, frame navigation and jump start/end through the existing shortcut registry.
- [x] Real Split-at-Playhead with exactly-one-main-clip eligibility, source/time preservation, Magnet OFF/ON behavior, Undo/Redo and no ripple.
- [x] Preview timecode from the playhead and project end; explicit no-media placeholder; no fake playback or decoding.
- [x] Playhead is not persisted and never dirties project content; save/reopen after Split preserves exact results and allocator state.
- [x] Unit, integration and real Windows QA at 1920×1080, 1600×900 and 1210×710.
- [x] Media probing/decoding and decoded preview. (Implemented in M3-C/M3-D; this M3-B item was completed by later milestones.)

**M3-B evidence (2026-10-31, Windows):** ruler click at 12 s produced exactly 1,440,000 ticks; dragging produced 11 s; keyboard next produced 1,324,000; End/Home reached exact 4,200,000/0; two zoom steps preserved the playhead; horizontal scroll moved the 12 s position offscreen and clicking the old x clamped to the exact 35 s project end. Split at 12 s produced left/right IDs `3`/`6` with source ranges `0–240,000`/`240,000–600,000` and `next_clip_id=7`; Undo/Redo and Save/Reopen preserved the result, playhead reset to 0, and a second split allocated ID `7`. Real split checks passed at all three viewport sizes.

### M3-C — Probe, decode, and real frame Preview

**Status:** IMPLEMENTED — the session-only clip binding described in its checklist was SUPERSEDED by M3-D persistent media assets; the probe/decode/PTS/stale-result behavior remains current.

**Goal:** make one trusted local H.264 source produce correct frames at the authoritative playhead without blocking the UI.

- [x] Choose and document the Windows backend (ADR-0005: runtime-discovered local FFmpeg CLI; no bundled binaries or FFI linkage).
- [x] Probe container, stream index, codec, dimensions, pixel format, rational timebase, nominal/average rate, duration, stream start, rotation, audio presence, and typed errors.
- [x] Decode real frames with actual presentation timestamps; seek to a decode start and decode forward to the selected PTS; handle B-frames, VFR, nonzero start, rotation, long GOP, EOS, and source switching.
- [x] Keep `TimelineTicks`, clip `SourceTicks`, and decoder PTS distinct with checked rational conversion, trim offset, stream start, and explicit speed/source rejection.
- [x] Run probe/decode on one bounded background worker with a latest-request slot, generation/source identity, stale-result suppression, and clean shutdown.
- [x] Display real frames in the existing Preview panel with real aspect/orientation, one reused texture, no unchanged-frame reuploads, explicit empty/end/error states, and a session-only clip binding that never changes `.cutcraft` or project state.
- [x] Verify with generated fixtures, automated tests, and real Windows screenshots/logs. 4K is a single-frame diagnostic only.

**Acceptance evidence (2026-10-31, Windows):** real app opened 1920×1080/30 media, showed exact probe metadata, displayed red at playhead 0, green at 1 s, blue at 2 s, and green again after a backward seek. Eight rapid scrubs produced stale generations 6–11 that were ignored and settled on generation 12 at PTS 4.100 s. The exclusive 5 s clip end showed no frame. Switching to 1280×720 then back to 1080p and to two other sources displayed only the current source's frame. VFR at a 0.5333 s request displayed actual PTS 0.500 s. Unsupported MPEG-4 and truncated MP4 returned visible typed errors. Measured probe was 24–31 ms; 1080p seek-to-frame was ~146–880 ms including process startup (warm ~146–201 ms); one 4K frame decoded in 302 ms with a 328 ms request (not a realtime claim). The same bound 1 s frame was verified at 1210×710, 1600×900, and 1920×1080, with the Open Media modal fitting at 1210×710. No audio, playback, compositor, export, or hardware decode is implemented.

### M3-D — Managed projects, persistent media assets, and clip-aware AutoSave

**Status:** IMPLEMENTED — LIMITED SCOPE. Managed creation/AutoSave, schema v2, assets, relink, Home, and clip Preview work; no AutoRecovery existed until M3-D.1.

**Goal:** make “Create Project → import media → edit → return Home → reopen → continue editing” a real, restart-safe workflow without manual Save As or hidden media copying.

- [x] Add a bounded engine media asset registry with stable `MediaAssetId`s, probe-derived stream metadata, deterministic locators, and clip references that participate in commands/undo/redo.
- [x] Extend `.cutcraft` to strict schema v2 (project identity, timestamps, save generation, assets, clip `media_asset_id`); migrate v1 in memory without rewriting it on open (ADR-0006).
- [x] Add `AddMediaAsset`/`RelinkMediaAsset`/`AppendMediaClip`/`RenameProject` commands with exact ticks, stable Clip IDs, Magnet semantics, and Undo/Redo.
- [x] Create managed projects under a library root with automatic initial persistence, Home Recent Projects, rename/duplicate, and invalid-entry reporting.
- [x] Implement generation-safe AutoSave with ~1.5 s debounce, ~15 s maximum interval, one bounded worker, dirty-generation retention, failure preservation, external-conflict pause, and navigation/exit flush (ADR-0006).
- [x] Replace the session-only Preview binding with active-clip resolution through persistent assets, exact trim/PTS mapping, offline/changed detection, explicit relink, multiple-layer reporting, and stale-result suppression.
- [x] Build real Media Pool cards/import/append/relink UI and remove the remaining synthetic media grid.
- [x] Verify with automated engine/project/UI tests and a real Windows two-video workflow.

**Acceptance evidence (2026-10-31, Windows):** The real app created a managed project with no destination dialog and showed “Saved” after generation 0. Media Pool imported `qa-a rouge.mp4` (1280×720) and `qa-b bleu.mp4` (640×360), then appended both to V1 at exact ticks `0–240,000` and `240,000–480,000` (Clip IDs 1/2, Asset IDs 1/2). Preview showed red PTS 0.000 s at 0 s, red PTS 0.500 s at 0.5 s, blue PTS 0.000 s at the 2 s boundary, and blue PTS 1.000 s at 3 s; rapid scrubs settled on blue PTS 1.500 s. Undo removed the appended clip (no active clip at 3 s); Redo restored the same Clip 2/Asset 2 frame. AutoSave reached a saved generation after edits, Home showed the real Recent Projects entry (schema v2), and reopening restored the same assets/clips and frames. Moving `qa-b bleu.mp4` away produced an Offline Media Pool card and “Media offline” Preview; Relink to the renamed file preserved Asset 2 and displayed the blue frame again. The final `project.cutcraft` was schema v2, generation 7, `next_clip_id`/`next_asset_id` 3, stored the relinked absolute locator, and contained no media copies (3,429-byte project file only). Probe measured 22–23 ms; 720p decode/seek was 62–114 ms / 73–132 ms after the first 602 ms request. Test suites: engine 61, project 34, UI 296; the full workspace run passed 440 tests with 0 failures and 2 intentionally ignored tests that require a locally supplied private fixture. No commit or push was made.

**Limitations at M3-D:** No AutoRecovery journal (AutoSave is not crash-proof); conflict recovery offered Retry/Stay only. M3-D.1 adds bounded recovery snapshots and Save Recovery Copy; see the M3-D.1 section and ADR-0007. Inactive rename/duplicate run synchronously; no audio, playback, compositing, linked-media movement, nested editing, hardware decode, or export encoding.

### M3-D.1 — Recovery and data integrity

**Status:** IMPLEMENTED — LIMITED SCOPE. Bounded one-slot recovery snapshots, explicit restore as a new managed project, external-conflict recovery copies, FFmpeg process timeouts, abandoned-staging cleanup, worker panic containment, documentation reconciliation, and a media-enabled gate are implemented. No journal, OS-level multi-project lock, or power-loss guarantee is implemented.

- [x] Add strict recovery envelope, SHA-256 integrity, generation/identity validation, atomic one-slot publication, bounded recovery worker and panic containment (ADR-0007).
- [x] Detect/classify Newer/Same/Stale/Orphan/Invalid recovery candidates; never auto-apply; retain invalid/stale data for inspection.
- [x] Restore as a new managed ProjectId while preserving internal clip/asset IDs, exact ticks, allocator high-water marks, Magnet, locators, and media bytes; rebase relative locators; original primary stays unchanged.
- [x] Add explicit recovery UI (Restore as New Project, Open Last Saved, Keep Both) and Home recovery/warning entries.
- [x] Add external-conflict **Save Recovery Copy** as a new project without overwriting the externally modified file or discarding the live session.
- [x] Harden FFmpeg/FFprobe execution with hard timeouts, clean up stale staging files, contain persistence/recovery executor panics, and test read-only target failure.
- [x] Add the strict media gate (`CUTCRAFT_REQUIRE_MEDIA=1`), fail loudly when tools/fixtures are unavailable, and keep intentionally ignored private-fixture tests separate.
- [x] Document actual vs planned architecture, status reconciliation, media identity distinctions, and the ADR index.

**Acceptance evidence (2026-10-31, Windows):** The real app created a managed project at generation 4, appended a third clip, published a validated recovery snapshot at generation 5, and was force-terminated before the next primary AutoSave; the primary remained at generation 4. Relaunch detected the candidate and showed the explicit recovery modal; Restore as New Project published a distinct ProjectId at generation 0 with the same 3 clips and asset/clip IDs, the same exact ticks, and the original primary SHA-256 unchanged; the recovery file was removed only after successful restore. A second run externally modified the primary, detected Conflict, opened the save-problem dialog, and published a **Save Recovery Copy** with a new ProjectId and the same 3 clips while preserving the external file bytes; the copy appeared in Home and opened with working Preview. A stale candidate (recovery generation 3 vs primary 4) was not offered and remained retained; a corrupt candidate produced a Home warning, no Restore action, and the primary still opened unchanged. An abandoned 30-hour staging file was removed on project open; graceful close/reopen succeeded. The strict media gate ran 16 tests against `ffmpeg/ffprobe 8.1.2-full_build` and failed clearly when the tools were deliberately made unavailable. Workspace gates: **463 passed, 0 failed, 2 ignored** (tests requiring a locally supplied private fixture); engine 62, project 37, UI 305, media 16. Recovery publication p50/p95: empty 1.93/2.30 ms, 100 clips/20 assets 14.49/14.82 ms, 1000 clips/100 assets 114.82/214.13 ms (background worker, not a UI stall); controlled blocked-worker UI tick p95 ~0.001 ms. **Explicitly UNVERIFIED:** power-loss durability, full-disk behavior, antivirus/share-mode failure, OS-level multi-process locking/simultaneous writers, Windows parent-directory flush, and media content hashing. No commit or push was made.

### M3 — Import, probe, and decode a real video

**Goal:** retrieve correct decoded frames from common MP4 media.

- [x] Choose backend: local FFmpeg CLI adapter documented in ADR-0005 (library/native alternatives deferred).
- [x] Probe duration, codec, dimensions, frame rate, pixel format, rotation, and stream timebase.
- [x] Decode the first frame from a test MP4.
- [x] Request a frame near a specified timeline position using source PTS and a documented seek policy.
- [x] Return owned frame buffers with explicit lifetimes and bounded sizes.
- [x] Verify failure cases for missing files, bad timestamps, empty streams, and corrupt input.
- [x] Import media into the native timeline (M3-D registers persistent assets and appends asset-backed clips; gaps around trimmed clips and multi-layer composition remain explicit).

**Acceptance:** headless decoder tests can open known fixtures, return expected dimensions and exact PTS, and release resources without leaks or crashes. Unsupported formats return clear errors.

### M4 — GPU preview vertical slice

**Status:** M4-A / M4-A.1 IMPLEMENTED — LIMITED SCOPE. A renderer-owned wgpu path displays one decoded video layer through eframe's existing device/queue, with the existing egui texture as fallback, and its colour/stride/identity behaviour is now verified on a real device and in the real window. Continuous playback, composition of multiple layers, text/effects, GPU completion/present timing, and device-loss recovery remain PLANNED/UNVERIFIED.

**Goal:** move beyond a single egui-uploaded source frame to a GPU renderer/compositor Preview.

- [x] Select/pin wgpu integration and connect it to the application/window lifecycle. (ADR-0008: eframe 0.37 default wgpu; reuse `CreationContext.wgpu_render_state`; no second device.)
- [x] Implement frame upload/ownership and color-conversion strategy in `capcut-renderer` rather than the UI layer. (Renderer-owned `Rgba8Unorm` texture, sampler, bind group, pipeline; normalized RGBA8 only; no YUV/BGRA reinterpretation.)
- [x] Display decoded frames without blocking the UI thread. (Single-frame GPU upload is worker-independent and records CPU enqueue time; egui fallback remains; decode remains on the FFmpeg worker. Continuous playback is not implemented.)
- [x] Handle resizing, empty frames, source rotation, and one-layer composition. (Aspect/orientation preserved; rotated/portrait/4K tested; multiple active visual layers remain explicit `UnsupportedComposition`.)
- [ ] Handle GPU device loss and multiple composition inputs. (UNVERIFIED/PLANNED; no compositor yet.)
- [x] Instrument renderer upload/submission separately from CPU decode. (CPU enqueue, allocation and render-pass recording are measured per frame; GPU completion/present are `None` because they are not observable in this build.)
- [x] Verify presented colour, stride and identity on a real device and in the real window. (M4-A.1: 14 device-backed readback tests plus an eight-clip, 16-point-grid Windows QA at zero deviation; see `crates/renderer/tests/preview_gpu.rs` and `target/m4a1-qa/`.)
- [x] Integrate the shared working tree with concurrent in-tree work without losing either side. (M4-A.1: hash-verified freeze before formatting, formatting limited to the two unformatted Preview files, foreign hashes re-verified unchanged; 488 integrated tests pass.)
- [x] Attribute UI tick cost instead of assuming the upload dominates. (M4-A.1: sub-phase tracing showed ~2.8 ms egui layout/paint, ~0.2–3.5 ms enqueue, and a logging-dominated arrival tick; recorded with the observer-effect caveat in `docs/PERFORMANCE.md`.)
- [ ] Implement GPU text/effect/subtitle compositing. (PLANNED; future extension contract only.)

**M4-A evidence (2026-10-31, Windows):** The real app reused eframe's **Vulkan** device on an NVIDIA GeForce RTX 2080 SUPER (`target=Bgra8Unorm`) and displayed `GPU PREVIEW` frames through the renderer-owned pipeline. At 1600×900, real media produced red 1280×720 PTS 0.000/1.000 s, blue 640×360 PTS 1.000 s at the clip boundary, green portrait 360×640 PTS 1.000 s, rotated 360×640 red PTS 1.000 s, and gold 3840×2160 PTS 1.000 s at 9 s; center pixels matched the fixtures (`253,0,0`, `0,0,254`, `0,127,0`, `254,214,0`). Rapid scrubs settled on Clip 2/Asset 2 at PTS 1.500 s. The same frames rendered correctly at 1210×710, 1600×900, and 1920×1080. Forced `CUTCRAFT_PREVIEW_RENDERER=egui` produced the same red frame with `EGUI FALLBACK`. Offline B produced the explicit offline state; Relink returned the blue GPU frame. Opening a second project cleared the first frame and rendered the new project with a new source identity; close/reopen recreated the GPU path and rendered the correct red frame. Renderer metrics: 720p upload 0.43 ms, 4K upload 3.19 ms (CPU enqueue), renderer submit 0.0007–0.0016 ms, 4K texture 33,177,600 bytes, 8 reallocations/3 reuses during the source sequence; GPU upload completion and present time are **N/A**. 4K decode/seek was 263/275 ms on the first measured request and 325/345 ms on the second (single-frame diagnostic, not playback). Workspace gates: **471 passed, 0 failed, 2 ignored** (renderer 4, UI 305, media 16; the 2 ignored tests require a locally supplied private fixture). No commit or push was made.

**M4-A limitations:** no continuous playback, playback clock, audio, export, GPU compositor, text/effect rendering, YUV shader, HDR/color management, device-loss recovery, or GPU completion/present timing. Upload currently occurs on the UI thread via `queue.write_texture` and is measured; large-frame upload is not claimed to be free. The egui fallback remains the safe path when the renderer is unavailable.

**M4-A.1 evidence (2026-11-01, Windows) — IMPLEMENTED — VERIFIED, LIMITED SCOPE.** Shared working-tree integration plus GPU correctness. Concurrent in-tree work was preserved rather than overwritten: a freeze watch confirmed the other agent's files were byte-stable before formatting, formatting was then applied only to the two Preview files that were actually unformatted, and the foreign files' hashes were re-verified unchanged afterwards. Integrated gates: **488 passed, 0 failed, 2 ignored** (renderer 18, UI 307, format 24, project 44, media 10 + 16, engine 62), `cargo fmt --all --check` clean, Clippy `-D warnings` clean, `cargo check` clean, strict media gate 16/16. Correctness was strengthened with `crates/renderer/tests/preview_gpu.rs`: 14 device-backed tests that render fixtures through the real pipeline and read the **presented** pixels back on this machine's Vulkan/RTX 2080 SUPER device, covering BGRA memory order, byte-exact opaque passthrough, gradient pixel identity, straight-alpha blending, non-256-aligned row strides, portrait/landscape orientation, resolution change without stale pixels, generation/source/clip identity, cross-project id reuse, clear-then-repaint, rejection before allocation, and the sRGB target branch. Real-window QA (`target/m4a1-qa/`, eight owned synthetic H.264 clips) verified each frame on a 16-point canvas grid against FFmpeg's own decoded bytes: **worst deviation 0** at tolerance 3 for every clip, for rapid scrub settling, and at 1210×710, 1600×900 and 1920×1080; measured canvas geometry 435×243 px for 16:9 and 179×317 px for portrait. Forced egui fallback, the new deterministic `CUTCRAFT_PREVIEW_RENDERER=init-failure` seam (visible warning plus fallback frame), offline → Relink, close/reopen, unchanged project hash and continued AutoSave all passed. Measured CPU cost: upload enqueue 0.202/0.465/3.251 ms p50 for 640×360/1280×720/3840×2160, allocation ~0.21–0.26 ms, render-pass recording 0.0007–0.0012 ms, egui layout/paint 2.8 ms p50; peak texture and staging 33,177,600 bytes. **Explicitly unverified:** GPU upload completion and present time (`None`, not observable), device-loss recovery, playback, and production scrub latency — frame-arrival tick timings taken with `CUTCRAFT_RENDER_LOG=1` are dominated by the diagnostic writes themselves, which was established by experiment and is recorded in `docs/PERFORMANCE.md`. No commit or push was made; the 2 ignored tests require a locally supplied private fixture and stayed ignored.

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
- [x] Test save/reopen and interruption behavior once native project persistence exists. **IMPLEMENTED — LIMITED SCOPE:** M3-A covers native save/reopen; M3-D covers managed AutoSave, reopen with media, conflict failure preservation, and offline/relink. M3-D.1 covers bounded recovery snapshots after forced termination. Power-loss durability, full-disk behavior, and multi-process locking are **UNVERIFIED**; do not claim crash-proof recovery.

**Acceptance:** import → edit video + captions + audio → preview → export passes end-to-end integration checks on Windows.

### M8 — Compatibility, polish and acceleration

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
6. Reverse-engineering and third-party format research is not a roadmap track for this repository (see RULE 000 in `AGENTS.md`). Native editor work does not depend on it.

## 5. Roadmap update protocol

When completing a milestone task, provide: the commit/PR if one was authorized, changed paths, tests and their actual results, user-visible proof, any platform limitations, and the next unchecked task. Mark an item `[x]` only after its acceptance criteria pass. Do not estimate completion percentages from file counts.
