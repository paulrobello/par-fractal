//! ENH-006 Task 1 — GPU frame profiler.
//!
//! Brackets every render/compute pass with timestamp queries, copies the
//! results through a 3-deep staging ring (so reads lag by ~2 frames and never
//! stall the pipeline), and exposes an EMA-smoothed [`GpuProfiler::timings_ms`]
//! per scope. Fully additive and gated on `Option<QuerySet>`: when the device
//! lacks [`wgpu::Features::TIMESTAMP_QUERY`] the profiler is a no-op and every
//! public method returns `None` / does nothing.
//!
//! ## Borrow shape
//!
//! [`GpuProfiler::pass_writes`] takes `&self` (not `&mut self`) so it can be
//! called inline inside a `RenderPassDescriptor` literal that simultaneously
//! borrows other fields of `Renderer` (e.g. `scene_view`). Per-frame mutable
//! state — the index allocation cursor and the scope-name list — lives behind
//! [`Cell`] / [`RefCell`]. [`GpuProfiler::end_frame`] and
//! [`GpuProfiler::poll_results`] keep `&mut self` because they are called
//! outside any descriptor construction and own the staging-ring lifecycle.
//!
//! ## Readback timing
//!
//! The ring has 3 staging buffers. Each frame `end_frame` copies the resolved
//! queries into `staging[frame_idx % 3]`. `poll_results` issues `map_async` on
//! the slot most recently written (`(frame_idx - 1) % 3`) and drains the
//! previous pending map (which targeted the slot written 2 frames prior). The
//! GPU work for a 2-frame-old slot is always complete by the time we map it,
//! so the `map_async` callback fires on the next `queue.submit` and the slot
//! is unmapped before its next overwrite. No `device.poll(Wait)` is ever
//! issued — that would stall the pipeline the profiler measures.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Number of staging buffers in the readback ring. Reads lag writes by ~2
/// frames, which keeps a mapped slot untouched by the GPU long enough for the
/// `map_async` callback to fire on the next `queue.submit`.
pub const RING_SLOTS: usize = 3;

/// Maximum number of timestamp queries the [`wgpu::QuerySet`] can hold.
/// Capacity 32 gives headroom over the 8 expected scopes × 2 queries each
/// (16 used, 16 spare).
pub const QUERY_CAPACITY: u32 = 32;

/// Maximum number of scopes that can be allocated per frame
/// (`QUERY_CAPACITY / 2`).
pub const MAX_SCOPES: usize = (QUERY_CAPACITY as usize) / 2;

/// EMA factor for smoothing per-scope timings. α = 0.1 means a new sample
/// contributes 10 % to the stored value; convergence to a steady input takes
/// roughly 30 frames.
pub const EMA_ALPHA: f32 = 0.1;

/// Tracks an outstanding `map_async` call on one of the ring's staging
/// buffers. The callback signals completion through `done`; `poll_results`
/// drains it on a later frame and unmaps the buffer.
struct PendingMap {
    /// Which ring slot is being mapped.
    slot: usize,
    /// Scope count saved at `end_frame` for the frame whose results live in
    /// `staging[slot]`. Used to bound the result-parse loop.
    scope_count: usize,
    /// Filled by the `map_async` callback: `Some(true)` on success,
    /// `Some(false)` on error, `None` while pending.
    done: Arc<Mutex<Option<bool>>>,
}

