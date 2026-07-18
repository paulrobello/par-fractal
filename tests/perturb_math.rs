//! Throwaway diagnostic (ENH-001): mirror the `mandelbrot_perturb` WGSL in f64
//! and compare its escape iteration to the direct f64 Mandelbrot, at both
//! shallow (large Δc) and deep (small Δc) zoom. Decides whether the
//! perturbation MATH is correct (GPU-free, confound-free). Delete after.

type C = (f64, f64);
fn cmul(a: C, b: C) -> C {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}
fn cadd(a: C, b: C) -> C {
    (a.0 + b.0, a.1 + b.1)
}
fn cscalar(s: f64, a: C) -> C {
    (s * a.0, s * a.1)
}
fn mag2(a: C) -> f64 {
    a.0 * a.0 + a.1 * a.1
}

/// Reference orbit at c_ref, Z_0..Z_max (Z_0 = 0). Returns the series + escape idx.
fn ref_orbit(c_ref: C, max_iter: usize) -> (Vec<C>, Option<usize>) {
    let mut z = (0.0, 0.0);
    let mut series = vec![(0.0, 0.0)];
    let mut escaped: Option<usize> = None;
    for i in 0..max_iter {
        // series.back() is Z_i; test it
        if mag2(z) > 4.0 {
            escaped = Some(i);
            break;
        }
        let nr = z.0 * z.0 - z.1 * z.1 + c_ref.0;
        z = (nr, 2.0 * z.0 * z.1 + c_ref.1);
        series.push(z);
    }
    (series, escaped)
}

/// EXACT mirror of fractal.wgsl `mandelbrot_perturb` in f64.
fn perturb(
    orbit: &[C],
    delta_c: C,
    orbit_len: usize,
    ref_escaped_at: u32,
    max_iter: u32,
) -> Option<u32> {
    let mut dz: C = (0.0, 0.0);
    let mut z_full: C = (0.0, 0.0);
    let mut m: usize = 0;
    for i in 0..max_iter {
        if m >= orbit_len {
            dz = z_full;
            m = 0;
        }
        let zref = orbit[m];
        // dz ← 2·Z·dz + dz² + Δc   (both cmul on the OLD dz)
        let t1 = cmul(cscalar(2.0, zref), dz);
        let t2 = cmul(dz, dz);
        dz = cadd(cadd(t1, t2), delta_c);
        m += 1;
        if m >= orbit_len {
            z_full = dz;
        } else {
            z_full = cadd(orbit[m], dz);
        }
        if mag2(z_full) > 4.0 {
            return Some(i);
        }
        let ref_exhausted =
            m >= orbit_len - 1 || (ref_escaped_at > 0 && m as u32 >= ref_escaped_at);
        if mag2(z_full) < mag2(dz) || ref_exhausted {
            dz = z_full;
            m = 0;
        }
    }
    None
}

/// Direct f64 Mandelbrot: first iteration index i where |Z_i|² > 4 (Z_0 = 0).
fn direct(c: C, max_iter: u32) -> Option<u32> {
    let mut z: C = (0.0, 0.0);
    for i in 0..=max_iter {
        if mag2(z) > 4.0 {
            return Some(i);
        }
        z = cadd(cmul(z, z), c);
    }
    None
}

/// f32 mirror of `perturb` — mimics the GPU's actual arithmetic precision.
fn perturb_f32(
    orbit: &[C],
    delta_c: C,
    orbit_len: usize,
    ref_escaped_at: u32,
    max_iter: u32,
) -> Option<u32> {
    let cf = |v: C| (v.0 as f32, v.1 as f32);
    let mut dz = cf((0.0, 0.0));
    let mut z_full = cf((0.0, 0.0));
    let mut m: usize = 0;
    let delta_c = cf(delta_c);
    for i in 0..max_iter {
        if m >= orbit_len {
            dz = z_full;
            m = 0;
        }
        let zref = cf(orbit[m]);
        let cmulf = |a: (f32, f32), b: (f32, f32)| (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0);
        let t1 = cmulf((2.0 * zref.0, 2.0 * zref.1), dz);
        let t2 = cmulf(dz, dz);
        dz = (t1.0 + t2.0 + delta_c.0, t1.1 + t2.1 + delta_c.1);
        m += 1;
        if m >= orbit_len {
            z_full = dz;
        } else {
            let zm = cf(orbit[m]);
            z_full = (zm.0 + dz.0, zm.1 + dz.1);
        }
        if z_full.0 * z_full.0 + z_full.1 * z_full.1 > 4.0 {
            return Some(i);
        }
        let ref_exhausted =
            m >= orbit_len - 1 || (ref_escaped_at > 0 && m as u32 >= ref_escaped_at);
        if z_full.0 * z_full.0 + z_full.1 * z_full.1 < dz.0 * dz.0 + dz.1 * dz.1 || ref_exhausted {
            dz = z_full;
            m = 0;
        }
    }
    None
}

