// ============================================================================
// Light Direction Calculation
// ============================================================================

// Convert azimuth and elevation angles to a 3D direction vector
// Azimuth: horizontal angle in degrees (0-360), 0=+Z, 90=+X
// Elevation: vertical angle in degrees (5-90), 90=straight up
fn calculate_light_direction() -> vec3<f32> {
    let PI = 3.14159265359;
    let azimuth_rad = uniforms.light_azimuth * PI / 180.0;
    let elevation_rad = uniforms.light_elevation * PI / 180.0;

    // Spherical to Cartesian conversion
    let cos_elev = cos(elevation_rad);
    let sin_elev = sin(elevation_rad);
    let cos_azim = cos(azimuth_rad);
    let sin_azim = sin(azimuth_rad);

    // Build direction vector
    return normalize(vec3<f32>(
        cos_elev * sin_azim,  // X component
        sin_elev,             // Y component (up)
        cos_elev * cos_azim   // Z component
    ));
}

// ============================================================================
// Normal Calculation
// ============================================================================

fn calculate_normal(pos: vec3<f32>) -> vec3<f32> {
    let eps = 0.001;
    let h = vec2<f32>(eps, 0.0);
    return normalize(vec3<f32>(
        scene_de(pos + h.xyy) - scene_de(pos - h.xyy),
        scene_de(pos + h.yxy) - scene_de(pos - h.yxy),
        scene_de(pos + h.yyx) - scene_de(pos - h.yyx)
    ));
}

// ============================================================================
// Ambient Occlusion
// ============================================================================

fn calculate_ao(pos: vec3<f32>, normal: vec3<f32>) -> f32 {
    if (uniforms.ambient_occlusion == 0u) {
        return 1.0;
    }

    var occ = 0.0;
    var weight = 1.0;

    for (var i = 1; i <= 5; i = i + 1) {
        let h = 0.01 + uniforms.ao_step_size * f32(i) / 5.0;
        let d = scene_de(pos + h * normal);
        occ = occ + (h - d) * weight;
        weight = weight * 0.95;
    }

    return clamp(1.0 - uniforms.ao_intensity * occ, 0.0, 1.0);
}

// ============================================================================
// Soft Shadows
// ============================================================================

fn calculate_soft_shadow(origin: vec3<f32>, direction: vec3<f32>, mint: f32) -> f32 {
    // soft_shadows: 0 = off, 1 = hard, 2 = soft
    if (uniforms.soft_shadows == 0u) {
        return 1.0;
    }

    var result = 1.0;
    var t = mint;
    let maxt = uniforms.shadow_max_distance;
    // Use the same precision as main ray marching for consistent shadow detection
    let shadow_threshold = uniforms.min_distance * 2.0;

    // Use same bounding sphere as main ray marching
    let iteration_factor = f32(max(uniforms.max_iterations, uniforms.max_steps)) * 0.1;
    let bounding_radius = 50.0 * uniforms.fractal_scale * max(1.0, uniforms.fractal_fold) * max(1.0, iteration_factor);
    let bounding_center = vec3<f32>(0.0);

    // Use configurable number of samples for shadow detection quality
    let max_samples = i32(uniforms.shadow_samples);
    for (var i = 0; i < max_samples; i = i + 1) {
        let pos = origin + direction * t;

        // Check if shadow ray has exited the bounding sphere
        // If so, there's no fractal to occlude, so return full light
        if (length(pos - bounding_center) > bounding_radius) {
            return result;
        }

        // Types with a cheap bounding volume (the 3D attractors, whose exact
        // estimate re-simulates the whole trajectory per call) get a two-tier
        // evaluation: take the bound first, and only pay for the exact estimate
        // when the bound cannot settle this sample.
        //
        // This is exact, not an approximation. The exact estimate is always
        // >= the bound, so a bound that clears `shadow_threshold` proves the
        // exact value clears it too, and a bound whose penumbra term already
        // exceeds `result` proves the exact value cannot lower `result`
        // either. Stepping by the bound is likewise conservative.
        var step_dist: f32;
        var settled = false;
        if (has_conservative_bound()) {
            let bound = scene_de_with_material(pos, true).distance;
            let clears_occlusion = bound >= shadow_threshold;
            let cannot_darken = uniforms.soft_shadows != 2u
                || uniforms.shadow_softness * bound / t >= result;
            if (clears_occlusion && cannot_darken) {
                step_dist = bound;
                settled = true;
            }
        }

        if (!settled) {
            let h = scene_de(pos);
            if (h < shadow_threshold) {
                // Hard shadows: binary occlusion
                if (uniforms.soft_shadows == 1u) {
                    return 0.0;
                }
                // Soft shadows: accumulate penumbra factor
                result = 0.0;
                break;
            }
            if (uniforms.soft_shadows == 2u) {
                result = min(result, uniforms.shadow_softness * h / t);
            }
            step_dist = h;
        }

        // Conservative stepping to prevent missing thin features:
        // Use configurable step factor (default 0.6 = 60% of DE for safety margin)
        // Also cap at smaller maximum to prevent over-stepping
        let conservative_step = step_dist * uniforms.shadow_step_factor;
        let max_step = 0.05 * uniforms.fractal_scale;
        let step = min(conservative_step, max_step);
        t = t + step;

        if (t > maxt) {
            break;
        }
    }

    return result;
}
