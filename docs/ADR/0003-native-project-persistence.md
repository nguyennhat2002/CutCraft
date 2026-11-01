# ADR-0003: Native CutCraft Project Persistence

- Status: Accepted for M3-A
- Date: 2026-10-31

## Context

The native engine owns exact tick time, stable IDs, transactional edits, and engine-owned Magnet state, but has no persistence. Persistence must not turn the engine into a serialization/IO layer or couple native projects to a third-party project schema.

## Decision

1. Add an L2 `capcut-project` crate depending on `capcut-core`, `capcut-engine`, `serde`, `serde_json`, and `sha2`. It owns the versioned `.cutcraft` schema, document identity, bounded validation, and safe file IO.
2. Use versioned JSON (`schema_version: 1`) with integer tick fields and no authoritative float timing. Reject unknown fields and unsupported versions/format markers.
3. Keep `capcut-engine` free of serde and filesystem dependencies. Add only validated restore APIs: `Project::from_parts` and `NativeEditor::from_restored`.
4. Make project identity a document-level concern in `capcut-project`. Save preserves it; Save As publishes a new identity only after successful staged verification and replacement.
5. Persist explicit Magnet state. Loading validates continuity when ON and never normalizes; OFF preserves gaps.
6. Use staged sibling files, file synchronization, staged re-validation, and atomic replacement. Never delete a valid project before a verified replacement exists. Detect external modification by SHA-256 fingerprints.
7. Keep edit history session-only. Persist monotonic ID high-water marks and never reduce them to the maximum existing ID.
8. Keep `capcut-format` out of the native persistence design; it is not a native project store.

## Consequences

- Native projects are independently readable/testable and do not depend on external draft schemas.
- New crate and workspace dependency line; UI gains a dependency on `capcut-project`.
- Restore APIs expand the engine surface but do not add serialization/IO coupling.
- Save As is a document-identity boundary and requires careful failure atomicity.
- No migration is provided for future schema versions; they are rejected until an explicit migration exists.

## Validation

Round-trip tests cover exact ticks, IDs, allocators, source mappings, Magnet cases, post-Undo/Redo states, malformed/oversized inputs, unsupported versions, external modification, and failed saves. Real Windows UI QA covers create/edit/save/close/reopen/edit.
