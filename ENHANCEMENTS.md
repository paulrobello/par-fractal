# Enhancement Ideas — par-fractal

> **Date**: 2026-07-16 · **HEAD**: `8ee42cc` (v0.8.3)
> **Companion to**: `AUDIT.md` (defect findings). These are *opportunities beyond defects*,
> prioritized for the user focus: **performance and infinite zoom**.
> Each idea has a full implementation plan in `docs/fable/ENH-XXX-<slug>.md`, written to be
> executable by a smaller model without re-analysis.
>
> Graph context (par-mem, repo_id `par-fractal`): 42 Rust files / 1 WGSL megashader, 104 functions +
> 277 methods, 577 call edges. Complexity concentrates in `UI::render` (CC 288), `Uniforms::update`
> (CC 97), `App::input` (CC 65), `App::render` (CC 63) — all addressed by AUDIT remediation; the
> ideas below build on that foundation. Ordering assumes the AUDIT Phase 1–2 fixes land first
> (especially the ARC-002 deep-zoom correctness bundle and ARC-008 LOD ownership).

> **Status — verified 2026-07-19 at HEAD `5580784`:** ENH-007 (visual regression harness)
> shipped complete and was **removed from this list** — its CPU teeth (`tests/reference_math.rs`)
> and GPU goldens (`tests/golden/`) remain the regression backbone for every remaining item.
> Of the 7 still open: **3 effectively done (ENH-001, ENH-002, ENH-003); ENH-005,
> ENH-006, and ENH-008 done; 1 partial (ENH-004).** ENH-001's remaining work
> (BLA series-approximation) is a deferred perf optimization whose prerequisite — the
> ENH-006 profiler — is now shipped, so BLA can be revisited once per-pixel cost is
> measured as the bottleneck; it is not a correctness gap. Per-item verdicts with file:line evidence appear
> under each entry below, and the priority table has a Status column.

## Priority order

