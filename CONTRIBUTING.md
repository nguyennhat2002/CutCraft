# Contributing to CutCraft

Thanks for your interest. CutCraft is an early-stage native Rust video editor, and the way it is
developed matters as much as what it does: **every capability claim in this repository has to be backed by
evidence, and anything unimplemented stays labelled unimplemented.**

## Before you start

- **Platform:** Windows is the primary development and target platform. Changes should not break the Linux
  and macOS CI jobs, but they are not the design focus.
- **Toolchain:** Rust **1.97.1**, pinned by [`rust-toolchain.toml`](rust-toolchain.toml) with `rustfmt` and
  `clippy`. Install via `rustup`; the file does the rest.
- **Optional:** `ffmpeg` / `ffprobe` on `PATH` (or `CUTCRAFT_FFMPEG` / `CUTCRAFT_FFPROBE`) if you are touching
  the media path. FFmpeg is never bundled or linked.
- Read [`AGENTS.md`](AGENTS.md) and [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) first: they define the
  crate boundaries and the data-safety rules that review enforces.

## Making a change

```powershell
cargo run -p capcut-editor          # exercise it in the real app
cargo fmt --all -- --config-path .config/rustfmt.toml
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace
```

CI is configured to run the last four checks on Ubuntu, Windows and macOS (with `fmt --check`); Windows is the
platform where changes are actually verified. If you touched media decoding or the preview path, also run the
strict media gate:

```powershell
$env:CUTCRAFT_REQUIRE_MEDIA = "1"
cargo test -p capcut-media --test media_pipeline -- --nocapture
Remove-Item Env:CUTCRAFT_REQUIRE_MEDIA
```

Additional expectations:

- **Timing is integer-first.** Authoritative timeline time is `TimelineTicks` at 120,000/s; frame rates are
  rational (`30000/1001`, not `29.97`). Floating-point seconds are for display only.
- **No panics in production paths.** Prefer typed `Result`, explicit options, and honest `Unsupported`
  responses over `unwrap`/`expect`/`todo!`. Tests may panic deliberately.
- **Never block the UI thread** with decode, seek, file or process work; keep queues, caches and worker
  lifetimes bounded and cancellation-aware.
- **User data is sacred.** Saves use validated input, temporary files and atomic replacement; a failed
  import, save or edit must return an actionable error and leave the active project intact.
- **Layering:** no `egui`/`eframe`/`wgpu` in `core`, `engine`, `media` or `project`; GPU resources live only
  in `capcut-renderer`; UI owns viewport geometry and fallback presentation.
- **Architecture changes need an ADR.** Add a numbered document in [`docs/ADR/`](docs/ADR/) and update its
  index when you change a boundary, a file format, a threading model or a licence.

## RULE 000 — native CutCraft only

CutCraft is an independent editor, and this repository publishes **only** independently authored CutCraft
functionality. Contributions must not contain:

- compatibility implementations for another editor's project format, or readers/writers for it;
- reverse-engineering artifacts: evidence notes, ledgers, schema dumps, diff experiments, or tooling written
  to study another vendor's application or project library;
- private project fixtures, exported drafts, subtitles, voice tracks, or any media you do not own;
- third-party application binaries, icons, screenshots, or UI artwork;
- code or assets whose ownership you cannot confirm, or an imported file whose licence is incompatible with
  `MIT OR Apache-2.0`;
- claims of functionality that has not been demonstrated by a test, a log, or a reproducible run.

If a change you want to make requires any of the above, open an issue describing the *native* capability you
actually need instead — for example "import from format X" as a CutCraft feature — and we will discuss whether
it can be implemented independently. Work that studies another application belongs outside this repository.

## Reporting bugs

Include, as applicable:

- what you expected vs what happened, and the exact steps;
- CutCraft version or commit, OS build, and GPU backend if the preview is involved
  (`CUTCRAFT_RENDER_LOG=1` prints the backend line to stderr);
- the project's behaviour around the failure (AutoSave state, whether media was offline, whether recovery was
  offered) — never the contents of your project file;
- **no private media.** Attach a short synthetic clip you own, or describe the media precisely (codec,
  resolution, frame rate, pixel format).

## Proposing features

Open an issue before writing a large change. Useful context: the user-visible outcome, which crate layer it
touches, whether it changes the `.cutcraft` schema (that requires an ADR and a migration story), and how you
would demonstrate that it works.

## Tests and documentation

- Nontrivial behaviour ships with tests; bug fixes ship with a regression test.
- Timing conversions, trims, splits, undo/redo and invalid input handling are unit-tested without a GPU or
  media present.
- GPU preview correctness is verified in three tiers — headless validation, device-backed pixel readback
  (`crates/renderer/tests/preview_gpu.rs`, skipped when no adapter exists), and real-window QA. A skip is not
  a pass; say which tier your evidence comes from.
- Update [`ROADMAP.md`](ROADMAP.md) and the relevant docs when status or design actually changes. Do not mark
  a milestone complete after adding placeholders.

## Private data, fixtures and assets

- **Never commit** another vendor's project files, exports, personal media, credentials, machine-specific
  absolute paths, or QA output directories. Test fixtures must be synthetic and clearly owned.
- **Never commit** screenshots or artwork of another vendor's application, and do not add research notes about
  third-party formats or applications — that work belongs outside this repository (RULE 000).
- Screenshots used in the README live in [`assets/screenshots/`](assets/screenshots/) and must record
  provenance (build, date, capture method, media origin) and be reviewed for paths and identifying text.

## Licensing your contributions

Unless you explicitly state otherwise, any contribution you submit for inclusion in CutCraft is licensed
under both the [MIT license](LICENSE-MIT) and the [Apache License 2.0](LICENSE-APACHE), at the project
owner's option — the same `MIT OR Apache-2.0` expression the crate manifests declare — without any additional
terms or conditions.

There is **no CLA and no DCO sign-off requirement.** Please do not import code whose licence is incompatible
with that dual grant, and list any new dependency's licence in [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)
when your change adds one.

## Code style in one paragraph

`rustfmt` with the committed config decides formatting; Clippy with `-D warnings` decides linting; neither is
negotiable in review. Write small, focused modules with explicit ownership and error semantics, keep comments
to the parts that are genuinely non-obvious, and don't add abstractions for hypothetical future needs.
