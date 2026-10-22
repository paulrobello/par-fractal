// Newton fractal - finds roots of z^3 - 1
fn newton_fractal(c: vec2<f32>) -> f32 {
    var z = c;
    var iteration = 0u;
    let tolerance = 0.000001;

    // Roots of z^3 - 1
    let root1 = vec2<f32>(1.0, 0.0);
    let root2 = vec2<f32>(-0.5, 0.866025);
    let root3 = vec2<f32>(-0.5, -0.866025);

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        // Check convergence to roots
        if (distance(z, root1) < tolerance ||
            distance(z, root2) < tolerance ||
            distance(z, root3) < tolerance) {
            break;
        }

        // Newton iteration: z = z - f(z)/f'(z) where f(z) = z^3 - 1
        // f'(z) = 3z^2
        let z2 = vec2<f32>(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y);
        let z3 = vec2<f32>(z2.x * z.x - z2.y * z.y, z2.x * z.y + z2.y * z.x);
        let f_z = vec2<f32>(z3.x - 1.0, z3.y);
        let f_prime = vec2<f32>(3.0 * z2.x, 3.0 * z2.y);

        let denom = dot(f_prime, f_prime);
        if (denom < 0.0001) { break; }

        // Complex division: f_z / f_prime
        let div = vec2<f32>(
            (f_z.x * f_prime.x + f_z.y * f_prime.y) / denom,
            (f_z.y * f_prime.x - f_z.x * f_prime.y) / denom
        );
        z = z - div;
        iteration = i;
    }

    // Color based on which root and iteration count
    let d1 = distance(z, root1);
    let d2 = distance(z, root2);
    let d3 = distance(z, root3);
    var root_offset = 0.0;
    if (d2 < d1 && d2 < d3) { root_offset = 0.33; }
    else if (d3 < d1 && d3 < d2) { root_offset = 0.66; }

    return (f32(iteration) / f32(uniforms.max_iterations)) * 0.5 + root_offset * 0.5;
}

// Lyapunov fractal - stability diagram
fn lyapunov_fractal(c: vec2<f32>) -> f32 {
    let a = c.x * 2.0 + 2.0;  // Map to [0, 4] range
    let b = c.y * 2.0 + 2.0;

    var x = 0.5;
    var lyap = 0.0;
    let sequence_len = 12u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        // Alternate between a and b based on sequence position
        var r: f32;
        if ((i % sequence_len) < (sequence_len / 2u)) {
            r = a;
        } else {
            r = b;
        }

        // Logistic map: x = r * x * (1 - x)
        x = r * x * (1.0 - x);

        // Accumulate Lyapunov exponent
        let deriv = abs(r * (1.0 - 2.0 * x));
        if (deriv > 0.0001) {
            lyap = lyap + log(deriv);
        }
    }

    lyap = lyap / f32(uniforms.max_iterations);

    // Map Lyapunov exponent to color
    if (lyap < 0.0) {
        return 0.5 - clamp(-lyap * 0.5, 0.0, 0.5);  // Stable (convergent)
    } else {
        return 0.5 + clamp(lyap * 0.5, 0.0, 0.5);   // Chaotic
    }
}
