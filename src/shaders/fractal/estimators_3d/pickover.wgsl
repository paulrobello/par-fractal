// ============================================================================
// 3D Strange Attractor Distance Functions
// ============================================================================

// Pickover attractor: Creates a 3D orbit that we can ray march
// x' = sin(a*y) - z*cos(b*x), y' = z*sin(c*x) - cos(d*y), z' = sin(x)
// Signed distance to an axis-aligned box centred at the origin with
// half-extents `half`. Negative inside.
fn sd_box(p: vec3<f32>, half: vec3<f32>) -> f32 {
    let q = abs(p) - half;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

// True when the active fractal's estimator can return a cheap lower bound
// instead of its exact (very expensive) value — see `pickover_attractor_de`.
// Callers use this to decide whether a two-tier evaluation is worth it; for
// every other type the bound *is* the exact value, so the second tier would
// only duplicate work.
fn has_conservative_bound() -> bool {
    let ty = select(uniforms.fractal_type, SPECIALIZED_TYPE, SPECIALIZED_TYPE != 0xFFFFu);
    return ty == 35u;
}

fn pickover_attractor_de(pos: vec3<f32>, conservative: bool) -> f32 {
    let scale = uniforms.fractal_scale;
    let a = uniforms.julia_c.x;
    let b = uniforms.julia_c.y;
    let c = uniforms.power;
    let d = uniforms.fractal_fold;

    // The trajectory below does not depend on `pos`, yet it is re-run in full
    // for every ray-marching step of every pixel — the dominant cost of this
    // fractal by far. Almost all of those steps are in empty space, so when the
    // caller only needs a safe lower bound, reject them against the attractor's
    // bounding box instead of simulating.
    //
    // The bound is exact and parameter-independent: z = sin(x) gives |z| <= 1,
    // hence |x| <= |sin(a*y)| + |z * cos(b*x)| <= 2, and likewise |y| <= 2.
    // Distance to a box enclosing the point cloud is a lower bound on the
    // distance to the cloud, so sphere tracing stays correct.
    //
    // `conservative` is false for soft shadows and ambient occlusion: those
    // read the estimate's *magnitude* as a proximity signal, so a lower bound
    // makes them shade the bounding box rather than the attractor.
    let radius = 0.02 * scale;
    let bounds = vec3<f32>(2.0, 2.0, 1.0) * scale + vec3<f32>(radius);
    let outside = sd_box(pos, bounds);
    // Hand back to the exact estimator inside a thin shell around the box: the
    // marcher counts any estimate below `min_distance` as a surface hit, so an
    // early-out that reached zero would render the bounding box itself.
    if (conservative && outside > uniforms.min_distance * 4.0) {
        return outside;
    }

    // Build attractor point cloud and find minimum distance
    var min_dist = 1000.0;
    var x = 0.1;
    var y = 0.1;
    var z = 0.1;

    // Skip transient
    for (var i = 0u; i < 500u; i = i + 1u) {
        let x_new = sin(a * y) - z * cos(b * x);
        let y_new = z * sin(c * x) - cos(d * y);
        let z_new = sin(x);
        x = x_new;
        y = y_new;
        z = z_new;
    }

    // Sample points and find closest
    for (var i = 0u; i < 1000u; i = i + 1u) {
        let x_new = sin(a * y) - z * cos(b * x);
        let y_new = z * sin(c * x) - cos(d * y);
        let z_new = sin(x);
        x = x_new;
        y = y_new;
        z = z_new;

        let attractor_pos = vec3<f32>(x, y, z) * scale;
        let dist = length(pos - attractor_pos) - 0.02 * scale; // Small sphere at each point
        min_dist = min(min_dist, dist);
    }

    return min_dist;
}
