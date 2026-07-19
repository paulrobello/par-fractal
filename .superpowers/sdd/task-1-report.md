# Task 1 Report — GpuProfiler end-to-end

## Status: DONE

## What was implemented (per step)

### Step 1 — New file `src/renderer/profiler.rs`

A `GpuProfiler` struct owning:

- `query_set: Option<wgpu::QuerySet>` — `None` means no-op mode
- `resolve_buf: Option<wgpu::Buffer>` — single reuse destination for `resolve_query_set`
- `staging: [Option<wgpu::Buffer>; 3]` — 3-deep ring (results lag ~2 frames)
- `slot_mapped: [bool; 3]` — safety net tracking outstanding `map_async`
- `ring_scopes: [Vec<&'static str>; 3]` — per-slot saved scope-name lists
- `next_index: Cell<u32>` — per-frame allocation cursor (interior mutability)
- `frame_scopes: RefCell<Vec<&'static str>>` — current frame's scope names
- `frame_idx: Cell<usize>` — ring cursor
- `pending_map: Option<PendingMap>` — outstanding `map_async` state
- `pub timings_ms: HashMap<&'static str, f32>` — EMA per scope (the contract)
- `period: f32` — `queue.get_timestamp_period()` cached once at construction

Constants: `RING_SLOTS = 3`, `QUERY_CAPACITY = 32`, `MAX_SCOPES = 16`, `EMA_ALPHA = 0.1`.

### Step 2 — `src/renderer/initialization.rs`

- Added conditional request for `Features::TIMESTAMP_QUERY` only when
  `adapter.features()` already contains it (mirrors the existing CLEAR_TEXTURE
  pattern). Combined into one `required_features` set so we issue a single
  device request. Same code path serves wasm.
- Constructed `GpuProfiler::new(&device, &queue, timestamp_query_supported)`
  right before the `Self { ... }` literal.
- Added `pub profiler: GpuProfiler` field to `Renderer` in `src/renderer/mod.rs`.

### Step 3 — `src/app/render.rs`

Instrumented every render and compute pass with `timestamp_writes`:

| Pass (render.rs) | Scope name |
|---|---|
| Scene Render Pass (line 110) | `"scene"` |
| ENH-002 Refine Tile Pass (line 307) | `"scene"` (mutually exclusive with main scene) |
| Buddhabrot Copy Pass (line 477, compute) | `"buddhabrot_copy"` |
| Accumulation Display Pass (line 587) | `"scene"` (mutually exclusive with main scene) |
| Bloom Extract Pass (line 643) | `"bloom_extract"` |
| Blur Horizontal Pass (line 671) | `"bloom_h"` |
| Blur Vertical Pass (line 716) | `"bloom_v"` |
| Composite Pass (line 778) | `"composite"` |
| Final Pass (line 806) | `"fxaa"` |
| UI Render Pass / egui (line 978) | `"egui"` |

Plus the two compute dispatches inside `dispatch_accumulation` (attractor +
Buddhabrot main dispatches) use scope `"compute_accum"` via the new
`timestamp_writes` parameter on `AttractorComputePipeline::dispatch` and
`BuddhabrotComputePipeline::dispatch` in `src/renderer/compute.rs`.

`end_frame(&mut encoder)` is called once after `render_ui` returns, before
the final `queue.submit`. `poll_results(&device)` is called after submit.

## Final `GpuProfiler` public API

```rust
pub struct GpuProfiler {
    query_set: Option<wgpu::QuerySet>,
    resolve_buf: Option<wgpu::Buffer>,
    staging: [Option<wgpu::Buffer>; 3],
    slot_mapped: [bool; 3],
    ring_scopes: [Vec<&'static str>; 3],
    next_index: Cell<u32>,
    frame_scopes: RefCell<Vec<&'static str>>,
    frame_idx: Cell<usize>,
    pending_map: Option<PendingMap>,
    pub timings_ms: HashMap<&'static str, f32>,  // <-- Task 2/3 contract
    period: f32,
}

impl GpuProfiler {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, enabled: bool) -> Self;
    pub fn is_enabled(&self) -> bool;
    pub fn pass_writes(&self, name: &'static str) -> Option<wgpu::RenderPassTimestampWrites<'_>>;
    pub fn compute_writes(&self, name: &'static str) -> Option<wgpu::ComputePassTimestampWrites<'_>>;
    pub fn end_frame(&mut self, encoder: &mut wgpu::CommandEncoder);
    pub fn poll_results(&mut self, device: &wgpu::Device);
}
```

