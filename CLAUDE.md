# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Par Fractal is a cross-platform GPU-accelerated fractal renderer built with Rust and WebGPU. It supports both 2D escape-time fractals (Mandelbrot, Julia, etc.) and 3D ray-marched fractals (Mandelbulb, Menger Sponge, etc.) with advanced rendering features including PBR shading, ambient occlusion, soft shadows, and depth of field.

**Tech Stack:**
- Rust 1.70+ (Edition 2021)
- wgpu (WebGPU/wgpu-rs) - Cross-platform GPU API
- winit - Window creation and event handling
- egui - Immediate mode GUI
- glam - Mathematics library
- bytemuck - Safe GPU data casting

## Development Commands

### Building and Running
```bash
make r                   # Run in release mode (recommended for performance)
make run                 # Run in debug mode
make build               # Build debug
make build-release       # Build release
make run-reset           # Run with cleared settings
```

### Testing
```bash
make test                # Run all tests with verbose output
make t                   # Shortcut for test

# Run specific test
cargo test test_name

# Run test with output
cargo test -- --nocapture
```

### Code Quality
```bash
make checkall            # Run ALL checks: format, lint, tests (auto-fixes issues)
make lint                # Run clippy with auto-fix + format
make fmt                 # Format code (cargo fmt)
make f                   # Shortcut for fmt
make clippy              # Run clippy linter
make check               # Check code without building
make c                   # Shortcut for check
```

**IMPORTANT:** Always run `make checkall` before committing. The project must pass all formatting, linting, and tests to be production-ready.

### Documentation
```bash
make doc                 # Generate and open documentation (no deps)
make doc-all             # Generate docs including dependencies
```

### Deployment
```bash
make release             # Trigger GitHub release pipeline (with confirmation)
make deploy              # Trigger GitHub 'Release and Deploy' workflow (no confirmation)
```

### Platform-Specific
```bash
make run-linux           # Force Vulkan backend
make run-macos           # Force Metal backend
make run-windows-dx12    # Force DirectX 12 (Windows)
make run-windows-vulkan  # Force Vulkan (Windows)
```

### CLI Options
```bash
par-fractal --clear-settings              # Reset all saved preferences
par-fractal --preset "preset-name"        # Load specific preset
par-fractal --list-presets                # List available presets
par-fractal --screenshot-delay 5.0        # Screenshot after 5 seconds
par-fractal --exit-delay 10.0             # Exit after 10 seconds
```

## Architecture

### Module Structure

**Core Application:**
- `main.rs` - Entry point, CLI parsing, event loop setup
- `app/mod.rs` - Main application state and event loop coordination
  - `app/input.rs` - Input event processing
  - `app/update.rs` - Frame update logic
  - `app/render.rs` - Rendering coordination
  - `app/capture.rs` - Screenshot and video capture
  - `app/persistence.rs` - Settings save/load
  - `app/camera_transition.rs` - Smooth camera animations

**Fractal System:**
- `fractal/mod.rs` - Main fractal parameter management
  - `fractal/types.rs` - Fractal type enums and definitions
  - `fractal/settings.rs` - Settings serialization
  - `fractal/palettes.rs` - Color palette definitions
  - `fractal/presets.rs` - Preset management
  - `fractal/ui_state.rs` - UI state tracking

**Rendering:**
- `renderer/mod.rs` - GPU pipeline and WGPU management
  - `renderer/initialization.rs` - GPU device/surface setup
  - `renderer/uniforms.rs` - Uniform buffer definitions
  - `renderer/update.rs` - Render pipeline execution
- `shaders/fractal.wgsl` - Main fractal computation shader
- `shaders/postprocess.wgsl` - Post-processing effects (bloom, blur, FXAA)

**Supporting Systems:**
- `camera.rs` - Camera and camera controller (3D movement)
- `lod.rs` - Level of Detail system (adaptive quality)
- `ui/mod.rs` - EGUI interface management
- `command_palette.rs` - Quick command palette
- `video_recorder.rs` - Video recording system

### Critical Data Flow

**Uniform Buffer Synchronization:**

The `FractalUniforms` struct in `renderer/uniforms.rs` MUST exactly match the `Uniforms` struct in `shaders/fractal.wgsl`. Any mismatch will cause rendering errors or crashes.

**When modifying uniforms:**
1. Update `FractalUniforms` in `renderer/uniforms.rs`
2. Update `Uniforms` in `shaders/fractal.wgsl` to match
3. Ensure field types and ordering are identical
4. Maintain 16-byte alignment for GPU compatibility
5. Verify total struct size matches using `std::mem::size_of::<FractalUniforms>()`
6. Test all fractal types after changes

**Render Pipeline:**
```
User Input → UI/Controls → FractalParams → to_uniforms() → Uniforms →
GPU Buffer Upload → WGSL Shader → Fractal Computation → Frame Output
```

### Key Design Patterns

**Parameter Management:**
- `FractalParams` is the central state object for all rendering parameters
- Changes to `FractalParams` automatically propagate to GPU uniforms
- Settings are persisted to YAML files in user config directory
- Undo/redo system tracks parameter history

**LOD System:**
- Dynamically adjusts quality based on camera movement
- Reduces iterations/steps during motion for smooth interaction
- Restores full quality when camera is stationary
- Configurable quality profiles: Low, Medium, High, Ultra

**Post-Processing Pipeline:**
- Multi-pass rendering: Scene → Bloom Extract → Blur → Composite → FXAA
- Each pass uses separate textures and bind groups
- Enables bloom, FXAA, color grading, and other effects

## Supported Fractals