| ID | Title | Impact | Effort | Depends on | Status (2026-07-18) |
|----|-------|--------|--------|-----------|---------------------|
| ENH-002 | Progressive refinement + render-on-demand | High | Medium–High (~1 week) | ARC-006/008 fixes | **Done** — v1 Converged fast-path (`1c92fd2`) + v2 tile-progressive refinement (`615fb26`); plan's explicit settle-timer/LOD-bypass steps redundant with LOD's existing restore |
| ENH-003 | Dynamic resolution scaling (wire `render_scale`) | High | Medium (~2–3 days) | ARC-007/008 fixes | **Done** (commit `6c2079b`) — viewport + `scene_uv_scale` remap; no-op at scale 1.0 |
| ENH-001 | Perturbation-theory infinite zoom | Transformative | Very High (2–4 weeks) | ARC-001/002 bundle; harness ✓ (ENH-007 shipped) | **Phases A + B + C done** — all 4 kinds; 1e8 golden pinned & deterministic; decimal-string precise center (Phase C); BLA deferred (perf optimization, awaits ENH-006 profiler) |
| ENH-006 | GPU frame profiler (timestamp queries + HUD) | Medium (enables tuning) | Medium (~2 days) | none | **Done** — `GpuProfiler` (`src/renderer/profiler.rs`) + per-pass `timestamp_writes` + EMA HUD (`Shift+G`) + `--profile-dump`/`make profile` (commits `48a4ec0`→`afb8cbe`); degrades cleanly when `TIMESTAMP_QUERY` absent |
| ENH-005 | Half-resolution bloom pipeline | Medium | Low–Medium (~1 day) | ARC-005 (bloom gating) | **Done** — 3 bloom textures halved via `Renderer::bloom_size` (`src/renderer/{update,initialization}.rs`); blur self-corrects via `textureDimensions`; A/B pixel-identical (corr 1.0); capture path kept full-res |
| ENH-004 | Per-fractal pipeline specialization | Medium | Medium (~3 days) | ENH-006 (to measure) | **Partial** — 2D/3D entry-point split done; per-type specialization not |
| ENH-008 | `encase`-based uniform layout automation | Medium (kills the #1 crash class) | Medium (~2 days) | ARC-010 (offset tests as safety net) | **Done** — all 7 GPU uniform structs migrated to `#[derive(encase::ShaderType)]` (glam vec/mat types); ~14 `_padding_*` fields deleted from Rust + WGSL; `Uniforms` 896→768B; byte-pattern layout tests pin encase offsets; golden harness pixel-identical incl. 1e8 perturbation |

---

### ENH-001 — Perturbation-theory infinite zoom
**Plan**: `docs/fable/ENH-001-perturbation-deep-zoom.md`
**Status (2026-07-19, HEAD `8f78d25`):** Phase A + Phase B breadth landed. `dashu-float` is in `Cargo.toml`; `src/deep_zoom/{orbit,driver}.rs` compute the arbitrary-precision reference orbit off-thread and upload it to a GPU storage buffer; `mandelbrot_perturb` / `julia_perturb` / `tricorn_perturb` / `burning_ship_perturb` in `shaders/fractal.wgsl` iterate per-pixel f32 deltas; a `PERTURBATION_LOG2_GATE = 13.3` uniform engages it past zoom ~1e4 (lowered from 24 / ~1.6e7 to cover the full df-degraded band — commit `8f5b0bd`). The original >1e7 HP collapse is fixed and pinned by a deterministic 1e8 golden (`mandel-seahorse-1e8`, MAE 0.0 run-to-run). The recurrence math is CPU-verified against direct f64 in `tests/perturb_math.rs` (all four kinds). **Deferred — Phase B step 8 (BLA series-approximation tables):** measured the reference orbit at ~1.5 ms single / ~7.5 ms for the 9× probe at 1e8 (91 bits), scaling gently to ~1.9 ms at 1e30 — an earlier "~15–20 s" estimate was the pre-fix LOD-churn (orbit recomputed every frame), not the per-orbit cost, and is already resolved by the deterministic `perturbation_max_iterations` budget. GPU per-pixel cost is sub-frame at every testable zoom, so BLA's stated benefit (1e50+ "fast") remains unverifiable today (no 1e50 golden; BLA only pays off if per-pixel cost becomes the bottleneck). Re-open when the ENH-006 profiler measures per-pixel cost as the bottleneck — the >1e15 precise-center prerequisite landed in Phase C. **Phase B step 9 (progressive integration with ENH-002): delivered by ENH-002 v2** — tile refinement already handles perturbation views (`activate_perturbation` pins iterations to the orbit length, so only the pixel term applies — no LOD bypass, no orbit desync). **Phase C — complete:** (1) 10ⁿ zoom readout (commit `a05cbef`: the 2D panel "Zoom" label shows `≈ 1.23×10⁴⁵` at/above 1e4 with log₁₀/log₂/perturbation-active in the hover); (2) decimal-string precise center (commits `b577909` + `9a8745e` + `8f78d25`) — `parse_center_decimal` parses a decimal center straight to `FBig` (not bounded by f64), the perturbation worker uses `compute_reference_orbit_best_precise` when `Settings.center_2d_precise` is set, and the 2D panel exposes it via a "Go to" `re, im` field (+ optional `@ zoom`), a "Copy" button, and a 🔒 indicator. Pan / zoom-at-cursor clears the override; pure zoom-at-center keeps it; it serializes in `settings.yaml` / presets (`#[serde(default)]`, so old files load).

The current double-float pipeline hard-caps near zoom ~1e11 (AUDIT ARC-001). Perturbation theory
removes the cap: compute ONE reference orbit per view in arbitrary precision on the CPU, upload it
as a storage buffer, and iterate only per-pixel *deltas* on the GPU in plain f32 — `Δz ← 2·Z·Δz +
Δz² + Δc`. With bilinear approximation (BLA) to skip iterations and Pauldelbrot glitch detection
with re-basing, zooms of 1e100+ become routine (this is how Kalles Fraktaler / Fraktaler-3 work).
The repo already has the two hard prerequisites proven: storage-buffer plumbing (Buddhabrot
compute path) and correct DF math for the delta squaring. **Impact**: the headline feature —
actual infinite zoom. **Effort**: very high; phased plan delivers Mandelbrot-only MVP first.

### ENH-002 — Progressive refinement + render-on-demand
**Plan**: `docs/fable/ENH-002-progressive-refinement.md`
**Status (2026-07-18, HEAD `615fb26`):** **Done** — v1 Converged fast-path (`1c92fd2`) + v2
tile-progressive refinement (`615fb26`) both shipped.

**v1 — Converged fast-path (`1c92fd2`):** ARC-006 parks the loop when idle, but
`should_render_next_frame()` still triggered a render on every egui repaint, and `render()` ran the
fractal scene pass unconditionally — so hovering over a static deep-zoom view at high iterations
re-rendered the whole fractal every mouse-move frame. A `scene_converged` flag (set at the end of
`render()` once a frame completes clean + no continuous animation — always a full-quality frame,
since LOD restores to ultra when idle and LOD-off is always full quality; cleared by
`mark_scene_dirty`) lets converged UI-only repaints **skip the fractal pass** and re-composite the
cached `scene_texture` (`should_skip_scene_pass` in `src/app/mod.rs`; capture forces a fresh pass).
Pure decision logic unit-tested (`app::enh_002_tests`); the perf overlay gained a `Scene: Idle/Active`
row.

**v2 — Tile-progressive refinement (`615fb26`):** the remaining cost was the single full-quality
frame rendered when a deep-zoom view settles — at high iterations it froze the UI for tens to
hundreds of ms. v2 splits it: once a 2D view settles it renders tile-by-tile center-out at full
quality, one scissor-rect tile per frame with `LoadOp::Load` on `scene_texture` (finished tiles
persist), so detail pours in smoothly instead of one costly pop. New pure module
`src/app/refine.rs` holds the math (cost→grid, center-out order, tile-rect geometry — 10 unit
tests). `maybe_start_refinement` engages when settled (>150 ms since the last change), 2D,
non-accumulation, `scene_texture` initialized, and the extrapolated full-quality cost (last frame
ms ÷ render_scale²) exceeds one tile budget. At deep zoom the perturbation path pins iterations to
the orbit length (`activate_perturbation`), so only the pixel term applies and tiling needs no LOD
bypass — no orbit desync. `mark_scene_dirty` / capture abort in-flight refinement for a fresh full
pass; `Renderer.refining` forces `scene_render_scale = 1.0` during refinement. Diagnostic hook
`PAR_REFINE_FORCE_SIDE=<n>` (read once at startup) forces a grid to exercise the path. Verified:
golden harness 5/5 bit-identical; runtime smoke at zoom 1e8 with `PAR_REFINE_FORCE_SIDE=4` confirms
refinement engages across cycles with no wgpu validation errors and no crash.

**Not done (deferred as redundant):** the plan's v1 steps 2–4 (an explicit settle timer + LOD-bypass
"Refining" frame) are redundant with LOD's existing restore + ARC-006's animation tracking, and an
LOD bypass would risk desyncing ENH-001's perturbation `max_iter` lockstep. The settle contract they
described is delivered by v1+v2 without a parallel settle mechanism. This is the runtime foundation
ENH-001 Phase B step 9 (progressive integration) calls out.

