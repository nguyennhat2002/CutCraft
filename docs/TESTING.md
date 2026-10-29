# CutCraft — Testing, CI and Verification

> **Current status:** The repository contains a GitHub Actions configuration and a tested M1 egui/eframe UI shell with Home, Projects, Settings, Editor, and Export settings screens. Its media, engine, renderer, playback, and encoding systems remain scaffolds. Tests and benchmarks described here are future requirements unless explicitly recorded as executed.

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

### Current UI shell checks (M1)

Automated (`cargo test -p capcut-ui`):

- headless Home, Projects, Settings, Editor, and Export dialog rendering at
  960×600, 1280×800, and 1440×900 as applicable;
- navigation transitions, temporary-session preservation, and Home/Resume behavior;
- export setting validation and form persistence across dialog close/reopen;
- window title, default size, minimum inner size, and resizability;
- stable idle frames with **no** follow-up repaint for Home, Editor, and Export;
- timeline zero-time playhead/label visibility regression;
- timecode formatting, ruler tick/scale bounds, and 16:9/9:16 preview fitting;
- status-notice behavior, About dialog, and disabled-action states.

Manual Windows procedure:

1. Run `cargo run -p capcut-editor` with no media present.
2. Confirm the app starts on Home with no saved projects and a temporary-session caption.
3. Use Create Project; confirm the Editor shows the project bar, library tabs,
   Preview, Inspector, and Timeline.
4. Toggle 16:9 and 9:16; confirm the canvas preserves aspect and the resolution
   indicator updates (`1920×1080` / `1080×1920`).
5. Use File → Back to Home, then Resume editing; confirm the same temporary
   session card returns to the Editor.
6. Open Export; confirm filename, folder, resolution, frame rate, H.264, MP4,
   bitrate preset, and audio controls are present. Confirm Export is disabled and
   no output size/progress is shown. Cancel and confirm the Editor returns.
7. Resize between 960×600 and 1440×900; confirm no overlapping controls, panels
   scroll where needed, and the zero-time playhead remains visible.
8. Leave idle and confirm the process remains responsive.
9. Close with the window button; verify a normal exit.

Result recorded 2026-10-29 on Windows: Home at startup, Create Project → Editor,
Home → Resume, 16:9 → 9:16 (1080×1920), Export modal, and Cancel → Editor were
verified. Editor and Export layouts were inspected at 1440×900 and 960×600, and
window close returned exit code 0. Because the interactive desktop session was
locked, window content was inspected with `PrintWindow`, and real buttons were
invoked via Windows UI Automation. `File → Exit` was not separately clicked.
CI on Linux/macOS was not run.

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

## 7. Failure and data-integrity tests

When persistence and format imports are implemented, include: invalid JSON; unknown/missing fields; long strings; high track/clip counts; cyclic references; incompatible schema versions; disk full; read-only destination; interrupted save; and corrupted cache. The expected outcome is a clear error and no silent loss of the previously valid project.

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
