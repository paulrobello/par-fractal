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

use crate::deep_zoom::orbit::{
    FractalKind, ReferenceOrbit, compute_reference_orbit_best,
    compute_reference_orbit_best_precise, parse_center_decimal, precision_bits_for_zoom,
};
use crate::fractal::{FractalType, RenderMode};

/// Zoom (as log2) above which perturbation activates.
///
/// The 2026-07-17 root-cause placed the double-float collapse at ~3e7
/// (log2 ≈ 24.8) and set the gate at 24, claiming HP rendered correctly
/// below it. A 2026-07-18 GPU-vs-CPU-f64 crosscheck (Pearson correlation
/// over luma) disproved that: df corr holds ~0.6 at zoom 1e3 but falls to
/// 0.47 by 4.48e5 (log2 18.8) with visible per-pixel noise — the
/// "blocky/pixelated" mid-zoom symptom — and to 0.43 by 1.1e7. The loss is
/// the same Metal FTZ / sub-ULP lo-word mechanism, just arriving far earlier
/// than the 07-17 measurement (taken at 256x256) suggested.
///
/// Lowered to 13.3 (zoom ≈ 1e4) so perturbation — whose reference orbit is a
/// CPU BigFloat, immune to the shader df loss — covers the entire degraded
/// band; f32 (clean below ~1e4) handles the rest. For the four
/// perturbation-eligible types this supersedes the HP path (the shader checks
/// perturbation first); HP remains the deep-zoom path for the other 2D types.
pub const PERTURBATION_LOG2_GATE: f64 = 13.3;

/// True when perturbation should engage for this view.
///
/// Eligible for every 2D escape-time type that maps to a [`FractalKind`]
/// (Mandelbrot, Julia, Burning Ship, Tricorn — Phase B step 7 lifted the
/// Mandelbrot-only restriction). 3D / attractor / Buddhabrot paths return
/// `None` from [`FractalKind::from_fractal_type`] and fall through to their
/// own renderers. Below the gate the existing HP path renders correctly.
pub fn perturbation_eligible(
    zoom_2d: f64,
    fractal_type: FractalType,
    render_mode: RenderMode,
) -> bool {
    FractalKind::from_fractal_type(fractal_type).is_some()
        && render_mode == RenderMode::TwoD
        && zoom_2d.log2() > PERTURBATION_LOG2_GATE
}

/// View signature used to invalidate the reference orbit. Centers are the
/// CPU-side f64 `FractalParams.center_2d` (still the entry point in Phase
/// A); `max_iter` is the GPU-effective value (zoom bonus + LOD scale) so a
/// recompute always serves the exact iteration budget the shader uses.
/// `aspect` is included because the 3×3 probe grid spreads candidates by
/// `0.5 * view_half_extent`, which depends on aspect — a window resize that
/// changes aspect changes the probe locations and must invalidate.
///
/// `fractal_type` and `julia_c` are part of the signature (Phase B step 7):
/// switching fractal type (Mandelbrot ↔ Julia ↔ …) or editing `julia_c`
/// changes the reference orbit and must invalidate.
#[derive(Debug, Clone, PartialEq)]
struct ViewSignature {
    center: [f64; 2],
    /// High-precision decimal-string override (ENH-001 Phase C). `None` uses
    /// the f64 `center`; `Some` re-routes the orbit through `parse_center_decimal`.
    center_precise: Option<[String; 2]>,
    zoom: f64,
    aspect: f32,
    max_iter: u32,
    fractal_type: FractalType,
    julia_c: [f32; 2],
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
    /// always serves the exact budget the shader loop runs. `aspect` is the
    /// camera's width/height — used by the probe-grid selector to spread
    /// candidates across the visible view.
    ///
    /// `fractal_type` and `julia_c` are part of the signature (Phase B step
    /// 7): switching type or editing `julia_c` invalidates the orbit.
    #[allow(clippy::too_many_arguments)] // +center_precise (ENH-001 Phase C); each field is independently meaningful.
    pub fn note_view(
        &mut self,
        center: [f64; 2],
        center_precise: Option<[String; 2]>,
        zoom: f64,
        aspect: f32,
        max_iter: u32,
        fractal_type: FractalType,
        julia_c: [f32; 2],
    ) {
        let sig = ViewSignature {
            center,
            center_precise,
            zoom,
            aspect,
            max_iter,
            fractal_type,
            julia_c,
        };
        // Compare by reference so `sig` can still move into `last_view`
        // (ViewSignature owns Strings → not Copy).
        let changed = self.last_view.as_ref() != Some(&sig);
        if changed {
            self.dirty = true;
        }
        self.last_view = Some(sig);
    }

