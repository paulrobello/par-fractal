struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding1: f32,

    center: vec2<f32>,
    zoom: f32,
    max_iterations: u32,

    julia_c: vec2<f32>,
    fractal_type: u32,
    render_mode: u32,

    power: f32,
    max_steps: u32,
    min_distance: f32,
    fractal_scale: f32,
    fractal_fold: f32,
    fractal_min_radius: f32,
    _padding2: vec2<f32>,

    palette: array<vec4<f32>, 8>,

    ambient_occlusion: u32,
    soft_shadows: u32,
    depth_of_field: u32,
    shading_model: u32,
    color_mode: u32,
    orbit_trap_scale: f32,
    palette_offset: f32,
    channel_r: u32,
    channel_g: u32,
    channel_b: u32,
    _padding_color_0: u32,

    roughness: f32,
    metallic: f32,
    // WGSL adds 12 bytes implicit padding here to align vec3 to offset 352
    _padding_before_albedo: vec3<f32>, // vec3 field aligned to 16-byte boundary
    // WGSL adds 4 bytes implicit padding here to align vec3 to offset 368
    albedo: vec3<f32>,
    _padding3: f32,

    dof_focal_length: f32,
    dof_aperture: f32,
    dof_samples: u32,
    time: f32,
    light_intensity: f32,
    ambient_light: f32,
    ao_intensity: f32,
    ao_step_size: f32,
    shadow_softness: f32,
    shadow_max_distance: f32,
    shadow_samples: u32,
    shadow_step_factor: f32,

    // Light direction
    light_azimuth: f32,     // Horizontal angle in degrees (0-360)
    light_elevation: f32,   // Vertical angle in degrees (5-90)
    _padding_light: vec2<f32>, // Maintain 16-byte alignment

    show_floor: u32,
    floor_height: f32,
    _padding_floor: vec2<f32>,
    floor_color1: vec3<f32>,
    _padding_floor1: f32,
    floor_color2: vec3<f32>,
    floor_reflections: u32,
    floor_reflection_strength: f32,
    _padding_floor3_align_0: f32,
    _padding_floor3_align_1: f32,
    _padding_floor3_align_2: f32,
    _padding_floor3_0: f32,
    _padding_floor3_1: f32,
    _padding_floor3_2: f32,

    use_adaptive_step: u32,
    fixed_step_size: f32,
    step_multiplier: f32,
    max_distance: f32,

    fog_enabled: u32,
    fog_mode: u32,  // 0: Linear, 1: Exponential, 2: Quadratic
    fog_density: f32,
    _padding_fog: f32,
    _padding_fog_vec3_align: f32,  // Align fog_color to 16-byte boundary
    fog_color: vec3<f32>,
    _padding_fog_color: f32,

    // Post-processing
    brightness: f32,
    contrast: f32,
    saturation: f32,
    hue_shift: f32,
    vignette_enabled: u32,
    vignette_intensity: f32,
    vignette_radius: f32,
    bloom_enabled: u32,
    bloom_threshold: f32,
    bloom_intensity: f32,
    bloom_radius: f32,
    fxaa_enabled: u32,

    // High-precision center for deep zoom (double-float emulation)
    // Each coordinate stored as (hi, lo) pair where value = hi + lo
    center_hi: vec2<f32>,   // High part of center (x, y)
    center_lo: vec2<f32>,   // Low part of center (x, y)
    high_precision: u32,    // Flag: 1 = use high precision
    _hp_padding: vec3<f32>, // Padding to maintain alignment

    // LOD debug visualization
    lod_debug_enabled: u32,  // Flag: 1 = show LOD zones as colors
    lod_zone1: f32,          // Distance threshold: Ultra -> High
    lod_zone2: f32,          // Distance threshold: High -> Medium
    lod_zone3: f32,          // Distance threshold: Medium -> Low

    // Aspect ratio stored in a vec4 slot to guarantee 16-byte alignment
    aspect_ratio: vec4<f32>, // .x = width/height, others unused

    // Procedural palette parameters
    procedural_palette_type: u32, // 0=None (use static), 1=Firestrm, 2=Rainbow, etc.
    _padding_proc_pal_0: u32,
    _padding_proc_pal_1: u32,
    _padding_proc_pal_2: u32,
    // Custom procedural palette: color(t) = brightness + contrast * cos(2π * (frequency * t + phase))
    procedural_brightness: vec4<f32>, // [r, g, b, _]
    procedural_contrast: vec4<f32>,   // [r, g, b, _]
    procedural_frequency: vec4<f32>,  // [r, g, b, _]
    procedural_phase: vec4<f32>,      // [r, g, b, _]

    // Padding to align struct to 864 bytes (54 × 16)
    _padding_end: array<vec4<f32>, 2>,  // 32 bytes
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.position;
    return output;
}

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

// ============================================================================
// Double-Float Arithmetic (emulated f64 using two f32)
// A double-float number is stored as (hi, lo) where value = hi + lo
// This provides ~14 decimal digits of precision vs ~7 for f32
// ============================================================================

// Double-float structure for complex numbers
struct df2 {
    hi: vec2<f32>,  // High parts (real, imag)
    lo: vec2<f32>,  // Low parts (real, imag)
}

// Quick two-sum: a + b = s + e, where s = fl(a+b) and e = error
fn two_sum(a: f32, b: f32) -> vec2<f32> {
    let s = a + b;
    let v = s - a;
    let e = (a - (s - v)) + (b - v);
    return vec2<f32>(s, e);
}

// Double-float addition: (a_hi, a_lo) + (b_hi, b_lo)
fn df_add(a_hi: f32, a_lo: f32, b_hi: f32, b_lo: f32) -> vec2<f32> {
    let s = two_sum(a_hi, b_hi);
    let t = two_sum(a_lo, b_lo);
    var c = s.y + t.x;
    let v = two_sum(s.x, c);
    c = t.y + v.y;
    return vec2<f32>(v.x + c, 0.0);  // Simplified: just return (sum, 0)
}

// More accurate df_add with proper error propagation
fn df_add_full(a_hi: f32, a_lo: f32, b_hi: f32, b_lo: f32) -> vec2<f32> {
    let s1 = two_sum(a_hi, b_hi);
    let s2 = two_sum(a_lo, b_lo);
    let s3 = two_sum(s1.x, s1.y + s2.x);
    return vec2<f32>(s3.x, s3.y + s2.y);
}

// Quick two-product: a * b = p + e (using FMA if available, otherwise split)
fn two_prod(a: f32, b: f32) -> vec2<f32> {
    let p = a * b;
    let e = fma(a, b, -p);
    return vec2<f32>(p, e);
}

// Double-float multiplication: (a_hi, a_lo) * (b_hi, b_lo)
fn df_mul(a_hi: f32, a_lo: f32, b_hi: f32, b_lo: f32) -> vec2<f32> {
    let p = two_prod(a_hi, b_hi);
    let e = a_hi * b_lo + a_lo * b_hi;
    return two_sum(p.x, p.y + e);
}

// Double-float subtraction
fn df_sub(a_hi: f32, a_lo: f32, b_hi: f32, b_lo: f32) -> vec2<f32> {
    return df_add_full(a_hi, a_lo, -b_hi, -b_lo);
}

// Complex double-float addition: a + b
fn df2_add(a: df2, b: df2) -> df2 {
    let r = df_add_full(a.hi.x, a.lo.x, b.hi.x, b.lo.x);
    let i = df_add_full(a.hi.y, a.lo.y, b.hi.y, b.lo.y);
    return df2(vec2<f32>(r.x, i.x), vec2<f32>(r.y, i.y));
}

// Complex double-float multiplication: a * b
// (a + bi)(c + di) = (ac - bd) + (ad + bc)i
fn df2_mul(a: df2, b: df2) -> df2 {
    // Real part: a.real * b.real - a.imag * b.imag
    let ac = df_mul(a.hi.x, a.lo.x, b.hi.x, b.lo.x);
    let bd = df_mul(a.hi.y, a.lo.y, b.hi.y, b.lo.y);
    let real = df_sub(ac.x, ac.y, bd.x, bd.y);

    // Imag part: a.real * b.imag + a.imag * b.real
    let ad = df_mul(a.hi.x, a.lo.x, b.hi.y, b.lo.y);
    let bc = df_mul(a.hi.y, a.lo.y, b.hi.x, b.lo.x);
    let imag = df_add_full(ad.x, ad.y, bc.x, bc.y);

    return df2(vec2<f32>(real.x, imag.x), vec2<f32>(real.y, imag.y));
}

// Complex double-float square: z^2
fn df2_square(z: df2) -> df2 {
    // (a + bi)^2 = (a^2 - b^2) + 2abi
    let a2 = df_mul(z.hi.x, z.lo.x, z.hi.x, z.lo.x);
    let b2 = df_mul(z.hi.y, z.lo.y, z.hi.y, z.lo.y);
    let real = df_sub(a2.x, a2.y, b2.x, b2.y);

    let ab = df_mul(z.hi.x, z.lo.x, z.hi.y, z.lo.y);
    let imag = vec2<f32>(ab.x * 2.0, ab.y * 2.0);  // 2 * ab

    return df2(vec2<f32>(real.x, imag.x), vec2<f32>(real.y, imag.y));
}

// Get magnitude squared of double-float complex (returns f32, sufficient for escape test)
fn df2_mag_sq(z: df2) -> f32 {
    return z.hi.x * z.hi.x + z.hi.y * z.hi.y;
}

// High-precision Mandelbrot
fn mandelbrot_hp(c_hi: vec2<f32>, c_lo: vec2<f32>) -> f32 {
    var z = df2(vec2<f32>(0.0, 0.0), vec2<f32>(0.0, 0.0));
    let c = df2(c_hi, c_lo);
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (df2_mag_sq(z) > 4.0) {
            break;
        }

        // z = z^2 + c
        z = df2_add(df2_square(z), c);
        iteration = i;
    }

    if (iteration >= uniforms.max_iterations - 1u) {
        return 0.0;
    }

    // Smooth coloring using high part only (sufficient precision)
    let mag_sq = df2_mag_sq(z);
    let log_zn = log(mag_sq) / 2.0;
    let nu = log(log_zn / log(2.0)) / log(2.0);
    return (f32(iteration) + 1.0 - nu) / f32(uniforms.max_iterations);
}

// High-precision Julia
fn julia_hp(z_hi: vec2<f32>, z_lo: vec2<f32>) -> f32 {
    var z = df2(z_hi, z_lo);
    // Julia C parameter - use standard precision (it's a constant, not zoomed)
    let c = df2(uniforms.julia_c, vec2<f32>(0.0, 0.0));
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (df2_mag_sq(z) > 4.0) {
            break;
        }

        z = df2_add(df2_square(z), c);
        iteration = i;
    }

    if (iteration >= uniforms.max_iterations - 1u) {
        return 0.0;
    }

    let mag_sq = df2_mag_sq(z);
    let log_zn = log(mag_sq) / 2.0;
    let nu = log(log_zn / log(2.0)) / log(2.0);
    return (f32(iteration) + 1.0 - nu) / f32(uniforms.max_iterations);
}

