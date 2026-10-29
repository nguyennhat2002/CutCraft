# CutCraft — Session Instructions for Claude and Coding Agents

`AGENTS.md` is the authoritative repository-wide policy. **Read it first.** This file describes how to operate in CutCraft, not a second conflicting set of rules. Codex and other agents may also use this entry point.

## 1. First five minutes of a session

1. Read `AGENTS.md` and this file.
2. Read `ROADMAP.md` to identify the current approved milestone and its acceptance criteria.
3. Read `docs/ARCHITECTURE.md`, then the relevant crate `Cargo.toml`, `src/lib.rs` and implementation files.
4. For an implementation task, read `docs/AGENT_WORKFLOW.md`, `docs/CODING_STANDARDS.md`, and `docs/TESTING.md`.
5. For preview, decode, rendering, or performance work, also read `docs/PERFORMANCE.md`; for CapCut behavior or compatibility research, read `docs/RESEARCH_POLICY.md`.
6. Inspect repository status before changing files. Honor uncommitted edits and the user's current constraints.

## 2. What exists today

CutCraft is an **early-stage** Rust video editor with a working desktop UI shell. Current workspace members:

```text
apps/capcut-editor
crates/core
crates/capcut-format
crates/engine
crates/media
crates/renderer
crates/ui
xtask
```

The desktop `main.rs` launches an eframe window using pinned `egui`/`eframe` 0.37.0 (ADR-0001). The app is a dark, resizable **UI shell** with Home/Projects/Settings navigation, a temporary in-memory Editor session (library tabs, Preview, Timeline, Inspector), and a settings-only Export modal. Editing, media import, playback, and export encoding are not implemented; their controls are disabled, and no project is written to disk. The timeline/command model, playback controller, decoder, frame cache, compositor, video preview, format import/export, and packaging modules remain placeholders. FFmpeg and a video compositor backend have not been selected; eframe's wgpu renderer paints the UI only. Avoid claiming any of these systems works until the code and tests prove it.

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
- Preserve the user's API and UI scope. Don't implement speculative CapCut features from reverse-engineering notes.
- Prefer tests and explicit errors over demo-only happy paths.
- Separate observed research from independent design choices; use `CONFIRMED`, `INFERRED`, and `UNKNOWN` for reverse-engineering notes.
- Treat `unsafe`, native media integration, licensing, export, and persistent file writes as review-sensitive.

## 5. Quality gates

```powershell
cargo fmt --all -- --check --config-path .config/rustfmt.toml
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace
```

These commands should be run from the workspace root. If the environment lacks Windows, a compiler, libraries, GPU access, or media files, mark the affected verification as **NOT RUN**; never replace it with a fabricated pass. Do not invoke `cargo xtask ci` as an established gate: `xtask` is presently a placeholder.

## 6. Expected final response format

Use a compact engineering summary:

- **Implemented:** observable behavior and relevant files.
- **Verified:** exact commands/tests and their actual results.
- **Not verified / risks:** platform dependencies, missing checks, limitations.
- **Next step:** one small, high-value action tied to the roadmap.

When a task is only research/planning, explicitly state that no source files were modified.
