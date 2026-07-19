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

use crate::fractal::FractalType;
use core::str::FromStr;
use dashu_float::DBig;
use dashu_float::FBig;
use dashu_float::ops::Abs;

/// Which 2D escape-time map the reference orbit iterates (ENH-001 Phase B
/// step 7). Each kind has its own per-pixel delta recurrence in
/// `shaders/fractal.wgsl`; the CPU orbit just iterates the underlying map
/// (`mandelbrot_hp` / `julia_hp` / `burning_ship_hp` / `tricorn_hp`) in
/// arbitrary precision, starting from the kind-appropriate initial condition:
///
/// - `Mandelbrot` / `Tricorn` / `BurningShip`: `Z_0 = 0`, c = view center.
///   The per-pixel variable is c (the position); Δc = c_pixel − c_ref.
/// - `Julia`: `Z_0 = view center`, c = `julia_c` (fixed for the whole view).
///   The per-pixel variable is z_0 (the starting point); Δz_0 = z_0_pixel −
///   center. The Δc term in the recurrence cancels because c is identical
///   for every pixel.
///
/// Built from a [`FractalType`] via [`Self::from_fractal_type`]; types that
/// don't use the 2D perturbation path (attractors, 3D, Buddhabrot) map to
/// `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractalKind {
    Mandelbrot,
    Julia,
    BurningShip,
    Tricorn,
}

impl FractalKind {
    /// Map a [`FractalType`] to its perturbation kind, or `None` if the type
    /// doesn't use the 2D perturbation path (handled by other renderers).
    /// The discriminants match `src/shaders/fractal.wgsl`'s
    /// `fs_main_2d` perturbation gate (`0, 1, 4, 5`).
    pub fn from_fractal_type(t: FractalType) -> Option<Self> {
        match t {
            FractalType::Mandelbrot2D => Some(Self::Mandelbrot),
            FractalType::Julia2D => Some(Self::Julia),
            FractalType::BurningShip2D => Some(Self::BurningShip),
            FractalType::Tricorn2D => Some(Self::Tricorn),
            _ => None,
        }
    }
}

/// One step of each map in `FBig`. Used by [`compute_reference_orbit`] so the
/// BigFloat walk matches the shader's `*_hp` recurrence exactly.
///
/// - Mandelbrot / Julia: `Z² + c`
/// - Tricorn: `conj(Z)² + c = (re² − im², −2·re·im) + c`
/// - Burning Ship: `(|re| + i|im|)² + c = (re² − im², 2·|re|·|im|) + c`
///   (the imaginary part is non-negative because |re|, |im| ≥ 0).
fn step_complex(
    kind: FractalKind,
    zr: &FBig,
    zi: &FBig,
    cr: &FBig,
    ci: &FBig,
    two: &FBig,
) -> (FBig, FBig) {
    match kind {
        FractalKind::Mandelbrot | FractalKind::Julia => {
            let nr = zr.clone() * zr.clone() - zi.clone() * zi.clone() + cr.clone();
            let ni = (zr.clone() * zi.clone()) * two.clone() + ci.clone();
            (nr, ni)
        }
        FractalKind::Tricorn => {
            // conj(z)² + c: real part is re² − im² (same as Mandelbrot),
            // imaginary part is −2·re·im.
            let nr = zr.clone() * zr.clone() - zi.clone() * zi.clone() + cr.clone();
            let ni = ci.clone() - (zr.clone() * zi.clone()) * two.clone();
            (nr, ni)
        }
        FractalKind::BurningShip => {
            // (|re| + i|im|)² = (re² − im², 2·|re|·|im|); apply abs first.
            let ar = zr.clone().abs();
            let ai = zi.clone().abs();
            let nr = ar.clone() * ar.clone() - ai.clone() * ai.clone() + cr.clone();
            let ni = (ar * ai) * two.clone() + ci.clone();
            (nr, ni)
        }
    }
}

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