// High-precision Burning Ship
fn burning_ship_hp(c_hi: vec2<f32>, c_lo: vec2<f32>) -> f32 {
    var z = df2(vec2<f32>(0.0, 0.0), vec2<f32>(0.0, 0.0));
    let c = df2(c_hi, c_lo);
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (df2_mag_sq(z) > 4.0) {
            break;
        }

        // Burning Ship: z = (|Re(z)| + i|Im(z)|)^2 + c
        // Take absolute value of both components (using hi part, lo follows sign)
        let z_abs = df2(abs(z.hi), abs(z.lo) * sign(z.hi));
        z = df2_add(df2_square(z_abs), c);
        iteration = i;
    }

    if (iteration >= uniforms.max_iterations - 1u) {
        return 0.0;
    }

    let mag_sq = df2_mag_sq(z);
    let log_zn = log(mag_sq) / 2.0;
    let nu = log(log_zn / log(2.0)) / log(2.0);
    return (f32(iteration) + 1.0 - nu) / f32(uniforms.max_iterations);
}

// High-precision Tricorn
fn tricorn_hp(c_hi: vec2<f32>, c_lo: vec2<f32>) -> f32 {
    var z = df2(vec2<f32>(0.0, 0.0), vec2<f32>(0.0, 0.0));
    let c = df2(c_hi, c_lo);
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (df2_mag_sq(z) > 4.0) {
            break;
        }

        // Tricorn: z = conj(z)^2 + c
        // conj(z) = (re, -im)
        let z_conj = df2(vec2<f32>(z.hi.x, -z.hi.y), vec2<f32>(z.lo.x, -z.lo.y));
        z = df2_add(df2_square(z_conj), c);
        iteration = i;
    }

    if (iteration >= uniforms.max_iterations - 1u) {
        return 0.0;
    }

    let mag_sq = df2_mag_sq(z);
    let log_zn = log(mag_sq) / 2.0;
    let nu = log(log_zn / log(2.0)) / log(2.0);
    return (f32(iteration) + 1.0 - nu) / f32(uniforms.max_iterations);
}

// ============================================================================
// 2D Fractals (Standard Precision)
// ============================================================================

fn mandelbrot(c: vec2<f32>) -> f32 {
    var z = vec2<f32>(0.0, 0.0);
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > 4.0) {
            break;
        }

        let z_real = z.x * z.x - z.y * z.y + c.x;
        let z_imag = 2.0 * z.x * z.y + c.y;
        z = vec2<f32>(z_real, z_imag);
        iteration = i;
    }

    if (iteration >= uniforms.max_iterations - 1u) {
        return 0.0;
    }

    // Smooth coloring
    let log_zn = log(dot(z, z)) / 2.0;
    let nu = log(log_zn / log(2.0)) / log(2.0);
    return (f32(iteration) + 1.0 - nu) / f32(uniforms.max_iterations);
}

fn julia(z_start: vec2<f32>) -> f32 {
    var z = z_start;
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > 4.0) {
            break;
        }

        let z_real = z.x * z.x - z.y * z.y + uniforms.julia_c.x;
        let z_imag = 2.0 * z.x * z.y + uniforms.julia_c.y;
        z = vec2<f32>(z_real, z_imag);
        iteration = i;
    }

    if (iteration >= uniforms.max_iterations - 1u) {
        return 0.0;
    }

    let log_zn = log(dot(z, z)) / 2.0;
    let nu = log(log_zn / log(2.0)) / log(2.0);
    return (f32(iteration) + 1.0 - nu) / f32(uniforms.max_iterations);
}

fn sierpinski(coord: vec2<f32>) -> f32 {
    // Sierpinski carpet - map to [0, 1] range
    var p = coord * 0.5 + 0.5;

    // Check if outside main square
    if (p.x < 0.0 || p.x >= 1.0 || p.y < 0.0 || p.y >= 1.0) {
        return 0.0;
    }

    var scale = 1.0;
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        iteration = i;

        // Scale coordinates to current level
        let scaled_x = p.x * scale;
        let scaled_y = p.y * scale;

        // Get position within current 3x3 grid (modulo 3)
        let cell_x = floor(fract(scaled_x / 3.0) * 3.0);
        let cell_y = floor(fract(scaled_y / 3.0) * 3.0);

        // Check if in the center cell (removed region)
        if (cell_x == 1.0 && cell_y == 1.0) {
            return 0.0; // Point is removed
        }

        // Move to next subdivision level
        scale = scale * 3.0;

        // Stop if we've reached pixel precision
        if (scale > 1000000.0) {
            break;
        }
    }

    // Point is in the Sierpinski carpet
    return f32(iteration) / f32(uniforms.max_iterations);
}

// High-precision version for deep zooms
fn sierpinski_hp(coord_hi: vec2<f32>, coord_lo: vec2<f32>) -> f32 {
    // Recombine hi+lo for best precision at large zooms
    let coord = vec2<f32>(coord_hi.x + coord_lo.x, coord_hi.y + coord_lo.y);
    return sierpinski(coord);
}

// Sierpinski Gasket (Triangle) using barycentric subdivision
fn sierpinski_triangle(coord: vec2<f32>) -> f32 {
    // Map coord from [-1,1] to [0,1] range
    let p = coord * 0.5 + 0.5;

    // Equilateral triangle vertices
    let v0 = vec2<f32>(0.0, 0.0);       // Bottom-left
    let v1 = vec2<f32>(1.0, 0.0);       // Bottom-right
    let v2 = vec2<f32>(0.5, sqrt(3.0) * 0.5);  // Top

    // Compute barycentric coordinates
    let v0v1 = v1 - v0;
    let v0v2 = v2 - v0;
    let v0p = p - v0;

    let d00 = dot(v0v1, v0v1);
    let d01 = dot(v0v1, v0v2);
    let d11 = dot(v0v2, v0v2);
    let d20 = dot(v0p, v0v1);
    let d21 = dot(v0p, v0v2);

    let denom = d00 * d11 - d01 * d01;
    let bv = (d11 * d20 - d01 * d21) / denom;
    let bw = (d00 * d21 - d01 * d20) / denom;
    let bu = 1.0 - bv - bw;

    var bary = vec3<f32>(bu, bv, bw);  // (u=v0 weight, v=v1 weight, w=v2 weight)

    // Check if outside the main triangle
    if (bary.x < 0.0 || bary.y < 0.0 || bary.z < 0.0) {
        return 0.0;  // Outside triangle - render as black
    }

    // Iterate using barycentric subdivision
    // Color holes based on iteration depth (like Mandelbrot escape-time)
    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        // Check if in center hole (all coords < 0.5)
        if (bary.x < 0.5 && bary.y < 0.5 && bary.z < 0.5) {
            // In a hole at iteration i - color based on depth
            // Add smooth coloring based on how close to edge of hole
            let max_bary = max(max(bary.x, bary.y), bary.z);
            let smooth_val = max_bary / 0.5;  // 0 at center, 1 at edge
            return (f32(i) + smooth_val) / f32(uniforms.max_iterations);
        }

        // Transform to the appropriate corner sub-triangle
        if (bary.x >= 0.5) {
            // Bottom-left corner sub-triangle (near v0)
            bary = vec3<f32>(2.0 * bary.x - 1.0, 2.0 * bary.y, 2.0 * bary.z);
        } else if (bary.y >= 0.5) {
            // Bottom-right corner sub-triangle (near v1)
            bary = vec3<f32>(2.0 * bary.x, 2.0 * bary.y - 1.0, 2.0 * bary.z);
        } else {
            // Top corner sub-triangle (near v2, since bary.z >= 0.5)
            bary = vec3<f32>(2.0 * bary.x, 2.0 * bary.y, 2.0 * bary.z - 1.0);
        }
    }

    // Point survived all iterations - it's in the fractal (like points in Mandelbrot set)
    return 0.0;
}

// High-precision version for extreme zooms
fn sierpinski_triangle_hp(coord_hi: vec2<f32>, coord_lo: vec2<f32>) -> f32 {
    let coord = vec2<f32>(coord_hi.x + coord_lo.x, coord_hi.y + coord_lo.y);
    return sierpinski_triangle(coord);
}

fn burning_ship(c: vec2<f32>) -> f32 {
    var z = vec2<f32>(0.0, 0.0);
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > 4.0) {
            break;
        }

        // Burning Ship: z = (|Re(z)| + i|Im(z)|)^2 + c
        let z_abs = abs(z);
        let z_real = z_abs.x * z_abs.x - z_abs.y * z_abs.y + c.x;
        let z_imag = 2.0 * z_abs.x * z_abs.y + c.y;
        z = vec2<f32>(z_real, z_imag);
        iteration = i;
    }

    if (iteration >= uniforms.max_iterations - 1u) {
        return 0.0;
    }

    let log_zn = log(dot(z, z)) / 2.0;
    let nu = log(log_zn / log(2.0)) / log(2.0);
    return (f32(iteration) + 1.0 - nu) / f32(uniforms.max_iterations);
}

fn tricorn(c: vec2<f32>) -> f32 {
    var z = vec2<f32>(0.0, 0.0);
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > 4.0) {
            break;
        }

        // Tricorn (Mandelbar): z = conj(z)^2 + c
        let z_conj = vec2<f32>(z.x, -z.y);
        let z_real = z_conj.x * z_conj.x - z_conj.y * z_conj.y + c.x;
        let z_imag = 2.0 * z_conj.x * z_conj.y + c.y;
        z = vec2<f32>(z_real, z_imag);
        iteration = i;
    }

    if (iteration >= uniforms.max_iterations - 1u) {
        return 0.0;
    }

    let log_zn = log(dot(z, z)) / 2.0;
    let nu = log(log_zn / log(2.0)) / log(2.0);
    return (f32(iteration) + 1.0 - nu) / f32(uniforms.max_iterations);
}

fn phoenix(c: vec2<f32>) -> f32 {
    var z = vec2<f32>(0.0, 0.0);
    var z_prev = vec2<f32>(0.0, 0.0);
    var iteration = 0u;

    // Phoenix parameters
    let p = vec2<f32>(0.5667, 0.0);

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > 4.0) {
            break;
        }

        // Phoenix: z = z^2 + c + p*z_prev
        let z_real = z.x * z.x - z.y * z.y + c.x + p.x * z_prev.x - p.y * z_prev.y;
        let z_imag = 2.0 * z.x * z.y + c.y + p.x * z_prev.y + p.y * z_prev.x;
        z_prev = z;
        z = vec2<f32>(z_real, z_imag);
        iteration = i;
    }

    if (iteration >= uniforms.max_iterations - 1u) {
        return 0.0;
    }

    let log_zn = log(dot(z, z)) / 2.0;
    let nu = log(log_zn / log(2.0)) / log(2.0);
    return (f32(iteration) + 1.0 - nu) / f32(uniforms.max_iterations);
}

