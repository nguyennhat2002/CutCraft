# CapCut Rust Editor
Workspace scaffold for a Rust desktop video editor.
- apps/capcut-editor: application binary
- crates/core: shared time, IDs, errors, and math
- crates/capcut-format: CapCut draft schema, import, export, and validation
- crates/engine: project model, commands, history, selection, and playback
- crates/media: decoding, encoding, probing, frame cache, and audio resampling
- crates/renderer: GPU context, pipelines, compositor, and shaders
- crates/ui: egui application UI and widgets
- xtask: project build, test, and packaging automation

The directories and module boundaries are in place. Editing, CapCut JSON compatibility, media processing, GPU rendering, and UI behavior are not implemented yet. External library versions will be selected with their integrations.