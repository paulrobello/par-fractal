fn menger_sponge_de(pos: vec3<f32>) -> f32 {
    // Use inverse scale for intuitive behavior (higher = bigger)
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var p = pos * scale_inv;  // Adjustable initial scale (default 2.0 -> 0.5)
    var scale = scale_inv;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        p = abs(p);

        if (p.x < p.y) {
            p = p.yxz;
        }
        if (p.x < p.z) {
            p = p.zyx;
        }
        if (p.y < p.z) {
            p = p.xzy;
        }

        p = p * 3.0;
        scale = scale * 3.0;
        p = p - vec3<f32>(2.0, 2.0, 2.0);

        if (p.z < -1.0) {
            p.z = p.z + 2.0;
        }
    }

    let d = max(max(abs(p.x), abs(p.y)), abs(p.z)) - 1.0;
    return d / scale;
}
