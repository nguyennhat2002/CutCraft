# CutCraft — Testing, CI and Verification

> **Current status:** The repository contains a GitHub Actions configuration and a tested egui/eframe editor with native `.cutcraft` schema v2 persistence (M3-A/M3-D), managed project AutoSave, persistent media assets, bounded recovery snapshots and external-conflict preservation (M3-D.1), an authoritative playhead/split path, a bounded H.264 FFmpeg-CLI probe/decode/clip-aware single-frame Preview, and an implemented renderer-owned single-layer wgpu Preview with egui fallback (M4-A/M4-A.1). Playback, audio and encoding are not implemented, and the GPU renderer is a one-layer Preview rather than a compositor. AutoRecovery is bounded snapshot recovery, not a journal and not a power-loss guarantee. Tests and benchmarks described here are future requirements unless explicitly recorded as executed.

## 1. Current commands

Use the pinned Rust toolchain (`rust-toolchain.toml`: `1.97.1`) and run at the workspace root:

```powershell
cargo fmt --all -- --check --config-path .config/rustfmt.toml
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace
```

The existing `.github/workflows/ci.yml` runs formatting, Clippy and tests on Windows, Linux and macOS runners. CI configuration does **not** mean CI has passed. Once platform-specific dependencies arrive, update that workflow rather than assuming native GPU/media integration builds identically on all OSes. `xtask` is currently a scaffold; its build/test/package modules are not working quality gates.

For a focused edit, iterate with `cargo test -p <package>` or `cargo check -p <package>` and finish with the full applicable workspace checks. Avoid adding release/packaging gates to the required list until their implementations exist.

### Media-enabled gate

The ordinary workspace run may skip FFmpeg-dependent integration tests when the tools are absent; a skip is not a media pass. Run the explicit media gate on a machine with `ffmpeg`/`ffprobe` and H.264 encoder support:

```powershell
$env:CUTCRAFT_REQUIRE_MEDIA = "1"
cargo test -p capcut-media --test media_pipeline -- --nocapture
Remove-Item Env:CUTCRAFT_REQUIRE_MEDIA
```

In this mode missing tools, probe/decode fixture generation failure, or a missing H.264 encoder fails the run. Record the exact executable names and `-version` output printed by the gate. The two `capcut-format` private-fixture tests remain intentionally ignored unless `REDACTED-PRIVATE-FIXTURE-ENV` (and the copy destination for the copy test) are supplied; they are separate from the media gate.

### Current UI shell checks (M1.2)

Automated (M1.2 historical run: 70 tests; the current UI suite is 305 tests):

- headless Home, Projects, Settings, Editor, and Export dialog rendering at
  960×600, 1080×720, and 1600×900 as applicable;
- workspace geometry at the reference viewport (33/29/38 columns, usable center
  at small widths) and timeline height ratios;
- navigation transitions, native-session preservation, and Home/Resume behavior;
- export setting validation and form persistence across dialog close/reopen;
- window title, default size, minimum inner size, and resizability;
- stable idle frames with **no** follow-up repaint for Home, Editor, and Export;
- timeline zoom clamping, zero-time playhead visibility, ruler/ticks, and 16:9/9:16 fitting;
- category-specific library, tabbed inspector, dedicated controls, and disabled-action states.

Manual Windows procedure:

1. Run `cargo run -p capcut-editor` with no media present.
2. At approximately 1600×900, confirm Home shows the Create Project banner first,
   feature cards second, and Recent projects underneath.
3. Use Create Project; confirm the Editor shows nine tool categories, a center
   monitor, a tabbed Inspector, and a timeline occupying roughly 40–45% of the
   window height with four empty tracks.
4. Confirm splitter handles, timeline zoom controls, disabled track controls,
   disabled playback, and disabled editing actions are honest.
