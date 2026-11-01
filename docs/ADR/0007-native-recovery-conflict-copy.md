# ADR-0007: Bounded Native Recovery Snapshots and External-Conflict Preservation

- Status: Accepted for M3-D.1
- Date: 2026-10-31
- Extends ADR-0006; does not replace the authoritative staged project store

## Context

ADR-0006 defined managed AutoSave and explicitly stated that no AutoRecovery journal exists. A forced termination, power loss, or crash can leave the primary `.cutcraft` file at generation N while the editor held generation N+1 or later. M3-D.1 must provide a recoverable route without silently overwriting the last valid project and without presenting recovery as a crash-proof guarantee.

The existing project store is a staged, checksum-verified, atomic replacement writer. It must remain the authority for the primary project. Recovery is a separate, bounded artifact.

## Decision

### 1. Recovery is a single-slot, validated snapshot per project

- Recovery files live under `<managed-project-root>/.recovery/<ProjectId>.cutcraft-recovery`.
- The `.recovery` directory is hidden from project scanning and is never inside a non-native project folder.
- Recovery is written by a dedicated one-worker `RecoveryWorker` (one pending latest job, one in-flight job, one result slot). It is not the primary AutoSave worker.
- Only the newest recovery snapshot per project is retained. A newer snapshot atomically replaces an older one after full validation. There is no unbounded history.
- Recovery files are removed only by an explicit successful restore (after the new project is durably published) or by an explicit discard action. Stale, invalid, or foreign recovery files are not silently deleted.
- The primary project file is never modified by recovery publication or detection.

### 2. Envelope and integrity

The recovery file is strict JSON with `deny_unknown_fields`:

```text
format = "cutcraft-recovery"
recovery_version = 1
project_id
original_project_path        (absolute path at snapshot time)
document_generation          (editor generation being snapshotted)
last_saved_generation        (last verified primary generation at snapshot time)
last_saved_unix_ms
created_at_unix_ms
written_at_unix_ms
document_sha256              (SHA-256 of document_json bytes)
document_json                (a complete, strict schema-v2 ProjectFileV2)
```

Validation requirements:

1. File size is bounded (`MAX_PROJECT_FILE_BYTES + 64 KiB`) and JSON depth is bounded.
2. The envelope is parsed strictly; unknown fields, bad format/version, empty paths, and inconsistent generations are rejected.
3. `document_json` bytes must hash exactly to `document_sha256`.
4. `document_json` is parsed with the existing strict schema-v2 parser and fully restored through `ProjectFileV2::into_opened`; all engine invariants, MediaAsset references, allocator high-water marks, and Magnet state must validate.
5. `document_generation` must equal the embedded document `save_generation`; `last_saved_generation <= document_generation`; both are nonzero.
6. The embedded `project_id` must equal the envelope; the embedded name is used only for display.

A corrupt, truncated, partial, or unreadable recovery file is reported as invalid recovery data. It cannot damage or replace the primary project.

### 3. Generation contract

The authoritative primary file and recovery use the same document generation sequence:

| Situation | Primary generation | Recovery generation | Behavior |
|---|---:|---:|---|
| Clean AutoSave | 21 | absent or ≤21 | no recovery prompt |
| Crash after recovery publish, before AutoSave | 20 | 21 | offer recovery; primary stays at 20 |
| Crash before recovery publish | 20 | absent or ≤20 | no recovery; edits since 20 are lost |
| Stale recovery | 30 | 29 | do not offer as newer work |
| Same-generation recovery | 30 | 30 | do not claim newer work |
| Corrupt recovery | 30 | 31 (invalid) | keep primary; report invalid recovery separately |
| Edit during recovery publication | 31 | 31 in flight, editor at 32 | recovery 31 may be durable; 32 remains pending and is never claimed durable until published |
| Old primary, external conflict | 30 (externally changed) | any | recovery/conflict copy is a new document; primary is not overwritten |

The recovery worker uses immutable `ProjectSnapshot` values and the generation captured at submission time. A success result for generation G updates the UI's "recovery published" watermark only to G; a newer editor generation stays pending.

### 4. Eligibility and identity

A recovery candidate is offered only when:

- the envelope and embedded document validate;
- its `project_id` matches a valid managed library entry;
- its document generation is **strictly greater** than the primary file's `save_generation`;
- it is not a foreign recovery file and not a stale/same-generation candidate.

The original project path is displayed and used to resolve project-relative media when restoring. Project identity is the `ProjectId`; a path change alone does not make recovery foreign. Orphan recovery files with no matching library entry, and invalid candidates, are reported but never applied automatically.

### 5. Restore policy: a new recovered project

Restoring always creates a **new managed project** with a new `ProjectId` and a new directory. It preserves:

