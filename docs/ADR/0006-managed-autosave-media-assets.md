# ADR-0006: Managed Project Library, Persistent Media Assets, and Generation-Safe AutoSave

- Status: Accepted for M3-D
- Date: 2026-10-31

> M3-D.1 extends this decision with bounded native recovery snapshots and external-conflict preservation in [ADR-0007](0007-native-recovery-conflict-copy.md). The M3-D decisions below remain historical and are not silently rewritten.

## Context

M3-C proved a real H.264 probe/decode path, but the video binding was session-only and `.cutcraft` v1 had no media identity. That produced a workflow that could not survive a restart and required manual Save/Save As. M3-D changes the product workflow to CapCut-style managed projects:

- a new project is created inside a managed CutCraft project library and is persisted before the Editor is exposed;
- edits are AutoSaved without repeated destination dialogs;
- media files are registered as reusable project assets referenced in place, never copied;
- the Preview follows the active Timeline clip through its asset instead of a manual session binding;
- offline media is detected and recovered through explicit relink;
- no silent overwrite, no fake "Saved" state, and no automatic external-file resolution.

The store must also remain honest: AutoSave is not crash-proof recovery, and a v1 project must not be rewritten merely by opening it.

## Decision

### 1. Media assets live in the engine project, not a sidecar

`capcut-engine` owns a bounded media asset registry inside `Project`:

- `MediaAssetId` is a stable project-local identity allocated monotonically (`next_asset_id`), never reused after undo/redo.
- `MediaAsset` holds a display name, locator, video kind, probe-derived `MediaAssetMetadata` (container, stream index, codec, dimensions, pixel format, rational timebase, stream start, duration, nominal/average rate, rotation, audio presence), and an optional file-size hint.
- A `Clip` may reference one `MediaAssetId`. Its existing `SourceMapping` remains in the 120,000-tick source domain; decoder stream timebase and start stay in asset metadata. The three time domains (`TimelineTicks`, clip `SourceTicks`, decoder PTS) are not collapsed.
- Asset-backed clips must satisfy: the asset exists, the clip kind is video, the source mapping is present, and the used source range is within the asset's converted duration. Validation rejects dangling references.
- `AddMediaAsset`, `RelinkMediaAsset`, `AppendMediaClip`, and `RenameProject` participate in the existing transactional command/history model. Split copies the asset reference to both halves; Undo/Redo preserve monotonic asset IDs.

Asset bounds: 4,096 assets per project, 4,096-byte locator, 256-byte display name, 8,192 pixels per dimension, 33,554,432 pixels per frame.

### 2. `.cutcraft` schema v2 is additive and strictly validated

- `SCHEMA_VERSION = 2`; v1 remains readable.
- v1 is migrated **in memory** into an empty asset registry (`next_asset_id = 1`) with no clip references. Opening v1 does not rewrite the file. An explicit save publishes v2.
- v2 stores project ID, name, rational FPS, timebase, main track, allocators, Magnet state, creation/last-saved timestamps, save generation, assets, tracks, and clip `media_asset_id`.
- Parsing is strict: unknown fields, duplicate IDs, duplicate locators, dangling asset references, invalid metadata, and future schema versions are typed errors. Future schema versions are rejected from the header before a full parse.
- Save writes a temporary file, validates the staged document, flushes, and atomically replaces the target while checking the previously observed file snapshot. External modification or disappearance is a conflict, never an overwrite.

### 3. Locator policy is deterministic and copy-free

