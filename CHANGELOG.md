# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.7.0]: https://github.com/paulrobello/par-fractal/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/paulrobello/par-fractal/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/paulrobello/par-fractal/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/paulrobello/par-fractal/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/paulrobello/par-fractal/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/paulrobello/par-fractal/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/paulrobello/par-fractal/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/paulrobello/par-fractal/releases/tag/v0.1.0