#[test]
fn perturb_matches_direct_across_zooms() {
    let c_ref = (-0.743_643_887_0, 0.131_825_904_2);
    let max_iter = 1000u32;
    let (orbit, esc) = ref_orbit(c_ref, max_iter as usize);
    let orbit_len = orbit.len();
    let ref_escaped_at = esc.map(|i| i as u32).unwrap_or(0);
    eprintln!("ref orbit: len={orbit_len} escaped={esc:?}");

    for &dc_mag in &[2.0_f64, 2e-2, 2e-5, 2e-8] {
        let mut m_f64 = 0usize;
        let mut m_f32 = 0usize;
        let mut checked = 0usize;
        for &(dx, dy) in &[(0.3, 0.1), (-0.2, 0.4), (0.6, -0.5), (-0.05, 0.05)] {
            let delta_c = (dc_mag * dx, dc_mag * dy);
            let c_pixel = cadd(c_ref, delta_c);
            let d = direct(c_pixel, max_iter);
            let p64 = perturb(&orbit, delta_c, orbit_len, ref_escaped_at, max_iter);
            let p32 = perturb_f32(&orbit, delta_c, orbit_len, ref_escaped_at, max_iter);
            checked += 1;
            let agree = |p: Option<u32>, d: Option<u32>| match (p, d) {
                (None, None) => true,
                (Some(a), Some(b)) => (a as i64 - b as i64).abs() <= 1,
                _ => false,
            };
            if !agree(p64, d) {
                m_f64 += 1;
            }
            if !agree(p32, d) {
                m_f32 += 1;
            }
        }
        eprintln!(
            "dc_mag={dc_mag:e}: f64-mismatches={m_f64}/{checked}  f32-mismatches={m_f32}/{checked}"
        );
    }
}

#[test]
fn real_orbit_data_perturbs_correctly() {
    // Feed the REAL orbit (compute_reference_orbit_best — exactly what the GPU
    // uploads) through the verified f32 mirror. If this matches direct f64, the
    // orbit DATA is correct and the GPU bug is in execution, not data.
    use par_fractal::deep_zoom::orbit::compute_reference_orbit_best;

    let c_center = (-0.743_643_887_0, 0.131_825_904_2);
    let zoom = 1e5_f64;
    let half = 2.0 / zoom; // per-axis view half-extent at aspect 1 (matches delta_c_scale)
    let max_iter = 1000u32;

    let orbit = compute_reference_orbit_best(
        par_fractal::deep_zoom::orbit::FractalKind::Mandelbrot,
        c_center.0,
        c_center.1,
        half,
        half,
        [0.0, 0.0],
        max_iter,
        81,
    );
    let orbit_len = orbit.z.len();
    let ref_escaped_at = orbit.escaped_at.unwrap_or(0);
    let c_ref = (
        c_center.0 - orbit.reference_offset[0] as f64,
        c_center.1 - orbit.reference_offset[1] as f64,
    );
    eprintln!(
        "real orbit: len={orbit_len} escaped={:?} c_ref={c_ref:?} offset={:?}",
        orbit.escaped_at, orbit.reference_offset
    );
    let show = orbit.z.len().min(4);
    eprintln!("first Z_n: {:?}", &orbit.z[..show]);

    // Orbit data as f64 pairs for the mirror (mirror casts back to f32 internally).
    let orbit_c: Vec<C> = orbit.z.iter().map(|&[a, b]| (a as f64, b as f64)).collect();

    let mut mism = 0usize;
    for &(dx, dy) in &[
        (0.3, 0.1),
        (-0.2, 0.4),
        (0.6, -0.5),
        (-0.05, 0.05),
        (0.0, 0.0),
    ] {
        // Shader: delta_c = reference_offset + uv*half; c_pixel = c_center + uv*half.
        let delta_c = (
            orbit.reference_offset[0] as f64 + dx * half,
            orbit.reference_offset[1] as f64 + dy * half,
        );
        let c_pixel = (c_center.0 + dx * half, c_center.1 + dy * half);
        let p = perturb_f32(&orbit_c, delta_c, orbit_len, ref_escaped_at, max_iter);
        let d = direct(c_pixel, max_iter);
        let agree = match (p, d) {
            (None, None) => true,
            (Some(a), Some(b)) => (a as i64 - b as i64).abs() <= 1,
            _ => false,
        };
        if !agree {
            mism += 1;
            eprintln!("  MISMATCH uv=({dx},{dy}): perturb={p:?} direct={d:?}");
        }
    }
    eprintln!("real-data mismatches: {mism}/5");
    assert_eq!(
        mism, 0,
        "real orbit data diverges from direct f64 — orbit data is the bug"
    );
}