fn celtic(c: vec2<f32>) -> f32 {
    var z = vec2<f32>(0.0, 0.0);
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > 4.0) {
            break;
        }

        // Celtic: z = (|Re(z^2)| + i*Im(z^2)) + c
        let z_sq_real = z.x * z.x - z.y * z.y;
        let z_sq_imag = 2.0 * z.x * z.y;
        let z_real = abs(z_sq_real) + c.x;
        let z_imag = z_sq_imag + c.y;
        z = vec2<f32>(z_real, z_imag);
        iteration = i;
    }

    if (iteration >= uniforms.max_iterations - 1u) {
        return 0.0;
    }

    let log_zn = log(dot(z, z)) / 2.0;
    let nu = log(log_zn / log(2.0)) / log(2.0);
    return (f32(iteration) + 1.0 - nu) / f32(uniforms.max_iterations);
}

// Newton fractal - finds roots of z^3 - 1
fn newton_fractal(c: vec2<f32>) -> f32 {
    var z = c;
    var iteration = 0u;
    let tolerance = 0.000001;

    // Roots of z^3 - 1
    let root1 = vec2<f32>(1.0, 0.0);
    let root2 = vec2<f32>(-0.5, 0.866025);
    let root3 = vec2<f32>(-0.5, -0.866025);

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        // Check convergence to roots
        if (distance(z, root1) < tolerance ||
            distance(z, root2) < tolerance ||
            distance(z, root3) < tolerance) {
            break;
        }

        // Newton iteration: z = z - f(z)/f'(z) where f(z) = z^3 - 1
        // f'(z) = 3z^2
        let z2 = vec2<f32>(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y);
        let z3 = vec2<f32>(z2.x * z.x - z2.y * z.y, z2.x * z.y + z2.y * z.x);
        let f_z = vec2<f32>(z3.x - 1.0, z3.y);
        let f_prime = vec2<f32>(3.0 * z2.x, 3.0 * z2.y);

        let denom = dot(f_prime, f_prime);
        if (denom < 0.0001) { break; }

        // Complex division: f_z / f_prime
        let div = vec2<f32>(
            (f_z.x * f_prime.x + f_z.y * f_prime.y) / denom,
            (f_z.y * f_prime.x - f_z.x * f_prime.y) / denom
        );
        z = z - div;
        iteration = i;
    }

    // Color based on which root and iteration count
    let d1 = distance(z, root1);
    let d2 = distance(z, root2);
    let d3 = distance(z, root3);
    var root_offset = 0.0;
    if (d2 < d1 && d2 < d3) { root_offset = 0.33; }
    else if (d3 < d1 && d3 < d2) { root_offset = 0.66; }

    return (f32(iteration) / f32(uniforms.max_iterations)) * 0.5 + root_offset * 0.5;
}

// Lyapunov fractal - stability diagram
fn lyapunov_fractal(c: vec2<f32>) -> f32 {
    let a = c.x * 2.0 + 2.0;  // Map to [0, 4] range
    let b = c.y * 2.0 + 2.0;

    var x = 0.5;
    var lyap = 0.0;
    let sequence_len = 12u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        // Alternate between a and b based on sequence position
        var r: f32;
        if ((i % sequence_len) < (sequence_len / 2u)) {
            r = a;
        } else {
            r = b;
        }

        // Logistic map: x = r * x * (1 - x)
        x = r * x * (1.0 - x);

        // Accumulate Lyapunov exponent
        let deriv = abs(r * (1.0 - 2.0 * x));
        if (deriv > 0.0001) {
            lyap = lyap + log(deriv);
        }
    }

    lyap = lyap / f32(uniforms.max_iterations);

    // Map Lyapunov exponent to color
    if (lyap < 0.0) {
        return 0.5 - clamp(-lyap * 0.5, 0.0, 0.5);  // Stable (convergent)
    } else {
        return 0.5 + clamp(lyap * 0.5, 0.0, 0.5);   // Chaotic
    }
}

// Nova fractal - Newton-Mandelbrot hybrid
fn nova_fractal(c: vec2<f32>) -> f32 {
    var z = c;
    var iteration = 0u;
    let relaxation = 1.0;  // Nova parameter

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > 4.0) {
            break;
        }

        // Nova: z = z - R * f(z)/f'(z) + c where f(z) = z^3 - 1
        let z2 = vec2<f32>(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y);
        let z3 = vec2<f32>(z2.x * z.x - z2.y * z.y, z2.x * z.y + z2.y * z.x);
        let f_z = vec2<f32>(z3.x - 1.0, z3.y);
        let f_prime = vec2<f32>(3.0 * z2.x, 3.0 * z2.y);

        let denom = dot(f_prime, f_prime);
        if (denom < 0.0001) { break; }

        let div = vec2<f32>(
            (f_z.x * f_prime.x + f_z.y * f_prime.y) / denom,
            (f_z.y * f_prime.x - f_z.x * f_prime.y) / denom
        );

        // Nova modification: add c after Newton step
        z = z - relaxation * div + vec2<f32>(uniforms.julia_c.x, uniforms.julia_c.y);
        iteration = i;
    }

    if (iteration >= uniforms.max_iterations - 1u) {
        return 0.0;
    }

    let log_zn = log(dot(z, z)) / 2.0;
    let nu = log(max(log_zn / log(2.0), 0.0001)) / log(2.0);
    return (f32(iteration) + 1.0 - nu) / f32(uniforms.max_iterations);
}

// Magnet Type 1 fractal - z = ((z² + c - 1) / (2z + c - 2))²
fn magnet_fractal(c: vec2<f32>) -> f32 {
    var z = vec2<f32>(0.0, 0.0);
    var iteration = 0u;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (dot(z, z) > 10000.0) {
            break;
        }

        // Check for convergence to fixed point at z = 1
        let dist_to_one = distance(z, vec2<f32>(1.0, 0.0));
        if (dist_to_one < 0.0001) {
            return 0.0;  // Converged to attractor
        }

        // z² = (z.x + iz.y)² = z.x² - z.y² + 2i*z.x*z.y
        let z2 = vec2<f32>(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y);

        // Numerator: z² + c - 1
        let num = vec2<f32>(z2.x + c.x - 1.0, z2.y + c.y);

        // Denominator: 2z + c - 2
        let den = vec2<f32>(2.0 * z.x + c.x - 2.0, 2.0 * z.y + c.y);

        // Complex division: num / den
        let den_mag_sq = dot(den, den);
        if (den_mag_sq < 0.0001) { break; }

        let div = vec2<f32>(
            (num.x * den.x + num.y * den.y) / den_mag_sq,
            (num.y * den.x - num.x * den.y) / den_mag_sq
        );

        // Square the result
        z = vec2<f32>(div.x * div.x - div.y * div.y, 2.0 * div.x * div.y);
        iteration = i;
    }

    if (iteration >= uniforms.max_iterations - 1u) {
        return 0.0;
    }

    let log_zn = log(max(dot(z, z), 1.0)) / 2.0;
    let nu = log(max(log_zn / log(2.0), 0.0001)) / log(2.0);
    return (f32(iteration) + 1.0 - nu) / f32(uniforms.max_iterations);
}

// Collatz fractal - based on Collatz conjecture generalized to complex numbers
fn collatz_fractal(c: vec2<f32>) -> f32 {
    var z = c;
    var iteration = 0u;
    var min_dist = 1000.0;  // Track minimum distance to origin for coloring

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let mag_sq = dot(z, z);
        if (mag_sq > 10000.0) {
            // Escaped - return smooth iteration count
            let log_zn = log(mag_sq) / 2.0;
            let nu = log(max(log_zn / log(2.0), 0.0001)) / log(2.0);
            return fract((f32(i) + 1.0 - nu) / 50.0);
        }

        min_dist = min(min_dist, sqrt(mag_sq));

        // Collatz map generalized to complex: z = 0.25 * (2 + 7z - (2 + 5z) * cos(π * z))
        let pi = 3.14159265359;

        // cos(π * z) for complex z
        let piz_real = pi * z.x;
        let piz_imag = pi * z.y;
        let cos_real = cos(piz_real) * cosh(piz_imag);
        let cos_imag = -sin(piz_real) * sinh(piz_imag);
        let cos_piz = vec2<f32>(cos_real, cos_imag);

        // (2 + 5z)
        let term1 = vec2<f32>(2.0 + 5.0 * z.x, 5.0 * z.y);

        // (2 + 5z) * cos(π * z)
        let product = vec2<f32>(
            term1.x * cos_piz.x - term1.y * cos_piz.y,
            term1.x * cos_piz.y + term1.y * cos_piz.x
        );

        // 2 + 7z - product, then multiply by 0.25
        z = vec2<f32>(2.0 + 7.0 * z.x - product.x, 7.0 * z.y - product.y) * 0.25;
        iteration = i;
    }

    // Didn't escape - color by minimum distance to origin
    return fract(min_dist * 2.0);
}

// Helper functions for Collatz
fn cosh(x: f32) -> f32 {
    return (exp(x) + exp(-x)) * 0.5;
}

fn sinh(x: f32) -> f32 {
    return (exp(x) - exp(-x)) * 0.5;
}

// ============================================================================
// 2D Strange Attractor Functions (from xfractint)
// ============================================================================

// Hopalong attractor: x' = y - sign(x)*sqrt(|b*x - c|), y' = a - x
// Parameters: a = julia_c.x, b = julia_c.y, c = 0
fn hopalong_attractor(coord: vec2<f32>) -> f32 {
    let a = uniforms.julia_c.x;
    let b = uniforms.julia_c.y;
    let c = 0.0;

    var x = 0.1;
    var y = 0.0;
    var hit_count = 0.0;

    // Skip initial transient (fewer iterations for faster startup)
    for (var i = 0u; i < 50u; i = i + 1u) {
        let x_new = y - sign(x) * sqrt(abs(b * x - c));
        let y_new = a - x;
        x = x_new;
        y = y_new;
    }

    // Pixel threshold based on zoom level - smaller threshold for point scatter
    // At zoom 1, view is ~4 units wide, with ~1000 pixels, so pixel_size ~= 0.004
    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0; // 2 pixel radius

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let x_new = y - sign(x) * sqrt(abs(b * x - c));
        let y_new = a - x;
        x = x_new;
        y = y_new;

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    // Normalize hit count - saturate gradually for visible points
    return clamp(hit_count / 5.0, 0.0, 1.0);
}

// Hénon attractor: x' = 1 + y - a*x², y' = b*x
// Parameters: a = julia_c.x, b = julia_c.y
fn henon_attractor(coord: vec2<f32>) -> f32 {
    let a = uniforms.julia_c.x;
    let b = uniforms.julia_c.y;

    var x = 0.0;
    var y = 0.0;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        let x_new = 1.0 + y - a * x * x;
        let y_new = b * x;
        x = x_new;
        y = y_new;
        if (abs(x) > 100.0 || abs(y) > 100.0) {
            x = 0.1;
            y = 0.1;
        }
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let x_new = 1.0 + y - a * x * x;
        let y_new = b * x;
        x = x_new;
        y = y_new;

        if (abs(x) > 100.0 || abs(y) > 100.0) { break; }

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    return clamp(hit_count / 5.0, 0.0, 1.0);
}

