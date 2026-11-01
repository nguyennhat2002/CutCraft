# ADR-0002: Native timeline timebase

Status: Accepted
Date: 2026-10-30

## Context

CutCraft needs authoritative edit timing for native timeline commands. The UI
currently stores display positions as `f64` seconds and has no engine model.
Floating-point seconds and rounded microseconds cannot represent rational frame
boundaries exactly, especially 24000/1001 and 30000/1001, and repeated edit
operations would accumulate drift.

The workspace rules require separate timeline time, source-media PTS, frame
indices, rational frame rates, and display coordinates. Cross-timebase math must
use checked wide intermediates and documented rounding.

## Decision

Use an integer tick timebase of **120,000 ticks per second** for native timeline
edits:

```text
TimelineTicks(i64)
TICKS_PER_SECOND = 120_000
```

The supported frame rates are validated and converted exactly:

| Rate | Ticks per frame |
|---|---:|
| 24/1 | 5,000 |
| 25/1 | 4,800 |
| 30/1 | 4,000 |
| 60/1 | 2,000 |
| 24000/1001 | 5,005 |
| 30000/1001 | 4,004 |

`frame_index_to_ticks` uses checked `i128` multiplication/division and requires
an exact result. Unsupported rates return a typed error rather than rounding.
`ticks_to_frame_index` uses documented floor semantics for display/diagnostics;
it is not authoritative edit storage.

Source media time is a separate type (`SourceTicks`) with its own timebase.
M2 supports only exact speed=1 source mappings whose source timebase matches
120,000 ticks/second. Other source timebases, speeds, and unknown mappings are
rejected by trim/split commands. This is intentionally conservative until a
media/PTS policy is implemented.

Timeline content end is derived from clip ranges. Display seconds are generated
only in the UI projection and are never written back into engine state.

## Alternatives considered

- **`f64` seconds**: simple, but violates exact editing, frame boundaries, and
  drift requirements.
- **Microseconds (`i64`)**: exact for integer microseconds but not for
  24000/1001 or 30000/1001 frame boundaries; would require silent rounding.
- **Arbitrary rational `Time { num, den }`**: maximally general but introduces
  normalization, overflow, and comparison complexity before the editor has a
  proven need. Can be considered later behind a migration.
- **Per-frame integer indices**: exact for one constant rate but complicates
  mixed-rate source mapping and sub-frame edit positions.

## Consequences and risks

- The supported rate set is explicit; adding a rate requires proving exact
  tick conversion or extending the representation.
- i64 ticks cover very long sequences; intermediate i128 checks reject overflow.
- Source PTS conversion is deliberately narrow in M2 and must be expanded with
  real media fixtures and a separate ADR before media-backed editing.
- Undo/redo and command snapshots store only project metadata, selection, and
  the engine-owned Magnet preference; no frames, GPU resources, or external
  document bytes.
- The engine remains independent of external draft JSON field names.

## Validation and rollback

Tests cover all supported rates, exact frame/tick conversions, unsupported-rate
rejection, long-sequence conversion, overflow boundaries, repeated split/trim
without drift, and source-range safety. If the representation proves
insufficient, a new ADR can migrate to rational time while preserving engine
invariants and rejecting silent precision loss.

M2.1 verified the adopted decision at frame index 10,000,000 for 24/1,
25/1, 30/1, 60/1, 24000/1001 and 30000/1001, including overflow rejection.
Native edit code contains no authoritative f32/f64 time math; UI projection is
display-only. ID allocation is monotonic across Undo/Redo and history is
metadata-bounded at 8 MiB.

M2.2 adds `NormalizeMainTrack`, which rewrites main-track starts/ends with
integer tick arithmetic only, preserving durations, source ranges and IDs.
Magnet state is captured in the same snapshots as the positions, so Undo/Redo
cannot restore a compacted project while claiming Magnet is OFF or vice versa.