// ============================================================================
// Phase B step 7 — Julia, Tricorn, Burning Ship delta recurrences.
//
// Each mirror follows the same shape as the Mandelbrot `perturb` above: an
// f64 walk of the per-pixel delta against a reference orbit, plus a direct
// f64 iteration of the same map at the pixel coordinate. The mirror matching
// direct f64 (±1 iteration, like Mandelbrot) is the correctness gate — it
// catches a wrong recurrence BEFORE the GPU shader touches it.
//
// Math (derived in docs/fable/ENH-001-perturbation-deep-zoom.md, step 7):
//
// * Tricorn  z ← conj(z)² + c
//   Reference iterates the tricorn map. Per-pixel Δc is the c-offset from
//   the reference, same as Mandelbrot:
//     dz ← 2·conj(Z)·conj(dz) + conj(dz)² + Δc
//
// * Julia  z ← z² + c, c FIXED = julia_c
//   Per-pixel variation is the STARTING z (z_0 = pixel_coord), not c.
//   Reference iterates from Z_0 = view_center with fixed c = julia_c.
//   Per-pixel dz_0 = pixel_coord − center (the screen offset). c is identical
//   for every pixel, so the Δc term cancels:
//     dz ← 2·Z·dz + dz²      (no Δc term)
//
// * Burning Ship  z ← (|re(z)| + i|im(z)|)² + c
//   Define W_m = |Z_m| per component, σ_m = sign(Z_m) per component (with
//   sign(0) = +1 to match the shader's `select(1.0, -1.0, x < 0.0)`). Away
//   from zero-crossings (the glitch zone — rare off-axis):
//     dz ← 2·W·σ(dz) + σ(dz)² + Δc
//   where σ(dz) = (σ_re·dz.re, σ_im·dz.im).
// ============================================================================

// --- Tricorn ----------------------------------------------------------

fn tricorn_ref_orbit(c_ref: C, max_iter: usize) -> (Vec<C>, Option<usize>) {
    let mut z = (0.0, 0.0);
    let mut series = vec![(0.0, 0.0)];
    let mut escaped: Option<usize> = None;
    for i in 0..max_iter {
        if mag2(z) > 4.0 {
            escaped = Some(i);
            break;
        }
        // conj(z)² + c = (re² − im², −2·re·im) + c
        z = (z.0 * z.0 - z.1 * z.1 + c_ref.0, -2.0 * z.0 * z.1 + c_ref.1);
        series.push(z);
    }
    (series, escaped)
}

fn tricorn_direct(c: C, max_iter: u32) -> Option<u32> {
    let mut z: C = (0.0, 0.0);
    for i in 0..=max_iter {
        if mag2(z) > 4.0 {
            return Some(i);
        }
        z = (z.0 * z.0 - z.1 * z.1 + c.0, -2.0 * z.0 * z.1 + c.1);
    }
    None
}

