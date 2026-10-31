# ADR-0004: Session Playhead and Split-at-Playhead

- Status: Accepted for M3-B
- Date: 2026-10-31

## Context

The native engine has exact `TimelineTicks`, rational FPS, transaction history and `SplitClip`. The Timeline still painted a static zero playhead. M3-B needs interactive scrubbing and keyboard frame navigation without making UI float seconds authoritative.

## Decision

1. The playhead lives in the UI `EditorSession` as `TimelineTicks`, default zero, session-only. It is not written to `.cutcraft`, does not mark the document dirty, and does not create history entries.
2. Playhead range is `[0, native project content_end]`. Empty projects clamp to zero.
3. Pointer scrubbing converts egui points through a single `TimelineMapping`, rounds to a frame index with the project's rational `FrameRate`, and stores exact `FrameRate::frame_to_ticks` output. f64 is used only for pointer/display math.
4. Keyboard Previous/Next derives from frame indices, not repeated tick additions. Jump Start/End use zero/content end exactly.
5. After supported edits, Undo/Redo or Magnet changes, the absolute playhead time is preserved and clamped to the new content end. It is not restored by history and is not clamped upward.
6. Ruler clicks/drags are the only scrub activation surface. Clip-body clicks keep selection behavior.
7. Split-at-Playhead calls the existing engine transaction after `validate_command` checks exactly one selected main-track clip, source support, link/nested/lock restrictions and a strictly interior playhead. Split never ripples/normalizes and preserves duration.
8. Preview displays authoritative playhead timecode only; no decoded frame, thumbnail or playback is simulated.

## Consequences and validation

- No persistence schema change; opening/new resets playhead to zero.
- The UI must keep mapping/painting from the same ticks to avoid drift.
- Tests cover all supported rates, scroll/zoom/DPI-style scaling, clamping, dirty independence, split edge cases, history and persistence.
- Future decoder seeking can consume `playhead_ticks` and content end without changing this policy.
