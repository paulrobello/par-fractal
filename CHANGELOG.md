# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.2.0]: https://github.com/paulrobello/par-fractal/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/paulrobello/par-fractal/releases/tag/v0.1.0