/// EXACT mirror of the planned `tricorn_perturb` WGSL in f64.
fn tricorn_perturb(
    orbit: &[C],
    delta_c: C,
    orbit_len: usize,
    ref_escaped_at: u32,
    max_iter: u32,
) -> Option<u32> {
    let mut dz: C = (0.0, 0.0);
    let mut z_full: C = (0.0, 0.0);
    let mut m: usize = 0;
    for i in 0..max_iter {
        if m >= orbit_len {
            dz = z_full;
            m = 0;
        }
        let zref = orbit[m];
        // dz ← 2·conj(Z)·conj(dz) + conj(dz)² + Δc
        let conj_z = (zref.0, -zref.1);
        let conj_dz = (dz.0, -dz.1);
        let t1 = cmul(cscalar(2.0, conj_z), conj_dz);
        let t2 = cmul(conj_dz, conj_dz);
        dz = cadd(cadd(t1, t2), delta_c);
        m += 1;
        if m >= orbit_len {
            z_full = dz;
        } else {
            z_full = cadd(orbit[m], dz);
        }
        if mag2(z_full) > 4.0 {
            return Some(i);
        }
        let ref_exhausted =
            m >= orbit_len - 1 || (ref_escaped_at > 0 && m as u32 >= ref_escaped_at);
        if mag2(z_full) < mag2(dz) || ref_exhausted {
            dz = z_full;
            m = 0;
        }
    }
    None
}

// --- Julia ------------------------------------------------------------

fn julia_ref_orbit(z0: C, c: C, max_iter: usize) -> (Vec<C>, Option<usize>) {
    let mut z = z0;
    let mut series = vec![z0];
    let mut escaped: Option<usize> = None;
    for i in 0..max_iter {
        if mag2(z) > 4.0 {
            escaped = Some(i);
            break;
        }
        z = cadd(cmul(z, z), c);
        series.push(z);
    }
    (series, escaped)
}

fn julia_direct(z0: C, c: C, max_iter: u32) -> Option<u32> {
    let mut z = z0;
    for i in 0..=max_iter {
        if mag2(z) > 4.0 {
            return Some(i);
        }
        z = cadd(cmul(z, z), c);
    }
    None
}

/// EXACT mirror of the planned `julia_perturb` WGSL in f64.
/// dz_0 = pixel_offset from the reference's starting Z_0. No Δc term — c is
/// identical for every pixel, so it cancels in the difference.
fn julia_perturb(
    orbit: &[C],
    dz0: C,
    orbit_len: usize,
    ref_escaped_at: u32,
    max_iter: u32,
) -> Option<u32> {
    let mut dz: C = dz0;
    let mut z_full: C = (0.0, 0.0);
    let mut m: usize = 0;
    for i in 0..max_iter {
        if m >= orbit_len {
            dz = z_full;
            m = 0;
        }
        let zref = orbit[m];
        // dz ← 2·Z·dz + dz²   (NO Δc term)
        let t1 = cmul(cscalar(2.0, zref), dz);
        let t2 = cmul(dz, dz);
        dz = cadd(t1, t2);
        m += 1;
        if m >= orbit_len {
            z_full = dz;
        } else {
            z_full = cadd(orbit[m], dz);
        }
        if mag2(z_full) > 4.0 {
            return Some(i);
        }
        let ref_exhausted =
            m >= orbit_len - 1 || (ref_escaped_at > 0 && m as u32 >= ref_escaped_at);
        if mag2(z_full) < mag2(dz) || ref_exhausted {
            dz = z_full;
            m = 0;
        }
    }
    None
}

// --- Burning Ship -----------------------------------------------------

fn burning_ship_ref_orbit(c_ref: C, max_iter: usize) -> (Vec<C>, Option<usize>) {
    let mut z = (0.0, 0.0);
    let mut series = vec![(0.0, 0.0)];
    let mut escaped: Option<usize> = None;
    for i in 0..max_iter {
        if mag2(z) > 4.0 {
            escaped = Some(i);
            break;
        }
        // (|re| + i|im|)² = (re² − im², 2·|re|·|im|); the imaginary part is
        // always non-negative because |re|, |im| ≥ 0.
        let ar = z.0.abs();
        let ai = z.1.abs();
        z = (ar * ar - ai * ai + c_ref.0, 2.0 * ar * ai + c_ref.1);
        series.push(z);
    }
    (series, escaped)
}

