# ENH-005 — Half-Resolution Bloom Pipeline

> **Impact**: Medium — ~4× less bloom bandwidth/fragment work when bloom is on; reclaims ~¾ of the
> bloom-chain memory (three of five Rgba16Float intermediates shrink 4×; ~75 MB saved at 4K).
> **Effort**: Low–Medium (~1 day).
> **Prerequisites**: AUDIT ARC-005 merged (bloom passes gated on `bloom_enabled`) — this plan
> optimizes the enabled path.

## Goal

Run bloom extract and both blur passes at half resolution. Blur is a low-pass filter, so
downsampling first is visually near-lossless (industry standard); composite upsamples bilinearly.

## Current state (verified at HEAD 8ee42cc, post-ARC-005)

- Pass chain when bloom on (`src/app/render.rs:360-469`): scene → bloom-extract →
  H-blur → V-blur → composite (+FXAA). All intermediates are FULL window-size `Rgba16Float`
  created in `src/renderer/update.rs` (`create_render_texture` :7 and the texture set around :23;
  read `Renderer::resize` :171 to find every bloom-related texture: extract target, blur ping,
  blur pong).
- Blur shaders (`src/shaders/postprocess.wgsl`) sample with a texel-size uniform (find the
  `texel_size`/resolution field in the post uniforms — blur offsets are computed from it).
- Composite samples the final blur target with a filtering sampler (verify `FilterMode::Linear`
  in `initialization.rs` sampler; linear is required for clean upsample — if Nearest, add a
  linear sampler for the bloom slot).

## Implementation steps

1. **Size the bloom textures at half res** (`src/renderer/update.rs`): where the three bloom
   textures are created (initial + resize), use `w2 = (config.width / 2).max(1)`,
   `h2 = (config.height / 2).max(1)`. Keep the scene, composite, and FXAA targets full-size.
2. **Viewport/UVs**: each bloom pass renders a fullscreen triangle into its (now half-size)
   target — no viewport call needed (render pass inherits target size); the passes' UV mapping is
   0..1 over the target — unchanged. The ONLY sampling change: bloom-extract samples the
   FULL-size scene texture with 0..1 UVs — unchanged too (automatic downsample via linear
   filtering; confirm the extract's sampler is Linear — a 2× minification with linear filter is a
   proper box-ish downsample; acceptable for bloom).
3. **Texel-size uniforms**: the blur passes' texel size must now be `1.0 / half_size` — find where
   `PostProcessUniforms` (or per-pass uniforms) sets resolution/texel size
   (`src/renderer/update.rs:191-234` writes them) and use each pass's TARGET size, not the window
   size. This is the one real bug surface: if texel size stays full-res, blur radius visually
   halves.
4. **Composite**: samples the final blur target at 0..1 UVs with linear filtering → automatic
   upsample. No shader change (verify composite doesn't use texelFetch on the bloom texture —
   grep `textureLoad` in the composite section of `postprocess.wgsl`; if it does, switch that
   sample to `textureSample`).
5. **Bloom-threshold parity check**: extract at half res sees pre-filtered (averaged) brights —
   slightly dimmer peaks. If A/B looks weaker, compensate by lowering the bloom threshold ~10%
   or bumping intensity — expose no new setting; pick a constant that A/Bs well (document the
   chosen factor in a comment).
6. **Optional (skip if any friction)**: quarter-res for the blur pair only (extract half, blur
   quarter) — bigger perceived radius for free. Only if step 5's A/B holds up.

## Files to touch

| File | Change |
|------|--------|
| `src/renderer/update.rs` | half-size creation for 3 bloom textures (init + resize); per-pass texel-size uniforms |
| `src/renderer/uniforms.rs` | only if texel size lives in a shared uniform — split per-pass values |
| `src/shaders/postprocess.wgsl` | only if composite texelFetches bloom (switch to sampled) |
| `src/renderer/initialization.rs` | only if a Nearest sampler needs a Linear companion |

## Verification

1. `make checkall`.
2. A/B screenshots, bloom ON, same scene (a bright 3D fractal preset): pre-change vs post-change —
   bloom shape/intensity visually equivalent (allow minor softening); no half-pixel offset streaks
   (symptom of wrong texel size / half-texel misalignment).
3. Resize the window across odd sizes (e.g. 1101×733) — no crash, no edge artifacts (the `max(1)`
   and integer division must hold).
4. FPS/pass timing with bloom on (ENH-006 if present): blur passes ~4× faster; total frame
   improvement measurable at 4K.
5. Bloom OFF path untouched: identical screenshots pre/post (ARC-005's gating still bypasses all
   of this).
6. `make web-build` + browser smoke with bloom enabled.

## Rollback

One commit; revert restores full-size textures. No settings, no schema, no shader-contract
changes (unless step 4's texelFetch edit landed — included in the same revert).

## Pitfalls

- Odd window dimensions: `width/2` truncation + UV 0..1 sampling is safe, but compute texel size
  from the ACTUAL half-size ints, not `window*0.5` floats.
- The extract's linear downsample can shimmer on subpixel-bright specks during motion; if
  observed, a 4-tap box in the extract shader (sample 4 corners, average) stabilizes it — ~4 lines
  of WGSL, add only if shimmer is visible.
- Don't shrink the FXAA or composite targets — only the three bloom intermediates.
