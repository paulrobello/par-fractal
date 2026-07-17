# ENH-001 — Perturbation-Theory Infinite Zoom

> **Impact**: Transformative — removes the ~1e11 zoom ceiling; 1e100+ becomes routine.
> **Effort**: Very high (2–4 weeks, phased; Phase A alone is a shippable win).
> **Prerequisites**: AUDIT ARC-002 bundle merged (hp math correct), ARC-013/ARC-001 merged
> (single `zoom_at` seam, f64 zoom), ENH-007 harness in place (this work is unverifiable without it).
> **Scope**: 2D escape-time fractals, Mandelbrot first. NOT 3D, NOT attractors.

## Goal

At any zoom the app can represent (effectively unbounded), render Mandelbrot (later Julia,
Burning Ship, Tricorn) correctly and interactively, using perturbation theory:

- CPU: one **reference orbit** per view computed in arbitrary precision.
- GPU: per-pixel **delta orbits** in plain f32 (`Δz ← 2·Z_n·Δz + Δz² + Δc`), reading `Z_n` from a
  storage buffer.
- Glitch detection (Pauldelbrot criterion) + re-basing for pixels the reference doesn't serve.

## Current state (verified at HEAD 8ee42cc)

- `zoom_2d` is f64 CPU-side after ARC-001; GPU uniform zoom is f32; center is f64 → hi/lo DF pair
  (`src/renderer/uniforms.rs:283-301`).
- DF shader math (`src/shaders/fractal.wgsl:355-438`) is correct post-ARC-002-bundle; ceiling
  ~48-49 mantissa bits ⇒ zoom ~1e11 at 1080p.
- **Storage-buffer precedent**: the Buddhabrot path already creates, binds, and reads a large
  storage buffer in a compute+render pair — `src/renderer/compute.rs` (`BuddhabrotAccumulationBuffer`,
  bind group layouts at `create_compute_storage_layout` :308) and `src/shaders/buddhabrot_*.wgsl`.
  Copy this plumbing pattern for the orbit buffer.
- Iteration scaling: `max_iterations + log2(zoom)*15` (`uniforms.rs:300`).
- No arbitrary-precision dependency exists yet in `Cargo.toml`.

## Design decisions (made here so the implementer doesn't relitigate)

1. **Arbitrary precision crate**: `dashu-float` (pure Rust, no C deps — `rug`/GMP breaks the wasm
   and cross-compile story). Precision: `max(64, ceil(log2(zoom)) + 64)` bits.
2. **Zoom representation**: keep f64 `zoom_2d` for UI/compat; add `center_precise: BigFloat`
   pair + `log2_zoom: f64` inside the new subsystem only. `FractalParams.center_2d: [f64; 2]`
   remains the low-precision mirror for everything else (UI display, presets store both; the
   precise center serializes as a decimal string in settings — `#[serde(default)]` so old files load).
3. **Where deltas run**: fragment shader (not compute) — reuses the existing fullscreen pass and
   post chain unchanged. `Δc` per pixel is computed from `pixel_offset * pixel_spacing` in f32,
   which is safe because offsets are small by construction (that is the whole point of perturbation).
4. **Reference orbit buffer**: `array<vec2<f32>>` (Z_n as f32 pairs is sufficient — deltas carry the
   precision; this is standard). Capacity = `max_iterations` clamped to 1M entries (8 MB).
5. **Activation**: perturbation replaces the DF path when `log2_zoom > 34` (≈1.7e10, safely below
   the DF ceiling); DF remains for the 1e4–1e10 band; plain f32 below (thresholds from ARC-002's
   derived gate, extended).

## Implementation steps

### Phase A — Mandelbrot MVP (shippable)

1. **New module** `src/deep_zoom/mod.rs` (+ `orbit.rs`):
   ```rust
   pub struct ReferenceOrbit {
       pub z: Vec<[f32; 2]>,      // Z_n low-precision mirror for GPU
       pub escaped_at: Option<u32>,
       pub center: (BigFloat, BigFloat),
   }
   pub fn compute_reference_orbit(center_re: &BigFloat, center_im: &BigFloat,
                                  max_iter: u32, precision_bits: u32) -> ReferenceOrbit
   ```
   Plain arbitrary-precision Mandelbrot iteration; push `(Z.re as f32, Z.im as f32)` per step;
   stop at escape (|Z|² > 4) or max_iter. Add `dashu-float` to `Cargo.toml` (workspace-checked
   version; run `cargo add dashu-float` and pin).
2. **Unit-test the orbit** against the f64 renderer from ENH-007 at shallow zoom (orbits must
   match f64 within f32 tolerance for the first ~1000 iterations).
3. **GPU plumbing** (copy Buddhabrot pattern in `renderer/compute.rs` / `initialization.rs`):
   - `OrbitBuffer { buffer: wgpu::Buffer /* STORAGE | COPY_DST */, len: u32 }`, recreated only
     when capacity grows; `queue.write_buffer` on view change.
   - Extend the main fragment bind group layout with the storage buffer at a new binding
     (read-only storage, FRAGMENT visibility). Uniform additions: `perturbation_enabled: u32`,
     `orbit_len: u32`, `ref_escaped_at: u32`, `delta_c_scale: vec2<f32>` (pixel→Δc mapping),
     `delta_c_origin: vec2<f32>` (screen-center Δc, normally 0). Follow the uniform-sync rules
     (CLAUDE.md; update BOTH structs + size assert + ARC-010 offset tests).