- internal track/clip IDs and allocator high-water marks;
- exact `TimelineTicks`, source mappings, trims, splits, and undo/redo-relevant content;
- media asset IDs, metadata, locators, and file-size hints;
- Magnet state;
- project content name (with a recovery suffix for the display copy).

Project-relative media locators are rebased to absolute locators pointing at the original source files, using the original path recorded in the envelope (or the current primary path when equivalent), so the new directory does not break media resolution. Media bytes are never copied or rewritten. If a source is offline, the recovered project preserves the locator and reports offline media normally.

The primary project is not overwritten. The original recovery file is deleted only after the new project has been staged, verified, and published; a failed restore leaves both the primary and recovery file intact.

### 6. Recovery UI

- At managed-library startup/open, candidates are scanned and validated with strict bounds.
- Valid newer candidates produce an explicit modal (and a Home recovery entry when dismissed) with:
  - **Restore as New Project**
  - **Open Last Saved Version**
  - **Keep Both** (retain the recovery file; suppress the prompt for this session)
- Invalid candidates produce a separate warning; no Restore action is offered.
- The UI displays the candidate generation, saved generation, timestamp, and original path.
- No automatic recovery, no automatic overwrite, and no "Saved" status before the recovered copy is actually published.
- Recovery restore and conflict-copy publication run on the bounded persistence/recovery workers; UI work remains nonblocking.

### 7. External-conflict preservation

When AutoSave detects an external modification (or external deletion), the primary file remains untouched and the document stays dirty/conflicted. The UI offers **Save Recovery Copy**:

1. Take the current in-memory `ProjectSnapshot` (including assets, clips, Magnet, allocators).
2. Rebase project-relative media locators against the current primary project directory to absolute paths.
3. Prepare a new managed project with a new `ProjectId` and a recovery-copy display name.
4. Publish through the existing staged writer and verify it.
5. Do not modify the externally changed primary file or clear the current session's conflict/dirty state.
6. On success, show the new project in Home/Recent Projects and report its name/path. On failure, preserve the current session and report the failure.

If the external file changes again during copy publication, the copy is independent; the original remains untouched. Automatic merging of divergent histories is not implemented.

### 8. Windows file-safety policy

- Primary publication remains staged + `sync_all` + staged re-parse + `rename`. On Windows, `std::fs::rename` replaces an existing file via the platform rename API, but this is not treated as a guarantee on every filesystem.
- Parent-directory flush is best effort; on Windows it is currently a no-op. Document this as a durability limitation.
- Abandoned `.project.cutcraft.<pid>.<n>.tmp` staging files older than 24 hours are removed best effort when a project is opened/scanned. Recent staging files are retained because another process may be writing them.
- There is **no OS-level multi-process lock**. CutCraft assumes one writer per project. Hash checking detects ordinary external modification but a check→rename race and two simultaneous writers remain possible. The product must not claim collision-proof saving.
- Locked/permission-denied targets, full volumes, antivirus/share-mode failures, and power loss are not guaranteed to be simulated; they must be reported as UNVERIFIED where a deterministic test is unavailable. A failed publish must preserve the previous valid file.

### 9. Recovery vs AutoSave vs conflict

| State | Meaning |
|---|---|
| AutoSave success | Primary file validated and atomically published at generation G; `Saved` may be shown |
| Valid newer recovery | A separate validated snapshot has generation > primary; primary remains valid and untouched |
| Corrupt/partial recovery | Candidate cannot be validated; ignore for restore; report separately; primary untouched |
| Stale recovery | Candidate generation ≤ primary; do not offer as newer work |
| External conflict | Primary changed on disk; AutoSave pauses; current edits remain dirty; optional new recovery copy |
| Forced termination before recovery publication | No durable snapshot of the latest edits; only the last valid primary/recovery remains |

No mechanism promises zero data loss when no durable snapshot exists.

## Consequences

- Additional disk writes proportional to project metadata changes; bounded to one recovery file per project.
- Recovery restore creates a new project identity; users see both the original and recovered copy.
- Conflict preservation creates a new managed project rather than merging histories.
- The authoritative project store, schema v2, generation semantics, and staged writer remain unchanged.
- Multi-process writing, parent-directory flush, full-volume behavior, and crash-before-recovery-publication remain explicit limitations.

## Rejected alternatives

- **Journal every edit**: more durable but larger, more complex, and unnecessary until the snapshot design is proven; a journal can be added later without changing the authoritative store.
- **Overwrite the primary with recovery on confirmation**: risks data loss and conflicts with the requirement not to overwrite automatically; a new project identity is safer.
- **Feed a recovery file to a third-party project-format reader**: this repository has no such reader, and doing so would confuse schemas.
- **Automatic merge of external and editor histories**: speculative and unsafe.
- **Store recovery inside each project directory**: would be copied by duplicate/move operations and could be mistaken for project content; a root-level hidden recovery area keeps identity explicit.
