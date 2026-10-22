// Icosahedral IFS - 20-fold kaleidoscopic symmetry (most complex!)
fn icosahedral_ifs_de(pos: vec3<f32>) -> f32 {
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var p = pos * scale_inv;
    let scale = uniforms.fractal_fold + 1.0;
    var dr = 1.0;

    // Golden ratio for icosahedral symmetry
    let phi = (1.0 + sqrt(5.0)) / 2.0;

    // Pre-compute normalized icosahedral fold normals
    // These define the 15 mirror planes of icosahedral symmetry
    let n1 = normalize(vec3<f32>(1.0, phi, 0.0));
    let n2 = normalize(vec3<f32>(0.0, 1.0, phi));
    let n3 = normalize(vec3<f32>(phi, 0.0, 1.0));
    // Additional planes for full icosahedral symmetry
    let n4 = normalize(vec3<f32>(-1.0, phi, 0.0));
    let n5 = normalize(vec3<f32>(0.0, -1.0, phi));

    for (var i = 0u; i < uniforms.max_steps; i = i + 1u) {
        // Icosahedral folding - creates 20-fold symmetry
        // Fold across icosahedral mirror planes (before abs!)
        var d = dot(p, n1);
        if (d < 0.0) { p = p - 2.0 * d * n1; }

        d = dot(p, n2);
        if (d < 0.0) { p = p - 2.0 * d * n2; }

        d = dot(p, n3);
        if (d < 0.0) { p = p - 2.0 * d * n3; }

        d = dot(p, n4);
        if (d < 0.0) { p = p - 2.0 * d * n4; }

        d = dot(p, n5);
        if (d < 0.0) { p = p - 2.0 * d * n5; }

        // Apply abs for remaining octant folding
        p = abs(p);

        // Sort coordinates to ensure consistent fundamental domain
        if (p.x < p.y) {
            let temp = p.x;
            p.x = p.y;
            p.y = temp;
        }
        if (p.y < p.z) {
            let temp = p.y;
            p.y = p.z;
            p.z = temp;
        }
        if (p.x < p.y) {
            let temp = p.x;
            p.x = p.y;
            p.y = temp;
        }

        // Apply scaling and translation
        p = p * scale - vec3<f32>(1.0) * (scale - 1.0);
        dr = dr * scale;

        if (length(p) > 100.0) {
            break;
        }
    }

    return (length(p) - 2.0) / abs(dr);
}
