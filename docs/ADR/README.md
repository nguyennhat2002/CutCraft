# Architecture Decision Records

One numbered Markdown file per accepted architecture decision. Status and purpose are recorded here; the decision text lives in the linked file. Historical ADR wording is retained; later ADRs may explicitly supersede or extend a specific part.

| ADR | Status | Purpose |
|---|---|---|
| [0001](0001-desktop-ui-shell.md) | Accepted — M1 | egui/eframe 0.37.0 desktop shell; UI dependencies confined to the application/UI layer |
| [0002](0002-native-timeline-timebase.md) | Accepted — M2 | 120,000-tick integer timeline timebase and rational frame-rate conversion |
| [0003](0003-native-project-persistence.md) | Accepted — M3-A; v1 format extended by 0006 | Native `.cutcraft` document identity, staged atomic save, external-change detection |
| [0004](0004-authoritative-playhead.md) | Accepted — M3-B | Authoritative session playhead, scrubbing and Split-at-Playhead |
| [0005](0005-ffmpeg-cli-preview-backend.md) | Accepted — M3-C; media binding superseded by 0006 | Runtime-discovered FFmpeg CLI probe/decode backend, PTS selection, resource bounds |
| [0006](0006-managed-autosave-media-assets.md) | Accepted — M3-D; recovery extended by 0007 | Managed project library, persistent media assets, schema v2, generation-safe AutoSave |
| [0007](0007-native-recovery-conflict-copy.md) | Accepted — M3-D.1 | Bounded recovery snapshots, recovery restore policy, external-conflict preservation, file-safety limits |
| [0008](0008-wgpu-preview-renderer.md) | Accepted — M4-A | Renderer-owned single-layer wgpu Preview pipeline reusing eframe device/queue, with egui texture fallback |

## Reading rules

- **Status** describes the decision's scope, not a claim that every consequence is implemented.
- A later ADR that changes a durable policy must link to the earlier ADR and state exactly what is superseded or extended.
- Do not rewrite an accepted ADR's historical decision merely because the implementation progressed. Add a pointer and a new ADR for changed policy.
- Research notes are kept outside this repository (RULE 000 in `../../AGENTS.md`) and are never ADRs; they must not be treated as accepted architecture without an explicit decision.