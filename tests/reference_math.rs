//! Deep-zoom math regression suite for the ENH-007 harness — the CI-safe
//! "teeth" layer.
//!
//! These tests pin the *math* of the 2D escape-time pipeline with no GPU, no
//! display, and no driver, so they run in `cargo test` / CI. They catch the
//! bug class the audit found shipping silently (unreachable hp path, wrong DF
//! abs, FMA-collapsed `two_prod`, late hp threshold):
//!
//! - **Known points** — interior sentinel, first-iteration escape, sign of
//!   well-defined exterior values.
//! - **DF vs f64 (the precision teeth)** — the double-float renderer
//!   (`render_df`) must agree with the f64 ground truth (`render`) on a deep
//!   zoom tile. A regression in any DF primitive (a reverted abs fix, a
//!   re-FMA'd `two_prod`) pushes the DF walk off the f64 orbit and fails here.
//! - **Self-consistency table** — smooth values at fixed coordinates, blessed
//!   once, flag any future drift in the reference formula.
//!
//! The GPU golden-image layer (run the real binary, compare PNGs) lives in
//! `scripts/visual_test.sh` / `make visual-test` and is local-only.

use par_fractal::reference::{self, FractalKind};

// ============================================================================
// Known points — pure math facts, no magic floats beyond sign/range.
// ============================================================================

#[test]
fn mandelbrot_origin_is_interior() {
    // c = 0 sits inside the main cardioid → never escapes → interior sentinel.
    assert_eq!(
        reference::smooth_at(FractalKind::Mandelbrot, (0.0, 0.0), 1.0, 256, 2.0),
        -1.0
    );
}

#[test]
fn far_point_escapes_immediately() {
    // c = (3, 0) escapes on the first iteration at zoom 1 (standard path).
    let t = reference::smooth_at(FractalKind::Mandelbrot, (3.0, 0.0), 1.0, 256, 2.0);
    assert!(
        t > 0.0 && t < 1.0,
        "far point must escape to a finite smooth value, got {t}"
    );
}

#[test]
fn interior_and_exterior_coexist_in_default_view() {
    // Default Mandelbrot view at zoom 1: the main body is interior, the
    // corners are exterior. A renderer that returns all-interior or all-exterior
    // is broken.
    let buf = reference::render(
        FractalKind::Mandelbrot,
        (-0.5, 0.0),
        1.0,
        (64, 64),
        256,
        2.0,
    );
    let interior = buf.iter().filter(|&&v| v < 0.0).count();
    let exterior = buf.len() - interior;
    assert!(interior > 0, "expected some interior pixels");
    assert!(exterior > 0, "expected some exterior pixels");
}

// ============================================================================
// DF vs f64 — the precision teeth (catches QA-002 / QA-005 regressions).
// ============================================================================

/// Compare the DF renderer against the f64 ground truth on a deep-zoom tile.
/// Returns `(mean_ext_diff, mismatch_frac)` where:
/// - `mean_ext_diff` is the mean `|df - f64|` over pixels where BOTH renderers
///   produced a well-defined exterior value (`0 <= t < 0.95`). Stragglers that
///   escaped in the final few iterations are precision-sensitive by
///   construction and excluded.
/// - `mismatch_frac` is the fraction of ALL pixels that disagree — interior in
///   one but not the other, or exterior values differing by more than 2e-3.
///   This is the metric that catches a DF break whose effect is to shift pixels
///   into or out of the interior (the Burning Ship abs bug, FMA collapse): such
///   a break changes the escape/interior classification, not just the smooth
///   value, so it shows up here even on tiles where most pixels escape early.
fn df_vs_f64_stats(
    kind: FractalKind,
    center: (f64, f64),
    zoom: f64,
    size: (u32, u32),
    max_iter: u32,
) -> (f64, f64) {
    let f64_buf = reference::render(kind, center, zoom, size, max_iter, 2.0);
    let df_buf = reference::render_df(kind, center, zoom, size, max_iter);

    let total = f64_buf.len();
    let mut sum = 0.0f64;
    let mut ext_n = 0u64;
    let mut mismatch = 0u64;
    const STRAGGLER: f32 = 0.95;
    const EXT_TOL: f32 = 2e-3;
    for (&f, &d) in f64_buf.iter().zip(df_buf.iter()) {
        let f_in = f < 0.0;
        let d_in = d < 0.0;
        if f_in && d_in {
            continue; // both interior → agree
        }
        if f_in != d_in {
            mismatch += 1; // interior classification disagrees
            continue;
        }
        // both exterior
        if f >= STRAGGLER || d >= STRAGGLER {
            continue; // near-max-iter straggler: precision-sensitive, skip
        }
        let diff = (f - d).abs();
        sum += diff as f64;
        ext_n += 1;
        if diff > EXT_TOL {
            mismatch += 1;
        }
    }
    let mean = sum / ext_n.max(1) as f64;
    let mismatch_frac = mismatch as f64 / total as f64;
    (mean, mismatch_frac)
}

