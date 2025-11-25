# Par-Fractal TODOs

## In Progress

### Strange Attractor Texture-Based Accumulation (Infrastructure Complete)
**Priority**: Medium
**Complexity**: High
**Status**: Infrastructure implemented, render loop integration pending

The modular compute shader system and UI controls have been implemented. The remaining work is to wire the compute passes into the main render loop.

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
- `src/shaders/attractor_display.wgsl` - Display shader:
  - Log scaling for density visualization
  - Palette-based coloring
  - Gamma correction
- UI controls in `src/ui/mod.rs`:
  - Enable/disable accumulation mode
  - Iterations per frame slider (10k-1M)
  - Log scale adjustment
  - Total iterations counter
  - Clear accumulation button
- FractalParams fields for accumulation settings

**Remaining work**:
1. Add compute pipeline and accumulation texture to Renderer struct
2. Wire compute dispatch into render loop (before scene render pass)
3. Use accumulation texture as scene source when mode is enabled
4. Handle texture resize and clear operations
5. Test and optimize performance

**Files affected**:
- `src/renderer/mod.rs` - Add compute fields to Renderer
- `src/renderer/initialization.rs` - Initialize compute pipeline
- `src/app/render.rs` - Add compute dispatch pass
- `src/app/update.rs` - Handle accumulation clear requests

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
