// ============================================================================
// 2D Strange Attractor Functions (from xfractint)
// ============================================================================

// Hopalong attractor: x' = y - sign(x)*sqrt(|b*x - c|), y' = a - x
// Parameters: a = julia_c.x, b = julia_c.y, c = 0
fn hopalong_attractor(coord: vec2<f32>) -> f32 {
    let a = uniforms.julia_c.x;
    let b = uniforms.julia_c.y;
    let c = 0.0;

    var x = 0.1;
    var y = 0.0;
    var hit_count = 0.0;

    // Skip initial transient (fewer iterations for faster startup)
    for (var i = 0u; i < 50u; i = i + 1u) {
        let x_new = y - sign(x) * sqrt(abs(b * x - c));
        let y_new = a - x;
        x = x_new;
        y = y_new;
    }

    // Pixel threshold based on zoom level - smaller threshold for point scatter
    // At zoom 1, view is ~4 units wide, with ~1000 pixels, so pixel_size ~= 0.004
    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0; // 2 pixel radius

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let x_new = y - sign(x) * sqrt(abs(b * x - c));
        let y_new = a - x;
        x = x_new;
        y = y_new;

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    // Normalize hit count - saturate gradually for visible points
    return clamp(hit_count / 5.0, 0.0, 1.0);
}

// Hénon attractor: x' = 1 + y - a*x², y' = b*x
// Parameters: a = julia_c.x, b = julia_c.y
fn henon_attractor(coord: vec2<f32>) -> f32 {
    let a = uniforms.julia_c.x;
    let b = uniforms.julia_c.y;

    var x = 0.0;
    var y = 0.0;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        let x_new = 1.0 + y - a * x * x;
        let y_new = b * x;
        x = x_new;
        y = y_new;
        if (abs(x) > 100.0 || abs(y) > 100.0) {
            x = 0.1;
            y = 0.1;
        }
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let x_new = 1.0 + y - a * x * x;
        let y_new = b * x;
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

// Martin attractor: x' = y - sin(x), y' = a - x
// Parameters: a = julia_c.x
fn martin_attractor(coord: vec2<f32>) -> f32 {
    let a = uniforms.julia_c.x;

    var x = 0.1;
    var y = 0.0;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        let x_new = y - sin(x);
        let y_new = a - x;
        x = x_new;
        y = y_new;
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let x_new = y - sin(x);
        let y_new = a - x;
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

// Gingerbreadman: x' = 1 - y + |x|, y' = x
fn gingerbreadman_attractor(coord: vec2<f32>) -> f32 {
    var x = -0.1;
    var y = 0.0;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        let x_new = 1.0 - y + abs(x);
        let y_new = x;
        x = x_new;
        y = y_new;
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let x_new = 1.0 - y + abs(x);
        let y_new = x;
        x = x_new;
        y = y_new;

        if (abs(x) > 1000.0 || abs(y) > 1000.0) { break; }

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    return clamp(hit_count / 5.0, 0.0, 1.0);
}
