// Hybrid Mandelbulb-Julia fractal
fn hybrid_mandelbulb_julia_de(pos: vec3<f32>) -> f32 {
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var z = pos * scale_inv;
    var dr = 1.0;
    var r = length(z);

    // Julia constant from julia_c parameter
    let c = vec3<f32>(uniforms.julia_c.x, uniforms.julia_c.y, 0.3);

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (r > 2.0) { break; }

        // Convert to spherical
        let theta = acos(z.z / r);
        let phi = atan2(z.y, z.x);
        dr = pow(r, uniforms.power - 1.0) * uniforms.power * dr + 1.0;

        // Scale and rotate
        let zr = pow(r, uniforms.power);
        let new_theta = theta * uniforms.power;
        let new_phi = phi * uniforms.power;

        // Back to cartesian
        z = zr * vec3<f32>(
            sin(new_theta) * cos(new_phi),
            sin(new_theta) * sin(new_phi),
            cos(new_theta)
        );

        // Alternate between Mandelbrot (add pos) and Julia (add c)
        if (i % 2u == 0u) {
            z = z + pos * scale_inv;
        } else {
            z = z + c;
        }

        r = length(z);
    }

    return 0.5 * log(r) * r / dr;
}
