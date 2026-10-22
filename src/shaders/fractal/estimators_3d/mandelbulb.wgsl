// ============================================================================
// 3D Distance Functions
// ============================================================================

fn mandelbulb_de(pos: vec3<f32>) -> f32 {
    // Apply inverse scale so higher slider value = bigger fractal (more intuitive)
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var z = pos * scale_inv;
    var dr = 1.0;
    var r = 0.0;
    let power = uniforms.power;

    for (var i = 0u; i < 16u; i = i + 1u) {
        r = length(z);
        if (r > 2.0) {
            break;
        }

        // Convert to polar coordinates
        var theta = acos(z.z / r);
        var phi = atan2(z.y, z.x);
        dr = pow(r, power - 1.0) * power * dr + 1.0;

        // Scale and rotate the point
        let zr = pow(r, power);
        theta = theta * power;
        phi = phi * power;

        // Convert back to cartesian coordinates
        z = zr * vec3<f32>(
            sin(theta) * cos(phi),
            sin(phi) * sin(theta),
            cos(theta)
        );
        z = z + pos * scale_inv;
    }

    return 0.5 * log(r) * r / (dr * scale_inv);
}