After the dirty-flag fix (ARC-006) stops idle re-rendering, invert the remaining tradeoff: during
interaction render cheap (low iterations / low scale), and while idle *converge* — re-render at
full quality once, or tile-by-tile for extreme views, reusing the attractor accumulation
architecture (`renderer/compute.rs`) that already implements persist-accumulate-invalidate.
This makes arbitrarily expensive views (deep zoom + high iterations) feel interactive: you never
wait on a frame; detail pours in when you stop. **Impact**: high — transforms deep-zoom UX and
is the runtime foundation ENH-001 needs (perturbation frames are expensive). **Effort**: ~1 week.

### ENH-003 — Dynamic resolution scaling (wire `render_scale`)
**Plan**: `docs/fable/ENH-003-dynamic-render-scale.md`
**Status (2026-07-18, HEAD `6c2079b`):** **Done.** `QualityLevel.render_scale` is now
applied end-to-end (commit `6c2079b`, on top of phase-1 plumbing `8f2db86`). During LOD
motion `Renderer::update` resolves the active scale (`src/renderer/update.rs`) and
`App::render` sets a sub-rect `set_viewport` on the fractal pass (`src/app/render.rs`); the
post chain upsamples via `scene_uv_scale` in `BloomUniforms`/`PostProcessUniforms` and a
`scene_sample_uv` helper (`src/shaders/postprocess.wgsl`) that remaps the `scene_texture`
read in bloom-extract + composite with a half-texel clamp. **Design note:** the plan's step 2
(`render_size` uniform + coord-mapping change) was unnecessary — `fs_main_2d` derives coords
from NDC `uv` (viewport-independent), so the viewport scales the raster without cropping.
`scene_sample_uv` is a bit-for-bit no-op at `scene_uv_scale == 1.0` (idle / LOD-off / goldens
untouched); scale is forced to 1.0 for accumulation display and high-res capture
(`render_scale_override`). Verified: scale=1.0 LOD-ultra vs LOD-off default NCC 0.998 (no-op);
scale=0.5 shares framing with scale=1.0 (NCC 0.945, identical black-fraction — no crop/offset).
The slider is re-enabled (`src/ui/panels/lod.rs`).

