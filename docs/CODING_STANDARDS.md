# CutCraft — Rust Coding Standards

> Standards for code as it is added to the existing Rust 2021 workspace. `AGENTS.md` takes precedence in case of conflict. Rules below distinguish immediate correctness requirements from future architecture goals.

## 1. Code organization

- Each crate owns its domain: `capcut-core` (time, IDs, math, errors); `capcut-format` (CapCut interchange); `capcut-engine` (project, edit commands, playback state); `capcut-media` (media I/O and caches); `capcut-renderer` (GPU textures/composition); `capcut-ui` (egui); the application wires them together.
- Public types should not require UI, decoder, or OS-specific resources unless the crate explicitly owns them.
- Make modules small and organized by responsibility. Prefer readable Rust and typed APIs over a large `utils.rs` or a single global mutable app state.
- Avoid circular dependencies, platform-specific paths in shared types, and persistent state hidden in thread-locals.
- `capcut-engine` currently depends on `capcut-format`; any future format-agnostic restructuring requires an ADR and migration tests.

## 2. Errors and panics

- Prefer `Result<T, E>` with descriptive domain error variants. Propagate with `?`; attach file/stream/operation context at boundaries.
- Never use `unwrap()` or `expect()` for media, external files, dynamic configuration, poisoned locks, optional streams, or user-driven state.
- Placeholder production APIs should return explicit `Unsupported`/`NotImplemented` errors rather than `todo!()` or successful empty values.
- Do not swallow errors simply to keep an animation running. UI-facing errors should be logged and visible without destroying the current project.
- For optional recoverable paths (e.g. missing thumbnail), report a warning and continue only when falling back is semantically valid.
- Tests may use assertions and `expect` to express failure conditions.

## 3. Time, FPS and edit math

**This is a correctness invariant, not a UI convenience.**

- Keep separate types or explicit wrappers for `TimelineTime`, `MediaPts`, `FrameIndex`, `FrameRate`, and `ClockInstant`.
- Use integer/rational time. Choose the canonical timeline representation through an ADR before implementing it; do not silently set incompatible units across crates.
- A rate is a fraction `(numerator, denominator)` with strictly positive denominator; support `30000/1001`, `24000/1001`, `25/1`, and `30/1` correctly.
- Use `i128` checked intermediate multiplication/division for cross-timebase conversion; return `Result` on overflow or invalid ranges. Define and test rounding semantics (floor, ceil, nearest, or toward zero) at each API boundary.
- Derive each frame timestamp from its frame index and rational rate; do not accumulate a rounded per-frame duration over thousands of frames.
- Treat source media PTS and timeline position separately: `source_pts = source_in + mapped_timeline_delta` only when speed and edit mapping warrant it. Never assume media starts at PTS zero or has constant frame duration.
- Represent clip ranges as half-open `[start, end)` and define behavior for zero-length ranges, overlap, reverse/speed changes, and end-of-file.
- `f32`/`f64` are acceptable for GUI pixel coordinates and shader math, **not** the authoritative storage of edit positions.

## 4. Ownership and data lifetime

- Never expose a borrowed FFmpeg/Media Foundation pixel pointer beyond its documented decoder/frame lifetime.
- Every decoded frame must identify format, width/height, plane strides, crop/rotation if relevant, media PTS/timebase, and ownership model.
- Prefer RAII to manual resource cleanup. Enforce release order for GPU resources, native decoder sessions, worker threads and file handles.
- Do not create unbounded queues of `Arc<Frame>`; `Arc` manages ownership, not memory capacity.
- Be explicit about CPU frames versus GPU textures. No assumption of zero-copy without a verified compatible API and synchronization contract.
- Clear stale seek tasks and prevent old results from replacing newer playhead state.

## 5. Concurrency and responsiveness

- UI event handlers should be quick. Use dedicated bounded worker queues for probe/decode/thumbnail/export work.
- Prefer message passing to broad shared mutable locks; never hold a project/UI mutex while awaiting decode, network, disk, or GPU work.
- Every asynchronous request requires a lifecycle: initiated, running, completed, cancelled, failed; provide cancellation and deterministic shutdown where feasible.
- Avoid busy-waiting. Document the master clock and any time-based wait behavior in the playback module once implemented.
- Treat poison/panic handling as a subsystem boundary decision; errors must surface and not leave the UI permanently stuck in a running state.
- Maintain monotonic frame-request generation IDs so a slow old seek cannot overwrite a newer result.

## 6. Commands, project model and undo/redo

- **FUTURE CONTRACT:** every project-changing operation is a command with validated parameters and a reversible action or immutable-snapshot transaction.
- UI widgets emit intentions; engine commands own mutation and invariants. Never let the preview or track widget mutate persistent project structures directly.
- Ensure stable clip/track identity and deterministic selection. Splitting creates appropriate new identities; moving must not duplicate an existing clip identity.
- On failed operations, retain project, selection, undo and redo state unless the command explicitly defines a partial transaction.
- Document serialization format/version, load/migrate behavior, and atomic save when implementing native project persistence.

## 7. GPU, shaders, and image formats

- Choose and pin a GPU API through an ADR. `wgpu` is a **proposal**, not an installed dependency or implemented renderer.
- Document color matrices, full/limited range, transfer functions, premultiplication, and pixel order. Test both RGBA and common YUV source formats when supported.
- Respect per-plane strides and padded row alignment; width × bytes-per-pixel is not always the physical stride.
- Never upload from a dropped or mutable buffer without explicit synchronization and ownership.
- Recreate or resize device-dependent surface resources safely; handle device-loss paths without losing project state.
- Distinguish decoded, composited, submitted, and actually presented frames in logs and metrics.

## 8. `unsafe`, native APIs and dependencies

- `unsafe` is allowed only when justified by an approved design for low-level FFI/GPU/native integration, localized to a small documented module.
- Each unsafe block requires a `// SAFETY:` explanation covering pointer validity, aliasing, alignment, threading, and lifetime; keep safe public APIs and tests around it.
- Research and document dependency version/MSRV, supported Windows targets, native binaries, runtime discovery, and applicable licensing before adding it.
- FFmpeg bindings are an option, not an entitlement to ship all codec libraries. Decide GPL/LGPL implications and dynamic/static linking in an ADR before release.
- Do not force an unrelated dependency upgrade or edition change as part of a small feature.

## 9. Formatting, API quality and testability

- Run the repository's `rustfmt` configuration and Clippy gate; follow Rust naming conventions.
- Prefer immutable values and explicit constructors that validate domain invariants.
- Public API changes include doc comments for meaningful invariants, errors, and units.
- Test deterministic logic without a GUI/GPU. Inject a fake clock, fake decoder or frame source when that makes tests reliable.
- Avoid optimization claims without a benchmark. Profile before caching or adding parallelism.
- Do not leave dead commented-out code, fake success paths, or unexplained magic numeric constants.

## 10. Comment and documentation style

Comments explain **why and what assumptions hold**, not simply repeat the line below. Place architecture tradeoffs in ADRs. Keep example code accurate for the current version of the repository; mark pseudocode and planned interfaces clearly. Do not assert that a class, integration or test exists until it can be found in the tree and verified.
