# Par Fractal - GPU-Accelerated Cross-Platform Fractal Renderer

I'm excited to share **Par Fractal**, a high-performance, GPU-accelerated fractal renderer I've been working on. It's built with Rust and WebGPU to deliver smooth, real-time exploration of both 2D and 3D fractals.

## What's New in v0.4.0

**Procedural Palettes** - 12 mathematically-generated color palettes using cosine-based formulas:
- **Fire Storm** - Classic Fractint `firestrm` palette with RGB phase-shifted cosines
- **Rainbow** - Full spectrum HSV hue rotation
- **Electric, Sunset, Forest, Ocean** - Themed gradient palettes
- **Grayscale, Hot, Cool** - Classic colormaps
- **Plasma, Viridis** - Scientific visualization palettes
- **Custom** - User-defined with adjustable brightness, contrast, frequency, and phase

**New Keyboard Shortcuts:**
- `Shift+P` - Cycle through procedural palettes
- `P` - Cycle through static palettes (unchanged)

**Bug Fixes:**
- Fixed command palette fractal selection not properly switching fractal type
- Fixed Rainbow procedural palette being identical to Fire Storm

## What's New in v0.3.0

**Strange Attractors** - Added 9 new strange attractor fractals for a total of 34 fractal types:
- **2D Attractors:** Hopalong, Martin, Gingerbreadman, Chip, Quadruptwo, Threeply

**Enhanced Command Palette** - New commands for shading models, fog modes, and per-channel color source selection for advanced visualization control.

**Quality Improvements** - Toast notification system prevents stacking, improved UI layout for custom palette editor.

## What Makes It Special?

**34 Fractal Types** spanning 2D escape-time, 3D ray-marched, and strange attractors:

**2D Fractals (19 types):**
- Classic fractals: Mandelbrot, Julia, Burning Ship, Tricorn
- Advanced types: Phoenix, Celtic, Newton, Nova, Lyapunov, Magnet, Collatz
- Sierpinski: Carpet and Triangle
- Strange Attractors: Hopalong, Martin, Gingerbreadman, Chip, Quadruptwo, Threeply

**3D Fractals (15 types):**
- Mandelbulb with configurable power
- Menger Sponge, Mandelbox, Julia Set 3D
- Advanced structures: Octahedral/Icosahedral IFS
- Exotic types: Apollonian Gasket, Kleinian, Hybrid Mandelbulb-Julia, Quaternion Cubic
- Sierpinski: Pyramid and Gasket

## Key Features

**Advanced Rendering:**
- Real-time GPU-accelerated rendering using WebGPU
- PBR (Physically Based Rendering) and Blinn-Phong shading for 3D fractals
- Ambient occlusion and soft shadows
- Depth of field effects
- Post-processing: Bloom, color grading
- Fog modes: Linear, Exponential, Quadratic

**Interactive Exploration:**
- Smooth camera controls with WASD + mouse
- Dynamic Level of Detail (LOD) system
- 46 static + 12 procedural color palettes, plus custom palette support
- Orbit traps and advanced coloring methods
- Command palette for quick access to features
- Per-channel color source control for advanced visualization

**Quality of Life:**
- Built-in preset system
- Undo/redo for parameter changes
- Screenshot and video recording
- Settings persistence
- Cross-platform: Windows, macOS, Linux
- Web/WASM: Try it in your browser!

## Performance

Built with Rust and leveraging modern GPU APIs (Metal on macOS, Vulkan on Linux, DirectX 12/Vulkan on Windows), Par Fractal can handle complex fractals at high resolutions with smooth real-time interaction. The adaptive LOD system maintains 60+ FPS even during camera movement.

## Try It Yourself

**Try in Browser:** [par-fractal.pardev.net](https://par-fractal.pardev.net) - No installation required!

The project is open source and available on GitHub: [github.com/paulrobello/par-fractal](https://github.com/paulrobello/par-fractal)

**Installation:**
```bash
# From source (requires Rust 1.70+)
git clone https://github.com/paulrobello/par-fractal.git
cd par-fractal
cargo run --release

# Or download pre-built binaries from the releases page
```

## Screenshots

[If you have screenshots, add them here]

## What's Next?

I'm actively developing Par Fractal and would love to hear feedback from the community! Future plans include:
- Animation timeline system
- Shader hot-reloading for experimentation
- More post-processing effects
- Additional fractal types and coloring modes

## Feedback Welcome

Whether you're a fractal enthusiast, mathematician, or just curious about beautiful math visualizations, I'd love to hear your thoughts! Feel free to:
- Try it out and share your renders
- Report bugs or request features on GitHub
- Contribute to the project (it's MIT licensed!)

Happy fractal exploring!

---

*Built with: Rust, wgpu, winit, egui, glam*
