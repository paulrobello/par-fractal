// Post-processing shaders for multi-pass rendering
// Handles: bloom extraction, Gaussian blur, compositing, FXAA

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@group(0) @binding(0)
var t_scene: texture_2d<f32>;
@group(0) @binding(1)
var s_scene: sampler;

// Vertex shader - simple fullscreen quad
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coords = input.tex_coords;
    return output;
}

// ============================================================================
// Bloom Extract Pass - Extract bright pixels above threshold
// ============================================================================

struct BloomUniforms {
    threshold: f32,
    intensity: f32,
    _padding: vec2<f32>,
}

@group(1) @binding(0)
var<uniform> bloom_params: BloomUniforms;

@fragment
fn fs_bloom_extract(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_scene, s_scene, input.tex_coords).rgb;
    let luminance = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));

    if (luminance > bloom_params.threshold) {
        // Extract bright pixels with smooth falloff
        let bloom_amount = pow((luminance - bloom_params.threshold) / (1.0 - bloom_params.threshold + 0.001), 0.3);
        // Make extraction more aggressive - multiply by 2.0
        return vec4<f32>(color * bloom_amount * 2.0, 1.0);
    }

    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

// ============================================================================
// Gaussian Blur Pass - Separable blur (horizontal or vertical)
// ============================================================================

struct BlurUniforms {
    direction: vec2<f32>,  // (1,0) for horizontal, (0,1) for vertical
    _padding: vec2<f32>,
}

@group(1) @binding(0)
var<uniform> blur_params: BlurUniforms;

// 9-tap Gaussian blur weights
const BLUR_WEIGHTS = array<f32, 5>(
    0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216
);

@fragment
fn fs_blur(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(t_scene));
    let tex_offset = 1.0 / tex_size;

    var result = textureSample(t_scene, s_scene, input.tex_coords).rgb * BLUR_WEIGHTS[0];

    // Manually unroll the loop to avoid dynamic array indexing
    // Use 2x spread for wider bloom
    let offset1 = blur_params.direction * tex_offset * 2.0;
    result += textureSample(t_scene, s_scene, input.tex_coords + offset1).rgb * BLUR_WEIGHTS[1];
    result += textureSample(t_scene, s_scene, input.tex_coords - offset1).rgb * BLUR_WEIGHTS[1];

    let offset2 = blur_params.direction * tex_offset * 4.0;
    result += textureSample(t_scene, s_scene, input.tex_coords + offset2).rgb * BLUR_WEIGHTS[2];
    result += textureSample(t_scene, s_scene, input.tex_coords - offset2).rgb * BLUR_WEIGHTS[2];

    let offset3 = blur_params.direction * tex_offset * 6.0;
    result += textureSample(t_scene, s_scene, input.tex_coords + offset3).rgb * BLUR_WEIGHTS[3];
    result += textureSample(t_scene, s_scene, input.tex_coords - offset3).rgb * BLUR_WEIGHTS[3];

    let offset4 = blur_params.direction * tex_offset * 8.0;
    result += textureSample(t_scene, s_scene, input.tex_coords + offset4).rgb * BLUR_WEIGHTS[4];
    result += textureSample(t_scene, s_scene, input.tex_coords - offset4).rgb * BLUR_WEIGHTS[4];

    return vec4<f32>(result, 1.0);
}

// ============================================================================
// Composite Pass - Combine scene + bloom + apply color grading + vignette
// ============================================================================

@group(0) @binding(2)
var t_bloom: texture_2d<f32>;
@group(0) @binding(3)
var s_bloom: sampler;

struct PostProcessUniforms {
    // Color grading
    brightness: f32,        // offset 0
    contrast: f32,          // offset 4
    saturation: f32,        // offset 8
    hue_shift: f32,         // offset 12

    // Vignette
    vignette_enabled: u32,      // offset 16
    vignette_intensity: f32,    // offset 20
    vignette_radius: f32,       // offset 24
    _padding1: f32,             // offset 28

