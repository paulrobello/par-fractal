// ============================================================================
// Depth of Field Helpers
// ============================================================================

// Simple hash function for pseudo-random numbers
fn hash2(p: vec2<f32>) -> vec2<f32> {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let p3_shifted = p3 + dot(p3, vec3<f32>(p3.y, p3.z, p3.x) + 33.33);
    return fract(vec2<f32>(
        (p3_shifted.x + p3_shifted.y) * p3_shifted.z,
        (p3_shifted.x + p3_shifted.z) * p3_shifted.y
    ));
}

// Sample a point on a disk (for aperture) with sample index for multi-sampling
fn sample_disk_indexed(uv: vec2<f32>, sample_index: u32, num_samples: u32) -> vec2<f32> {
    // Use deterministic Vogel disk distribution (golden angle spiral)
    // This creates evenly distributed samples without noise
    let golden_angle = 2.39996322972; // Golden angle in radians
    let angle = f32(sample_index) * golden_angle;
    let radius = sqrt((f32(sample_index) + 0.5) / f32(num_samples)); // +0.5 for better center coverage

    return vec2<f32>(cos(angle), sin(angle)) * radius;
}

// Sample a point on a disk (for aperture) - single sample version
fn sample_disk(uv: vec2<f32>) -> vec2<f32> {
    // Use pixel position for stable sampling (not time-based to avoid jitter)
    let hash = hash2(uv);
    let angle = hash.x * 6.28318530718; // 2 * PI
    let radius = sqrt(hash.y);
    return vec2<f32>(cos(angle), sin(angle)) * radius;
}
