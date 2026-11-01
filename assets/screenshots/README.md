# Public screenshots

Images in this directory are used by the repository README.

## `editor-gpu-preview.png`

- **Subject:** CutCraft editor showing the native Timeline, Media Pool, Inspector, and the
  renderer-owned wgpu Preview. The on-canvas badge reads `GPU PREVIEW` with the
  decoded clip's asset, dimensions, frame rate and presentation timestamp.
- **Captured:** 2026-11-01, Windows 11 (10.0.26300), x86_64.
- **Build:** debug binary of the M4-A.1 GPU Preview integration commit (`cargo run -p capcut-editor`), egui/eframe
  0.37.0, wgpu 30.0.1, Vulkan backend on an NVIDIA GeForce RTX 2080 SUPER.
- **Window:** 1600×900 client area; the PNG is the 1616×939 outer window, captured with
  Win32 `PrintWindow` (`PW_RENDERFULLCONTENT`) by `target/m4a_readme_shots.ps1`.
- **Media shown:** synthetic H.264 test clips generated locally for QA
  (`qa-hgradient.mp4`, a black-to-white ramp, plus seven other solid-colour and
  portrait/4K fixtures). They contain no personal, licensed or third-party footage.
- **Project shown:** an auto-named managed project (`Project 2026-10-31 16:45`) in an
  isolated QA project library under `target/`, not a real user library.
- **Privacy review:** checked for local paths, usernames, account names and personal
  media names. A companion Home-screen capture was **rejected and deleted** because the
  Recent Projects card renders the project's absolute file path.

Reuse: the screenshot is part of this repository's documentation and is covered by the
project's `MIT OR Apache-2.0` grant. CutCraft's interface uses icons from
[Lucide](https://lucide.dev) (ISC) — see [`../../THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md).
