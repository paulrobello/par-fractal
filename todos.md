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
