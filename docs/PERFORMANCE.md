# CutCraft — Preview and Playback Performance Contract

> **Target**, not measured capability: practical preview of a TikTok-style timeline containing one 3840×2160/30 FPS video layer, timed subtitles, and audio on a suitable Windows computer. M3-C implements correctness-first single-frame H.264 probing/decode and display with a bounded process-backed worker. M3-D adds bounded media validation and generation-safe AutoSave. M3-D.1 adds bounded recovery snapshots, conflict preservation, process timeouts, and a measured serialization/save/recovery probe. M4-A adds a renderer-owned single-layer wgpu Preview with measured CPU upload/renderer-submit costs; GPU completion/present timing is not observable. The strict media gate is separate from ordinary workspace runs. There is no playback scheduler, audio, GPU compositor, hardware decode, AutoRecovery benchmark harness, or realtime playback claim yet.

## 1. User-visible outcomes

A responsive editor should accept input while loading media, scrubbing and playing. Playback rate should track the intended media clock within the documented policy. Correctness is more important than artificially high decoded-frame counts.

**Early vertical slice:** open MP4 → request one frame → show it → seek to a new frame → play/pause. Build 1080p/30 correctness first, then benchmark 4K/30 and optimize bottlenecks proven by traces.

### Frame budget

For 30 FPS output, the nominal display interval is `1000 / 30 = 33.333… ms`. This is a **total pacing budget**, not a promise that each individual decode, render, or I/O operation finishes under 33 ms. Pipeline stages may overlap, and buffering, display refresh and audio-clock sync alter deadlines. A 60 Hz UI may render at a different cadence than 30 FPS media.

A single 3840×2160 RGBA8 CPU frame requires:

`3840 × 2160 × 4 = 33,177,600 bytes ≈ 31.64 MiB`.

Eight such frames occupy approximately `253.13 MiB` **before** decoder references, audio, staging buffers, thumbnails, textures or allocator overhead. Avoid unbounded caches; hardware YUV surfaces may have different sizes and lifetimes.

## 2. Separate the pipeline stages

```text
Media file / demux
      ↓
Decode (CPU or platform hardware; future)
      ↓
Owned frame / GPU handle + source PTS
      ↓
Bounded frame cache / request queue
      ↓
Timeline mapping and frame scheduler
      ↓
Composition / subtitle overlay
      ↓
GPU surface submission and presentation
```

This diagram is **CutCraft's proposed architecture**, not an end-to-end internal CapCut call graph confirmed by reverse engineering.

Measure each stage independently; 'decoded' is not 'displayed'. Track a request ID through probe, decode, scheduling, texture upload, compositor, submit and present if platform APIs expose those timestamps.

## 3. Clock and scheduling policy — design requirements

- Use monotonic time for playback deadlines; do not use wall-clock UTC for frame timing.
- Store source PTS and timebase in the frame, and convert rationally to timeline positions.
- Allow variable frame rate: map playhead time to the appropriate frame by PTS, not by `index = time × assumed_fps` on every source.
- Use a bounded queue with a documented maximum age and memory budget.
- On seek, advance a generation token and discard old results before display.
- When playback is late, choose a documented policy that prefers freshness (skip an already-stale preview frame) without losing the authoritative playhead position; never silently skip export frames.
- Keep thumbnail generation and export from starving interactive preview; establish worker priorities or separate pools when evidence justifies them.
- With audio, decide a master clock in an ADR and record synchronization metrics; do not assume `VideoClockTime` in a reverse-engineering report determines our policy.

## 4. Cache and memory strategy

The cache needs a clear unit and owner:

- CPU decoded-frame bytes, GPU texture bytes, and compressed packet buffers are different budgets.
- Key cached frames by asset identity, decoder configuration, stream, source PTS and any relevant transformation; invalidate on media replacement or incompatible settings.
- Use a bounded LRU or comparable policy and record hits, misses and evictions.
- Implement deduplication only when lifetime semantics and key equivalence are proven.
- Constrain queue depth and prefetch based on measured decode cost, device memory and seek behavior.
- Prefer dropping stale prefetch rather than allocating unlimited textures.
- Keep a safe CPU fallback when future hardware decoding is unavailable; downscaled preview/proxy playback is a legitimate quality strategy, but report the scale used.

Do not claim 'zero-copy' until CPU/GPU ownership, texture format, device interop and synchronization are verified in code and metrics.

## 5. Instrumentation — initial metric names

