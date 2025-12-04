# Par Fractal Web/WASM Build Plan

## Status: COMPLETE ✅
## Mobile Touch Support: IN PROGRESS 🚧

**Core web build:** All phases complete. Web build compiles and deploys successfully.
**Mobile improvements (v0.5.0):** Viewport, resize, pan working. Pinch zoom working but needs 10x speed increase.

## Overview

Add web/WASM support to Par Fractal with feature parity between desktop and web versions. Use conditional compilation with features for platform-specific code.

**Target:** WebGPU only (Chrome 113+, Edge 113+, Firefox 141+, Safari 26+)
**Build Tool:** Trunk
**Deployment:** GitHub Pages via GitHub Actions
**URL:** https://par-fractal.pardev.net

---

## Implementation Phases - All Complete

### Phase 1: Project Setup & Dependencies - DONE
- [x] Updated `Cargo.toml` with features (native/web), conditional dependencies
- [x] Added lib target with `crate-type = ["cdylib", "rlib"]`
- [x] Created `index.html` - HTML wrapper with canvas and WebGPU detection
- [x] Created `Trunk.toml` - Trunk build configuration
- [x] Updated `src/lib.rs` - Library entry point with conditional modules

### Phase 2: Platform Abstraction Layer - DONE
- [x] Created `src/platform/mod.rs` - Trait definitions (Storage, FileDialog, Capture)
- [x] Created `src/platform/native/mod.rs` - Native platform context
- [x] Created `src/platform/native/storage.rs` - File-based storage using `directories` crate
- [x] Created `src/platform/native/file_dialog.rs` - `rfd` wrapper
- [x] Created `src/platform/native/capture.rs` - Screenshot via `image` crate

### Phase 3: Web Entry Point & Event Loop - DONE
- [x] Created `src/web_main.rs` - WASM entry point with `#[wasm_bindgen(start)]`
- [x] Binary marked as native-only via `required-features = ["native"]`

### Phase 4: Web Platform Implementation - DONE
- [x] Created `src/platform/web/mod.rs` - Web platform context
- [x] Created `src/platform/web/storage.rs` - localStorage implementation
- [x] Created `src/platform/web/file_dialog.rs` - Blob download API
- [x] Created `src/platform/web/capture.rs` - PNG Blob download

### Phase 5: Disable Video Recording for Web - DONE
- [x] Added `#[cfg(feature = "native")]` to video_recorder module
- [x] Added web stub `VideoRecorder` in `src/app/mod.rs`
- [x] Added `VideoFormat` stub in `src/ui/mod.rs` for web

### Phase 6: Build & Deploy Configuration - DONE
- [x] Created `.github/workflows/deploy-web.yml` - GitHub Actions workflow
- [x] Updated `Makefile` - Added web build targets (web-install, web-build, web-serve, web-clean, web-deploy)

### Phase 7: Conditional Compilation Guards - DONE
- [x] Added cfg guards to `src/app/render.rs` for video recording and chrono
- [x] Added cfg guards to `src/app/update.rs` for persistence and high-res rendering
- [x] Added cfg guards to `src/fractal/palettes.rs` for CustomPaletteGallery
- [x] Added cfg guards to `src/fractal/presets.rs` for AppPreferences, BookmarkGallery, PresetGallery
- [x] Added cfg guards to `src/fractal/mod.rs` for save_to_file/load_from_file
- [x] Added cfg guards to `src/renderer/initialization.rs` for GPU enumeration
- [x] Added cfg guards to `src/ui/toast_ui.rs` for file opening

---

## Build Commands

```bash
# Native build (default)
cargo build --release

# Web build
trunk build --release

# Web development server
trunk serve
```

## Files Created (13 files)

| File | Purpose |
|------|---------|
| `src/web_main.rs` | WASM entry point |
| `src/platform/mod.rs` | Trait definitions |
| `src/platform/native/mod.rs` | Native platform context |
| `src/platform/native/storage.rs` | File-based storage |
| `src/platform/native/file_dialog.rs` | rfd wrapper |
| `src/platform/native/capture.rs` | Native screenshot |
| `src/platform/web/mod.rs` | Web platform context |
| `src/platform/web/storage.rs` | localStorage storage |
| `src/platform/web/file_dialog.rs` | Blob download API |
| `src/platform/web/capture.rs` | PNG Blob download |
| `index.html` | HTML wrapper with WebGPU detection |
| `Trunk.toml` | Trunk config |
| `.github/workflows/deploy-web.yml` | CI/CD |