5. Toggle 16:9 and 9:16; confirm the canvas preserves aspect.
6. Use File → Back to Home, then Resume editing; confirm native-session state survives and unsaved changes are guarded.
7. Open Export; confirm output name/destination, format, codec, resolution,
   frame rate, color space, bitrate, and audio. Confirm Export is disabled and
   no output size/progress is shown. Cancel and confirm the Editor returns.
8. Resize to 1080×720 and back; confirm no overlaps, scroll areas work, and the
   layout remains usable.
9. Close with the window button; verify a normal exit.

Result recorded 2026-10-29 on Windows: Home, Editor, and Export were captured at
1600×900 and verified at 1080×720. The Editor showed the target column
proportions, nine category shells, four track rows, a full-height timeline
workspace, and the tabbed Inspector; the Export modal, Cancel return, portrait
preview, Home/Resume state preservation, and `cargo run -p capcut-editor` exit
code 0 were verified. Because the interactive desktop was locked, captures used
`PrintWindow` and real buttons were invoked with Windows UI Automation. The four
supplied reference screenshots were not present in the workspace/session, so a
direct pixel/visual similarity comparison and a 90% claim remain **unverified**.
`File → Exit` was not separately clicked. CI on Linux/macOS was not run.

Before/after findings from this iteration:

- the preview was far too wide and the side panels too narrow; the new workspace
  is 33% library / 29% preview / 38% inspector at 1600×900 with draggable splitters;
- the timeline was too short; it now targets roughly 40–45% of the window height
  and fills its workspace below four tracks;
- Home lacked the target hierarchy; the Create banner now leads, followed by
  feature cards and project management;
- badges stretched in several layouts and were replaced with fixed painted pills;
- the Inspector and tool panel were sparse; they now use dense tabbed controls and
  nine category-specific shells.

## 2. Test pyramid

| Level | Purpose | Typical location |
|---|---|---|
| Unit | Exact time/rational math, validation, clip transformations, cache decisions | Owning crate `src/` or `tests/` |
| Property / adversarial | Invalid durations, overflow, cyclic sequences, corrupt inputs | Core/engine/format/media tests |
| Integration | Probe + decode + requested frame; engine commands + undo/redo | `tests/` or dedicated test harness |
| Snapshot / image comparison | Rendering geometry, text placement, color conversion | Renderer tests/fixtures |
| UI smoke | Window, input, panels, error feedback and resizing | Windows/manual or automated UI harness |
| Performance | Frame pacing, p95 seek, CPU/GPU/memory budget | Reproducible benchmark harness |
| End-to-end | Import → edit → preview → export → inspect output | Application acceptance suite |

Do not make low-level tests depend on desktop UI initialization when the behavior can be tested headlessly.

## 3. Time and timeline test matrix

Implement as soon as time/project logic exists:

- Frame rates: `24/1`, `25/1`, `30/1`, `24000/1001`, `30000/1001`, `60/1`.
- Times: zero, one frame, last valid frame, clip in/out boundaries, negative input, empty/zero-duration clip, and overflow-adjacent values.
- Media timebases: `1/90000`, `1/48000`, `1/1000`, large/nonzero start PTS, non-monotonic or missing timestamps where the backend permits them.
- Round-trip conversions, documented rounding modes, no cumulative fractional-frame drift.
- Command invariants: adding a clip, trimming, splitting, moving, deleting, undo and redo; invalid edits leave project/history unchanged.
- Snapshot behavior: a worker reading an older immutable project cannot mutate the currently active edit state.

When modifying the time model, add a test that would have failed under the previous bug or an incorrect float-based implementation.

## 4. Media and decoder fixtures

Use owned, original, permissively licensed or synthetic clips; keep privacy and reuse rights documented. Do not commit third-party copyrighted clips or local personal footage. Prefer small deterministic fixtures generated by a tool such as FFmpeg for **testing**, subject to tool licensing and distribution rules.

Suggested initial fixture matrix:

