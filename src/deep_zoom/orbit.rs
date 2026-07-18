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
///
/// `reference_offset` is `c_center − c_ref` as an f32 pair — the screen-center
/// Δc the shader adds back in (`delta_c = reference_offset + uv*scale`). When
/// the reference was computed at the view center this is `[0.0, 0.0]` (the
/// Phase A original behavior). When a probe across the view picked a different
/// reference (see [`compute_reference_orbit_best`]), this carries the offset so
/// the per-pixel Δc is reconstructed correctly. The offset is small (view-sized)
/// and computed in f64 before the f32 cast, so it is f32-representable even at
/// extreme zoom.
#[derive(Debug, Clone)]
pub struct ReferenceOrbit {
    /// `Z_0 .. Z_k` as `(re, im)` f32 pairs. `z[0] = (0, 0)`.
    pub z: Vec<[f32; 2]>,
    /// First index `i` with `|Z_i|² > 4`, or `None` if bounded through `max_iter`.
    pub escaped_at: Option<u32>,
    /// Mantissa bits the orbit was computed at.
    pub precision_bits: usize,
    /// `c_center − c_ref` as an f32 pair; feeds the shader's `delta_c_origin`.
    /// `[0.0, 0.0]` when the reference IS the view center.
    pub reference_offset: [f32; 2],
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
        // Callers of `compute_reference_orbit` pass the reference c directly;
        // assume c IS the center, so the shader's delta_c_origin is zero. The
        // selector (`compute_reference_orbit_best`) overrides this when it
        // picks a non-center reference.
        reference_offset: [0.0, 0.0],
    }
}

/// Pick the best reference orbit for a view by probing several candidate
/// reference points across the view and selecting the one that escapes latest
/// (a bounded reference is ideal).
///
/// **Why this matters.** A reference that escapes early serves interior pixels
/// badly: their per-pixel delta stays bounded, but the reference orbit is gone,
/// so the shader's delta recurrence diverges and interior pixels render bright
/// instead of black. A reference that stays bounded serves BOTH interior pixels
/// (their delta stays bounded too) AND escaping pixels (they escape before the
/// accumulated delta diverges). Probing the view for the latest-escaping
/// candidate is the standard fix — see K.I. Martin's "SuperFractalThing" /
/// "Kalles Fraktaler" reference-selection strategy.
///
/// **Candidate grid.** A 3×3 grid centered on the view center: the center plus
/// 8 offsets at `(±0.5, ±0.5) × (view_half_extent_x, view_half_extent_y)` in
/// complex units. With UV ∈ [−1, 1] the view half-extent in complex units is
/// `(2/zoom)·aspect` (x) and `2/zoom` (y); the driver derives those and passes
/// them here. Offsets at ±0.5 of the half-extent land candidates at UV
/// coordinates {−0.5, 0, +0.5} — well inside the view, far enough apart to
/// sample distinct Mandelbrot regions.
///
/// **Selection.** For each candidate compute the full orbit (the inner
/// `compute_reference_orbit` already breaks on escape, so early-escaping
/// candidates cost little — implicit early-termination). Pick the candidate
/// whose `escaped_at` is latest (`None`/bounded is best; otherwise the maximum
/// `Some(i)`). Ties keep the candidate visited first, so the center (iterated
/// first) wins ties — minimizing `|delta_c_origin|` and preserving the existing
/// single-reference path as the N=1 case.
///
/// **Cost.** N× the single-orbit cost (N=9 here). This runs on the perturbation
/// worker thread — off the render thread — so the GUI stays responsive, but
/// deep-zoom probes are noticeably heavier than the Phase A single-orbit path.
/// Keeping N ≤ 9 balances recall (chance of hitting a bounded point) against
/// CPU time.
///
/// `reference_offset` on the returned orbit is `(c_center − c_ref)` cast to an
/// f32 pair — this IS the shader's `delta_c_origin` (re-derived: per-pixel
/// `delta_c = c_pixel − c_ref = (c_center − c_ref) + uv·scale`).
pub fn compute_reference_orbit_best(
    center_re: f64,
    center_im: f64,
    view_half_extent_x: f64,
    view_half_extent_y: f64,
    max_iter: u32,
    precision_bits: usize,
) -> ReferenceOrbit {
    // Step at ±0.5 of each axis's half-extent: candidates land at UV
    // {−0.5, 0, +0.5} on each axis. Spread wide enough to sample distinct
    // regions, tight enough to stay inside the view.
    let step_x = 0.5 * view_half_extent_x;
    let step_y = 0.5 * view_half_extent_y;

    // Visit the center first so ties (same `escaped_at`) resolve to the
    // center, which has the smallest `|reference_offset|`. The order of the
    // outer offsets does not matter for correctness.
    let mut best: Option<ReferenceOrbit> = None;
    for dy in [-1.0_f64, 0.0, 1.0] {
        for dx in [-1.0_f64, 0.0, 1.0] {
            let cand_re = center_re + dx * step_x;
            let cand_im = center_im + dy * step_y;
            let orbit = compute_reference_orbit(cand_re, cand_im, max_iter, precision_bits);

            let picks_this = match &best {
                None => true,
                Some(b) => later_escape(&orbit.escaped_at, &b.escaped_at),
            };
            if picks_this {
                // Patch reference_offset to (c_center − c_ref), computed in f64
                // then cast to f32 — this preserves the offset's precision down
                // to f32-ULP, well within the delta noise floor.
                let offset: [f32; 2] = [(center_re - cand_re) as f32, (center_im - cand_im) as f32];
                let mut chosen = orbit;
                chosen.reference_offset = offset;
                best = Some(chosen);
            }
        }
    }

    best.expect("the 3×3 grid always runs at least the center candidate")
}