## Files Modified

| File | Changes |
|------|---------|
| `Cargo.toml` | Add features, web deps, lib target, binary required-features |
| `src/lib.rs` | Add conditional module exports |
| `src/app/mod.rs` | Add VideoRecorder stub for web |
| `src/app/render.rs` | Add cfg guards for video/capture |
| `src/app/update.rs` | Add cfg guards for persistence |
| `src/fractal/mod.rs` | Add cfg guards for save/load |
| `src/fractal/presets.rs` | Add cfg guards for file operations |
| `src/fractal/palettes.rs` | Add cfg guards for file operations |
| `src/renderer/initialization.rs` | Separate native/web GPU init |
| `src/ui/mod.rs` | Add VideoFormat stub for web |
| `src/ui/toast_ui.rs` | Add cfg guard for file opening |
| `Makefile` | Add web build targets |

---

## Feature Comparison

| Feature | Desktop | Web |
|---------|---------|-----|
| All fractal types | Yes | Yes |
| Post-processing | Yes | Yes |
| Camera controls | Yes | Yes |
| LOD system | Yes | Yes |
| Settings persistence | YAML files | localStorage |
| User presets | YAML files | localStorage |
| Camera bookmarks | YAML files | localStorage |
| Custom palettes | YAML files | localStorage |
| Screenshots | Save to disk | PNG blob download |
| High-res render | Save to disk | PNG blob download |
| Video recording | ffmpeg | Not supported (too slow in WASM) |
| File import/export | rfd dialogs | Blob download |
| GPU selection | Manual | Browser-managed |

---

## Recent Progress Updates

### Mobile Touch Support (2025-11-26) - 95% COMPLETE

**Status**: Core functionality working. Panning works well, pinch zoom detection needs threshold tuning.

**Completed This Session (2025-11-26):**
- ✅ iOS Safari viewport fills entire screen (viewport-fit=cover, position:fixed)
- ✅ Browser window resize and orientation change handling
- ✅ Touch event routing (bypassed egui pointer blocking for touches)
- ✅ Single-finger pan working smoothly in 2D mode
- ✅ Single-finger camera rotation working in 3D mode
- ✅ Pinch zoom sensitivity increased 10x (5% → 50%)
- ✅ Pinch zoom centers at pinch point (not screen center)
- ✅ Phantom touch detection implemented (time-distance heuristic)
- ✅ Fixed touch/mouse event collision (prevented double-processing)
- ✅ Custom preset storage for web (localStorage)
- ✅ Preset UI improvements (2x taller scroll area, export buttons)

**Remaining Work:**
- ⚠️ **CRITICAL:** Phantom touch detection too strict - pinch zoom "flaky"
  - Current: `elapsed_ms < 100 && distance > 300` rejects legitimate pinches
  - Recommended: Adjust to `elapsed_ms < 50 && distance > 400` (see handoff.md)
  - Users must wait before placing second finger or zoom doesn't register
- 🔧 Remove debug logging (🔧 emoji markers) after threshold tuning complete
- 📝 Update CHANGELOG.md with final v0.5.0 notes

**Files Modified:**
- `index.html` - Viewport meta tags, iOS-specific CSS
- `src/web_main.rs` - Device pixel ratio, resize event listeners
- `src/app/mod.rs` - Added touch tracking fields (active_touches, last_touch_time)
- `src/app/input.rs` - Touch event handling, phantom detection, zoom centering
- `src/camera.rs` - Touch support for 3D camera
- `src/ui/mod.rs` - Preset scroll area height, export buttons
- `src/fractal/presets.rs` - localStorage implementation, export functions
- `CHANGELOG.md` - v0.5.0 mobile improvements documented

**Key Technical Solutions:**
1. **Phantom Touch Detection**: Time-distance heuristic rejects phantom touches from settings panel close or palm
   - Rejects if: `elapsed_ms < 100 && distance > 300` (TOO STRICT - needs tuning)
   - See `src/app/input.rs:448-472`