`QualityLevel.render_scale` (0.25–1.0) exists, is user-visible, serialized, and interpolated — but
was never applied (AUDIT ARC-007). Because the scene already renders to an intermediate
`scene_texture` before a 6-pass post chain, rendering that texture at reduced size and letting the
existing composite/upsample stretch it is a contained change. LOD then gets its strongest lever:
quarter-resolution during motion ≈ 4–16× fragment-cost reduction, imperceptible while moving.
**Impact**: high — the single biggest interactive-performance win available. **Effort**: ~2–3 days.

### ENH-004 — Per-fractal pipeline specialization
**Plan**: `docs/fable/ENH-004-pipeline-specialization.md`
**Status (2026-07-18, HEAD `48f4427`):** Partial. Stage 1 (2D/3D entry-point split — `fs_main_2d`/`fs_main_3d`, `src/renderer/initialization.rs:277-343`) landed. Stage 2 (per-fractal WGSL `override` constants + lazy per-type pipeline cache) did not — there are no `override` declarations, no `PipelineCompilationOptions.constants`, and all 3D fractals still share one `pipeline_3d` (`src/renderer/initialization.rs:327`, stored once at line 967).

One 3,119-line ubershader compiles all 28+ fractals, DF variants, and the full 3D lighting stack
into a single fragment pipeline; the simple Mandelbrot path pays worst-case register/occupancy
cost (AUDIT ARC-009/QA-015). Split entry points (2D vs 3D minimum), specialize with WGSL
`override` constants or preprocessed variants, and cache pipelines per fractal type, compiling
lazily on first switch. **Impact**: medium (occupancy-bound GPUs benefit most; measure with
ENH-006 first). **Effort**: ~3 days.

