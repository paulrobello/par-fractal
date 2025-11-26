# Hacker News Release

## Title (80 char limit: 71/80)
Show HN: Par Fractal – GPU-accelerated fractal renderer in Rust/WebGPU

## URL
https://github.com/paulrobello/par-fractal

## Demo URL (optional, can be included in text)
https://par-fractal.pardev.net

## Text (2000 char limit: 1847/2000)

Hey HN! I've been building Par Fractal, a GPU-accelerated fractal renderer using Rust and WebGPU. You can try it in your browser (link in my profile) or install from source.

**What it does:**

• Renders 34 fractal types: 19 2D (Mandelbrot, Julia, Burning Ship, etc.) and 15 3D (Mandelbulb, Menger Sponge, Mandelbox, etc.)

• Real-time GPU rendering with smooth camera controls

• Advanced effects: PBR shading, ambient occlusion, soft shadows, depth of field, bloom

• 58 color palettes (46 static + 12 procedural including Fractint-style Fire Storm)

• Strange attractors: Hopalong, Martin, Gingerbreadman, and more

• Command palette for quick access, presets, undo/redo, screenshot/video recording

**Technical highlights:**

Built with wgpu (cross-platform WebGPU), supporting Metal on macOS, Vulkan on Linux, and DirectX 12/Vulkan on Windows. The web version compiles to WASM and runs in modern browsers.

Adaptive LOD system maintains 60+ FPS during camera movement by dynamically adjusting quality. 3D fractals use ray marching with distance estimation functions.

**Latest updates (v0.4.0):**

Added 12 procedural palettes using cosine-based color generation, including the classic Fractint Fire Storm palette. Also added 6 new strange attractor fractals bringing the total to 34 types.

**Installation:**

```bash
cargo install par-fractal
# or
git clone https://github.com/paulrobello/par-fractal.git
cd par-fractal
cargo run --release
```

The project is MIT licensed. I'm actively developing it and would love feedback from the community! Try the web demo (link in profile) - no installation needed.

Built with: Rust, wgpu, winit, egui, glam, bytemuck

Happy to answer questions about the implementation, GPU programming, or fractal math!
