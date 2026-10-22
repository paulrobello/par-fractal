// Quaternion Julia Set 3D
fn julia_set_3d_de(pos: vec3<f32>) -> f32 {
    // Apply inverse scale so higher slider value = bigger fractal (more intuitive)
    let scale_inv = 1.0 / uniforms.fractal_scale;
    let scaled_pos = pos * scale_inv;
    var q = vec4<f32>(scaled_pos.x, scaled_pos.y, scaled_pos.z, 0.0);
    var c = vec4<f32>(uniforms.julia_c.x, uniforms.julia_c.y, 0.0, 0.2);
    var dq = vec4<f32>(1.0, 0.0, 0.0, 0.0);

    for (var i = 0u; i < 16u; i = i + 1u) {
        // Quaternion multiplication: dq = 2 * q * dq
        dq = 2.0 * vec4<f32>(
            q.x * dq.x - q.y * dq.y - q.z * dq.z - q.w * dq.w,
            q.x * dq.y + q.y * dq.x + q.z * dq.w - q.w * dq.z,
            q.x * dq.z + q.z * dq.x + q.w * dq.y - q.y * dq.w,
            q.x * dq.w + q.w * dq.x + q.y * dq.z - q.z * dq.y
        );

        // Quaternion square: q = q * q + c
        q = vec4<f32>(
            q.x * q.x - q.y * q.y - q.z * q.z - q.w * q.w,
            2.0 * q.x * q.y,
            2.0 * q.x * q.z,
            2.0 * q.x * q.w
        ) + c;

        let r2 = dot(q, q);
        if (r2 > 4.0) {
            break;
        }
    }

    let r = length(q);
    let dr = length(dq);
    return 0.5 * r * log(r) / (dr * scale_inv);
}
