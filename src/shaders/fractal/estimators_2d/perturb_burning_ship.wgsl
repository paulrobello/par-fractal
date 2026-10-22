// Perturbation-theory Burning Ship (ENH-001 Phase B step 7).
//
// Burning Ship map: z ← (|re(z)| + i|im(z)|)² + c. Define
// W_m = (|Z_m.re|, |Z_m.im|) and σ_m = sign per component (sign(0) = +1
// via `select(1.0, -1.0, x < 0.0)`, matching the DF abs fix QA-002). When
// Z + dz does not cross zero on either axis (the glitch zone — rare
// off-axis), |re(Z+dz)| = |re(Z)| + σ_re·dz.re, and similarly for im, so
// w = (|re(z)|, |im(z)|) = W + σ(dz) where σ(dz) = σ·dz per-component.
// Then w² + c = W² + 2·W·σ(dz) + σ(dz)² + c, and since
// Z_{n+1} = W² + c_ref:
//
//     dz ← 2·W·σ(dz) + σ(dz)² + Δc
//
// The reference orbit iterates the burning-ship map on the CPU
// (`compute_reference_orbit(FractalKind::BurningShip, …)`); W and σ are
// derived per-iteration from the orbit's Z_m.
fn burning_ship_perturb(delta_c: vec2<f32>) -> f32 {
    var dz = vec2<f32>(0.0, 0.0);
    var z_full = vec2<f32>(0.0, 0.0);
    var m = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (m >= uniforms.orbit_len) {
            dz = z_full;
            m = 0u;
        }

        // W = |Z| per component, σ = sign(Z) per component (sign(0) = +1).
        let zref = ref_orbit[m];
        let w = abs(zref);
        let sgn = select(vec2<f32>(1.0), vec2<f32>(-1.0), zref < vec2<f32>(0.0));
        let sigma_dz = sgn * dz;
        // dz ← 2·W·σ(dz) + σ(dz)² + Δc
        dz = cmul(2.0 * w, sigma_dz) + cmul(sigma_dz, sigma_dz) + delta_c;
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