/// Parse a decimal-string center coordinate to a base-2 `FBig` at the given
/// mantissa precision (ENH-001 Phase C — precise center).
///
/// Accepts the decimal forms `DBig::from_str` supports: plain (`-0.7436…`),
/// with explicit exponent (`1.5e-10`), and signed. The result is a base-2
/// `FBig` (the orbit's working type) at exactly `max(precision_bits, 53)`
/// mantissa bits, so it composes directly with the f64-derived operands in the
/// orbit walk. This is the entry point that frees the reference orbit's center
/// from f64 precision, keeping it valid past the ~1e11 f64 ceiling.
///
/// Returns `Err(message)` on malformed input (the caller — UI / settings loader
/// — surfaces the message); never panics.
pub fn parse_center_decimal(s: &str, precision_bits: usize) -> Result<FBig, String> {
    let p = precision_bits.max(53);
    let decimal = DBig::from_str(s).map_err(|e| format!("{e}"))?;
    // Base-10 → base-2 with Zero rounding (the orbit `FBig`'s mode); the
    // `.with_precision` then pins the exact target mantissa width.
    let binary = decimal.to_binary().value();
    Ok(binary.with_precision(p).value())
}

/// Compute the reference orbit at center `(center_re, center_im)` from f64
/// coordinates (Phase A entry point).
///
/// Iterates the map selected by `kind` (Mandelbrot / Julia / Burning Ship /
/// Tricorn), stopping at `|Z|² > 4` (escape) or `max_iter`. Each `Z_n` is
/// emitted as an f32 pair. `precision_bits` controls the `FBig` mantissa
/// (use [`precision_bits_for_zoom`]).
///
/// **Per-kind initial condition.** For Mandelbrot / Tricorn / Burning Ship
/// the per-pixel variable is `c`, so `Z_0 = 0` and `c = (center_re,
/// center_im)`. For Julia the per-pixel variable is `z_0` (the starting
/// point), so `Z_0 = (center_re, center_im)` and `c = julia_c` (fixed for
/// the whole view — passed as the `julia_c` parameter, ignored for the
/// other kinds).
///
/// For a center beyond f64 precision use [`compute_reference_orbit_precise`]
/// with a value from [`parse_center_decimal`]; raising the `FBig` precision
/// here still lets the *iteration* carry guard bits f64 cannot, which keeps
/// the orbit valid partway past the ~1e11 f64 ceiling.
pub fn compute_reference_orbit(
    kind: FractalKind,
    center_re: f64,
    center_im: f64,
    julia_c: [f64; 2],
    max_iter: u32,
    precision_bits: usize,
) -> ReferenceOrbit {
    // f64→FBig is fallible (NaN/Inf have no finite representation); the center
    // is finite upstream, a NaN here is a real bug worth surfacing.
    let to_fbig = |v: f64| FBig::try_from(v).expect("reference orbit center must be finite");
    // Per-kind (c, Z_0). Julia inverts the role: c is the fixed `julia_c`
    // and Z_0 is the view center; the other kinds put c at the view center
    // and start Z_0 at zero.
    let (cr, ci, z0r, z0i) = match kind {
        FractalKind::Julia => (
            to_fbig(julia_c[0]),
            to_fbig(julia_c[1]),
            to_fbig(center_re),
            to_fbig(center_im),
        ),
        _ => (
            to_fbig(center_re),
            to_fbig(center_im),
            to_fbig(0.0),
            to_fbig(0.0),
        ),
    };
    orbit_from_fbig(kind, cr, ci, z0r, z0i, max_iter, precision_bits)
}

/// Compute the reference orbit at a precise (decimal-string-derived) center
/// (ENH-001 Phase C).
///
/// Identical to [`compute_reference_orbit`] but enters from `FBig` center
/// coordinates parsed by [`parse_center_decimal`], so the reference orbit's
/// center is not bounded by f64. `julia_c` remains f64 here — it is the fixed c
/// for Julia (stored as f32 in settings); only the *center* carries the precise
/// path in this phase.
pub fn compute_reference_orbit_precise(
    kind: FractalKind,
    center_re: FBig,
    center_im: FBig,
    julia_c: [f64; 2],
    max_iter: u32,
    precision_bits: usize,
) -> ReferenceOrbit {
    let to_fbig = |v: f64| FBig::try_from(v).expect("julia_c must be finite");
    let (cr, ci, z0r, z0i) = match kind {
        FractalKind::Julia => (
            to_fbig(julia_c[0]),
            to_fbig(julia_c[1]),
            center_re,
            center_im,
        ),
        _ => (center_re, center_im, to_fbig(0.0), to_fbig(0.0)),
    };
    orbit_from_fbig(kind, cr, ci, z0r, z0i, max_iter, precision_bits)
}

