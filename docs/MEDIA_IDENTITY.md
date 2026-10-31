# CutCraft Media Identity and Locator Policy

- Status: M3-D.1 audit reference
- Applies to native `.cutcraft` media assets, duplicate/recovery copies, relink, and Preview validation
- This document describes current behavior, not a content-addressing claim.

## Five distinct questions

| Question | Current representation | What it does **not** prove |
|---|---|---|
| Same filename? | `Path::file_name` / display name | Same file, same content, same project asset |
| Same locator? | `MediaLocator { kind, value }`, compared with Windows case-insensitive and `\\?\` normalization | Same physical file or same bytes |
| Same physical file? | Best-effort canonical path via `canonicalize`; project-relative locators joined to the project directory | Symlink/junction aliases can resolve elsewhere; hard links and copies are not detected |
| Same metadata? | Persisted probe fields: stream index, codec, dimensions, rational timebase, stream start, duration, nominal/average rate, rotation | Same media content; another encode can produce identical probe metadata |
| Same actual media content? | Not checked during asset validation or recovery. There is no media-byte hash. | A replaced file with identical filename, size and probe metadata is not distinguished until decode differences are observed |

## Project asset identity

- `MediaAssetId` is the stable project-local identity. Relink preserves it while changing locator and metadata.
- A clip references `MediaAssetId`; duplicate and recovery restore preserve internal asset IDs even though the new project receives a new `ProjectId`.
- A locator is a pointer, not a content hash. Same locator is the normal case for the same file, but external replacement is still possible.
- Media bytes are referenced in place. Import, save, duplicate, recovery publication, recovery restore, and conflict copy never rewrite source media and never copy media bytes into the project.
- Project-relative locators are used only for files inside the managed project directory. They are rebased to absolute locators when a duplicate/recovered/conflict copy is created in a new directory, so the copy continues to reference the original files.
- Moving a project root keeps in-project relative media resolvable. Moving or renaming the referenced external files requires Relink.

## Detection behavior

- **Offline:** the resolved path is absent/non-file; Media Pool shows Offline and Preview reports `Media offline`.
- **Unreadable:** path exists but probe/access fails; typed unreadable state.
- **Changed:** live probe metadata differs from the persisted metadata (including stream index, codec, dimensions, timebase, start, duration, rates, rotation); relink is required.
- **Physical replacement with identical probe metadata:** not detected by validation. Do not claim content identity from locator, file size, or metadata equality.
- **Symlinks/junctions:** resolution follows the filesystem. CutCraft does not implement an OS-level real-path containment check for project-relative media; a link inside a project can point outside it. This is documented, not a security boundary.
- **UNC/drive/Unicode paths:** supported as text locators; Windows comparison is case-insensitive and strips verbatim prefixes for equivalence. Actual filesystem support depends on the OS/share.

## Recovery implications

A recovery envelope stores the same locators and metadata plus the original project path. Restoring creates a new project and rebases relative locators against the original path (or the current primary path when the original path is unavailable). If the source was offline at recovery time, the recovered project is also offline; if it was relinked after the recovery snapshot was published, the recovery snapshot still contains the prior locator. The user may need to relink the recovered copy.

No recovery, save, or duplicate operation hashes media bytes. A recovery snapshot validates the **project document**, not the media content.

## Non-guarantees

- Same filename/locator/metadata does not imply same physical file or same content.
- File size is a hint, not an integrity proof.
- Relink validates that the new probe duration covers every referencing clip; it does not prove visual identity.
- Do not promise content-addressed media or automatic detection of bit-identical/different media without an explicit future media-hash design.
