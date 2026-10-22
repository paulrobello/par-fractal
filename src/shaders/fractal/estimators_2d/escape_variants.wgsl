fn burning_ship(c: vec2<f32>) -> f32 {
    var z = vec2<f32>(0.0, 0.0);
    var iteration = 0u;
    let n = uniforms.power;
    let escape_radius = select(4.0, pow(2.0, 2.0 / abs(n)), abs(n) < 2.0);

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > escape_radius * escape_radius) {
            break;
        }

        // Burning Ship: z = (|Re(z)| + i|Im(z)|)^n + c
        let z_abs = abs(z);
        z = complex_pow(z_abs, n) + c;
        iteration = i;
    }

    if (iteration >= max(uniforms.max_iterations, 1u) - 1u) {
        return -1.0;
    }

    return smooth_iteration_count(iteration, dot(z, z), escape_radius, n, uniforms.max_iterations);
}

fn tricorn(c: vec2<f32>) -> f32 {
    var z = vec2<f32>(0.0, 0.0);
    var iteration = 0u;
    let n = uniforms.power;
    let escape_radius = select(4.0, pow(2.0, 2.0 / abs(n)), abs(n) < 2.0);

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > escape_radius * escape_radius) {
            break;
        }

        // Tricorn (Mandelbar/Multicorn): z = conj(z)^n + c
        let z_conj = vec2<f32>(z.x, -z.y);
        z = complex_pow(z_conj, n) + c;
        iteration = i;
    }

    if (iteration >= max(uniforms.max_iterations, 1u) - 1u) {
        return -1.0;
    }

    return smooth_iteration_count(iteration, dot(z, z), escape_radius, n, uniforms.max_iterations);
}

fn phoenix(c: vec2<f32>) -> f32 {
    var z = vec2<f32>(0.0, 0.0);
    var z_prev = vec2<f32>(0.0, 0.0);
    var iteration = 0u;
    let n = uniforms.power;
    let escape_radius = select(4.0, pow(2.0, 2.0 / abs(n)), abs(n) < 2.0);

    // Phoenix parameters
    let p = vec2<f32>(0.5667, 0.0);

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > escape_radius * escape_radius) {
            break;
        }

        // Phoenix: z = z^n + c + p*z_prev
        let z_powered = complex_pow(z, n);
        // Complex multiplication for p * z_prev
        let p_times_prev = vec2<f32>(
            p.x * z_prev.x - p.y * z_prev.y,
            p.x * z_prev.y + p.y * z_prev.x
        );
        z_prev = z;
        z = z_powered + c + p_times_prev;
        iteration = i;
    }

    if (iteration >= max(uniforms.max_iterations, 1u) - 1u) {
        return -1.0;
    }

    return smooth_iteration_count(iteration, dot(z, z), escape_radius, n, uniforms.max_iterations);
}

fn celtic(c: vec2<f32>) -> f32 {
    var z = vec2<f32>(0.0, 0.0);
    var iteration = 0u;
    let n = uniforms.power;
    let escape_radius = select(4.0, pow(2.0, 2.0 / abs(n)), abs(n) < 2.0);

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > escape_radius * escape_radius) {
            break;
        }

        // Celtic: z = (|Re(z^n)| + i*Im(z^n)) + c
        let z_pow = complex_pow(z, n);
        z = vec2<f32>(abs(z_pow.x) + c.x, z_pow.y + c.y);
        iteration = i;
    }

    if (iteration >= max(uniforms.max_iterations, 1u) - 1u) {
        return -1.0;
    }

    return smooth_iteration_count(iteration, dot(z, z), escape_radius, n, uniforms.max_iterations);
}
