# ENH-006 — GPU Frame Profiler (Timestamp Queries + HUD)

> **Impact**: Medium directly; High as an enabler — converts ENH-003/004/005 and LOD tuning from
> guesswork to measurement.
> **Effort**: Medium (~2 days).
> **Prerequisites**: none. Independent of all other work.

## Goal

Per-pass GPU timings (fractal scene, bloom×3, composite, FXAA, egui, compute accumulation)
displayed in a debug overlay and dumpable to CSV/JSON via a CLI flag, so agents and humans can
measure every optimization.

## Current state (verified at HEAD 8ee42cc)

- No GPU timing exists; only a CPU-side FPS counter (LOD tracks FPS in `src/lod.rs:409`).
- wgpu 29 exposes `Features::TIMESTAMP_QUERY` (+ `TIMESTAMP_QUERY_INSIDE_ENCODERS` /
  `..._INSIDE_PASSES` variants); Metal/Vulkan/DX12 all support the base feature on the target
  hardware. WebGPU (wasm): `timestamp-query` is an optional feature, available in Chrome behind
  flag/GPU — the profiler must degrade gracefully when absent.
- Device creation: `src/renderer/initialization.rs` (`new_with_gpu_preference` :44/:98 —
  `required_features` set near device request; read it).
- The pass sequence lives in `src/app/render.rs` (scene ~:300s, bloom :360-469, composite/FXAA
  ~:470-540, egui later) and compute dispatches at :77-140.
- Debug-overlay pattern to copy: `render_lod_debug_overlay` (`src/ui/overlays.rs:348`).
- Existing agent-operability flags in `src/main.rs` (clap) — add the dump flag alongside.

## Design

- `GpuProfiler` struct owning: `wgpu::QuerySet` (type Timestamp, capacity 2×MAX_SCOPES),
  a resolve buffer + N staging buffers (ring of 3 — readback lags frames; never stall),
  scope-name registry, and an EMA per scope.
- API: `profiler.scope(encoder, "bloom_h") -> ScopeToken` writing start/end timestamps via
  `encoder.write_timestamp` (feature `TIMESTAMP_QUERY_INSIDE_ENCODERS`) — simpler and more
  portable than inside-pass timestamps; brackets whole passes, which is the granularity we want.
  If `INSIDE_ENCODERS` is unavailable on a backend, fall back to
  `RenderPassDescriptor.timestamp_writes` (core wgpu, works per-pass without the encoder feature)
  — **prefer `timestamp_writes` as the primary mechanism** since it's the most portable: each
  pass descriptor gets `timestamp_writes: Some(RenderPassTimestampWrites { query_set,
  beginning_of_pass_write_index, end_of_pass_write_index })`; compute passes have the same field.
- End of frame: `encoder.resolve_query_set` → copy to the ring staging buffer → `map_async` two
  frames later → convert ticks via `queue.get_timestamp_period()` → ms per scope → EMA (α=0.1).

## Implementation steps

1. **New file `src/renderer/profiler.rs`**:
   ```rust
   pub struct GpuProfiler { query_set: Option<wgpu::QuerySet>, /* None = feature absent */
       resolve_buf: wgpu::Buffer, staging: [wgpu::Buffer; 3], frame_idx: usize,
       scopes: Vec<&'static str>, pub timings_ms: HashMap<&'static str, f32>, period: f32 }
   impl GpuProfiler {
       pub fn new(device: &Device, queue: &Queue, enabled: bool) -> Self;   // None-set if !enabled or feature missing
       pub fn pass_writes(&mut self, name: &'static str) -> Option<RenderPassTimestampWrites>; // allocates 2 indices
       pub fn compute_writes(&mut self, name: &'static str) -> Option<ComputePassTimestampWrites>;
       pub fn end_frame(&mut self, encoder: &mut CommandEncoder);           // resolve + copy to ring
       pub fn poll_results(&mut self, device: &Device);                     // map the (frame-2) staging buf, update EMA
   }
   ```
   Query budget: 2 per scope, ~10 scopes → 32-capacity set. Reset scope allocation each frame.
2. **Request the feature** (`initialization.rs`): add `Features::TIMESTAMP_QUERY` to
   `required_features` ONLY if `adapter.features()` contains it (use
   `adapter.features() & (TIMESTAMP_QUERY)` merged into the request — never hard-require; the
   profiler runs disabled otherwise). Same conditional for wasm.
