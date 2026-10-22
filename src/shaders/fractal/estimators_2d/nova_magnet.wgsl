// Nova fractal - Newton-Mandelbrot hybrid
fn nova_fractal(c: vec2<f32>) -> f32 {
    var z = c;
    var iteration = 0u;
    let relaxation = 1.0;  // Nova parameter

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > 4.0) {
            break;
        }

        // Nova: z = z - R * f(z)/f'(z) + c where f(z) = z^3 - 1
        let z2 = vec2<f32>(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y);
        let z3 = vec2<f32>(z2.x * z.x - z2.y * z.y, z2.x * z.y + z2.y * z.x);
        let f_z = vec2<f32>(z3.x - 1.0, z3.y);
        let f_prime = vec2<f32>(3.0 * z2.x, 3.0 * z2.y);

        let denom = dot(f_prime, f_prime);
        if (denom < 0.0001) { break; }

        let div = vec2<f32>(
            (f_z.x * f_prime.x + f_z.y * f_prime.y) / denom,
            (f_z.y * f_prime.x - f_z.x * f_prime.y) / denom
        );

        // Nova modification: add c after Newton step
        z = z - relaxation * div + vec2<f32>(uniforms.julia_c.x, uniforms.julia_c.y);
        iteration = i;
    }

    if (iteration >= max(uniforms.max_iterations, 1u) - 1u) {
        return -1.0;
    }

    // Smooth coloring — defensive max(... , eps) clamp preserved (unreachable
    // under the >4 bailout but kept verbatim). Not consolidated into
    // smooth_iteration_count because that would silently drop the clamp.
    let log_zn = log(dot(z, z)) / 2.0;
    let nu = log(max(log_zn / log(2.0), 0.0001)) / log(2.0);
    return (f32(iteration) + 1.0 - nu) / f32(uniforms.max_iterations);
}

// Magnet Type 1 fractal - z = ((z² + c - 1) / (2z + c - 2))²
fn magnet_fractal(c: vec2<f32>) -> f32 {
    var z = vec2<f32>(0.0, 0.0);
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > 10000.0) {
            break;
        }

        // Check for convergence to fixed point at z = 1
        let dist_to_one = distance(z, vec2<f32>(1.0, 0.0));
        if (dist_to_one < 0.0001) {
            return -1.0;  // Converged to attractor (sentinel: render as interior)
        }

        // z² = (z.x + iz.y)² = z.x² - z.y² + 2i*z.x*z.y
        let z2 = vec2<f32>(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y);

        // Numerator: z² + c - 1
        let num = vec2<f32>(z2.x + c.x - 1.0, z2.y + c.y);

        // Denominator: 2z + c - 2
        let den = vec2<f32>(2.0 * z.x + c.x - 2.0, 2.0 * z.y + c.y);

        // Complex division: num / den
        let den_mag_sq = dot(den, den);
        if (den_mag_sq < 0.0001) { break; }

        let div = vec2<f32>(
            (num.x * den.x + num.y * den.y) / den_mag_sq,
            (num.y * den.x - num.x * den.y) / den_mag_sq
        );

        // Square the result
        z = vec2<f32>(div.x * div.x - div.y * div.y, 2.0 * div.x * div.y);
        iteration = i;
    }

    if (iteration >= max(uniforms.max_iterations, 1u) - 1u) {
        return -1.0;
    }

    // Smooth coloring — defensive clamps preserved (unreachable under the
    // >10000 bailout but kept verbatim). Not consolidated into
    // smooth_iteration_count because that would silently drop the clamps.
    let log_zn = log(max(dot(z, z), 1.0)) / 2.0;
    let nu = log(max(log_zn / log(2.0), 0.0001)) / log(2.0);
    return (f32(iteration) + 1.0 - nu) / f32(uniforms.max_iterations);
}
