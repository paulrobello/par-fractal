# Par Fractal - GPU-Accelerated Cross-Platform Fractal Renderer

I'm excited to share **Par Fractal**, a high-performance, GPU-accelerated fractal renderer I've been working on. It's built with Rust and WebGPU to deliver smooth, real-time exploration of both 2D and 3D fractals.

## What Makes It Special?

**24 Fractal Types** spanning both 2D escape-time and 3D ray-marched fractals:

**2D Fractals:**
- Classic fractals: Mandelbrot, Julia, Burning Ship, Tricorn
- Advanced types: Phoenix, Celtic, Newton, Nova
- Experimental: Lyapunov, Magnet, Collatz, Sierpinski

**3D Fractals:**
- Mandelbulb with configurable power
- Menger Sponge, Mandelbox, Julia Set 3D
- Advanced structures: Tglad Formula, Octahedral/Icosahedral IFS
- Exotic types: Apollonian Gasket, Kleinian, Hybrid Mandelbulb-Julia

## Key Features

**Advanced Rendering:**
- Real-time GPU-accelerated rendering using WebGPU
- PBR (Physically Based Rendering) shading for 3D fractals
- Ambient occlusion and soft shadows
- Depth of field effects
- Post-processing: Bloom, FXAA anti-aliasing, color grading

**Interactive Exploration:**
- Smooth camera controls with WASD + mouse
- Dynamic Level of Detail (LOD) system
- Multiple color palettes and customizable gradients
- Orbit traps and advanced coloring methods
- Command palette for quick access to features

**Quality of Life:**
- Built-in preset system
- Undo/redo for parameter changes
- Screenshot and video recording
- Settings persistence
- Cross-platform: Windows, macOS, Linux

## Performance

Built with Rust and leveraging modern GPU APIs (Metal on macOS, Vulkan on Linux, DirectX 12/Vulkan on Windows), Par Fractal can handle complex fractals at high resolutions with smooth real-time interaction. The adaptive LOD system maintains 60+ FPS even during camera movement.

## Try It Yourself

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
- Additional fractal types
- Animation timeline system
- Shader hot-reloading for experimentation
- More post-processing effects

## Feedback Welcome

Whether you're a fractal enthusiast, mathematician, or just curious about beautiful math visualizations, I'd love to hear your thoughts! Feel free to:
- Try it out and share your renders
- Report bugs or request features on GitHub
- Contribute to the project (it's MIT licensed!)

Happy fractal exploring!

---

*Built with: Rust, wgpu, winit, egui, glam*
