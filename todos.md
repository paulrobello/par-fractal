# Par-Fractal TODOs

## Future Enhancements

### Strange Attractor Texture-Based Accumulation
**Priority**: Medium
**Complexity**: High

Implement true texture-based accumulation for strange attractors to enable much higher iteration counts (millions) while maintaining 60 FPS.

**Current limitation**: Per-pixel rendering requires every pixel to compute the full orbit, limiting practical iteration counts to ~2000-4000 before FPS drops significantly.

**Proposed approach**:
1. Use compute shaders to iterate the attractor and write orbit points directly to an accumulation texture
2. Each frame, compute a batch of orbit points (e.g., 100k-1M) and increment the corresponding texture pixels
3. Display the accumulation texture with log scaling for contrast
4. Decouples iteration count from pixel count - can accumulate indefinitely at 60 FPS
5. Add UI controls for:
   - Batch size per frame
   - Clear/reset accumulation
   - Total accumulated iterations display

**Benefits**:
- Millions of iterations at 60 FPS
- Progressive refinement (image gets more detailed over time)
- Classic attractor visualization style (density histogram)

**Files likely affected**:
- `src/shaders/` - New compute shader for attractor iteration
- `src/renderer/` - Accumulation texture management
- `src/ui/` - Controls for accumulation mode

---

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
