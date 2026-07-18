# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Perturbation deep-zoom (ENH-001), Phase A + breadth.** Arbitrary-precision
  reference orbit (`dashu-float`, `src/deep_zoom/`) computed off-thread and uploaded
  as a storage buffer; per-pixel f32 delta recurrences for Mandelbrot, Julia, Burning
  Ship, and Tricorn in `shaders/fractal.wgsl`. Engages past zoom ~1.6e7
  (`PERTURBATION_LOG2_GATE = 24`) and fixes the >1e7 HP coordinate-precision collapse.
  Recurrence math CPU-verified vs direct f64 in `tests/perturb_math.rs`.
- **1e8 deep-zoom golden** (`mandel-seahorse-1e8`) — pins the collapse-fix in the
  visual-regression harness: structured output (39 distinct gray values vs the 3 of the
  pre-fix collapse) and pixel-identical across runs (MAE 0.0) via the deterministic
  `perturbation_max_iterations` budget.
- **Deep-zoom visual-regression harness (ENH-007).** A CPU f64 reference renderer
  (`src/reference.rs`) plus a byte-by-byte Rust mirror of the shader's double-float
  math, with CI-safe precision teeth in `tests/reference_math.rs` (DF-vs-f64 on
  deep-zoom tiles, error-free-transform checks on `two_prod`/`two_sum`, known-point
  and drift-table pins). A local GPU golden-image layer (`scripts/visual_test.sh`,
  `make visual-test` / `make visual-bless`) drives the real binary per manifest row
  and compares PNGs via the new `imgdiff` bin; skips cleanly on headless boxes.
- CLI flags `--screenshot-path <path>` and `--window-size <WxH>` for deterministic,
  scriptable captures.

### Changed
- `capture_screenshot` honors `--screenshot-path` (no timestamp / toast / auto-open
  in harness mode); interactive behavior is unchanged.

### Notes
- The GPU's double-float HP path was correct through ~1e7 zoom but collapsed above ~3e7
  (root-caused via the harness: a fast ~1.2 ms frame with no device-loss, but per-pixel
  coordinate precision collapsed to one shared orbit → a near-uniform image; naga's Metal
  `two_prod` EFT was intact, so the collapse was downstream Metal flush-to-zero / sub-ULP
  lo-word loss). **Now fixed by ENH-001 perturbation** (engages past ~1e6e7) and pinned by
  the 1e8 golden; deep-zoom *math* correctness is also guarded by the CI-safe CPU teeth in
  `tests/reference_math.rs`.
- The reference orbit is cheap — ~1.5 ms single / ~7.5 ms for the 9× probe at 1e8 (91 bits),
  ~1.9 ms at 1e30 (measured via `cargo test orbit_timing -- --ignored`). An earlier "~15–20 s"
  estimate was the pre-fix LOD-churn (the orbit recomputed every frame because LOD's
  `iteration_scale` varied), resolved by the deterministic budget — not the per-orbit cost.
  Phase B step 8 (BLA series-approximation tables) is therefore deferred: GPU per-pixel cost
  is sub-frame at every testable zoom, and BLA's benefit only appears at 1e50+ (unverifiable
  today, since the f64 center carries ~15 significant digits).
