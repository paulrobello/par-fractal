// Amazing Surface / Pseudo-Kleinian fractal
fn kleinian_de(pos: vec3<f32>) -> f32 {
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var p = pos * scale_inv;

    let scale = uniforms.fractal_fold + 1.0;
    let csize = vec3<f32>(1.0, 1.0, 1.3);
    let c = vec3<f32>(0.0, 0.0, -uniforms.fractal_min_radius);
    var de = 1.0;

    for (var i = 0u; i < 12u; i = i + 1u) {
        // Folding operations
        p = abs(p);

        // Conditional folds for more complex structure
        if (p.x - p.y < 0.0) { let t = p.x; p.x = p.y; p.y = t; }
        if (p.x - p.z < 0.0) { let t = p.x; p.x = p.z; p.z = t; }
        if (p.y - p.z < 0.0) { let t = p.y; p.y = p.z; p.z = t; }

        // Box fold
        p = p * scale - csize * (scale - 1.0);

        // Sphere inversion
        let r2 = dot(p, p);
        let k = clamp(1.0 / r2, 1.0, 3.0);
        p = p * k;
        de = de * k;

        // Translation
        p = p + c;
        de = de * scale;
    }

    return (length(p) - 0.4) / de;
}