- Media outside the managed project directory uses an absolute locator. Windows drive-letter and UNC paths are accepted; relative traversal and non-absolute "absolute" values are rejected.
- Media inside the project directory uses a `ProjectRelative` locator so moving the whole project folder keeps it resolvable.
- Import, relink, and duplicate never copy media bytes. Duplicate rebases relative locators to absolute paths pointing at the original files, so a duplicate remains valid regardless of where it is opened.
- Path comparison handles Windows case-insensitivity and canonical `\\?\`/`\\?\UNC\` prefixes.

### 4. Managed project library

- Root resolution: `CUTCRAFT_PROJECTS_DIR`, otherwise Windows `LOCALAPPDATA\CutCraft\Projects` (XDG on other platforms).
- Layout: `<root>/<ProjectId>/project.cutcraft`. The display name is metadata and is not the path identity.
- Creation prepares the project, writes and verifies the initial document, and only then exposes the Editor. Failed creation discards the prepared directory.
- Scan is bounded to 128 entries and reports invalid entries explicitly; invalid projects are not silently skipped or opened.
- Rename changes display metadata and preserves Project ID and path. Duplicate creates a new Project ID and directory while preserving internal track/clip/asset IDs and rebasing relative locators. Inactive rename/duplicate are small synchronous library operations; project persistence writes still go through the bounded worker.

### 5. Generation-safe AutoSave

Timing defaults (injected-clock deterministic):

| Parameter | Value |
|---|---|
| Debounce after a change | 1,500 ms |
| Maximum interval while continuously dirty | 15,000 ms |
| Retry after a failed save | 5,000 ms |

Architecture:

1. A pure `AutoSaveScheduler` tracks dirty state, last change/save time, and generation. It emits at most one save intent per tick and never treats "scheduled" as "saved".
2. `PersistenceWorker` owns one thread, one pending latest-job slot, one in-flight job, and one result slot. A newer job replaces a pending job; it never cancels a write already in progress.
3. Every persistent edit increments the document generation. `ProjectSnapshot { project, magnet_enabled }` is immutable and excludes playhead, selection, zoom, focus, decoded frames, and textures.
4. Save results carry kind, project ID, generation, and result. A successful older generation updates `persisted_generation` only; a newer dirty generation remains dirty and must be saved separately. "Saved" is shown only when the persisted generation equals the current generation.
5. Failure (I/O, serialization, external conflict) preserves dirty state and the last valid file. External modification pauses writes and requires explicit Retry/Stay; no automatic overwrite.
6. Navigation and application exit first flush through the same nonblocking coordinator. The wait dialog is explicit, the UI remains responsive, and navigation proceeds only after the required generation is persisted. A failed flush offers Retry or Stay in the Editor.
7. AutoSave persists project content including assets, clips, name, and Magnet state. It never serializes transient UI state, decoded frames, or scrub position.

### 6. Clip-aware Preview replaces session binding

- The Preview resolves the active non-audio clip at the authoritative playhead using half-open clip ranges. Multiple simultaneously active visual layers report `UnsupportedComposition`; a gap reports `NoActiveClip`.
- A video clip without an asset reports `ClipWithoutMedia`; unsupported visual layers are typed rather than approximated.
- For an asset-backed clip, Preview resolves the project path and locator, checks availability, probes through the bounded probe queue if needed, verifies live probe metadata against persisted metadata, and maps `playhead - clip.start + source_used.start` to decoder PTS with checked arithmetic.
- Frame requests carry source, generation, clip tag, and asset tag. Results for stale sources, generations, clips, or assets are ignored.
- Missing, unreadable, or changed assets show explicit states and never substitute another file. Relink preserves the asset ID, validates that the new file covers every referencing clip, and updates the locator/metadata as a normal undoable edit.
- Asset availability is validated once per project revision. Import/relink use a single bounded probe worker (64 queued requests/results).

## Consequences

- A project now survives restart with real media references and remains usable after moving the project root when media lives inside it.
- Editing and reopening no longer require manual Save/Save As; duplicate and rename remain available in Home.
- The decoder remains replaceable and unaware of UI, persistence, and library policy.
- Media is never written into the project library by import, relink, duplicate, or save.

## Explicit limitations

- **No AutoRecovery journal.** AutoSave writes the managed project file; the last successfully published file is preserved on failure, but there is no crash-recovery copy. Do not describe AutoSave as crash-proof.
- Conflict recovery offers Retry Save or Stay in the Editor. There is no "Save a Copy" action yet.
- Inactive rename/duplicate run synchronously on the UI thread. Only document writes are worker-backed.
- No source-media copying, no CapCut AppData/draft modification, no CapCut-draft AutoSave, and no CapCut-to-native conversion.
- No audio, playback clock, compositing, transitions, linked-media movement, nested editing, hardware decode, GPU compositor, or export encoding.
- FFmpeg remains a runtime-discovered external prerequisite (ADR-0005); no binary is bundled or linked.
- The QA/managed-project root can be isolated with `CUTCRAFT_PROJECTS_DIR`; this is a test/QA facility, not a user-facing destination picker.