| Metric | Interpretation |
|---|---|
| `media_probe_ms` | Open and discover streams/metadata |
| `first_frame_ms` | Import/request to frame usable by renderer |
| `seek_latency_ms` | New seek request to matching presented frame |
| `decode_frame_ms` | Decode work per output frame, excluding presentation |
| `frame_queue_depth` | Pending/ready frames, bounded |
| `cache_cpu_bytes`, `cache_gpu_bytes` | Measured ownership footprint |
| `cache_hit_ratio` | Useful reuse for same request profile |
| `texture_submit_ms` | CPU-side texture delta submission; M3-C does not observe GPU completion |
| `texture_upload_ms` | Future measured CPU → GPU staging/upload completion, when applicable |
| `compose_ms` | Subtitle/layer composition |
| `present_interval_ms` | Actual output/display cadence, if observable |
| `late_frames`, `dropped_frames` | Explicit selection policy outcomes |
| `audio_video_offset_ms` | Signed A/V difference under documented clock policy |
| `ui_input_latency_ms` | Responsiveness while media operations run |
| `autosave_debounce_ms` | Configured quiet period and observed edit→write delay |
| `autosave_write_ms` | Snapshot serialize + stage + verify + atomic replace latency |
| `autosave_pending_jobs` | Latest-job coalescing depth (target: ≤1 pending, ≤1 in flight) |
| `probe_queue_depth` | Import/validation requests awaiting the single probe worker |
| `asset_validation_ms` | Locator resolve + probe verification per project revision |

Do not fill unavailable measurements with zeros: use `N/A` and explain why. Separate in-process instrumentation from OS/GPU external profiler estimates.

### M3-C measured instrumentation

M3-C exposes `probe_latency_ms`, `last_seek_latency_ms` (request dispatch to accepted frame), adapter `decode_frame_ms`, CPU `texture_submit_ms`, request count, stale-result count, bounded outstanding requests (maximum 1), and decoded bytes. `CUTCRAFT_MEDIA_LOG=1` writes concise events and timings to stderr.

On the QA Windows machine with a local Gyan FFmpeg 8.1.2 CLI (not bundled), generated 1920×1080/30 H.264 fixtures produced probe times of 24–31 ms. Request-to-display latency was roughly 146–880 ms per seek including process startup and seek/decode: first frames measured ~836/880 ms and warm frames ~146–201 ms. Adapter decode time was roughly 129–188 ms. One 3840×2160 H.264 frame decoded in 302 ms with 328 ms request latency. `texture_submit_ms` was effectively 0 ms because egui records CPU-side delta submission, not GPU completion. These are individual functional QA measurements, not p50/p95, not playback throughput, and not a realtime/decode-rate claim.

### M4-A measured GPU Preview instrumentation

QA hardware/backend: NVIDIA GeForce RTX 2080 SUPER, driver `NVIDIA`, eframe/egui-wgpu 0.37, wgpu 30.0.1, Vulkan backend, target format `Bgra8Unorm`, local Windows desktop. Fixtures: 1280×720 red, 640×360 blue, 360×640 portrait green, 640×360 rotated red, 3840×2160 gold. Preview canvas was approximately 430 px wide at 1600×900.

| Observation | Measured value |
|---|---|
| 720p `frame_upload_cpu_ms` (CPU enqueue) | 0.43 ms |
| 4K `frame_upload_cpu_ms` (CPU enqueue) | 3.19 ms |
| `renderer_submit_ms` (CPU render-pass recording) | 0.0007–0.0016 ms |
| 720p `gpu_texture_bytes` | 3,686,400 |
| 4K `gpu_texture_bytes` | 33,177,600 |
| 4K `cpu_staging_bytes` | 33,177,600 |
| Reallocations / texture reuses in source sequence | 8 / 3 |
| 4K first measured decode / seek-to-accepted-frame | 263 ms / 275 ms |
| 4K second measured decode / seek-to-accepted-frame | 325 ms / 345 ms |
| `gpu_upload_completion_ms` | **N/A** (not observed) |
| `preview_present_ms` | **N/A** (not observed) |
| Continuous playback FPS | **N/A** (not implemented) |

The upload metric measures the CPU-side `queue.write_texture` call, not GPU completion. The submit metric measures command recording in the callback, not presented frames. One 4K frame is a diagnostic, not a realtime or 4K/30 playback claim. The egui fallback path uses the existing `TextureHandle` upload and was verified visually with the same red fixture.

### M4-A.1 measured Preview cost and tick attribution

Same QA hardware/backend as M4-A (Vulkan, RTX 2080 SUPER, `Bgra8Unorm`). All values below are **CPU-side** measurements taken inside the running app with `CUTCRAFT_RENDER_LOG=1`. `gpu_upload_completion_ms` and `preview_present_ms` stay `None` in every sample: this build has no timestamp query or present fence, so GPU execution and presentation time remain **unmeasured**, and no number here may be read as GPU completion.

