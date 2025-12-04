# Par Fractal - GPU-Accelerated Cross-Platform Fractal Renderer

I'm excited to share **Par Fractal**, a high-performance, GPU-accelerated fractal renderer I've been working on. It's built with Rust and WebGPU to deliver smooth, real-time exploration of both 2D and 3D fractals.

## What's New in v0.7.1

**Bug Fix** - Fixed Buddhabrot high-resolution screenshot capture using incorrect rendering pipeline.

## What's New in v0.7.0

**Buddhabrot** - A stunning density visualization of Mandelbrot escape trajectories:
- Discovered by Melinda Green in 1993, resembles a seated Buddha figure
- Uses compute shaders with atomic storage buffers for real-time accumulation
- Higher iteration counts reveal more detail in the ethereal "Buddha" shape
- Included preset: "Buddhabrot Classic" with optimized settings

## What's New in v0.6.0

**Variable Power for 2D Fractals** - Explore infinite variations with adjustable exponents (z^n + c):
- **6 fractals** now support variable power: Mandelbrot, Julia, Burning Ship, Tricorn, Phoenix, Celtic
- Power range: -32 to 32 with 0.1 step increments
- Power=3, 4, 5... creates multi-fold symmetry patterns (Multibrot, Multicorn, Multi-ship)
- Negative powers create mesmerizing inverse fractal patterns
- Dynamic escape radius with smooth coloring

**macOS App Bundle** - Native `.app` bundle support with proper icon for macOS users

## What's New in v0.5.0

**Full Mobile Touch Support** - Explore fractals on your phone or tablet:
- **iOS Safari** - Fixed viewport issues, works perfectly on iPhone/iPad with notch support
- **Single-finger pan** - Drag to move around 2D fractals or rotate 3D camera
- **Two-finger pinch zoom** - Intuitive pinch gestures for zooming
- **Browser resize** - Automatically adapts to window/orientation changes
- Smooth gesture transitions between pan and zoom modes

## Previous Highlights (v0.3.0 - v0.4.0)

**Procedural Palettes** - 12 mathematically-generated color palettes including Fire Storm (Fractint-style), Rainbow, Plasma, Viridis, and Custom with adjustable parameters.

**Strange Attractors** - 9 chaotic attractor fractals: Hopalong, Martin, Gingerbreadman, Chip, Quadruptwo, Threeply (2D) and Pickover, Lorenz, Rossler (3D).

**Enhanced Command Palette** - Shading models, fog modes, per-channel color source selection, and more.

## What Makes It Special?

**35 Fractal Types** spanning 2D escape-time, density visualization, 3D ray-marched, and strange attractors:

**2D Fractals (20 types):**
- Classic fractals: Mandelbrot, Julia, Burning Ship, Tricorn (all with variable power!)
- Advanced types: Phoenix, Celtic, Newton, Nova, Lyapunov, Magnet, Collatz
- Density visualization: Buddhabrot
- Sierpinski: Carpet and Triangle
- Strange Attractors: Hopalong, Martin, Gingerbreadman, Chip, Quadruptwo, Threeply

**3D Fractals (15 types):**
- Mandelbulb with configurable power
- Menger Sponge, Mandelbox, Julia Set 3D
- Advanced structures: Octahedral/Icosahedral IFS
- Exotic types: Apollonian Gasket, Kleinian, Hybrid Mandelbulb-Julia, Quaternion Cubic
- Sierpinski: Pyramid and Gasket
- Strange Attractors: Pickover, Lorenz, Rossler

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
- **Mobile touch support** - Pan and pinch-to-zoom on iOS/Android
- Dynamic Level of Detail (LOD) system
- 48 static + 12 procedural color palettes, plus custom palette support
- **Variable power** for escape-time fractals (Multibrot, Multicorn, etc.)
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
- **Homebrew cask** for easy macOS installation

## Performance

Built with Rust and leveraging modern GPU APIs (Metal on macOS, Vulkan on Linux, DirectX 12/Vulkan on Windows), Par Fractal can handle complex fractals at high resolutions with smooth real-time interaction. The adaptive LOD system maintains 60+ FPS even during camera movement.

## Try It Yourself

**Try in Browser:** [par-fractal.pardev.net](https://par-fractal.pardev.net) - No installation required!

The project is open source and available on GitHub: [github.com/paulrobello/par-fractal](https://github.com/paulrobello/par-fractal)

**Installation:**
```bash
# macOS (Homebrew)
brew tap paulrobello/par-fractal
brew install --cask par-fractal

# From crates.io
cargo install par-fractal

# From source (requires Rust 1.70+)
git clone https://github.com/paulrobello/par-fractal.git
cd par-fractal
cargo run --release

# Or download pre-built binaries from the releases page
```

## Screenshots

![User Interface](https://raw.githubusercontent.com/paulrobello/par-fractal/main/screenshots/ui.png)

![Mandelbrot](https://raw.githubusercontent.com/paulrobello/par-fractal/main/screenshots/mandelbrot.png)

![Buddhabrot](https://raw.githubusercontent.com/paulrobello/par-fractal/main/screenshots/buddhabrot.png)

![Menger Sponge](https://raw.githubusercontent.com/paulrobello/par-fractal/main/screenshots/menger_sponge.png)

![Hopalong Attractor](https://raw.githubusercontent.com/paulrobello/par-fractal/main/screenshots/hopalong.png)

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
