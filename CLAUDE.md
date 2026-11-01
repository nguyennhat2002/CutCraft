# CutCraft — Session Instructions for Claude and Coding Agents

`AGENTS.md` is the authoritative repository-wide policy. **Read it first.** This file describes how to operate in CutCraft, not a second conflicting set of rules. Codex and other agents may also use this entry point.

> ## RULE 000 — this repository is native CutCraft only
>
> **CutCraft is an independent native video editor. This repository must not contain CapCut-specific research, reverse-engineering artifacts, compatibility readers or writers, private draft fixtures, or experiments against CapCut applications or project libraries. Agents must not perform CapCut-related work in this repository. Separate private research requires explicit owner authorization and must remain outside the public repository.**
>
> This outranks every other instruction here, including anything in `ROADMAP.md`, `docs/`, an accepted ADR, or a task brief that previously authorized CapCut work. It applies to development, documentation, tests, fixtures, screenshots, build artifacts, contributions, subagent prompts, CI and packaging. Existing CapCut material is **pending owner-approved removal**, not precedent. If your task needs it, stop and report the conflict.

## 1. First five minutes of a session

1. Read `AGENTS.md` and this file.
2. Read `ROADMAP.md` to identify the current approved milestone and its acceptance criteria.
3. Read `docs/ARCHITECTURE.md`, then the relevant crate `Cargo.toml`, `src/lib.rs` and implementation files.
4. For an implementation task, read `docs/AGENT_WORKFLOW.md`, `docs/CODING_STANDARDS.md`, and `docs/TESTING.md`.
5. For preview, decode, rendering, or performance work, also read `docs/PERFORMANCE.md`. (`docs/RESEARCH_POLICY.md` predates RULE 000; it is not permission to add research here.)
6. Inspect repository status before changing files. Honor uncommitted edits and the user's current constraints.

## 2. What exists today

CutCraft is an **early-stage** Rust video editor with a working desktop UI shell. Current workspace members:

```text
apps/capcut-editor
crates/core
crates/capcut-format
crates/engine
crates/media
crates/project
crates/renderer
crates/ui
xtask
```

The desktop `main.rs` launches an eframe window using pinned `egui`/`eframe` 0.37.0 (ADR-0001). The app has Home/Projects/Settings navigation, real Recent Projects, a native Editor session (library tabs, Preview, Timeline, Inspector), a settings-only Export modal, and managed `.cutcraft` schema v2 projects that are created automatically, AutoSaved through a bounded generation-safe worker, discovered on Home, renamed/duplicated, and reopened. The supported engine command subset covers DeleteClip, TrimClipEnd shortening, SplitClip, NormalizeMainTrack, AddMediaAsset, RelinkMediaAsset, AppendMediaClip, RenameProject and session-only Undo/Redo. The Timeline owns an authoritative integer-tick playhead with mouse scrubbing, frame keyboard navigation and Split-at-Playhead. Media Pool import registers local H.264 videos as persistent assets without copying media bytes; import/append/relink and asset validation use bounded background probe work. M4-A adds a renderer-owned single-layer wgpu Preview that reuses eframe's existing device/queue, preserves clip/PTS identity and aspect/orientation, bounds texture resources, and falls back to the egui texture path when GPU preview is unavailable. The clip-aware Preview resolves the active Timeline clip through its asset, maps the playhead through checked rational PTS conversion, and displays real decoded frames with generation-based stale-result suppression; gaps, offline/changed media, and multiple visual layers are explicit states. AutoSave exposes Saving/Saved/Unsaved changes/Save failed/Conflict, flushes before navigation/exit, preserves dirty state on failure, and never silently overwrites an externally modified file. M3-D.1 adds checksum-validated one-slot recovery snapshots, an explicit Restore as New Project flow, and a Save Recovery Copy route for conflicts; recovery never automatically overwrites the primary. There is no AutoRecovery journal, so AutoSave is not crash-proof; inactive rename/duplicate are synchronous library operations, and there is no OS-level multi-process lock. Audio, playback, compositing, hardware decode, linked-media movement, and export encoding are not implemented. FFmpeg is a runtime external prerequisite and not bundled or linked; eframe's wgpu renderer paints the UI and uploads the Preview texture, but there is no video compositor backend. Avoid claiming any of these systems works until the code and tests prove it.

**Primary product goal:** a reliable Windows desktop editor for simple TikTok-style timelines (one 4K/30 FPS video layer, subtitles, and audio), followed by more complex editing. Prioritize a working vertical slice over breadth.

## 3. Default work loop

**Understand → Plan → Implement → Test → Report**

- Understand the task, identify the owning crate and downstream consumers, and check prior ADRs.
- For a nontrivial task, write a concise plan with acceptance tests. Avoid prolonged planning when a small implementation is clear.
- Implement within the crate boundary. Do not silently add libraries, APIs, performance promises, or compatibility claims.
- Run formatting, Clippy, unit tests, and check/build relevant targets as available. For GUI changes, inspect the running interface when tools allow. For decode/render changes, include a real or synthetic fixture and measurable result.
- Summarize what changed, what ran, what failed, and what remains unknown. If a change was not tested in a real Windows environment, say so.

Use `docs/AGENT_WORKFLOW.md` for branch/worktree practices, failure handling, and Definition of Done.

## 4. Working boundaries

- **Don't push or commit unless explicitly requested.** Never discard local work without approval.
- Don't create a new crate or restructure the workspace for convenience. Propose an ADR when the boundary is material.
- Don't hard-code absolute local paths, user names, machine configuration, or test media locations.
- Preserve the user's API and UI scope. Under RULE 000, do not implement, extend, test, or document any CapCut-related feature or research in this repository — including "read-only" work — and do not remove the existing research files either without explicit owner authorization.
- Prefer tests and explicit errors over demo-only happy paths.
- Keep reverse-engineering evidence, draft schemas, `CONFIRMED`/`INFERRED`/`UNKNOWN` ledgers and fixture dumps **outside this repository**; if you need them, request an owner-authorized private workspace elsewhere.
- Treat `unsafe`, native media integration, licensing, export, and persistent file writes as review-sensitive.

## 5. Quality gates

```powershell
cargo fmt --all -- --check --config-path .config/rustfmt.toml
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace

# Separate media-enabled gate (requires local FFmpeg/FFprobe and H.264 encoder)
$env:CUTCRAFT_REQUIRE_MEDIA = "1"
cargo test -p capcut-media --test media_pipeline -- --nocapture
Remove-Item Env:CUTCRAFT_REQUIRE_MEDIA
```

These commands should be run from the workspace root. If the environment lacks Windows, a compiler, libraries, GPU access, or media files, mark the affected verification as **NOT RUN**; never replace it with a fabricated pass. Do not invoke `cargo xtask ci` as an established gate: `xtask` is presently a placeholder.

## 6. Expected final response format

Use a compact engineering summary:

- **Implemented:** observable behavior and relevant files.
- **Verified:** exact commands/tests and their actual results.
- **Not verified / risks:** platform dependencies, missing checks, limitations.
- **Next step:** one small, high-value action tied to the roadmap.

When a task is only research/planning, explicitly state that no source files were modified.