| Fixture | What it tests |
|---|---|
| 640×360, constant 30 FPS, short H.264 MP4 | Basic probe and frame decode |
| 1920×1080, 30 FPS | Preview scalability baseline |
| 3840×2160, 30 FPS | Main performance target |
| 30000/1001 FPS | Rational frame-boundary and PTS behavior |
| VFR clip (when supported) | Use PTS instead of assuming fixed frame spacing |
| No audio / audio-only / empty streams | Missing stream handling |
| Missing/truncated/corrupt media | Graceful error, no crash, bounded memory |
| Rotated / non-square-pixel metadata | Display geometry and aspect corrections |
| YUV limited/full range | Color conversion and stride correctness |

Record generator command, tool version, seed, checksum, expected metadata, and any licensing information in fixture documentation. Store large fixtures outside the repository or in a clearly controlled test-data mechanism. For codec output comparisons, decode to a canonical format and account for expected tolerances instead of demanding byte identity across hardware backends.

## 5. Playback/seek acceptance tests

Once the engine exists:

1. Start → pause → resume → stop produces valid state transitions without blocking the UI.
2. Repeated seeks at different timestamps are **latest-request-wins**; a stale decoder result is ignored or cancelled.
3. Seek beyond end-of-media, before in-point, and to exact clip boundaries behaves according to the documented policy.
4. Queue depth and cache memory never grow without a configured cap.
5. Frame selection uses presentation timestamps; decode-order and presentation-order are not assumed equal.
6. If a decoder worker crashes or an API returns an error, playback becomes recoverable and reports the error.
7. If audio is present, the chosen master-clock behavior and sync thresholds are tested.
8. Preview output timestamps and displayed frame indices are logged separately from decode completion events.

A test passing on a fake clock does not prove real-world 4K performance. Run dedicated performance tests.

## 6. Renderer and GPU tests

- Validate plane stride, pixel format, dimensions, alpha blending and source rotation.
- Compare reference CPU output (once implemented) to GPU output using an explicitly stated pixel/PSNR/SSIM tolerance; do not pretend the CPU reference exists now.
- Check resizing (including tiny/zero-sized surfaces), device loss, swapchain errors, and UI reopen where supported.
- Check repeated import/close cycles for retained CPU buffers, textures, and GPU allocations.
- Test subtitle placement and scaling independently of source video size.
- Verify the measured presented frame, not merely the number of decoded frames.

### Current M3-A native persistence checks

Automated `capcut-project` tests cover: empty/populated round trips; exact sub-second ticks; rational FPS; track/clip IDs, order, links, locks and nested metadata; main-track identity; allocator high-water marks; Magnet OFF gaps, ON continuity, Save after activation/Undo/Redo/delete/trim; reopen-and-edit with fresh history; target-exists and external-modification/deletion conflicts; malformed, truncated, oversized, deeply nested, unsupported-schema/timebase/FPS files; duplicate/zero IDs; broken links; allocator drift; save-not-mutating history/selection/revision/Magnet; and resource limits.

`capcut-ui` tests cover Save As identity commit only on success, failed Save As preservation, open failure preserving the session, unsupported-schema feedback, dirty transitions for Magnet/Undo/Redo/Delete, preference-only Magnet changes, and Save/Discard/Cancel guards for New/Open/Home/Exit including canceled Save As and failed Save.

Manual M3-A flow (recorded 2026-10-31): save gapped QA project → activate Magnet → save/close/reopen → verify compacted starts and Magnet ON → disable Magnet → save/close/reopen → verify compacted starts and OFF → delete/Undo/Redo in the reopened session → externally modify the file and verify a visible conflict without overwrite → close dirty and exercise Save/Discard/Cancel. Toolbar geometry was checked at 1920×1080, 1600×900 and 1210×710; no unrelated Timeline-toolbar controls shifted across OFF→ON→Undo→Redo.

### Current M3-B playhead/split checks

