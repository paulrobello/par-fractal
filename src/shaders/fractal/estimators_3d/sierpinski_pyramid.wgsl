// Sierpinski Pyramid/Tetrahedron
fn sierpinski_pyramid_de(pos: vec3<f32>) -> f32 {
    var p = pos;
    var scale = uniforms.fractal_scale;  // Adjustable initial scale (default 2.0)

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        // Fold into tetrahedron
        if (p.x + p.y < 0.0) {
            p = vec3<f32>(-p.y, -p.x, p.z);
        }
        if (p.x + p.z < 0.0) {
            p = vec3<f32>(-p.z, p.y, -p.x);
        }
        if (p.y + p.z < 0.0) {
            p = vec3<f32>(p.x, -p.z, -p.y);
        }

        // Scale and translate
        p = p * 2.0 - vec3<f32>(1.0, 1.0, 1.0);
        scale = scale * 2.0;
    }

    // Distance to tetrahedron
    return (length(p) - 1.0) / scale;
}