3. **Instrument the passes** (`src/app/render.rs`): each `begin_render_pass` /
   `begin_compute_pass` descriptor gets `timestamp_writes: self.renderer.profiler.pass_writes("scene")`
   etc. Names: `compute_accum`, `scene`, `bloom_extract`, `bloom_h`, `bloom_v`, `composite`,
   `fxaa`, `egui`. After the last pass: `profiler.end_frame(&mut encoder)`; after submit:
   `profiler.poll_results(&device)`.
4. **HUD** (`src/ui/overlays.rs`): `render_gpu_profile_overlay` modeled on the LOD overlay (:348):
   a small table `scope | ms (EMA) | bar`, plus total GPU ms and the CPU frame ms for contrast.
   Toggle: add to the existing debug/overlay toggles (find how the LOD overlay toggles — a
   `UIState` bool + keybinding/panel checkbox; mirror it as `show_gpu_profile`).
5. **CLI dump for agents** (`src/main.rs` + `src/app/capture.rs` pattern): flag
   `--profile-dump <path>`: after N warmup frames (default 120), write JSON
   `{"scope": ms_ema, ...}` to the path and (combined with `--exit-delay`) exit. Implementation:
   store the flag in App; in `App::update`, when frame_count == 120 && flag set → serialize
   `profiler.timings_ms` with serde_json (already a dependency? check Cargo.toml — `serde_yaml`
   is present; use YAML if json isn't, format is unimportant) → write file.
6. **Makefile**: `make profile` target:
   `cargo run --release -- --profile-dump target/profile.yaml --exit-delay 6` then `cat` it.
7. **Persist nothing**: profiler state is runtime-only; no settings/schema changes (the HUD
   toggle may live in UIState if that's persisted — fine, it's a bool with serde default).

## Files to touch

| File | Change |
|------|--------|
| `src/renderer/profiler.rs` | new |
| `src/renderer/mod.rs` | own a `GpuProfiler`; expose to render path |
| `src/renderer/initialization.rs` | conditional feature request; profiler construction |
| `src/app/render.rs` | `timestamp_writes` on every pass; end_frame/poll calls |
| `src/ui/overlays.rs` | GPU profile overlay |
| `src/fractal/ui_state.rs` | `show_gpu_profile: bool` (serde default) |
| `src/main.rs` | `--profile-dump` flag |
| `src/app/update.rs` | dump-at-frame-N logic |
| `Makefile` | `profile` target |

## Verification

1. `make checkall`.
2. `make profile` → YAML/JSON with plausible numbers (scene pass dominates on a heavy fractal;
   bloom ≈0 when disabled post-ARC-005 — a nice self-check: toggle bloom, scopes appear).
3. HUD toggles on/off; numbers stable (EMA) and sum ≈ total GPU frame time.
4. Run on a GPU/driver WITHOUT the feature (or hack `enabled=false`): app runs normally, HUD says
   "timestamp queries unavailable", dump writes an empty map — no panic.
5. Ring-buffer correctness: resize the window rapidly while HUD is up — no validation errors
   (mapped-buffer hazards are the risk; the 3-deep ring plus mapping only the frame-2 buffer
   avoids them).
6. `make web-build` compiles (feature absent path on wasm).

## Rollback

Fully additive and behind `Option<QuerySet>`; disable by constructing with `enabled: false`
(one line) or revert the branch. No schema impact beyond one defaulted UI bool.

## Pitfalls

- NEVER `device.poll(Wait)` for query results in the frame loop — that stalls the pipeline the
  profiler is measuring. The 2-frame-latent ring is the whole design; results lag, which is fine.
- `queue.get_timestamp_period()` is in nanoseconds-per-tick and can vary per device — fetch once
  at init.
- Metal quirk: timestamps may be per-stage-boundary granular; pass-level bracketing (the chosen
  design) is the supported granularity everywhere — don't switch to inside-pass writes.
- The egui pass is recorded by egui-wgpu's renderer — bracketing it needs the timestamp_writes on
  the pass YOU create for egui (check how egui's render pass is begun in `app/render.rs:~600s`;
  it is begun in-app per the transmute list, so it works like the others).
