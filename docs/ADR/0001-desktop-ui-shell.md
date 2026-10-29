# ADR-0001: Desktop UI shell with egui and eframe

Status: Accepted
Date: 2026-10-29

## Context

CutCraft needs a launchable native Windows desktop shell before timeline, media,
and playback systems exist. The workspace reserves `crates/ui` for egui
application state and widgets, and `apps/capcut-editor` as the native entry
point. The roadmap's M1 milestone requires pinned UI dependencies, a resizable
window, a dark editor layout, and no blocking or misleading media operations.

The scaffold previously had no GUI dependencies and the binary only printed a
message. egui/eframe is the intended direction in `docs/ARCHITECTURE.md`, but a
concrete version, rendering backend, and responsibility boundary had not been
recorded.

## Decision

- Pin `egui` and `eframe` **0.37.0** as workspace dependencies. This release
  declares Rust 1.95 as its MSRV, so it is compatible with the workspace's
  pinned Rust 1.97.1 toolchain. `Cargo.lock` is the exact resolved version.
- Keep the GUI framework confined to `capcut-ui` and `apps/capcut-editor`.
  `core`, `capcut-format`, `engine`, `media`, and `renderer` remain free of
  `egui`/`eframe`, window types, and UI state.
- Use eframe's default **wgpu** renderer for painting the application shell.
  This is a UI presentation choice only. It does **not** select or implement the
  video compositor/preview backend described for `capcut-renderer`; that remains
  an open decision requiring its own ADR and evidence.
- Keep UI-only state (`capcut-ui::app_state::ShellState`) separate from future
  engine project/timeline state. Panels currently display static placeholders
  and do not mutate any engine data.
- Use egui's reactive repaint model. The shell must not install a periodic
  repaint loop; it redraws on input and will request repaints only for real
  asynchronous work once such work exists.
- Apply a dark theme at application creation, set an explicit minimum window
  size, and use resizable panels with bounded minimum/maximum sizes.

## Alternatives considered

- **Raw winit + wgpu UI**: more control, but substantially more code for menus,
  fonts, layout, and accessibility before any editing feature exists.
- **eframe with the glow renderer**: fewer GPU abstraction layers, but adds a
  second renderer choice and may be less reliable on systems with limited
  OpenGL support. eframe's supported default is wgpu.
- **Web/Tauri shell**: rejected for an independent native editor; adds web
  runtime packaging and does not advance the native rendering goal.
- **Delaying UI work until the engine exists**: rejected because M1 requires a
  responsive, inspectable shell that later milestones can wire to commands.

## Consequences and risks

- The dependency graph and compile time grow significantly. This is accepted
  for the desktop shell; no unused GUI dependency is introduced into lower
  layers.
- egui, eframe, and wgpu declare MIT OR Apache-2.0 licensing, compatible with
  this workspace's declared license. A full transitive license and advisory
  audit is still required before distributing binaries.
- egui is an immediate-mode UI. Long-running work must later be moved to
  bounded workers; this milestone performs no I/O, decoding, or blocking work
  in the UI.
- The wgpu UI surface is not a commitment to a video rendering architecture.
  If preview later needs to share a device or texture path, that requires a new
  ADR and explicit ownership rules.
- egui 0.37 changed the `eframe::App` entry point to `App::ui(&mut Ui, ...)`
  and introduced the unified `egui::Panel` API. Code and tests target that API;
  upgrading egui versions is a deliberate migration, not an incidental bump.

## Validation and rollback

Validation for the initial change:

- workspace format, Clippy, tests, and check gates;
- headless egui smoke test rendering the shell at the minimum window size;
- a test that a stable idle frame requests no immediate or periodic repaint;
- manual launch, resize, and close of `capcut-editor` on Windows.

The subsequent M1 UI iterations added Home/Projects/Settings navigation, a
temporary in-memory editor session, nine category-specific library shells, a
tabbed Inspector, aspect-aware preview, a large four-track timeline with zoom,
and a settings-only Export modal. The M1.2 visual pass added a neutral
charcoal/cyan design system and a 33/29/38 resizable workspace. All of this
remains inside `capcut-ui`/the application, adds no dependency, and does not
change the engine or media boundaries. It is UI state only: no project is
persisted, decoded, or encoded.

Rollback is limited to removing the two workspace dependencies, restoring the
console `main.rs`, and deleting the UI shell modules. No persistent format or
engine API depends on this decision.