**2D Fractals (13 types):**
Mandelbrot2D, Julia2D, Sierpinski2D, SierpinskiTriangle2D, BurningShip2D, Tricorn2D, Phoenix2D, Celtic2D, Newton2D, Lyapunov2D, Nova2D, Magnet2D, Collatz2D

**3D Fractals (15 types):**
Mandelbulb3D, MengerSponge3D, SierpinskiPyramid3D, SierpinskiGasket3D, JuliaSet3D, Mandelbox3D, OctahedralIFS3D, IcosahedralIFS3D, ApollonianGasket3D, Kleinian3D, HybridMandelbulbJulia3D, QuaternionCubic3D, Pickover3D, Lorenz3D, Rossler3D

## Testing Guidelines

**Test Structure:**
- Integration tests in `tests/integration_tests.rs`
- Unit tests alongside module code in `mod.rs` files (e.g., `src/fractal/tests.rs`)
- UI tests in `src/ui/tests.rs`

**IMPORTANT Testing Rules:**
- When fixing failing tests, first understand WHAT the test is validating
- Verify if the failure indicates an actual bug in the code, not just the test
- Don't simply make tests pass - ensure correctness of underlying implementation
- Run `make checkall` to verify all tests pass before considering work complete

**Common Test Patterns:**
```rust
#[test]
fn test_fractal_params_conversion() {
    let params = FractalParams::default();
    let uniforms = params.to_uniforms(/* ... */);
    // Verify uniform values match expected params
}
```

## Shader Development

**WGSL Guidelines:**
- All shaders in `src/shaders/`
- Main fractal shader: `fractal.wgsl`
- Post-processing: `postprocess.wgsl`
- Use WGSL syntax (similar to Rust)
- All uniforms must be 16-byte aligned
- Vector math uses WGSL built-ins: `length()`, `dot()`, `normalize()`, etc.

**Distance Estimators (3D):**
Each 3D fractal must provide a distance estimator function for ray marching. Distance estimators return the shortest distance to the fractal surface.

**Coloring Methods:**
- Escape-time iteration count (2D)
- Distance estimation (3D)
- Orbit traps
- Channel mixing (r, g, b from different sources)

## Performance Considerations

**Release Mode Required:**
Debug builds are significantly slower. Always use `make r` (release mode) for actual rendering work.

**GPU Selection:**
- Settings saved in `~/.config/par-fractal/settings.yaml` (Linux/macOS)
- Can specify preferred GPU index via preferences
- Automatically selects discrete GPU when available

**Quality vs Performance:**
- Lower iterations/steps for faster rendering
- Use LOD system to maintain responsiveness
- Disable expensive effects (DoF, soft shadows) for lower-end GPUs
- Reduce ray marching steps for faster 3D rendering

## Configuration Files

**Settings Location:**
- Linux/macOS: `~/.config/par-fractal/`
- Windows: `%APPDATA%\par-fractal\`

**Files:**
- `settings.yaml` - Main application settings
- `presets/` - User-saved presets

**Settings Structure:**
```yaml
fractal_params:
  fractal_type: Mandelbulb3D
  max_iterations: 500
  power: 8.0
  # ... all fractal parameters
camera:
  position: [x, y, z]
  rotation: [yaw, pitch, roll]
window:
  width: 1920
  height: 1080
preferred_gpu_index: 0
```

## Common Development Tasks

**Adding a New Fractal:**
1. Add fractal type to `FractalType` enum in `fractal/types.rs`
2. Implement distance estimator in `shaders/fractal.wgsl`
3. Add UI controls in `ui/mod.rs` if needed
4. Create preset in `fractal/presets.rs`
5. Test rendering and parameter adjustments
6. Update documentation

**Modifying Uniforms:**
1. Update `Uniforms` struct in `renderer/uniforms.rs`
2. Update matching struct in `shaders/fractal.wgsl`
3. Modify `to_uniforms()` method in `fractal/mod.rs`
4. Verify struct sizes match
5. Test all fractal types
6. Run `make checkall`

**Adding UI Controls:**
1. Modify `UI::render()` in `ui/mod.rs`
2. Update `FractalParams` in `fractal/mod.rs`
3. Ensure settings persistence in `fractal/settings.rs`
4. Add to undo/redo tracking
5. Test parameter changes

## Known Constraints

**Uniform Buffer Size:**
Current size is 784 bytes. WGPU has platform-dependent limits (typically 64KB minimum). If adding fields, maintain 16-byte alignment.

**WGSL Limitations:**
- No recursion
- Limited control flow (loops must be bounded)
- No dynamic array indexing in some contexts
- All shader code must compile for target GPU capabilities

**Platform Differences:**
- macOS: Metal backend (MoltenVK not needed)
- Linux: Vulkan backend (requires Vulkan drivers)
- Windows: DirectX 12 or Vulkan

## Documentation

Full documentation in `docs/` directory:
- `docs/ARCHITECTURE.md` - Detailed architecture and data flow
- `docs/QUICKSTART.md` - Installation and basic usage
- `docs/CONTROLS.md` - Complete keyboard/mouse reference
- `docs/FEATURES.md` - Feature descriptions
- `docs/FRACTALS2D.md` - 2D fractal guide
- `docs/FRACTALS3D.md` - 3D fractal guide

## Troubleshooting

**Black Screen:**
- Check uniform buffer synchronization
- Verify shader compiles without errors
- Ensure GPU drivers are up to date
- Check fractal parameters are within valid ranges

**Performance Issues:**
- Use release mode (`make r`)
- Lower iteration count or ray steps
- Disable expensive effects (DoF, soft shadows, AO)
- Reduce resolution or use LOD system

**Build Errors:**
- Update Rust: `rustup update`
- Clear build cache: `make clean && make build`
- Check Linux dependencies: `make install-deps`
- make sure to keep the about page in the app up to date with readme whats new