// Martin attractor: x' = y - sin(x), y' = a - x
// Parameters: a = julia_c.x
fn martin_attractor(coord: vec2<f32>) -> f32 {
    let a = uniforms.julia_c.x;

    var x = 0.1;
    var y = 0.0;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        let x_new = y - sin(x);
        let y_new = a - x;
        x = x_new;
        y = y_new;
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let x_new = y - sin(x);
        let y_new = a - x;
        x = x_new;
        y = y_new;

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    return clamp(hit_count / 5.0, 0.0, 1.0);
}

// Gingerbreadman: x' = 1 - y + |x|, y' = x
fn gingerbreadman_attractor(coord: vec2<f32>) -> f32 {
    var x = -0.1;
    var y = 0.0;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        let x_new = 1.0 - y + abs(x);
        let y_new = x;
        x = x_new;
        y = y_new;
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let x_new = 1.0 - y + abs(x);
        let y_new = x;
        x = x_new;
        y = y_new;

        if (abs(x) > 1000.0 || abs(y) > 1000.0) { break; }

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    return clamp(hit_count / 5.0, 0.0, 1.0);
}

// Latoocarfian: x' = sin(y*b) + c*sin(x*b), y' = sin(x*a) + d*sin(y*a)
// Parameters: a = julia_c.x, b = julia_c.y, c = power, d = fractal_fold
fn latoocarfian_attractor(coord: vec2<f32>) -> f32 {
    let a = uniforms.julia_c.x;
    let b = uniforms.julia_c.y;
    let c = uniforms.power;
    let d = uniforms.fractal_fold;

    var x = 0.1;
    var y = 0.1;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        let x_new = sin(y * b) + c * sin(x * b);
        let y_new = sin(x * a) + d * sin(y * a);
        x = x_new;
        y = y_new;
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let x_new = sin(y * b) + c * sin(x * b);
        let y_new = sin(x * a) + d * sin(y * a);
        x = x_new;
        y = y_new;

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    return clamp(hit_count / 5.0, 0.0, 1.0);
}

// Chip: x' = y - sign(x)*cos(log²(|b*x - c|))*arctan(log²(|c*x - b|)), y' = a - x
// Parameters: a = julia_c.x, b = julia_c.y, c = power
fn chip_attractor(coord: vec2<f32>) -> f32 {
    let a = uniforms.julia_c.x;
    let b = uniforms.julia_c.y;
    let c = uniforms.power;

    var x = 0.1;
    var y = 0.0;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        let log1 = log(max(abs(b * x - c), 0.001));
        let log2 = log(max(abs(c * x - b), 0.001));
        let x_new = y - sign(x) * cos(log1 * log1) * atan(log2 * log2);
        let y_new = a - x;
        x = x_new;
        y = y_new;
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let log1 = log(max(abs(b * x - c), 0.001));
        let log2 = log(max(abs(c * x - b), 0.001));
        let x_new = y - sign(x) * cos(log1 * log1) * atan(log2 * log2);
        let y_new = a - x;
        x = x_new;
        y = y_new;

        if (abs(x) > 10000.0 || abs(y) > 10000.0) { break; }

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    return clamp(hit_count / 5.0, 0.0, 1.0);
}

// Quadruptwo: x' = y - sign(x)*sin(log(|b*x - c|))*arctan(log²(|c*x - b|)), y' = a - x
// Parameters: a = julia_c.x, b = julia_c.y, c = power
fn quadruptwo_attractor(coord: vec2<f32>) -> f32 {
    let a = uniforms.julia_c.x;
    let b = uniforms.julia_c.y;
    let c = uniforms.power;

    var x = 0.1;
    var y = 0.0;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        let log1 = log(max(abs(b * x - c), 0.001));
        let log2 = log(max(abs(c * x - b), 0.001));
        let x_new = y - sign(x) * sin(log1) * atan(log2 * log2);
        let y_new = a - x;
        x = x_new;
        y = y_new;
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let log1 = log(max(abs(b * x - c), 0.001));
        let log2 = log(max(abs(c * x - b), 0.001));
        let x_new = y - sign(x) * sin(log1) * atan(log2 * log2);
        let y_new = a - x;
        x = x_new;
        y = y_new;

        if (abs(x) > 10000.0 || abs(y) > 10000.0) { break; }

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    return clamp(hit_count / 5.0, 0.0, 1.0);
}

// Threeply: x' = y - sign(x)*|sin(x)*cos(b) + c - x*sin(a+b+c)|, y' = a - x
// Parameters: a = julia_c.x, b = julia_c.y, c = power
fn threeply_attractor(coord: vec2<f32>) -> f32 {
    let a = uniforms.julia_c.x;
    let b = uniforms.julia_c.y;
    let c = uniforms.power;

    var x = 0.1;
    var y = 0.0;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        let term = sin(x) * cos(b) + c - x * sin(a + b + c);
        let x_new = y - sign(x) * abs(term);
        let y_new = a - x;
        x = x_new;
        y = y_new;
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let term = sin(x) * cos(b) + c - x * sin(a + b + c);
        let x_new = y - sign(x) * abs(term);
        let y_new = a - x;
        x = x_new;
        y = y_new;

        if (abs(x) > 100000.0 || abs(y) > 100000.0) { break; }

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    return clamp(hit_count / 5.0, 0.0, 1.0);
}

// Icon fractal with rotational symmetry
// Parameters: lambda = julia_c.x, alpha = julia_c.y, beta = power, gamma = fractal_fold, omega = fractal_min_radius, degree = fractal_scale
fn icon_attractor(coord: vec2<f32>) -> f32 {
    let lambda = uniforms.julia_c.x;
    let alpha = uniforms.julia_c.y;
    let beta = uniforms.power;
    let gamma = uniforms.fractal_fold;
    let omega = uniforms.fractal_min_radius;
    let degree = i32(uniforms.fractal_scale);

    var x = 0.1;
    var y = 0.1;
    var hit_count = 0.0;

    // Skip initial transient
    for (var i = 0u; i < 50u; i = i + 1u) {
        var zn_real = 1.0;
        var zn_imag = 0.0;
        for (var j = 0; j < degree - 2; j = j + 1) {
            let temp = zn_real * x - zn_imag * y;
            zn_imag = zn_real * y + zn_imag * x;
            zn_real = temp;
        }

        let zzbar = x * x + y * y;
        let p = lambda + alpha * zzbar + beta * (x * zn_real - y * zn_imag);
        let x_new = p * x + gamma * zn_real - omega * y;
        let y_new = p * y - gamma * zn_imag + omega * x;
        x = x_new;
        y = y_new;
    }

    let pixel_size = 4.0 / (uniforms.zoom * 1000.0);
    let threshold_sq = pixel_size * pixel_size * 4.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        var zn_real = 1.0;
        var zn_imag = 0.0;
        for (var j = 0; j < degree - 2; j = j + 1) {
            let temp = zn_real * x - zn_imag * y;
            zn_imag = zn_real * y + zn_imag * x;
            zn_real = temp;
        }

        let zzbar = x * x + y * y;
        let p = lambda + alpha * zzbar + beta * (x * zn_real - y * zn_imag);
        let x_new = p * x + gamma * zn_real - omega * y;
        let y_new = p * y - gamma * zn_imag + omega * x;
        x = x_new;
        y = y_new;

        if (abs(x) > 100.0 || abs(y) > 100.0) { break; }

        let diff = vec2<f32>(x, y) - coord;
        let dist_sq = dot(diff, diff);
        if (dist_sq < threshold_sq) {
            hit_count += 1.0;
        }
    }

    return clamp(hit_count / 5.0, 0.0, 1.0);
}

// ============================================================================
// 3D Distance Functions
// ============================================================================

fn mandelbulb_de(pos: vec3<f32>) -> f32 {
    // Apply inverse scale so higher slider value = bigger fractal (more intuitive)
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var z = pos * scale_inv;
    var dr = 1.0;
    var r = 0.0;
    let power = uniforms.power;

    for (var i = 0u; i < 16u; i = i + 1u) {
        r = length(z);
        if (r > 2.0) {
            break;
        }

        // Convert to polar coordinates
        var theta = acos(z.z / r);
        var phi = atan2(z.y, z.x);
        dr = pow(r, power - 1.0) * power * dr + 1.0;

        // Scale and rotate the point
        let zr = pow(r, power);
        theta = theta * power;
        phi = phi * power;

        // Convert back to cartesian coordinates
        z = zr * vec3<f32>(
            sin(theta) * cos(phi),
            sin(phi) * sin(theta),
            cos(theta)
        );
        z = z + pos * scale_inv;
    }

    return 0.5 * log(r) * r / (dr * scale_inv);
}

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

fn menger_sponge_de(pos: vec3<f32>) -> f32 {
    // Use inverse scale for intuitive behavior (higher = bigger)
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var p = pos * scale_inv;  // Adjustable initial scale (default 2.0 -> 0.5)
    var scale = scale_inv;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        p = abs(p);

        if (p.x < p.y) {
            p = p.yxz;
        }
        if (p.x < p.z) {
            p = p.zyx;
        }
        if (p.y < p.z) {
            p = p.xzy;
        }

        p = p * 3.0;
        scale = scale * 3.0;
        p = p - vec3<f32>(2.0, 2.0, 2.0);

        if (p.z < -1.0) {
            p.z = p.z + 2.0;
        }
    }

    let d = max(max(abs(p.x), abs(p.y)), abs(p.z)) - 1.0;
    return d / scale;
}

// Sierpinski Pyramid/Tetrahedron
fn sierpinski_pyramid_de(pos: vec3<f32>) -> f32 {
    var p = pos;
    var scale = uniforms.fractal_scale;  // Adjustable initial scale (default 2.0)

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        // Fold into tetrahedron
        if (p.x + p.y < 0.0) {
            p = vec3<f32>(-p.y, -p.x, p.z);
        }
        if (p.x + p.z < 0.0) {
            p = vec3<f32>(-p.z, p.y, -p.x);
        }
        if (p.y + p.z < 0.0) {
            p = vec3<f32>(p.x, -p.z, -p.y);
        }

        // Scale and translate
        p = p * 2.0 - vec3<f32>(1.0, 1.0, 1.0);
        scale = scale * 2.0;
    }

    // Distance to tetrahedron
    return (length(p) - 1.0) / scale;
}