fn burning_ship_direct(c: C, max_iter: u32) -> Option<u32> {
    let mut z: C = (0.0, 0.0);
    for i in 0..=max_iter {
        if mag2(z) > 4.0 {
            return Some(i);
        }
        let ar = z.0.abs();
        let ai = z.1.abs();
        z = (ar * ar - ai * ai + c.0, 2.0 * ar * ai + c.1);
    }
    None
}

/// EXACT mirror of the planned `burning_ship_perturb` WGSL in f64.
/// Per-component abs signs come from the reference orbit Z (assumed not to
/// cross zero on either axis — the glitch zone; rare off-axis).
fn burning_ship_perturb(
    orbit: &[C],
    delta_c: C,
    orbit_len: usize,
    ref_escaped_at: u32,
    max_iter: u32,
) -> Option<u32> {
    let mut dz: C = (0.0, 0.0);
    let mut z_full: C = (0.0, 0.0);
    let mut m: usize = 0;
    for i in 0..max_iter {
        if m >= orbit_len {
            dz = z_full;
            m = 0;
        }
        let zref = orbit[m];
        // W = |Z| per component, σ = sign(Z) per component (sign(0) = +1, to
        // match the shader's `select(1.0, -1.0, x < 0.0)`).
        let w = (zref.0.abs(), zref.1.abs());
        let sgn = (
            if zref.0 < 0.0 { -1.0 } else { 1.0 },
            if zref.1 < 0.0 { -1.0 } else { 1.0 },
        );
        let sigma_dz = (sgn.0 * dz.0, sgn.1 * dz.1);
        // dz ← 2·W·σ(dz) + σ(dz)² + Δc
        let t1 = cmul(cscalar(2.0, w), sigma_dz);
        let t2 = cmul(sigma_dz, sigma_dz);
        dz = cadd(cadd(t1, t2), delta_c);
        m += 1;
        if m >= orbit_len {
            z_full = dz;
        } else {
            z_full = cadd(orbit[m], dz);
        }
        if mag2(z_full) > 4.0 {
            return Some(i);
        }
        let ref_exhausted =
            m >= orbit_len - 1 || (ref_escaped_at > 0 && m as u32 >= ref_escaped_at);
        if mag2(z_full) < mag2(dz) || ref_exhausted {
            dz = z_full;
            m = 0;
        }
    }
    None
}

// --- Phase B gate tests ----------------------------------------------

// All three gates follow the same shape: iterate Δc across magnitudes from
// shallow (informational; perturbation is allowed to degrade — its valid
// regime is `log2(zoom) > 24` ⇒ |Δc| < ~1.2e-7) to deep (the actual
// activation regime; must match direct f64 to ±1 iteration). The
// `dc_mag ≤ PERTURB_VALID_MAX` cases are hard-asserted; larger magnitudes
// are logged only. This mirrors how the Mandelbrot path was verified.
const PERTURB_VALID_MAX: f64 = 2e-5;

#[test]
fn tricorn_perturb_matches_direct() {
    let c_ref = (-0.5_f64, 0.0_f64);
    let max_iter = 1000u32;
    let (orbit, esc) = tricorn_ref_orbit(c_ref, max_iter as usize);
    let orbit_len = orbit.len();
    let ref_escaped_at = esc.map(|i| i as u32).unwrap_or(0);
    eprintln!("tricorn orbit: len={orbit_len} escaped={esc:?}");

    let mut deep_mism = 0usize;
    for &dc_mag in &[2.0_f64, 2e-2, 2e-5, 2e-8] {
        let mut mism = 0usize;
        let mut checked = 0usize;
        for &(dx, dy) in &[(0.3, 0.1), (-0.2, 0.4), (0.6, -0.5), (-0.05, 0.05)] {
            let delta_c = (dc_mag * dx, dc_mag * dy);
            let c_pixel = cadd(c_ref, delta_c);
            let d = tricorn_direct(c_pixel, max_iter);
            let p = tricorn_perturb(&orbit, delta_c, orbit_len, ref_escaped_at, max_iter);
            checked += 1;
            let agree = match (p, d) {
                (None, None) => true,
                (Some(a), Some(b)) => (a as i64 - b as i64).abs() <= 1,
                _ => false,
            };
            if !agree {
                mism += 1;
                eprintln!(
                    "  TRICORN MISMATCH dc_mag={dc_mag:e} uv=({dx},{dy}): perturb={p:?} direct={d:?}"
                );
            }
        }
        eprintln!("tricorn dc_mag={dc_mag:e}: mismatches={mism}/{checked}");
        if dc_mag <= PERTURB_VALID_MAX {
            deep_mism += mism;
        }
    }
    assert_eq!(
        deep_mism, 0,
        "tricorn delta recurrence diverges from direct f64 in the activation regime"
    );
}

