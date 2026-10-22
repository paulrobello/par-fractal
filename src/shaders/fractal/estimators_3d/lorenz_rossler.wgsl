// Lorenz attractor: dx/dt = σ(y-x), dy/dt = x(ρ-z)-y, dz/dt = xy - βz
fn lorenz_attractor_de(pos: vec3<f32>) -> f32 {
    let scale = uniforms.fractal_scale;
    let sigma = uniforms.julia_c.x;   // Default: 10
    let rho = uniforms.julia_c.y;     // Default: 28
    let beta = uniforms.power;        // Default: 8/3 ≈ 2.666

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

        // Lorenz center is around (0, 0, 27) with wings spanning ~30 units
        let attractor_pos = vec3<f32>(x, y, z - 25.0) * scale;
        let dist = length(pos - attractor_pos) - 0.3 * scale;
        min_dist = min(min_dist, dist);
    }

    return min_dist;
}

// Rossler attractor: dx/dt = -y - z, dy/dt = x + a*y, dz/dt = b + z*(x - c)
fn rossler_attractor_de(pos: vec3<f32>) -> f32 {
    let scale = uniforms.fractal_scale;
    let a = uniforms.julia_c.x;   // Default: 0.2
    let b = uniforms.julia_c.y;   // Default: 0.2
    let c = uniforms.power;       // Default: 5.7

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

        let attractor_pos = vec3<f32>(x, y, z) * scale;
        let dist = length(pos - attractor_pos) - 0.15 * scale;
        min_dist = min(min_dist, dist);
    }

    return min_dist;
}
