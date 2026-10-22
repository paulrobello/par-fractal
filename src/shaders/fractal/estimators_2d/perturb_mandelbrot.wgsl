// Complex multiply: (a.x*b.x - a.y*b.y, a.x*b.y + a.y*b.x). f32 only — used by
// the perturbation delta recurrence, where both operands are small by design.
fn cmul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

// Perturbation-theory Mandelbrot (ENH-001 Phase A step 4).
//
// Iterates the per-pixel delta `Δz` against the precomputed reference orbit
// `Z_n` (uploaded to `ref_orbit` by the CPU driver), using the Zhuoran
// "single-reference rebasing" formulation:
//
//     Δz ← 2·Z_m·Δz + Δz² + Δc
//
// The full pixel value is `z_full = Z_m + Δz`; escape and coloring use that.
// When the reference is exhausted (`m` reaches `orbit_len` or `ref_escaped_at`)
// or the Pauldelbrot glitch test fires (`|z_full|² < |Δz|²`, meaning the delta
// has grown larger than the full value — a tell-tale of precision loss), the
// delta is re-anchored to the current full value and the reference index
// restarts from 0. This is an approximation (a fresh reference would be ideal),
// but it is robust enough that one reference per view covers the vast majority
// of deep-zoom compositions; Phase B may add proper glitch re-rendering.
//
// Bailout `|z|² > 4`, the interior sentinel `-1.0`, and the call signature of
// `smooth_iteration_count` all match `mandelbrot_hp` so coloring is consistent
// across the DF/perturbation boundary. Mandelbrot-only in Phase A
// (`fractal_type == 0u` gate in `fs_main_2d`).
fn mandelbrot_perturb(delta_c: vec2<f32>) -> f32 {
    var dz = vec2<f32>(0.0, 0.0);
    var z_full = vec2<f32>(0.0, 0.0);
    var m = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        // Defensive top-of-loop bounds check. The bottom glitch guard should
        // keep m < orbit_len at every iteration exit, but if a future change
        // violates that invariant, re-anchor against the last known full value
        // rather than read past the populated region of ref_orbit.
        if (m >= uniforms.orbit_len) {
            dz = z_full;
            m = 0u;
        }

        // Δz ← 2·Z_m·Δz + Δz² + Δc
        let zref = ref_orbit[m];
        dz = cmul(2.0 * zref, dz) + cmul(dz, dz) + delta_c;
        m = m + 1u;

        // Bounds-check the post-increment reference index before reading
        // Z_{m+1}. The buffer is over-allocated; use the populated
        // `orbit_len`, NOT `arrayLength(&ref_orbit)` (ENH-001 pitfall #1).
        // When the reference is exhausted, drop its contribution and carry
        // dz forward as z_full — after the next rebase to m=0, Z_0 = 0, so
        // dz alone correctly represents the full value.
        if (m >= uniforms.orbit_len) {
            z_full = dz;
        } else {
            z_full = ref_orbit[m] + dz;
        }

        let mag2 = dot(z_full, z_full);

        if (mag2 > 4.0) {
            return smooth_iteration_count(i, mag2, 2.0, 2.0, uniforms.max_iterations);
        }

        // Pauldelbrot glitch guard + reference exhaustion. `ref_escaped_at`
        // is 0 when the reference stayed bounded, so the second clause only
        // fires when a real escape index was recorded.
        let ref_exhausted = (m >= uniforms.orbit_len - 1u) ||
                            (uniforms.ref_escaped_at > 0u && m >= uniforms.ref_escaped_at);
        if (mag2 < dot(dz, dz) || ref_exhausted) {
            dz = z_full;
            m = 0u;
        }
    }

    // Loop exhausted without escape → interior sentinel (matches mandelbrot_hp
    // and QA-023: fs_main_2d maps t < 0.0 to black).
    return -1.0;
}
