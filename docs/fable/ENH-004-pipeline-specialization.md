# ENH-004 — Per-Fractal Pipeline Specialization

> **Impact**: Medium — occupancy/register-pressure win for the cheap 2D path; faster shader
> iteration for developers. Measure with ENH-006 before/after; on some GPUs the win is small.
> **Effort**: Medium (~3 days).
> **Prerequisites**: ENH-006 (to measure honestly) recommended; AUDIT QA-016/QA-023/QA-030 shader
> cleanups ideally merged first (fewer conflicting hunks in `fractal.wgsl`).

## Goal

Stop compiling all 28+ fractals + DF variants + the full 3D lighting stack (AO, soft shadows, DoF,
reflections) into one fragment pipeline. Split into specialized pipelines selected per fractal
type, compiled lazily and cached, so the simple 2D path runs with a small register footprint and
shader edits recompile less.

## Current state (verified at HEAD 8ee42cc)

- `src/shaders/fractal.wgsl` (3,119 lines) has ONE fragment entry `fs_main` that branches on
  `uniforms.fractal_type` (2D dispatch ~:2936+, 3D DE dispatch `scene_de_with_material`
  :2128-2184 called per ray-march step and per normal/shadow/AO sample).
- One `wgpu::RenderPipeline` is built at startup (`src/renderer/initialization.rs:161-163, 208`)
  from the whole module; `naga` compiles everything for the active backend.
- Branching is on a uniform → no warp divergence, but register allocation is worst-case-path.
- Fractal switching goes through `FractalParams::switch_fractal` (`src/fractal/mod.rs:539`) and a
  UI/command path; nothing renderer-side reacts to type changes today (uniform-only).
- wgpu 29 supports WGSL `override` constants (pipeline-overridable constants) via
  `wgpu::PipelineCompilationOptions::constants`.

## Design decisions

1. **Stage 1 (this plan): split 2D vs 3D entry points** — two pipelines, biggest structural win,
   zero string preprocessing. The 2D entry contains only escape-time + palette code; the 3D entry
   only ray-marching + lighting.
2. **Stage 2 (this plan): specialize the 3D DE with an `override` constant** —
   `override FRACTAL_TYPE_CONST: u32;` used in `scene_de_with_material`'s dispatch. Backends
   constant-fold the switch and dead-strip untaken DE branches at pipeline-compile time. One
   pipeline per 3D fractal type, LAZILY compiled on first use and cached.
3. **NOT doing**: source-level preprocessing/codegen of per-fractal WGSL files. `override` gives
   the same dead-code elimination without a build system.

## Implementation steps

1. **Preparation — module hygiene** in `fractal.wgsl`: group functions into clearly-commented
   sections (`// === DF library ===`, `// === 2D fractals ===`, `// === 3D DEs ===`,
   `// === lighting ===`). No behavioral change; makes the split reviewable.
2. **Split entry points**:
   - `fs_main_2d`: vertex output → 2D dispatch (escape-time + hp/DF variants + attractor display
     path if it flows through fs_main today — check how attractor display renders:
     `attractor_display.wgsl` is separate, so no) → palette/color → output. Copy the shared
     prologue (coord mapping) as-is.
   - `fs_main_3d`: ray-march + lighting + fog/reflections → output.
   - Keep shared helpers (palette, smooth count, coord mapping) at module top — both entries
     reference them; naga dead-strips per entry point.
3. **Two pipelines at init** (`initialization.rs`): duplicate the pipeline descriptor with
   `entry_point: Some("fs_main_2d")` / `Some("fs_main_3d")`; same layout/formats. Store as
   `pipeline_2d`, `pipeline_3d` on `Renderer` (replacing the single `render_pipeline` field —
   grep its uses: `grep -rn 'render_pipeline' src/renderer src/app`).
4. **Selection per frame** (`src/app/render.rs`, scene pass): `let pipeline = if
   params.fractal_type.is_3d() { &self.renderer.pipeline_3d } else { &self.renderer.pipeline_2d };`
   — `is_3d()` exists or is trivial from the enum (check `src/fractal/types.rs`; 3D types carry
   the `3D` suffix; there may already be a `is_3d`/`dimension()` helper — search first).
