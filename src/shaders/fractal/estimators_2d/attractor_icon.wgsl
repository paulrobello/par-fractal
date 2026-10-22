// Icon fractal with rotational symmetry
// Parameters: lambda = julia_c.x, alpha = julia_c.y, beta = power, gamma = fractal_fold, omega = fractal_min_radius, degree = fractal_scale
fn icon_attractor(coord: vec2<f32>) -> f32 {
    let lambda = uniforms.julia_c.x;
    let alpha = uniforms.julia_c.y;
    let beta = uniforms.power;
    let gamma = uniforms.fractal_fold;
    let omega = uniforms.fractal_min_radius;
    let degree = i32(uniforms.fractal_scale);

    var x = 0.1;
    var y = 0.1;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        var zn_real = 1.0;
        var zn_imag = 0.0;
        for (var j = 0; j < degree - 2; j = j + 1) {
            let temp = zn_real * x - zn_imag * y;
            zn_imag = zn_real * y + zn_imag * x;
            zn_real = temp;
        }

        let zzbar = x * x + y * y;
        let p = lambda + alpha * zzbar + beta * (x * zn_real - y * zn_imag);
        let x_new = p * x + gamma * zn_real - omega * y;
        let y_new = p * y - gamma * zn_imag + omega * x;
        x = x_new;
        y = y_new;
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        var zn_real = 1.0;
        var zn_imag = 0.0;
        for (var j = 0; j < degree - 2; j = j + 1) {
            let temp = zn_real * x - zn_imag * y;
            zn_imag = zn_real * y + zn_imag * x;
            zn_real = temp;
        }

        let zzbar = x * x + y * y;
        let p = lambda + alpha * zzbar + beta * (x * zn_real - y * zn_imag);
        let x_new = p * x + gamma * zn_real - omega * y;
        let y_new = p * y - gamma * zn_imag + omega * x;
        x = x_new;
        y = y_new;

        if (abs(x) > 100.0 || abs(y) > 100.0) { break; }

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    return clamp(hit_count / 5.0, 0.0, 1.0);
}