    // Bloom
    bloom_enabled: u32,         // offset 32
    bloom_intensity: f32,       // offset 36
    _padding2: vec2<f32>,       // offset 40

    _padding3: vec4<f32>,       // offset 48
}

@group(1) @binding(0)
var<uniform> postfx: PostProcessUniforms;

// RGB to HSV conversion
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

// HSV to RGB conversion
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

@fragment
fn fs_composite(input: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(t_scene, s_scene, input.tex_coords).rgb;

    // Apply color grading FIRST (before bloom, so we don't clamp it)
    // Brightness
    color = color * postfx.brightness;

    // Contrast
    color = ((color - 0.5) * postfx.contrast) + 0.5;

    // Saturation and hue shift
    if (postfx.saturation != 1.0 || postfx.hue_shift != 0.0) {
        var hsv = rgb_to_hsv(color);
        hsv.y = hsv.y * postfx.saturation;
        hsv.x = fract(hsv.x + postfx.hue_shift);
        color = hsv_to_rgb(hsv);
    }

    // Clamp after color grading but before bloom
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    // Add bloom AFTER color grading and clamping (so bloom can exceed 1.0)
    if (postfx.bloom_enabled == 1u) {
        let bloom = textureSample(t_bloom, s_bloom, input.tex_coords).rgb;
        color = color + bloom * postfx.bloom_intensity;
    }

    // Apply vignette
    if (postfx.vignette_enabled == 1u) {
        let center = vec2<f32>(0.5, 0.5);
        let dist = length(input.tex_coords - center);
        let vignette = smoothstep(postfx.vignette_radius, postfx.vignette_radius * 0.5, dist);
        let factor = mix(1.0 - postfx.vignette_intensity, 1.0, vignette);
        color = color * factor;
    }

    return vec4<f32>(color, 1.0);
}

// ============================================================================
// FXAA Pass - Fast Approximate Anti-Aliasing
// ============================================================================

const FXAA_SPAN_MAX = 8.0;
const FXAA_REDUCE_MUL = 1.0 / 8.0;
const FXAA_REDUCE_MIN = 1.0 / 128.0;

@fragment
fn fs_fxaa(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(t_scene));
    let inv_tex_size = 1.0 / tex_size;

    let rgbNW = textureSample(t_scene, s_scene, input.tex_coords + vec2<f32>(-1.0, -1.0) * inv_tex_size).rgb;
    let rgbNE = textureSample(t_scene, s_scene, input.tex_coords + vec2<f32>(1.0, -1.0) * inv_tex_size).rgb;
    let rgbSW = textureSample(t_scene, s_scene, input.tex_coords + vec2<f32>(-1.0, 1.0) * inv_tex_size).rgb;
    let rgbSE = textureSample(t_scene, s_scene, input.tex_coords + vec2<f32>(1.0, 1.0) * inv_tex_size).rgb;
    let rgbM = textureSample(t_scene, s_scene, input.tex_coords).rgb;

    let luma = vec3<f32>(0.299, 0.587, 0.114);
    let lumaNW = dot(rgbNW, luma);
    let lumaNE = dot(rgbNE, luma);
    let lumaSW = dot(rgbSW, luma);
    let lumaSE = dot(rgbSE, luma);
    let lumaM = dot(rgbM, luma);

    let lumaMin = min(lumaM, min(min(lumaNW, lumaNE), min(lumaSW, lumaSE)));
    let lumaMax = max(lumaM, max(max(lumaNW, lumaNE), max(lumaSW, lumaSE)));

    var dir: vec2<f32>;
    dir.x = -((lumaNW + lumaNE) - (lumaSW + lumaSE));
    dir.y = ((lumaNW + lumaSW) - (lumaNE + lumaSE));

    let dirReduce = max((lumaNW + lumaNE + lumaSW + lumaSE) * (0.25 * FXAA_REDUCE_MUL), FXAA_REDUCE_MIN);
    let rcpDirMin = 1.0 / (min(abs(dir.x), abs(dir.y)) + dirReduce);

    dir = min(vec2<f32>(FXAA_SPAN_MAX), max(vec2<f32>(-FXAA_SPAN_MAX), dir * rcpDirMin)) * inv_tex_size;

    let rgbA = 0.5 * (
        textureSample(t_scene, s_scene, input.tex_coords + dir * (1.0 / 3.0 - 0.5)).rgb +
        textureSample(t_scene, s_scene, input.tex_coords + dir * (2.0 / 3.0 - 0.5)).rgb
    );

    let rgbB = rgbA * 0.5 + 0.25 * (
        textureSample(t_scene, s_scene, input.tex_coords + dir * -0.5).rgb +
        textureSample(t_scene, s_scene, input.tex_coords + dir * 0.5).rgb
    );

    let lumaB = dot(rgbB, luma);

    if ((lumaB < lumaMin) || (lumaB > lumaMax)) {
        return vec4<f32>(rgbA, 1.0);
    } else {
        return vec4<f32>(rgbB, 1.0);
    }
}

