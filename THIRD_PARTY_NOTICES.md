# Third-party notices

CutCraft source and assets are licensed **MIT OR Apache-2.0** (see [`LICENSE-MIT`](LICENSE-MIT),
[`LICENSE-APACHE`](LICENSE-APACHE)). This file records the third-party material this repository depends on
or reproduces, which keeps its own terms. It is a factual inventory, not a legal opinion.

## How this list was produced

```powershell
cargo metadata --format-version 1          # declared licences per package
cargo tree -p capcut-editor --target x86_64-pc-windows-msvc -e normal,build
```

At the commit this file was written, **216 packages** resolve for `x86_64-pc-windows-msvc` (normal + build
dependencies of the `capcut-editor` binary). Every one of them is under a permissive licence or a permissive
dual licence (MIT, Apache-2.0, Zlib, BSD, ISC, Unicode-3.0, BSL-1.0, 0BSD, or a dual licence including one of
those). No copyleft-only dependency is linked into the shipped binary.

## Direct dependencies

| Package | Version | Licence | Notes |
|---|---|---|---|
| `egui` | 0.37.0 | MIT OR Apache-2.0 | immediate-mode UI |
| `eframe` | 0.37.0 | MIT OR Apache-2.0 | desktop backend; owns the wgpu device/queue that `capcut-renderer` reuses |
| `egui_extras` | 0.37.0 | MIT OR Apache-2.0 | table/image support |
| `egui-lucide` | 0.2.0 | MIT AND ISC | Lucide icon rendering for egui |
| `wgpu` | 30.0.1 | MIT OR Apache-2.0 | GPU preview pipeline and textures |
| `serde`, `serde_json` | 1.0.229 / 1.0.154 | MIT OR Apache-2.0 | `.cutcraft` and draft JSON |
| `sha2` | 0.10.9 | MIT OR Apache-2.0 | recovery-snapshot and external-change checksums |
| `libc` | 0.2.191 | MIT OR Apache-2.0 | transitive system bindings |
| `windows-sys` | 0.61.2 | MIT OR Apache-2.0 | Win32 interop |

## Notable transitive items

These need attention when the project is ever distributed as a binary:

- **`epaint_default_fonts` 0.37.0 — `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0`.** Pulled in by
  `epaint` (via `egui`/`eframe`). The fonts are **compiled into the application binary**, so a distributed
  build must keep the SIL Open Font License 1.1 notice and the Ubuntu font licence alongside the code
  licence. This is the one dependency whose terms are not purely code licences.
- **Lucide icon set — ISC (with Feather-derived icons under MIT).** The UI renders icons from
  [`egui-lucide`](https://crates.io/crates/egui-lucide) 0.2.0, which compiles a pinned **Lucide 1.61.0** icon
  set into the crate: nothing is downloaded at build time and nothing is fetched at runtime. Full notices are
  kept in [`docs/licenses/`](docs/licenses/):
  [`LUCIDE_LICENSE.txt`](docs/licenses/LUCIDE_LICENSE.txt) (ISC, including the list of Feather-derived icons)
  and [`EGUI_LUCIDE_LICENSE.txt`](docs/licenses/EGUI_LUCIDE_LICENSE.txt) (MIT).
- **`self_cell` 1.3.0 — `Apache-2.0 OR GPL-2.0-only`.** Used by `epaint`. CutCraft takes the **Apache-2.0**
  option of that dual licence; nothing is distributed under the GPL alternative.
- **`tiny-skia` / `tiny-skia-path` 0.11.4 — BSD-3-Clause**, **`arrayref` — BSD-2-Clause**,
  **`bytemuck` / `foldhash` — Zlib OR Apache-2.0 OR MIT**, **`memchr` / `byteorder-lite` — Unlicense OR MIT**,
  **`libloading` — ISC**, **`zerocopy` — BSD-2-Clause OR Apache-2.0 OR MIT**. All permissive; kept for
  completeness because they land in the binary.

## How those dependencies are used

- **SVG rasterisation** goes through `egui_extras` 0.37.0 with its `svg` feature and the `resvg` / `usvg`
  0.45.1 chain (MIT/Apache-2.0 family). Icons are rendered locally from the compiled-in set; the application
  performs no network access for assets.
- **Configuration and project serialisation** uses `serde` / `serde_json` (plus `itoa` 1.0.18 for number
  formatting). Shortcut profiles and `.cutcraft` documents are written to local disk only; nothing is
  uploaded.
- **Checksums** for recovery snapshots and external-change detection use `sha2` (MIT OR Apache-2.0).

## Original CutCraft assets

The CutCraft brand mark, the illustrative banner and demo artwork, the theme, and the WGSL preview shader in
`crates/renderer/src/shaders/` are original to this project and are covered by the `MIT OR Apache-2.0` grant.
No third-party application's icons, illustrations, fonts or UI artwork are included anywhere in the repository.

## FFmpeg

CutCraft does **not** bundle, link, embed or statically include FFmpeg. The media layer discovers a
`ffmpeg` / `ffprobe` executable at runtime from `PATH` or `CUTCRAFT_FFMPEG` / `CUTCRAFT_FFPROBE` and invokes
it as a separate, time-limited process (see
[ADR-0005](docs/ADR/0005-ffmpeg-cli-preview-backend.md)).

Consequences:

- FFmpeg's own LGPL/GPL obligations belong to whoever builds and distributes that FFmpeg binary; they are not
  inherited by CutCraft source, and licensing CutCraft does not license FFmpeg.
- Anyone shipping CutCraft together with an FFmpeg build must satisfy that build's terms separately, and must
  verify codec obligations for the formats they enable. No such bundling decision has been made here.

## Trademarks and other editors

- CutCraft is an independent project. It is **not affiliated with, sponsored by, or endorsed by** any other
  video editor vendor. All third-party names and trademarks belong to their owners and are used only to
  identify the products or technologies they refer to.
- This repository distributes **no third-party editor code, binaries, UI artwork, screenshots, project files,
  or fixtures**, and does not host research into other vendors' formats — see `RULE 000` in
  [`AGENTS.md`](AGENTS.md).
- Native `.cutcraft` files are an independent format created by this project and share no schema with any
  other editor's project format.

## Media and documentation assets

- Screenshots in [`assets/screenshots/`](assets/screenshots/) and [`docs/ui/screenshots/`](docs/ui/screenshots/)
  are captures of CutCraft itself, produced by this project's own QA runs; the video content in them is
  synthetic test footage generated locally. Provenance for the README image is recorded in
  [`assets/screenshots/README.md`](assets/screenshots/README.md).
- UI geometry recorded in `docs/ui/` was measured from material held outside this repository; the measured
  values are documented here, the source captures are deliberately not stored in the repo.

## Adding a dependency

Check its licence with `cargo metadata`, prefer MIT / Apache-2.0 / BSD / Zlib / ISC, avoid anything copyleft
that would apply to the shipped binary, and record anything that ships fonts, data files or non-code assets in
this file.