// Quaternion Julia Set 3D
fn julia_set_3d_de(pos: vec3<f32>) -> f32 {
    // Apply inverse scale so higher slider value = bigger fractal (more intuitive)
    let scale_inv = 1.0 / uniforms.fractal_scale;
    let scaled_pos = pos * scale_inv;
    var q = vec4<f32>(scaled_pos.x, scaled_pos.y, scaled_pos.z, 0.0);
    var c = vec4<f32>(uniforms.julia_c.x, uniforms.julia_c.y, 0.0, 0.2);
    var dq = vec4<f32>(1.0, 0.0, 0.0, 0.0);

    for (var i = 0u; i < 16u; i = i + 1u) {
        // Quaternion multiplication: dq = 2 * q * dq
        dq = 2.0 * vec4<f32>(
            q.x * dq.x - q.y * dq.y - q.z * dq.z - q.w * dq.w,
            q.x * dq.y + q.y * dq.x + q.z * dq.w - q.w * dq.z,
            q.x * dq.z + q.z * dq.x + q.w * dq.y - q.y * dq.w,
            q.x * dq.w + q.w * dq.x + q.y * dq.z - q.z * dq.y
        );

        // Quaternion square: q = q * q + c
        q = vec4<f32>(
            q.x * q.x - q.y * q.y - q.z * q.z - q.w * q.w,
            2.0 * q.x * q.y,
            2.0 * q.x * q.z,
            2.0 * q.x * q.w
        ) + c;

        let r2 = dot(q, q);
        if (r2 > 4.0) {
            break;
        }
    }

    let r = length(q);
    let dr = length(dq);
    return 0.5 * r * log(r) / (dr * scale_inv);
}

fn mandelbox_de(pos: vec3<f32>) -> f32 {
    // Use fractal_scale for size control (inverse for intuitive behavior)
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var p = pos * scale_inv * 3.305;  // Scale with adjustable parameter
    var dr = 1.0;
    // Use power parameter as internal scale (default 8.0, typically -3.0 to -1.5 for detail)
    let internal_scale = -(uniforms.power / 4.0);  // Maps 8.0 -> -2.0
    let fold_limit = uniforms.fractal_fold;  // Adjustable fold limit (default 1.0)
    let min_radius2 = uniforms.fractal_min_radius * uniforms.fractal_min_radius;  // Square for r2 comparison
    let fixed_radius2 = 1.0;

    for (var i = 0u; i < uniforms.max_steps; i = i + 1u) {
        // Box fold
        p = clamp(p, vec3<f32>(-fold_limit), vec3<f32>(fold_limit)) * 2.0 - p;

        // Sphere fold
        let r2 = dot(p, p);

        if (r2 < min_radius2) {
            let temp = fixed_radius2 / min_radius2;
            p = p * temp;
            dr = dr * temp;
        } else if (r2 < fixed_radius2) {
            let temp = fixed_radius2 / r2;
            p = p * temp;
            dr = dr * temp;
        }

        // Scale and translate
        p = p * internal_scale + pos * scale_inv * 3.305;
        dr = dr * abs(internal_scale) + 1.0;

        if (length(p) > 100.0) {
            break;
        }
    }

    // Scale result back down (20% larger than initial 75% reduction)
    return (length(p) / abs(dr)) * 0.3025;
}

// Octahedral IFS - 8-fold kaleidoscopic symmetry
fn octahedral_ifs_de(pos: vec3<f32>) -> f32 {
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var p = pos * scale_inv;
    var scale = uniforms.fractal_fold + 1.0;  // Use fold parameter for scaling (2.0-3.0 range works well)
    var dr = 1.0;

    for (var i = 0u; i < uniforms.max_steps; i = i + 1u) {
        // Octahedral folding - creates 8-fold symmetry
        p = abs(p);
        if (p.x - p.y < 0.0) {
            let temp = p.x;
            p.x = p.y;
            p.y = temp;
        }
        if (p.x - p.z < 0.0) {
            let temp = p.x;
            p.x = p.z;
            p.z = temp;
        }
        if (p.y - p.z < 0.0) {
            let temp = p.y;
            p.y = p.z;
            p.z = temp;
        }

        // Apply scaling and translation
        p = p * scale - vec3<f32>(1.0) * (scale - 1.0);
        dr = dr * scale;

        // Bailout
        if (length(p) > 100.0) {
            break;
        }
    }

    // Distance estimate - box shape
    return (length(p) - 2.0) / abs(dr);
}

// Icosahedral IFS - 20-fold kaleidoscopic symmetry (most complex!)
fn icosahedral_ifs_de(pos: vec3<f32>) -> f32 {
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var p = pos * scale_inv;
    let scale = uniforms.fractal_fold + 1.0;
    var dr = 1.0;

    // Golden ratio for icosahedral symmetry
    let phi = (1.0 + sqrt(5.0)) / 2.0;

    // Pre-compute normalized icosahedral fold normals
    // These define the 15 mirror planes of icosahedral symmetry
    let n1 = normalize(vec3<f32>(1.0, phi, 0.0));
    let n2 = normalize(vec3<f32>(0.0, 1.0, phi));
    let n3 = normalize(vec3<f32>(phi, 0.0, 1.0));
    // Additional planes for full icosahedral symmetry
    let n4 = normalize(vec3<f32>(-1.0, phi, 0.0));
    let n5 = normalize(vec3<f32>(0.0, -1.0, phi));

    for (var i = 0u; i < uniforms.max_steps; i = i + 1u) {
        // Icosahedral folding - creates 20-fold symmetry
        // Fold across icosahedral mirror planes (before abs!)
        var d = dot(p, n1);
        if (d < 0.0) { p = p - 2.0 * d * n1; }

        d = dot(p, n2);
        if (d < 0.0) { p = p - 2.0 * d * n2; }

        d = dot(p, n3);
        if (d < 0.0) { p = p - 2.0 * d * n3; }

        d = dot(p, n4);
        if (d < 0.0) { p = p - 2.0 * d * n4; }

        d = dot(p, n5);
        if (d < 0.0) { p = p - 2.0 * d * n5; }

        // Apply abs for remaining octant folding
        p = abs(p);

        // Sort coordinates to ensure consistent fundamental domain
        if (p.x < p.y) {
            let temp = p.x;
            p.x = p.y;
            p.y = temp;
        }
        if (p.y < p.z) {
            let temp = p.y;
            p.y = p.z;
            p.z = temp;
        }
        if (p.x < p.y) {
            let temp = p.x;
            p.x = p.y;
            p.y = temp;
        }

        // Apply scaling and translation
        p = p * scale - vec3<f32>(1.0) * (scale - 1.0);
        dr = dr * scale;

        if (length(p) > 100.0) {
            break;
        }
    }

    return (length(p) - 2.0) / abs(dr);
}

// Apollonian Gasket - sphere packing fractal
fn apollonian_gasket_de(pos: vec3<f32>) -> f32 {
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var p = pos * scale_inv;

    // Use min_radius as the packing parameter (typically 0.5-2.0)
    let k = uniforms.fractal_min_radius;
    // Use fold parameter as scale multiplier for varying complexity
    let scale = (1.0 + k) * uniforms.fractal_fold;
    var dr = 1.0;

    for (var i = 0u; i < uniforms.max_steps; i = i + 1u) {
        // Inversion folding - key to Apollonian gasket
        p = abs(p);

        // Sphere inversion
        let r2 = dot(p, p);
        let k2 = k * k;

        if (r2 < k2) {
            p = p * (1.0 / k2);
            dr = dr * (1.0 / k2);
        } else if (r2 < 1.0) {
            p = p / r2;
            dr = dr / r2;
        }

        // Box folding for additional structure
        if (p.x - p.y < 0.0) {
            let temp = p.x;
            p.x = p.y;
            p.y = temp;
        }
        if (p.x - p.z < 0.0) {
            let temp = p.x;
            p.x = p.z;
            p.z = temp;
        }
        if (p.y - p.z < 0.0) {
            let temp = p.y;
            p.y = p.z;
            p.z = temp;
        }

        // Scale and translate
        p = p * scale - vec3<f32>(k, k, k);
        dr = dr * scale;

        if (length(p) > 100.0) {
            break;
        }
    }

    // Sphere distance
    return (length(p) - 1.0) / abs(dr);
}

// Amazing Surface / Pseudo-Kleinian fractal
fn kleinian_de(pos: vec3<f32>) -> f32 {
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var p = pos * scale_inv;

    let scale = uniforms.fractal_fold + 1.0;
    let csize = vec3<f32>(1.0, 1.0, 1.3);
    let c = vec3<f32>(0.0, 0.0, -uniforms.fractal_min_radius);
    var de = 1.0;

    for (var i = 0u; i < 12u; i = i + 1u) {
        // Folding operations
        p = abs(p);

        // Conditional folds for more complex structure
        if (p.x - p.y < 0.0) { let t = p.x; p.x = p.y; p.y = t; }
        if (p.x - p.z < 0.0) { let t = p.x; p.x = p.z; p.z = t; }
        if (p.y - p.z < 0.0) { let t = p.y; p.y = p.z; p.z = t; }

        // Box fold
        p = p * scale - csize * (scale - 1.0);

        // Sphere inversion
        let r2 = dot(p, p);
        let k = clamp(1.0 / r2, 1.0, 3.0);
        p = p * k;
        de = de * k;

        // Translation
        p = p + c;
        de = de * scale;
    }

    return (length(p) - 0.4) / de;
}

// Hybrid Mandelbulb-Julia fractal
fn hybrid_mandelbulb_julia_de(pos: vec3<f32>) -> f32 {
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var z = pos * scale_inv;
    var dr = 1.0;
    var r = length(z);

    // Julia constant from julia_c parameter
    let c = vec3<f32>(uniforms.julia_c.x, uniforms.julia_c.y, 0.3);

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        if (r > 2.0) { break; }

        // Convert to spherical
        let theta = acos(z.z / r);
        let phi = atan2(z.y, z.x);
        dr = pow(r, uniforms.power - 1.0) * uniforms.power * dr + 1.0;

        // Scale and rotate
        let zr = pow(r, uniforms.power);
        let new_theta = theta * uniforms.power;
        let new_phi = phi * uniforms.power;

        // Back to cartesian
        z = zr * vec3<f32>(
            sin(new_theta) * cos(new_phi),
            sin(new_theta) * sin(new_phi),
            cos(new_theta)
        );

        // Alternate between Mandelbrot (add pos) and Julia (add c)
        if (i % 2u == 0u) {
            z = z + pos * scale_inv;
        } else {
            z = z + c;
        }

        r = length(z);
    }

    return 0.5 * log(r) * r / dr;
}

// Quaternion cubic Julia set (z³ + c)
fn quaternion_cubic_de(pos: vec3<f32>) -> f32 {
    let scale_inv = 1.0 / uniforms.fractal_scale;
    // Map 3D position to quaternion (w=0)
    var q = vec4<f32>(pos * scale_inv, 0.0);
    var dq = vec4<f32>(1.0, 0.0, 0.0, 0.0);

    // Julia constant as quaternion
    let c = vec4<f32>(uniforms.julia_c.x, uniforms.julia_c.y, 0.3, 0.0);

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let r = length(q);
        if (r > 2.0) { break; }

        // Quaternion squaring: q² (for derivative)
        let q2 = quat_mul(q, q);

        // Derivative: dq = 3*q²*dq (chain rule for z³)
        dq = quat_mul(quat_mul(vec4<f32>(3.0, 0.0, 0.0, 0.0), q2), dq);

        // Quaternion cubing: q³ = q * q²
        let q3 = quat_mul(q, q2);

        // z = z³ + c
        q = q3 + c;
    }

    let r = length(q);
    let dr = length(dq);
    return 0.5 * log(r) * r / dr;
}

