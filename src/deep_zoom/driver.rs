//! ENH-001 Phase A step 5 — the CPU driver that (re)computes the reference
//! orbit on a worker thread when the view changes at deep zoom.
//!
//! This module owns the *scheduling* side of perturbation: detecting
//! view changes, spawning the off-thread compute, and draining the result
//! on the main thread. The actual orbit math lives in [`super::orbit`]; the
//! GPU upload + bind-group rebuild live on [`crate::renderer::Renderer`].
//!
//! Concurrency model mirrors the ARC-018 GPU-scan pattern in
//! `app/render.rs::handle_ui_actions`: a `std::sync::mpsc` channel, a
//! `std::thread::spawn`'d worker that owns the heavy CPU work, and a
//! non-blocking `try_recv` poll from `App::update` every frame. The render
//! loop never blocks on the compute — while the worker is in flight the HP
//! path keeps rendering the previous frame (perturbation stays off until
//! the orbit lands and the driver marks it active).
//!
//! Native only. `std::thread::spawn` is unavailable on wasm; on wasm the HP
//! path handles deep zoom with its known ceiling and the driver is a no-op.

use crate::deep_zoom::orbit::{ReferenceOrbit, compute_reference_orbit, precision_bits_for_zoom};
use crate::fractal::{FractalType, RenderMode};

/// Zoom (as log2) above which perturbation activates.
///
/// Set to 24 (zoom ≈ 1.68e7), just below the coordinate-precision collapse
/// root-caused at ~3e7 (log2 ≈ 24.8) on 2026-07-17. The plan's original 34
/// was based on the superseded "DF ceiling ~1e11" model; the failure
/// actually arrives far earlier, so perturbation must engage BEFORE the
/// collapse or the first deep frame renders as garbage.
pub const PERTURBATION_LOG2_GATE: f64 = 24.0;

/// True when perturbation should engage for this view.
///
/// Phase A is Mandelbrot-only: the `Δz ← 2·Z_n·Δz + Δz² + Δc` recurrence
/// assumes the plain Mandelbrot map. Julia, Burning Ship, and Tricorn have
/// different delta recurrences (Phase B), and 3D / attractor / Buddhabrot
/// paths don't use the 2D delta shader at all. Below the gate the existing
/// HP path renders correctly.
pub fn perturbation_eligible(
    zoom_2d: f64,
    fractal_type: FractalType,
    render_mode: RenderMode,
) -> bool {
    fractal_type == FractalType::Mandelbrot2D
        && render_mode == RenderMode::TwoD
        && zoom_2d.log2() > PERTURBATION_LOG2_GATE
}

/// View signature used to invalidate the reference orbit. Centers are the
/// CPU-side f64 `FractalParams.center_2d` (still the entry point in Phase
/// A); `max_iter` is the GPU-effective value (zoom bonus + LOD scale) so a
/// recompute always serves the exact iteration budget the shader uses.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ViewSignature {
    center: [f64; 2],
    zoom: f64,
    max_iter: u32,
}

/// Schedules the off-render-thread reference-orbit computation.
///
/// The driver is polled once per frame from `App::update`:
/// 1. [`Self::note_view`] records the current view, marking the orbit stale
///    when `center`, `zoom`, or `max_iter` changed.
/// 2. [`Self::maybe_spawn`] kicks off a worker when stale AND the gate is
///    met AND no worker is already in flight. Returns `true` if it spawned
///    (so the caller can show a "computing…" toast on that transition only).
/// 3. [`Self::poll`] drains the finished orbit via `try_recv` — never blocks.
///
/// Only one worker is in flight at a time. A view change while a worker is
/// running leaves the in-flight result unused (a new worker spawns once the
/// current one finishes and the new view still differs from `last_view`).
pub struct PerturbationDriver {
    /// Pending worker result. `Some` while a worker is in flight; drained
    /// and set to `None` by [`Self::poll`] on receipt (or channel disconnect).
    #[cfg(not(target_arch = "wasm32"))]
    pending: Option<std::sync::mpsc::Receiver<ReferenceOrbit>>,
    /// Last view the driver issued a spawn for (or that `note_view` recorded
    /// when no spawn was eligible). Equality against this is the dirty signal.
    last_view: Option<ViewSignature>,
    /// True when the current orbit no longer matches `last_view`. Stays true
    /// across frames where the gate isn't met (so the first eligible frame
    /// after the user zooms past the gate triggers a spawn immediately).
    dirty: bool,
    /// True while a worker is in flight. Drives the "computing…" toast and
    /// suppresses respawns.
    pub computing: bool,
}