## Borrow-shape decision

**Single-call form, but with `&self` + interior mutability** (not `&mut self`).

The brief recommends returning the full `Option<RenderPassTimestampWrites>`
from one method. The wrinkle is that `render.rs` constructs the descriptor
inline, simultaneously borrowing other `Renderer` fields (`scene_view`,
`bright_view`, etc.). With `&mut self` on `pass_writes`, the borrow checker
rejects the inline form because `&mut renderer.profiler` and `&renderer.scene_view`
both reach through `&self.renderer`.

The fix is to make `pass_writes` take `&self` so all call-site borrows are
*shared*, which the borrow checker happily allows on disjoint fields. The two
mutable pieces of per-frame state (`next_index` cursor, `frame_scopes` name
list) move behind `Cell`/`RefCell`. This keeps the call site ergonomic
(`timestamp_writes: self.renderer.profiler.pass_writes("scene")` works
inline in the descriptor literal) and avoids the indices + getter fallback.

`end_frame` and `poll_results` keep `&mut self` — they're called outside any
descriptor construction and own the staging-ring lifecycle.

## Files changed

- `src/renderer/profiler.rs` (NEW, 487 lines incl. tests)
- `src/renderer/mod.rs` (+12 lines: module decl, import, struct field)
- `src/renderer/initialization.rs` (+23 lines: conditional feature request,
  profiler construction, struct field init)
- `src/renderer/compute.rs` (+4 lines: optional `timestamp_writes` param on
  both `dispatch` methods)
- `src/app/render.rs` (+24 lines: instrumentation on every pass,
  `end_frame` + `poll_results` calls)

## Testing

- `make checkall`: GREEN
  - tests pass (237 total across all test binaries; 4 pre-existing ignored)
  - clippy clean with `-D warnings`
  - fmt clean

- `cargo build --release`: succeeds.

- `cargo test --lib profiler`: 8 unit tests pass:
  - `test_ring_slot_rotation`: `frame_idx % RING_SLOTS` cycles 0,1,2 correctly
  - `test_ring_read_slot_after_increment`: `(frame_idx - 1) % RING_SLOTS` matches the slot just written
  - `test_ema_converges_to_steady_input`: 60 samples of 10.0 ms converge within 5%
  - `test_ema_first_sample_seeds`: first sample becomes the initial value (no spurious 0)
  - `test_ema_one_step_toward_new_sample`: 0.0 → 100.0 produces exactly 10.0 (α=0.1)
  - `test_query_budget_enforced`: 16 allocations fit, 17th rejected (capacity 32)
  - `test_disabled_profiler_state`: disabled profiler has all `None` fields, period=1.0
  - `test_ema_is_stable_under_steady_input`: stable input keeps EMA constant

- Runtime smoke (`par-fractal --exit-delay 5`): CLEAN
  - Exit code 0, no wgpu validation errors, no uncaptured errors, no profiler warnings.
  - Verified at both default and `RUST_LOG=error` / `RUST_LOG=warn` levels.
  - The only log line is a pre-existing egui_wgpu linear-framebuffer warning,
    unrelated to this change.

## Self-review against the brief's checklist

- ✅ **Every public method handles `query_set == None` without panicking** —
  `pass_writes` / `compute_writes` early-return `None` via `?`;
  `end_frame` checks the `query_set`/`resolve_buf`/`staging` triple and falls
  through to scope-reset logic without recording; `poll_results` early-returns
  when `query_set.is_none()`.