// Sierpinski Gasket (Tetrahedral IFS with sphere folding)
fn sierpinski_gasket_de(pos: vec3<f32>) -> f32 {
    let scale_inv = 1.0 / uniforms.fractal_scale;
    var p = pos * scale_inv;
    let scale_factor = 2.0 + uniforms.fractal_fold * 0.5;  // Base scale factor (2.0-3.0 range)
    var scale = 1.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        // Tetrahedral folding - creates 4-fold symmetry
        if (p.x + p.y < 0.0) {
            p = vec3<f32>(-p.y, -p.x, p.z);
        }
        if (p.x + p.z < 0.0) {
            p = vec3<f32>(-p.z, p.y, -p.x);
        }
        if (p.y + p.z < 0.0) {
            p = vec3<f32>(p.x, -p.z, -p.y);
        }

        // Sphere inversion folding to create "gasket" holes
        let r2 = dot(p, p);
        let min_r2 = uniforms.fractal_min_radius * uniforms.fractal_min_radius;

        if (r2 < min_r2) {
            let k = min_r2 / r2;
            p = p * k;
            scale = scale * k;
        }

        // Scale and translate to create the gasket structure
        p = p * scale_factor - vec3<f32>(1.0, 1.0, 1.0) * (scale_factor - 1.0);
        scale = scale * scale_factor;
    }

    // Distance to tetrahedron
    return (length(p) - 1.0) / scale;
}

// ============================================================================
// 3D Strange Attractor Distance Functions
// ============================================================================

// Pickover attractor: Creates a 3D orbit that we can ray march
// x' = sin(a*y) - z*cos(b*x), y' = z*sin(c*x) - cos(d*y), z' = sin(x)
fn pickover_attractor_de(pos: vec3<f32>) -> f32 {
    let scale = uniforms.fractal_scale;
    let a = uniforms.julia_c.x;
    let b = uniforms.julia_c.y;
    let c = uniforms.power;
    let d = uniforms.fractal_fold;

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
    let dt = 0.01;

    // Skip transient
    for (var i = 0u; i < 500u; i = i + 1u) {
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

// Quaternion multiplication helper
fn quat_mul(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(
        a.x * b.x - a.y * b.y - a.z * b.z - a.w * b.w,
        a.x * b.y + a.y * b.x + a.z * b.w - a.w * b.z,
        a.x * b.z - a.y * b.w + a.z * b.x + a.w * b.y,
        a.x * b.w + a.y * b.z - a.z * b.y + a.w * b.x
    );
}

// Floor distance estimation
fn floor_de(pos: vec3<f32>) -> f32 {
    return pos.y - uniforms.floor_height;
}

// Checkered pattern for floor
fn checkered(pos: vec3<f32>) -> vec3<f32> {
    let scale = 1.0;
    let ix = floor(pos.x / scale);
    let iz = floor(pos.z / scale);
    return select(uniforms.floor_color2, uniforms.floor_color1, (i32(ix) + i32(iz)) % 2 == 0);
}

struct SceneResult {
    distance: f32,
    material_id: u32, // 0 = fractal, 1 = floor
}

fn scene_de_with_material(pos: vec3<f32>) -> SceneResult {
    var result: SceneResult;
    var fractal_dist = 1000.0;

    // 3D fractals start at type 13 (after 13 2D types: 0-12)
    if (uniforms.fractal_type == 13u) {
        fractal_dist = mandelbulb_de(pos);
    } else if (uniforms.fractal_type == 14u) {
        fractal_dist = menger_sponge_de(pos);
    } else if (uniforms.fractal_type == 15u) {
        fractal_dist = sierpinski_pyramid_de(pos);
    } else if (uniforms.fractal_type == 16u) {
        fractal_dist = julia_set_3d_de(pos);
    } else if (uniforms.fractal_type == 17u) {
        fractal_dist = mandelbox_de(pos);
    } else if (uniforms.fractal_type == 18u) {
        fractal_dist = octahedral_ifs_de(pos);
    } else if (uniforms.fractal_type == 19u) {
        fractal_dist = icosahedral_ifs_de(pos);
    } else if (uniforms.fractal_type == 20u) {
        fractal_dist = apollonian_gasket_de(pos);
    } else if (uniforms.fractal_type == 21u) {
        fractal_dist = kleinian_de(pos);
    } else if (uniforms.fractal_type == 22u) {
        fractal_dist = hybrid_mandelbulb_julia_de(pos);
    } else if (uniforms.fractal_type == 23u) {
        fractal_dist = quaternion_cubic_de(pos);
    } else if (uniforms.fractal_type == 24u) {
        fractal_dist = sierpinski_gasket_de(pos);
    // 3D Strange Attractors (types 35-37)
    } else if (uniforms.fractal_type == 35u) {
        fractal_dist = pickover_attractor_de(pos);
    } else if (uniforms.fractal_type == 36u) {
        fractal_dist = lorenz_attractor_de(pos);
    } else if (uniforms.fractal_type == 37u) {
        fractal_dist = rossler_attractor_de(pos);
    }

    var floor_dist = 1000.0;
    if (uniforms.show_floor == 1u) {
        floor_dist = floor_de(pos);
    }

    if (floor_dist < fractal_dist) {
        result.distance = floor_dist;
        result.material_id = 1u;
    } else {
        result.distance = fractal_dist;
        result.material_id = 0u;
    }

    return result;
}

fn scene_de(pos: vec3<f32>) -> f32 {
    return scene_de_with_material(pos).distance;
}

// ============================================================================
// Ray Marching
// ============================================================================

// Sphere intersection for bounding sphere acceleration
// Returns distance to entry point, or -1.0 if no intersection
fn intersect_sphere(origin: vec3<f32>, direction: vec3<f32>, center: vec3<f32>, radius: f32) -> f32 {
    let oc = origin - center;
    let a = dot(direction, direction);
    let b = 2.0 * dot(oc, direction);
    let c = dot(oc, oc) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;

    if (discriminant < 0.0) {
        return -1.0; // No intersection
    }

    let sqrt_discriminant = sqrt(discriminant);
    let t1 = (-b - sqrt_discriminant) / (2.0 * a);
    let t2 = (-b + sqrt_discriminant) / (2.0 * a);

    // Return closest positive intersection
    if (t1 > 0.0) {
        return t1;
    } else if (t2 > 0.0) {
        return t2;
    }

    return -1.0; // Ray origin is beyond sphere
}

struct RayMarchResult {
    hit: bool,
    distance: f32,
    steps: u32,
    position: vec3<f32>,
    material_id: u32,
}

fn ray_march(origin: vec3<f32>, direction: vec3<f32>) -> RayMarchResult {
    var result: RayMarchResult;
    result.hit = false;
    result.distance = 0.0;
    result.steps = 0u;
    result.material_id = 0u;

    // Bounding sphere acceleration: skip empty space when camera is outside sphere
    var total_distance = 0.0;
    // Conservative bounding radius to account for all fractal types and parameters
    // Different fractals grow at different rates with iterations:
    // - Menger Sponge: 3^iterations
    // - IFS fractals: (fold+1)^steps
    // - Apollonian: complex growth with min_radius
    // Use very conservative multiplier to ensure we never clip the fractal
    let iteration_factor = f32(max(uniforms.max_iterations, uniforms.max_steps)) * 0.1;
    let bounding_radius = 50.0 * uniforms.fractal_scale * max(1.0, uniforms.fractal_fold) * max(1.0, iteration_factor);
    let bounding_center = vec3<f32>(0.0);

    // Check if camera is outside the bounding sphere
    let camera_to_center = length(origin - bounding_center);
    if (camera_to_center > bounding_radius) {
        // Camera is outside - use sphere intersection to skip empty space if ray hits sphere
        let sphere_hit = intersect_sphere(origin, direction, bounding_center, bounding_radius);
        if (sphere_hit > 0.0) {
            // Check if we would hit the floor before the sphere
            var floor_hit = -1.0;
            if (uniforms.show_floor == 1u && abs(direction.y) > 0.0001) {
                let t = (uniforms.floor_height - origin.y) / direction.y;
                if (t > 0.0) {
                    floor_hit = t;
                }
            }

            // Only skip to sphere if floor is farther away or doesn't exist
            if (floor_hit < 0.0 || sphere_hit < floor_hit) {
                total_distance = sphere_hit;
            }
        }
        // If ray misses the sphere, continue with normal ray marching (for floor, etc.)
    }
    // If camera is inside sphere, start ray marching from camera position (total_distance = 0.0)

    for (var i = 0u; i < uniforms.max_steps; i = i + 1u) {
        result.steps = i;
        let pos = origin + direction * total_distance;
        let scene_result = scene_de_with_material(pos);
        let dist = scene_result.distance;

        // Determine step size based on mode
        var step_size: f32;
        if (uniforms.use_adaptive_step == 1u) {
            // Adaptive stepping: use distance estimate
            step_size = dist * uniforms.step_multiplier;
        } else {
            // Fixed stepping: constant step size
            step_size = uniforms.fixed_step_size;
        }

        total_distance = total_distance + step_size;

        if (dist < uniforms.min_distance) {
            result.hit = true;
            result.distance = total_distance;
            result.position = pos;
            result.material_id = scene_result.material_id;
            break;
        }

        if (total_distance > uniforms.max_distance) {
            break;
        }
    }

    return result;
}

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

        // Conservative stepping to prevent missing thin features:
        // Use configurable step factor (default 0.6 = 60% of DE for safety margin)
        // Also cap at smaller maximum to prevent over-stepping
        let conservative_step = h * uniforms.shadow_step_factor;
        let max_step = 0.05 * uniforms.fractal_scale;
        let step = min(conservative_step, max_step);
        t = t + step;

        if (t > maxt) {
            break;
        }
    }

    return result;
}

// ============================================================================
// Lighting Models
// ============================================================================

fn blinn_phong(normal: vec3<f32>, view_dir: vec3<f32>, light_dir: vec3<f32>, albedo: vec3<f32>) -> vec3<f32> {
    let ambient = uniforms.ambient_light;
    let diffuse = max(dot(normal, light_dir), 0.0) * uniforms.light_intensity;

    let half_dir = normalize(light_dir + view_dir);
    let specular = pow(max(dot(normal, half_dir), 0.0), 32.0) * uniforms.light_intensity;

    return albedo * (ambient + diffuse) + vec3<f32>(specular);
}

// PBR Functions
fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * denom * denom);
}

fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k);
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let ggx2 = geometry_schlick_ggx(n_dot_v, roughness);
    let ggx1 = geometry_schlick_ggx(n_dot_l, roughness);
    return ggx1 * ggx2;
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - cos_theta, 5.0);
}

fn pbr(normal: vec3<f32>, view_dir: vec3<f32>, light_dir: vec3<f32>, albedo: vec3<f32>, metallic: f32, roughness: f32) -> vec3<f32> {
    let n_dot_v = max(dot(normal, view_dir), 0.0);
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let half_dir = normalize(view_dir + light_dir);
    let n_dot_h = max(dot(normal, half_dir), 0.0);
    let h_dot_v = max(dot(half_dir, view_dir), 0.0);

    // Calculate F0 (surface reflection at zero incidence)
    var f0 = vec3<f32>(0.04);
    f0 = mix(f0, albedo, metallic);

    // Cook-Torrance BRDF
    let ndf = distribution_ggx(n_dot_h, roughness);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    let f = fresnel_schlick(h_dot_v, f0);

    let numerator = ndf * g * f;
    let denominator = 4.0 * n_dot_v * n_dot_l + 0.001;
    let specular = numerator / denominator;

    let k_s = f;
    var k_d = vec3<f32>(1.0) - k_s;
    k_d = k_d * (1.0 - metallic);

    let radiance = vec3<f32>(uniforms.light_intensity);

    // Ambient light for PBR
    let ambient = albedo * uniforms.ambient_light;

    return (k_d * albedo / 3.14159265 + specular) * radiance * n_dot_l + ambient;
}

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

// ============================================================================
// Ray Rendering Helper (for DOF multi-sampling)
// ============================================================================

fn render_ray(ray_origin: vec3<f32>, ray_dir: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    // Ray march
    let result = ray_march(ray_origin, ray_dir);

    if (!result.hit) {
        // Background - use fog color if fog is enabled, otherwise gradient
        var bg: vec3<f32>;
        if (uniforms.fog_enabled != 0u) {
            bg = uniforms.fog_color;
        } else {
            let t = (uv.y + 1.0) * 0.5;
            bg = mix(vec3<f32>(0.1, 0.1, 0.15), vec3<f32>(0.0, 0.0, 0.0), t);
        }
        return bg;
    }

    let pos = result.position;
    let normal = calculate_normal(pos);
    let view_dir = normalize(ray_origin - pos);

    // Lighting
    let light_dir = calculate_light_direction();

    // Calculate shadow with small offset to avoid self-intersection
    // Use 3x min_distance as offset - just enough to clear the surface
    let shadow_offset = uniforms.min_distance * 3.0;
    let shadow = calculate_soft_shadow(pos + normal * shadow_offset, light_dir, shadow_offset);

    // Calculate AO
    let ao = calculate_ao(pos, normal);

    // Determine albedo based on material and color mode
    var albedo: vec3<f32>;
    var final_color: vec3<f32>;
    var apply_shading = true;

    // Floor always uses checkered pattern regardless of color mode
    if (result.material_id == 1u) {
        albedo = checkered(pos);
    } else {
        // Apply color mode visualization to fractal only
        if (uniforms.color_mode == 1u) {
            // Ray Steps visualization
            let step_t = f32(result.steps) / f32(uniforms.max_steps);
            albedo = vec3<f32>(step_t, step_t * 0.5, 1.0 - step_t);
        } else if (uniforms.color_mode == 2u) {
            // Normals visualization (no shading)
            final_color = normal * 0.5 + 0.5;
            apply_shading = false;
        } else if (uniforms.color_mode == 3u) {
            // Orbit Trap XYZ - color based on position components using palette
            let xyz_sum = abs(fract(pos.x * uniforms.orbit_trap_scale * 1.5)) + abs(fract(pos.y * uniforms.orbit_trap_scale * 1.5)) + abs(fract(pos.z * uniforms.orbit_trap_scale * 1.5));
            let trap_t = fract(xyz_sum / 3.0);
            albedo = get_palette_color(trap_t);
        } else if (uniforms.color_mode == 4u) {
            // Orbit Trap Radial - color based on distance patterns using palette
            let dist = length(pos);
            let radial_t = fract(dist * uniforms.orbit_trap_scale * 2.0);
            albedo = get_palette_color(radial_t);
        } else if (uniforms.color_mode == 5u) {
            // World Position visualization
            albedo = abs(fract(pos * 0.5));
        } else if (uniforms.color_mode == 6u) {
            // Local Position visualization (relative to fractal center)
            albedo = abs(fract(pos * 2.0));
        } else if (uniforms.color_mode == 7u) {
            // Ambient Occlusion only (no shading)
            final_color = vec3<f32>(ao);
            apply_shading = false;
        } else if (uniforms.color_mode == 8u) {
            // Per-Channel mode - map different sources to R,G,B independently
            let iter_value = f32(result.steps) / f32(uniforms.max_steps);
            let dist_value = clamp(result.distance * 10.0, 0.0, 1.0);

            // Get value for each channel based on source
            var r_val = 0.0;
            if (uniforms.channel_r == 0u) { r_val = iter_value; }  // Iterations
            else if (uniforms.channel_r == 1u) { r_val = dist_value; }  // Distance
            else if (uniforms.channel_r == 2u) { r_val = abs(fract(pos.x)); }  // PositionX
            else if (uniforms.channel_r == 3u) { r_val = abs(fract(pos.y)); }  // PositionY
            else if (uniforms.channel_r == 4u) { r_val = abs(fract(pos.z)); }  // PositionZ
            else if (uniforms.channel_r == 5u) { r_val = abs(normal.x); }  // Normal
            else if (uniforms.channel_r == 6u) { r_val = ao; }  // AO
            else if (uniforms.channel_r == 7u) { r_val = 0.0; }  // Constant

            var g_val = 0.0;
            if (uniforms.channel_g == 0u) { g_val = iter_value; }
            else if (uniforms.channel_g == 1u) { g_val = dist_value; }
            else if (uniforms.channel_g == 2u) { g_val = abs(fract(pos.x)); }
            else if (uniforms.channel_g == 3u) { g_val = abs(fract(pos.y)); }
            else if (uniforms.channel_g == 4u) { g_val = abs(fract(pos.z)); }
            else if (uniforms.channel_g == 5u) { g_val = abs(normal.y); }
            else if (uniforms.channel_g == 6u) { g_val = ao; }
            else if (uniforms.channel_g == 7u) { g_val = 0.0; }

            var b_val = 0.0;
            if (uniforms.channel_b == 0u) { b_val = iter_value; }
            else if (uniforms.channel_b == 1u) { b_val = dist_value; }
            else if (uniforms.channel_b == 2u) { b_val = abs(fract(pos.x)); }
            else if (uniforms.channel_b == 3u) { b_val = abs(fract(pos.y)); }
            else if (uniforms.channel_b == 4u) { b_val = abs(fract(pos.z)); }
            else if (uniforms.channel_b == 5u) { b_val = abs(normal.z); }
            else if (uniforms.channel_b == 6u) { b_val = ao; }
            else if (uniforms.channel_b == 7u) { b_val = 0.0; }

            albedo = vec3<f32>(r_val, g_val, b_val);
        } else if (uniforms.color_mode == 9u) {
            // Distance Field visualization - show complexity of distance field
            // Use ray marching steps as proxy for distance field tightness
            // More steps = tighter/more complex distance field
            let steps_t = f32(result.steps) / f32(uniforms.max_steps);
            // Use log scale to emphasize differences in lower step counts
            let dist_t = clamp(log2(1.0 + steps_t * 15.0) / 4.0, 0.0, 1.0);
            final_color = vec3<f32>(dist_t, dist_t * 0.5, 1.0 - dist_t);
            apply_shading = false;
        } else if (uniforms.color_mode == 10u) {
            // Depth visualization - visualize distance from camera
            let depth = length(pos - uniforms.camera_pos);
            // Use a reasonable depth range (0-20 units) instead of max_distance
            let depth_t = clamp(depth / 20.0, 0.0, 1.0);
            final_color = vec3<f32>(1.0 - depth_t, depth_t * 0.5, depth_t);
            apply_shading = false;
        } else if (uniforms.color_mode == 11u) {
            // Convergence visualization - escape time (mainly for 2D fractals)
            let conv_t = f32(result.steps) / f32(max(uniforms.max_iterations, uniforms.max_steps));
            final_color = vec3<f32>(conv_t, 1.0 - conv_t, conv_t * (1.0 - conv_t) * 4.0);
            apply_shading = false;
        } else if (uniforms.color_mode == 12u) {
            // Lighting Only - show only the lighting (no fractal coloring)
            albedo = vec3<f32>(0.8, 0.8, 0.8);  // Neutral gray albedo
        } else if (uniforms.color_mode == 13u) {
            // Shadow Map visualization
            final_color = vec3<f32>(shadow);
            apply_shading = false;
        } else if (uniforms.color_mode == 14u) {
            // Camera Distance (LOD Zones) - shows distance from camera using LOD zone colors
            let distance = length(pos - uniforms.camera_pos);
            final_color = get_lod_debug_color(distance);
            apply_shading = false;
        } else if (uniforms.color_mode == 15u) {
            // Distance Grayscale - visualize raw distance from camera as brightness
            let distance = length(pos - uniforms.camera_pos);
            // Map distance 0-100 units to grayscale
            let gray = clamp(distance / 100.0, 0.0, 1.0);
            final_color = vec3<f32>(gray);
            apply_shading = false;
        } else {
            // Standard palette mode (0)
            let color_t = f32(result.steps) / f32(uniforms.max_steps);
            albedo = get_palette_color(color_t);
        }
    }

    // Apply shading to modes that need it
    if (apply_shading) {
        if (uniforms.shading_model == 0u) {
            final_color = blinn_phong(normal, view_dir, light_dir, albedo);
        } else {
            final_color = pbr(normal, view_dir, light_dir, albedo, uniforms.metallic, uniforms.roughness);
        }

        // Apply shadow
        if (uniforms.soft_shadows != 0u) {
            final_color = final_color * shadow;
        }

        // Apply ambient occlusion
        if (uniforms.ambient_occlusion != 0u) {
            final_color = final_color * ao;
        }
    }

    // Apply screen-space reflections for floor
    if (result.material_id == 1u && uniforms.floor_reflections != 0u) {
        // Reflect the view direction about the floor normal (upward)
        let floor_normal = vec3<f32>(0.0, 1.0, 0.0);
        let reflect_dir = reflect(-view_dir, floor_normal);

        // Cast a reflection ray from the floor position
        let reflect_origin = pos + floor_normal * uniforms.min_distance * 3.0;
        let reflect_result = ray_march(reflect_origin, reflect_dir);

        if (reflect_result.hit && reflect_result.material_id == 0u) {
            // We hit the fractal - calculate its color
            let reflect_pos = reflect_result.position;
            let reflect_normal = calculate_normal(reflect_pos);
            let reflect_view_dir = normalize(reflect_origin - reflect_pos);
            let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));

            // Get fractal albedo based on color mode
            var reflect_albedo: vec3<f32>;
            if (uniforms.color_mode == 1u) {
                let step_t = f32(reflect_result.steps) / f32(uniforms.max_steps);
                reflect_albedo = vec3<f32>(step_t, step_t * 0.5, 1.0 - step_t);
            } else {
                let color_t = f32(reflect_result.steps) / f32(uniforms.max_steps);
                reflect_albedo = get_palette_color(color_t);
            }

            // Apply lighting to reflection
            var reflect_color: vec3<f32>;
            if (uniforms.shading_model == 0u) {
                reflect_color = blinn_phong(reflect_normal, reflect_view_dir, light_dir, reflect_albedo);
            } else {
                reflect_color = pbr(reflect_normal, reflect_view_dir, light_dir, reflect_albedo, uniforms.metallic, uniforms.roughness);
            }

            // Apply AO and shadows to reflection
            if (uniforms.ambient_occlusion != 0u) {
                let reflect_ao = calculate_ao(reflect_pos, reflect_normal);
                reflect_color = reflect_color * reflect_ao;
            }
            if (uniforms.soft_shadows != 0u) {
                let shadow_offset = uniforms.min_distance * 3.0;
                let reflect_shadow = calculate_soft_shadow(reflect_pos + reflect_normal * shadow_offset, light_dir, shadow_offset);
                reflect_color = reflect_color * reflect_shadow;
            }

            // Fresnel effect - more reflection at grazing angles
            let fresnel = pow(1.0 - abs(dot(view_dir, floor_normal)), 2.0);
            // Scale reflection strength by user parameter (0.0 = no reflection, 1.0 = full strength)
            // Base range is 0.3-0.7, scaled by floor_reflection_strength
            let base_strength = mix(0.3, 0.7, fresnel);
            let reflection_strength = base_strength * uniforms.floor_reflection_strength;

            // Mix floor color with reflection
            final_color = mix(final_color, reflect_color, reflection_strength);
        }
    }

    // Apply fog if enabled
    if (uniforms.fog_enabled != 0u) {
        let dist = length(pos - ray_origin);
        var fog_factor: f32;

        if (uniforms.fog_mode == 0u) {
            // Linear fog
            fog_factor = clamp(dist * uniforms.fog_density, 0.0, 1.0);
        } else if (uniforms.fog_mode == 1u) {
            // Exponential fog
            fog_factor = 1.0 - exp(-uniforms.fog_density * dist);
        } else {
            // Quadratic fog
            fog_factor = 1.0 - exp(-uniforms.fog_density * dist * dist);
        }

        fog_factor = clamp(fog_factor, 0.0, 1.0);
        final_color = mix(final_color, uniforms.fog_color, fog_factor);
    }

    // LOD debug visualization removed - use Color Mode "Camera Distance LOD" instead

    return final_color;
}

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

