# Hacker News Release

## Title (80 char limit: 71/80)
Show HN: Par Fractal – GPU-accelerated fractal renderer in Rust/WebGPU

## URL
https://github.com/paulrobello/par-fractal

## Demo URL (optional, can be included in text)
https://par-fractal.pardev.net

## Text (2000 char limit: 1976/2000)

Hey HN! I've been building Par Fractal, a GPU-accelerated fractal renderer using Rust and WebGPU. You can try it in your browser (link in my profile) or install from source.

**What it does:**

• Renders 35 fractal types: 20 2D (Mandelbrot, Julia, Buddhabrot, etc.) and 15 3D (Mandelbulb, Menger Sponge, Mandelbox, etc.)

• Real-time GPU rendering with smooth camera controls

• Advanced effects: PBR shading, ambient occlusion, soft shadows, depth of field, bloom

• 60 color palettes (48 static + 12 procedural including Fractint-style Fire Storm)

• Variable power (z^n + c) for 6 escape-time fractals - explore Multibrots!

• Strange attractors: Hopalong, Martin, Lorenz, Pickover, and more

• Mobile touch support with pinch-to-zoom on iOS/Android

• Command palette, presets, undo/redo, screenshot/video recording

**Technical highlights:**

Built with wgpu (cross-platform WebGPU), supporting Metal on macOS, Vulkan on Linux, and DirectX 12/Vulkan on Windows. The web version compiles to WASM and runs in modern browsers.

Adaptive LOD system maintains 60+ FPS during camera movement. 3D fractals use ray marching with distance estimation. Buddhabrot uses compute shaders with atomic storage buffers.

**Latest updates (v0.7.1):**

New Buddhabrot fractal - density visualization of Mandelbrot escape trajectories using compute shader accumulation. Variable power for escape-time fractals. Full mobile touch support.

**Installation:**

```bash
brew tap paulrobello/par-fractal && brew install --cask par-fractal  # macOS
cargo install par-fractal  # or from crates.io
```

The project is MIT licensed. Try the web demo (link in profile) - no installation needed.

Built with: Rust, wgpu, winit, egui, glam

Happy to answer questions about implementation, GPU programming, or fractal math!
