fn mandelbox_de(pos: vec3<f32>) -> f32 {
    // Use fractal_scale for size control (inverse for intuitive behavior)
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var p = pos * scale_inv * 3.305;  // Scale with adjustable parameter
    var dr = 1.0;
    // Use power parameter as internal scale (default 8.0, typically -3.0 to -1.5 for detail)
    let internal_scale = -(uniforms.power / 4.0);  // Maps 8.0 -> -2.0
    let fold_limit = uniforms.fractal_fold;  // Adjustable fold limit (default 1.0)
    let min_radius2 = uniforms.fractal_min_radius * uniforms.fractal_min_radius;  // Square for r2 comparison
    let fixed_radius2 = 1.0;

    for (var i = 0u; i < uniforms.max_steps; i = i + 1u) {
        // Box fold
        p = clamp(p, vec3<f32>(-fold_limit), vec3<f32>(fold_limit)) * 2.0 - p;

        // Sphere fold
        let r2 = dot(p, p);

        if (r2 < min_radius2) {
            let temp = fixed_radius2 / min_radius2;
            p = p * temp;
            dr = dr * temp;
        } else if (r2 < fixed_radius2) {
            let temp = fixed_radius2 / r2;
            p = p * temp;
            dr = dr * temp;
        }

        // Scale and translate
        p = p * internal_scale + pos * scale_inv * 3.305;
        dr = dr * abs(internal_scale) + 1.0;

        if (length(p) > 100.0) {
            break;
        }
    }

    // Scale result back down (20% larger than initial 75% reduction)
    return (length(p) / abs(dr)) * 0.3025;
}
