# Par Fractal Web/WASM Build Plan

## Status: COMPLETE

All phases have been implemented. The web build compiles successfully and all tests pass.

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
| Settings persistence | YAML files | Not yet implemented |
| User presets | YAML files | Not yet implemented |
| Camera bookmarks | YAML files | Not yet implemented |
| Custom palettes | YAML files | Not yet implemented |
| Screenshots | Save to disk | Not yet implemented |
| Video recording | ffmpeg | Disabled |
| File import/export | rfd dialogs | Not yet implemented |
| GPU selection | Manual | Browser-managed |

---

## Deployment Setup

1. Push changes to main branch
2. Enable GitHub Pages (Settings > Pages > Source: GitHub Actions)
3. Add custom domain `par-fractal.pardev.net` in GitHub Pages settings
4. Ensure DNS CNAME record exists: `par-fractal.pardev.net` → `paulrobello.github.io`
5. Run `make web-deploy` or push to trigger deployment
