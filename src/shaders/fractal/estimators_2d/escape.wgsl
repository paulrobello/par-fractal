// ============================================================================
// 2D Fractals (Standard Precision)
// ============================================================================

// Complex power function: z^n using polar form
// z = r * e^(i*theta), z^n = r^n * e^(i*n*theta)
fn complex_pow(z: vec2<f32>, n: f32) -> vec2<f32> {
    let r = length(z);
    // Handle zero case
    if (r < 0.0000001) {
        return vec2<f32>(0.0, 0.0);
    }
    let theta = atan2(z.y, z.x);
    let r_n = pow(r, n);
    let n_theta = n * theta;
    return vec2<f32>(r_n * cos(n_theta), r_n * sin(n_theta));
}

fn mandelbrot(c: vec2<f32>) -> f32 {
    var z = vec2<f32>(0.0, 0.0);
    var iteration = 0u;
    let n = uniforms.power;
    let escape_radius = select(4.0, pow(2.0, 2.0 / abs(n)), abs(n) < 2.0);

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > escape_radius * escape_radius) {
            break;
        }

        // z = z^n + c using complex power
        z = complex_pow(z, n) + c;
        iteration = i;
    }

    if (iteration >= max(uniforms.max_iterations, 1u) - 1u) {
        return -1.0;
    }

    // Smooth coloring (adjusted for variable power)
    return smooth_iteration_count(iteration, dot(z, z), escape_radius, n, uniforms.max_iterations);
}

fn julia(z_start: vec2<f32>) -> f32 {
    var z = z_start;
    var iteration = 0u;
    let n = uniforms.power;
    let escape_radius = select(4.0, pow(2.0, 2.0 / abs(n)), abs(n) < 2.0);

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > escape_radius * escape_radius) {
            break;
        }

        // z = z^n + c using complex power
        z = complex_pow(z, n) + uniforms.julia_c;
        iteration = i;
    }

    if (iteration >= max(uniforms.max_iterations, 1u) - 1u) {
        return -1.0;
    }

    // Smooth coloring (adjusted for variable power)
    return smooth_iteration_count(iteration, dot(z, z), escape_radius, n, uniforms.max_iterations);
}
