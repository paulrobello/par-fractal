// Latoocarfian: x' = sin(y*b) + c*sin(x*b), y' = sin(x*a) + d*sin(y*a)
// Parameters: a = julia_c.x, b = julia_c.y, c = power, d = fractal_fold
fn latoocarfian_attractor(coord: vec2<f32>) -> f32 {
    let a = uniforms.julia_c.x;
    let b = uniforms.julia_c.y;
    let c = uniforms.power;
    let d = uniforms.fractal_fold;

    var x = 0.1;
    var y = 0.1;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        let x_new = sin(y * b) + c * sin(x * b);
        let y_new = sin(x * a) + d * sin(y * a);
        x = x_new;
        y = y_new;
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let x_new = sin(y * b) + c * sin(x * b);
        let y_new = sin(x * a) + d * sin(y * a);
        x = x_new;
        y = y_new;

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    return clamp(hit_count / 5.0, 0.0, 1.0);
}

// Chip: x' = y - sign(x)*cos(log²(|b*x - c|))*arctan(log²(|c*x - b|)), y' = a - x
// Parameters: a = julia_c.x, b = julia_c.y, c = power
fn chip_attractor(coord: vec2<f32>) -> f32 {
    let a = uniforms.julia_c.x;
    let b = uniforms.julia_c.y;
    let c = uniforms.power;

    var x = 0.1;
    var y = 0.0;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        let log1 = log(max(abs(b * x - c), 0.001));
        let log2 = log(max(abs(c * x - b), 0.001));
        let x_new = y - sign(x) * cos(log1 * log1) * atan(log2 * log2);
        let y_new = a - x;
        x = x_new;
        y = y_new;
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let log1 = log(max(abs(b * x - c), 0.001));
        let log2 = log(max(abs(c * x - b), 0.001));
        let x_new = y - sign(x) * cos(log1 * log1) * atan(log2 * log2);
        let y_new = a - x;
        x = x_new;
        y = y_new;

        if (abs(x) > 10000.0 || abs(y) > 10000.0) { break; }

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    return clamp(hit_count / 5.0, 0.0, 1.0);
}

// Quadruptwo: x' = y - sign(x)*sin(log(|b*x - c|))*arctan(log²(|c*x - b|)), y' = a - x
// Parameters: a = julia_c.x, b = julia_c.y, c = power
fn quadruptwo_attractor(coord: vec2<f32>) -> f32 {
    let a = uniforms.julia_c.x;
    let b = uniforms.julia_c.y;
    let c = uniforms.power;

    var x = 0.1;
    var y = 0.0;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        let log1 = log(max(abs(b * x - c), 0.001));
        let log2 = log(max(abs(c * x - b), 0.001));
        let x_new = y - sign(x) * sin(log1) * atan(log2 * log2);
        let y_new = a - x;
        x = x_new;
        y = y_new;
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let log1 = log(max(abs(b * x - c), 0.001));
        let log2 = log(max(abs(c * x - b), 0.001));
        let x_new = y - sign(x) * sin(log1) * atan(log2 * log2);
        let y_new = a - x;
        x = x_new;
        y = y_new;

        if (abs(x) > 10000.0 || abs(y) > 10000.0) { break; }

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    return clamp(hit_count / 5.0, 0.0, 1.0);
}

// Threeply: x' = y - sign(x)*|sin(x)*cos(b) + c - x*sin(a+b+c)|, y' = a - x
// Parameters: a = julia_c.x, b = julia_c.y, c = power
fn threeply_attractor(coord: vec2<f32>) -> f32 {
    let a = uniforms.julia_c.x;
    let b = uniforms.julia_c.y;
    let c = uniforms.power;

    var x = 0.1;
    var y = 0.0;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        let term = sin(x) * cos(b) + c - x * sin(a + b + c);
        let x_new = y - sign(x) * abs(term);
        let y_new = a - x;
        x = x_new;
        y = y_new;
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let term = sin(x) * cos(b) + c - x * sin(a + b + c);
        let x_new = y - sign(x) * abs(term);
        let y_new = a - x;
        x = x_new;
        y = y_new;

        if (abs(x) > 100000.0 || abs(y) > 100000.0) { break; }

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    return clamp(hit_count / 5.0, 0.0, 1.0);
}
