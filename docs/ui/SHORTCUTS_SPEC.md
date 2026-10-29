# Keyboard Shortcuts Specification

## Source and scope

- Visual reference: user-supplied CapCut Shortcuts dialog screenshot, 721×663 px. The reference is external and is not copied into this repository.
- CutCraft dialog target: 720×660 maximum, 640×500 minimum, centered over a dimmed backdrop.
- The feature is a real shortcut manager: profiles execute implemented commands immediately, persist to user configuration, and reject conflicting overlapping bindings. Planned commands remain visible but can never execute.

## Toolbar entry point

A compact keyboard icon sits immediately left of **Export** in the Editor project bar. Its accessible name is **Keyboard Shortcuts**; the tooltip includes the live binding (default `Ctrl+/`). The button appears between Export and the disabled Undo/Redo controls, without moving Export or the right-aligned editor chrome.

## Dialog structure

- Header: **Shortcuts** plus a close button.
- Category tabs: **Timeline**, **Player**, **Basic**, **Other**, with a clearly filled active tab.
- Profile selector: **CutCraft Default** and **Custom** only.
- Two-column scrollable command list; falls back to one column when the available width is below 560 px.
- Each row: command name, availability status, editable shortcut field, keycaps, and a clear-binding control when bound.
- Footer: **Reset** (left), layout note, **Save** and **Cancel** (right).
- Save is disabled until the pending draft differs from the active profile or has no unresolved conflict/capture.
- Keycap labels follow modifier order: Ctrl, Shift, Alt, key. Arrow keys use compact arrow glyphs.

## Registry and command identity

`crates/ui/src/shortcuts/registry.rs` is the single registry. Every command has:

- stable persisted identifier;
- display name and one or more category views;
- command context (`Global`, `Homepage`, `Editor`, `Timeline`, `Player`);
- default binding;
- implementation status;
- selection/timeline availability requirements.

Undo and Redo appear in both Timeline and Basic but share one command identity and one binding.

| Implemented command | Default | Context |
|---|---|---|
| Deselect All | Ctrl+Shift+A | Timeline |
| Snapping Toggle | Shift+S | Timeline |
| Linked Selection | Ctrl+L | Timeline |
| Zoom In | Ctrl+= | Timeline |
| Zoom Out | Ctrl+- | Timeline |
| New Project | Ctrl+N | Homepage |
| Export (opens settings) | Ctrl+E | Editor |
| Shortcuts Dialog | Ctrl+/ | Global |

All editing, playback, file, panel and workspace commands not listed above are registered as **Unavailable**. They can be rebound, but dispatch skips them and no fake handler exists.

## Execution contexts

- `Global` runs on any screen.
- `Homepage` runs only outside the Editor.
- `Editor`, `Timeline` and `Player` are Editor sub-surfaces and overlap one another.
- Homepage and Editor/Timeline/Player never overlap.
- Two commands may share a binding only when their contexts cannot be active simultaneously.

Runtime blockers take priority over context:

- any focused text edit;
- the Export, About or Shortcuts modal;
- shortcut capture mode.

## Capture behavior

- Clicking a shortcut field enters capture mode and displays **Press shortcut…**.
- Letter, number, function, arrow, Home/End/PageUp/PageDown, Insert/Delete/Backspace, Tab, Enter, Space and supported punctuation keys are accepted.
- Ctrl, Shift and Alt are normalized and exact-matched. Extra modifiers never match a shorter binding.
- Standalone modifier presses, key repeats and unsupported keys are ignored.
- Escape cancels capture and restores the prior binding.
- Alt+F4 is rejected as reserved. Windows-key combinations never reach the application, so they cannot be recorded.
- All keyboard events are swallowed while recording; application shortcuts and dialog navigation cannot execute.
- Bindings use egui's logical key, which follows the active keyboard layout (with a physical-key fallback for non-Latin layouts). The dialog displays this choice in the footer.

## Conflicts

Before applying a capture, the pending profile is searched for the same normalized binding in an overlapping context. If found:

- the new binding is not applied;
- a warning banner names the conflicting command(s);
- **Reassign** clears the conflicting binding(s) and applies the new one;
- **Cancel** leaves the previous binding unchanged.

Nonoverlapping duplicates are allowed without a warning. Loaded configurations are sanitized: later duplicate overlapping entries lose their binding with a status-bar warning.

## Profiles and persistence

- **CutCraft Default** is the built-in read-only preset. Editing a field automatically switches the pending profile to **Custom**.
- Pending edits are isolated in dialog state. Save is the only commit point.
- Cancel, X, backdrop click and Escape discard pending edits.
- Reset requires explicit confirmation and restores the pending bindings to defaults without changing the selected profile.
- Save persists and activates the selected profile immediately; menus, tooltips and keyboard dispatch read the same manager.

Windows path:

```text
%APPDATA%\CutCraft\shortcuts.json
```

Non-Windows fallback:

```text
$XDG_CONFIG_HOME/cutcraft/shortcuts.json
$HOME/.config/cutcraft/shortcuts.json
```

Format is versioned JSON:

```json
{
  "version": 1,
  "active_profile": "custom",
  "custom_bindings": {
    "timeline.zoom_in": "Ctrl+Equals",
    "timeline.split_all": null
  }
}
```

Safety:

- bounded read (1 MiB) and parse;
- malformed JSON, unsupported versions and invalid bindings fall back to defaults with a visible warning;
- unknown command IDs are ignored;
- atomic sibling-temp write, flush, then rename;
- failed saves leave the previous file and active manager unchanged;
- configuration is written only on Save, never per frame.

## Dispatch rules

`ShortcutManager::dispatch` walks registry order, skips planned/unavailable commands, consumes at most one exact key event, and returns one command. The application may execute multiple distinct events in one frame, but each event can never execute twice. Dispatch runs before widgets and respects text focus and modal state.

## Verification

Automated:

- 46 shortcut-domain tests plus UI/app integration tests cover defaults, custom profiles, exact modifiers, conflicts, overlapping/non-overlapping contexts, focus/modal suppression, clear, reset, save/cancel, persistence, reload, corrupt fallback, immediate activation, disabled commands and single-handler dispatch.
- Workspace gates: format, Clippy `-D warnings`, tests, check.

Manual evidence:

- `docs/ui/screenshots/m21-shortcuts-dialog-1600x900.png`
- `docs/ui/screenshots/m21-shortcuts-dialog-closeup.png`
- `docs/ui/screenshots/m21-shortcuts-dialog-1080x720.png`
- `docs/ui/screenshots/m21-shortcuts-toolbar-closeup.png`
- `docs/ui/screenshots/m21-shortcuts-capture.png`
- `docs/ui/screenshots/m21-shortcuts-conflict.png`

## Remaining differences from the reference

- CutCraft shows **Unavailable** beneath planned commands instead of presenting every command as working.
- CutCraft's maximum dialog height is 660 px and width 720 px to match the reference proportions while fitting the 960×600 minimum window.
- The reference contains more timeline commands; CutCraft's registry is limited to commands it can describe honestly and extend later without redesigning the dialog.
- Profile names are CutCraft Default/Custom rather than the reference's numbered presets.
- CutCraft uses its own charcoal/cyan design tokens and Lucide keyboard glyph; no CapCut artwork is reused.