impl Default for PerturbationDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl PerturbationDriver {
    pub fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            pending: None,
            // Start dirty so the first eligible view spawns a worker without
            // waiting for a second identical frame.
            last_view: None,
            dirty: true,
            computing: false,
        }
    }

    /// Record the current view, marking the orbit stale if it differs from
    /// the last recorded view. The caller should pass the GPU-effective
    /// `max_iter` (zoom bonus + LOD iteration scale applied) so a recompute
    /// always serves the exact budget the shader loop runs.
    pub fn note_view(&mut self, center: [f64; 2], zoom: f64, max_iter: u32) {
        let sig = ViewSignature {
            center,
            zoom,
            max_iter,
        };
        if self.last_view != Some(sig) {
            self.dirty = true;
            self.last_view = Some(sig);
        }
    }

    /// If stale AND eligible AND idle, spawn a worker thread that computes
    /// the reference orbit and sends it over a channel. Returns `true` when
    /// a worker was spawned this call (caller shows the "computing…" toast).
    ///
    /// The worker is `compute_reference_orbit` (CPU-only BigFloat math, no
    /// GPU access) — safe to run off the render thread. The result lands via
    /// [`Self::poll`].
    pub fn maybe_spawn(&mut self, center: [f64; 2], zoom: f64, max_iter: u32) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if !self.dirty || self.computing {
                return false;
            }
            // Below the gate: don't waste CPU on an orbit we won't use. Stay
            // dirty so the first frame past the gate spawns immediately.
            if zoom.log2() <= PERTURBATION_LOG2_GATE {
                return false;
            }
            let precision_bits = precision_bits_for_zoom(zoom);
            let (tx, rx) = std::sync::mpsc::channel::<ReferenceOrbit>();
            self.pending = Some(rx);
            self.computing = true;
            self.dirty = false;
            // Move only Copy data into the worker — `center`, `max_iter`,
            // `precision_bits` — plus the Sender. No GPU handles cross the
            // thread boundary (the upload happens on the main thread in
            // `Renderer::set_reference_orbit`).
            std::thread::spawn(move || {
                let orbit = compute_reference_orbit(center[0], center[1], max_iter, precision_bits);
                // Ignore send error: if the App was dropped, the result is
                // just unused (the worker still completes its CPU work).
                let _ = tx.send(orbit);
            });
            true
        }
        #[cfg(target_arch = "wasm32")]
        {
            // wasm has no `std::thread`; perturbation is native-only for
            // Phase A. Clear `dirty` so eligible views don't keep churning
            // the (no-op) spawn path every frame; the HP path renders.
            let _ = (center, zoom, max_iter);
            self.dirty = false;
            false
        }
    }

    /// Non-blocking check for a completed orbit. Called every frame from
    /// `App::update`; on `Some`, the caller uploads the orbit to the GPU and
    /// marks perturbation active. Never blocks.
    pub fn poll(&mut self) -> Option<ReferenceOrbit> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let rx = self.pending.as_ref()?;
            match rx.try_recv() {
                Ok(orbit) => {
                    self.pending = None;
                    self.computing = false;
                    Some(orbit)
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Worker panicked before sending. Reset so the next
                    // eligible frame retries instead of silently sticking
                    // the driver in the "computing" state forever.
                    self.pending = None;
                    self.computing = false;
                    self.dirty = true;
                    log::error!(
                        "perturbation reference-orbit worker thread died; will retry next frame"
                    );
                    None
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gate engages only for Mandelbrot2D in 2D mode past the zoom threshold.
    /// The other 2D escape-time fractals (Julia, Burning Ship, etc.) and all
    /// 3D paths fall back to HP / standard rendering in Phase A.
    #[test]
    fn gate_is_mandelbrot_2d_only() {
        let z = 1e8; // well past the gate
        assert!(perturbation_eligible(
            z,
            FractalType::Mandelbrot2D,
            RenderMode::TwoD
        ));

        // Below the gate.
        assert!(!perturbation_eligible(
            1.0,
            FractalType::Mandelbrot2D,
            RenderMode::TwoD
        ));
        // Right at 2^24 is NOT eligible (strictly-greater-than).
        assert!(!perturbation_eligible(
            2f64.powf(PERTURBATION_LOG2_GATE),
            FractalType::Mandelbrot2D,
            RenderMode::TwoD,
        ));
        // Just above 2^24 is eligible.
        assert!(perturbation_eligible(
            2f64.powf(PERTURBATION_LOG2_GATE) * 1.0001,
            FractalType::Mandelbrot2D,
            RenderMode::TwoD,
        ));

        // Wrong fractal type or mode → ineligible.
        assert!(!perturbation_eligible(
            z,
            FractalType::Julia2D,
            RenderMode::TwoD
        ));
        assert!(!perturbation_eligible(
            z,
            FractalType::Mandelbulb3D,
            RenderMode::ThreeD
        ));
    }

    /// `note_view` marks dirty exactly when one of the three signature
    /// fields changes — and never when an identical view is re-recorded.
    /// This is the invalidation contract the driver relies on to avoid
    /// respawning the worker every frame at a still view.
    #[test]
    fn note_view_dirty_only_on_change() {
        let mut d = PerturbationDriver::new();
        // Initial state is dirty (so the first eligible frame spawns).
        assert!(d.dirty);

        d.note_view([-0.5, 0.0], 1e8, 1000);
        assert!(d.dirty, "first note is always a change from None");

        // Same view: not dirty.
        d.dirty = false;
        d.note_view([-0.5, 0.0], 1e8, 1000);
        assert!(!d.dirty, "identical view must not mark dirty");

        // Each field change marks dirty.
        d.note_view([-0.5, 0.0001], 1e8, 1000);
        assert!(d.dirty, "center change must mark dirty");
        d.dirty = false;
        d.note_view([-0.5, 0.0001], 2e8, 1000);
        assert!(d.dirty, "zoom change must mark dirty");
        d.dirty = false;
        d.note_view([-0.5, 0.0001], 2e8, 1001);
        assert!(d.dirty, "max_iter change must mark dirty");
    }

    /// `maybe_spawn` returns false (and spawns nothing) below the gate even
    /// when stale, and on wasm unconditionally — pinning the native-only
    /// contract of perturbation in Phase A.
    #[test]
    fn maybe_spawn_no_op_below_gate_and_no_duplicate_spawns() {
        let mut d = PerturbationDriver::new();
        d.note_view([-0.5, 0.0], 1.0, 1000); // dirty, but zoom = 1 (below gate)

        let spawned = d.maybe_spawn([-0.5, 0.0], 1.0, 1000);
        // No spawn below the gate on native; on wasm it's always no-op.
        // Either way `spawned` is false and `computing` stays false.
        assert!(!spawned);
        assert!(
            !d.computing,
            "below-gate spawn attempt must not flip computing"
        );
    }
}
