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

> **Status — verified 2026-07-17 at HEAD `8cdd915` (v0.9.0):** The v0.9.0 audit
> remediation shipped several ENH *prerequisites* (ARC-002 HP math, ARC-005 bloom gating,
> ARC-006 render-on-demand, ARC-010 offset tests) but no complete enhancement. Net result:
> **1 of 8 fully done (ENH-007); 2 partial (ENH-002, ENH-004); 5 not started.** Per-item
> verdicts with file:line evidence appear under each entry below, and the priority table
> has a Status column.

## Priority order

| ID | Title | Impact | Effort | Depends on | Status (v0.9.0) |
|----|-------|--------|--------|-----------|-----------------|
| ENH-007 | Deep-zoom visual regression harness | High (guards every other change) | Low–Medium (~1–2 days) | none — do first | **Done** (CPU teeth + GPU goldens; root-caused the >1e7 HP precision collapse → ENH-001) |
| ENH-002 | Progressive refinement + render-on-demand | High | Medium–High (~1 week) | ARC-006/008 fixes | **Partial** — render-on-demand done (ARC-006) |
| ENH-003 | Dynamic resolution scaling (wire `render_scale`) | High | Medium (~2–3 days) | ARC-007/008 fixes | Not started (render_scale still inert) |
| ENH-001 | Perturbation-theory infinite zoom | Transformative | Very High (2–4 weeks) | ARC-001/002 bundle, ENH-007 | Not started (HP prereq landed via ARC-002) |
| ENH-006 | GPU frame profiler (timestamp queries + HUD) | Medium (enables tuning) | Medium (~2 days) | none | Not started |
| ENH-005 | Half-resolution bloom pipeline | Medium | Low–Medium (~1 day) | ARC-005 (bloom gating) | Not started (bloom gating done via ARC-005) |
| ENH-004 | Per-fractal pipeline specialization | Medium | Medium (~3 days) | ENH-006 (to measure) | **Partial** — 2D/3D split done |
| ENH-008 | `encase`-based uniform layout automation | Medium (kills the #1 crash class) | Medium (~2 days) | ARC-010 (offset tests as safety net) | Not started (offset tests landed via ARC-010) |

---

### ENH-001 — Perturbation-theory infinite zoom
**Plan**: `docs/fable/ENH-001-perturbation-deep-zoom.md`
**Status (2026-07-17, v0.9.0):** Not started. Only the ARC-002 high-precision/DF prerequisite landed. No arbitrary-precision crate in `Cargo.toml`, no `deep_zoom/` module, no reference-orbit storage buffer, and no per-pixel delta-iteration shader path exist.

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
**Status (2026-07-17, v0.9.0):** Partial. The render-on-demand half landed via ARC-006 (`scene_dirty` flag + `ControlFlow::Wait`, `src/app/mod.rs:87`). The progressive-refinement half (idle quality ramp / tile convergence) is not — `src/app/mod.rs:86` explicitly defers it to ENH-002, and the scene pass is a single full-frame render (`src/app/render.rs:82-114`).

After the dirty-flag fix (ARC-006) stops idle re-rendering, invert the remaining tradeoff: during
interaction render cheap (low iterations / low scale), and while idle *converge* — re-render at
full quality once, or tile-by-tile for extreme views, reusing the attractor accumulation
architecture (`renderer/compute.rs`) that already implements persist-accumulate-invalidate.
This makes arbitrarily expensive views (deep zoom + high iterations) feel interactive: you never
wait on a frame; detail pours in when you stop. **Impact**: high — transforms deep-zoom UX and
is the runtime foundation ENH-001 needs (perturbation frames are expensive). **Effort**: ~1 week.

### ENH-003 — Dynamic resolution scaling (wire `render_scale`)
**Plan**: `docs/fable/ENH-003-dynamic-render-scale.md`
**Status (2026-07-17, v0.9.0):** Not started. `render_scale` is defined, serialized, and interpolated (`src/lod.rs:79`) but never applied — the scene texture is still created at full window resolution (`src/renderer/initialization.rs:358-367`). The slider label itself reads "not yet applied — see ENH-003" (`src/ui/panels/lod.rs:384`).

`QualityLevel.render_scale` (0.25–1.0) exists, is user-visible, serialized, and interpolated — but
was never applied (AUDIT ARC-007). Because the scene already renders to an intermediate
`scene_texture` before a 6-pass post chain, rendering that texture at reduced size and letting the
existing composite/upsample stretch it is a contained change. LOD then gets its strongest lever:
quarter-resolution during motion ≈ 4–16× fragment-cost reduction, imperceptible while moving.
**Impact**: high — the single biggest interactive-performance win available. **Effort**: ~2–3 days.

### ENH-004 — Per-fractal pipeline specialization
**Plan**: `docs/fable/ENH-004-pipeline-specialization.md`
**Status (2026-07-17, v0.9.0):** Partial. Stage 1 (2D/3D entry-point split — `fs_main_2d`/`fs_main_3d`, `src/renderer/initialization.rs:242-324`) landed. Stage 2 (per-fractal WGSL `override` constants + lazy per-type pipeline cache) did not — there are no `override` declarations, no `PipelineCompilationOptions.constants`, and all 3D fractals still share one `pipeline_3d`.

One 3,119-line ubershader compiles all 28+ fractals, DF variants, and the full 3D lighting stack
into a single fragment pipeline; the simple Mandelbrot path pays worst-case register/occupancy
cost (AUDIT ARC-009/QA-015). Split entry points (2D vs 3D minimum), specialize with WGSL
`override` constants or preprocessed variants, and cache pipelines per fractal type, compiling
lazily on first switch. **Impact**: medium (occupancy-bound GPUs benefit most; measure with
ENH-006 first). **Effort**: ~3 days.

### ENH-005 — Half-resolution bloom pipeline
**Plan**: `docs/fable/ENH-005-half-res-bloom.md`
**Status (2026-07-17, v0.9.0):** Not started. Bloom is gated off by default (ARC-005), but the extract and both blur passes still run at full scene resolution (`src/renderer/initialization.rs:360-365`, `src/app/render.rs:489-594`); no half/quarter-res downsample of the bloom intermediates.

After ARC-005 gates bloom off-by-default cost to zero, make bloom cheap when it IS on: extract and
blur at half (or quarter) resolution — the standard technique; blur is a low-pass filter, so
downsampling first is visually near-lossless and cuts bloom bandwidth ~4× (¾ of ~99 MB of 4K
Rgba16Float intermediates reclaimed). **Impact**: medium. **Effort**: ~1 day.

### ENH-006 — GPU frame profiler (timestamp queries + HUD)
**Plan**: `docs/fable/ENH-006-gpu-profiler-hud.md`
**Status (2026-07-17, v0.9.0):** Not started. No `TIMESTAMP_QUERY` feature, no query set, no `write_timestamp`, no profiler HUD (the overlay at `src/ui/overlays.rs:131` is FPS-only), and no CSV-dump CLI flag in `src/main.rs`.

Every performance decision above is currently guesswork — the app has an FPS counter but no
per-pass timing. wgpu exposes `Features::TIMESTAMP_QUERY`; wrap each pass (fractal, bloom×3,
composite, FXAA, egui, compute) in timestamp pairs and show a per-pass ms breakdown in the debug
overlay (the LOD overlay pattern at `ui/overlays.rs:348` is the template), plus CSV dump via a CLI
flag for agent-driven regression checks. **Impact**: medium directly, high as an enabler — it
converts ENH-003/004/005 and LOD tuning from faith to measurement. **Effort**: ~2 days.

### ENH-007 — Deep-zoom visual regression harness
**Plan**: `docs/fable/ENH-007-visual-regression-harness.md`
**Status (2026-07-17, v0.9.0):** **Done.** Two-layer harness shipped:
- **CPU teeth (CI-safe, `tests/reference_math.rs` + `src/reference.rs`):** an f64 escape-time
  reference renderer + a byte-for-byte Rust mirror of the shader's double-float primitives
  (`two_prod`/`two_sum`/`df_mul`/`df2_square`/abs). Tests pin the EFT property of `two_prod`/
  `two_sum` (catches FMA-collapse deterministically), known points, a blessed smooth-value
  drift table, and DF-vs-f64 agreement on 32×32 deep-zoom tiles. Mutation-verified: collapsing
  `df_mul` to plain f32 fails the Mandelbrot deep-zoom test at ~200× the threshold.
- **GPU golden layer (local, `scripts/visual_test.sh` / `make visual-test`):** `imgdiff` bin
  compares the real binary's screenshots against committed `tests/golden/*.png` tiles; new CLI
  flags `--screenshot-path` / `--window-size` give deterministic captures; `gen-preset` builds
  row presets via the app's own serializer. Skips cleanly on headless boxes.

**Finding surfaced by the harness (root-caused 2026-07-17; fix is ENH-001):** the GPU's
double-float HP path is *correct* through ~1e7 — it renders proper seahorse structure given
adequate settle time (the earlier "solid black above 1e5" was a too-short settle / screenshot-
vs-first-frame race, not a render failure). Above ~3e7 it collapses: the frame is fast (~1.2 ms)
and the device is NOT lost (verified via a device-lost callback + uncaptured-error handler),
but per-pixel coordinate precision collapses so the whole frame computes one shared orbit — a
near-uniform image (3 distinct gray values at 1e8) instead of the seahorse. naga's Metal output
preserves the `two_prod` error-free transform (no FMA fusion), so the collapse is downstream
(Metal flush-to-zero / sub-ULP lo-word loss over long near-boundary orbits); the CPU DF mirror
doesn't collapse, which is why the DF-vs-f64 teeth pass and never caught it. This is a
fundamental limit of multi-thousand-iteration double-float on Metal — exactly what perturbation
(ENH-001) eliminates by iterating plain-f32 deltas against a CPU reference orbit. Manifest
goldens stay at ≤1e5; deep-zoom *math* correctness stays guarded by the CPU teeth.

The audit found four deep-zoom correctness bugs that shipped silently (unreachable hp path, wrong
DF abs, late threshold, FMA dependence) — precisely the class a screenshot-vs-reference harness
catches. Build a CPU f64 reference renderer (tiny: escape-time only, ~100 lines), render small
tiles at fixed deep-zoom coordinates via the existing `--screenshot-delay`/`--exit-delay` flags,
and compare with perceptual + per-pixel tolerance in `cargo test` / CI. **Impact**: high — it
guards ENH-001 and the ARC-002 bundle, and every future shader edit. **Effort**: low–medium
(~1–2 days). **Do this one first.**

### ENH-008 — `encase`-based uniform layout automation
**Plan**: `docs/fable/ENH-008-encase-uniform-layout.md`
**Status (2026-07-17, v0.9.0):** Not started. `encase` is not in `Cargo.toml`; `Uniforms` still uses `#[repr(C)]` + manual `_padding_*` fields + bytemuck (`src/renderer/uniforms.rs`). Its intended safety net, the ARC-010 `offset_of!` layout tests, did land.

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
