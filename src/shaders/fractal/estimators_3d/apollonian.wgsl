// Apollonian Gasket - sphere packing fractal
fn apollonian_gasket_de(pos: vec3<f32>) -> f32 {
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var p = pos * scale_inv;

    // Use min_radius as the packing parameter (typically 0.5-2.0)
    let k = uniforms.fractal_min_radius;
    // Use fold parameter as scale multiplier for varying complexity
    let scale = (1.0 + k) * uniforms.fractal_fold;
    var dr = 1.0;

    for (var i = 0u; i < uniforms.max_steps; i = i + 1u) {
        // Inversion folding - key to Apollonian gasket
        p = abs(p);

        // Sphere inversion
        let r2 = dot(p, p);
        let k2 = k * k;

        if (r2 < k2) {
            p = p * (1.0 / k2);
            dr = dr * (1.0 / k2);
        } else if (r2 < 1.0) {
            p = p / r2;
            dr = dr / r2;
        }

        // Box folding for additional structure
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

        // Scale and translate
        p = p * scale - vec3<f32>(k, k, k);
        dr = dr * scale;

        if (length(p) > 100.0) {
            break;
        }
    }

    // Sphere distance
    return (length(p) - 1.0) / abs(dr);
}
