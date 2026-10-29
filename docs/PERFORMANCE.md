# CutCraft — Preview and Playback Performance Contract

> **Target**, not measured capability: practical preview of a TikTok-style timeline containing one 3840×2160/30 FPS video layer, timed subtitles, and audio on a suitable Windows computer. No decoder, frame scheduler, GPU renderer or performance harness is implemented in the current scaffold.

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
| `texture_upload_ms` | CPU → GPU staging/upload work, when applicable |
| `compose_ms` | Subtitle/layer composition |
| `present_interval_ms` | Actual output/display cadence, if observable |
| `late_frames`, `dropped_frames` | Explicit selection policy outcomes |
| `audio_video_offset_ms` | Signed A/V difference under documented clock policy |
| `ui_input_latency_ms` | Responsiveness while media operations run |

Do not fill unavailable measurements with zeros: use `N/A` and explain why. Separate in-process instrumentation from OS/GPU external profiler estimates.

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