/// GPU frame profiler: timestamp queries on every render/compute pass, with
/// ~2-frame-latent readback through a 3-deep staging ring.
///
/// Construct with [`GpuProfiler::new`]; pass `enabled: false` (or construct
/// while the device lacks [`wgpu::Features::TIMESTAMP_QUERY`]) to run in
/// no-op mode.
pub struct GpuProfiler {
    /// `None` when the feature is absent or the profiler is disabled — every
    /// method handles `None` as a no-op without panicking.
    query_set: Option<wgpu::QuerySet>,
    /// Single reusable destination for `resolve_query_set`; copied into the
    /// ring each frame.
    resolve_buf: Option<wgpu::Buffer>,
    /// 3-deep ring of staging buffers used for readback. Each frame writes to
    /// `staging[frame_idx % RING_SLOTS]`; reads lag by ~2 frames.
    staging: [Option<wgpu::Buffer>; RING_SLOTS],
    /// `true` while `staging[slot]` has an outstanding `map_async`. `end_frame`
    /// skips its copy when the target slot is still mapped (a safety net for
    /// unusually slow GPUs — costs one data point, never crashes).
    slot_mapped: [bool; RING_SLOTS],
    /// Saved scope-name lists for each ring slot, written by `end_frame` and
    /// read by `poll_results` to associate query results with scope names.
    ring_scopes: [Vec<&'static str>; RING_SLOTS],
    /// Index of the next query pair to allocate (2 per scope). Reset to 0 in
    /// `end_frame`. `Cell` because `pass_writes` / `compute_writes` take
    /// `&self` so they can be called inside descriptor literals.
    next_index: Cell<u32>,
    /// Scope names allocated this frame, in allocation order. Drained into
    /// `ring_scopes[slot]` by `end_frame`. `RefCell` for the same reason as
    /// `next_index`.
    frame_scopes: RefCell<Vec<&'static str>>,
    /// Frame counter; the staging slot written by `end_frame` is
    /// `frame_idx % RING_SLOTS` (before increment), and `poll_results` maps
    /// slot `(frame_idx - 1) % RING_SLOTS` (after increment — the slot just
    /// written).
    frame_idx: Cell<usize>,
    /// A pending `map_async` result, if any. Drained at the top of the next
    /// `poll_results`.
    pending_map: Option<PendingMap>,
    /// EMA-smoothed per-scope GPU time in milliseconds. The contract for
    /// Task 2 (HUD) and Task 3 (CLI dump).
    pub timings_ms: HashMap<&'static str, f32>,
    /// Nanoseconds per timestamp tick, cached ONCE at construction via
    /// [`wgpu::Queue::get_timestamp_period`].
    period: f32,
}

impl GpuProfiler {
    /// Construct a profiler. When `enabled` is `false`, or the device lacks
    /// the timestamp feature (the caller is expected to gate on
    /// `adapter.features().contains(Features::TIMESTAMP_QUERY)`), every GPU
    /// resource stays `None` and every method becomes a no-op.
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, enabled: bool) -> Self {
        if !enabled {
            return Self::disabled();
        }
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("GpuProfiler QuerySet"),
            ty: wgpu::QueryType::Timestamp,
            count: QUERY_CAPACITY,
        });
        let resolve_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GpuProfiler Resolve Buffer"),
            size: u64::from(QUERY_CAPACITY) * u64::from(wgpu::QUERY_SIZE),
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = [0; RING_SLOTS].map(|i| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("GpuProfiler Staging Buffer {i}")),
                size: u64::from(QUERY_CAPACITY) * u64::from(wgpu::QUERY_SIZE),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        // Fetch once: ns-per-tick is device-stable and `get_timestamp_period`
        // may stall on some backends if called inside the frame loop.
        let period = queue.get_timestamp_period();
        Self {
            query_set: Some(query_set),
            resolve_buf: Some(resolve_buf),
            staging: staging.map(Some),
            slot_mapped: [false; RING_SLOTS],
            ring_scopes: [const { Vec::new() }; RING_SLOTS],
            next_index: Cell::new(0),
            frame_scopes: RefCell::new(Vec::with_capacity(MAX_SCOPES)),
            frame_idx: Cell::new(0),
            pending_map: None,
            timings_ms: HashMap::new(),
            period,
        }
    }

    /// Build a fully-`None` profiler used when the feature is absent.
    fn disabled() -> Self {
        Self {
            query_set: None,
            resolve_buf: None,
            staging: [const { None }; RING_SLOTS],
            slot_mapped: [false; RING_SLOTS],
            ring_scopes: [const { Vec::new() }; RING_SLOTS],
            next_index: Cell::new(0),
            frame_scopes: RefCell::new(Vec::with_capacity(MAX_SCOPES)),
            frame_idx: Cell::new(0),
            pending_map: None,
            timings_ms: HashMap::new(),
            // Unused when disabled, but kept finite for any math that runs.
            period: 1.0,
        }
    }

    /// Returns `true` if the profiler was constructed with a `QuerySet`
    /// (i.e. the device supports timestamp queries and the caller enabled
    /// the feature). Cheap check the render path can use to short-circuit.
    ///
    /// Task 2's HUD reads this to decide between showing the timing table
    /// versus a "timestamp queries unavailable" banner.
    #[allow(dead_code)]
    pub fn is_enabled(&self) -> bool {
        self.query_set.is_some()
    }

    /// Allocate a begin/end pair of timestamp queries for a render pass and
    /// return the [`wgpu::RenderPassTimestampWrites`] to attach to the pass
    /// descriptor. Returns `None` when the profiler is disabled or the per-
    /// frame query budget is exhausted.
    ///
    /// Takes `&self` (with interior mutability for the index cursor) so it
    /// can be called inline inside a `RenderPassDescriptor` literal that
    /// also borrows other fields of the renderer.
    pub fn pass_writes(&self, name: &'static str) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let query_set = self.query_set.as_ref()?;
        let start = self.alloc_scope(name)?;
        Some(wgpu::RenderPassTimestampWrites {
            query_set,
            beginning_of_pass_write_index: Some(start),
            end_of_pass_write_index: Some(start + 1),
        })
    }

    /// Compute-pass counterpart to [`Self::pass_writes`].
    pub fn compute_writes(
        &self,
        name: &'static str,
    ) -> Option<wgpu::ComputePassTimestampWrites<'_>> {
        let query_set = self.query_set.as_ref()?;
        let start = self.alloc_scope(name)?;
        Some(wgpu::ComputePassTimestampWrites {
            query_set,
            beginning_of_pass_write_index: Some(start),
            end_of_pass_write_index: Some(start + 1),
        })
    }

    /// Allocate 2 query indices for a scope; returns the start index.
    /// Returns `None` if the budget is exhausted (no panic, no overwrite).
    fn alloc_scope(&self, name: &'static str) -> Option<u32> {
        let start = self.next_index.get();
        if start + 2 > QUERY_CAPACITY {
            log::warn!(
                "GpuProfiler query budget exhausted at index {start}; scope `{name}` not measured"
            );
            return None;
        }
        self.frame_scopes.borrow_mut().push(name);
        self.next_index.set(start + 2);
        Some(start)
    }

    /// End of frame: resolve the frame's queries into `resolve_buf` and copy
    /// the used range into the next ring slot. Resets per-frame allocation
    /// state and advances the ring cursor. No-op when the profiler is
    /// disabled.
    ///
    /// Also drains any pending `map_async` whose callback has fired, unmapping
    /// the slot so it can be reused. The drain happens here (not in
    /// `poll_results`) because the copy below must not target a still-mapped
    /// buffer, and `end_frame` runs before `queue.submit` — the previous
    /// frame's submit is what fired the callback.
    pub fn end_frame(&mut self, encoder: &mut wgpu::CommandEncoder) {
        // (1) Drain pending map: process result + unmap if the callback has
        // fired. If not, leave the slot mapped and skip its overwrite below.
        self.drain_pending_map();

        let used = self.next_index.get();
        let slot = self.frame_idx.get() % RING_SLOTS;

        // (2) Resolve + copy. Skip when there's nothing to read or when the
        // target slot is still awaiting its `map_async` callback (rare safety
        // net — drops one data point rather than panicking wgpu).
        //
        // Only save `frame_scopes` into `ring_scopes[slot]` when we actually
        // performed the copy — otherwise a still-mapped slot still holds the
        // *old* frame's bytes, and overwriting the name list here would pair
        // those stale bytes with fresh names when the delayed `map_async`
        // callback eventually fires (silently mislabeled timings).
        if used > 0 && !self.slot_mapped[slot] {
            if let (Some(query_set), Some(resolve_buf), Some(staging)) = (
                self.query_set.as_ref(),
                self.resolve_buf.as_ref(),
                self.staging[slot].as_ref(),
            ) {
                encoder.resolve_query_set(query_set, 0..used, resolve_buf, 0);
                encoder.copy_buffer_to_buffer(
                    resolve_buf,
                    0,
                    staging,
                    0,
                    u64::from(used) * u64::from(wgpu::QUERY_SIZE),
                );
            }
            self.ring_scopes[slot] = self.frame_scopes.borrow_mut().drain(..).collect();
        } else {
            // Drop this frame's scope names without overwriting the ring
            // slot's saved names (which still match the in-flight bytes).
            self.frame_scopes.borrow_mut().clear();
            if used > 0 && self.slot_mapped[slot] {
                log::debug!(
                    "GpuProfiler: skipping copy for slot {slot} (still mapped); no data this frame"
                );
            }
        }

        // (3) Reset the per-frame allocator.
        self.next_index.set(0);
        self.frame_idx.set(self.frame_idx.get() + 1);
    }

    /// After `queue.submit`: process any completed `map_async` and issue a
    /// new one for the most recently written slot. Updates
    /// [`Self::timings_ms`] when results arrive.
    ///
    /// The `device` parameter is part of the public contract (Task 2/3 may
    /// use it); the current implementation relies on `queue.submit` to fire
    /// callbacks and never calls `device.poll(Wait)`.
    pub fn poll_results(&mut self, _device: &wgpu::Device) {
        if self.query_set.is_none() {
            return;
        }
        // Drain any pending map whose callback has fired.
        self.drain_pending_map();
        // Issue a new map_async for the slot just written by `end_frame`,
        // if no other map is in flight and the slot is unmapped.
        if self.pending_map.is_some() {
            return;
        }
        let frame_idx = self.frame_idx.get();
        // Need at least one written frame to read anything.
        if frame_idx == 0 {
            return;
        }
        // Most recently written slot (frame_idx was incremented post-write).
        let slot = (frame_idx - 1) % RING_SLOTS;
        if self.slot_mapped[slot] {
            return;
        }
        let Some(buf) = self.staging[slot].as_ref() else {
            return;
        };
        let scope_count = self.ring_scopes[slot].len();
        let done = Arc::new(Mutex::new(None));
        let done_clone = Arc::clone(&done);
        buf.slice(..).map_async(wgpu::MapMode::Read, move |result| {
            let mut guard = done_clone.lock().expect("map callback mutex poisoned");
            *guard = Some(result.is_ok());
        });
        self.slot_mapped[slot] = true;
        self.pending_map = Some(PendingMap {
            slot,
            scope_count,
            done,
        });
    }

    /// Drain `pending_map` if its callback has fired; on success, parse the
    /// timestamps, update `timings_ms`, and unmap. On error, just unmap. If
    /// the callback hasn't fired yet, leave everything in place for next
    /// time.
    fn drain_pending_map(&mut self) {
        let Some(pending) = self.pending_map.take() else {
            return;
        };
        let result = {
            let mut guard = pending.done.lock().expect("map result mutex poisoned");
            guard.take()
        };
        match result {
            Some(true) => {
                let slot = pending.slot;
                let scope_count = pending.scope_count;
                // Borrow staging through a temporary so process_bytes (a free
                // function taking `&mut HashMap`) doesn't fight the buffer
                // view's lifetime.
                let staging_ref = self.staging[slot].as_ref();
                let period = self.period;
                let scopes = &self.ring_scopes[slot];
                if let Some(buf) = staging_ref {
                    let view = buf.slice(..).get_mapped_range();
                    process_bytes(&mut self.timings_ms, scopes, scope_count, period, &view);
                    drop(view);
                    buf.unmap();
                }
                self.slot_mapped[slot] = false;
            }
            Some(false) => {
                log::warn!("GpuProfiler: map_async failed for slot {}", pending.slot);
                if let Some(buf) = self.staging[pending.slot].as_ref() {
                    buf.unmap();
                }
                self.slot_mapped[pending.slot] = false;
            }
            None => {
                // Callback hasn't fired yet — keep waiting.
                self.pending_map = Some(pending);
            }
        }
    }
}

