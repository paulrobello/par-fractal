# Par-Fractal TODOs

## Completed

### Strange Attractor Texture-Based Accumulation
**Priority**: Medium
**Complexity**: High
**Status**: ✅ Implemented

The compute shader-based accumulation system is fully integrated and functional.

**Implemented components**:
- `src/renderer/compute.rs` - Modular compute infrastructure:
  - `AccumulationTexture` - Reusable storage texture abstraction for accumulation effects
  - `AttractorComputePipeline` - Compute pipeline for attractor iteration
  - `AttractorComputeUniforms` - Uniform buffer for compute parameters
  - Helper functions for bind group layouts
- `src/shaders/attractor_compute.wgsl` - Compute shader supporting all 9 2D strange attractors:
  - Hopalong, Hénon, Martin, Gingerbreadman, Latoocarfian, Chip, Quadruptwo, Threeply, Icon
  - Pseudo-random orbit initialization
  - Per-thread orbit iteration with divergence handling
  - World-to-screen coordinate transformation
  - Pixel hit count accumulation
- `src/shaders/attractor_display.wgsl` - Standalone display shader (not used in final implementation)
- `src/shaders/postprocess.wgsl` - Added `fs_accumulation_display` function:
  - Log scaling for high dynamic range density visualization
  - Heat-map coloring (blue -> cyan -> green -> yellow -> red)
  - Gamma correction for better contrast
- `src/renderer/initialization.rs` - Added `accumulation_display_pipeline` and `init_accumulation_compute()`
- `src/app/render.rs` - Integrated compute dispatch into render loop
- UI controls in `src/ui/mod.rs`:
  - Enable/disable accumulation mode checkbox
  - Iterations per frame slider (10k-1M, logarithmic)
  - Log scale adjustment slider
  - Total iterations counter
  - Clear accumulation button
- FractalParams fields for accumulation settings (persisted in settings.yaml)

**How to use**:
1. Select a 2D strange attractor (Hopalong, Hénon, Martin, etc.)
2. Scroll to "Accumulation Mode (Experimental)" in 2D Parameters
3. Enable the checkbox
4. Adjust iterations per frame for speed/quality tradeoff
5. Watch the attractor build up over time
6. Use "Clear Accumulation" to reset

**Note**: Requires GPU support for Rgba32Float read-write storage textures, which may not be available on all hardware.

---

## Future Enhancements

### 3D Strange Attractors (Currently Disabled)
**Priority**: Low
**Complexity**: High

Re-enable 3D strange attractors (Lorenz, Rossler, Pickover) with a viable rendering approach.

**Why disabled**: The current ray-marching approach computes distance to a point cloud, requiring 1000-3000 attractor iterations **per ray step** (~200 steps per pixel). This results in ~300-600 billion iterations per frame, causing GPU timeout/crash.

**Possible approaches**:
1. **Instanced point rendering**: Compute attractor orbit once, render as instanced spheres/points
2. **Volumetric rendering**: Render attractor as a 3D density field (voxels or ray-traced volume)
3. **Precomputed SDF**: Bake attractor to a 3D texture SDF, sample during ray marching

**Files affected**:
- `src/shaders/fractal.wgsl` - Currently has `pickover_attractor_de`, `lorenz_attractor_de`, `rossler_attractor_de`
- `src/ui/mod.rs` - Buttons commented out
- `src/fractal/types.rs` - Types still defined: `Pickover3D`, `Lorenz3D`, `Rossler3D`

---

### Emulated Double Precision for 2D Deep Zoom
**Priority**: Medium
**Complexity**: High
**Status**: Planned

Implement emulated f64 (double-single arithmetic) for 2D fractals to enable deeper zoom levels beyond the ~10^7 limit of f32.

**Why needed**: Standard f32 has ~7 decimal digits of precision. At deep zoom levels, adjacent pixels become indistinguishable, causing blocky/pixelated artifacts. Emulated f64 would extend precision to ~15 digits, enabling zoom to ~10^15.

**Implementation approach**:
1. **Double-single arithmetic in WGSL**: Use two f32 values to represent one extended-precision number
   - Dekker/Knuth techniques for add, subtract, multiply
   - ~2-4x performance cost
2. **2D only**: Apply only to escape-time 2D fractals (Mandelbrot, Julia, etc.)
   - 3D ray marching doesn't benefit significantly (camera-based navigation, not mathematical zoom)
   - Keep f32 for 3D to maintain performance
3. **Conditional compilation**: Use shader preprocessor to select precision mode

**Reference implementations**:
- [WebGPU-WGSL-64bit-BigInt](https://github.com/Gold18K/WebGPU-WGSL-64bit-BigInt) - Arbitrary precision in WGSL
- [GAPFixFractal](https://github.com/bernds/GAPFixFractal) - Fixed-point GPU fractals (CUDA)
- XaoS project uses compile-time selectable precision (double, long double, __float128)

**Files to modify**:
- `src/shaders/fractal.wgsl` - Add double-single math functions, update 2D fractal calculations
- `src/fractal/mod.rs` - Add precision mode setting
- `src/ui/mod.rs` - UI toggle for high-precision mode

**Trade-offs**:
| Precision | Zoom Depth | Performance |
|-----------|------------|-------------|
| f32 (current) | ~10^7 | Fastest |
| Emulated f64 | ~10^15 | ~2-4x slower |

**Related GitHub issue**: #1 (Feature requests - Higher precision math)