/// Iterate the reference orbit from already-`FBig` starting values (shared core
/// for the f64 and precise-center entry points). All inputs are normalized to
/// `max(precision_bits, 53)` mantissa bits — widening is exact, so mixing an
/// f64-derived value with a 100-bit precise center loses nothing.
fn orbit_from_fbig(
    kind: FractalKind,
    cr: FBig,
    ci: FBig,
    z0r: FBig,
    z0i: FBig,
    max_iter: u32,
    precision_bits: usize,
) -> ReferenceOrbit {
    let p = precision_bits.max(53);
    let norm = |v: FBig| v.with_precision(p).value();
    let cr = norm(cr);
    let ci = norm(ci);
    let mut zr = norm(z0r);
    let mut zi = norm(z0i);
    let two = norm(FBig::try_from(2.0).expect("2.0 is finite"));
    let four = norm(FBig::try_from(4.0).expect("4.0 is finite"));

    let mut z: Vec<[f32; 2]> = Vec::with_capacity(max_iter as usize);
    let mut escaped_at: Option<u32> = None;

    for i in 0..max_iter {
        // Emit Z_i (z[0] = Z_0). Done before the escape test so the shader
        // has the value whose magnitude is being tested.
        z.push([zr.to_f64().value() as f32, zi.to_f64().value() as f32]);

        // Escape test on the emitted Z_i.
        if zr.clone() * zr.clone() + zi.clone() * zi.clone() > four.clone() {
            escaped_at = Some(i);
            break;
        }

        // Z_{n+1} = step(Z_n, c).
        let (nr, ni) = step_complex(kind, &zr, &zi, &cr, &ci, &two);
        zr = nr;
        zi = ni;
    }

    ReferenceOrbit {
        z,
        escaped_at,
        precision_bits: p,
        // Callers of `compute_reference_orbit[_precise]` pass the reference c
        // directly; assume c IS the center, so the shader's delta_c_origin is
        // zero. The selector (`compute_reference_orbit_best[_precise]`)
        // overrides this when it picks a non-center reference.
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
#[allow(clippy::too_many_arguments)] // 8 args: kind + center + 2 half-extents + julia_c + iter + precision — all individually meaningful; a struct would obscure the call sites.
pub fn compute_reference_orbit_best(
    kind: FractalKind,
    center_re: f64,
    center_im: f64,
    view_half_extent_x: f64,
    view_half_extent_y: f64,
    julia_c: [f64; 2],
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
            let orbit =
                compute_reference_orbit(kind, cand_re, cand_im, julia_c, max_iter, precision_bits);

            let picks_this = match &best {
                None => true,
                Some(b) => later_escape(&orbit.escaped_at, &b.escaped_at),
            };
            if picks_this {
                // Patch reference_offset to (c_center − c_ref), computed in f64
                // then cast to f32 — this preserves the offset's precision down
                // to f32-ULP, well within the delta noise floor.
                //
                // For Julia, `cand` is the candidate Z_0 (not c — c is the
                // fixed julia_c for every candidate). The shader's per-pixel
                // variable for Julia is z_0, so the same offset semantics
                // carry: `delta_c_origin = z_0_center − z_0_ref`.
                let offset: [f32; 2] = [(center_re - cand_re) as f32, (center_im - cand_im) as f32];
                let mut chosen = orbit;
                chosen.reference_offset = offset;
                best = Some(chosen);
            }
        }
    }

    best.expect("the 3×3 grid always runs at least the center candidate")
}