- The harness's optional CPU cross-check (`CROSSCHECK=1`, `imgdiff render-ref`) is now
  structurally aligned with the GPU framebuffer: `reference::pixel_to_c` maps `py = 0`
  (the screenshot's top row) to clip-space `uv.y = +1`, matching WebGPU's y-up clip
  space. Previously the y-axis was inverted, so render-ref rendered a vertically flipped
  image (the known-good 1e5 golden matched only after a vertical flip, `r(vflip)=0.73`
  vs `r(direct)=0.04`); after the fix render-ref matches the GPU directly (`r≈0.73` at
  1e5, `≈0.82` at 1e8 — the residual gap from 1.0 is post-processing: FXAA, color
  grading, bloom). It remains a qualitative gross-failure check, not a last-bit
  comparison; the CI-safe CPU teeth in `tests/reference_math.rs` guard deep-zoom
  correctness.

## [0.9.0] - 2026-07-17

Full remediation of the 2026-07-16 audit (81 issues across security, architecture, code quality, and documentation). See `git log` for per-commit detail.

### Security
- Clamp untrusted preset/settings/imported-JSON resource values at the trust boundary (rejects NaN/Inf) — closes a persistent GPU-DoS via hostile presets
- Replace all `unsafe transmute` render-pass lifetime extensions with `RenderPass::forget_lifetime()` (`app/render.rs` is now `unsafe`-free)
- Harden capture readback channels (device-lost no longer panics the render loop)
- Pin third-party CI actions (`Ilshidur/action-discord`, `rustsec/audit-check`) to commit SHAs
- Add a `cargo-audit` CI gate (`.cargo/audit.toml`); sanitize gallery filenames (CWE-22); clamp internal hi-res render resolution (CWE-789); surface a toast when imported presets are clamped

### Fixed
- **UI no longer freezes when clicking egui panels** — render-on-demand (ARC-006) parked the event loop based on egui's repaint flag, but egui only updates its pointer state during a render, so after one unresolved click it could never request the repaint it needed to resume (death spiral). Every interactive input event now forces a redraw; idle-sleep is preserved.
- **Deep-zoom correctness bundle**: high-precision (double-float) now auto-enables at zoom > 1e4 (was 1e6 — two decades late), `tricorn_hp` is reachable (gate `<=4`→`<=5`), `burning_ship_hp` DF `abs` negates both words correctly, and `two_prod` uses a Dekker split so DF no longer collapses to f32 on backends without fused FMA
- Clamp the `acos` input in LOD motion tracking — fixes permanent NaN-poisoning of the motion EMA mid-session
- Guard `max_iterations == 0` underflow in the shader; fix the inside-set sentinel (`0.0`→`-1.0`) so a legitimate first-iteration value isn't colored as interior
- GPU-index out-of-range now logs and falls back instead of unwrap-panicking on startup
- Make the Buddhabrot/attractor counter wrap explicit and documented

### Performance
- Render-on-demand: a `scene_dirty` flag + `ControlFlow::Wait` lets static scenes sleep instead of re-rendering at 60 Hz
- Gate the three full-screen bloom passes on `bloom_enabled` (off by default — was always-on)
- 2D LOD: `iteration_scale` quality lever + 2D zoom/pan motion detection (LOD previously couldn't reduce 2D cost)
- Split `fs_main` into `fs_main_2d`/`fs_main_3d` so each pipeline drops the other's register/occupancy footprint (per-entry-point DCE)
- Reusable `clear_texture`/`clear_buffer` accumulation clear (no per-frame multi-MB staging allocation during zoom/pan)

### Changed
- Migrate to winit 0.30 `ApplicationHandler` (drop deprecated `event_loop.run`)
- Store `zoom_2d` as `f64` (was `f32`); extract a single `FractalParams::zoom_at()` seam
- Stop LOD from mutating `FractalParams` — effective values are computed at uniform-build time (user slider edits are no longer clobbered; degraded values no longer persist)
- Decompose the 900-line `App::render` into `dispatch_accumulation` / `run_post_chain` / `render_ui` / `handle_ui_actions`
- Split the `FractalParams` God object into `RenderSettings` / `LodRuntime` / `AccumulationState` (undo no longer clones FPS/accumulation state); YAML schema unchanged (guarded by roundtrip tests)
- Deduplicate the native/web `App` constructors via a shared `init_common`
- Replace `UI::render`'s 11-element tuple with a named `UiActions` struct; split `ui/mod.rs` into per-panel submodules (3,349 → 604 lines)
- `#[repr(u32)]` discriminants on GPU-crossing enums (collapses the 30-arm match + triplicated channel matches); standardize logging on `log::` (keep CLI stdout)
- Move GPU enumeration off the render path (background thread + "Scanning…" UX); `VecDeque` undo history; Makefile bundle version derived from `Cargo.toml`; `typecheck` target added

### Added
- Numeric-core test suite (DF split, zoom→iteration bonus, enum discriminants, YAML/preset roundtrip, LOD NaN regression) — 54 → 114 tests
- Uniform-layout `offset_of!` tests cross-checked against the WGSL struct
- `CONTRIBUTING.md`; rustdoc for the crate and public API

### Documentation
- Correct README keybindings (F12 / `/`·Ctrl-K / E-Q), MSRV (Rust 1.85+, Edition 2024), and counts (35 fractals, 48 palettes, 864-byte uniform buffer)
- Sync ARCHITECTURE/FRACTALS3D/FEATURES (LOD 325/250/175/100, compute integrated, epsilon 0.00035), document deep-zoom thresholds, refresh the docs index, remove the unimplemented wheel-speed claim, fix stale CLAUDE.md facts

## [0.8.3] - 2026-07-07

### Changed
- Upgraded egui ecosystem to 0.35 (**egui** / **egui-wgpu** / **egui-winit** 0.34 → 0.35)
- Migrated Rust edition from 2021 to 2024
- Updated all dependencies to latest compatible versions
  - **log** → 0.4.33, **rand** → 0.10.2, **serde_json** → 1.0.150
  - **chrono** → 0.4.45, **env_logger** → 0.11.11, **open** → 5.3.6, **crossbeam-channel** → 0.5.16
  - Web: **wasm-bindgen** → 0.2.126, **web-sys** / **js-sys** → 0.3.103, **wasm-bindgen-futures** → 0.4.76, **getrandom** → 0.4.3
- **wgpu** held at 29.0.4 — wgpu 30 not adopted (no released egui-wgpu supports it yet)
- Added gitleaks secret-scanning pre-commit hook
- Bumped CI actions/checkout from v4 to v5

## [0.8.1] - 2025-12-26

### Fixed
- **Quality level CLI/URL parameters now apply** - `--quality`/`-q` and `?quality=` URL parameters were parsed but not actually wired into the initial quality level; they now take effect on startup
- Added missing `web-sys` `Location` feature required for URL parameter parsing on the web build

## [0.8.2] - 2026-04-11

### Changed
- Upgraded major dependencies to latest versions
  - **wgpu** 27 → 29.0.1 (migrated through API breaking changes)
  - **egui** / **egui-wgpu** / **egui-winit** 0.33 → 0.34.1
  - **glam** 0.31 → 0.32.1
  - **rand** 0.9 → 0.10
  - **image** → 0.25.10, **chrono** → 0.4.44, **env_logger** → 0.11.10
  - Web: **wasm-bindgen** 0.2.118, **web-sys** / **js-sys** 0.3.95, **wasm-bindgen-futures** 0.4.68, **gloo-timers** 0.4, **getrandom** 0.4.2

### Fixed
- Migrated to wgpu 29 `CurrentSurfaceTexture` enum-based surface acquisition
- Updated `InstanceDescriptor` construction for wgpu 29 (no more `Default` impl)
- Wrapped bind group layout entries in `Option` per wgpu 29 API
- Renamed `push_constant_ranges` → `immediate_size` per wgpu 28
- Added `multiview_mask` field to all render pipeline and render pass descriptors per wgpu 29
- Switched `MipmapFilterMode` for mipmap filtering per wgpu 29
- Updated `rand` API to use `RngExt` trait for `random_range`/`random_bool`
- Fixed egui 0.34 deprecations: `egui_wants_pointer_input`, `is_pointer_over_egui`, `egui_is_using_pointer`, `Context::run_ui`

## [0.8.0] - 2025-12-25

### Added

#### Quality Level CLI and URL Parameters
- **`--quality` / `-q` CLI parameter** - Set rendering quality level from command line
  - Supports: `low`, `medium`, `high`, `ultra` (and abbreviations: `l`, `m`, `h`, `u`)
  - Example: `par-fractal --quality low` or `par-fractal -q high`
- **URL parameters for web version** - Set quality and preset via URL
  - Query string: `?quality=high` or `?q=medium&preset=Mandelbulb`
  - Hash format: `#quality=low&p=Mandelbulb`
  - Preset parameter also supported: `?preset=name` or `?p=name`

### Changed
- Updated all dependencies to latest versions
  - egui ecosystem updated to 0.33.3
  - winit updated to 0.30.12
  - Various other dependency updates

## [0.7.2] - 2025-12-06

### Changed
- Updated dependencies to latest versions
- Added macOS quarantine troubleshooting documentation

## [0.7.1] - 2025-12-04

### Fixed
- **Buddhabrot screenshot capture** - Fixed high-resolution screenshots incorrectly using the standard fractal pipeline instead of the accumulation display pipeline for Buddhabrot
  - Changed `is_2d_attractor()` check to `uses_accumulation()` to properly include Buddhabrot in accumulation mode rendering

## [0.7.0] - 2025-12-03

### Added

#### New Fractal Type - Buddhabrot
- **Buddhabrot 2D** - Density visualization of Mandelbrot escape trajectories
  - Discovered by Melinda Green in 1993, resembles a seated Buddha figure
  - Uses compute shaders with atomic storage buffers for accumulation
  - Renders escape paths that leave the Mandelbrot set
  - Higher iteration counts reveal more detail in the "Buddha" shape
  - Preset: "Buddhabrot Classic" with optimized settings

### Changed
- Total fractal count increased from 34 to 35 (20 2D + 15 3D)
- Gallery updated with Buddhabrot and Julia 3D screenshots

## [0.6.1] - 2025-11-29

### Added
- **macOS App Bundle Support** - `make bundle` and `make run-bundle` targets for creating and running a proper macOS .app bundle with icon.

## [0.6.0] - 2025-11-26

### Added

#### Variable Power for 2D Fractals
- **Power slider for 6 escape-time 2D fractals** - Mandelbrot, Julia, Burning Ship, Tricorn, Phoenix, and Celtic fractals now support variable power (z^n + c)
  - Power range: -32 to 32 with 0.1 step increments
  - Classic fractals use power=2 by default
  - Power=3, 4, 5... creates multi-fold symmetry patterns (Multibrot, Multicorn, Multi-ship)
  - Negative powers create inverse fractal patterns
  - Smooth coloring adjusted for variable power with dynamic escape radius

### Fixed

#### Palette Animation
- **Fixed color jump when changing animation speed** - Palette animation now uses delta-time accumulation instead of elapsed-time multiplication
  - Changing speed no longer causes colors to jump to a different position
  - Animation continues smoothly from current position when speed is adjusted
  - Properly handles reverse animation direction with `rem_euclid`

## [0.5.0] - 2025-11-26

### Fixed

#### Web/Mobile Support
- **iOS Safari viewport fix** - Application now properly fills the entire viewport on iPhone devices
  - Added `viewport-fit=cover` and `maximum-scale=1.0` to viewport meta tag for proper handling of devices with notches
  - Fixed canvas sizing to use device pixel ratio for crisp rendering on high-DPI displays
  - Added `touch-action: none` and other iOS-specific CSS properties for proper touch event handling
  - Fixed position: fixed layout to prevent iOS Safari layout issues
  - Added `overscroll-behavior: none` to prevent pull-to-refresh and overscroll bounce
- **Browser window resize support** - Application now properly resizes when browser window is resized or device orientation changes
  - Added event listeners for `resize` and `orientationchange` events
  - Automatically updates canvas dimensions and notifies app of size changes
  - Properly handles device pixel ratio changes during resize
- **Touch panning and camera control** - Touch gestures now work correctly on mobile devices
  - Added explicit `WindowEvent::Touch` handling for 2D panning in `handle_2d_input()`
  - Added touch event support to 3D camera controller for rotation/looking around
  - Touch events properly map to pan/drag behavior (TouchPhase::Started/Moved/Ended)
  - Single-finger drag now works for both 2D fractal panning and 3D camera rotation
  - Disabled text selection and tap highlighting during touch interactions
- **Pinch-to-zoom gesture support** - Two-finger pinch gestures now control zoom on mobile
  - Multi-touch tracking with HashMap to manage multiple simultaneous touch points
  - Pinch-in/out gestures smoothly zoom 2D fractals in/out
  - Zoom center calculated between two fingers for intuitive zooming
  - Automatic transition between pan (1 finger) and zoom (2 fingers) modes
  - Smooth zoom factor calculation with 50% sensitivity for responsive control
  - Fixed touch state management - gestures now work reliably after UI interactions
  - Settings panel defaults to hidden on web builds for immediate gesture testing
  - Removed phantom touch detection - no longer needed with hidden settings panel
  - Natural simultaneous and delayed pinch gestures fully supported (no artificial timing constraints)

## [0.4.0] - 2025-11-25

### Added

#### Procedural Palettes
- **12 mathematically-generated color palettes** using cosine-based formulas:
  - **Fire Storm** - Classic Fractint `firestrm` palette with RGB phase-shifted cosines
  - **Rainbow** - Full spectrum HSV hue rotation
  - **Electric** - Cyan to blue to purple gradient
  - **Sunset** - Warm oranges to purples
  - **Forest** - Greens and earth tones
  - **Ocean** - Deep blues to cyan
  - **Grayscale** - Simple black to white
  - **Hot** - Black to red to yellow to white
  - **Cool** - Cyan to magenta gradient
  - **Plasma** - Purple to orange (scientific visualization)
  - **Viridis** - Perceptually uniform (scientific visualization)
  - **Custom** - User-defined cosine palette with adjustable brightness, contrast, frequency, and phase parameters

#### Command Palette Enhancements
- Added "Next Procedural Palette" command (`Shift+P`) to cycle through procedural palettes
- Renamed "Next Palette" to "Next Static Palette" for clarity

#### Keyboard Shortcuts
- `Shift+P` - Cycle procedural palette (new)
- `P` - Cycle static palette (unchanged)

### Changed
- Procedural palettes use GPU-computed colors for smooth, continuous gradients
- Palette animation works with both static and procedural palettes
- UI shows procedural palette preview and custom parameter controls

### Fixed
- Fixed command palette fractal selection not properly switching fractal type (was not calling `switch_fractal()`)
- Fixed Rainbow procedural palette being identical to Fire Storm (now uses proper HSV hue rotation)

## [0.3.0] - 2025-11-25

### Added

#### New Fractal Types - Strange Attractors
- **2D Strange Attractors (6 new):**
  - Hopalong - Barry Martin's hopalong attractor
  - Martin - Barry Martin's original attractor
  - Gingerbreadman - Chaotic 2D map
  - Chip - Chip attractor variant
  - Quadruptwo - Quadruptwo strange attractor
  - Threeply - Threeply strange attractor

- **3D Strange Attractors (3 new):**
  - Pickover - Clifford Pickover's chaotic attractor
  - Lorenz - Classic Lorenz butterfly attractor
  - Rossler - Rossler system attractor

#### Command Palette Enhancements
- Added all new strange attractor fractals to command palette
- Added shading model commands (Blinn-Phong, PBR)
- Added fog mode commands (Linear, Exponential, Quadratic)
- Added per-channel color source commands for PerChannel color mode:
  - Red/Green/Blue channel sources: Iterations, Distance, Position X/Y/Z, Normal, AO, Constant
- Toast notifications for palette changes to prevent notification stacking

### Changed
- Total fractal count increased from 26 to 34 (19 2D + 15 3D)

### Removed
- TgladFormula3D fractal type (consolidated into other IFS fractals)

### Fixed
- Fixed toast notification stacking when rapidly changing palettes
- Reduced custom palette preview squares from 30x30 to 20x20 for better UI fit

## [0.2.0] - 2025-11-24

### Added

#### Web/WASM Support
- WebGPU/WASM build support via Trunk
- Web deployment to [par-fractal.pardev.net](https://par-fractal.pardev.net)
- Platform abstraction layer for native and web builds
- Web-specific implementations for storage (localStorage), file dialogs (Blob downloads), and screenshots
- GitHub Actions workflow for automatic web deployment

#### New Fractal Types
- Sierpinski Triangle (2D) - Classic self-similar triangle fractal
- Sierpinski Gasket (3D) - 3D version of the Sierpinski fractal

#### Command Palette Enhancements
- Organized commands into categories: Fractal, Preset, Effect, Color, Camera, Recording, LOD, UI, Settings, Debug
- Fuzzy search matching for quick command filtering
- Keyboard shortcuts displayed for common commands
- Aliases for flexible command matching (e.g., "mb" for Mandelbrot)
- 10+ new commands including:
  - LOD profile switching (Balanced, Quality First, Performance First, Distance Only, Motion Only)
  - Color mode switching (17 modes including debug visualizations)
  - Effect toggles (AO, shadows, DoF, fog, bloom, vignette, FXAA, SSR, floor)
  - Recording commands (MP4, WebM, GIF)
  - Settings management (save/load presets, import/export)

### Changed
- Reorganized platform-specific code into `src/platform/` module structure
- Video recording disabled on web platform (requires ffmpeg)
- GPU selection handled by browser on web platform

### Fixed
- Improved conditional compilation guards for native-only features

## [0.1.0] - 2025-11-24

### Added
- Initial release
- GPU-accelerated rendering via WebGPU (wgpu-rs)
- 12 2D fractal types: Mandelbrot, Julia, Sierpinski Carpet, Burning Ship, Tricorn, Phoenix, Celtic, Newton, Lyapunov, Nova, Magnet, Collatz
- 12 3D fractal types: Mandelbulb, Menger Sponge, Sierpinski Pyramid, Julia Set 3D, Mandelbox, Tglad Formula, Octahedral IFS, Icosahedral IFS, Apollonian Gasket, Kleinian, Hybrid Mandelbulb-Julia, Quaternion Cubic
- Advanced rendering features:
  - PBR and Blinn-Phong shading models
  - Ambient occlusion
  - Soft shadows
  - Depth of field
  - Fog (linear, exponential, quadratic)
  - Bloom
  - FXAA anti-aliasing
  - Screen-space reflections
  - Ground plane with reflections
- Camera system with smooth animations and bookmarks
- LOD (Level of Detail) system with multiple profiles
- 6 built-in color palettes plus custom palette support
- Command palette (Ctrl/Cmd+P) for quick access to features
- Preset system for saving and loading configurations
- Undo/redo history
- Screenshot capture (PNG)
- Video recording (MP4, WebM, GIF via ffmpeg)
- Settings persistence (YAML)
- Cross-platform support: Windows (DX12/Vulkan), macOS (Metal), Linux (Vulkan)

<!-- v0.8.2 has no git tag; compare anchors use v0.8.1 as the base. -->
[0.9.0]: https://github.com/paulrobello/par-fractal/compare/v0.8.3...v0.9.0
[0.8.3]: https://github.com/paulrobello/par-fractal/compare/v0.8.1...v0.8.3
[0.8.2]: https://github.com/paulrobello/par-fractal/compare/v0.8.1...v0.8.2
[0.8.1]: https://github.com/paulrobello/par-fractal/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/paulrobello/par-fractal/compare/v0.7.2...v0.8.0
[0.7.2]: https://github.com/paulrobello/par-fractal/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/paulrobello/par-fractal/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/paulrobello/par-fractal/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/paulrobello/par-fractal/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/paulrobello/par-fractal/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/paulrobello/par-fractal/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/paulrobello/par-fractal/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/paulrobello/par-fractal/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/paulrobello/par-fractal/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/paulrobello/par-fractal/releases/tag/v0.1.0
