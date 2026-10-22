# ENH-001 — Split `fractal.wgsl` into per-fractal modules

**Priority**: medium · **Estimate**: L · **Card**: filed as `[ENH-001]` with the `enhancement` tag

## Goal

Break the 3,521-line `src/shaders/fractal.wgsl` into per-domain WGSL modules assembled at load
time, so estimator code is reviewable, per-type iteration limits become possible, and the
Rust/WGSL contract tests (ARC-027) can validate per module.

## Current state

- `src/shaders/fractal.wgsl` holds: uniforms, ray-marching scaffolding, 12 distance estimators,
  6+ attractor functions, coloring, the dead post-processing path (removed by ARC-034), and both
  2D/3D fragment entry points — all in one file.
- WGSL has no `#include`; wgpu consumes one source string per module. The codebase already uses
  `include_str!` at every load site, and QA-039/ARC-037 introduce the `concat!(include_str!(…))`
  pattern for `common.wgsl` — this enhancement generalizes that mechanism.
- Estimated assembly order matters: uniforms and helpers must precede estimators, which must
  precede entry points.

## Implementation

1. **Prerequisite**: QA-031 (correct dispatch), ARC-034 (dead path removed), QA-039/ARC-037
   (`common.wgsl` mechanism exists). Land after those.
2. Create `src/shaders/fractal/` with modules, each a standalone-compilable fragment:
   - `uniforms.wgsl` — the `Uniforms` struct + small helpers
   - `raymarch.wgsl` — sphere tracing, DE scaling, lighting/AO scaffolding
   - `estimators_3d/mandelbulb.wgsl`, `menger.wgsl`, `mandelbox.wgsl`, … (one per 3D type or
     per family; keep ≤ ~200 lines each)
   - `estimators_2d/escape.wgsl`, `estimators_2d/attractors.wgsl`
   - `coloring.wgsl` — orbit traps, iteration coloring, palette mapping
   - `main_2d.wgsl`, `main_3d.wgsl` — the entry points and the dispatch switch
3. Add a tiny build-time assembler: a `const SHADER: &str = concat!(include_str!(…), …)` chain in
   a `src/shaders/fractal_sourced.rs` module (pure `concat!` keeps it zero-cost and lets rustdoc/
   clippy see the final string; no build script needed). Order: uniforms → common → raymarch →
   estimators → coloring → mains.
4. Replace every `include_str!("fractal.wgsl")` load site with the assembled constant.
5. Keep `fractal.wgsl` as a deprecation shim for one release? No — no external consumers; delete
   it in the same change and update CLAUDE.md/ARCHITECTURE paths.
6. Extend the QA-031 dispatch test to read the assembled constant instead of one file.
7. ARC-027's naga test validates the assembled string — unchanged API, now also covers each
   module in isolation if you parse them individually (optional, do it: it catches
   module-scoped symbol collisions early).

## Files touched

`src/shaders/fractal.wgsl` (deleted), `src/shaders/fractal/*` (new), `src/shaders/fractal_sourced.rs`
(new), all `include_str!` load sites (grep), `CLAUDE.md`, `docs/ARCHITECTURE.md` shader paths.

## Verify

- `make checkall` green; wasm + native compile.
- The dispatch/layout tests pass against the assembled source.
- Visual A/B: render one 2D and one 3D type before and after (`--screenshot-delay`) — pixel-identical
  or within golden-image tolerance.

## Rollback

Revert restores the single file; no data-format change, no uniform change. Keep the split in one
commit so revert is trivial.
