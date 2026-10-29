# Architecture

Dependency layers:
- Layer 0: capcut-core
- Layer 1: capcut-format depends on capcut-core
- Layer 2: capcut-engine depends on core and format; capcut-media depends on core
- Layer 3: capcut-renderer depends on core, media, and engine
- Layer 4: capcut-ui depends on engine, renderer, and media
- Application: apps/capcut-editor depends on capcut-ui, engine, renderer, and media
- Automation: xtask

Keep shared time, identifiers, errors, and math in capcut-core. Keep CapCut JSON mapping in capcut-format and validate imported data before conversion to an engine project. The engine owns project state and commands. Media I/O and GPU rendering stay separate from UI state.

These are intended boundaries. The current modules are scaffolding and do not implement the behaviors yet.