// ============================================================================
// Main Fragment Shader
// ============================================================================

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if (uniforms.render_mode == 0u) {
        // 2D Mode
        // Use the aspect ratio supplied by the host (window or capture target)
        let aspect = uniforms.aspect_ratio.x;

        var t: f32;
        var coord: vec2<f32>;

        // Check if high-precision mode is enabled and fractal supports it
        if (uniforms.high_precision == 1u && uniforms.fractal_type <= 4u) {
            // High-precision coordinate calculation
            // offset = uv * 2.0 / zoom * aspect (for x) or uv * 2.0 / zoom (for y)
            let offset_x = input.uv.x * 2.0 / uniforms.zoom * aspect;
            let offset_y = input.uv.y * 2.0 / uniforms.zoom;

            // Add offset to high-precision center using double-float arithmetic
            let coord_x = df_add_full(uniforms.center_hi.x, uniforms.center_lo.x, offset_x, 0.0);
            let coord_y = df_add_full(uniforms.center_hi.y, uniforms.center_lo.y, offset_y, 0.0);
            let coord_hi = vec2<f32>(coord_x.x, coord_y.x);
            let coord_lo = vec2<f32>(coord_x.y, coord_y.y);
            coord = coord_hi; // Use high part for color modes

            // Use high-precision fractal functions
            if (uniforms.fractal_type == 0u) {
                t = mandelbrot_hp(coord_hi, coord_lo);
            } else if (uniforms.fractal_type == 1u) {
                t = julia_hp(coord_hi, coord_lo);
            } else if (uniforms.fractal_type == 2u) {
                t = sierpinski_hp(coord_hi, coord_lo);
            } else if (uniforms.fractal_type == 3u) {
                t = sierpinski_triangle_hp(coord_hi, coord_lo);
            } else if (uniforms.fractal_type == 4u) {
                t = burning_ship_hp(coord_hi, coord_lo);
            } else {
                t = tricorn_hp(coord_hi, coord_lo);
            }
        } else {
            // Standard precision coordinate
            coord = vec2<f32>(
                uniforms.center.x + (input.uv.x * 2.0 / uniforms.zoom) * aspect,
                uniforms.center.y + (input.uv.y * 2.0 / uniforms.zoom)
            );

            if (uniforms.fractal_type == 0u) {
                t = mandelbrot(coord);
            } else if (uniforms.fractal_type == 1u) {
                t = julia(coord);
            } else if (uniforms.fractal_type == 2u) {
                t = sierpinski(coord);
            } else if (uniforms.fractal_type == 3u) {
                t = sierpinski_triangle(coord);
            } else if (uniforms.fractal_type == 4u) {
                t = burning_ship(coord);
            } else if (uniforms.fractal_type == 5u) {
                t = tricorn(coord);
            } else if (uniforms.fractal_type == 6u) {
                t = phoenix(coord);
            } else if (uniforms.fractal_type == 7u) {
                t = celtic(coord);
            } else if (uniforms.fractal_type == 8u) {
                t = newton_fractal(coord);
            } else if (uniforms.fractal_type == 9u) {
                t = lyapunov_fractal(coord);
            } else if (uniforms.fractal_type == 10u) {
                t = nova_fractal(coord);
            } else if (uniforms.fractal_type == 11u) {
                t = magnet_fractal(coord);
            } else if (uniforms.fractal_type == 12u) {
                t = collatz_fractal(coord);
            // Strange Attractors (types 26-34, indices after 3D fractals)
            } else if (uniforms.fractal_type == 26u) {
                t = hopalong_attractor(coord);
            } else if (uniforms.fractal_type == 27u) {
                t = henon_attractor(coord);
            } else if (uniforms.fractal_type == 28u) {
                t = martin_attractor(coord);
            } else if (uniforms.fractal_type == 29u) {
                t = gingerbreadman_attractor(coord);
            } else if (uniforms.fractal_type == 30u) {
                t = latoocarfian_attractor(coord);
            } else if (uniforms.fractal_type == 31u) {
                t = chip_attractor(coord);
            } else if (uniforms.fractal_type == 32u) {
                t = quadruptwo_attractor(coord);
            } else if (uniforms.fractal_type == 33u) {
                t = threeply_attractor(coord);
            } else if (uniforms.fractal_type == 34u) {
                t = icon_attractor(coord);
            } else {
                t = collatz_fractal(coord);
            }
        }

        if (t == 0.0) {
            // No post-processing - render raw fractal (post-FX done in multi-pass pipeline)
            return vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }

        var color: vec3<f32>;
        if (uniforms.color_mode == 1u) {
            // Iteration visualization (similar to ray steps)
            color = vec3<f32>(t, t * 0.5, 1.0 - t);
        } else if (uniforms.color_mode == 2u) {
            // Grayscale iteration count
            color = vec3<f32>(t);
        } else if (uniforms.color_mode == 3u) {
            // Orbit Trap XYZ - color based on coordinate components using palette
            let xy_sum = abs(fract(coord.x * uniforms.orbit_trap_scale * 2.0)) + abs(fract(coord.y * uniforms.orbit_trap_scale * 2.0));
            let trap_t = fract(xy_sum / 2.0);
            color = get_palette_color(trap_t);
        } else if (uniforms.color_mode == 4u) {
            // Orbit Trap Radial - color based on distance from origin using palette
            let dist = length(coord);
            let radial_t = fract(dist * uniforms.orbit_trap_scale * 3.0);
            color = get_palette_color(radial_t);
        } else if (uniforms.color_mode == 5u || uniforms.color_mode == 6u) {
            // Position-based coloring for 2D
            color = vec3<f32>(abs(fract(coord.x)), abs(fract(coord.y)), abs(fract(coord.x + coord.y)));
        } else {
            // Palette mode (default)
            color = get_palette_color(t);
        }

        // No post-processing - render raw fractal (post-FX done in multi-pass pipeline)
        return vec4<f32>(color, 1.0);

    } else {
        // 3D Mode
        // UV coordinates are already in NDC space (-1 to 1)
        let ndc_x = input.uv.x;
        let ndc_y = input.uv.y;

        // Unproject a point on the far plane to get initial ray direction
        let far_point = vec4<f32>(ndc_x, ndc_y, 1.0, 1.0);
        var far_world = uniforms.inv_view_proj * far_point;
        far_world = far_world / far_world.w;

        // Initial ray from camera
        let base_ray_origin = uniforms.camera_pos;
        let base_ray_dir = normalize(far_world.xyz - base_ray_origin);

        var final_color: vec3<f32>;

        // Apply depth of field if enabled with multi-sampling
        if (uniforms.depth_of_field == 1u) {
            // Multi-sample DOF - configurable quality vs performance
            let num_samples = uniforms.dof_samples;
            var accumulated_color = vec3<f32>(0.0);

            // Calculate focal point
            let focal_point = base_ray_origin + base_ray_dir * uniforms.dof_focal_length;

            // Calculate camera right and up vectors
            let camera_forward = base_ray_dir;
            let world_up = vec3<f32>(0.0, 1.0, 0.0);
            let camera_right = normalize(cross(camera_forward, world_up));
            let camera_up = cross(camera_right, camera_forward);

            // Take multiple samples
            for (var i = 0u; i < num_samples; i = i + 1u) {
                // Sample aperture with indexed pattern
                let aperture_sample = sample_disk_indexed(input.clip_position.xy, i, num_samples);
                let aperture_offset = (camera_right * aperture_sample.x + camera_up * aperture_sample.y) * uniforms.dof_aperture;

                // Offset ray origin and recalculate direction to focal point
                let ray_origin = base_ray_origin + aperture_offset;
                let ray_dir = normalize(focal_point - ray_origin);

                // Render this ray and accumulate
                accumulated_color = accumulated_color + render_ray(ray_origin, ray_dir, input.uv);
            }

            // Average the samples
            final_color = accumulated_color / f32(num_samples);
        } else {
            // No DOF - single sample
            final_color = render_ray(base_ray_origin, base_ray_dir, input.uv);
        }

        // No post-processing - render raw fractal (post-FX done in multi-pass pipeline)
        return vec4<f32>(final_color, 1.0);
    }
}
