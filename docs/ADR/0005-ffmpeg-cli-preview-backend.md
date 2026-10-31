# ADR-0005: Local FFmpeg CLI Backend for M3-C Probing and Frame Decoding

- Status: Accepted for M3-C; media binding superseded by ADR-0006 (M3-D)
- Date: 2026-10-31

> The runtime-discovered FFmpeg CLI adapter, time-domain mapping, PTS selection, resource bounds, and single-worker decode architecture remain current. The session-only clip binding described below was replaced in M3-D by persistent media assets and clip-aware Preview (ADR-0006).

## Context

M3-C needs a trustworthy Windows path from a real local H.264 MP4 to a real decoded frame in the existing Preview panel. The repository had no media backend, no bundled codec libraries, and no assumption that FFmpeg was installed. The milestone requires:

- exact probe metadata, including rational timebase, stream start, rotation, and VFR hints;
- presentation-timestamp-based frame selection rather than keyframe or DTS substitution;
- asynchronous decoding that never blocks egui;
- bounded requests, memory, workers, and stale-result suppression;
- no undocumented binary downloads, hardcoded developer paths, or committed personal media.

The goal is correctness and replaceability, not playback, audio sync, hardware decode, compositing, export, or a 4K realtime claim.

## Candidates

| Candidate | Strengths | Costs and risks for this milestone |
|---|---|---|
| FFmpeg libraries via Rust bindings (`ffmpeg-next` or similar) | In-process demux/decode, lower per-request overhead, direct PTS/timebase APIs | Native build/link setup, unsafe FFI boundary, library version/build-matrix management, redistribution and LGPL/GPL configuration review, packaging DLLs, no library assumed present |
| Windows Media Foundation | OS component, H.264 support, no bundled decoder library | COM/IMF async lifetime complexity, Windows-specific adapter and tests, rotation/VFR/seek details still need explicit handling, larger initial implementation |
| Local FFmpeg executable adapter | No native linking or unsafe FFI in CutCraft; actual demux/decode; JSON probe and raw RGBA output; can be replaced behind the same API; no binary bundled | Process spawn cost per request, external prerequisite, output parsing, no fine-grained decode cancellation, unsuitable as a playback engine |

## Decision

M3-C uses a **runtime-discovered FFmpeg command-line adapter** inside `capcut-media`:

1. `ffmpeg` and `ffprobe` are discovered from `CUTCRAFT_FFMPEG` / `CUTCRAFT_FFPROBE`, otherwise from `PATH`. No binary path, download, or version is embedded in the application.
2. The adapter is isolated behind a backend-independent request API:
   - probe returns `MediaProbe`;
   - decode requests carry source identity, generation, stream index, rational target timestamp, and return owned RGBA `DecodedFrame` values with actual PTS.
3. `PreviewEngine` owns one named worker thread, one latest-request command slot, and one result slot. UI calls `poll` and never waits for probe or decode. Results carry `source_id` and `generation`; the UI ignores obsolete results.
4. FFprobe supplies stream metadata and the actual frame PTS list near the requested time. FFmpeg seeks to a decode start, decodes forward, and returns one raw RGBA frame. The displayed timestamp comes from `showinfo` presentation time, never packet DTS. A PTS mismatch is an error.
5. Frame selection is: greatest actual presentation PTS less than or equal to the requested target; if no frame precedes the target, the earliest available frame. The keyframe used for seeking is not treated as the target frame.
6. When a frame has no usable `best_effort_timestamp` or `pts`, probing/decoding returns a typed `MissingPresentationTimestamp` error rather than inventing a timestamp.
7. All process output and frame sizes are capped. The adapter accepts H.264 video streams only; other codecs return `UnsupportedCodec`.

## Time and clip mapping

The three time domains remain distinct:

1. `TimelineTicks` at 120,000 ticks/second;
2. clip source time (`SourceTicks`) from the engine's `SourceMapping`;
3. decoder PTS in the probed stream timebase.

For M3-C the UI maps:

```text
source_ticks = clip.source_used.start + (playhead_ticks - clip.range.start)
decoder_ticks = round_nearest(source_ticks * timebase.denominator
                              / (120000 * timebase.numerator))
decoder_ticks += stream_start_ticks
```

All arithmetic is checked `i128` with explicit i64 conversion. `TimelineTicks` are never cast directly to FFmpeg timestamps. Speed other than 1/1, unsupported clip source mapping, or missing source data fails explicitly. Clip ranges are half-open: the exact clip end has no frame.

