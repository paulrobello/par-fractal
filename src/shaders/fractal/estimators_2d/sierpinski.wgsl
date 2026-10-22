fn sierpinski(coord: vec2<f32>) -> f32 {
    // Sierpinski carpet - map to [0, 1] range
    var p = coord * 0.5 + 0.5;

    // Check if outside main square
    if (p.x < 0.0 || p.x >= 1.0 || p.y < 0.0 || p.y >= 1.0) {
        return -1.0;
    }

    var scale = 1.0;
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        iteration = i;

        // Scale coordinates to current level
        let scaled_x = p.x * scale;
        let scaled_y = p.y * scale;

        // Get position within current 3x3 grid (modulo 3)
        let cell_x = floor(fract(scaled_x / 3.0) * 3.0);
        let cell_y = floor(fract(scaled_y / 3.0) * 3.0);

        // Check if in the center cell (removed region)
        if (cell_x == 1.0 && cell_y == 1.0) {
            return -1.0; // Point is removed (sentinel: render as interior)
        }

        // Move to next subdivision level
        scale = scale * 3.0;

        // Stop if we've reached pixel precision
        if (scale > 1000000.0) {
            break;
        }
    }

    // Point is in the Sierpinski carpet
    return f32(iteration) / f32(uniforms.max_iterations);
}

// High-precision version for deep zooms
fn sierpinski_hp(coord_hi: vec2<f32>, coord_lo: vec2<f32>) -> f32 {
    // Recombine hi+lo for best precision at large zooms
    let coord = vec2<f32>(coord_hi.x + coord_lo.x, coord_hi.y + coord_lo.y);
    return sierpinski(coord);
}

// Sierpinski Gasket (Triangle) using barycentric subdivision
fn sierpinski_triangle(coord: vec2<f32>) -> f32 {
    // Map coord from [-1,1] to [0,1] range
    let p = coord * 0.5 + 0.5;

    // Equilateral triangle vertices
    let v0 = vec2<f32>(0.0, 0.0);       // Bottom-left
    let v1 = vec2<f32>(1.0, 0.0);       // Bottom-right
    let v2 = vec2<f32>(0.5, sqrt(3.0) * 0.5);  // Top

    // Compute barycentric coordinates
    let v0v1 = v1 - v0;
    let v0v2 = v2 - v0;
    let v0p = p - v0;

    let d00 = dot(v0v1, v0v1);
    let d01 = dot(v0v1, v0v2);
    let d11 = dot(v0v2, v0v2);
    let d20 = dot(v0p, v0v1);
    let d21 = dot(v0p, v0v2);

    let denom = d00 * d11 - d01 * d01;
    let bv = (d11 * d20 - d01 * d21) / denom;
    let bw = (d00 * d21 - d01 * d20) / denom;
    let bu = 1.0 - bv - bw;

    var bary = vec3<f32>(bu, bv, bw);  // (u=v0 weight, v=v1 weight, w=v2 weight)

    // Check if outside the main triangle
    if (bary.x < 0.0 || bary.y < 0.0 || bary.z < 0.0) {
        return -1.0;  // Outside triangle - render as black (sentinel)
    }

    // Iterate using barycentric subdivision
    // Color holes based on iteration depth (like Mandelbrot escape-time)
    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        // Check if in center hole (all coords < 0.5)
        if (bary.x < 0.5 && bary.y < 0.5 && bary.z < 0.5) {
            // In a hole at iteration i - color based on depth
            // Add smooth coloring based on how close to edge of hole
            let max_bary = max(max(bary.x, bary.y), bary.z);
            let smooth_val = max_bary / 0.5;  // 0 at center, 1 at edge
            return (f32(i) + smooth_val) / f32(uniforms.max_iterations);
        }

        // Transform to the appropriate corner sub-triangle
        if (bary.x >= 0.5) {
            // Bottom-left corner sub-triangle (near v0)
            bary = vec3<f32>(2.0 * bary.x - 1.0, 2.0 * bary.y, 2.0 * bary.z);
        } else if (bary.y >= 0.5) {
            // Bottom-right corner sub-triangle (near v1)
            bary = vec3<f32>(2.0 * bary.x, 2.0 * bary.y - 1.0, 2.0 * bary.z);
        } else {
            // Top corner sub-triangle (near v2, since bary.z >= 0.5)
            bary = vec3<f32>(2.0 * bary.x, 2.0 * bary.y, 2.0 * bary.z - 1.0);
        }
    }

    // Point survived all iterations - it's in the fractal (like points in Mandelbrot set)
    return -1.0;
}

// High-precision version for extreme zooms
fn sierpinski_triangle_hp(coord_hi: vec2<f32>, coord_lo: vec2<f32>) -> f32 {
    let coord = vec2<f32>(coord_hi.x + coord_lo.x, coord_hi.y + coord_lo.y);
    return sierpinski_triangle(coord);
}
