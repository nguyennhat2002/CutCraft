# CutCraft — Agent Development Workflow

> A working procedure for Codex, Claude, DSH-based agents, and human contributors. `../AGENTS.md` contains the non-negotiable policies; this file tells you how to apply them.

## 1. Before modifying the repository

1. Read `AGENTS.md`, `CLAUDE.md`, `ROADMAP.md`, and `docs/ARCHITECTURE.md`.
2. Inspect the task's crate (`Cargo.toml`, public interfaces, implementation files, and tests).
3. Inspect `git status --short` and `git diff` to avoid overwriting the user's work.
4. Classify the task: documentation, bug fix, feature, experiment, dependency/architecture change, or research.
5. Determine the smallest acceptance test that would demonstrate completion.
6. Confirm runtime assumptions: Windows/compiler availability, GPU, native codec dependencies, and input fixtures.

Do not spend an entire session planning a trivial documentation fix. For cross-crate feature work, record a short implementation plan and identify which layer owns each responsibility.

## 2. Scope and approval boundaries

**Allowed without separate approval when requested:** read relevant files; implement scoped changes; add appropriately scoped unit tests; run existing local checks; update directly impacted docs.

**Request approval before:** deleting/migrating user project data; rewriting Git history; force-pushing; publishing a package/release; uploading private content; making a broad architecture rewrite; adding a dependency with significant licensing/runtime requirements; or introducing native/unsafe code with new distribution implications.

Do not commit or push unless the user explicitly requests it. If an agent cannot edit the user's local working tree, prepare an exact patch or ready-to-copy files and say so.

## 3. Implement in small vertical slices

For a task that touches UI, engine and media:

1. Define the core data type or interface first.
2. Implement pure logic and tests in the owning crate.
3. Add an adapter in `media` or `renderer` without exporting framework types into `core`/`engine`.
4. Connect the application/UI only after the lower layers have stable APIs.
5. Exercise the full behavior using real or generated non-copyright-sensitive fixtures.

Prefer completing **open MP4 → decode one frame → render it** before designing a generalized plugin graph. Keep feature flags and backend choices explicit. Avoid speculative abstractions copied from unrelated projects.

## 4. Automated validation

Run from the workspace root as appropriate:

```powershell
cargo fmt --all -- --check --config-path .config/rustfmt.toml
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace
```

For localized iteration, `cargo test -p <actual-package>` or `cargo check -p <actual-package>` is acceptable, but run workspace gates before claiming a cross-crate task is complete. New integration/FFI dependencies may require Windows toolchain components: report platform-specific blockers instead of claiming the code works elsewhere.

**Current warning:** the `xtask` binary prints a scaffold-only message. It does not implement aggregate gates, asset validators, layering checks, or packaging. Do not use unimplemented `cargo xtask ...` commands as proof of success.

## 5. Testing by change type

| Change | Required evidence |
|---|---|
| Core time / math | Rational-rate edge cases, overflow, exact boundary tests |
| Timeline command | Correct mutation, invalid input, undo/redo reversibility |
| Project format / parser | Good + malformed files, unknown fields, no unexpected data loss |
| Decoder | Known fixture metadata, frame ownership, seek/PTS, corrupted file handling |
| Renderer | Frame dimensions/stride, scaling, screenshot or image comparison if possible |
| Playback | Monotonic clock, pause/seek cancellation, queue cap, no UI-thread blocking |
| UI | Interaction/manual smoke test and error state, not only widget construction |
| Performance | Hardware and scenario recorded, warm/cold runs, p50/p95 and memory |

See `TESTING.md` and `PERFORMANCE.md` for detailed methodology.

## 6. Definition of Done (DoD)

A completed task must satisfy all applicable items:

- [ ] The requested behavior is implemented, not just an empty module or a TODO.
- [ ] Changes obey the crate ownership and dependency graph.
- [ ] Invalid inputs and operational failures return actionable results.
- [ ] Automated tests cover main behavior and important edge cases.
- [ ] Format/Clippy/tests/check pass, or failures and their causes are listed.
- [ ] Interactive features were exercised (or marked **NOT VERIFIED** with reason).
- [ ] No private assets, credentials, generated binaries or copyrighted media were committed.
- [ ] Relevant docs/roadmap/ADR reflect the real new state, not hoped-for functionality.
- [ ] The final report states exactly which files changed and the evidence.

## 7. Decision records (ADRs)

Create an ADR for decisions with durable cost or cross-crate impact:

- timeline time representation and frame/PTS mapping;
- decoder/encoder library choice, linking, license, and distribution;
- GPU/graphics backend and device/surface lifetimes;
- engine dependency on CapCut-specific formats;
- project persistence, autosave and schema migrations;
- main A/V clock and seek/frame-drop policy.

Recommended ADR format:

```markdown
# ADR-0001: Decision title
Status: Proposed | Accepted | Superseded
Date: YYYY-MM-DD
## Context
## Decision
## Alternatives considered
## Consequences and risks
## Validation / rollback plan
```

Do not treat a proposed ADR as an approved implementation mandate.

## 8. Working with multiple agents

- Assign disjoint files/crates where possible and define the interface contract first.
- For overlapping work use separate Git branches/worktrees, not simultaneous uncoordinated edits.
- Do not use shared uncommitted changes as a synchronization protocol; communicate the current branch, commit, and API shape.
- Re-run relevant checks after integration, including the full workspace when cross-crate dependencies changed.
- Preserve existing user edits. Never auto-resolve a conflict by dropping a side.

## 9. Progress and reporting

Provide progress only when there is meaningful evidence: a resolved API, passing test, verified UI path, or a clear blocker. Suggested final report:

```text
Implemented: <behavior and exact affected files>
Verified: <commands, pass/fail, OS, hardware if relevant>
Not verified: <blocked scenarios / missing environment>
Decisions: <new ADRs or explicit assumptions>
Next: <one actionable next milestone task>
```

When analysis stops at an indirect call or missing proof, state the boundary. For reverse-engineering results, distinguish `CONFIRMED`, `INFERRED`, and `UNKNOWN` and never turn a class-name match into an execution/dataflow claim.