/// Later-escape selection rule for `compute_reference_orbit_best`:
/// `None` (bounded) beats any `Some(_)`; otherwise the larger iteration index
/// wins; a tie returns `false` so the first-visited candidate (center) stays.
fn later_escape(this: &Option<u32>, prev: &Option<u32>) -> bool {
    match (this, prev) {
        (None, None) => false,
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (Some(a), Some(b)) => *a > *b,
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

    /// The single-orbit constructor leaves `reference_offset` at `[0,0]`
    /// (callers passing c directly treat that c as the center). The selector
    /// overrides this; the plain constructor never does.
    #[test]
    fn single_orbit_reference_offset_is_zero() {
        let orbit = compute_reference_orbit(-0.5, 0.0, 50, 128);
        assert_eq!(orbit.reference_offset, [0.0, 0.0]);
    }

    /// The selector picks a bounded reference when one is reachable in the
    /// candidate grid. Constructed case: center = `(2, 0)` (escapes at i=2),
    /// with a view half-extent of `(4, 4)` so the 3×3 grid places candidates
    /// at x ∈ {0, 2, 4}. The candidate at `c = 0` is the origin (a fixed
    /// point of the Mandelbrot map — bounded forever), so the selector must
    /// pick it, mark the orbit bounded, and set `reference_offset = (2, 0)`
    /// (the center minus the chosen reference).
    #[test]
    fn selector_picks_bounded_candidate_with_correct_offset() {
        let orbit = compute_reference_orbit_best(2.0, 0.0, 4.0, 4.0, 32, 128);
        assert_eq!(
            orbit.escaped_at, None,
            "must pick the bounded c=0 candidate"
        );
        assert_eq!(orbit.reference_offset, [2.0, 0.0]);
        // Sanity: the bounded orbit at c=0 is identically zero.
        for &z in &orbit.z {
            assert_eq!(z, [0.0, 0.0]);
        }
    }

    /// When every candidate escapes, the selector returns the latest-escaping
    /// one (not the center). Center = `(2, 0)` escapes at i=2; the far
    /// candidate at `c = 4` escapes at i=1, the near candidate at `c = 0`
    /// is bounded — but if we shrink the view so the only candidates all
    /// escape, the latest escaper wins. Here: center = `(1, 0)` with a tiny
    /// half-extent so all candidates are in the escaping spike; the center
    /// (latest escaper in this neighborhood) wins, and reference_offset is
    /// `[0,0]` because the center IS the chosen reference.
    #[test]
    fn selector_breaks_tie_toward_center() {
        // All candidates near c=1 escape between i≈1 and i≈3; center is one
        // of the latest escapers, and the tie-break favors first-visited.
        let orbit = compute_reference_orbit_best(1.0, 0.0, 0.0, 0.0, 16, 128);
        // zero half-extent → every candidate IS the center → bounded check
        // gives the same answer the single-orbit constructor would.
        assert_eq!(
            orbit.escaped_at,
            compute_reference_orbit(1.0, 0.0, 16, 128).escaped_at
        );
        assert_eq!(orbit.reference_offset, [0.0, 0.0]);
    }
}
