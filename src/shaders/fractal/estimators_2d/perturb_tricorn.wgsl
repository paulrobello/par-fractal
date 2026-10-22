// Perturbation-theory Tricorn (ENH-001 Phase B step 7).
//
// Tricorn map: z ← conj(z)² + c. Per-pixel delta recurrence (derived in
// docs/fable/ENH-001-perturbation-deep-zoom.md step 7): given z = Z + dz and
// c = c_ref + Δc,
//
//     conj(z)² = (conj(Z) + conj(dz))²
//              = conj(Z)² + 2·conj(Z)·conj(dz) + conj(dz)²
//
// so z_{n+1} = conj(Z)² + c + (2·conj(Z)·conj(dz) + conj(dz)²), and since
// Z_{n+1} = conj(Z)² + c_ref,
//
//     dz ← 2·conj(Z)·conj(dz) + conj(dz)² + Δc
//
// The reference orbit iterates the tricorn map (CPU-side
// `compute_reference_orbit(FractalKind::Tricorn, …)`); Δc is the same
// Mandelbrot-style per-pixel c-offset. Structure mirrors
// `mandelbrot_perturb` exactly; only the recurrence step differs.
fn tricorn_perturb(delta_c: vec2<f32>) -> f32 {
    var dz = vec2<f32>(0.0, 0.0);
    var z_full = vec2<f32>(0.0, 0.0);
    var m = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (m >= uniforms.orbit_len) {
            dz = z_full;
            m = 0u;
        }

        // dz ← 2·conj(Z)·conj(dz) + conj(dz)² + Δc
        let zref = ref_orbit[m];
        let conj_z = vec2<f32>(zref.x, -zref.y);
        let conj_dz = vec2<f32>(dz.x, -dz.y);
        dz = cmul(2.0 * conj_z, conj_dz) + cmul(conj_dz, conj_dz) + delta_c;
        m = m + 1u;

        if (m >= uniforms.orbit_len) {
            z_full = dz;
        } else {
            z_full = ref_orbit[m] + dz;
        }

        let mag2 = dot(z_full, z_full);
        if (mag2 > 4.0) {
            return smooth_iteration_count(i, mag2, 2.0, 2.0, uniforms.max_iterations);
        }

        let ref_exhausted = (m >= uniforms.orbit_len - 1u) ||
                            (uniforms.ref_escaped_at > 0u && m >= uniforms.ref_escaped_at);
        if (mag2 < dot(dz, dz) || ref_exhausted) {
            dz = z_full;
            m = 0u;
        }
    }

    return -1.0;
}
