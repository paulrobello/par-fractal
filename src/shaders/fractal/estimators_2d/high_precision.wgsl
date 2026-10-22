// High-precision Mandelbrot
fn mandelbrot_hp(c_hi: vec2<f32>, c_lo: vec2<f32>) -> f32 {
    var z = df2(vec2<f32>(0.0, 0.0), vec2<f32>(0.0, 0.0));
    let c = df2(c_hi, c_lo);
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (df2_mag_sq(z) > 4.0) {
            break;
        }

        // z = z^2 + c
        z = df2_add(df2_square(z), c);
        iteration = i;
    }

    if (iteration >= max(uniforms.max_iterations, 1u) - 1u) {
        return -1.0;
    }

    // Smooth coloring using high part only (sufficient precision)
    let mag_sq = df2_mag_sq(z);
    return smooth_iteration_count(iteration, mag_sq, 2.0, 2.0, uniforms.max_iterations);
}

// High-precision Julia
fn julia_hp(z_hi: vec2<f32>, z_lo: vec2<f32>) -> f32 {
    var z = df2(z_hi, z_lo);
    // Julia C parameter - use standard precision (it's a constant, not zoomed)
    let c = df2(uniforms.julia_c, vec2<f32>(0.0, 0.0));
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (df2_mag_sq(z) > 4.0) {
            break;
        }

        z = df2_add(df2_square(z), c);
        iteration = i;
    }

    if (iteration >= max(uniforms.max_iterations, 1u) - 1u) {
        return -1.0;
    }

    let mag_sq = df2_mag_sq(z);
    return smooth_iteration_count(iteration, mag_sq, 2.0, 2.0, uniforms.max_iterations);
}

// High-precision Burning Ship
fn burning_ship_hp(c_hi: vec2<f32>, c_lo: vec2<f32>) -> f32 {
    var z = df2(vec2<f32>(0.0, 0.0), vec2<f32>(0.0, 0.0));
    let c = df2(c_hi, c_lo);
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (df2_mag_sq(z) > 4.0) {
            break;
        }

        // Burning Ship: z = (|Re(z)| + i|Im(z)|)^2 + c
        // DF abs: when the hi word of a component is negative, BOTH words must be
        // negated. Using `select` (rather than `sign(hi)`) avoids `sign(0.0) == 0.0`
        // silently zeroing the lo word when hi == 0. (QA-002)
        let neg = select(vec2<f32>(1.0), vec2<f32>(-1.0), z.hi < vec2<f32>(0.0));
        let z_abs = df2(abs(z.hi), z.lo * neg);
        z = df2_add(df2_square(z_abs), c);
        iteration = i;
    }

    if (iteration >= max(uniforms.max_iterations, 1u) - 1u) {
        return -1.0;
    }

    let mag_sq = df2_mag_sq(z);
    return smooth_iteration_count(iteration, mag_sq, 2.0, 2.0, uniforms.max_iterations);
}

// High-precision Tricorn
fn tricorn_hp(c_hi: vec2<f32>, c_lo: vec2<f32>) -> f32 {
    var z = df2(vec2<f32>(0.0, 0.0), vec2<f32>(0.0, 0.0));
    let c = df2(c_hi, c_lo);
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (df2_mag_sq(z) > 4.0) {
            break;
        }

        // Tricorn: z = conj(z)^2 + c
        // conj(z) = (re, -im)
        let z_conj = df2(vec2<f32>(z.hi.x, -z.hi.y), vec2<f32>(z.lo.x, -z.lo.y));
        z = df2_add(df2_square(z_conj), c);
        iteration = i;
    }

    if (iteration >= max(uniforms.max_iterations, 1u) - 1u) {
        return -1.0;
    }

    let mag_sq = df2_mag_sq(z);
    return smooth_iteration_count(iteration, mag_sq, 2.0, 2.0, uniforms.max_iterations);
}
