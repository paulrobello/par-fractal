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

## Priority order

| ID | Title | Impact | Effort | Depends on |
|----|-------|--------|--------|-----------|
| ENH-007 | Deep-zoom visual regression harness | High (guards every other change) | Low–Medium (~1–2 days) | none — do first |
| ENH-002 | Progressive refinement + render-on-demand | High | Medium–High (~1 week) | ARC-006/008 fixes |
| ENH-003 | Dynamic resolution scaling (wire `render_scale`) | High | Medium (~2–3 days) | ARC-007/008 fixes |
| ENH-001 | Perturbation-theory infinite zoom | Transformative | Very High (2–4 weeks) | ARC-001/002 bundle, ENH-007 |
| ENH-006 | GPU frame profiler (timestamp queries + HUD) | Medium (enables tuning) | Medium (~2 days) | none |
| ENH-005 | Half-resolution bloom pipeline | Medium | Low–Medium (~1 day) | ARC-005 (bloom gating) |
| ENH-004 | Per-fractal pipeline specialization | Medium | Medium (~3 days) | ENH-006 (to measure) |
| ENH-008 | `encase`-based uniform layout automation | Medium (kills the #1 crash class) | Medium (~2 days) | ARC-010 (offset tests as safety net) |

---

### ENH-001 — Perturbation-theory infinite zoom
**Plan**: `docs/fable/ENH-001-perturbation-deep-zoom.md`

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

After the dirty-flag fix (ARC-006) stops idle re-rendering, invert the remaining tradeoff: during
interaction render cheap (low iterations / low scale), and while idle *converge* — re-render at
full quality once, or tile-by-tile for extreme views, reusing the attractor accumulation
architecture (`renderer/compute.rs`) that already implements persist-accumulate-invalidate.
This makes arbitrarily expensive views (deep zoom + high iterations) feel interactive: you never
wait on a frame; detail pours in when you stop. **Impact**: high — transforms deep-zoom UX and
is the runtime foundation ENH-001 needs (perturbation frames are expensive). **Effort**: ~1 week.

### ENH-003 — Dynamic resolution scaling (wire `render_scale`)
**Plan**: `docs/fable/ENH-003-dynamic-render-scale.md`

`QualityLevel.render_scale` (0.25–1.0) exists, is user-visible, serialized, and interpolated — but
was never applied (AUDIT ARC-007). Because the scene already renders to an intermediate
`scene_texture` before a 6-pass post chain, rendering that texture at reduced size and letting the
existing composite/upsample stretch it is a contained change. LOD then gets its strongest lever:
quarter-resolution during motion ≈ 4–16× fragment-cost reduction, imperceptible while moving.
**Impact**: high — the single biggest interactive-performance win available. **Effort**: ~2–3 days.

### ENH-004 — Per-fractal pipeline specialization
**Plan**: `docs/fable/ENH-004-pipeline-specialization.md`

One 3,119-line ubershader compiles all 28+ fractals, DF variants, and the full 3D lighting stack
into a single fragment pipeline; the simple Mandelbrot path pays worst-case register/occupancy
cost (AUDIT ARC-009/QA-015). Split entry points (2D vs 3D minimum), specialize with WGSL
`override` constants or preprocessed variants, and cache pipelines per fractal type, compiling
lazily on first switch. **Impact**: medium (occupancy-bound GPUs benefit most; measure with
ENH-006 first). **Effort**: ~3 days.

### ENH-005 — Half-resolution bloom pipeline
**Plan**: `docs/fable/ENH-005-half-res-bloom.md`

After ARC-005 gates bloom off-by-default cost to zero, make bloom cheap when it IS on: extract and
blur at half (or quarter) resolution — the standard technique; blur is a low-pass filter, so
downsampling first is visually near-lossless and cuts bloom bandwidth ~4× (¾ of ~99 MB of 4K
Rgba16Float intermediates reclaimed). **Impact**: medium. **Effort**: ~1 day.

### ENH-006 — GPU frame profiler (timestamp queries + HUD)
**Plan**: `docs/fable/ENH-006-gpu-profiler-hud.md`

Every performance decision above is currently guesswork — the app has an FPS counter but no
per-pass timing. wgpu exposes `Features::TIMESTAMP_QUERY`; wrap each pass (fractal, bloom×3,
composite, FXAA, egui, compute) in timestamp pairs and show a per-pass ms breakdown in the debug
overlay (the LOD overlay pattern at `ui/overlays.rs:348` is the template), plus CSV dump via a CLI
flag for agent-driven regression checks. **Impact**: medium directly, high as an enabler — it
converts ENH-003/004/005 and LOD tuning from faith to measurement. **Effort**: ~2 days.

### ENH-007 — Deep-zoom visual regression harness
**Plan**: `docs/fable/ENH-007-visual-regression-harness.md`

The audit found four deep-zoom correctness bugs that shipped silently (unreachable hp path, wrong
DF abs, late threshold, FMA dependence) — precisely the class a screenshot-vs-reference harness
catches. Build a CPU f64 reference renderer (tiny: escape-time only, ~100 lines), render small
tiles at fixed deep-zoom coordinates via the existing `--screenshot-delay`/`--exit-delay` flags,
and compare with perceptual + per-pixel tolerance in `cargo test` / CI. **Impact**: high — it
guards ENH-001 and the ARC-002 bundle, and every future shader edit. **Effort**: low–medium
(~1–2 days). **Do this one first.**

### ENH-008 — `encase`-based uniform layout automation
**Plan**: `docs/fable/ENH-008-encase-uniform-layout.md`

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