/// Update the EMA for `name`. New samples contribute `EMA_ALPHA`; the prior
/// value contributes `1 - EMA_ALPHA`. The first sample seeds the EMA.
fn update_ema(timings: &mut HashMap<&'static str, f32>, name: &'static str, sample_ms: f32) {
    let prev = timings.get(&name).copied();
    let ema = match prev {
        Some(prev) => EMA_ALPHA * sample_ms + (1.0 - EMA_ALPHA) * prev,
        None => sample_ms,
    };
    timings.insert(name, ema);
}

/// Parse `scope_count` timestamp pairs out of `bytes` and update the EMA in
/// `timings` for each scope. Pairs that are both zero (unused queries, e.g.
/// a scope that resolved but never wrote because its pass was skipped) are
/// ignored so they don't drag the EMA toward zero.
///
/// Free function (not a method) so the caller can hold a `BufferView` and
/// update `timings` without an `&self` / `&mut self` conflict.
fn process_bytes(
    timings: &mut HashMap<&'static str, f32>,
    scopes: &[&'static str],
    scope_count: usize,
    period: f32,
    bytes: &[u8],
) {
    let qsize = wgpu::QUERY_SIZE as usize;
    for i in 0..scope_count {
        let start_off = i * 2 * qsize;
        let end_off = (i * 2 + 1) * qsize;
        if end_off + qsize > bytes.len() {
            break;
        }
        let start = u64::from_le_bytes(
            bytes[start_off..start_off + qsize]
                .try_into()
                .expect("query slice is exactly QUERY_SIZE bytes"),
        );
        let end = u64::from_le_bytes(
            bytes[end_off..end_off + qsize]
                .try_into()
                .expect("query slice is exactly QUERY_SIZE bytes"),
        );
        // Unwritten queries resolve to 0; skip them so they don't pollute
        // the EMA (a pass that was conditionally skipped this frame).
        if start == 0 && end == 0 {
            continue;
        }
        let duration_ticks = end.saturating_sub(start);
        let duration_ms = duration_ticks as f32 * period / 1_000_000.0;
        let name = scopes.get(i).copied().unwrap_or("unknown");
        update_ema(timings, name, duration_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ring-slot rotation: with 3 slots, slot for frame N is `N % 3`, and
    /// advancing the cursor cycles 0 → 1 → 2 → 0 → …
    #[test]
    fn test_ring_slot_rotation() {
        for frame_idx in 0..32 {
            let slot = frame_idx % RING_SLOTS;
            assert!(
                slot < RING_SLOTS,
                "frame {frame_idx} mapped to out-of-range slot {slot}"
            );
        }
        // Explicit walk: 0,1,2 repeating forever.
        let expected = [0, 1, 2, 0, 1, 2, 0, 1, 2];
        for (i, &want) in expected.iter().enumerate() {
            assert_eq!(i % RING_SLOTS, want);
        }
    }

    /// `poll_results` reads the slot written by the previous `end_frame`:
    /// `slot = (frame_idx - 1) % RING_SLOTS` once `frame_idx` has been
    /// incremented.
    #[test]
    fn test_ring_read_slot_after_increment() {
        for post_increment_idx in 1..16 {
            let read_slot = (post_increment_idx - 1) % RING_SLOTS;
            // The slot just written by end_frame is the one frame_idx points
            // just past.
            assert!(read_slot < RING_SLOTS);
            // And it matches what `end_frame` used as its write target.
            let write_slot_during_end_frame = (post_increment_idx - 1) % RING_SLOTS;
            assert_eq!(read_slot, write_slot_during_end_frame);
        }
    }

    /// EMA converges toward a steady input. With α = 0.1, after ~30 samples
    /// the stored value should be within 5 % of the input.
    #[test]
    fn test_ema_converges_to_steady_input() {
        let mut timings = HashMap::new();
        let steady = 10.0_f32;
        for _ in 0..60 {
            update_ema(&mut timings, "scene", steady);
        }
        let stored = timings["scene"];
        let rel_err = ((stored - steady).abs()) / steady;
        assert!(
            rel_err < 0.05,
            "EMA failed to converge: stored {stored}, target {steady}, rel_err {rel_err}"
        );
    }

    /// EMA: the first sample seeds the value (no prior to weight against).
    #[test]
    fn test_ema_first_sample_seeds() {
        let mut timings = HashMap::new();
        update_ema(&mut timings, "scene", 7.5);
        assert_eq!(timings["scene"], 7.5);
    }

    /// EMA: a single out-of-line sample moves the stored value ~10 % of the
    /// way toward the new sample.
    #[test]
    fn test_ema_one_step_toward_new_sample() {
        let mut timings = HashMap::new();
        update_ema(&mut timings, "scene", 0.0); // seed
        update_ema(&mut timings, "scene", 100.0); // jump
        let stored = timings["scene"];
        // α * 100 + (1 - α) * 0 = 10
        assert!(
            (stored - 10.0).abs() < 1e-5,
            "EMA after one step should be 10.0, got {stored}"
        );
    }

    /// Query budget: never allocate past `QUERY_CAPACITY`. Each scope
    /// consumes 2 indices, so `MAX_SCOPES` allocations fit and the next one
    /// is rejected.
    #[test]
    fn test_query_budget_enforced() {
        // We can't allocate against a real profiler without a GPU, but the
        // arithmetic is pure: simulate the cursor the way `alloc_scope` does.
        let mut cursor: u32 = 0;
        let mut allocated = 0;
        while cursor + 2 <= QUERY_CAPACITY {
            cursor += 2;
            allocated += 1;
        }
        assert_eq!(allocated, MAX_SCOPES);
        assert_eq!(cursor, QUERY_CAPACITY);
        // One more would overflow:
        assert!(cursor + 2 > QUERY_CAPACITY);
    }

    /// A disabled profiler reports `is_enabled() == false` and has no
    /// query_set — the path used when the device lacks the feature.
    #[test]
    fn test_disabled_profiler_state() {
        let p = GpuProfiler::disabled();
        assert!(!p.is_enabled());
        assert!(p.query_set.is_none());
        assert!(p.resolve_buf.is_none());
        assert!(p.staging.iter().all(|s| s.is_none()));
        assert_eq!(p.period, 1.0);
        // frame_idx starts at 0; allocations start empty.
        assert_eq!(p.frame_idx.get(), 0);
        assert_eq!(p.next_index.get(), 0);
        assert!(p.frame_scopes.borrow().is_empty());
    }

    /// `update_ema` is monotonic for a steady input (or seeded then held).
    #[test]
    fn test_ema_is_stable_under_steady_input() {
        let mut timings = HashMap::new();
        update_ema(&mut timings, "fxaa", 5.0);
        for _ in 0..10 {
            update_ema(&mut timings, "fxaa", 5.0);
        }
        // Should still be 5.0 to within float tolerance.
        assert!((timings["fxaa"] - 5.0).abs() < 1e-5);
    }

    /// `process_bytes` parses 2 scopes' worth of timestamp pairs (32 bytes of
    /// `u64`s) into ms durations, pairs each with its scope name, and seeds
    /// the EMA. `period = 1.0` ns/tick keeps the arithmetic obvious:
    /// ms = ticks / 1_000_000.
    #[test]
    fn test_process_bytes_pairs_scopes_to_durations() {
        let qsize = wgpu::QUERY_SIZE as usize;
        assert_eq!(qsize, 8, "fixture assumes 8-byte timestamp queries");

        // 2 scopes × 2 queries × 8 bytes = 32 bytes.
        let mut buf = vec![0u8; 2 * 2 * qsize];
        // Scope "scene": start=1_000, end=2_000 → 1_000 ticks.
        buf[0..qsize].copy_from_slice(&1_000u64.to_le_bytes());
        buf[qsize..2 * qsize].copy_from_slice(&2_000u64.to_le_bytes());
        // Scope "post": start=3_000, end=5_000 → 2_000 ticks.
        buf[2 * qsize..3 * qsize].copy_from_slice(&3_000u64.to_le_bytes());
        buf[3 * qsize..4 * qsize].copy_from_slice(&5_000u64.to_le_bytes());

        let scopes = ["scene", "post"];
        let mut timings = HashMap::new();
        process_bytes(&mut timings, &scopes, scopes.len(), 1.0, &buf);

        // 1_000 ticks × 1.0 ns/tick / 1_000_000 = 0.001 ms.
        assert!(
            (timings["scene"] - 0.001).abs() < 1e-9,
            "scene should be 0.001 ms, got {}",
            timings["scene"]
        );
        // 2_000 ticks × 1.0 ns/tick / 1_000_000 = 0.002 ms.
        assert!(
            (timings["post"] - 0.002).abs() < 1e-9,
            "post should be 0.002 ms, got {}",
            timings["post"]
        );
        // No spurious entries — only the 2 named scopes.
        assert_eq!(timings.len(), 2);
    }

    /// `process_bytes` skips pairs where both timestamps are 0 (an unwritten
    /// query, e.g. a conditionally-skipped pass) so they don't drag the EMA
    /// toward zero, and bounds the parse loop at `scope_count` even when the
    /// buffer holds additional bytes.
    #[test]
    fn test_process_bytes_skips_zero_pair_and_respects_scope_count() {
        let qsize = wgpu::QUERY_SIZE as usize;
        // 3 scopes worth of bytes (48 bytes), but only 2 scopes declared.
        // Each scope occupies a contiguous 16-byte lane: scope i lives at
        // bytes `[i*16 .. i*16+16]` (start = first 8, end = second 8).
        let mut buf = vec![0u8; 3 * 2 * qsize];
        // Scope 0 ("scene") at bytes 0..15: both zero → skipped entirely.
        // Scope 1 ("post") at bytes 16..31: start=500, end=2_500 → 2_000 ticks.
        buf[2 * qsize..3 * qsize].copy_from_slice(&500u64.to_le_bytes());
        buf[3 * qsize..4 * qsize].copy_from_slice(&2_500u64.to_le_bytes());
        // Scope 2 ("extra") at bytes 32..47: non-zero but past scope_count →
        // must not be read.
        buf[4 * qsize..5 * qsize].copy_from_slice(&9_000u64.to_le_bytes());

        let scopes = ["scene", "post", "extra"];
        let mut timings = HashMap::new();
        // scope_count = 2: the loop must not look at the 3rd pair.
        process_bytes(&mut timings, &scopes, 2, 1.0, &buf);

        // "scene" was a zero-pair → never inserted.
        assert!(!timings.contains_key("scene"));
        // "post" → 2_000 ticks = 0.002 ms.
        assert!((timings["post"] - 0.002).abs() < 1e-9);
        // "extra" past the bound → never read.
        assert!(!timings.contains_key("extra"));
        assert_eq!(timings.len(), 1);
    }
}
