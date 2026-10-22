// Quaternion cubic Julia set (z³ + c)
fn quaternion_cubic_de(pos: vec3<f32>) -> f32 {
    let scale_inv = 1.0 / uniforms.fractal_scale;
    // Map 3D position to quaternion (w=0)
    var q = vec4<f32>(pos * scale_inv, 0.0);
    var dq = vec4<f32>(1.0, 0.0, 0.0, 0.0);

    // Julia constant as quaternion
    let c = vec4<f32>(uniforms.julia_c.x, uniforms.julia_c.y, 0.3, 0.0);

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let r = length(q);
        if (r > 2.0) { break; }

        // Quaternion squaring: q² (for derivative)
        let q2 = quat_mul(q, q);

        // Derivative: dq = 3*q²*dq (chain rule for z³)
        dq = quat_mul(quat_mul(vec4<f32>(3.0, 0.0, 0.0, 0.0), q2), dq);

        // Quaternion cubing: q³ = q * q²
        let q3 = quat_mul(q, q2);

        // z = z³ + c
        q = q3 + c;
    }

    let r = length(q);
    let dr = length(dq);
    return 0.5 * log(r) * r / dr;
}
