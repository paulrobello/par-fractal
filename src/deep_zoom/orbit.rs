//! Arbitrary-precision Mandelbrot reference orbit (ENH-001, Phase A step 1).
//!
//! One reference orbit per view, computed in [`dashu_float::FBig`] (arbitrary
//! precision, pure Rust, wasm-safe), emitted as `Z_n` f32 pairs for the GPU
//! delta-iteration shader to read from a storage buffer. The orbit is the
//! source of truth that makes per-pixel f32 deltas correct at any zoom: the
//! GPU never carries a full-magnitude coordinate, only small `Δz`/`Δc`.
//!
//! The recurrence is the plain Mandelbrot map `Z_{n+1} = Z_n² + C` with
//! bailout `|Z|² > 4`, matching the shader's `mandelbrot_hp`
//! (`shaders/fractal.wgsl`) and the f64 mirror `reference::mandelbrot_hp_f64`.
//! The tests pin the orbit to a direct f64 iteration so a regression in the
//! BigFloat walk is caught without a GPU.

use dashu_float::FBig;

/// Precision (mantissa bits) the reference orbit needs at a given zoom.
///
/// Perturbation deltas are f32 (~24-bit), so the reference must carry enough
/// headroom that `Δc` (the per-pixel spacing) is resolved well above the
/// delta noise floor. `ceil(log2(zoom)) + 64` is the standard budget — 64 guard
/// bits over the pixel spacing — floored at 64. Matches ENH-001's design
/// decision (`max(64, ceil(log2(zoom)) + 64)`).
pub fn precision_bits_for_zoom(zoom: f64) -> usize {
    let bits = (zoom.max(1.0).log2().ceil() as usize) + 64;
    bits.max(64)
}

/// A computed reference orbit.
///
/// `z[n]` holds `Z_n` as an `(re, im)` f32 pair — the low-precision mirror the
/// GPU reads from the storage buffer (deltas carry the precision, so f32 pairs
/// are sufficient; this is standard). `escaped_at` is the index at which the
/// reference itself exceeded the bailout (`None` if it stayed bounded through
/// `max_iter`); pixels needing more iterations than the reference served must
/// rebase (handled in the shader).
#[derive(Debug, Clone)]
pub struct ReferenceOrbit {
    /// `Z_0 .. Z_k` as `(re, im)` f32 pairs. `z[0] = (0, 0)`.
    pub z: Vec<[f32; 2]>,
    /// First index `i` with `|Z_i|² > 4`, or `None` if bounded through `max_iter`.
    pub escaped_at: Option<u32>,
    /// Mantissa bits the orbit was computed at.
    pub precision_bits: usize,
}