## Binding and persistence

`.cutcraft` v1 has no media path field. M3-C therefore uses an **explicit session-only binding** from one local video to one selected main-track video clip. The binding:

- is not written to `.cutcraft`;
- does not mutate the project or dirty the document;
- is cleared when its clip disappears or the project changes;
- is visible as `SESSION PREVIEW` and a one-source/no-compositing disclaimer;
- requires explicit Open Media → Probe → Bind actions and never attaches media silently to synthetic QA clips.

The native QA fixture keeps its synthetic clips; real media is attached only through this explicit mechanism.

## Resource bounds

| Limit | Value |
|---|---|
| Media file size | 64 GiB |
| Probe/frame-query stdout | 4 MiB |
| Frame query stderr | 256 KiB |
| Decode stderr | 1 MiB |
| Decoded RGBA frame | 256 MiB |
| Frame dimension | 8192 each side |
| Frame pixels | 33,554,432 |
| Worker threads | 1 decode worker; bounded per-process pipe readers |
| Pending frame commands | 1 latest-request slot |
| Preview frames/textures | 1 owned CPU frame and 1 reused egui texture |

## Licensing and distribution

CutCraft is not bundling or shipping FFmpeg in M3-C. The local development machine uses a Gyan FFmpeg 8.1.2 **full GPL** build, invoked as an external executable.

Implications:

- The M3-C repository adds no FFmpeg binary, source, or library; ``Cargo.lock`` does not gain an FFmpeg linkage dependency.
- Shipping or bundling any FFmpeg build later requires a separate distribution decision: exact build configuration, LGPL vs GPL obligations, source/notice requirements, codec patent considerations, and legal review.
- A build using `--enable-gpl` is not interchangeable with an LGPL-only build. Do not describe a future bundled build as LGPL merely because the application is MIT/Apache-2.0.
- Invoking a user-supplied executable at arm's length is the current integration; this ADR makes no broader legal claim.
- The executable path is configuration/environment, not project data.

## Consequences

- Correctness is established without unsafe FFI or native linking, but each seek starts a process. Real QA seek-to-frame latency was roughly 146–880 ms for 1080p fixtures (first frames ~836/880 ms; warm frames ~146–201 ms) and about 328 ms for one 4K frame on the QA machine. This is **not** playback and does not establish 4K realtime capability.
- Initial decode latency includes process startup; the UI remains event-driven while the worker runs.
- Actual GPU upload completion is not observable through egui. Reported `texture_submit_ms` measures CPU-side texture submission only, not GPU completion.
- Old process work cannot be interrupted mid-decode. Its result is bounded and discarded by source/generation checks, so it cannot replace a newer Preview frame.
- The adapter can be replaced by a library or Media Foundation implementation without changing the UI-facing request/result identity model, provided PTS, ownership, and error semantics remain equivalent.
- Audio is probed for presence only. There is no audio decode, sync, playback, compositor, transition, or export path.

## Validation

Automated:

- generated H.264 fixtures for 30 fps, 30000/1001, short and long GOP, B-frames, VFR, nonzero start, rotation, unsupported codec, missing/corrupt/truncated files, and probe audio presence;
- rational source-to-PTS conversion, trim/exclusive-end mapping, media-end mapping;
- 1080p and 4K single-frame decode bounds;
- stale generation/source rejection, latest-request behavior, decoder error recovery, source switching, and frame ownership;
- UI texture reuse, empty/exclusive-end fallback, binding validation, explicit binding without project mutation, and asynchronous open.

Real Windows QA (`target/m3c-qa/`, `target/m3c-media-qa.log`, `target/m3c-qa/media-events.log`, app stderr log):

- probe metadata for 1920×1080/30, 1280×720/30, 1920×1080 30000/1001, rotated 640×360, VFR, and 3840×2160;
- playhead 0 → red frame, 1 s → green frame, 2 s → blue frame, then backward to 1 s;
- eight rapid ruler positions produced stale generations 6–11 that were ignored; the final generation 12 displayed PTS 4.100 s and magenta;
- exact clip end showed `End of bound clip` with no frame;
- switching 1080p → 720p → 1080p → 4K displayed only the new source's frames;
- VFR request at 0.5333 s displayed actual PTS 0.500 s;
- unsupported MPEG-4 and a truncated MP4 produced typed errors and no frame.