### ENH-005 — Half-resolution bloom pipeline
**Plan**: `docs/fable/ENH-005-half-res-bloom.md`
**Status (2026-07-19, HEAD `5580784`):** **Done.** The three bloom intermediates
(`bright`/`blur_temp`/`bloom`) now render at half the surface resolution via a new
`Renderer::bloom_size(width, height)` helper (`src/renderer/update.rs`), used at both
the init site (`src/renderer/initialization.rs`) and the resize site
(`src/renderer/update.rs`); scene + composite stay full-res. The plan's "one real bug
surface" — stale texel-size uniforms — turned out to be a non-issue in the current code:
`fs_blur` derives its texel offset from `textureDimensions(t_scene)` of the bound
(now half-size) texture (`src/shaders/postprocess.wgsl:76-77`), composite upsamples via
`textureSample` (line 237), extract downsamples via `textureSample` (line 44), and the
single shared sampler is already `Linear`/`Linear` (`src/renderer/initialization.rs:405-406`)
— so plan steps 2–4 were automatic no-ops and no shader/uniform/sampler changes were needed.
**Scoped to the interactive renderer textures only**: the high-res capture path
(`src/app/capture.rs`, `src/app/capture_web.rs`) keeps full-res bloom, so screenshots and
goldens are byte-identical (capture = max-quality path). **Verification:** `make checkall`
green (fmt + clippy + 120 tests, incl. new `bloom_size_floors_and_halves` unit test locking
the `≥1` floor + odd-dim truncation); `make visual-test` identical with vs. without the
change (the 3 pre-existing FAILs are unrelated golden drift present on clean HEAD; the 2
deterministic deep-zoom goldens pass); A/B on an actively-blooming 3D preset (Mandelbox
Cubic, 1024×768, bloom ON — ENH-006 profiler confirms `bloom_extract`/`bloom_h`/`bloom_v`
execute at 0.04/0.06/0.08 ms) is pixel-identical (corr 1.0, MAE 0.0); odd window 1101×733
and the 1×1 worst case (bloom textures floored to 1×1) render with no wgpu validation
errors. Plan step 5 (threshold tweak) not needed — half-res bloom is bit-identical to
full-res, so there is no dimming to compensate.

After ARC-005 gates bloom off-by-default cost to zero, make bloom cheap when it IS on: extract and
blur at half (or quarter) resolution — the standard technique; blur is a low-pass filter, so
downsampling first is visually near-lossless and cuts bloom bandwidth ~4× (¾ of ~99 MB of 4K
Rgba16Float intermediates reclaimed). **Impact**: medium. **Effort**: ~1 day.