Automated tests cover timeline tick↔pointer mapping at 24, 25, 30, 60, 24000/1001 and 30000/1001 FPS; exact frame boundaries; negative/nonfinite pointers; large timestamps; zero duration; scroll origins; zoom limits and 2x affine scaling. Session tests cover empty projects, previous/next/start/end keys, playhead clamping after delete/Magnet changes, playhead independence from selection and dirty state, split eligibility (no/multiple selection, boundaries, locked/linked/nested/non-main/source-less clips), exact left/right ranges and source ranges, Magnet ON/OFF, Undo/Redo, no stale ID reuse and save/reopen with fresh allocation. Widget tests cover ruler click, ruler/handle drag, body-drag isolation, clip-body selection, no layout shift, and scroll/zoom alignment. Preview tests cover exact tick timecode at integer and fractional rates.

Manual M3-B QA (2026-10-31): at 1600×900, ruler click at 12 s produced 1,440,000 ticks; ruler drag produced 11 s; arrow-right/End/Home produced exact frame/end/start values; two zoom steps and horizontal scroll preserved the playhead time. Split at 12 s on the main-track clip produced two clips with exact ranges/source and fresh right ID; Undo/Redo and Save/Close/Reopen preserved both clips and reset the playhead to zero; a second split after reopen allocated the next monotonic ID. The same split flow was run at 1920×1080 and 1210×710. Screenshots/logs: `target/m3b-qa/`, `target/m3b-*.log`.

### Current M3-C media probe/decode/Preview checks

Automated `capcut-media` integration tests generate local H.264 fixtures with FFmpeg and skip with an explicit message if `ffmpeg`/`ffprobe` are unavailable. Coverage includes probe metadata (container, codec, 320×180/1920×1080, yuv420p, `1/15360`, 30 fps, duration, no audio), missing file, corrupt bytes, truncated MP4, unsupported MPEG-4, 30000/1001, rotation 90 and display dimensions 180×320, nonzero stream start, VFR PTS selection, B-frame display ordering, long-GOP non-keyframe seek, end-of-stream, decode recovery, source switching, frame ownership, audio presence, and rapid latest-generation coalescing. Tests assert exact PTS and center-pixel color segments; test logs do not dump full RGBA buffers.

`capcut-ui` M3-C tests covered checked source-to-PTS conversion with trim, exclusive clip end, media end, fractional timebase and stream start; one-request scheduling; stale result rejection; oversized buffer rejection; texture handle reuse with one upload per frame; empty/end/error states; and a decoded texture replacing the placeholder. The explicit session-binding dialog and its tests were superseded and removed by M3-D; the scheduling/stale/texture coverage remains, and M3-D adds the managed persistent-asset and clip-Preview coverage recorded above.

The same bound 1 s frame was rechecked at client sizes 1210×710, 1600×900, and 1920×1080 using `target/m3c_sizes_qa.ps1` (`target/m3c-sizes-qa.log`): every size reported `Preview frame PTS 1.000s 1920x1080` with center pixel `R0 G127 B0`, and the 660 px Open Media modal fit at 1210×710. Screenshots: `target/m3c-qa/m3c-size-*.png`.

Manual M3-C QA at 1600×900 (`target/m3c-qa/`, `target/m3c-media-qa.log`, `target/m3c-qa/app-stderr.log`): opened 1920×1080/30 colors, probed 1920×1080 h264 yuv420p `1/15360` 30 fps 8.000 s, bound to `QA Clip A`, and displayed red PTS 0.000 s. Ruler click at 1 s displayed green PTS 1.000 s; 2 s displayed blue; seeking back to 1 s restored green. Eight rapid ruler positions settled at playhead 492000 ticks and PTS 4.100 s; media logs show generations 6–11 ignored as stale and generation 12 displayed. 4.9 s showed magenta, and the exact 5.0 s clip end showed `End of bound clip` with no frame. Switching to 1280×720 displayed only the purple 1 s frame, switching back displayed green again, and a 4K source displayed gold 3840×2160. VFR request at 0.5333 s displayed actual PTS 0.500 s. Unsupported MPEG-4 and truncated MP4 showed typed errors and no frame. Probe latency was 24–31 ms; 1080p seek-to-frame was ~146–880 ms including process startup (first frames ~836/880 ms, warm frames ~146–201 ms); adapter decode was 129–188 ms; one 4K frame decoded in 302 ms with a 328 ms request. Texture-submit timing measures CPU submission only. These numbers are not playback or 4K realtime evidence.

