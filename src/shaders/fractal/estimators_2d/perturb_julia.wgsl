// Perturbation-theory Julia (ENH-001 Phase B step 7).
//
// Julia map: z ← z² + c with c FIXED (uniforms.julia_c). The per-pixel
// variation is the STARTING z (z_0 = pixel_coord), not c. The reference
// orbit iterates from Z_0 = view_center with fixed c = julia_c
// (`compute_reference_orbit(FractalKind::Julia, …)` inverts (c, Z_0) vs the
// Mandelbrot path). Per-pixel dz_0 = z_0_pixel − Z_0 (the screen offset,
// reconstructed by the caller and passed in as `initial_dz`). c is identical
// for every pixel, so the Δc term cancels:
//
//     dz ← 2·Z·dz + dz²     (no Δc term)
//
// `initial_dz` is computed in `fs_main_2d` from the same `delta_c_origin +
// uv·delta_c_scale` mapping the Mandelbrot path uses for Δc — same screen
// geometry, different per-pixel variable.
fn julia_perturb(initial_dz: vec2<f32>) -> f32 {
    var dz = initial_dz;
    var z_full = vec2<f32>(0.0, 0.0);
    var m = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (m >= uniforms.orbit_len) {
            dz = z_full;
            m = 0u;
        }

        // dz ← 2·Z·dz + dz²   (NO Δc term — c is identical for every pixel)
        let zref = ref_orbit[m];
        dz = cmul(2.0 * zref, dz) + cmul(dz, dz);
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
