// Lorenz attractor: dx/dt = σ(y-x), dy/dt = x(ρ-z)-y, dz/dt = xy - βz
fn lorenz_attractor_de(pos: vec3<f32>, conservative: bool) -> f32 {
    let scale = uniforms.fractal_scale;
    let sigma = uniforms.julia_c.x;   // Default: 10
    let rho = uniforms.julia_c.y;     // Default: 28
    let beta = uniforms.power;        // Default: 8/3 ≈ 2.666

    // Empty-space rejection, same pattern as `pickover_attractor_de`: the
    // trajectory (defaults measured over 8000 steps) stays inside |x| <= 17,
    // |y| <= 22, z in [3, 44], so its box distance is a valid lower bound on
    // the distance to the cloud. Halves grow with sigma/rho so slider moves
    // keep the box a superset; too generous only skips less empty space.
    let tube = 0.3 * scale;
    let half = vec3<f32>(max(20.0, sigma * 1.7), max(25.0, sigma * 2.2), max(24.0, rho * 0.9)) * scale
        + vec3<f32>(tube);
    let outside = sd_box(pos, half);
    if (conservative && outside > uniforms.min_distance * 4.0) {
        return outside;
    }

    var min_dist = 1000.0;
    var x = 0.1;
    var y = 0.0;
    var z = 0.0;
    let dt = 0.005;

    // Skip transient
    for (var i = 0u; i < 1000u; i = i + 1u) {
        let dx = sigma * (y - x);
        let dy = x * (rho - z) - y;
        let dz = x * y - beta * z;
        x = x + dx * dt;
        y = y + dy * dt;
        z = z + dz * dt;
    }

    // Sample points and find closest
    for (var i = 0u; i < 2000u; i = i + 1u) {
        let dx = sigma * (y - x);
        let dy = x * (rho - z) - y;
        let dz = x * y - beta * z;
        x = x + dx * dt;
        y = y + dy * dt;
        z = z + dz * dt;

        // The butterfly's envelope midpoint is z ≈ 23.5 (rho - 4.5 at the
        // default rho = 28); the shift centres the cloud on the origin the
        // default camera frames.
        let attractor_pos = vec3<f32>(x, y, z - 23.5) * scale;
        let dist = length(pos - attractor_pos) - 0.3 * scale;
        min_dist = min(min_dist, dist);
    }

    return min_dist;
}

// Rossler attractor: dx/dt = -y - z, dy/dt = x + a*y, dz/dt = b + z*(x - c)
fn rossler_attractor_de(pos: vec3<f32>, conservative: bool) -> f32 {
    let scale = uniforms.fractal_scale;
    let a = uniforms.julia_c.x;   // Default: 0.2
    let b = uniforms.julia_c.y;   // Default: 0.2
    let c = uniforms.power;       // Default: 5.7

    // Empty-space rejection (see `lorenz_attractor_de`). Defaults measured
    // over 6000 steps: x in [-9.4, 12.1], y in [-11.3, 8.0], z in [0, 29] —
    // envelope midpoint (1.3, -1.7, 14.5), halves (10.7, 9.7, 14.5). Halves
    // grow with c (the extent scales roughly linearly in c).
    let tube = 0.15 * scale;
    let center = vec3<f32>(1.3, -1.7, 14.5);
    let half = vec3<f32>(max(12.0, c * 2.2), max(11.0, c * 2.0), max(16.0, c * 2.8)) * scale
        + vec3<f32>(tube);
    let outside = sd_box(pos, half);
    if (conservative && outside > uniforms.min_distance * 4.0) {
        return outside;
    }

    var min_dist = 1000.0;
    var x = 0.1;
    var y = 0.0;
    var z = 0.0;
    // 0.01 with a 500-step transient never leaves the origin: the trajectory
    // is still at ~(0.1, -0.2, 0.0) when sampling starts, so the attractor
    // rendered as a tiny spiral. 0.02 x 2000 reaches the true extent (~26).
    let dt = 0.02;

    // Skip transient
    for (var i = 0u; i < 2000u; i = i + 1u) {
        let dx = -y - z;
        let dy = x + a * y;
        let dz = b + z * (x - c);
        x = x + dx * dt;
        y = y + dy * dt;
        z = z + dz * dt;
    }

    // Sample points and find closest
    for (var i = 0u; i < 1500u; i = i + 1u) {
        let dx = -y - z;
        let dy = x + a * y;
        let dz = b + z * (x - c);
        x = x + dx * dt;
        y = y + dy * dt;
        z = z + dz * dt;

        // Envelope-midpoint recenter (see the conservative box above): the
        // raw cloud sits around (1.3, -1.7, 14.5), so shift it onto the
        // origin the default camera frames.
        let attractor_pos = vec3<f32>(x - center.x, y - center.y, z - center.z) * scale;
        let dist = length(pos - attractor_pos) - 0.15 * scale;
        min_dist = min(min_dist, dist);
    }

    return min_dist;
}
