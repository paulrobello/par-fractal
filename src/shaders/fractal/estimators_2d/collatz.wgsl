// Collatz fractal - based on Collatz conjecture generalized to complex numbers
fn collatz_fractal(c: vec2<f32>) -> f32 {
    var z = c;
    var iteration = 0u;
    var min_dist = 1000.0;  // Track minimum distance to origin for coloring

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let mag_sq = dot(z, z);
        if (mag_sq > 10000.0) {
            // Escaped - return smooth iteration count
            let log_zn = log(mag_sq) / 2.0;
            let nu = log(max(log_zn / log(2.0), 0.0001)) / log(2.0);
            return fract((f32(i) + 1.0 - nu) / 50.0);
        }

        min_dist = min(min_dist, sqrt(mag_sq));

        // Collatz map generalized to complex: z = 0.25 * (2 + 7z - (2 + 5z) * cos(π * z))
        let pi = 3.14159265359;

        // cos(π * z) for complex z
        let piz_real = pi * z.x;
        let piz_imag = pi * z.y;
        let cos_real = cos(piz_real) * cosh(piz_imag);
        let cos_imag = -sin(piz_real) * sinh(piz_imag);
        let cos_piz = vec2<f32>(cos_real, cos_imag);

        // (2 + 5z)
        let term1 = vec2<f32>(2.0 + 5.0 * z.x, 5.0 * z.y);

        // (2 + 5z) * cos(π * z)
        let product = vec2<f32>(
            term1.x * cos_piz.x - term1.y * cos_piz.y,
            term1.x * cos_piz.y + term1.y * cos_piz.x
        );

        // 2 + 7z - product, then multiply by 0.25
        z = vec2<f32>(2.0 + 7.0 * z.x - product.x, 7.0 * z.y - product.y) * 0.25;
        iteration = i;
    }

    // Didn't escape - color by minimum distance to origin
    return fract(min_dist * 2.0);
}

// Helper functions for Collatz
fn cosh(x: f32) -> f32 {
    return (exp(x) + exp(-x)) * 0.5;
}

fn sinh(x: f32) -> f32 {
    return (exp(x) - exp(-x)) * 0.5;
}