### Current M3-D managed-project, asset, AutoSave and clip-Preview checks

Automated `capcut-engine` tests cover media asset construction/limits, locator validation (including Windows drive/UNC), checked rational duration conversion, asset registration/relink/rename, asset-backed append (main-track creation, locked/ambiguous track rejection, overflow), split preserving asset references, undo/redo preserving monotonic asset IDs, and invalid metadata. `capcut-project` tests cover strict schema v2 round trips, in-memory v1 migration without file rewrite, dangling/duplicate asset references, future-schema rejection, managed create/scan/rename/duplicate identity rules, invalid entries, project-relative vs absolute locator policy, moving the library root with relative media, duplicate rebasing without media copies, Windows verbatim-prefix comparison, AutoSave debounce/max-interval/failure/retry and newer-generation dirty retention, one-worker pending-job replacement, and controlled worker ordering/shutdown.

`capcut-ui` tests cover managed creation and Home discovery without Save As; edit/asset/clip flush before Home; reconnect with the same asset and clip IDs; graceful exit waiting for the saved generation; external-file conflict without overwrite and with dirty state preserved; simulated save failure preserving dirty state and blocking navigation with a problem dialog; deterministic edit-during-save generation retention; real FFmpeg import of two non-ASCII filenames with spaces, availability validation, append, clip-aware PTS mapping and pixel colors, Undo/Redo, Home/reopen, offline detection, relink with the same Asset ID, and no media copying into the project; Preview gap/clip-without-media/multiple-layer/missing-asset states; Media Pool category/card rendering and content-click isolation; Home Recent Projects and AutoSave text; and AppState-level Saved/Unsaved-changes/Failed/Conflict transitions. The import/relink dialogs and navigation wait/problem dialogs are covered by real-app QA rather than dedicated headless widget tests.

Manual M3-D QA at 1600×900 (2026-10-31, `target/m3d-qa/`, `target/m3d-qa/qa-console.log`, `target/m3d-qa/app-stderr.log`): created a managed project without a destination dialog (Saved, generation 0); imported 1280×720 red `qa-a rouge.mp4` and 640×360 blue `qa-b bleu.mp4`; appended both to V1 at exact ticks `0–240,000` and `240,000–480,000`; observed red PTS 0.000 s at 0 s, red PTS 0.500 s at 0.5 s, blue PTS 0.000 s at the 2 s boundary, blue PTS 1.000 s at 3 s, and blue PTS 1.500 s after rapid scrubs; Undo produced “No active clip” at 3 s and Redo restored Clip 2/Asset 2; AutoSave showed Unsaved changes then Saved; Home showed the real Recent Projects entry; reopening restored both assets/clips and frames; moving the B file offline showed the Offline card and “Media offline” Preview; relinking to the renamed file preserved Asset 2 and returned the blue frame; the final v2 document recorded generation 7, `next_clip_id`/`next_asset_id` 3, the relinked absolute locator, and no copied media. Probe was 22–23 ms; warm 720p decode/seek was 62–114 ms / 73–132 ms (first request 602 ms). The full suite result recorded for this milestone was engine 61, project 34, UI 296, with 440 workspace tests passing, 0 failing, and 2 intentionally ignored private-fixture tests. No commit or push was made.

