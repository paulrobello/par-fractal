// Sierpinski Gasket (Tetrahedral IFS with sphere folding)
fn sierpinski_gasket_de(pos: vec3<f32>) -> f32 {
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var p = pos * scale_inv;
    let scale_factor = 2.0 + uniforms.fractal_fold * 0.5;  // Base scale factor (2.0-3.0 range)
    var scale = 1.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        // Tetrahedral folding - creates 4-fold symmetry
        if (p.x + p.y < 0.0) {
            p = vec3<f32>(-p.y, -p.x, p.z);
        }
        if (p.x + p.z < 0.0) {
            p = vec3<f32>(-p.z, p.y, -p.x);
        }
        if (p.y + p.z < 0.0) {
            p = vec3<f32>(p.x, -p.z, -p.y);
        }

        // Sphere inversion folding to create "gasket" holes
        let r2 = dot(p, p);
        let min_r2 = uniforms.fractal_min_radius * uniforms.fractal_min_radius;

        if (r2 < min_r2) {
            let k = min_r2 / r2;
            p = p * k;
            scale = scale * k;
        }

        // Scale and translate to create the gasket structure
        p = p * scale_factor - vec3<f32>(1.0, 1.0, 1.0) * (scale_factor - 1.0);
        scale = scale * scale_factor;
    }

    // Distance to tetrahedron
    return (length(p) - 1.0) / scale;
}
