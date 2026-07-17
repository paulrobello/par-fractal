# ENH-002 — Progressive Refinement + Render-on-Demand

> **Impact**: High — deep-zoom interaction stays fluid at any cost level; idle frames converge to
> full quality instead of burning power re-rendering.
> **Effort**: Medium–High (~1 week).
> **Prerequisites**: AUDIT ARC-006 (dirty-flag redraw) and ARC-008 (LOD no longer mutates params)
> merged — this plan BUILDS ON the dirty flag; it does not re-implement it. ENH-003 (render scale)
> pairs naturally but is independent.

## Goal

Invert the frame-loop contract for 2D fractals:

- **While interacting** (zooming/panning): render cheap — reduced iterations and/or reduced
  resolution (via LOD/ENH-003). Never block the input loop on an expensive frame.
- **While idle**: refine — re-render once at full quality (v1), then tile-progressive for extreme
  views (v2), then present the converged image with zero further GPU work (dirty flag from ARC-006).

## Current state (verified at HEAD 8ee42cc, assumes ARC-006/008 merged)

- Post-ARC-006 there is a `scene_dirty` flag in `App` (`src/app/mod.rs`); when clean, no redraw.
- The scene renders to an intermediate `scene_texture` (`src/renderer/update.rs:7`
  `create_render_texture`, Rgba16Float) before the post chain — so "re-present without recompute"
  is already structurally possible.
- The in-repo template for accumulate/invalidate is the attractor path: `src/app/render.rs:77-108`
  (view-change detection via `attractor_last_center/zoom`, `attractor_pending_clear`) and
  `src/renderer/compute.rs` (persistent accumulation targets).
- LOD (post-ARC-007) has `iteration_scale` and 2D motion detection.
- `max_iterations` reaches the shader via the uniform (`src/renderer/uniforms.rs:299-301`).

## Design

Three-state scene lifecycle replacing the boolean dirty flag:

```rust
pub enum SceneState {
    Interactive,      // params changing; render cheap every frame
    Refining(u32),    // params stable; refinement pass N in flight
    Converged,        // full quality rendered; present cached texture only
}
```

- Input/param change → `Interactive` (and marks dirty).
- No param change for `settle_ms` (default 150 ms) → `Refining(0)`.
- Refinement passes complete → `Converged`.
- v1 refinement = ONE full-quality frame. v2 = tiled multi-pass for expensive views.

## Implementation steps

### v1 — settle-then-refine (2–3 days)

1. Add `scene_state: SceneState` + `last_param_change: Instant` to `App` (`src/app/mod.rs`).
   Replace ARC-006's boolean where appropriate: dirty ⇒ `Interactive`; clean ⇒
   `Converged` (mapping is mechanical; keep the redraw-request logic driven by
   `scene_state != Converged || ui_needs_repaint`).
2. Central param-change detection: after ARC-008, `FractalParams`'s user-facing fields change only
   via input/UI. Cheapest robust detector: hash the uniform-relevant fields once per frame
   (`#[derive(Hash)]` on a small `ViewKey` struct: fractal type, center bits, zoom bits,
   iterations, palette index, all shader params — build it in `app/update.rs`) and compare with
   the previous frame's hash. Changed → `Interactive`, stamp `last_param_change`.
   (The attractor path already does a hand-rolled version of this for 4 fields — reuse the idea,
   not the code.)
3. In `app/update.rs`: `Interactive` + `now - last_param_change > 150ms` → `Refining(0)`.
4. Quality selection at uniform build (`src/renderer/uniforms.rs`, the ARC-008 merge point):
   - `Interactive`: effective iterations = `user_iters * lod_iteration_scale` (LOD as merged);
     render scale per ENH-003 if present.
   - `Refining`/`Converged`: full user quality (LOD bypassed — the user is not moving).
5. In `app/render.rs`: when `Refining` completes its frame → `Converged`; `Converged` frames skip
   the scene pass and run only post+UI (the post chain reads the preserved `scene_texture` —
   verify the scene pass is the only writer to it; it is, per the pass chain in `app/render.rs`).