**Limitations to test/communicate:** AutoSave has no crash-recovery journal and is not crash-proof; conflict recovery offers Retry/Stay plus a separate new-project recovery copy; inactive rename/duplicate are synchronous library metadata operations; audio, playback, compositing, linked-media movement, nested editing, hardware decode, and export are not implemented.

### Current M3-D.1 recovery, conflict and file-safety checks

Automated `capcut-project` tests cover:

- strict recovery envelope round trip with project identity, original path, document/last-saved generations, timestamps, checksum, embedded v2 document, assets, exact ticks and Magnet;
- checksum, identity, generation, version and corrupt-document rejection;
- publisher precedence (never replacing a newer valid recovery with an older one; replacing a corrupt one), bounded size, staged validation and atomic publication;
- recovery worker latest-job ordering, clean Drop, and executor panic containment;
- candidate classification Newer / SameGeneration / Stale / Orphan / Invalid against the current library entries;
- restore preparation as a new ProjectId with internal clip/asset IDs, allocator high-water marks, exact source ranges and Magnet preserved; relative locators rebased to original media without copying or rewriting media;
- guarded discard (paths outside `<library>/.recovery/` are rejected);
- abandoned staging cleanup (old `.project.cutcraft.<pid>.<counter>.tmp` removed; recent staging retained);
- read-only target publication failure preserving the previous valid project;
- persistence worker panic containment;
- serialization/save/recovery timing probes (see `PERFORMANCE.md`).

Automated `capcut-ui` tests cover:

- pure recovery scheduler debounce, saved-generation suppression, newer-generation retention during publication, failure backoff, and no false published watermark;
- AppState startup detection of a newer recovery without opening/overwriting the primary;
- restore creates a new project, opens the editor, preserves clip/asset IDs and Magnet, leaves the original primary byte-identical, consumes the recovery file only after publication, and displays Saved only after success;
- Open Last Saved retains the recovery candidate;
- corrupt recovery appears as a warning with no Restore action; the primary still opens and the invalid file is retained;
- conflict recovery copy creates a new managed project, keeps the original external file and current dirty/conflicted session untouched, appears in Home, and preserves asset/clip identity;
- a second external modification during copy publication does not affect the copy;
- a simulated copy failure leaves the live session, primary file, and library unchanged;
- a blocked recovery worker does not block `update_auto_save_at` (recorded p95 in the test output).

Automated `capcut-media` tests cover hard process timeouts in addition to the existing probe/decode matrix: a normal child succeeds, a hung direct child is terminated at the timeout, and the media-enabled gate fails when tools/fixture generation are unavailable.

**Recovery acceptance criteria:** a newer validated recovery is offered; a restore never overwrites the primary; stale/same-generation candidates are not presented as newer work; corrupt candidates are reported but not applied; a recovery failure leaves the dirty session and last valid files intact; conflict copies use a new ProjectId; and no source media is rewritten.

**Real Windows M3-D.1 QA (2026-10-31, `target/m3d1-qa/`):** force-terminated at editor generation 5 while the primary remained generation 4; relaunch detected the validated candidate and Restore as New Project produced a distinct ProjectId, generation-0 file, 3 preserved clips, same asset/clip IDs and exact ticks, unchanged original primary SHA-256, and consumed recovery only after success. External modification produced Conflict; Save Recovery Copy published a new ProjectId with the same 3 clips while preserving external bytes. Stale generation-3 recovery vs primary generation 4 was ignored and retained; corrupt recovery produced a warning and no Restore; both primaries opened unchanged. A 30-hour abandoned staging file was removed on open, and graceful close/reopen passed. Exact project IDs, names, generations, file paths, stderr frame logs, and screenshots are in `target/m3d1-qa/qa-console.log` and `target/m3d1-qa/m3d1-*.png`. The strict media gate ran 16 tests against FFmpeg 8.1.2 and failed clearly with deliberately missing tool names.

**Final M3-D.1 workspace gate:** 463 passed, 0 failed, 2 ignored (private CapCut fixture tests); engine 62, project 37, UI 305, media integration 16. `cargo fmt`, Clippy `-D warnings`, and `cargo check` passed. Media gate: 16/16. No commit or push.