// ============================================================================
// Copy/Passthrough - Simple texture copy to screen
// ============================================================================

@fragment
fn fs_copy(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_scene, s_scene, input.tex_coords);
}

// ============================================================================
// Accumulation Display - Visualize accumulated hit counts with log scaling
// ============================================================================

struct AccumulationDisplayUniforms {
    log_scale: f32,
    gamma: f32,
    _padding: vec2<f32>,
}

// This shader uses a separate bind group with only the uint accumulation texture
@group(0) @binding(0)
var t_accum: texture_2d<u32>;

@group(1) @binding(0)
var<uniform> accum_uniforms: AccumulationDisplayUniforms;

@fragment
fn fs_accumulation_display(input: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate texture coordinates as integers
    let tex_size = textureDimensions(t_accum);
    let coord = vec2<i32>(
        i32(input.tex_coords.x * f32(tex_size.x)),
        i32(input.tex_coords.y * f32(tex_size.y))
    );

    // Load hit count (R32Uint format - single u32 value)
    let accumulated = textureLoad(t_accum, coord, 0);
    let hit_count = f32(accumulated.r);

    // If no hits, return black
    if (hit_count < 0.5) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // Apply log scaling for better contrast across large dynamic range
    // log_scale from uniforms controls the compression (higher = more compressed)
    let max_value = 5000.0 / accum_uniforms.log_scale;
    let log_value = log(1.0 + hit_count) / log(1.0 + max_value);

    // Apply gamma correction (from uniforms)
    let adjusted = pow(clamp(log_value, 0.0, 1.0), accum_uniforms.gamma);

    // Classic "fire" palette - similar to classic fractal viewers
    // Goes from black -> dark red -> red -> orange -> yellow -> white
    var color: vec3<f32>;
    let t = adjusted;

    if (t < 0.2) {
        // Black to dark purple/blue
        let s = t * 5.0;
        color = mix(vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(0.1, 0.0, 0.3), s);
    } else if (t < 0.4) {
        // Dark purple to magenta
        let s = (t - 0.2) * 5.0;
        color = mix(vec3<f32>(0.1, 0.0, 0.3), vec3<f32>(0.6, 0.0, 0.4), s);
    } else if (t < 0.6) {
        // Magenta to orange
        let s = (t - 0.4) * 5.0;
        color = mix(vec3<f32>(0.6, 0.0, 0.4), vec3<f32>(1.0, 0.4, 0.0), s);
    } else if (t < 0.8) {
        // Orange to yellow
        let s = (t - 0.6) * 5.0;
        color = mix(vec3<f32>(1.0, 0.4, 0.0), vec3<f32>(1.0, 0.9, 0.2), s);
    } else {
        // Yellow to white
        let s = (t - 0.8) * 5.0;
        color = mix(vec3<f32>(1.0, 0.9, 0.2), vec3<f32>(1.0, 1.0, 1.0), s);
    }

    return vec4<f32>(color, 1.0);
}