/// Compute the Mandelbrot reference orbit at center `(c_re, c_im)`.
///
/// `Z_0 = 0`, `Z_{n+1} = Z_n² + C`, stopping at `|Z|² > 4` (escape) or
/// `max_iter`. Each `Z_n` is emitted as an f32 pair. `precision_bits` controls
/// the `FBig` mantissa (use [`precision_bits_for_zoom`]).
///
/// `c_re`/`c_im` are f64 here (Phase A enters from the existing f64 center). A
/// later phase stores the center as a decimal string and parses it straight to
/// `FBig` so precision is not bounded by f64 — but even from f64, raising the
/// `FBig` precision lets the *iteration* carry guard bits f64 cannot, which is
/// what keeps the orbit valid past the ~1e11 f64 ceiling.
pub fn compute_reference_orbit(
    c_re: f64,
    c_im: f64,
    max_iter: u32,
    precision_bits: usize,
) -> ReferenceOrbit {
    let p = precision_bits.max(53);
    // f64→FBig is fallible (NaN/Inf have no finite representation), hence
    // `try_from`. The center is finite upstream; a NaN here is a real bug worth
    // surfacing rather than silently rendering garbage.
    let with_p = |v: f64| -> FBig {
        FBig::try_from(v)
            .expect("reference orbit center must be finite")
            .with_precision(p)
            .value()
    };
    let cr = with_p(c_re);
    let ci = with_p(c_im);
    let two = with_p(2.0);
    let four = with_p(4.0);

    let mut zr = with_p(0.0);
    let mut zi = with_p(0.0);

    let mut z: Vec<[f32; 2]> = Vec::with_capacity(max_iter as usize);
    let mut escaped_at: Option<u32> = None;

    for i in 0..max_iter {
        // Emit Z_i (z[0] = Z_0 = 0). Done before the escape test so the
        // shader has the value whose magnitude is being tested.
        z.push([zr.to_f64().value() as f32, zi.to_f64().value() as f32]);

        // Escape test on the emitted Z_i.
        if zr.clone() * zr.clone() + zi.clone() * zi.clone() > four.clone() {
            escaped_at = Some(i);
            break;
        }

        // Z_{n+1} = Z_n² + C:  new_re = re² − im² + c_re,  new_im = 2·re·im + c_im.
        let nr = zr.clone() * zr.clone() - zi.clone() * zi.clone() + cr.clone();
        let ni = (zr.clone() * zi.clone()) * two.clone() + ci.clone();
        zr = nr;
        zi = ni;
    }

    ReferenceOrbit {
        z,
        escaped_at,
        precision_bits: p,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `precision_bits_for_zoom` follows `max(64, ceil(log2(zoom)) + 64)`.
    #[test]
    fn precision_bits_for_zoom_scales() {
        assert_eq!(precision_bits_for_zoom(1.0), 64);
        assert_eq!(precision_bits_for_zoom(0.5), 64); // floored, no negatives
        // log2(1e10) ≈ 33.22 → ceil 34 → 98
        assert_eq!(precision_bits_for_zoom(1e10), 98);
        // log2(1e30) ≈ 99.66 → ceil 100 → 164
        assert_eq!(precision_bits_for_zoom(1e30), 164);
    }

    /// `c = 0` is the fixed point: the orbit stays at the origin and never escapes.
    #[test]
    fn orbit_origin_is_stationary() {
        let orbit = compute_reference_orbit(0.0, 0.0, 50, 128);
        assert_eq!(orbit.escaped_at, None);
        assert_eq!(orbit.z.len(), 50);
        for &z in &orbit.z {
            assert_eq!(z, [0.0, 0.0]);
        }
    }

    /// A clearly-escaping point sets `escaped_at` at the first index whose
    /// magnitude² exceeds 4, and that emitted value satisfies the bailout.
    #[test]
    fn orbit_escapes_and_marks_index() {
        // c = 2: Z_0=0, Z_1=2 (|Z|²=4, not >4), Z_2=6 (|Z|²=36>4) → escape at i=2.
        let orbit = compute_reference_orbit(2.0, 0.0, 64, 128);
        let escaped = orbit.escaped_at.expect("should escape");
        assert_eq!(orbit.z[0], [0.0, 0.0]);
        let [re, im] = orbit.z[escaped as usize];
        assert!(
            re * re + im * im > 4.0,
            "emitted escaping value must satisfy bailout"
        );
    }

    /// The orbit must agree with a direct f64 Mandelbrot walk for a bounded
    /// point — this is the core correctness pin tying the BigFloat path to the
    /// ENH-007 f64 ground truth without a GPU. Uses `c = -0.5`, whose orbit
    /// stays small and bounded, so the f32 mirror matches f64 tightly.
    #[test]
    fn orbit_matches_f64_recurrence() {
        let max_iter = 100u32;
        let orbit = compute_reference_orbit(-0.5, 0.0, max_iter, 200);

        // Direct f64 walk of the same recurrence.
        let (mut zr, mut zi): (f64, f64) = (0.0, 0.0);
        for n in 0..orbit.z.len().min(40) {
            let (er, ei) = (orbit.z[n][0] as f64, orbit.z[n][1] as f64);
            let dr = (er - zr).abs();
            let di = (ei - zi).abs();
            // f32 mirror vs f64: bounded orbit stays O(1), so agree to ~1e-6.
            assert!(dr < 1e-6, "n={n}: re drift {dr:e} (orbit={er}, f64={zr})");
            assert!(di < 1e-6, "n={n}: im drift {di:e}");
            let nr = zr * zr - zi * zi + -0.5;
            zi *= 2.0 * zr;
            zr = nr;
        }
    }

    /// `escaped_at` lines up with the f64 walk's escape iteration at a real
    /// deep-zoom target (the seahorse valley center), confirming the BigFloat
    /// orbit — not just a bounded point — matches f64 ground truth.
    #[test]
    fn orbit_escape_matches_f64_at_seahorse() {
        let (cr, ci, max_iter) = (-0.7436438870, 0.1318259042, 1000u32);
        let orbit = compute_reference_orbit(cr, ci, max_iter, 200);

        // f64 escape iteration (mirror of the shader's HP walk).
        let (mut zr, mut zi): (f64, f64) = (0.0, 0.0);
        let mut f64_escaped: Option<u32> = None;
        for i in 0..max_iter {
            if zr * zr + zi * zi > 4.0 {
                f64_escaped = Some(i);
                break;
            }
            let nr = zr * zr - zi * zi + cr;
            zi = 2.0 * zr * zi + ci;
            zr = nr;
        }
        assert_eq!(
            orbit.escaped_at, f64_escaped,
            "escape iteration must match f64"
        );
    }
}