/// Precise-center variant of [`compute_reference_orbit_best`] (ENH-001 Phase C).
///
/// Identical 3×3 probe and latest-escape selection, but the center is an
/// `FBig` parsed by [`parse_center_decimal`], so the probe candidates inherit
/// the center's full precision. The per-axis offsets are view-sized f64 values
/// (exact to add — they sit far above the delta noise floor). The driver uses
/// this when a `center_2d_precise` override is set; otherwise the f64
/// [`compute_reference_orbit_best`] applies.
#[allow(clippy::too_many_arguments)] // mirrors the f64 selector's signature
pub fn compute_reference_orbit_best_precise(
    kind: FractalKind,
    center_re: FBig,
    center_im: FBig,
    view_half_extent_x: f64,
    view_half_extent_y: f64,
    julia_c: [f64; 2],
    max_iter: u32,
    precision_bits: usize,
) -> ReferenceOrbit {
    let step_x = 0.5 * view_half_extent_x;
    let step_y = 0.5 * view_half_extent_y;

    let mut best: Option<ReferenceOrbit> = None;
    for dy in [-1.0_f64, 0.0, 1.0] {
        for dx in [-1.0_f64, 0.0, 1.0] {
            // Offsets are view-sized (f64-precise is plenty — they sit far
            // above the delta noise floor); convert to FBig and add to the
            // precise center so candidate coordinates inherit full precision.
            let off_x = FBig::try_from(dx * step_x).expect("half-extent offset is finite");
            let off_y = FBig::try_from(dy * step_y).expect("half-extent offset is finite");
            let cand_re = center_re.clone() + off_x;
            let cand_im = center_im.clone() + off_y;
            let orbit = compute_reference_orbit_precise(
                kind,
                cand_re,
                cand_im,
                julia_c,
                max_iter,
                precision_bits,
            );

            let picks_this = match &best {
                None => true,
                Some(b) => later_escape(&orbit.escaped_at, &b.escaped_at),
            };
            if picks_this {
                // reference_offset = c_center − c_ref, cast to f32 — identical
                // to the f64 selector's offset (the offset is the view-sized
                // probe step, which f32 represents fine).
                let offset: [f32; 2] = [(-(dx * step_x)) as f32, (-(dy * step_y)) as f32];
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
        let orbit = compute_reference_orbit(FractalKind::Mandelbrot, 0.0, 0.0, [0.0, 0.0], 50, 128);
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
        let orbit = compute_reference_orbit(FractalKind::Mandelbrot, 2.0, 0.0, [0.0, 0.0], 64, 128);
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
        let orbit = compute_reference_orbit(
            FractalKind::Mandelbrot,
            -0.5,
            0.0,
            [0.0, 0.0],
            max_iter,
            200,
        );

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
        let orbit =
            compute_reference_orbit(FractalKind::Mandelbrot, cr, ci, [0.0, 0.0], max_iter, 200);

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
        let orbit =
            compute_reference_orbit(FractalKind::Mandelbrot, -0.5, 0.0, [0.0, 0.0], 50, 128);
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
        let orbit = compute_reference_orbit_best(
            FractalKind::Mandelbrot,
            2.0,
            0.0,
            4.0,
            4.0,
            [0.0, 0.0],
            32,
            128,
        );
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
        let orbit = compute_reference_orbit_best(
            FractalKind::Mandelbrot,
            1.0,
            0.0,
            0.0,
            0.0,
            [0.0, 0.0],
            16,
            128,
        );
        // zero half-extent → every candidate IS the center → bounded check
        // gives the same answer the single-orbit constructor would.
        assert_eq!(
            orbit.escaped_at,
            compute_reference_orbit(FractalKind::Mandelbrot, 1.0, 0.0, [0.0, 0.0], 16, 128)
                .escaped_at
        );
        assert_eq!(orbit.reference_offset, [0.0, 0.0]);
    }

    // --- Phase B step 7: per-kind orbit sanity ----------------------------

    /// `from_fractal_type` maps exactly the four eligible 2D escape-time
    /// types and nothing else. Anything off-path returns `None`.
    #[test]
    fn fractal_kind_from_type_maps_eligible_types() {
        assert_eq!(
            FractalKind::from_fractal_type(FractalType::Mandelbrot2D),
            Some(FractalKind::Mandelbrot)
        );
        assert_eq!(
            FractalKind::from_fractal_type(FractalType::Julia2D),
            Some(FractalKind::Julia)
        );
        assert_eq!(
            FractalKind::from_fractal_type(FractalType::BurningShip2D),
            Some(FractalKind::BurningShip)
        );
        assert_eq!(
            FractalKind::from_fractal_type(FractalType::Tricorn2D),
            Some(FractalKind::Tricorn)
        );
        // Non-eligible types fall through.
        assert_eq!(FractalKind::from_fractal_type(FractalType::Phoenix2D), None);
        assert_eq!(
            FractalKind::from_fractal_type(FractalType::Mandelbulb3D),
            None
        );
    }

    /// Julia's orbit starts at Z_0 = center (not zero) and iterates against
    /// the fixed `julia_c`. Pin the first few Z_n to the direct f64 walk to
    /// confirm the (c, Z_0) inversion is correct: with `Z_0 = (0, 0)` and
    /// `julia_c = (-0.7, 0.27015)`, `Z_1 = julia_c`.
    #[test]
    fn julia_orbit_starts_at_center_uses_fixed_c() {
        let julia_c = [-0.7_f64, 0.27015_f64];
        let orbit = compute_reference_orbit(FractalKind::Julia, 0.0, 0.0, julia_c, 16, 128);
        // Z_0 = center = (0, 0).
        assert_eq!(orbit.z[0], [0.0, 0.0]);
        // Z_1 = Z_0² + julia_c = julia_c (within f32 cast).
        let [r, i] = orbit.z[1];
        assert!((r - julia_c[0] as f32).abs() < 1e-6, "Z_1.re={r}");
        assert!((i - julia_c[1] as f32).abs() < 1e-6, "Z_1.im={i}");
    }

    /// Tricorn's orbit iterates `conj(Z)² + c`: with `c = 0` and `Z_0 = 0`
    /// the orbit stays at the origin (same as Mandelbrot at c=0); but with
    /// `c = (1, 0)` the imaginary part should flip sign each step (the
    /// `−2·re·im` term), distinguishing it from the Mandelbrot recurrence.
    #[test]
    fn tricorn_orbit_uses_conjugate_square() {
        let orbit = compute_reference_orbit(FractalKind::Tricorn, 1.0, 1.0, [0.0, 0.0], 8, 128);
        // Z_0 = 0. Z_1 = conj(0)² + (1,1) = (1, 1).
        assert_eq!(orbit.z[0], [0.0, 0.0]);
        let [r1, i1] = orbit.z[1];
        assert!(
            (r1 - 1.0).abs() < 1e-6 && (i1 - 1.0).abs() < 1e-6,
            "Z_1=( {r1}, {i1} )"
        );
        // Z_2 = conj(Z_1)² + c = (1, -1)² + (1, 1) = (1 - 1 + 1, -2 + 1) = (1, -1).
        let [r2, i2] = orbit.z[2];
        assert!((r2 - 1.0).abs() < 1e-6, "Z_2.re={r2}");
        assert!(
            (i2 - (-1.0)).abs() < 1e-6,
            "Z_2.im={i2} (conj square negates im)"
        );
    }

    /// Burning Ship's orbit iterates `(|re| + i|im|)² + c`: at `c = (1, 0)`
    /// starting from `Z_0 = 0`, `Z_1 = (1, 0)`; then `|Z_1| = (1, 0)`,
    /// `Z_2 = (1, 0)² + (1, 0) = (2, 0)`. Real-axis walk — confirms abs is
    /// applied before squaring.
    #[test]
    fn burning_ship_orbit_applies_abs_before_square() {
        let orbit = compute_reference_orbit(FractalKind::BurningShip, 1.0, 0.0, [0.0, 0.0], 8, 128);
        assert_eq!(orbit.z[0], [0.0, 0.0]);
        let [r1, i1] = orbit.z[1];
        assert!(
            (r1 - 1.0).abs() < 1e-6 && (i1 - 0.0).abs() < 1e-6,
            "Z_1=({r1}, {i1})"
        );
        let [r2, i2] = orbit.z[2];
        // |1|² + 1 = 2 on re; im stays 0.
        assert!(
            (r2 - 2.0).abs() < 1e-6 && (i2 - 0.0).abs() < 1e-6,
            "Z_2=({r2}, {i2})"
        );
    }

    // --- ENH-001 step 8: deep-zoom orbit timing (ignored; run with --ignored) ---
    //
    // Measures where time goes at the engagement zoom (1e8) so step 8 picks
    // the lever that actually addresses the bottleneck rather than guessing.

    /// Realistic 1e8 timing: single orbit vs the 9× probe the driver runs in
    /// production, plus per-candidate escape status. Run with:
    ///   cargo test -r orbit_timing_1e8 -- --ignored --nocapture
    #[test]
    #[ignore]
    fn orbit_timing_1e8() {
        let (cr, ci) = (-0.7436438870_f64, 0.1318259042_f64); // seahorse valley
        let zoom = 1e8_f64;
        let max_iter = (1000u32 + ((zoom.log2() * 15.0) as u32)).max(16); // 1398
        let precision = precision_bits_for_zoom(zoom); // 91
        // View half-extents at 1e8, 16:9.
        let hx = (2.0 / zoom) * (16.0 / 9.0);
        let hy = 2.0 / zoom;

        let t0 = std::time::Instant::now();
        let single = compute_reference_orbit(
            FractalKind::Mandelbrot,
            cr,
            ci,
            [0.0, 0.0],
            max_iter,
            precision,
        );
        let t_single = t0.elapsed();

        // Per-candidate escape status (which of the 9 probes are bounded?).
        let step_x = 0.5 * hx;
        let step_y = 0.5 * hy;
        let mut bounded = 0u32;
        let mut latest_escape: Option<u32> = None;
        for dy in [-1.0_f64, 0.0, 1.0] {
            for dx in [-1.0_f64, 0.0, 1.0] {
                let o = compute_reference_orbit(
                    FractalKind::Mandelbrot,
                    cr + dx * step_x,
                    ci + dy * step_y,
                    [0.0, 0.0],
                    max_iter,
                    precision,
                );
                match o.escaped_at {
                    None => bounded += 1,
                    Some(e) => latest_escape = Some(latest_escape.map_or(e, |p| p.max(e))),
                }
            }
        }

        let t0 = std::time::Instant::now();
        let _best = compute_reference_orbit_best(
            FractalKind::Mandelbrot,
            cr,
            ci,
            hx,
            hy,
            [0.0, 0.0],
            max_iter,
            precision,
        );
        let t_probe = t0.elapsed();

        eprintln!(
            "ORBIT-TIMING 1e8: max_iter={max_iter} prec={precision}bits\n\
             single-orbit: {t_single:?}  (len={}, escaped_at={:?})\n\
             9x probe:     {t_probe:?}\n\
             candidates:   bounded={bounded}/9  latest_escape={latest_escape:?}",
            single.z.len(),
            single.escaped_at,
        );
    }

    /// Zoom-ladder timing: how does single-orbit time scale with zoom? Run with:
    ///   cargo test -r orbit_timing_zoom_ladder -- --ignored --nocapture
    #[test]
    #[ignore]
    fn orbit_timing_zoom_ladder() {
        let (cr, ci) = (-0.7436438870_f64, 0.1318259042_f64);
        eprintln!(
            "ZOOM-LADDER single orbit (seahorse): zoom  max_iter  prec  time  len  escaped_at"
        );
        for &zoom in &[1e6_f64, 1e8, 1e12, 1e20, 1e30] {
            let max_iter = (1000u32 + ((zoom.log2() * 15.0) as u32)).max(16);
            let precision = precision_bits_for_zoom(zoom);
            let t0 = std::time::Instant::now();
            let o = compute_reference_orbit(
                FractalKind::Mandelbrot,
                cr,
                ci,
                [0.0, 0.0],
                max_iter,
                precision,
            );
            let t = t0.elapsed();
            eprintln!(
                "  {zoom:>7.0e}  {max_iter:>8}  {precision:>4}  {t:?}  len={}  escaped_at={:?}",
                o.z.len(),
                o.escaped_at,
            );
        }
    }

    // ---- ENH-001 Phase C: decimal-string center parsing (precise center) ----

    /// A decimal center string parses to a base-2 `FBig` that preserves digits
    /// beyond f64 precision. Two strings that f64 conflates (they agree in the
    /// first ~16 significant digits) must parse to *distinct* `FBig` values at
    /// 80 bits of mantissa — this is the whole point of the precise-center
    /// path: the reference orbit's center is not bounded by f64.
    #[test]
    fn parse_center_decimal_preserves_sub_f64_precision() {
        // Differ only in the 17th significant digit → f64 collapses them.
        let a = "-0.743643887037151071";
        let b = "-0.743643887037151087";
        let fa: f64 = a.parse().unwrap();
        let fb: f64 = b.parse().unwrap();
        assert_eq!(
            fa.to_bits(),
            fb.to_bits(),
            "test premise: f64 must conflate these two strings"
        );

        let pa = parse_center_decimal(a, 80).unwrap();
        let pb = parse_center_decimal(b, 80).unwrap();
        assert_ne!(pa, pb, "80-bit parse must distinguish sub-f64 centers");
    }

    /// A center within f64 range round-trips: parsing the decimal string and
    /// converting back to f64 lands on the same value (to f64 ULP tolerance;
    /// `FBig`'s Zero rounding mode differs from f64's round-to-nearest, so an
    /// exact bit-equality assertion would be brittle).
    #[test]
    fn parse_center_decimal_round_trips_f64_values() {
        let s = "-0.743643887037151";
        let f: f64 = s.parse().unwrap();
        let parsed = parse_center_decimal(s, 80).unwrap();
        assert!(
            (parsed.to_f64().value() - f).abs() < 1e-14,
            "f64 round-trip drifted: {} vs {}",
            parsed.to_f64().value(),
            f
        );
    }

    /// `DBig::from_str` accepts scientific notation (`E`), so the parser must
    /// too — deep-zoom coordinates are often written compactly (e.g.
    /// `-7.43…e-1`). Verifies `E` is accepted and the value is correct (a
    /// full-precision mantissa keeps the f64 comparison tight).
    #[test]
    fn parse_center_decimal_accepts_scientific_notation() {
        let parsed = parse_center_decimal("1.2345678901234567e-2", 80).unwrap();
        let expected: f64 = 1.2345678901234567e-2;
        let got = parsed.to_f64().value();
        assert!(
            (got - expected).abs() / expected < 1e-15,
            "scientific notation mis-parsed: {} vs {}",
            got,
            expected
        );
    }

    /// Garbage input is an error, not a panic — the UI hands user-typed strings
    /// to this function.
    #[test]
    fn parse_center_decimal_rejects_garbage() {
        assert!(parse_center_decimal("not_a_number", 80).is_err());
        assert!(parse_center_decimal("", 80).is_err());
        assert!(parse_center_decimal("nan", 80).is_err());
    }

    /// An f64-representable center parsed as a decimal string must produce the
    /// same orbit as the f64 entry path — the precise path is a superset, not a
    /// divergence. `-0.5` is exactly representable in both f64 and decimal, so
    /// the two orbits must agree to f32 mirror tolerance.
    #[test]
    fn precise_center_orbit_matches_f64_path() {
        let kind = FractalKind::Mandelbrot;
        let max_iter = 200u32;
        let precision = 128usize;

        let f64_orbit = compute_reference_orbit(kind, -0.5, 0.0, [0.0, 0.0], max_iter, precision);
        let cr = parse_center_decimal("-0.5", precision).unwrap();
        let ci = parse_center_decimal("0.0", precision).unwrap();
        let precise_orbit =
            compute_reference_orbit_precise(kind, cr, ci, [0.0, 0.0], max_iter, precision);

        assert_eq!(f64_orbit.escaped_at, precise_orbit.escaped_at);
        assert_eq!(f64_orbit.z.len(), precise_orbit.z.len());
        for (a, b) in f64_orbit.z.iter().zip(precise_orbit.z.iter()) {
            assert!((a[0] - b[0]).abs() < 1e-6, "re drift: {} vs {}", a[0], b[0]);
            assert!((a[1] - b[1]).abs() < 1e-6, "im drift: {} vs {}", a[1], b[1]);
        }
    }

    /// The precise selector picks the same reference (same `escaped_at` and same
    /// `reference_offset`) as the f64 selector for an f64-representable center —
    /// the precise path is a strict superset, not a behavioral fork. This is the
    /// contract Phase 2's driver relies on when it swaps in the precise selector.
    #[test]
    fn best_precise_matches_f64_best_for_representable_center() {
        let kind = FractalKind::Mandelbrot;
        let (cr, ci) = (-0.7436438870_f64, 0.1318259042_f64);
        let zoom = 1e8_f64;
        let hx = (2.0 / zoom) * (16.0 / 9.0);
        let hy = 2.0 / zoom;
        let max_iter = 500u32;
        let precision = precision_bits_for_zoom(zoom);

        let f64_best =
            compute_reference_orbit_best(kind, cr, ci, hx, hy, [0.0, 0.0], max_iter, precision);
        let cr_b = parse_center_decimal(&format!("{cr}"), precision).unwrap();
        let ci_b = parse_center_decimal(&format!("{ci}"), precision).unwrap();
        let prec_best = compute_reference_orbit_best_precise(
            kind,
            cr_b,
            ci_b,
            hx,
            hy,
            [0.0, 0.0],
            max_iter,
            precision,
        );

        assert_eq!(
            f64_best.escaped_at, prec_best.escaped_at,
            "same reference selected"
        );
        assert_eq!(
            f64_best.reference_offset, prec_best.reference_offset,
            "same delta_c_origin offset"
        );
    }
}
