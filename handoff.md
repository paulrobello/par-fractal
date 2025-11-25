# Handoff Document - Strange Attractor Accumulation Mode

**Date**: 2025-11-25
**Status**: Partially working, two issues remain

## Read First

If the project has a `plan.md` file, read it before starting work for full context on the compute shader accumulation implementation.

## Current State

The compute shader-based accumulation mode for 2D strange attractors is implemented and functional, but has two outstanding issues:

### Issue 1: Log Scale Slider Has No Effect

**Symptom**: Moving the "Log Scale" slider in the UI does not visibly change the accumulation visualization.

**What was implemented**:
- `AccumulationDisplayUniforms` struct in `src/renderer/compute.rs:26-42`
- Uniform buffer created in `src/renderer/initialization.rs:661-678`
- Shader reads from uniforms in `src/shaders/postprocess.wgsl:311-312`
- Render loop writes uniforms in `src/app/render.rs:78-88`

**Possible causes to investigate**:
1. The uniform buffer may not be getting written correctly - verify `bytemuck::cast_slice` is working
2. The bind group 1 may not be bound correctly in the render pass
3. The shader formula `let max_value = 5000.0 / accum_uniforms.log_scale` may not produce visible changes with the slider range (0.1-10.0)
4. The slider value in `params.attractor_log_scale` may not be updating correctly

**Debug approach**:
- Add debug output to verify the uniform value is changing
- Try a more dramatic formula like `let max_value = hit_count * accum_uniforms.log_scale` to confirm uniforms are being read
- Check if the uniform bind group layout in the pipeline matches what's being bound

### Issue 2: Accumulator Uses Fixed Palette Instead of Selected Palette

**Symptom**: The accumulation display always uses a fixed "fire" palette (black → purple → magenta → orange → yellow → white) regardless of which palette is selected in the UI.

**Why this happens**: The accumulation display shader (`src/shaders/postprocess.wgsl:340-365`) has a hardcoded palette rather than reading from the main uniforms which contain the user-selected palette.

**What needs to be done**:
1. Pass the palette colors to the accumulation display shader
2. Either:
   - **Option A**: Add the palette as part of `AccumulationDisplayUniforms` (adds 5 × vec4 = 80 bytes)
   - **Option B**: Share the main uniform buffer with the accumulation display shader (more complex, requires matching bind group layouts)
   - **Option C**: Create a separate palette-only uniform buffer shared between shaders

**Implementation details**:
- The main fractal shader palette is defined in `src/shaders/fractal.wgsl:21` as `palette: array<vec4<f32>, 5>`
- The palette is populated from `FractalParams` in `src/fractal/mod.rs` via `to_uniforms()`
- The postprocess shader already has access to `t_scene` texture in group 0, but the accumulation display uses a different bind group

**Recommended approach**:
Option A is simplest - extend `AccumulationDisplayUniforms` to include palette colors:

```rust
pub struct AccumulationDisplayUniforms {
    pub log_scale: f32,
    pub gamma: f32,
    pub palette_offset: f32,
    pub _padding: f32,
    pub palette: [[f32; 4]; 5], // 5 colors, RGBA
}
```

Then update the shader to sample from these palette colors instead of the hardcoded gradient.

## Key Files

### Compute Infrastructure
- `src/renderer/compute.rs` - `AccumulationTexture`, `AttractorComputePipeline`, `AccumulationDisplayUniforms`
- `src/shaders/attractor_compute.wgsl` - GPU compute shader for orbit iteration
- `src/shaders/attractor_display.wgsl` - Standalone display shader (not currently used)

### Integration
- `src/renderer/initialization.rs:628-715` - Pipeline and bind group creation
- `src/renderer/initialization.rs:898-930` - Lazy initialization of compute resources
- `src/app/render.rs:23-124` - Render loop integration with accumulation mode

### Display Shader
- `src/shaders/postprocess.wgsl:297-368` - `fs_accumulation_display` function with hardcoded palette

### UI
- `src/ui/mod.rs:423-454` - Strange attractor buttons with auto-enable accumulation
- `src/ui/mod.rs:1255-1295` - Accumulation mode controls (enable, iterations, log scale, clear)

### Parameters
- `src/fractal/mod.rs` - `attractor_accumulation_enabled`, `attractor_iterations_per_frame`, `attractor_log_scale`, `attractor_total_iterations`, `attractor_pending_clear`
- `src/fractal/settings.rs` - Persistence of accumulation settings

## Testing

1. Run `make r` to start the application
2. Click on any 2D strange attractor (Hopalong, Hénon, Martin, etc.)
3. Accumulation mode should auto-enable
4. Test log scale slider - should change contrast/brightness of visualization
5. Test palette selection - should change colors (currently broken)
6. Test "Clear Accumulation" button - should reset the image

## Recent Commits

- `b3f73c7` - Auto-enable accumulation for strange attractors, improve palette
- `4bf6d20` - Fix bytes_per_row alignment for texture clear
- `74ea53d` - Simplify accumulation display bind group
- `24a32a1` - Simplify AccumulationTexture
- `5b94b7c` - Use R32Uint format for compatibility
- `852dedf` - Connect log scale slider (attempted fix, not working)

## Notes

- The R32Uint texture format was chosen for wide GPU compatibility (Rgba32Float read-write is not universally supported)
- The accumulation texture stores hit counts as unsigned integers, converted to float in the display shader
- Clearing the texture requires 256-byte row alignment per WebGPU spec
