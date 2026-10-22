// ENH-008: the ~14 `_padding_*` fields are gone — encase (Rust) and WGSL
// layout rules independently derive the same offsets from this field order, so
// both sides agree without hand-maintained padding. Keep this declaration's
// field ORDER identical to the Rust `Uniforms` struct (`renderer/uniforms.rs`);
// add new fields at the end here and in Rust, no padding math required.
struct Uniforms {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,

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

    roughness: f32,
    metallic: f32,
    albedo: vec3<f32>,

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

    show_floor: u32,
    floor_height: f32,
    floor_color1: vec3<f32>,
    floor_color2: vec3<f32>,
    floor_reflections: u32,
    floor_reflection_strength: f32,

    use_adaptive_step: u32,
    fixed_step_size: f32,
    step_multiplier: f32,
    max_distance: f32,

    fog_enabled: u32,
    fog_mode: u32,  // 0: Linear, 1: Exponential, 2: Quadratic
    fog_density: f32,
    fog_color: vec3<f32>,

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

    // LOD debug visualization
    lod_debug_enabled: u32,  // Flag: 1 = show LOD zones as colors
    lod_zone1: f32,          // Distance threshold: Ultra -> High
    lod_zone2: f32,          // Distance threshold: High -> Medium
    lod_zone3: f32,          // Distance threshold: Medium -> Low

    // Aspect ratio stored in a vec4 slot to guarantee 16-byte alignment
    aspect_ratio: vec4<f32>, // .x = width/height, others unused

    // Procedural palette parameters
    procedural_palette_type: u32, // 0=None (use static), 1=Firestrm, 2=Rainbow, etc.
    // Custom procedural palette: color(t) = brightness + contrast * cos(2π * (frequency * t + phase))
    procedural_brightness: vec4<f32>, // [r, g, b, _]
    procedural_contrast: vec4<f32>,   // [r, g, b, _]
    procedural_frequency: vec4<f32>,  // [r, g, b, _]
    procedural_phase: vec4<f32>,      // [r, g, b, _]

    // Perturbation uniforms (ENH-001 Phase A step 3 — plumbing only;
    // perturbation stays OFF here, step 5 populates these and uploads a
    // real orbit). Mirrored by the Rust `Uniforms` struct.
    perturbation_enabled: u32, // 0 = OFF (default), 1 = use perturbation delta path
    orbit_len: u32,            // entries of ref_orbit actually populated
    ref_escaped_at: u32,       // index where the reference escaped (0 if bounded)
    delta_c_scale: vec2<f32>,  // pixel → Δc mapping (per-pixel delta magnitude)
    delta_c_origin: vec2<f32>, // screen-center Δc (normally 0)
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// ENH-001 Phase A step 3: perturbation reference orbit storage buffer.
// Read-only; bound at group(0) binding(1) so it shares the same bind group
// as the uniforms (both pipeline_2d and pipeline_3d share group 0).
// `fs_main_2d` will read this when `perturbation_enabled == 1` (step 4);
// `fs_main_3d` declares but does not use it (dead-code-eliminated per
// entry point). Unused in step 3 — perturbation is OFF.
@group(0) @binding(1)
var<storage, read> ref_orbit: array<vec2<f32>>;