- ✅ **`timestamp_period` fetched exactly once** — only in `new()`, stored in
  `self.period`, never re-read.
- ✅ **Per-frame index allocation resets; no overflow past 32** — `end_frame`
  sets `next_index` back to 0 after resolve; `alloc_scope` refuses allocations
  where `start + 2 > QUERY_CAPACITY` and logs a warning instead of panicking.
- ✅ **No `device.poll(Wait)` in the frame loop** — none anywhere. `poll_results`
  takes `&Device` purely as part of the public contract; the implementation
  relies on `queue.submit` to fire ready `map_async` callbacks.
- ✅ **`pass_writes` borrow shape allows repeated per-frame calls** — `&self`
  with `Cell`/`RefCell` interior mutability. Multiple shared borrows coexist.
- ✅ **Egui pass begun in-app** — confirmed at `src/app/render.rs:978` (UI
  Render Pass inside `render_ui`), not inside egui-wgpu's renderer.
  `timestamp_writes` attached successfully.

## Concerns

- The map_async timing has a theoretical race: with the 3-slot ring, after
  issuing `map_async` on the just-written slot, the callback must fire before
  that slot's next overwrite (3 frames later at the latest). In practice the
  GPU work for the resolve+copy completes within 1–2 frames, so the callback
  fires on the next `queue.submit` and the slot is unmapped well before its
  overwrite. A `slot_mapped: [bool; 3]` safety net in `end_frame` skips the
  copy on the rare frame where the callback hasn't fired yet (drops one data
  point, never panics wgpu). No skip was observed during smoke testing.

- The profiler allocates one extra scope (`"buddhabrot_copy"`) beyond the
  brief's 8 named scopes, used only in Buddhabrot-accumulation mode. Max
  simultaneous queries: 9 scopes × 2 = 18, well within the 32-query capacity.

## Fix round 1

Applied the Task-1 reviewer's one Important + one Minor finding. No structural
deviation from the suggested shape; field/method names match the existing code.

### Fix 1 (Important) — `ring_scopes[slot]` overwrite no longer races the `slot_mapped` safety net

In `end_frame`, the unconditional `self.ring_scopes[slot] = self.frame_scopes.borrow_mut().drain(..).collect();`
ran even when the `!slot_mapped[slot]` guard had skipped the `resolve_query_set`
+ `copy_buffer_to_buffer`. When an outstanding `map_async` callback lagged past
~3 frames, the staging buffer still held the *old* frame's bytes while the name
list was overwritten with the *current* frame's scopes; the delayed callback
would then pair stale bytes with fresh names → silently mislabeled timings.

**Hunk applied** to `src/renderer/profiler.rs` (`end_frame`):

```diff
@@
         if used > 0 && !self.slot_mapped[slot] {
             if let (Some(query_set), Some(resolve_buf), Some(staging)) = (
                 self.query_set.as_ref(),
                 self.resolve_buf.as_ref(),
                 self.staging[slot].as_ref(),
             ) {
                 encoder.resolve_query_set(query_set, 0..used, resolve_buf, 0);
                 encoder.copy_buffer_to_buffer(/* … */);
             }
-        } else if used > 0 && self.slot_mapped[slot] {
-            log::debug!(
-                "GpuProfiler: skipping copy for slot {slot} (still mapped); no data this frame"
-            );
+            self.ring_scopes[slot] = self.frame_scopes.borrow_mut().drain(..).collect();
+        } else {
+            // Drop this frame's scope names without overwriting the ring
+            // slot's saved names (which still match the in-flight bytes).
+            self.frame_scopes.borrow_mut().clear();
+            if used > 0 && self.slot_mapped[slot] {
+                log::debug!(
+                    "GpuProfiler: skipping copy for slot {slot} (still mapped); no data this frame"
+                );
+            }
         }
-
-        // (3) Save this frame's scope names into the ring slot so a later
-        // `poll_results` can pair query results with names. Then reset the
-        // per-frame allocator.
-        self.ring_scopes[slot] = self.frame_scopes.borrow_mut().drain(..).collect();
+        // (3) Reset the per-frame allocator.
         self.next_index.set(0);
         self.frame_idx.set(self.frame_idx.get() + 1);
```