**Still missing / explicitly unverified:** power-loss durability, full-disk behavior, antivirus/share-mode failure, an OS-level multi-process lock and simultaneous-writer collision proof, parent-directory flush on Windows, and content hashing for media identity. Library scan/open and recovery-candidate validation are synchronous bounded UI operations; AutoSave/recovery writes are worker-backed. FFmpeg timeout covers the direct child, not an arbitrary wrapper process tree. A recovery snapshot cannot exist for edits made before its publication; no zero-data-loss claim is made. See also `MEDIA_IDENTITY.md`.

### Current M4-A / M4-A.1 GPU Preview checks

Preview rendering is verified at three distinct levels. A pass at one level is not evidence for another, and a skip is never reported as a pass.

1. **Headless, no device** (`cargo test -p capcut-renderer` lib tests): RGBA8 validation (dimensions, exact buffer length, 8192/256 MiB bounds), `FrameIdentity` composition, the target→texture format rule, and the deliberately-`None` GPU completion/present fields. These prove the CPU-side contract only.
2. **Device-backed, offscreen** (`crates/renderer/tests/preview_gpu.rs`, 14 tests): creates its own headless wgpu device (the application never creates one; `crates/renderer/src` contains no `Instance`/`request_adapter`/`request_device` call), renders real fixtures through `PreviewRenderer::paint`, and reads the **presented** pixels back. Covers: BGRA memory order, byte-exact opaque passthrough, per-pixel gradient identity, straight-alpha blending against a measured background, non-256-aligned row strides, portrait/landscape non-transposition, resolution change with no stale pixels, generation/source/clip identity, cross-project id reuse, clear-then-repaint, rejection-before-allocation, the sRGB target branch, and `None` GPU timing. When no adapter is available these tests print `SKIP …` and return, so a headless CI green does **not** prove colour correctness.
3. **Real window** (Windows QA below): the actual executable, real decoded MP4 frames, real surface.

Manual M4-A QA at Windows 1600×900, 1210×710, and 1920×1080 (`target/m4a-qa/`, `target/m4a-qa/qa-console.log`, `target/m4a-qa/stderr-gpu.log`): the actual app reused eframe's Vulkan device on an NVIDIA RTX 2080 SUPER (`target=Bgra8Unorm`) and displayed `GPU PREVIEW`. Frames were verified by accessible identity and center pixels: A 1280×720 red at PTS 0.000/1.000 s, B 640×360 blue at the 3 s boundary, portrait 360×640 green, rotated 360×640 red, and 4K 3840×2160 gold; rapid scrubs settled on B PTS 1.500 s. Forced egui fallback displayed the same red frame with `EGUI FALLBACK`. Offline media showed the explicit offline state; Relink returned the blue GPU frame; opening a second project rendered only the new source identity; close/reopen recreated the GPU path and correct red frame. Resizing produced the correct frame at all three sizes. Renderer CPU upload was 0.43 ms at 720p and 3.19 ms at 4K; renderer submit was 0.0007–0.0016 ms; 4K texture was 33,177,600 bytes; 8 reallocations/3 reuses observed. GPU completion/present remain **N/A/UNVERIFIED**. 4K single-frame decode/seek was 263/275 ms and 325/345 ms, not playback. Full gates: **471 passed, 0 failed, 2 ignored** (renderer 4, UI 305, media 16); media gate 16/16 previously recorded and unchanged by M4-A. No commit or push.