#[test]
fn julia_perturb_matches_direct() {
    let julia_c = (-0.7_f64, 0.27015_f64);
    let center = (-0.5_f64, 0.0_f64); // reference Z_0
    let max_iter = 1000u32;
    let (orbit, esc) = julia_ref_orbit(center, julia_c, max_iter as usize);
    let orbit_len = orbit.len();
    let ref_escaped_at = esc.map(|i| i as u32).unwrap_or(0);
    eprintln!("julia orbit: len={orbit_len} escaped={esc:?}");

    let mut deep_mism = 0usize;
    for &dc_mag in &[2.0_f64, 2e-2, 2e-5, 2e-8] {
        let mut mism = 0usize;
        let mut checked = 0usize;
        for &(dx, dy) in &[
            (0.3, 0.1),
            (-0.2, 0.4),
            (0.6, -0.5),
            (-0.05, 0.05),
            (0.0, 0.0),
        ] {
            // dz_0 = pixel_offset from center (the reference Z_0).
            let dz0 = (dc_mag * dx, dc_mag * dy);
            let z0_pixel = cadd(center, dz0);
            let d = julia_direct(z0_pixel, julia_c, max_iter);
            let p = julia_perturb(&orbit, dz0, orbit_len, ref_escaped_at, max_iter);
            checked += 1;
            let agree = match (p, d) {
                (None, None) => true,
                (Some(a), Some(b)) => (a as i64 - b as i64).abs() <= 1,
                _ => false,
            };
            if !agree {
                mism += 1;
                eprintln!(
                    "  JULIA MISMATCH dc_mag={dc_mag:e} uv=({dx},{dy}): perturb={p:?} direct={d:?}"
                );
            }
        }
        eprintln!("julia dc_mag={dc_mag:e}: mismatches={mism}/{checked}");
        if dc_mag <= PERTURB_VALID_MAX {
            deep_mism += mism;
        }
    }
    assert_eq!(
        deep_mism, 0,
        "julia delta recurrence diverges from direct f64 in the activation regime"
    );
}

#[test]
fn burning_ship_perturb_matches_direct() {
    let c_ref = (-0.5_f64, -0.5_f64);
    let max_iter = 1000u32;
    let (orbit, esc) = burning_ship_ref_orbit(c_ref, max_iter as usize);
    let orbit_len = orbit.len();
    let ref_escaped_at = esc.map(|i| i as u32).unwrap_or(0);
    eprintln!("burning ship orbit: len={orbit_len} escaped={esc:?}");

    let mut deep_mism = 0usize;
    for &dc_mag in &[2.0_f64, 2e-2, 2e-5, 2e-8] {
        let mut mism = 0usize;
        let mut checked = 0usize;
        for &(dx, dy) in &[(0.3, 0.1), (-0.2, 0.4), (0.6, -0.5), (-0.05, 0.05)] {
            let delta_c = (dc_mag * dx, dc_mag * dy);
            let c_pixel = cadd(c_ref, delta_c);
            let d = burning_ship_direct(c_pixel, max_iter);
            let p = burning_ship_perturb(&orbit, delta_c, orbit_len, ref_escaped_at, max_iter);
            checked += 1;
            let agree = match (p, d) {
                (None, None) => true,
                (Some(a), Some(b)) => (a as i64 - b as i64).abs() <= 1,
                _ => false,
            };
            if !agree {
                mism += 1;
                eprintln!(
                    "  BURNING SHIP MISMATCH dc_mag={dc_mag:e} uv=({dx},{dy}): perturb={p:?} direct={d:?}"
                );
            }
        }
        eprintln!("burning ship dc_mag={dc_mag:e}: mismatches={mism}/{checked}");
        if dc_mag <= PERTURB_VALID_MAX {
            deep_mism += mism;
        }
    }
    assert_eq!(
        deep_mism, 0,
        "burning ship delta recurrence diverges from direct f64 in the activation regime"
    );
}