2. **Zoom-to-Point**: Converts pinch center to fractal coords, adjusts view center to maintain zoom point
   - See `src/app/input.rs:475-509`
3. **Preset Storage**: Web uses localStorage, native uses file system
   - Export button (💾) next to each preset downloads JSON
   - See `src/fractal/presets.rs:915-976`

**Latest Commits:**
- `f6ea910` - Phantom touch detection (time-distance heuristic)
- `3505235` - Zoom-to-pinch-center implementation
- `06db164` - Preset UI improvements (2x scroll, export buttons)

**See:** `handoff.md` for detailed threshold adjustment recommendations and testing instructions.

---

## Past Issues (Resolved)

### Strange Attractor Rendering Bug (2025-11-24) - FIXED

**Status**: Fixed. All 9 2D attractor functions updated to use distance-based coloring.

**Solution**: Changed from hit-count-based rendering to minimum distance-based rendering:
- Each pixel now finds the minimum distance to the attractor orbit
- Colors based on proximity: `1.0 - clamp(min_dist / threshold, 0.0, 1.0)`
- Distance threshold scales with zoom: `0.5 / uniforms.zoom`

**Files Modified**: `src/shaders/fractal.wgsl` lines 906-1246 (all 9 2D attractor functions)

**Note**: 3D attractors (Pickover, Lorenz, Rossler) already use distance-based approach and should work correctly.

---

### Web Video Recording - REMOVED

**Status**: Feature removed from web build.

**Reason**: GIF encoding using NeuQuant color quantization is too slow in WASM. Even with async yielding, encoding a 30-frame recording at 1720x941 takes several minutes and produces jittery results due to inconsistent frame capture timing.

**Alternative**: Users can take screenshots on web. For video recording, use the native desktop build with ffmpeg.

---

## Deployment Setup

1. Push changes to main branch
2. Enable GitHub Pages (Settings > Pages > Source: GitHub Actions)
3. Add custom domain `par-fractal.pardev.net` in GitHub Pages settings
4. Ensure DNS CNAME record exists: `par-fractal.pardev.net` → `paulrobello.github.io`
5. Run `make web-deploy` or push to trigger deployment

---

## Strange Attractor Compute Shader Accumulation (2025-11-25)

### Status: COMPLETE

A compute shader-based accumulation system for 2D strange attractors was implemented to enable millions of iterations at 60 FPS. Attractor formulas, UI controls, and screenshot capture are all working.

### Working Features
- Compute shader iterates attractor orbits and accumulates hit counts to R32Uint texture
- Auto-enables accumulation when selecting a 2D strange attractor
- Clear accumulation button works (with proper 256-byte row alignment)
- **Auto-clear on view change**: Zooming, panning, or changing attractor parameters automatically clears the accumulation to prevent smearing
- Iterations per frame slider works
- Total iterations counter displays correctly
- Density scale slider controls saturation point (default: 4.0)
- User-selected palette is used for coloring (not hardcoded)
- Palette offset animates/shifts the colors
- Accumulation mode is always enabled for strange attractors (no checkbox)
- Max iterations slider hidden for strange attractors
- **UI parameter controls** for all attractors (a, b, c sliders with Reset button)
- **Accumulation texture resize handling** - properly recreates texture with correct dimensions
- **Screenshot capture** - works correctly, captures what's displayed on screen

### Completed This Session (2025-11-25)
- Fixed Quadruptwo formula (atan argument was using log² instead of absolute²)
- Removed Icon attractor (formula was completely wrong, decided to remove)
- Added UI parameter sliders for all attractors
- Increased Threeply zoom 10x (now 50x from original)
- Fixed accumulation texture resize handling (was causing screenshot capture bug)
- Fixed screenshot capture bug - `init_accumulation_compute()` now properly recreates texture when dimensions change
- Fixed high-resolution render to use accumulation display pipeline for strange attractors

### Removed Attractors
- Hénon (boring thin curves)
- Latoocarfian (not interesting)
- Icon (formula issues, removed)

### Current Attractors (6 total)
- Hopalong - working
- Martin - working
- Gingerbreadman - working
- Chip - working
- Quadruptwo - working (formula fixed)
- Threeply - working (50x zoom applied)