**M4-A.1 integration and colour QA** (`target/m4a1-qa/`, driver `target/m4a1_qa.ps1`, fixtures `target/m4a1-qa/media/`, decode reference `target/m4a1-qa/ground-truth.txt`): eight owned synthetic H.264 clips (1280×720 red, 640×360 blue, 3840×2160 gold, mid-gray, non-primary 212/84/30, black→white ramp, 360×640 portrait, rotated) were imported into one managed project and verified on a 16-point grid inside the letterboxed canvas, compared against **FFmpeg's own decoded bytes** rather than the ideal synthetic colour (the H.264 4:2:0 round trip moves saturated primaries by up to 2, e.g. 255→253). Every clip reported `worst-deviation=0` against tolerance 3, the ramp increased monotonically left→right with zero channel separation, and the same frame held at 1210×710, 1600×900 and 1920×1080. Measured canvas geometry: 4K source presented 435×243 px (1.790 vs 1.778 expected) and portrait 179×317 px (0.5647 vs 0.5625), i.e. aspect preserved within ~1 px of rounding. Rapid alternating scrubs settled on the requested clip with `worst-deviation=0`, proving no stale texture was presented. `CUTCRAFT_PREVIEW_RENDERER=egui` produced `EGUI FALLBACK` with the identical frame; the new `CUTCRAFT_PREVIEW_RENDERER=init-failure` seam produced the visible status warning plus the same fallback frame. Offline media reported `Preview status Media offline` and Relink returned the blue GPU frame; close/reopen rebuilt the GPU path; the project file hash was unchanged across graceful close and AutoSave kept returning to `Saved` while frames were being replaced. Alpha edge cases exist only as synthetic renderer inputs (YUV sources carry no alpha), which is what the device-backed suite covers.

**M4-A / M4-A.1 limitations:** single-frame presentation only; no continuous playback, playback clock, audio, export, multi-layer GPU compositor, text/effect/subtitle compositing, YUV/HDR/color-management shader, hardware decode, or device-loss recovery. Upload runs on the UI thread via `queue.write_texture` and is measured; the egui texture fallback remains. A degenerate/zero-size canvas is refused in code (`canvas.width() <= 1.0`) but was only exercised down to 1210×710 at runtime. Shader/pipeline creation failure is not injectable, so only the environment-requested initialization-failure path is verified. Tick timings taken with `CUTCRAFT_RENDER_LOG=1` include the cost of the diagnostic writes themselves (see `PERFORMANCE.md`) and are not production scrub latency.

## 7. Failure and data-integrity tests

Native persistence currently covers invalid JSON; unknown/missing/extra fields; long strings; high track/clip counts; broken links; incompatible schema versions; oversized input; deep JSON; read-only/blocked destination creation; target overwrite refusal; external modification/deletion; staged validation; failed save preserving the previous file; AutoSave save failure preserving dirty state and blocking navigation; external-conflict pause; and older-generation success not clearing a newer dirty generation. The expected outcome is a clear error and no silent loss of the previously valid project. Still to test with real media/export backends: disk full, power loss during write (no AutoRecovery journal exists), locked destination file beyond the current error path, corrupted external media, and multi-process writers.

For exported media, check stream metadata, duration and monotonic timestamps, subtitle timing, and A/V sync with independent tools. Record which codec/backend produced the result.

## 8. Performance testing and reporting

See `PERFORMANCE.md`. Every benchmark report must include:

- Git commit, OS, Rust and backend versions, CPU, GPU, RAM, hardware decode mode.
- Fixture identity, resolution, codec, FPS, track count and subtitle/audio settings.
- Warm/cold start, preview resolution/proxy policy and cache state.
- p50/p95 seek latency, frame presentation rate, late/drop counts and memory.
- Repeated runs and uncertainty; no unqualified 'real-time 4K' conclusion.

Benchmarks are measurements, not CI pass criteria until the hardware test environment and thresholds are formally established.

## 9. Regression rule and CI growth

Every reported bug gets a reproducer and a regression test where technically feasible. As the project grows, add optional checks for dependency licenses, vulnerability advisories, ADR/layer policy and asset provenance, but implement the command first, make it reliable, then require it. Do not declare a fictional `cargo xtask ci` successful before the project provides that command.