4. **WGSL** in `fractal.wgsl`:
   ```wgsl
   @group(0) @binding(N) var<storage, read> ref_orbit: array<vec2<f32>>;

   fn mandelbrot_perturb(delta_c: vec2<f32>, max_iterations: u32) -> f32 {
       var dz = vec2<f32>(0.0);
       var m = 0u; // reference index
       for (var i = 0u; i < max_iterations; i = i + 1u) {
           let zref = ref_orbit[m];
           // Δz ← 2·Z·Δz + Δz² + Δc  (complex ops)
           dz = cmul(2.0 * zref, dz) + cmul(dz, dz) + delta_c;
           m = m + 1u;
           let z_full = ref_orbit[m] + dz;
           let mag2 = dot(z_full, z_full);
           if (mag2 > 4.0) { return smooth_iteration_count(i, mag2, max_iterations); }
           // Pauldelbrot glitch test + rebase: if |z_full| < |dz|, rebase to z_full as new dz from orbit start
           if (mag2 < dot(dz, dz) || m >= arrayLength(&ref_orbit) - 1u) {
               dz = z_full; m = 0u;
           }
       }
       return INSIDE_SENTINEL; // match QA-023 convention
   }
   ```
   (This is the Zhuoran "rebasing" formulation — simpler and more robust than classic glitch
   *detection+re-render*; single reference suffices for the vast majority of views.)
   Wire into `fs_main`'s 2D dispatch: `if perturbation_enabled == 1u && fractal_type == 0u { ... }`.
5. **CPU driver** in `src/deep_zoom/mod.rs`: on view change (hook the same invalidation point the
   attractor uses in `app/render.rs:77-108`): if `log2_zoom > 34`, (re)compute the reference orbit
   at the view center on a worker thread (`std::thread` + channel, pattern from ARC-018), upload,
   set uniforms. Show a small "computing reference…" toast for slow orbits; keep rendering the
   previous frame meanwhile.
6. **Δc mapping**: `delta_c_scale = (4.0 / (zoom * height), y-flip …)` — derive from the existing
   coord mapping at `fractal.wgsl:2946-2972`; deltas are pixel_ndc × scale. All f32.

### Phase B — Quality & breadth
7. Julia (`Δz` iteration identical; reference orbit iterates from the view center with fixed c),
   Burning Ship and Tricorn (per-type delta recurrences — document each formula in the module),
   sharing the orbit buffer plumbing.
8. **Series approximation / BLA**: implement bilinear approximation tables (per Zhuoran/Fraktaler-3)
   to skip iterations: precompute on CPU per orbit, upload as a second storage buffer. This is
   what makes 1e50+ *fast* rather than merely correct. Gate behind a settings flag initially.
9. Progressive integration: perturbation frames render through ENH-002's progressive path when
   available (tile refinement), else full-frame.

### Phase C — UX
10. Zoom UI: display zoom as 10^x; presets/bookmarks store the decimal-string center; deep-zoom
    locations shareable as text.

## Files to touch

| File | Change |
|------|--------|
| `Cargo.toml` | + `dashu-float` |
| `src/deep_zoom/mod.rs`, `orbit.rs` | new subsystem (CPU orbit, driver, precision mgmt) |
| `src/lib.rs` | register module |
| `src/renderer/initialization.rs` | bind group layout + orbit buffer binding |
| `src/renderer/compute.rs` or new `renderer/orbit_buffer.rs` | buffer wrapper (copy Buddhabrot pattern) |
| `src/renderer/uniforms.rs` + `src/shaders/fractal.wgsl` | new uniforms (BOTH sides + asserts/tests) |
| `src/shaders/fractal.wgsl` | `mandelbrot_perturb` + dispatch wiring |
| `src/app/render.rs` / `app/update.rs` | view-change hook → orbit recompute trigger |
| `src/fractal/mod.rs`, `settings.rs` | precise-center storage (decimal string, serde default) |
| `tests/` | orbit-vs-f64 tests; harness cases at 1e12/1e20/1e50 (ENH-007) |

## Verification

1. `make checkall` at every phase boundary.
2. ENH-007 harness: add reference tiles at zoom 1e12, 1e20, 1e50 (reference rendered CPU-side with
   `dashu` at high precision — extend the harness's f64 renderer with a BigFloat mode for these).
3. Interactive: continuous zoom Mandelbrot seahorse valley (−0.7436438870, 0.1318259042) from 1
   to 1e30 — no pixelation, no glitch rings; FPS acceptable (with BLA: interactive; without: slow
   but correct).
4. Cross-backend: run on Metal (native) and `make web-build` + browser (WebGPU) — perturbation is
   f32-only so both must agree with the reference tiles.
5. Regression: zooms below 1e10 pixel-match pre-change screenshots (DF path untouched).

## Rollback

Feature-isolated: `perturbation_enabled` uniform gates all shader changes; the module is additive.
Rollback = set the activation threshold to infinity (one constant) or revert the branch. Settings
schema additions are `#[serde(default)]` — old files unaffected. Keep each phase a separate PR.

## Pitfalls for the implementer

- **Never** compute `Δc` by subtracting two large f64s per pixel on the GPU — derive from pixel
  offsets (small numbers) only. The moment a full-magnitude coordinate enters f32, precision dies.
- The orbit buffer must hold `escaped_at` handling: if the REFERENCE escapes early, pixels needing
  more iterations must rebase (the `m >= len-1` clause above) — forget it and you get black rings.
- `arrayLength` on a storage array counts the whole binding; use the `orbit_len` uniform, not
  `arrayLength`, if the buffer is over-allocated.
- wgpu storage buffers in fragment shaders require no special feature (core WebGPU) — but
  `min_storage_buffer_offset_alignment` matters if you sub-allocate; simplest is one buffer per
  purpose, offset 0.
- Keep the DF path fully intact — it is the fallback and the mid-band (1e4–1e10) renderer.