### ENH-006 — GPU frame profiler (timestamp queries + HUD)
**Plan**: `docs/fable/ENH-006-gpu-profiler-hud.md`
**Status (2026-07-19, HEAD `afb8cbe`):** **Done.** `GpuProfiler` (`src/renderer/profiler.rs`) owns a 32-capacity timestamp `QuerySet`, a 3-deep staging ring for 2-frame-latent readback (no `device.poll(Wait)` in the frame loop), and an EMA (α=0.1) per scope. `Features::TIMESTAMP_QUERY` is requested only when `adapter.features()` contains it (`src/renderer/initialization.rs`) — on wasm / unsupported drivers the profiler is a clean no-op and the HUD reports "timestamp queries unavailable". Every render/compute pass in `src/app/render.rs` and the compute dispatches in `src/renderer/compute.rs` carry pass-level `timestamp_writes` (the portable mechanism, per the plan's pitfall guidance): `compute_accum`, `scene`, `bloom_extract`, `bloom_h`, `bloom_v`, `composite`, `fxaa`, `egui`, plus `buddhabrot_copy` (Buddhabrot mode only — worst case 9 scopes × 2 = 18 queries, within the 32 cap). The HUD (`render_gpu_profile_overlay`, modeled on the LOD overlay) shows per-scope ms + a proportional bar + total GPU ms + CPU frame ms; toggle is `Shift+G` (plain-`G` floor toggle unchanged) or the Settings checkbox, and `show_gpu_profile` persists (`#[serde(default)]`). Agent dump: `--profile-dump <path>` writes the EMA map to YAML once at frame ≥ 120 (`src/app/update.rs`), exposed as `make profile` (the old Linux `perf` target is now `make profile-cpu`). Unit tests cover ring-slot rotation, EMA math, and `process_bytes` byte→scope pairing; `make checkall` green; `make profile` verified producing `target/profile.yaml` (scene pass dominates, bloom absent when off). This ships the per-pixel-cost measurement ENH-001's deferred BLA re-evaluation needs.

Every performance decision above is currently guesswork — the app has an FPS counter but no
per-pass timing. wgpu exposes `Features::TIMESTAMP_QUERY`; wrap each pass (fractal, bloom×3,
composite, FXAA, egui, compute) in timestamp pairs and show a per-pass ms breakdown in the debug
overlay (the LOD overlay pattern at `ui/overlays.rs:348` is the template), plus CSV dump via a CLI
flag for agent-driven regression checks. **Impact**: medium directly, high as an enabler — it
converts ENH-003/004/005 and LOD tuning from faith to measurement. **Effort**: ~2 days.

### ENH-008 — `encase`-based uniform layout automation
**Plan**: `docs/fable/ENH-008-encase-uniform-layout.md`
**Status (2026-07-19, HEAD `2a00e4f`):** **Done.** All seven GPU uniform struct
families migrated from `#[repr(C)] + bytemuck + manual padding` to
`#[derive(encase::ShaderType)]`: `Uniforms` (896→768B), `BloomUniforms`,
`BlurUniforms`, `PostProcessUniforms` (64→48B), `AccumulationDisplayUniforms`,
`AttractorComputeUniforms`, `BuddhabrotComputeUniforms`. vec/mat fields use
glam types (`Vec2`/`Vec3`/`Vec4`/`Mat4`) via glam's `encase` feature (dep
direction is glam→encase, not encase→glam — the plan got this backwards).
~14 `_padding_*` fields deleted from **both** the Rust structs and the WGSL
declarations; encase and WGSL now independently derive the same compact layout
from the identical field order. Uploads go through `write_uniform_bytes`
(encase `UniformBuffer::write`); buffer sizes use `ShaderType::min_size()`.
Deduped the three local `struct BlurUniforms` copies in the capture/render
paths (latent drift bug) to reuse the renderer's. **Verification:** per-struct
byte-pattern layout tests pin encase-computed offsets (the strongest form —
they assert the bytes the GPU receives); `make checkall` green (113 tests);
full golden harness pixel-identical across all 5 rows incl. the 1e8
perturbation deep-zoom golden (corr 1.0, MAE 0.0); 3D Mandelbulb + Buddhabrot
runtime smokes render real frames (1907 / 286 unique colors, no wgpu
validation errors). The add-a-field dry-run (plan step 6) confirmed the new
workflow: a new field compiles with zero padding math and the
`uniforms_byte_layout` test catches the offset/size shift with a pinpoint
message.

The 864-byte `Uniforms` struct is hand-mirrored against WGSL with ~14 manual padding fields —
the project's documented #1 source of silent GPU corruption (AUDIT ARC-010). The `encase` crate
derives WGSL-correct std140/storage layout automatically (`#[derive(ShaderType)]`), eliminating
padding fields and offset arithmetic entirely. Migrate after ARC-010's offset tests exist (they
become the migration's safety net, then stay as regression tests). **Impact**: medium — removes a
whole bug class and makes adding uniforms (which ENH-001/003 both need) safe. **Effort**: ~2 days.

---

## Explicitly not proposed

- **f64 WGSL shaders** — WebGPU/WGSL has no f64; native-only `SHADER_F64` exists in wgpu for a
  subset of backends, but it forks the shader per platform, is slow on consumer GPUs (1/32–1/64
  rate), and perturbation (ENH-001) is strictly better. Rejected on portability + perf.
- **Multi-GPU rendering** — complexity far exceeds benefit for this app's workload.
- **Dead-code deletion sweep** — covered by AUDIT QA-020; par-mem's 43-candidate list was ~90%
  trait-dispatch false positives, so bulk deletion would be harmful. Handled issue-by-issue there.
