// Attractor Display Shader
//
// Renders the accumulation texture with log scaling for contrast.
// This shader reads from the accumulated hit count texture and
// applies color mapping based on the selected palette.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct DisplayUniforms {
    // Palette colors (5 colors)
    palette0: vec4<f32>,
    palette1: vec4<f32>,
    palette2: vec4<f32>,
    palette3: vec4<f32>,
    palette4: vec4<f32>,

    // Display parameters
    log_scale: f32,         // Scale factor for log mapping
    max_value: f32,         // Maximum expected hit count (for normalization)
    palette_offset: f32,    // Palette cycling offset
    gamma: f32,             // Gamma correction

    // Additional parameters
    invert: u32,            // Invert colors (0 or 1)
    _padding: vec3<u32>,
}

@group(0) @binding(0)
var accumulation_texture: texture_2d<f32>;

@group(0) @binding(1)
var tex_sampler: sampler;

@group(1) @binding(0)
var<uniform> uniforms: DisplayUniforms;

@vertex
fn vs_main(@location(0) position: vec2<f32>, @location(1) tex_coords: vec2<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.tex_coords = tex_coords;
    return output;
}

// Sample palette at position t in [0, 1]
fn sample_palette(t: f32) -> vec3<f32> {
    let t_clamped = clamp(t, 0.0, 1.0);
    let scaled = t_clamped * 4.0;
    let idx = u32(floor(scaled));
    let frac = fract(scaled);

    var c0: vec3<f32>;
    var c1: vec3<f32>;

    switch idx {
        case 0u: {
            c0 = uniforms.palette0.rgb;
            c1 = uniforms.palette1.rgb;
        }
        case 1u: {
            c0 = uniforms.palette1.rgb;
            c1 = uniforms.palette2.rgb;
        }
        case 2u: {
            c0 = uniforms.palette2.rgb;
            c1 = uniforms.palette3.rgb;
        }
        case 3u: {
            c0 = uniforms.palette3.rgb;
            c1 = uniforms.palette4.rgb;
        }
        default: {
            c0 = uniforms.palette4.rgb;
            c1 = uniforms.palette4.rgb;
        }
    }

    return mix(c0, c1, frac);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample accumulation texture
    let accumulated = textureSample(accumulation_texture, tex_sampler, input.tex_coords);

    // Get hit count from R channel
    let hit_count = accumulated.r;

    // If no hits, return black
    if (hit_count < 0.5) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // Apply log scaling for better contrast across large dynamic range
    // log(1 + x) / log(1 + max) gives values in [0, 1]
    let log_value = log(1.0 + hit_count * uniforms.log_scale) / log(1.0 + uniforms.max_value * uniforms.log_scale);

    // Apply gamma correction
    let adjusted = pow(log_value, uniforms.gamma);

    // Apply palette offset for cycling
    let palette_t = fract(adjusted + uniforms.palette_offset);

    // Sample color from palette
    var color = sample_palette(palette_t);

    // Apply gamma correction to final color
    color = pow(color, vec3<f32>(1.0 / 2.2));

    // Optionally invert
    if (uniforms.invert == 1u) {
        color = vec3<f32>(1.0) - color;
    }

    return vec4<f32>(color, 1.0);
}