Renderer cost per accepted frame (`frame_upload_cpu_ms` is the `queue.write_texture` enqueue, `texture_allocation_cpu_ms` is texture + view + bind-group creation):

| Frame | Texture bytes | uploads | upload p50 / p95 / max (ms) | allocation p50 (ms) |
|---|---|---|---|---|
| 640×360 | 921,600 | 13–16 | 0.202 / 0.228 / 0.238 | 0.21 |
| 1280×720 | 3,686,400 | 6–18 | 0.465 / 0.482 / 0.536 | 0.26 |
| 3840×2160 | 33,177,600 | 1–17 | 3.251 / 3.379 / 8.356 | 0.22 |

Render-pass recording inside the egui-wgpu callback (`renderer_submit_ms`) measured 0.0007–0.0012 ms. Peak GPU texture and CPU staging bytes both reached 33,177,600 during 4K replacement; `uploads_skipped` grew by hundreds of ticks (the identity cache prevents re-staging an unchanged frame) with `upload_rejections: 0` throughout.

UI tick attribution (`draw_ms` = state update + egui layout/paint inside the app callback; `state_ms` is the app-state phase, `shell_ms` is `show_shell`):

| Condition | samples | draw p50 / p95 (ms) | state p50 (ms) | shell p50 (ms) |
|---|---|---|---|---|
| Pointer motion, frame already uploaded | 108 | 2.893 / 3.340 | 0.086 | 2.802 |
| Tick where a decoded 720p frame arrived | 16 | 88.126 / 89.014 | 84.689 | 2.994 |
| AutoSave edit storm, all ticks | 263 | 2.846 / 27.252 | 0.092 | 2.745 |
| AutoSave edit storm, frame-arrival ticks | 10 | 90.511 / 93.233 | 87.184 | 3.098 |

Sub-phase trace on frame-arrival ticks: `poll_ms` p50 13.496 (receiving the owned frame from the decoder worker), `sync_ms` p50 0.032, `gpu_ms` p50 61.601, `fallback_ms` p50 0.000. Since the renderer's own enqueue on the same tick is 0.17–3.5 ms, the bulk of `gpu_ms` is **not** `write_texture`.

**Measurement caveat, established by experiment:** adding one unconditional trace line per tick raised `state_ms` from 0.086 ms to 10.9 ms (n=74–108), i.e. a synchronous write of one short log line to the redirected stderr file costs ~11 ms on this machine. The frame-arrival path additionally prints a ~600-character `Debug` metrics line. The ~85 ms arrival-tick figure is therefore dominated by diagnostic logging that only exists while `CUTCRAFT_RENDER_LOG` is set, and must not be read as production scrub cost. The residual inside `gpu_ms` after subtracting the estimated log cost is not decomposed; re-measuring with logging off (or a cheaper log format) is a follow-up before any playback budget is claimed.

Also unmeasured: continuous playback FPS, GPU completion, present latency, and behavior under a concurrent decode storm — none of those paths exist yet.

### M3-D.1 measured save/recovery overhead

A synthetic-project probe (`cargo test -p capcut-project --test recovery_performance -- --nocapture`) measures 20 samples per scenario on the QA Windows machine using local NTFS temp paths. Values are milliseconds; p50/p95 are simple nearest-rank samples, not a production distribution.

| Scenario | Assets / clips | Serialized document | Recovery envelope | Serialize p50/p95 | Staged save p50/p95 (incl. serialize/write/verify) | Recovery publish p50/p95 | Recovery validate p50/p95 |
|---|---:|---:|---:|---:|---:|---:|---:|
| Empty | 0 / 0 | 577 B | 1,198 B | 0.013 / 0.014 | 1.700 / 1.935 | 1.933 / 2.303 | 0.101 / 0.124 |
| Small | 2 / 2 | 3,260 B | 4,206 B | 0.065 / 0.070 | 1.946 / 2.119 | 2.490 / 2.677 | 0.286 / 0.367 |
| Medium | 20 / 100 | 68,267 B | 76,424 B | 1.382 / 1.798 | 7.089 / 7.893 | 14.489 / 14.821 | 3.773 / 3.918 |
| Large synthetic | 100 / 1,000 | 598,209 B | 663,948 B | 11.556 / 12.731 | 47.159 / 47.798 | 114.818 / 214.126 | 66.978 / 68.244 |

Interpretation and limits:

