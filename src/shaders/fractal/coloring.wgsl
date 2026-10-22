// ============================================================================
// Color Palette Function
// ============================================================================

fn get_palette_color(t: f32) -> vec3<f32> {
    // Check if using procedural palette
    if (uniforms.procedural_palette_type > 0u) {
        return get_procedural_palette_color(t);
    }

    // Apply palette offset for animation, wrapping around with fract
    let t_animated = fract(t + uniforms.palette_offset);
    let t_clamped = clamp(t_animated, 0.0, 1.0);
    let scaled = t_clamped * 7.0;  // 8 colors = 7 segments
    let index = u32(floor(scaled));
    let fract_val = scaled - f32(index);

    if (index >= 7u) {
        return uniforms.palette[7].rgb;
    }

    let c1 = uniforms.palette[index].rgb;
    let c2 = uniforms.palette[index + 1u].rgb;
    return mix(c1, c2, fract_val);
}

// ============================================================================
// Procedural Palette Functions
// These generate colors mathematically using cosine-based formulas
// ============================================================================

const PI: f32 = 3.14159265359;
const TWO_PI: f32 = 6.28318530718;

// Generic cosine palette formula: color(t) = a + b * cos(2π * (c * t + d))
// where a = brightness, b = contrast, c = frequency, d = phase
fn cosine_palette(t: f32, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>) -> vec3<f32> {
    return a + b * cos(TWO_PI * (c * t + d));
}

fn get_procedural_palette_color(t: f32) -> vec3<f32> {
    // Apply palette offset for animation
    let t_animated = fract(t + uniforms.palette_offset);

    // Select palette based on type
    switch (uniforms.procedural_palette_type) {
        case 1u: {
            // Firestrm - Classic Fractint fire storm palette
            // RGB phase-shifted cosines: r=cos(a), g=cos(a+2π/3), b=cos(a+4π/3)
            let angle = t_animated * TWO_PI;
            let r = (cos(angle) + 1.0) * 0.5;
            let g = (cos(angle + TWO_PI / 3.0) + 1.0) * 0.5;
            let b = (cos(angle + TWO_PI * 2.0 / 3.0) + 1.0) * 0.5;
            return vec3<f32>(r, g, b);
        }
        case 2u: {
            // Rainbow - HSV hue rotation (red -> yellow -> green -> cyan -> blue -> magenta -> red)
            let h = t_animated;
            let s = 1.0;
            let v = 1.0;
            // HSV to RGB conversion
            let c = v * s;
            let x = c * (1.0 - abs(fract(h * 6.0) * 2.0 - 1.0));
            let m = v - c;
            let h6 = h * 6.0;
            var rgb: vec3<f32>;
            if (h6 < 1.0) {
                rgb = vec3<f32>(c, x, 0.0);
            } else if (h6 < 2.0) {
                rgb = vec3<f32>(x, c, 0.0);
            } else if (h6 < 3.0) {
                rgb = vec3<f32>(0.0, c, x);
            } else if (h6 < 4.0) {
                rgb = vec3<f32>(0.0, x, c);
            } else if (h6 < 5.0) {
                rgb = vec3<f32>(x, 0.0, c);
            } else {
                rgb = vec3<f32>(c, 0.0, x);
            }
            return rgb + vec3<f32>(m, m, m);
        }
        case 3u: {
            // Electric - Cyan to blue to purple
            return cosine_palette(t_animated,
                vec3<f32>(0.5, 0.5, 0.5),
                vec3<f32>(0.5, 0.5, 0.5),
                vec3<f32>(1.0, 1.0, 1.0),
                vec3<f32>(0.5, 0.6, 0.7));
        }
        case 4u: {
            // Sunset - Warm oranges to purples
            return cosine_palette(t_animated,
                vec3<f32>(0.5, 0.5, 0.5),
                vec3<f32>(0.5, 0.5, 0.5),
                vec3<f32>(1.0, 1.0, 1.0),
                vec3<f32>(0.0, 0.1, 0.2));
        }
        case 5u: {
            // Forest - Greens and earth tones
            return cosine_palette(t_animated,
                vec3<f32>(0.5, 0.5, 0.5),
                vec3<f32>(0.5, 0.5, 0.5),
                vec3<f32>(1.0, 1.0, 0.5),
                vec3<f32>(0.3, 0.2, 0.2));
        }
        case 6u: {
            // Ocean - Deep blues to cyan
            return cosine_palette(t_animated,
                vec3<f32>(0.5, 0.5, 0.5),
                vec3<f32>(0.5, 0.5, 0.5),
                vec3<f32>(1.0, 1.0, 1.0),
                vec3<f32>(0.6, 0.7, 0.8));
        }
        case 7u: {
            // Grayscale - Simple black to white
            let v = t_animated;
            return vec3<f32>(v, v, v);
        }
        case 8u: {
            // Hot - Black to red to yellow to white
            return cosine_palette(t_animated,
                vec3<f32>(0.5, 0.5, 0.5),
                vec3<f32>(0.5, 0.5, 0.4),
                vec3<f32>(1.0, 1.0, 1.0),
                vec3<f32>(0.0, 0.15, 0.4));
        }
        case 9u: {
            // Cool - Cyan to magenta gradient
            return cosine_palette(t_animated,
                vec3<f32>(0.5, 0.5, 0.5),
                vec3<f32>(0.5, 0.5, 0.5),
                vec3<f32>(1.0, 1.0, 1.0),
                vec3<f32>(0.8, 0.9, 0.3));
        }
        case 10u: {
            // Plasma - Purple to orange (scientific visualization)
            return cosine_palette(t_animated,
                vec3<f32>(0.5, 0.5, 0.5),
                vec3<f32>(0.5, 0.5, 0.5),
                vec3<f32>(1.0, 1.0, 1.0),
                vec3<f32>(0.8, 0.9, 0.1));
        }
        case 11u: {
            // Viridis - Perceptually uniform (scientific visualization)
            return cosine_palette(t_animated,
                vec3<f32>(0.5, 0.5, 0.5),
                vec3<f32>(0.4, 0.5, 0.4),
                vec3<f32>(0.8, 0.8, 0.5),
                vec3<f32>(0.7, 0.5, 0.0));
        }
        case 12u: {
            // Custom - User-defined cosine palette parameters
            return cosine_palette(t_animated,
                uniforms.procedural_brightness.rgb,
                uniforms.procedural_contrast.rgb,
                uniforms.procedural_frequency.rgb,
                uniforms.procedural_phase.rgb);
        }
        default: {
            // Fallback to static palette if unknown type
            return uniforms.palette[0].rgb;
        }
    }
}

// ============================================================================
// LOD Debug Visualization
// ============================================================================

fn get_lod_debug_color(distance: f32) -> vec3<f32> {
    // Return color based on LOD zone distance thresholds
    if (distance < uniforms.lod_zone1) {
        // Ultra quality - Green
        return vec3<f32>(0.0, 1.0, 0.0);
    } else if (distance < uniforms.lod_zone2) {
        // High quality - Light Green
        return vec3<f32>(0.4, 1.0, 0.4);
    } else if (distance < uniforms.lod_zone3) {
        // Medium quality - Orange
        return vec3<f32>(1.0, 0.6, 0.0);
    } else {
        // Low quality - Red
        return vec3<f32>(1.0, 0.0, 0.0);
    }
}
