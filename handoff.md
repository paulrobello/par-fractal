# Handoff Document

## Before Starting

Read `plan.md` in the project root for overall project context and architecture.

## Current Issue - Strange Attractor Rendering Bug - FIXED

### Solution Applied

Changed all 9 2D strange attractor functions from hit-count-based rendering to distance-based rendering:
- Each pixel now tracks minimum distance to the attractor orbit
- Colors based on proximity: `1.0 - clamp(min_dist / threshold, 0.0, 1.0)`
- Distance threshold scales with zoom: `0.5 / uniforms.zoom`

All tests pass (`make checkall`). Visual testing recommended.

---

## Original Problem Description (for reference)

The 12 new strange attractor fractals (added from xfractint) render as a **solid color** instead of showing the attractor patterns. When iterations are reduced to around 10, concentric circles appear instead of the expected attractor patterns.

### Affected Fractals

**2D Strange Attractors (types 26-34):**
- Hopalong2D, Henon2D, Martin2D, Gingerbreadman2D
- Latoocarfian2D, Chip2D, Quadruptwo2D, Threeply2D, Icon2D

**3D Strange Attractors (types 35-37):**
- Pickover3D, Lorenz3D, Rossler3D

### Root Cause Analysis

The issue is in the shader implementation. Strange attractors work differently from escape-time fractals:

1. **Escape-time fractals** (Mandelbrot, Julia, etc.): Each pixel computes its own iteration sequence and colors based on how quickly it escapes.

2. **Strange attractors**: A single orbit is computed from a fixed starting point, and pixels are colored based on whether the orbit passes near them.

The current implementation in `src/shaders/fractal.wgsl` (lines 906-1272) attempts orbit-based rendering but has issues:

**Problem 1: Pixel size calculation**
```wgsl
let pixel_size = 4.0 / uniforms.zoom;
```
This doesn't properly account for the screen resolution and attractor scale.

**Problem 2: Hit detection threshold**
```wgsl
if (dist < pixel_size) {
    hit_count += 1.0 - dist / pixel_size;
}
```
The threshold may be too small relative to the attractor's natural scale, causing most pixels to never register hits.

**Problem 3: Single orbit path**
Each pixel computes the same orbit from the same starting point. This means all pixels see the same orbit, but only those near the orbit path should light up.

### Suggested Fixes

#### Option A: Density-based rendering (recommended)
Instead of binary hit detection, accumulate a density field:

```wgsl
fn hopalong_attractor(coord: vec2<f32>) -> f32 {
    // ... attractor parameters ...

    var density = 0.0;
    let sigma = 0.1 / uniforms.zoom;  // Gaussian width scales with zoom

    // Skip transient
    for (var i = 0u; i < 100u; i++) { /* iterate */ }

    // Accumulate density
    for (var i = 0u; i < uniforms.max_iterations; i++) {
        // iterate attractor...
        let dist_sq = dot(vec2(x, y) - coord, vec2(x, y) - coord);
        density += exp(-dist_sq / (2.0 * sigma * sigma));
    }

    return clamp(density * 0.1, 0.0, 1.0);
}
```

#### Option B: Increase hit radius
Quick fix - increase the pixel_size multiplier:
```wgsl
let pixel_size = 20.0 / uniforms.zoom;  // Was 4.0
```

#### Option C: Distance-based coloring
Color based on minimum distance to orbit rather than hit count:
```wgsl
var min_dist = 1000.0;
for (...) {
    min_dist = min(min_dist, distance(vec2(x, y), coord));
}
return 1.0 - clamp(min_dist * uniforms.zoom * 0.5, 0.0, 1.0);
```

### Files to Modify

1. **`src/shaders/fractal.wgsl`** (lines 906-1272)
   - 2D attractor functions: `hopalong_attractor`, `henon_attractor`, etc.
   - Fix the rendering algorithm for all 9 functions

2. **`src/shaders/fractal.wgsl`** (lines 1816-1937)
   - 3D attractor distance functions: `pickover_attractor_de`, `lorenz_attractor_de`, `rossler_attractor_de`
   - These may need different fix - point cloud approach might be too expensive for ray marching

### Testing the Fix

```bash
make r  # Run in release mode
```
Then:
1. Select a strange attractor from the UI (e.g., "Hopalong" under "2D Strange Attractors")
2. Adjust zoom and iterations
3. Verify patterns appear similar to reference images from xfractint

### Reference Material

- xfractint source code: `../xfractint-20.04p16/`
- Original formulas are documented in shader comments
- Wikipedia articles on each attractor type

## Recent Commits (for context)

1. `37d62f3` - feat(palettes): import 27 xfractint color maps
2. `a7c04e9` - feat(fractals): add 12 strange attractor fractals from xfractint
3. `af5bd0b` - feat(ui): add strange attractor buttons to fractal selector

## Build Commands

```bash
make checkall  # Run all checks (format, lint, tests)
make r         # Run in release mode
make test      # Run tests only
```

## Notes

- The existing escape-time fractals (Mandelbrot, Julia, etc.) work correctly
- The 27 imported xfractint palettes work correctly
- UI buttons for new fractals work correctly
- The issue is purely in the shader rendering algorithm