### Key Files
- `src/renderer/compute.rs` - Compute infrastructure, `AccumulationDisplayUniforms` with palette
- `src/shaders/attractor_compute.wgsl` - GPU orbit iteration
- `src/shaders/postprocess.wgsl:297-374` - Display shader with palette sampling
- `src/app/render.rs:23-172` - Render loop integration
- `src/app/render.rs:377-425` - Screenshot capture code
- `src/renderer/initialization.rs:628-715` - Pipeline setup
- `src/renderer/update.rs:169-187` - Resize handling
- `src/fractal/mod.rs:591-628` - Default parameters for each attractor
- `src/ui/mod.rs:1284-1357` - Attractor parameter UI controls

---

## Additional Updates (2025-11-25 Session 2)

### 8-Color Palette System - COMPLETE
- Expanded color palettes from 5 to 8 colors
- Updated all 21 built-in palettes and 27 xfractint-imported palettes
- Updated `ColorPalette` struct, `CustomPalette` struct, `Uniforms`, WGSL shaders
- Updated UI custom palette editor with 8 color pickers
- Preview squares reduced to 20x20 to fit 8 colors

### Toast Notifications - COMPLETE
- Added toast notifications when changing palettes (UI buttons, P key, command palette)
- Moved toast position from center to top-center of screen
- Fixed toast stacking issue when rapidly cycling palettes (simple toasts now replace previous)

### Removed TgladFormula3D - COMPLETE
- Removed TgladFormula3D fractal type entirely
- Renumbered remaining 3D fractal type IDs (18-24)
- Updated F-key shortcuts: F6=Octahedral, F7=Icosahedral, F8=Apollonian, F9=Kleinian, F10=Hybrid

---

## Known Issues

### IFS Fractal Clipping Artifacts
**Status:** UNRESOLVED

The following 3D fractals have visual clipping issues:
- Octahedral IFS (F6)
- Icosahedral IFS (F7)
- Apollonian Gasket (F8)

Parts of these fractals clip in and out depending on camera distance. See `handoff.md` for investigation steps.

---

## Buddhabrot Implementation (2025-12-03)

### Status: IN PROGRESS - Buffer persistence issue

**Version:** 0.7.0

### Completed Work
- [x] Added `Buddhabrot2D` to `FractalType` enum
- [x] Created `buddhabrot_compute.wgsl` compute shader
- [x] Added `BuddhabrotComputePipeline` to renderer
- [x] Added UI selection under "2D Density Fractals"
- [x] Added "Buddhabrot Classic" preset
- [x] Updated documentation (FEATURES.md, FRACTALS2D.md)
- [x] All tests passing
- [x] Converted to atomic storage buffer (fixes race conditions)
- [x] Created `buddhabrot_copy.wgsl` for buffer-to-texture copy
- [x] Added `BuddhabrotAccumulationBuffer` struct
- [x] Fixed X-axis flip in coordinate transformation

### Current Issue
**Buffer not persisting between frames.** Yellow square flashes briefly then screen goes black. This indicates:
1. Compute shader IS running
2. Buffer-to-texture copy IS working
3. Something is **recreating the buffer every frame**

### Latest Investigation (2025-12-03)
- Disabled view_changed auto-clear for Buddhabrot
- Commented out buffer.clear() in initialization
- Issue persists - buffer/texture likely being recreated, not just cleared
- `init_buddhabrot_compute()` is called every frame and may be triggering recreation

### Next Steps
1. Add logging to `needs_buffer` and `needs_texture` checks to identify recreation trigger
2. Move initialization out of render loop (only init once when fractal type changes)
3. Check if `self.renderer.size` is changing between frames

### Files Added/Modified
- `src/shaders/buddhabrot_copy.wgsl` - NEW buffer-to-texture copy shader
- `src/renderer/compute.rs` - Added `BuddhabrotAccumulationBuffer`, `create_buddhabrot_storage_layout()`
- `src/renderer/initialization.rs` - Added `ensure_accumulation_texture_for_buddhabrot()`
- `src/app/render.rs` - Updated for atomic buffer, disabled auto-clear

### See Also
- `handoff.md` - Detailed handoff document with investigation steps
- `src/shaders/buddhabrot_compute.wgsl` - Main compute shader (has debug code enabled)
- `src/app/render.rs:25-135` - Render loop integration