    /// If stale AND eligible AND idle, spawn a worker thread that computes
    /// the reference orbit and sends it over a channel. Returns `true` when
    /// a worker was spawned this call (caller shows the "computing…" toast).
    ///
    /// The worker runs `compute_reference_orbit_best` — a 3×3 probe across
    /// the view that picks the latest-escaping candidate (CPU-only BigFloat
    /// math, no GPU access) — safe to run off the render thread. The result
    /// lands via [`Self::poll`]. Probing 9 candidates is roughly 9× the
    /// single-orbit cost; the GUI stays responsive because the work happens
    /// off the render thread.
    ///
    /// `fractal_type` selects the recurrence via [`FractalKind`]; `julia_c`
    /// is the fixed c for Julia (ignored for the other kinds).
    #[allow(clippy::too_many_arguments)] // +center_precise (ENH-001 Phase C); mirrors note_view's signature.
    pub fn maybe_spawn(
        &mut self,
        center: [f64; 2],
        center_precise: Option<[String; 2]>,
        zoom: f64,
        aspect: f32,
        max_iter: u32,
        fractal_type: FractalType,
        julia_c: [f32; 2],
    ) -> bool {
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
            // Non-eligible type: nothing to compute. Stay non-dirty so we
            // don't churn; `perturbation_eligible` short-circuits this path
            // upstream, but the guard is cheap defense-in-depth.
            let kind = match FractalKind::from_fractal_type(fractal_type) {
                Some(k) => k,
                None => {
                    return false;
                }
            };
            let precision_bits = precision_bits_for_zoom(zoom);
            // View half-extent in complex units per axis — matches the
            // shader's `delta_c_scale` derivation so the probe grid lands at
            // the same UV spacing the per-pixel deltas use.
            let aspect_f64 = aspect as f64;
            let inv_zoom = 2.0 / zoom;
            let view_half_extent_x = inv_zoom * aspect_f64;
            let view_half_extent_y = inv_zoom;
            let julia_c_f64 = [julia_c[0] as f64, julia_c[1] as f64];
            let (tx, rx) = std::sync::mpsc::channel::<ReferenceOrbit>();
            self.pending = Some(rx);
            self.computing = true;
            self.dirty = false;
            // Move the view data into the worker — `center`, the half-extents,
            // `max_iter`, `precision_bits`, `kind`, `julia_c_f64`, and the
            // owned `center_precise` strings — plus the Sender. No GPU handles
            // cross the thread boundary (the upload happens on the main thread
            // in `Renderer::set_reference_orbit`).
            std::thread::spawn(move || {
                // ENH-001 Phase C: when a precise decimal-string center is set,
                // parse it to FBig and use the precise selector so the orbit's
                // center isn't bounded by f64. A malformed string falls back to
                // the f64 path (the UI validates on entry, so this is defense).
                let orbit = match center_precise {
                    Some(p) => match (
                        parse_center_decimal(&p[0], precision_bits),
                        parse_center_decimal(&p[1], precision_bits),
                    ) {
                        (Ok(cr), Ok(ci)) => compute_reference_orbit_best_precise(
                            kind,
                            cr,
                            ci,
                            view_half_extent_x,
                            view_half_extent_y,
                            julia_c_f64,
                            max_iter,
                            precision_bits,
                        ),
                        _ => compute_reference_orbit_best(
                            kind,
                            center[0],
                            center[1],
                            view_half_extent_x,
                            view_half_extent_y,
                            julia_c_f64,
                            max_iter,
                            precision_bits,
                        ),
                    },
                    None => compute_reference_orbit_best(
                        kind,
                        center[0],
                        center[1],
                        view_half_extent_x,
                        view_half_extent_y,
                        julia_c_f64,
                        max_iter,
                        precision_bits,
                    ),
                };
                // Ignore send error: if the App was dropped, the result is
                // just unused (the worker still completes its CPU work).
                let _ = tx.send(orbit);
            });
            true
        }
        #[cfg(target_arch = "wasm32")]
        {
            // wasm has no `std::thread`; perturbation is native-only.
            // Clear `dirty` so eligible views don't keep churning the
            // (no-op) spawn path every frame; the HP path renders.
            let _ = (
                center,
                center_precise,
                zoom,
                aspect,
                max_iter,
                fractal_type,
                julia_c,
            );
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

    /// Gate engages for every 2D escape-time type that maps to a
    /// [`FractalKind`] (Mandelbrot, Julia, Burning Ship, Tricorn — Phase B
    /// step 7 extended the Phase A Mandelbrot-only gate). Other 2D types
    /// (Phoenix, Newton, …) and all 3D paths fall back to HP / standard
    /// rendering.
    #[test]
    fn gate_covers_perturbation_eligible_types() {
        let z = 1e8; // well past the gate
        assert!(perturbation_eligible(
            z,
            FractalType::Mandelbrot2D,
            RenderMode::TwoD
        ));
        assert!(perturbation_eligible(
            z,
            FractalType::Julia2D,
            RenderMode::TwoD
        ));
        assert!(perturbation_eligible(
            z,
            FractalType::BurningShip2D,
            RenderMode::TwoD
        ));
        assert!(perturbation_eligible(
            z,
            FractalType::Tricorn2D,
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

        // Non-perturbation fractal types or modes → ineligible.
        assert!(!perturbation_eligible(
            z,
            FractalType::Phoenix2D,
            RenderMode::TwoD
        ));
        assert!(!perturbation_eligible(
            z,
            FractalType::Mandelbulb3D,
            RenderMode::ThreeD
        ));
    }

    /// `note_view` marks dirty exactly when one of the signature
    /// fields changes — and never when an identical view is re-recorded.
    /// This is the invalidation contract the driver relies on to avoid
    /// respawning the worker every frame at a still view.
    #[test]
    fn note_view_dirty_only_on_change() {
        let mut d = PerturbationDriver::new();
        // Initial state is dirty (so the first eligible frame spawns).
        assert!(d.dirty);

        d.note_view(
            [-0.5, 0.0],
            None,
            1e8,
            16.0 / 9.0,
            1000,
            FractalType::Mandelbrot2D,
            [0.0, 0.0],
        );
        assert!(d.dirty, "first note is always a change from None");

        // Same view: not dirty.
        d.dirty = false;
        d.note_view(
            [-0.5, 0.0],
            None,
            1e8,
            16.0 / 9.0,
            1000,
            FractalType::Mandelbrot2D,
            [0.0, 0.0],
        );
        assert!(!d.dirty, "identical view must not mark dirty");

        // Each field change marks dirty.
        d.note_view(
            [-0.5, 0.0001],
            None,
            1e8,
            16.0 / 9.0,
            1000,
            FractalType::Mandelbrot2D,
            [0.0, 0.0],
        );
        assert!(d.dirty, "center change must mark dirty");
        d.dirty = false;
        d.note_view(
            [-0.5, 0.0001],
            None,
            2e8,
            16.0 / 9.0,
            1000,
            FractalType::Mandelbrot2D,
            [0.0, 0.0],
        );
        assert!(d.dirty, "zoom change must mark dirty");
        d.dirty = false;
        d.note_view(
            [-0.5, 0.0001],
            None,
            2e8,
            4.0 / 3.0,
            1000,
            FractalType::Mandelbrot2D,
            [0.0, 0.0],
        );
        assert!(d.dirty, "aspect change must mark dirty");
        d.dirty = false;
        d.note_view(
            [-0.5, 0.0001],
            None,
            2e8,
            4.0 / 3.0,
            1001,
            FractalType::Mandelbrot2D,
            [0.0, 0.0],
        );
        assert!(d.dirty, "max_iter change must mark dirty");
        d.dirty = false;
        d.note_view(
            [-0.5, 0.0001],
            None,
            2e8,
            4.0 / 3.0,
            1001,
            FractalType::Julia2D,
            [0.0, 0.0],
        );
        assert!(d.dirty, "fractal_type change must mark dirty");
        d.dirty = false;
        d.note_view(
            [-0.5, 0.0001],
            None,
            2e8,
            4.0 / 3.0,
            1001,
            FractalType::Julia2D,
            [-0.7, 0.27015],
        );
        assert!(d.dirty, "julia_c change must mark dirty");
    }

    /// `maybe_spawn` returns false (and spawns nothing) below the gate even
    /// when stale, and on wasm unconditionally — pinning the native-only
    /// contract of perturbation.
    #[test]
    fn maybe_spawn_no_op_below_gate_and_no_duplicate_spawns() {
        let mut d = PerturbationDriver::new();
        // dirty, but zoom = 1 (below gate)
        d.note_view(
            [-0.5, 0.0],
            None,
            1.0,
            16.0 / 9.0,
            1000,
            FractalType::Mandelbrot2D,
            [0.0, 0.0],
        );

        let spawned = d.maybe_spawn(
            [-0.5, 0.0],
            None,
            1.0,
            16.0 / 9.0,
            1000,
            FractalType::Mandelbrot2D,
            [0.0, 0.0],
        );
        // No spawn below the gate on native; on wasm it's always no-op.
        // Either way `spawned` is false and `computing` stays false.
        assert!(!spawned);
        assert!(
            !d.computing,
            "below-gate spawn attempt must not flip computing"
        );
    }

    // ---- ENH-001 Phase C: precise (decimal-string) center invalidation ----

    /// A precise center override is part of the view signature: setting or
    /// changing it must mark the orbit stale so the driver recomputes against
    /// the high-precision center (not the truncated f64 mirror). This is the
    /// contract that makes a pasted deep-zoom coordinate actually take effect.
    #[test]
    fn note_view_dirty_on_precise_center_change() {
        let mut d = PerturbationDriver::new();
        let zoom = 1e8_f64;
        let aspect = 16.0 / 9.0;
        let max_iter = 1000u32;
        let jc = [0.0f32, 0.0];

        // Establish a baseline view (no precise center).
        d.note_view(
            [-0.5, 0.0],
            None,
            zoom,
            aspect,
            max_iter,
            FractalType::Mandelbrot2D,
            jc,
        );
        d.dirty = false;

        // Same f64 center, but a precise override now present → must invalidate.
        d.note_view(
            [-0.5, 0.0],
            Some([
                "-0.743643887037151071".to_string(),
                "0.1318259042".to_string(),
            ]),
            zoom,
            aspect,
            max_iter,
            FractalType::Mandelbrot2D,
            jc,
        );
        assert!(
            d.dirty,
            "setting a precise center must invalidate the orbit"
        );

        // Re-recording the identical precise view must NOT re-invalidate.
        d.dirty = false;
        d.note_view(
            [-0.5, 0.0],
            Some([
                "-0.743643887037151071".to_string(),
                "0.1318259042".to_string(),
            ]),
            zoom,
            aspect,
            max_iter,
            FractalType::Mandelbrot2D,
            jc,
        );
        assert!(!d.dirty, "identical precise view must not re-invalidate");

        // Changing the precise string's digits (sub-f64) must invalidate.
        d.note_view(
            [-0.5, 0.0],
            Some([
                "-0.743643887037151087".to_string(),
                "0.1318259042".to_string(),
            ]),
            zoom,
            aspect,
            max_iter,
            FractalType::Mandelbrot2D,
            jc,
        );
        assert!(
            d.dirty,
            "sub-f64 change to the precise string must invalidate"
        );

        // Clearing the precise override must invalidate (revert to f64 path).
        d.dirty = false;
        d.note_view(
            [-0.5, 0.0],
            None,
            zoom,
            aspect,
            max_iter,
            FractalType::Mandelbrot2D,
            jc,
        );
        assert!(d.dirty, "clearing the precise center must invalidate");
    }
}
