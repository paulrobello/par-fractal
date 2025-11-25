# Handoff Document

**Date:** 2025-11-25

**Important:** Read `plan.md` in the project root before starting work - it contains detailed documentation of recent work on the accumulation compute shader system.

---

## Immediate Issue: IFS Fractal Clipping Artifacts

### Problem
The following 3D fractals have visual clipping issues where parts of the geometry appear and disappear based on camera distance:

- **Octahedral IFS** (F6)
- **Icosahedral IFS** (F7)
- **Apollonian Gasket** (F8)

Parts of these fractals clip in and out depending on how close/far the camera is from the surface.

### Likely Causes

1. **Distance estimator (DE) returning negative or unstable values** - This is the most common cause of clipping in ray marching. If the DE returns values that oscillate or go negative at certain positions, the ray marcher will overstep or get stuck.

2. **Scale/position transformations causing overflow** - The IFS fractals use repeated folding and scaling operations that may cause numerical precision issues at certain distances.

3. **Bailout radius too small** - The fractals may be escaping bailout checks before reaching proper convergence.

4. **Ray marching step size issues** - The hit threshold or step multiplier may be too aggressive for these fractal types.

### Key Files to Investigate

- `src/shaders/fractal.wgsl`:
  - `octahedral_ifs_de()` - lines ~1488-1535
  - `icosahedral_ifs_de()` - lines ~1537-1590
  - `apollonian_gasket_de()` - lines ~1592-1640

- `src/fractal/mod.rs`:
  - Default parameters for these fractals in `switch_fractal()` method
  - `fractal_scale`, `fractal_fold`, `fractal_min_radius` values

### Suggested Investigation Steps

1. **Add debug output** - Temporarily modify the distance estimator to visualize the raw distance values or iteration counts to identify where issues occur.

2. **Check for negative DE values** - Add `max(de, 0.0)` or `abs(de)` at the return to see if negative values are the issue.

3. **Compare with reference implementations** - These fractal formulas may have issues compared to known-working implementations from Shadertoy or Fractal Forums.

4. **Test with fixed parameters** - Bypass the UI parameters temporarily and use hardcoded values known to work for these fractal types.

5. **Adjust ray marching tolerance** - The fractals may need different hit distance thresholds than the default. Look for `MIN_DIST` or hit threshold constants in the ray marching code.

---

## Recently Completed Work (This Session)

### 8-Color Palette System
- Expanded palette from 5 to 8 colors
- Updated all predefined palettes (21 built-in + 27 xfractint imports)
- Updated `ColorPalette`, `CustomPalette` structs, uniforms, shaders, and UI
- Commit: `4ffb0ad`

### Toast Notifications for Palette Changes
- Added toast when cycling palettes (P key, UI buttons, command palette)
- Moved toast position from center to top-center
- Fixed toast stacking when rapidly cycling palettes
- Commits: `644db2c`, `1b7e0e9`

### Removed TgladFormula3D
- Removed fractal type and shader function
- Renumbered remaining 3D fractal IDs
- Updated F-key shortcuts
- Commit: `d6a90f4`

### UI Fixes
- Reduced custom palette preview squares from 30x30 to 20x20 to fit 8 colors
- Commit: `0d8c80b`

---

## Project State

- All tests pass (`make checkall`)
- Native build works
- Main features working: all fractals, accumulation mode for 2D attractors, screenshots, palettes

---

## Reference Documentation

- `CLAUDE.md` - Project conventions and build commands
- `docs/ARCHITECTURE.md` - Detailed architecture and data flow
- `docs/FEATURES.md` - Feature descriptions
- `docs/FRACTALS3D.md` - 3D fractal guide

---

## Build Commands

```bash
make r              # Run in release mode (recommended)
make checkall       # Run all checks: format, lint, tests
make test           # Run tests only
```