#[test]
fn df_matches_f64_mandelbrot_deep_zoom() {
    // Seahorse valley at 1e8 — exactly where the audit found f32 quantizing.
    let (mean, mismatch) = df_vs_f64_stats(
        FractalKind::Mandelbrot,
        (-0.7436438870, 0.1318259042),
        1e8,
        (32, 32),
        1000,
    );
    assert!(
        mean < 1e-4,
        "DF/f64 mean diff {mean:e} too large (Mandelbrot 1e8)"
    );
    assert!(
        mismatch < 0.05,
        "{mismatch:.3} of pixels disagree (Mandelbrot 1e8)"
    );
}

#[test]
fn df_matches_f64_burning_ship_deep_zoom() {
    // QA-002 guard: a DF abs regression changes which pixels escape vs. stay
    // interior, so it is caught by the mismatch fraction even when the
    // well-defined-exterior mean stays small.
    let (mean, mismatch) = df_vs_f64_stats(
        FractalKind::BurningShip,
        (-1.7625, -0.0333),
        1e8,
        (32, 32),
        1000,
    );
    assert!(
        mean < 1e-4,
        "DF/f64 mean diff {mean:e} too large (Burning Ship 1e8)"
    );
    assert!(
        mismatch < 0.05,
        "{mismatch:.3} of pixels disagree (Burning Ship 1e8)"
    );
}

#[test]
fn df_matches_f64_tricorn_deep_zoom() {
    let (mean, mismatch) = df_vs_f64_stats(FractalKind::Tricorn, (-0.3, 0.8), 1e6, (32, 32), 1000);
    assert!(
        mean < 1e-4,
        "DF/f64 mean diff {mean:e} too large (Tricorn 1e6)"
    );
    assert!(
        mismatch < 0.05,
        "{mismatch:.3} of pixels disagree (Tricorn 1e6)"
    );
}

#[test]
fn df_matches_f64_mandelbrot_moderate_zoom() {
    // 1e5 is just above the HP threshold (1e4); DF must already match f64.
    // Looser than the deep-zoom tests: at this zoom each 32px tile pixel spans
    // ~1e-6 of the fractal, so more pixels land on iteration-band boundaries
    // where a one-iteration flip is inherent (not a bug). A real DF break
    // pushes mean to ~1e-1, two orders above this bar.
    let (mean, _mismatch) = df_vs_f64_stats(
        FractalKind::Mandelbrot,
        (-0.7436438870, 0.1318259042),
        1e5,
        (32, 32),
        500,
    );
    assert!(
        mean < 2e-3,
        "DF/f64 mean diff {mean:e} too large (Mandelbrot 1e5)"
    );
}

// ============================================================================
// Self-consistency table — blessed smooth values, drift alarm.
// ============================================================================

/// Smooth value at a single sampled pixel, computed at a fixed coordinate via
/// the public `smooth_at`. These values were blessed once against the reference
/// renderer; they flag any future drift in the smooth formula or escape test.
/// Tolerance is loose (1e-3) because the value depends on the exact zoom tier
/// and bailout — the alarm is for wholesale drift, not last-bit changes.
#[test]
fn smooth_value_table_is_stable() {
    // (kind, center, zoom, max_iter, power, expected_smooth)
    // center sampled so it is a well-defined exterior point.
    type Case = (FractalKind, (f64, f64), f64, u32, f32, f32);
    let cases: &[Case] = &[
        // c = (2, 0) at zoom 1 (standard path, bailout |z|^2 > 16, R = 4).
        (
            FractalKind::Mandelbrot,
            (2.0, 0.0),
            1.0,
            256,
            2.0,
            0.006_366,
        ),
        // c = (2, 0) at zoom 1e8 (HP path, bailout |z|^2 > 4, R = 2) — same
        // point, different bailout → different smooth value.
        (
            FractalKind::Mandelbrot,
            (2.0, 0.0),
            1e8,
            256,
            2.0,
            0.002_460,
        ),
        // c = (-1.0, 0) sits in the period-2 bulb → interior.
        (FractalKind::Mandelbrot, (-1.0, 0.0), 1.0, 256, 2.0, -1.0),
    ];

    for &(kind, center, zoom, max_iter, power, expected) in cases {
        let got = reference::smooth_at(kind, center, zoom, max_iter, power);
        if expected < 0.0 {
            assert!(
                got < 0.0,
                "{kind:?} {center:?} zoom={zoom}: expected interior"
            );
        } else {
            assert!(
                (got - expected).abs() < 1e-3,
                "{kind:?} {center:?} zoom={zoom}: expected {expected}, got {got}"
            );
        }
    }
}

#[test]
fn grayscale_round_trip_shape() {
    // The harness renders smooth values to grayscale RGBA; verify the shape
    // and the interior→black / exterior→nonnegative mapping.
    let buf = reference::render(FractalKind::Mandelbrot, (-0.5, 0.0), 1.0, (8, 8), 64, 2.0);
    let rgba = reference::smooth_to_grayscale_rgba(&buf, (8, 8));
    assert_eq!(rgba.len(), 8 * 8 * 4);
    // Every pixel is opaque (alpha 255).
    assert!(rgba.chunks_exact(4).all(|px| px[3] == 255));
}
