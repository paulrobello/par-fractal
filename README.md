# Par Fractal

[![Crates.io](https://img.shields.io/crates/v/par-fractal)](https://crates.io/crates/par-fractal)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Runs on Linux | MacOS | Windows | Web](https://img.shields.io/badge/runs%20on-Linux%20%7C%20MacOS%20%7C%20Windows%20%7C%20Web-blue)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)

A high-performance, cross-platform GPU-accelerated fractal renderer built with Rust and WebGPU. Features stunning 2D and immersive 3D fractal visualization with advanced rendering techniques.

![User Interface](https://raw.githubusercontent.com/paulrobello/par-fractal/main/screenshots/ui.png)

## Quick Start

```bash
# Install from crates.io
cargo install par-fractal

# Or build from source
git clone https://github.com/paulrobello/par-fractal.git
cd par-fractal
make r

# Or download pre-built binaries from releases

# Or try it in your browser (WebGPU required)
```

**[Try Par Fractal in your browser](https://par-fractal.pardev.net)** - No installation required!

See the [Quick Start Guide](docs/QUICKSTART.md) for detailed instructions.

## Features

- **GPU-Accelerated** - Efficient WebGPU rendering for 2D and 3D fractals
- **34 Fractal Types** - 19 2D and 15 3D fractals including Mandelbrot, Julia, Mandelbulb, Menger Sponge, strange attractors, and more
- **Variable Power** - Adjustable exponent (z^n + c) for 6 escape-time fractals: Multibrot, Multicorn, Multi-ship, and more
- **Advanced Rendering** - PBR shading, ambient occlusion, soft shadows, depth of field
- **Real-time Interaction** - Smooth pan/zoom, camera controls, parameter adjustment
- **Mobile Touch Support** - Full gesture support: single-finger pan, two-finger pinch zoom on iOS/Android
- **High-Quality Output** - PNG screenshots, video recording, custom resolutions
- **Productivity Tools** - Command palette, presets, bookmarks, undo/redo
- **Custom Palettes** - 48 static palettes, 12 procedural palettes (including Fractint-style), plus custom color schemes
- **Web Browser Support** - Run directly in browser via WebGPU/WASM
- **Performance Tuning** - LOD system, quality profiles, GPU selection

See [Features](docs/FEATURES.md) for complete feature documentation.

## Gallery

<table>
<tr>
<td width="50%">

**Mandelbrot Set**

![Mandelbrot](https://raw.githubusercontent.com/paulrobello/par-fractal/main/screenshots/mandelbrot.png)

</td>
<td width="50%">

**Julia Set**

![Julia](https://raw.githubusercontent.com/paulrobello/par-fractal/main/screenshots/julia.png)

</td>
</tr>
<tr>
<td>

**Menger Sponge**

![Menger Sponge](https://raw.githubusercontent.com/paulrobello/par-fractal/main/screenshots/menger_sponge.png)

</td>
<td>

**Mandelbox**

![Mandelbox](https://raw.githubusercontent.com/paulrobello/par-fractal/main/screenshots/mandelbox.png)

</td>
</tr>
<tr>
<td>

**Apollonian Gasket**

![Apollonian](https://raw.githubusercontent.com/paulrobello/par-fractal/main/screenshots/apollonian.png)

</td>
<td>

**Hopalong Attractor**

![Hopalong](https://raw.githubusercontent.com/paulrobello/par-fractal/main/screenshots/hopalong.png)

</td>
</tr>
<tr>
<td>

**Martin Attractor**

![Martin](https://raw.githubusercontent.com/paulrobello/par-fractal/main/screenshots/martin.png)

</td>
<td>

**Threeply Attractor**

![Threeply](https://raw.githubusercontent.com/paulrobello/par-fractal/main/screenshots/threeply.png)

</td>
</tr>
</table>

## Supported Fractals

### 2D Fractals (19)
**Escape-Time:** Mandelbrot, Julia, Sierpinski Carpet, Sierpinski Triangle, Burning Ship, Tricorn, Phoenix, Celtic, Newton, Lyapunov, Nova, Magnet, Collatz

**Strange Attractors:** Hopalong, Martin, Gingerbreadman, Chip, Quadruptwo, Threeply

### 3D Fractals (12)
**Ray-Marched:** Mandelbulb, Menger Sponge, Sierpinski Pyramid, Julia Set 3D, Mandelbox, Octahedral IFS, Icosahedral IFS, Apollonian Gasket, Kleinian, Hybrid Mandelbulb-Julia, Quaternion Cubic, Sierpinski Gasket

## Command Palette

Press **Ctrl/Cmd+P** to open the command palette for quick access to all features:

- **Fractal Selection** - Switch between all 31 fractal types
- **Effects** - Toggle AO, shadows, DoF, fog, bloom, FXAA
- **Color Modes** - Palette, normals, orbit traps, debug visualization
- **LOD Profiles** - Balanced, Quality First, Performance First
- **Recording** - Screenshots, MP4/WebM/GIF video recording
- **Settings** - Save/load presets, import/export configurations

Features fuzzy search matching - type partial names to filter commands.

## Documentation

### Getting Started
- [Quick Start Guide](docs/QUICKSTART.md) - Get up and running in 5 minutes
- [2D Fractals Guide](docs/FRACTALS2D.md) - Explore Mandelbrot and Julia sets
- [3D Fractals Guide](docs/FRACTALS3D.md) - Navigate Mandelbulb and Menger Sponge

### Reference
- [Features](docs/FEATURES.md) - Complete feature descriptions
- [Controls Reference](docs/CONTROLS.md) - Keyboard shortcuts and mouse actions
- [Architecture](docs/ARCHITECTURE.md) - System design and implementation
- [Documentation Index](docs/README.md) - Complete documentation overview

## Installation

### Using Cargo (Recommended)

```bash
# Install from crates.io
cargo install par-fractal

# Run the application
par-fractal
```

Requires Rust 1.70+. Install from [rustup.rs](https://rustup.rs/).

### Pre-built Binaries

Download pre-compiled binaries from the [GitHub Releases](https://github.com/paulrobello/par-fractal/releases) page:

1. Go to the [latest release](https://github.com/paulrobello/par-fractal/releases/latest)
2. Download the appropriate binary for your platform
3. Extract and run

**macOS users:** Allow the app in System Preferences → Security & Privacy if prompted.

### From Source

```bash
# Clone repository
git clone https://github.com/paulrobello/par-fractal.git
cd par-fractal

# Build and run (optimized)
make r

# Or use cargo directly
cargo run --release
```

## Basic Usage

```bash
# Run with default settings
par-fractal

# Or use the Makefile for development
make r              # Run in release mode
make build          # Build debug
make test           # Run tests
make clippy         # Run linter
make checkall       # Run all checks
```

### Key Bindings

| Shortcut | Action |
|----------|--------|
| **H** | Toggle UI panel |
| **R** | Reset view to default |
| **F9** | Take screenshot |
| **Ctrl/Cmd+P** | Open command palette |
| **Ctrl/Cmd+Z** | Undo |
| **1-4** | Quick switch fractals |
| **P** | Cycle static palettes |
| **Shift+P** | Cycle procedural palettes |

#### 2D Mode
| Shortcut | Action |
|----------|--------|
| **Mouse Drag** | Pan around |
| **Mouse Wheel** | Zoom in/out |

#### 3D Mode
| Shortcut | Action |
|----------|--------|
| **W/A/S/D** | Move forward/left/back/right |
| **Space/Shift** | Move up/down |
| **Mouse Drag** | Look around |
| **Mouse Wheel** | Adjust movement speed |

See [Controls Reference](docs/CONTROLS.md) for complete keyboard and mouse documentation.

## Platform Support

### Desktop
- **Windows** - DirectX 12 / Vulkan
- **macOS** - Metal
- **Linux** - Vulkan

### Web (WebGPU)
- **Chrome** 113+
- **Edge** 113+
- **Firefox** 141+
- **Safari** 26+

Cross-platform compatibility through WebGPU (wgpu-rs).

## Technology

- **Rust** 1.70+ - Core implementation
- **wgpu** - Cross-platform GPU API (WebGPU)
- **winit** - Window creation and event handling
- **egui** - Immediate mode GUI
- **glam** - Mathematics library
- **bytemuck** - Safe GPU data casting
- **image** - Screenshot encoding
- **Trunk** - Web/WASM build tool

## Web Build

Build and run Par Fractal in a web browser using WebGPU:

```bash
# Install Trunk (WASM build tool)
cargo install trunk

# Build for web (release)
trunk build --release

# Development server with hot reload
trunk serve
```

The web version is automatically deployed to [par-fractal.pardev.net](https://par-fractal.pardev.net) on releases.

**Note:** The web version has most features of the desktop app except video recording and file system access (presets/bookmarks use browser localStorage).

## Contributing

Contributions are welcome! Please read the contribution guidelines:

```bash
# Clone and setup
git clone https://github.com/paulrobello/par-fractal.git
cd par-fractal
cargo build

# Run quality checks
make checkall
```

All contributions must pass:
- Formatting (`cargo fmt` or `make fmt`)
- Linting (`cargo clippy` or `make clippy`)
- Tests (`cargo test` or `make test`)

## Resources

- [GitHub Repository](https://github.com/paulrobello/par-fractal)
- [Issue Tracker](https://github.com/paulrobello/par-fractal/issues)
- [Crates.io Package](https://crates.io/crates/par-fractal)
- [Documentation](docs/README.md)

## Performance Tips

For the best experience:
1. Run in release mode (`cargo run --release` or `make r`)
2. Ensure GPU drivers are up to date
3. Start with lower iteration counts, increase gradually
4. Use LOD quality profiles to balance quality and performance
5. Disable effects (DoF, soft shadows) if experiencing low FPS

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Author

Paul Robello - probello@gmail.com

## Acknowledgments

- Inspired by the mathematical beauty of fractals
- Built with the amazing Rust graphics ecosystem
- Thanks to the wgpu and egui communities

---

**Explore the infinite complexity of mathematics through the power of GPU rendering!**