- Recovery publication includes serialization, staging, staged re-read/validation, and atomic replacement; the 1,000-clip p95 of ~214 ms is a background worker cost, not a UI stall.
- The recovery envelope embeds the v2 document as a JSON string; its size is larger than the document because of escaping. Project files and recovery files are bounded independently (32 MiB project cap; recovery cap is 2× that plus 64 KiB).
- Recovery validation is a full strict parse plus engine restore. Directory scan is bounded to 128 candidates and was not separately timed; the single-artifact validation figures above exclude read-dir overhead. Invalid/stale candidates are rejected before exposure.
- A controlled blocked-recovery-worker test measures `update_auto_save_at` p95 at ~0.001 ms across 50 calls, proving the tick does not wait for recovery I/O. This is a headless tick measurement, not a full GPU-present frame-time claim.
- AutoSave queue depth is bounded by one pending latest job and one in-flight job; recovery uses a separate worker with the same one-slot policy. The probe does not measure real app paint latency while saving.
- AutoSave writes were not profiled as a separate p50/p95 in the real app; the staged-save figures above are the closest measured write/verify cost.

No playback, GPU composition, audio, power-loss, or hardware-decode performance is measured. GPU presentation metrics remain **N/A**.

### M3-D measured instrumentation

M3-D adds `source_opens`, frame requests, decode/seek timings, texture uploads, stale-result count, and bounded outstanding requests to the clip-aware Preview, plus `probe_requests`, `validation_requests`, and import/relink success counts for the bounded probe queue. It also counts source and asset identity on every request/frame so stale results can be proven rather than inferred. AutoSave exposes scheduler state (dirty, in flight, current/persisted generation, next deadline), worker metrics, and save failures; write latency is not yet instrumented as a percentile.

Real QA at 1600×900 used 1280×720 and 640×360 H.264 sources with a local FFmpeg CLI. Import/validation probe times were 22–23 ms. After source open, 720p decode was 62–114 ms and request-to-accepted-frame seek was 73–132 ms; the first request after opening a source measured 602 ms including process startup. The red/blue center-pixel checks and exact PTS values (`0`, `0.5`, `1.0`, `1.5 s`) verify mapping, not throughput. The managed project file was 3,429 bytes; no numeric AutoSave write-latency percentile or startup benchmark was recorded. These remain functional measurements, not playback or 4K performance claims.

## 6. Reproducible benchmark scenarios

| Scenario | Source | Purpose |
|---|---|---|
| P1 | 1080p/30 H.264, one video track | Correctness and baseline latency |
| P2 | 4K/30 H.264, one video track | Main decode/preview performance |
| P3 | 4K/30 + subtitle overlay + audio | First practical user target |
| P4 | 30000/1001 FPS and VFR samples | Timestamp and drift correctness |
| P5 | Rapid back-and-forth seek (cold/warm cache) | Cancellation, seek p95, stale-frame safety |
| P6 | Long repeated playback with resizing | Leak detection, steady memory, device/surface reuse |
| P7 | Hardware decode off vs on (if implemented) | CPU fallback and acceleration benefit |

For every scenario, record OS build, CPU/iGPU/dGPU, driver, RAM, decode backend, source codec/profile/pixel format, preview scale, power mode, and Git commit. Use original/generated test assets. Run multiple times and distinguish first-run costs from warm steady state.

## 7. Proposed acceptance measurements

At a minimum report p50/p95 seek latency, delivered/presented FPS, late/drop ratio, UI latency, peak RAM, peak GPU memory estimate, and any playback A/V offset. Acceptable numeric thresholds must be approved and tied to a known hardware baseline after an initial measurement run. Never declare 4K/30 supported based solely on a successful decode of a 4K frame.

For an early functionality milestone, a correct 1080p/30 preview with slower 4K/downscaled fallback is preferable to an unstable 'native 4K' checkbox.

## 8. Optimization order

1. Prove timestamp/frame correctness and track expensive steps.
2. Remove blocking UI operations and unbounded allocation.
3. Fix unnecessary pixel copies or conversions identified by profiling.
4. Add bounded cache/prefetch and smarter seek cancellation.
5. Introduce GPU texture reuse and optimized presentation as measured needs demand.
6. Consider optional hardware decode and zero-copy only after integration and license/design review.
7. Re-run the original fixtures; optimization must not regress video, subtitle timing or error recovery.

## 9. Reporting template

```text
Commit / Windows build / Rust version:
CPU / GPU / driver / RAM:
Media file, codec, dimensions, FPS, format:
Decoder backend and hardware acceleration mode:
Preview scale, subtitle/audio tracks, cache settings:
Warm/cold runs and sample count:
p50/p95 seek, first-frame latency:
Presented FPS, late/drop ratio, UI latency:
CPU peak/RAM peak/GPU memory estimate:
Observed bottleneck, correctness caveats, next experiment:
```
