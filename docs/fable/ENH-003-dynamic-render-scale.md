# ENH-003 — Dynamic Resolution Scaling (wire `render_scale`)

> **Impact**: High — 4–16× fragment-cost reduction during interaction, imperceptible in motion.
> **Effort**: Medium (~2–3 days).
> **Prerequisites**: AUDIT ARC-008 (LOD effective-value merge point exists). Pairs with ARC-007
> (2D motion detection) and ENH-002 (interactive/refine states) but works standalone.

## Goal

Make `QualityLevel.render_scale` (0.25–1.0) real: the fractal scene pass renders to a
reduced-size region/texture during motion, and the post chain upsamples to window size. Full
resolution returns when idle (or in ENH-002's Refining/Converged states).

## Current state (verified at HEAD 8ee42cc)

- `QualityLevel.render_scale` exists in all four presets, is serialized, lerped
  (`src/lod.rs:114-135`), UI-exposed (`src/ui/mod.rs:2502`), and displayed in the LOD overlay —
  but `apply_lod_quality()` never applies it (`src/fractal/mod.rs:1011-1012` comment admits it).
  AUDIT ARC-007 hid/annotated the slider pending this work.
- Render flow (`src/app/render.rs` + `src/renderer/update.rs`): scene pass → `scene_texture`
  (full-window Rgba16Float, created in `create_render_texture` `renderer/update.rs:7`) → bloom
  passes → composite → FXAA → surface. Every intermediate is window-sized; the composite/FXAA
  passes sample with a filtering sampler (verify the sampler filter mode — needs `Linear` for
  clean upsampling; check `initialization.rs` sampler creation).
- Textures are recreated on window resize (`Renderer::resize`, `renderer/update.rs:171`).

## Design decision: viewport-scaling, not texture-reallocation

Render the scene pass into a **sub-rect of the existing full-size `scene_texture`** using
`set_viewport(0, 0, w*scale, h*scale)`, then let the FIRST consumer of `scene_texture` (bloom
extract when bloom on; composite otherwise) remap UVs by `scale` when sampling. No per-frame
texture allocation, no bind-group churn, smooth per-frame scale changes (LOD lerps continuously).
Cost: sampled region must be clamped to avoid bleeding from the unused texture region.

## Implementation steps

1. **Plumb the effective scale**: at the ARC-008 merge point (`src/renderer/uniforms.rs`,
   `Uniforms::update`), compute `let render_scale = q.render_scale.clamp(0.25, 1.0);` from the
   active `QualityLevel` (1.0 when LOD disabled / idle / ENH-002 Refining+Converged). Store it on
   the `Renderer` (a plain field set per frame from `app/render.rs`) AND pass it to the
   post-process uniforms (see step 3).
2. **Scene pass viewport** (`src/app/render.rs`, the scene render pass): after
   `begin_render_pass`, when `render_scale < 1.0`:
   ```rust
   let sw = (config.width as f32 * render_scale).floor().max(1.0);
   let sh = (config.height as f32 * render_scale).floor().max(1.0);
   render_pass.set_viewport(0.0, 0.0, sw, sh, 0.0, 1.0);
   ```
   The fractal fragment shader computes coordinates from `@builtin(position)` or UV — **check
   which** (`fs_main` in `fractal.wgsl:2936+`): if it derives fractal coords from
   `position.xy / uniforms.resolution`, the shader must use the SCALED resolution for the mapping
   or the view will crop instead of shrink. Add `render_size: vec2<f32>` to `Uniforms` (BOTH
   sides + size assert + offset tests per CLAUDE.md rules) and use it in the coord mapping in
   place of the full resolution. Aspect ratio is unchanged (same scale both axes).
3. **UV remap in consumers** (`src/shaders/postprocess.wgsl`): the pass that samples
   `scene_texture` gets `scene_uv_scale: vec2<f32>` in `PostProcessUniforms`
   (`src/renderer/uniforms.rs` — find the post uniforms struct; add on BOTH sides):
   `let scene_uv = clamp(uv * scene_uv_scale, vec2(0.0), scene_uv_scale - texel * 0.5);`
   Apply in bloom-extract AND composite (both sample the scene — read `postprocess.wgsl` to
   enumerate scene_texture samplers; the blur passes sample bloom textures, which inherit the
   extract's output size — keep bloom at full-texture UVs by having extract WRITE full-size
   (upsampling implicitly), i.e. only the READ of scene_texture is remapped. This keeps every
   downstream pass unchanged.)
4. **Half-texel clamp** (in step 3's `clamp`): prevents linear-filter bleed from the garbage
   region beyond the viewport. Also clear `scene_texture` ONCE when scale drops below 1.0 the
   first time (stale full-res content outside the viewport is otherwise sampled by the clamp edge
   — one `LoadOp::Clear` on the transition frame, then `Load`; simplest: always `Clear` the scene
   pass — check current LoadOp; scene pass already clears each frame in the standard flow, in
   which case nothing to do).
5. **Re-enable the UI slider** (`src/ui/mod.rs:2502` or its post-QA-009 panel home): remove the
   ARC-007 annotation; range 0.25–1.0; tooltip "resolution scale during motion (LOD)".
6. **FXAA note**: FXAA runs post-composite at full resolution — it softens upsampling artifacts
   for free. No change.
7. **Optional polish** (skip if time-boxed): sharpen-on-upsample (FidelityFX-CAS-style) in
   composite when `scale < 1.0`. Not required for v1.

## Files to touch

| File | Change |
|------|--------|
| `src/renderer/uniforms.rs` | `render_size` in `Uniforms`; `scene_uv_scale` in post uniforms (both WGSL sides too) |
| `src/shaders/fractal.wgsl` | coord mapping uses `render_size` |
| `src/shaders/postprocess.wgsl` | scene_texture UV remap + clamp |
| `src/app/render.rs` | `set_viewport` on scene pass; scale source plumbing |
| `src/renderer/update.rs` | post-uniform update carries the scale |
| `src/fractal/mod.rs` / `src/lod.rs` | effective scale exposed from the ARC-008 merge point |
| `src/ui/mod.rs` (or panel file) | slider re-enabled |

## Verification

1. `make checkall` (uniform size asserts + offset tests catch layout mistakes).
2. Runtime: force scale via the LOD low preset (`render_scale 0.25`) and move the camera/zoom —
   image visibly softer but geometrically IDENTICAL in framing (no crop, no offset, no aspect
   change); release → sharp full-res returns.
3. Screenshot A/B: same view at scale 1.0 vs 0.5 — same framing, only sharpness differs. Any
   offset/crop = the step-2 coordinate mapping is wrong.
4. Bloom on + scale 0.5: no bright smearing at the right/bottom edges (clamp working).
5. ENH-007 harness green (harness runs at idle = scale 1.0; unaffected by construction — confirm).
6. FPS measurement (ENH-006 if present, else the FPS overlay): scale 0.5 during motion on a heavy
   view ⇒ scene-pass time drops ~4×.
7. `make web-build` + browser smoke.

## Rollback

Single-flag: force `render_scale = 1.0` at the step-1 merge point — all other code becomes inert
(viewport full-size, UV scale 1.0). The uniform layout additions are backward-compatible once both
sides ship together; reverting the whole change is one branch revert with no settings migration
(`render_scale` was already serialized).

## Pitfalls

- The #1 failure mode is mixing up "window resolution" vs "render resolution" in the fractal
  coord mapping — the mapping must use render_size, the post chain must use window size. Grep
  every `resolution` use in `fractal.wgsl` and audit each against this rule.
- `set_viewport` floors: keep `sw/sh ≥ 1.0` and recompute `scene_uv_scale` from the FLOORED pixel
  size (`sw / tex_width`), not from the raw scale float — off-by-a-texel here shows as edge shimmer.
- egui and screenshot capture read the FINAL surface — unaffected. But hi-res capture
  (`render_high_resolution`) builds its own textures — verify it pins scale to 1.0 (it constructs
  its own uniforms; ensure the new `render_size` there equals its full capture size).
