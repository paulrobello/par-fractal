fn box_fold(p: vec3<f32>, fold_limit: f32) -> vec3<f32> {
    return clamp(p, vec3<f32>(-fold_limit), vec3<f32>(fold_limit)) * 2.0 - p;
}

fn sphere_fold(p: vec3<f32>, min_r: f32, max_r: f32) -> vec3<f32> {
    let r2 = dot(p, p);
    if (r2 < min_r * min_r) {
        return p * (max_r * max_r / (min_r * min_r));
    } else if (r2 < max_r * max_r) {
        return p * (max_r * max_r / r2);
    }
    return p;
}