Net effect: `ring_scopes[slot]` is overwritten *only* in the branch that
actually performed the copy. The else branch `clear()`s `frame_scopes` so the
per-frame Vec doesn't grow across frames, and the existing `log::debug` for the
mapped-slot case is preserved verbatim. No renames, no other refactors.

### Fix 2 (Minor) — unit tests for `process_bytes`

`process_bytes` is a private free fn in the same module, so the existing
`#[cfg(test)] mod tests` (which already does `use super::*;`) reaches it
directly — no refactor needed. Added two tests next to the EMA/ring helpers:

1. `test_process_bytes_pairs_scopes_to_durations` — the requested 2-scope
   fixture (32 bytes of u64 timestamp pairs), `period = 1.0`. Assertions:

   - `timings["scene"] == 0.001` (1_000 ticks × 1.0 ns/tick / 1_000_000)
   - `timings["post"]  == 0.002` (2_000 ticks × 1.0 ns/tick / 1_000_000)
   - `timings.len() == 2` (no spurious entries)

2. `test_process_bytes_skips_zero_pair_and_respects_scope_count` — covers the
   zero-pair skip and the `scope_count` bound the reviewer called out. A 48-byte
   buffer holds 3 scope lanes; `scope_count = 2`. Lane 0 is all-zero (skipped),
   lane 1 is the only non-zero pair, lane 2 is non-zero but past the bound.
   Assertions:

   - `!timings.contains_key("scene")` (zero-pair skipped, EMA not seeded)
   - `timings["post"] == 0.002` (2_000 ticks = 500 → 2_500)
   - `!timings.contains_key("extra")` (3rd lane past `scope_count` not read)
   - `timings.len() == 1`

### Verification

**1. `cargo test --lib profiler`** — 10 passed, 0 failed (8 pre-existing + 2 new):

```
running 10 tests
test renderer::profiler::tests::test_query_budget_enforced ... ok
test renderer::profiler::tests::test_ring_read_slot_after_increment ... ok
test renderer::profiler::tests::test_ring_slot_rotation ... ok
test renderer::profiler::tests::test_disabled_profiler_state ... ok
test renderer::profiler::tests::test_ema_first_sample_seeds ... ok
test renderer::profiler::tests::test_process_bytes_pairs_scopes_to_durations ... ok
test renderer::profiler::tests::test_process_bytes_skips_zero_pair_and_respects_scope_count ... ok
test renderer::profiler::tests::test_ema_one_step_toward_new_sample ... ok
test renderer::profiler::tests::test_ema_is_stable_under_steady_input ... ok
test renderer::profiler::tests::test_ema_converges_to_steady_input ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 100 filtered out; finished in 0.00s
```

**2. `make checkall`** — green. Tail:

```
running 9 tests
test far_point_escapes_immediately ... ok
... (all 9 deep-zoom / decimal-float tests pass)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

Running Rust linters and auto-fixing issues...
cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.46s
cargo fmt

======================================================================
  Lint complete! (auto-fixed)
======================================================================

======================================================================
  All code quality checks passed!
======================================================================

Summary:
  ✓ Rust tests
  ✓ Rust format (auto-fixed)
  ✓ Rust lint (clippy auto-fixed)
```

**3. `cargo build --release`** — succeeded:

```
   Compiling par-fractal v0.9.0 (/Users/probello/Repos/par-fractal)
    Finished `release` profile [optimized] target(s) in 1m 17s
```

### Notes

- A first run of the zero-pair test failed on a fixture bug: I had initially
  placed scope-1's timestamp pair at byte offsets 24..40, but `process_bytes`
  lays each scope in a contiguous 16-byte lane (scope `i` at `[i*16 .. i*16+16]`),
  so scope 1 actually lives at bytes 16..31. Corrected the offsets; test passes.
  Recorded here so the miscalculation is visible, not hidden.
