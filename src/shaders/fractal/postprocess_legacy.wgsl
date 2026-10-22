// ============================================================================
// Post-Processing Effects
// ============================================================================

// Convert RGB to HSV color space
fn rgb_to_hsv(rgb: vec3<f32>) -> vec3<f32> {
    let cmax = max(max(rgb.r, rgb.g), rgb.b);
    let cmin = min(min(rgb.r, rgb.g), rgb.b);
    let delta = cmax - cmin;

    var h: f32 = 0.0;
    if (delta > 0.0001) {
        if (cmax == rgb.r) {
            h = ((rgb.g - rgb.b) / delta) % 6.0;
        } else if (cmax == rgb.g) {
            h = ((rgb.b - rgb.r) / delta) + 2.0;
        } else {
            h = ((rgb.r - rgb.g) / delta) + 4.0;
        }
        h = h / 6.0;
        if (h < 0.0) {
            h = h + 1.0;
        }
    }

    var s: f32 = 0.0;
    if (cmax > 0.0001) {
        s = delta / cmax;
    }

    return vec3<f32>(h, s, cmax);
}

// Convert HSV to RGB color space
fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let h = hsv.x * 6.0;
    let s = hsv.y;
    let v = hsv.z;

    let c = v * s;
    let x = c * (1.0 - abs((h % 2.0) - 1.0));
    let m = v - c;

    var rgb: vec3<f32>;
    if (h < 1.0) {
        rgb = vec3<f32>(c, x, 0.0);
    } else if (h < 2.0) {
        rgb = vec3<f32>(x, c, 0.0);
    } else if (h < 3.0) {
        rgb = vec3<f32>(0.0, c, x);
    } else if (h < 4.0) {
        rgb = vec3<f32>(0.0, x, c);
    } else if (h < 5.0) {
        rgb = vec3<f32>(x, 0.0, c);
    } else {
        rgb = vec3<f32>(c, 0.0, x);
    }

    return rgb + m;
}

// Apply color grading (brightness, contrast, saturation, hue shift)
fn apply_color_grading(color: vec3<f32>) -> vec3<f32> {
    var result = color;

    // Brightness
    result = result * uniforms.brightness;

    // Contrast (around middle gray)
    result = ((result - 0.5) * uniforms.contrast) + 0.5;

    // Saturation and hue shift via HSV
    if (uniforms.saturation != 1.0 || uniforms.hue_shift != 0.0) {
        var hsv = rgb_to_hsv(result);
        hsv.y = hsv.y * uniforms.saturation;  // Saturation
        hsv.x = fract(hsv.x + uniforms.hue_shift);  // Hue shift
        result = hsv_to_rgb(hsv);
    }

    return clamp(result, vec3<f32>(0.0), vec3<f32>(1.0));
}

// Apply vignette effect
fn apply_vignette(color: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    if (uniforms.vignette_enabled == 0u) {
        return color;
    }

    // Calculate distance from center (0-1 range)
    let center = vec2<f32>(0.0, 0.0);
    let dist = length(uv - center);

    // Apply smooth falloff
    let vignette = smoothstep(uniforms.vignette_radius, uniforms.vignette_radius * 0.5, dist);
    let factor = mix(1.0 - uniforms.vignette_intensity, 1.0, vignette);

    return color * factor;
}

// Enhanced bloom extraction with more aggressive glow
fn extract_bloom(color: vec3<f32>) -> vec3<f32> {
    if (uniforms.bloom_enabled == 0u) {
        return vec3<f32>(0.0);
    }

    let luminance = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    if (luminance > uniforms.bloom_threshold) {
        // Much more aggressive bloom calculation
        let bloom_amount = pow((luminance - uniforms.bloom_threshold) / (1.0 - uniforms.bloom_threshold + 0.001), 0.3);
        // Very high multiplier for strong visibility - affects entire bright areas
        return color * bloom_amount * uniforms.bloom_intensity * 5.0;
    }
    return vec3<f32>(0.0);
}

// FXAA is disabled - placeholder for future multi-pass implementation
fn apply_fxaa(color: vec3<f32>) -> vec3<f32> {
    return color;
}

// Main post-processing function
fn apply_post_processing(color: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    var result = color;

    // Apply bloom BEFORE color grading to prevent clamping
    if (uniforms.bloom_enabled == 1u) {
        let bloom = extract_bloom(result);
        result = result + bloom;
    }

    // Apply color grading
    result = apply_color_grading(result);

    // Apply vignette
    result = apply_vignette(result, uv);

    // FXAA removed from processing - not implemented
    // Future: add multi-pass rendering for proper FXAA

    return result;
}
