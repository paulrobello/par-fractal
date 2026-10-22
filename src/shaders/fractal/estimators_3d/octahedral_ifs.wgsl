// Octahedral IFS - 8-fold kaleidoscopic symmetry
fn octahedral_ifs_de(pos: vec3<f32>) -> f32 {
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var p = pos * scale_inv;
    var scale = uniforms.fractal_fold + 1.0;  // Use fold parameter for scaling (2.0-3.0 range works well)
    var dr = 1.0;

    for (var i = 0u; i < uniforms.max_steps; i = i + 1u) {
        // Octahedral folding - creates 8-fold symmetry
        p = abs(p);
        if (p.x - p.y < 0.0) {
            let temp = p.x;
            p.x = p.y;
            p.y = temp;
        }
        if (p.x - p.z < 0.0) {
            let temp = p.x;
            p.x = p.z;
            p.z = temp;
        }
        if (p.y - p.z < 0.0) {
            let temp = p.y;
            p.y = p.z;
            p.z = temp;
        }

        // Apply scaling and translation
        p = p * scale - vec3<f32>(1.0) * (scale - 1.0);
        dr = dr * scale;

        // Bailout
        if (length(p) > 100.0) {
            break;
        }
    }

    // Distance estimate - box shape
    return (length(p) - 2.0) / abs(dr);
}
