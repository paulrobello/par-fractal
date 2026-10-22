# ENH-002 — Implement the documented undo/redo shortcuts and double-click zoom

**Priority**: medium · **Estimate**: M · **Card**: filed as `[ENH-002]` with the `enhancement` tag
**Unblocks**: DOC-007 (doc alignment waits on this).

## Goal

Ship the keyboard undo/redo bindings and double-click-to-center zoom that README, FEATURES.md and
the UI tooltips already advertise, and make the three docs agree on the redo binding.

## Current state

- Undo/redo exist as UI buttons only (`ui/mod.rs` toolbar); the undo history itself works
  (ARC-015 scoped it to authored settings).
- `src/app/input.rs` has no `KeyZ`/`KeyY` handler. `ui/mod.rs:477,490` tooltips say "Ctrl+Z" /
  "Ctrl+Y"; README:248 / FEATURES:388-389 say Ctrl/Cmd+Z / Ctrl/Cmd+Shift+Z.
- No double-click handling anywhere in `src/`; FRACTALS2D.md:79,118 documents double-click to
  center and zoom for 2D fractals.

## Implementation

1. In `App::input` (or the QA-035 `CommandAction` layer if it landed first), add:
   - `Ctrl/Cmd + Z` → undo
   - `Ctrl/Cmd + Y` and `Ctrl/Cmd + Shift + Z` → redo (bind both so every documented form works;
     pick **Ctrl+Y as the canonical name** for tooltips and docs since it matches the existing
     tooltip)
   - Respect ` KeyCode modifier state` as the existing shortcut handling does.
2. Double-click (2D only): in the mouse handler, detect a double click (`click_count >= 2` from
   winit's `MouseButtonPressed`), convert the cursor position to 2D fractal coordinates (the
   same transform `zoom_at` uses), set `center_2d` to that point, zoom in one step (×2), clear
   `center_2d_precise` (per QA-033 semantics), mark scene dirty and record undo.
3. Docs (DOC-007's scope, coordinate): update README/FEATURES/FRACTALS2D to match shipped
   bindings; unify the redo name.
4. Tests: unit-test the double-click → center/zoom math; a behavior test that the undo/redo
   actions round-trip a parameter change.

## Files touched

`src/app/input.rs`, `src/app/mod.rs` (undo/redo invocation + click tracking), possibly
`src/ui/mod.rs` (tooltips, if canonical name changes), docs (DOC-007).

## Verify

- `make checkall` green.
- Manual: change a parameter → Ctrl+Z reverts, Ctrl+Shift+Z redoes; double-click a 2D fractal —
  the clicked point becomes the center, zoom doubles, and undo reverts it.
- Tooltip text matches an actual working binding.

## Rollback

Revert removes only new input handling; no persisted-state change.