5. **Stage 2 — 3D specialization**:
   - In WGSL: `override SPECIALIZED_TYPE: u32 = 0xFFFFu;` and in `scene_de_with_material`:
     `let ty = select(uniforms.fractal_type, SPECIALIZED_TYPE, SPECIALIZED_TYPE != 0xFFFFu);`
     then the existing dispatch on `ty`. (With a constant `ty`, backends fold the switch.)
   - `Renderer` gains `pipeline_3d_cache: HashMap<u32, wgpu::RenderPipeline>` + the generic
     `pipeline_3d` as fallback. On scene pass: look up `cache.get(&type_index)`; on miss, render
     THIS frame with the generic pipeline and spawn compilation of the specialized one
     (`device.create_render_pipeline` is synchronous in wgpu — a few ms; acceptable to do inline
     on the switch frame; if hitching is observed, defer to a background-thread device is NOT
     possible — instead compile on the frame after the switch, amortized; keep it simple: inline,
     measure with ENH-006).
   - Pass the constant: `PipelineCompilationOptions { constants: &[("SPECIALIZED_TYPE",
     type_index as f64)], .. }` on the fragment stage.
6. **Cache invalidation**: none needed (shader module is static per run). Pipelines are small;
   cache all 15 3D types worst-case.
7. **Measure** (ENH-006 or FPS overlay): record scene-pass ms for (a) Mandelbrot 2D before/after
   the split, (b) Mandelbulb before/after specialization, on the primary dev GPU (Apple Silicon
   Metal). Include numbers in the PR. If the win is <5% on both, stop after Stage 1 and note it —
   the maintainability win (2D/3D separation) stands alone; per-type caching may not pay.

## Files to touch

| File | Change |
|------|--------|
| `src/shaders/fractal.wgsl` | section grouping; `fs_main_2d`/`fs_main_3d`; `SPECIALIZED_TYPE` override |
| `src/renderer/initialization.rs` | two pipelines + compilation-options plumbing |
| `src/renderer/mod.rs` | pipeline fields + cache |
| `src/app/render.rs` | per-frame pipeline selection; cache lookup |
| `src/fractal/types.rs` | `is_3d()`/`shader_index()` helper (verify existing) |

## Verification

1. `make checkall`.
2. Screenshot sweep across ALL fractal types (script: iterate presets with `--preset` +
   `--screenshot-delay 3 --exit-delay 5`) — every type renders identically to pre-change goldens
   (ENH-007 harness rows + spot-checks; add one golden per 3D family if not covered).
3. Rapid fractal switching (hold F-keys / cycle via command palette) — no hitching beyond the
   first-switch compile, no wrong-fractal frames.
4. `make web-build` + browser: WebGPU supports override constants; verify on Chrome. If a browser
   lacks support, gate Stage 2 behind `cfg(not(target_arch = "wasm32"))` pipeline selection and
   keep web on the generic 3D pipeline.
5. Perf numbers recorded before/after (step 7).

## Rollback

Stage 2: stop consulting the cache (one line — always use generic `pipeline_3d`). Stage 1: point
both selectors at a rebuilt single-entry pipeline (revert commit). No data/settings impact.

## Pitfalls

- The uniform struct is SHARED by both entries — do not fork it; both entry points bind the same
  group layouts (pipeline layout identical) so bind groups need no changes.
- `override` constants in `PipelineCompilationOptions.constants` take `f64` values keyed by
  name — the WGSL declaration must have no `@id` or use matching ids; name-keyed is fine.
- Attractor/Buddhabrot display uses separate pipelines/shaders already — do not touch them.
- If `fs_main` currently contains 2D/3D-shared post logic (fog over 2D? unlikely — check the tail
  of `fs_main`), replicate carefully into both entries; diff the two entries against the original
  to prove nothing was dropped.