6. Palette animation / procedural phase: these change the IMAGE but not the fractal computation.
   Check where palette animates (uniform `time`-driven — `procedural_phase` in uniforms): if the
   palette applies in the FRACTAL pass (it does — coloring happens there), animation must keep
   the scene in `Interactive`-style rendering; gate: animation active ⇒ never `Converged`.
   (ENH-004's future split of compute vs colorize would fix this properly; out of scope here.)

### v2 — tiled progressive refinement (2–3 days, only after v1 ships)

7. Add a tile pass mode: uniform additions `tile_rect: vec4<f32>` (NDC scissor) — or simpler and
   better: use `render_pass.set_scissor_rect(x, y, w, h)` per pass, NO shader change, with
   `LoadOp::Load` on `scene_texture` so untouched tiles persist.
8. `Refining(n)` renders tile n of an N-tile grid (start 4×4, tiles ordered center-out) at full
   quality per frame; `n == N` → `Converged`. Present after each tile (progressive detail visibly
   pours in). During `Refining`, an incoming param change aborts to `Interactive` (tiles are
   naturally abandoned — `scene_texture` gets fully redrawn by the next interactive frame).
9. Tile count adaptive: estimate frame cost from ENH-006 timings (if present) or from the last
   full-frame duration: `N = clamp(last_frame_ms / 8ms, 1, 64)` rounded to a square grid.
10. First interactive frame after deep-zoom `Refining` abort can be expensive at high zoom even
    with LOD — that's ENH-003's render-scale job; no extra work here.

## Files to touch

| File | Change |
|------|--------|
| `src/app/mod.rs` | `SceneState`, timestamps, ViewKey hash state |
| `src/app/update.rs` | state transitions, settle timer, ViewKey computation |
| `src/app/render.rs` | state-driven pass selection; scissor tiles (v2); converged fast path |
| `src/renderer/uniforms.rs` | state-aware effective-quality selection (extends ARC-008 merge point) |
| `src/lod.rs` | no change expected (LOD supplies Interactive quality; bypassed otherwise) |
| `src/ui/overlays.rs` | show SceneState in the LOD/debug overlay (one line) |
| `src/main.rs` / `src/web_main.rs` | redraw-request condition includes `state != Converged` |
| `tests/` | state-machine transition tests (pure logic: settle timing, abort-on-change) |

## Verification

1. `make checkall`; new transition tests green (simulate: change → Interactive; wait → Refining →
   Converged; change mid-Refining → Interactive).
2. Runtime: zoom into Mandelbrot at 5000+ iterations — interaction stays >30 FPS (LOD-degraded);
   stop → visible snap/pour-in to full detail ≤1 s; idle GPU usage ~0% (Activity Monitor), matching
   ARC-006's guarantee.
3. Palette-animation preset: animation keeps running when "idle" (state never Converged while
   animating).
4. v2: at zoom 1e9 with 20k iterations, stopping shows tiles refining center-out; interrupting
   mid-refine snaps back to interactive with no half-rendered artifacts.
5. `make web-build` + browser smoke (rAF path must respect the state machine).
6. ENH-007 harness still green (harness runs settle → converged frames; screenshot delay ≥ settle
   time covers it — bump `--screenshot-delay` in the manifest if needed).

## Rollback

The state machine is additive around ARC-006's flag. Rollback v2 → v1: delete the scissor/tile
path (state machine unchanged). Rollback v1 → ARC-006 baseline: map `Interactive/Refining` → dirty,
`Converged` → clean; one small commit. No settings/schema changes at all.

## Pitfalls

- **Do not** drive refinement from wall-clock inside the render callback (frame pacing varies);
  drive from `update()` with `Instant` comparisons.
- Screenshot/video capture must always capture FULL quality: force `Refining→Converged` completion
  before honoring a capture request (check `capture_requested` handling order in `app/render.rs`),
  or capture only in `Converged` state — document which in code.
- The 3D path keeps its existing continuous behavior (camera moves smoothly; motion-LOD covers it).
  Scope this state machine to 2D mode + static 3D camera; do not regress 3D fly-through.
- egui overlays draw every presented frame regardless of scene state — UI repaint requests must
  not re-trigger the scene pass (they only re-run post+UI compositing).
