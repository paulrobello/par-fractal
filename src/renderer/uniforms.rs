use crate::camera::Camera;
use crate::fractal::FractalParams;
use bytemuck::{Pod, Zeroable};
use glam::Mat4;

/// Split an f64 into a (hi, lo) f32 pair for double-float arithmetic on the
/// GPU. `hi` is the value rounded to f32; `lo` is the residual
/// `(value - hi as f64)` re-cast to f32. Together they represent the original
/// f64 with ~48 bits of significand precision across the two f32s.
///
/// Extracted from `Uniforms::update` (QA-019) so the deep-zoom foundation can
/// be unit-tested directly; ENH-001's perturbation work builds on this split.
pub(crate) fn split_f64(v: f64) -> (f32, f32) {
    let hi = v as f32;
    let lo = (v - hi as f64) as f32;
    (hi, lo)
}

/// 2D iteration bonus as a function of zoom. Matches the inline computation
/// that used to live in `Uniforms::update`: `floor(log2(zoom.max(1.0)) * 15)`
/// as a u32. Clamped at `zoom >= 1.0` so deep zoom-out does not subtract
/// iterations. Used by the 2D escape-time path only.
///
/// Extracted (QA-019) so the bonus is unit-testable in isolation.
pub fn zoom_iteration_bonus(zoom_2d: f64) -> u32 {
    (zoom_2d.max(1.0).log2() * 15.0) as u32
}

/// The effective 2D iteration count the GPU shader's per-pixel loop runs,
/// combining the user's slider with the zoom bonus and the active LOD
/// quality's `iteration_scale` (then floored at 16 so deep zoom-out never
/// produces a blank render). 3D returns the user value unchanged — 3D cost
/// is dominated by `max_steps`, not iterations.
///
/// The CPU reference orbit (ENH-001 Phase A step 5) MUST be computed against
/// this same value: if it diverges, the shader's `orbit_len` either
/// under-serves the per-pixel loop (pixels needing more iterations than the
/// reference served rebase to noise) or over-serves it (wasted CPU work).
/// Centralizing the derivation here keeps the two paths locked. (QA-019 /
/// ENH-001.)
pub(crate) fn effective_2d_max_iterations(params: &FractalParams) -> u32 {
    let q = params.effective_quality();
    if params.settings.render_mode == crate::fractal::RenderMode::TwoD {
        let zoom_bonus = zoom_iteration_bonus(params.settings.zoom_2d);
        let scaled =
            ((params.settings.max_iterations + zoom_bonus) as f32 * q.iteration_scale) as u32;
        scaled.max(16)
    } else {
        params.settings.max_iterations
    }
}

/// Deterministic max_iter for the perturbation reference orbit: the full 2D
/// budget (user iterations + zoom bonus) WITHOUT LOD's `iteration_scale`.
///
/// Two reasons it must NOT be the LOD-scaled `effective_2d_max_iterations`:
/// (1) Determinism — LOD's scale varies frame-to-frame, so an LOD-scaled orbit
///     triggers continuous recomputes (each a full BigFloat walk) and the
///     async orbit that lands varies run-to-run ⇒ non-deterministic output
///     (verified MAE 117 between runs at 1e8). A fixed budget computes the
///     orbit ONCE.
/// (2) Coverage — the orbit must be at least as long as the shader's loop so
///     the delta path never reads past it. `activate_perturbation` pins the
///     shader's `max_iterations` to the orbit length, so a deterministic orbit
///     length gives a deterministic shader loop. Perturbation engages only at
///     zoom > ~1.6e7 (below that the HP path renders and this isn't called),
///     so not matching the LOD-scaled value at shallow zoom is irrelevant.
pub(crate) fn perturbation_max_iterations(params: &FractalParams) -> u32 {
    if params.settings.render_mode == crate::fractal::RenderMode::TwoD {
        let zoom_bonus = zoom_iteration_bonus(params.settings.zoom_2d);
        (params.settings.max_iterations + zoom_bonus).max(16)
    } else {
        params.settings.max_iterations
    }
}
/// ENH-003: the `scene_uv_scale` post-uniform value — the fraction of
/// `scene_texture` the fractal pass actually wrote — for a given render scale
/// and full-window pixel size. The fractal pass renders into the top-left
/// `floor(size * scale)` sub-rect via `set_viewport`; consumers sample only
/// that sub-rect and let the linear sampler stretch it to fill their output.
///
/// Returns `[1.0, 1.0]` at full resolution so the shader's `scene_sample_uv`
/// is a bit-for-bit no-op (idle / LOD-off / golden frames are untouched).
/// Below 1.0, returns the FLOORED-pixel ratio `[sw/full_w, sh/full_h]` rather
/// than the raw float: the viewport floors to integer pixels, so the sampled
/// region must match that floor exactly or the right/bottom edge shimmers by a
/// texel. `full_w`/`full_h` are clamped to ≥1 so a zero-size surface (before
/// the first resize) cannot divide by zero.
pub(crate) fn scene_uv_scale_for(render_scale: f32, full_w: u32, full_h: u32) -> [f32; 2] {
    if render_scale >= 1.0 {
        return [1.0, 1.0];
    }
    let full_w = full_w.max(1) as f32;
    let full_h = full_h.max(1) as f32;
    let sw = (full_w * render_scale).floor().max(1.0);
    let sh = (full_h * render_scale).floor().max(1.0);
    [sw / full_w, sh / full_h]
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Uniforms {
    // Camera (3D mode)
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    _padding1: f32,

    // 2D fractal parameters
    center: [f32; 2],
    zoom: f32,
    max_iterations: u32,

    // Julia set parameters
    julia_c: [f32; 2],
    fractal_type: u32, // 0: Mandelbrot, 1: Julia, 2: Mandelbulb, 3: Menger
    render_mode: u32,  // 0: 2D, 1: 3D

    // 3D fractal parameters
    power: f32,
    max_steps: u32,
    min_distance: f32,
    fractal_scale: f32,
    fractal_fold: f32,
    fractal_min_radius: f32,
    _padding2: [f32; 2], // Adjusted for alignment

    // Color palette
    palette: [[f32; 4]; 8], // 8 colors with padding

    // Rendering flags
    ambient_occlusion: u32,
    soft_shadows: u32,
    depth_of_field: u32,
    shading_model: u32, // 0: Blinn-Phong, 1: PBR
    color_mode: u32,    // Color visualization mode
    orbit_trap_scale: f32,
    palette_offset: f32,
    channel_r: u32,      // Red channel source
    channel_g: u32,      // Green channel source
    channel_b: u32,      // Blue channel source
    _padding_color: u32, // Padding for 16-byte alignment

    // Material properties
    roughness: f32,
    metallic: f32,
    _padding_vec3_align1: [f32; 3], // WGSL adds 12 bytes to align next vec3 to 16-byte boundary
    _padding_before_albedo: [f32; 3], // Actual vec3 field in WGSL
    _padding_vec3_align2: f32,      // WGSL adds 4 bytes to align next vec3 to 16-byte boundary
    albedo: [f32; 3],
    _padding3: f32,

    // DoF parameters
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
    light_azimuth: f32,       // Horizontal angle in degrees (0-360)
    light_elevation: f32,     // Vertical angle in degrees (5-90)
    _padding_light: [f32; 2], // Maintain 16-byte alignment

    // Floor
    show_floor: u32,
    floor_height: f32,
    _padding_floor: [f32; 2], // Padding for vec3 alignment
    floor_color1: [f32; 3],
    _padding_floor1: f32,
    floor_color2: [f32; 3],
    floor_reflections: u32,
    floor_reflection_strength: f32,
    _padding_floor3_align: [f32; 3], // Explicit padding to match WGSL implicit vec3 alignment to 16-byte boundary
    _padding_floor3: [f32; 3],

    // Ray marching
    use_adaptive_step: u32,
    fixed_step_size: f32,
    step_multiplier: f32,
    max_distance: f32,

    // Fog
    fog_enabled: u32,
    fog_mode: u32, // 0: Linear, 1: Exponential, 2: Quadratic
    fog_density: f32,
    _padding_fog: f32,            // Align to 8-byte boundary
    _padding_fog_vec3_align: f32, // Align fog_color to 16-byte boundary (WGSL requirement)
    fog_color: [f32; 3],
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
    center_hi: [f32; 2],         // High part of center (x, y)
    center_lo: [f32; 2],         // Low part of center (x, y)
    high_precision: u32,         // Flag: 1 = use high precision
    _hp_padding_align: [f32; 3], // WGSL adds 12 bytes implicit padding before vec3 to align to 16-byte boundary
    _hp_padding: [f32; 4],       // vec3 in WGSL (16 bytes with padding)

    // LOD debug visualization
    lod_debug_enabled: u32, // Flag: 1 = show LOD zones as colors
    lod_zone1: f32,         // Distance threshold: Ultra -> High
    lod_zone2: f32,         // Distance threshold: High -> Medium
    lod_zone3: f32,         // Distance threshold: Medium -> Low

    // Aspect ratio stored in a vec4 slot to guarantee 16-byte alignment
    aspect_ratio: [f32; 4], // .x = width/height, others unused

    // Procedural palette parameters
    procedural_palette_type: u32, // 0=None (use static), 1=Firestrm, 2=Rainbow, etc.
    _padding_proc_pal: [u32; 3],  // Align to 16 bytes
    /// Custom procedural palette: brightness (a), contrast (b), frequency (c), phase (d)
    /// color(t) = a + b * cos(2π * (c * t + d))
    procedural_brightness: [f32; 4], // [r, g, b, _]
    procedural_contrast: [f32; 4], // [r, g, b, _]
    procedural_frequency: [f32; 4], // [r, g, b, _]
    procedural_phase: [f32; 4],   // [r, g, b, _]

    // Padding for 16-byte alignment (reduced to accommodate procedural palette)
    _padding_end: [f32; 8], // 32 bytes

    // Perturbation uniforms (ENH-001 Phase A step 3 — plumbing only;
    // perturbation stays OFF, step 5 populates these and uploads a real
    // orbit). Mirrored byte-for-byte by the WGSL `Uniforms` declaration.
    perturbation_enabled: u32, // 0 = OFF (default), 1 = use perturbation delta path
    orbit_len: u32,            // entries of ref_orbit actually populated
    ref_escaped_at: u32,       // index where the reference escaped (0 if bounded)
    _padding_perturb: u32,     // align delta_c_scale to vec2<f32>'s 8-byte boundary
    delta_c_scale: [f32; 2],   // pixel → Δc mapping (per-pixel delta magnitude)
    delta_c_origin: [f32; 2],  // screen-center Δc (normally 0)
}

impl Default for Uniforms {
    fn default() -> Self {
        Self::new()
    }
}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            inv_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 3.0],
            _padding1: 0.0,
            center: [0.0, 0.0],
            zoom: 1.0,
            max_iterations: 80,
            julia_c: [-0.7, 0.27015],
            fractal_type: 0,
            render_mode: 0,
            power: 8.0,
            max_steps: 325,
            min_distance: 0.00035,
            fractal_scale: 2.0,
            fractal_fold: 1.0,
            fractal_min_radius: 0.5,
            _padding2: [0.0; 2],
            palette: [[0.0; 4]; 8],
            ambient_occlusion: 1,
            soft_shadows: 1,
            depth_of_field: 0,
            shading_model: 1, // PBR
            color_mode: 0,    // Palette
            orbit_trap_scale: 1.0,
            palette_offset: 0.0,
            channel_r: 0, // Iterations
            channel_g: 1, // Distance
            channel_b: 4, // PositionZ
            _padding_color: 0,
            roughness: 0.3,
            metallic: 0.15,
            _padding_vec3_align1: [0.0, 0.0, 0.0],
            _padding_before_albedo: [0.0, 0.0, 0.0],
            _padding_vec3_align2: 0.0,
            albedo: [0.8, 0.8, 0.8],
            _padding3: 0.0,
            dof_focal_length: 5.0,
            dof_aperture: 0.1,
            dof_samples: 2,
            time: 0.0,
            light_intensity: 4.5,
            ambient_light: 0.15,
            ao_intensity: 3.0,
            ao_step_size: 0.12,
            shadow_softness: 8.0,
            shadow_max_distance: 5.0,
            shadow_samples: 128,
            shadow_step_factor: 0.6,

            light_azimuth: 45.0,
            light_elevation: 60.0,
            _padding_light: [0.0; 2],

            show_floor: 1,
            floor_height: -2.0,
            _padding_floor: [0.0; 2],
            floor_color1: [1.0, 1.0, 1.0], // White
            _padding_floor1: 0.0,
            floor_color2: [0.0, 0.0, 0.0], // Black
            floor_reflections: 0,
            floor_reflection_strength: 0.7,
            _padding_floor3_align: [0.0; 3],
            _padding_floor3: [0.0; 3],

            use_adaptive_step: 1,
            fixed_step_size: 0.1,
            step_multiplier: 0.7,
            max_distance: 100.0,

            fog_enabled: 0,
            fog_mode: 1, // Exponential
            fog_density: 0.001,
            _padding_fog: 0.0,
            _padding_fog_vec3_align: 0.0,
            fog_color: [0.2, 0.2, 0.2], // Dark grey
            _padding_fog_color: 0.0,

            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            hue_shift: 0.0,
            vignette_enabled: 0,
            vignette_intensity: 0.5,
            vignette_radius: 0.8,
            bloom_enabled: 0,
            bloom_threshold: 0.7,
            bloom_intensity: 0.5,
            bloom_radius: 0.005,
            fxaa_enabled: 0,

            center_hi: [0.0, 0.0],
            center_lo: [0.0, 0.0],
            high_precision: 0,
            _hp_padding_align: [0.0; 3],
            _hp_padding: [0.0; 4],

            lod_debug_enabled: 0,
            // QA-025: shared with `LODConfig::default()` via `DEFAULT_LOD_ZONES`
            // so the CPU-side defaults and the GPU-side uniform defaults cannot
            // drift apart.
            lod_zone1: crate::lod::DEFAULT_LOD_ZONES[0],
            lod_zone2: crate::lod::DEFAULT_LOD_ZONES[1],
            lod_zone3: crate::lod::DEFAULT_LOD_ZONES[2],

            aspect_ratio: [16.0 / 9.0, 0.0, 0.0, 0.0], // Default aspect ratio

            // Procedural palette defaults
            procedural_palette_type: 0, // None (use static palette)
            _padding_proc_pal: [0; 3],
            procedural_brightness: [0.5, 0.5, 0.5, 0.0],
            procedural_contrast: [0.5, 0.5, 0.5, 0.0],
            procedural_frequency: [1.0, 1.0, 1.0, 0.0],
            procedural_phase: [0.0, 0.333, 0.667, 0.0],

            _padding_end: [0.0; 8],

            // ENH-001 Phase A step 3: perturbation OFF by default. Step 5
            // will populate these from a computed reference orbit.
            perturbation_enabled: 0,
            orbit_len: 0,
            ref_escaped_at: 0,
            _padding_perturb: 0,
            delta_c_scale: [0.0, 0.0],
            delta_c_origin: [0.0, 0.0],
        }
    }

    pub fn update(&mut self, camera: &Camera, params: &FractalParams, time: f32) {
        let view_proj = camera.build_view_projection_matrix();
        self.view_proj = view_proj.to_cols_array_2d();
        self.inv_view_proj = view_proj.inverse().to_cols_array_2d();
        self.camera_pos = camera.position.into();

        self.center = [
            params.settings.center_2d[0] as f32,
            params.settings.center_2d[1] as f32,
        ];
        // ARC-001: zoom_2d is f64 CPU-side; the GPU uniform stays f32 (casting here at
        // the boundary). The f32 GPU zoom is the remaining precision limiter; the
        // double-float center (hi/lo) is what actually extends the on-GPU ceiling.
        self.zoom = params.settings.zoom_2d as f32;
        self.aspect_ratio[0] = camera.aspect;

        // High-precision center: split f64 into (hi, lo) pair.
        //
        // hp auto-enable threshold (QA-004 / ARC-002). The previous `> 1e6` rule
        // engaged double-float two decades too late: f32 per-pixel spacing near a
        // unit-magnitude center collapses at zoom ~2e4–6e4 at 1080p, so by 1e6 the
        // image was already badly quantized. The derived form would scale pixel
        // spacing against `center_mag * f32::EPSILON * safety`, but plumbing window
        // height into `Uniforms::update` (which only sees `camera.aspect` today) is
        // out of scope for this bundle; the simple 1e4 threshold lands two decades
        // earlier at the conservative end of the visible-quantization range. hp is
        // ~10–20× slower — drop the constant toward ~1e3 only if perf regresses at
        // moderate zoom and the derived criterion is not yet plumbed.
        const HP_ZOOM_THRESHOLD: f64 = 1e4;
        let use_high_precision = params.settings.zoom_2d > HP_ZOOM_THRESHOLD;
        self.high_precision = if use_high_precision { 1 } else { 0 };

        // Split center coordinates into double-float pairs via the testable
        // helper. `split_f64` is exact-in-f32 by construction: `hi` is the
        // rounded f32 cast, `lo` captures the residual in f32 precision. This
        // is the correctness foundation for the ENH-001 perturbation work.
        let center_x = params.settings.center_2d[0];
        let center_y = params.settings.center_2d[1];
        let (hi_x, lo_x) = split_f64(center_x);
        let (hi_y, lo_y) = split_f64(center_y);
        self.center_hi = [hi_x, hi_y];
        self.center_lo = [lo_x, lo_y];

        // Auto-scale iterations with zoom for 2D fractals, combined with user
        // slider. ARC-007: also fold in the LOD `iteration_scale`, applied
        // AFTER the zoom bonus (so deep zoom does not re-lose detail it just
        // paid for) and floored at ≥16 to avoid blank renders at low quality.
        // 2D-only: 3D ray-march cost is dominated by `max_steps`, not iters.
        //
        // ENH-001 Phase A step 5: the CPU reference orbit (deep_zoom driver)
        // is computed against this same value via `effective_2d_max_iterations`,
        // so the shader's `orbit_len` always indexes a fully-served budget.
        // The shared helper is the single source of truth — keeping the
        // inline math here would let the two paths drift.
        let q = params.effective_quality();
        self.max_iterations = effective_2d_max_iterations(params);
        self.julia_c = params.settings.julia_c;

        // QA-017: enum→u32 via `#[repr(u32)]` discriminants (see
        // `fractal/types.rs` and the `gpu_discriminant_roundtrip` test, which
        // pins the wire-format IDs the shader reads). The match table that
        // used to live here was the GPU-contract source of truth; the
        // discriminants now are, and the test guards against drift.
        self.fractal_type = params.settings.fractal_type as u32;

        self.render_mode = params.settings.render_mode as u32;

        self.power = params.settings.power;
        // ARC-008: LOD no longer mutates FractalParams. The user's slider values
        // stay authored; we merge with the LOD-active `QualityLevel` here at
        // uniform-build time. Take the *cheaper* direction on every cost axis:
        //   - smaller sample/step counts → fewer GPU iterations
        //   - larger `min_distance`       → coarser surface precision
        //   - larger `shadow_step_factor` → less precise shadow rays
        //   - larger `ao_step_size`       → coarser AO sampling
        // This is the single place LOD quality influences the GPU.
        // (`q` was computed above, before the 2D iteration scaling.)
        self.max_steps = params.settings.max_steps.min(q.max_steps);
        self.min_distance = params.settings.min_distance.max(q.min_distance);
        // Pass scale parameters directly - each fractal handles them appropriately
        self.fractal_scale = params.settings.fractal_scale;
        self.fractal_fold = params.settings.fractal_fold;
        self.fractal_min_radius = params.settings.fractal_min_radius;

        // Update palette
        for (i, color) in params.settings.palette.colors.iter().enumerate() {
            self.palette[i] = [color.x, color.y, color.z, 1.0];
        }

        // Update procedural palette
        self.procedural_palette_type = params.settings.procedural_palette.shader_index();
        self.procedural_brightness = [
            params.settings.procedural_brightness[0],
            params.settings.procedural_brightness[1],
            params.settings.procedural_brightness[2],
            0.0,
        ];
        self.procedural_contrast = [
            params.settings.procedural_contrast[0],
            params.settings.procedural_contrast[1],
            params.settings.procedural_contrast[2],
            0.0,
        ];
        self.procedural_frequency = [
            params.settings.procedural_frequency[0],
            params.settings.procedural_frequency[1],
            params.settings.procedural_frequency[2],
            0.0,
        ];
        self.procedural_phase = [
            params.settings.procedural_phase[0],
            params.settings.procedural_phase[1],
            params.settings.procedural_phase[2],
            0.0,
        ];

        self.ambient_occlusion = if params.settings.ambient_occlusion {
            1
        } else {
            0
        };
        // shadow_mode: 0=off,1=hard,2=soft; pass through for shader
        self.soft_shadows = params.settings.shadow_mode;
        self.depth_of_field = if params.settings.depth_of_field { 1 } else { 0 };
        self.shading_model = params.settings.shading_model as u32;

        self.color_mode = params.settings.color_mode as u32;

        self.orbit_trap_scale = params.settings.orbit_trap_scale;
        self.palette_offset = params.settings.palette_offset;

        // Channel sources — same `as u32` mapping for all three channels
        // (discriminant contract; see `fractal/types.rs`).
        self.channel_r = params.settings.channel_r as u32;
        self.channel_g = params.settings.channel_g as u32;
        self.channel_b = params.settings.channel_b as u32;

        self.roughness = params.settings.roughness;
        self.metallic = params.settings.metallic;
        self.albedo = params.settings.albedo.into();

        self.dof_focal_length = params.settings.dof_focal_length;
        self.dof_aperture = params.settings.dof_aperture;
        self.dof_samples = params.settings.dof_samples.min(q.dof_samples);
        self.time = time;
        self.light_intensity = params.settings.light_intensity;
        self.ambient_light = params.settings.ambient_light;
        self.ao_intensity = params.settings.ao_intensity;
        self.ao_step_size = params.settings.ao_step_size.max(q.ao_step_size);
        self.shadow_softness = params.settings.shadow_softness;
        self.shadow_max_distance = params.settings.shadow_max_distance;
        self.shadow_samples = params.settings.shadow_samples.min(q.shadow_samples);
        self.shadow_step_factor = params.settings.shadow_step_factor.max(q.shadow_step_factor);

        self.light_azimuth = params.settings.light_azimuth;
        self.light_elevation = params.settings.light_elevation;

        self.show_floor = if params.settings.show_floor { 1 } else { 0 };
        self.floor_height = params.settings.floor_height;
        self.floor_color1 = params.settings.floor_color1.into();
        self.floor_color2 = params.settings.floor_color2.into();
        self.floor_reflections = if params.settings.floor_reflections {
            1
        } else {
            0
        };
        self.floor_reflection_strength = params.settings.floor_reflection_strength;

        self.use_adaptive_step = if params.settings.use_adaptive_step {
            1
        } else {
            0
        };
        self.fixed_step_size = params.settings.fixed_step_size;
        self.step_multiplier = params.settings.step_multiplier;
        self.max_distance = params.settings.max_distance;

        self.fog_enabled = if params.settings.fog_enabled { 1 } else { 0 };
        self.fog_mode = params.settings.fog_mode as u32;
        self.fog_density = params.settings.fog_density;
        self.fog_color = params.settings.fog_color.into();

        // Post-processing
        self.brightness = params.settings.brightness;
        self.contrast = params.settings.contrast;
        self.saturation = params.settings.saturation;
        self.hue_shift = params.settings.hue_shift;
        self.vignette_enabled = if params.settings.vignette_enabled {
            1
        } else {
            0
        };
        self.vignette_intensity = params.settings.vignette_intensity;
        self.vignette_radius = params.settings.vignette_radius;
        self.bloom_enabled = if params.settings.bloom_enabled { 1 } else { 0 };
        self.bloom_threshold = params.settings.bloom_threshold;
        self.bloom_intensity = params.settings.bloom_intensity;
        self.bloom_radius = params.settings.bloom_radius;
        self.fxaa_enabled = if params.settings.fxaa_enabled { 1 } else { 0 };

        // LOD debug visualization
        let lod_enabled =
            params.lod.lod_config.enabled && params.lod.lod_config.debug_visualization;
        self.lod_debug_enabled = if lod_enabled { 1 } else { 0 };
        self.lod_zone1 = params.lod.lod_config.distance_zones[0];
        self.lod_zone2 = params.lod.lod_config.distance_zones[1];
        self.lod_zone3 = params.lod.lod_config.distance_zones[2];

        // ENH-001 Phase A step 5: perturbation uniforms default to OFF here.
        // `Renderer::update` overlays non-zero values AFTER this call when
        // an active orbit has been uploaded (`Renderer::active_orbit`) AND
        // the gate is met (`perturbation_eligible`). Leaving them at zero
        // keeps the perturbation shader branch disabled (and `orbit_len`
        // short-circuits any accidental orbit read) on frames where the
        // orbit is stale, the gate isn't met, or the worker is still
        // computing — the HP path renders those frames.
        self.perturbation_enabled = 0;
        self.orbit_len = 0;
        self.ref_escaped_at = 0;
        self.delta_c_scale = [0.0, 0.0];
        self.delta_c_origin = [0.0, 0.0];
    }

    /// Creates a new Uniforms struct populated from camera and fractal parameters.
    /// This is useful for high-resolution rendering where we need immutable access to the renderer.
    #[cfg(target_arch = "wasm32")]
    pub fn from_camera_and_params(camera: &Camera, params: &FractalParams, time: f32) -> Self {
        let mut uniforms = Self::new();
        uniforms.update(camera, params, time);
        uniforms
    }

    /// ENH-001 Phase A step 5: populate the perturbation uniforms for an
    /// active orbit + the current view.
    ///
    /// Called by [`crate::renderer::Renderer::update`] AFTER [`Self::update`]
    /// when an orbit has been uploaded AND the activation gate is met; on
    /// every other frame `update()` leaves these zeroed and the shader's
    /// delta path stays disabled (HP / f32 path renders).
    ///
    /// `delta_c_scale` is the per-pixel Δc magnitude. It matches the
    /// shader's existing HP coordinate mapping — `uv * 2/zoom * aspect` for
    /// x, `uv * 2/zoom` for y — but computed here on the CPU so the value
    /// is identical to what the shader uses for its non-perturbation offset.
    /// The `2.0 / zoom` term is computed in f64 FIRST and cast to f32 only
    /// at the end: the spacing is small but representable in f32, and only
    /// the absolute center is unrepresentable in f32 (which is the whole
    /// point of perturbation).
    ///
    /// `reference_offset` is `c_center − c_ref` for the chosen reference
    /// orbit (an f32 pair, already computed in f64 on the worker before the
    /// cast). It flows directly into `delta_c_origin`: the shader computes
    /// `delta_c = delta_c_origin + uv·delta_c_scale = (c_center − c_ref) +
    /// (c_pixel − c_center) = c_pixel − c_ref`, which is the per-pixel Δc
    /// the delta recurrence needs. `[0,0]` when the reference IS the view
    /// center (the original Phase A behavior).
    pub(crate) fn activate_perturbation(
        &mut self,
        orbit_len: u32,
        ref_escaped_at: u32,
        zoom_2d: f64,
        aspect: f32,
        reference_offset: [f32; 2],
    ) {
        let inv_zoom_f64 = 2.0 / zoom_2d;
        let aspect_f64 = aspect as f64;
        self.perturbation_enabled = 1;
        self.orbit_len = orbit_len;
        // Pin the shader's loop to the orbit's length EXACTLY. The orbit is
        // computed async and LOD's `iteration_scale` varies between spawn
        // (`App::update`) and render, so an LOD-derived `max_iterations` desyncs
        // from the uploaded orbit — the 2026-07-18 deep-zoom root cause
        // (orbit=89 vs shader=329 ⇒ 240 iterations of rebasing ⇒ wrong output).
        // Pinning here makes orbit and shader agree deterministically, whatever
        // LOD state either side sees.
        self.max_iterations = orbit_len;
        self.ref_escaped_at = ref_escaped_at;
        self.delta_c_scale = [(inv_zoom_f64 * aspect_f64) as f32, inv_zoom_f64 as f32];
        self.delta_c_origin = reference_offset;
    }
}

// Compile-time assertion to ensure struct size matches WGSL expectations
const _: () = assert!(
    std::mem::size_of::<Uniforms>() == 896,
    "Uniforms struct must be exactly 896 bytes"
);

// Post-processing uniform structs
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub(super) struct BloomUniforms {
    pub(super) threshold: f32,
    pub(super) intensity: f32,
    /// ENH-003: fraction of `scene_texture` the fractal pass actually wrote
    /// (sub-rect viewport during LOD motion). `[1.0, 1.0]` at full resolution
    /// — the bloom-extract shader's `scene_sample_uv` is a no-op then. Repurposes
    /// the prior `_padding` slot, so the struct stays 16 bytes.
    pub(super) scene_uv_scale: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(super) struct BlurUniforms {
    pub(super) direction: [f32; 2], // (1,0) for horizontal, (0,1) for vertical
    pub(super) _padding: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub(super) struct PostProcessUniforms {
    pub(super) brightness: f32, // offset 0
    pub(super) contrast: f32,   // offset 4
    pub(super) saturation: f32, // offset 8
    pub(super) hue_shift: f32,  // offset 12

    pub(super) vignette_enabled: u32,   // offset 16
    pub(super) vignette_intensity: f32, // offset 20
    pub(super) vignette_radius: f32,    // offset 24
    pub(super) _padding1: f32,          // offset 28 (align to 16 bytes)

    pub(super) bloom_enabled: u32,   // offset 32
    pub(super) bloom_intensity: f32, // offset 36
    pub(super) _padding2: [f32; 2],  // offset 40 (pad to 48)

    pub(super) scene_uv_scale: [f32; 2], // offset 48 — ENH-003: sub-rect of scene_texture the fractal pass wrote ([1,1] = full res; composite's scene_sample_uv is a no-op then). Repurposes half of the prior _padding3 vec4.
    pub(super) _padding3: [f32; 2],      // offset 56 (pad to 64)
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use std::mem::offset_of;

    /// Locks the `Uniforms` Rust struct to the WGSL `Uniforms` declaration in
    /// `src/shaders/fractal.wgsl` (lines 1-148). Any field reorder, add, or
    /// remove breaks this test, forcing the change to be deliberate and paired
    /// with a matching WGSL edit.
    ///
    /// The compile-time `size_of == 896` assert above catches total-size
    /// drift but misses equal-size field swaps; offset asserts catch those.
    ///
    /// Expected offsets were cross-checked against the WGSL declaration by
    /// summing WGSL host-shareable layout rules:
    ///   - mat4x4<f32>: 64B, align 16
    ///   - vec4<f32>:   16B, align 16
    ///   - vec3<f32>:   16B effective in uniform structs (12B data + 4B pad)
    ///   - vec2<f32>:   8B,  align 8
    ///   - f32/u32:     4B,  align 4
    ///   - array<vec4<f32>, N>: stride 16, align 16
    ///
    /// Sentinels are spread across the struct (early, middle, late, trailing)
    /// so an insertion anywhere shifts at least one of them.
    #[test]
    fn wgsl_layout_contract() {
        assert_eq!(std::mem::size_of::<Uniforms>(), 896);

        // 2D-params row: center vec2 (8) + zoom f32 (4) + max_iterations u32 (4),
        // preceded by camera_pos vec3 + _padding1 = offset 144.
        assert_eq!(offset_of!(Uniforms, max_iterations), 156);

        // 3D-params row: power f32 + max_steps u32 + min_distance f32 + fractal_scale f32.
        assert_eq!(offset_of!(Uniforms, max_steps), 180);

        // palette: array<vec4<f32>, 8> — must start on 16B boundary; comes right
        // after the power/max_steps/min_distance/fractal_scale/fractal_fold/
        // fractal_min_radius/_padding2 block ending at offset 208.
        assert_eq!(offset_of!(Uniforms, palette), 208);

        // High-precision center block, immediately after the post-processing
        // scalars (which end at fxaa_enabled @ 668 + 4 = 672).
        assert_eq!(offset_of!(Uniforms, center_hi), 672);

        // aspect_ratio: vec4<f32> stored in an explicit 16B slot, after the
        // four LOD-debug scalars (lod_debug_enabled + 3× lod_zoneN = 16B).
        assert_eq!(offset_of!(Uniforms, aspect_ratio), 736);

        // procedural_phase: last vec4 in the procedural-palette block, just
        // before the trailing 32B _padding_end.
        assert_eq!(offset_of!(Uniforms, procedural_phase), 816);

        // ENH-001 Phase A step 3: perturbation block, appended after
        // _padding_end (which ends at offset 832 + 32 = 864). The three
        // u32 flags pack into the first 16B row; `_padding_perturb` is the
        // explicit 4B pad that lifts `delta_c_scale` to vec2<f32>'s 8-byte
        // alignment boundary (Rust's `#[repr(C)]` aligns `[f32; 2]` to 4,
        // so without this pad the Rust and WGSL layouts would diverge).
        assert_eq!(offset_of!(Uniforms, perturbation_enabled), 864);
        assert_eq!(offset_of!(Uniforms, orbit_len), 868);
        assert_eq!(offset_of!(Uniforms, ref_escaped_at), 872);
        assert_eq!(offset_of!(Uniforms, _padding_perturb), 876);
        assert_eq!(offset_of!(Uniforms, delta_c_scale), 880);
        assert_eq!(offset_of!(Uniforms, delta_c_origin), 888);
    }

    /// ENH-003: lock the post-processing uniform layouts. `scene_uv_scale`
    /// repurposes pre-existing padding, so the struct sizes must NOT change
    /// (16B / 64B) and the new field lands at the documented offset.
    #[test]
    fn post_uniform_layout_contract() {
        assert_eq!(std::mem::size_of::<BloomUniforms>(), 16);
        assert_eq!(offset_of!(BloomUniforms, threshold), 0);
        assert_eq!(offset_of!(BloomUniforms, intensity), 4);
        assert_eq!(offset_of!(BloomUniforms, scene_uv_scale), 8);

        assert_eq!(std::mem::size_of::<PostProcessUniforms>(), 64);
        assert_eq!(offset_of!(PostProcessUniforms, scene_uv_scale), 48);
        assert_eq!(offset_of!(PostProcessUniforms, _padding3), 56);
    }
}

/// Numeric-core tests for the deep-zoom / iteration helpers extracted out of
/// `Uniforms::update` (QA-019). These are the correctness foundation for
/// ENH-001's perturbation work — keep them strict.
#[cfg(test)]
mod numeric_tests {
    use super::*;

    /// Reference implementation of the f32 unit-in-the-last-place at `x`,
    /// used to bound `lo`'s magnitude in the DF split.
    fn f32_ulp(x: f32) -> f32 {
        // ulp(x) = 2^(floor(log2(|x|)) - 23) for normal numbers; fall back to
        // the smallest positive subnormal for 0.0/ subnormals.
        if x == 0.0 || !x.is_normal() {
            return f32::MIN_POSITIVE; // smallest positive normal as a safe upper bound
        }
        let abs_x = x.abs();
        // f32::EPSILON == 2^-23; spacing at x is approximately x * EPSILON for
        // values in [1, 2). For arbitrary magnitude, multiply by the power of
        // two bracketing x via raw bit manipulation.
        let bits = abs_x.to_bits();
        // Exponent mask: bits 23..30. ulp = 2^(exp - 23). For abs_x in
        // [2^exp, 2^(exp+1)), spacing is 2^(exp - 23).
        let exp = ((bits >> 23) & 0xFF) as i32 - 127; // unbiased exponent
        2f32.powi(exp - 23)
    }

    /// DF split roundtrip: across a sweep of f64 magnitudes spanning zero,
    /// unit, π, deep zoom, and large values, `hi + lo` must reconstruct the
    /// original to within f64 relative error 1e-14, and `|lo|` must not exceed
    /// half an ulp of `hi`. The ulp bound is the defining property of a
    /// correctly-rounded two-sum split.
    ///
    /// The sweep is bounded to the range that casts losslessly into f32
    /// (~[1.2e-38, 3.4e38]); outside that range `hi` becomes inf/0 and the
    /// split is meaningless (not a bug — `split_f64` is only called on
    /// center coordinates in deep-zoom range).
    #[test]
    fn df_split_roundtrip_strict() {
        let values: &[f64] = &[
            0.0,
            1.0,
            -1.0,
            std::f64::consts::PI,
            -std::f64::consts::PI,
            1e-15,
            -1e-15,
            0.1318259042, // canonical Mandelbrot seahorse valley y-coordinate
            -0.1318259042,
            1e10,
            1e-10,
            -1e10,
            1e30, // SEC-001 upper clamp on zoom-derived coords
            1e-30,
        ];
        for &v in values {
            let (hi, lo) = split_f64(v);
            let reconstructed = (hi as f64) + (lo as f64);
            let abs_err = (reconstructed - v).abs();
            // Strict relative bound; for v == 0.0 the absolute error is exactly
            // 0.0 (split_f64(0.0) == (0.0, 0.0)), so the relative check is
            // trivially satisfied — no division-by-zero special case needed.
            let rel_err = if v == 0.0 { abs_err } else { abs_err / v.abs() };
            assert!(
                rel_err < 1e-14,
                "split_f64({v:e}): reconstructed={reconstructed:e}, rel_err={rel_err:e}",
            );

            // |lo| must be ≤ ulp(hi)/2. ulp(hi)/2 is the maximum rounding
            // error of an f32 cast; lo is precisely that residual.
            let ulp = f32_ulp(hi);
            assert!(
                lo.abs() <= ulp / 2.0,
                "split_f64({v:e}): |lo|={lo:e} exceeds ulp(hi)/2 = {}",
                ulp / 2.0,
            );
        }
    }

    /// Zoom→iteration bonus: across the 2D zoom range (no zoom, moderate,
    /// deep), the bonus matches `floor(log2(zoom) * 15)` exactly and the total
    /// (`max_iterations + bonus`) never overflows u32 even at extreme values.
    #[test]
    fn zoom_iteration_bonus_matches_formula_and_no_overflow() {
        // (zoom, expected_bonus) pairs. expected_bonus = floor(log2(zoom) * 15).
        let cases: &[(f64, u32)] = &[
            (1.0, 0),                               // log2(1)=0
            (1e6, (1e6f64.log2() * 15.0) as u32),   // ~299
            (1e12, (1e12f64.log2() * 15.0) as u32), // ~597
            (2.0, 15),                              // exactly one octave
            (4.0, 30),                              // two octaves
        ];
        for &(zoom, expected) in cases {
            let got = zoom_iteration_bonus(zoom);
            assert_eq!(
                got, expected,
                "zoom_iteration_bonus({zoom:e}) = {got}, expected {expected}",
            );
        }

        // zoom < 1.0 must clamp to 0 bonus (no negative iteration counts).
        assert_eq!(zoom_iteration_bonus(0.5), 0);
        assert_eq!(zoom_iteration_bonus(1e-6), 0);

        // No u32 overflow at the realistic maximum user-facing zoom (SEC-001
        // clamps `zoom_2d` to ≤ 1e30). log2(1e30)*15 ≈ 1495, far below u32::MAX.
        let extreme = zoom_iteration_bonus(1e30);
        assert!(
            extreme < u32::MAX / 2,
            "extreme bonus near overflow: {extreme}"
        );

        // Combined with a high user iteration count, the total still fits u32.
        let max_user_iters: u32 = 100_000; // SEC-001 upper bound
        let total = max_user_iters + extreme;
        assert!(total < u32::MAX, "iteration total overflows u32: {total}",);
    }

    /// ENH-003: `scene_uv_scale_for` must (1) be exactly [1,1] at full
    /// resolution so idle/golden frames are untouched, (2) round DOWN to the
    /// floored pixel sub-rect (never exceed the viewport floor — that would
    /// sample the cleared margin), and (3) survive a zero-size surface without
    /// dividing by zero.
    #[test]
    fn scene_uv_scale_matches_floored_viewport() {
        // Full resolution is a perfect no-op.
        assert_eq!(scene_uv_scale_for(1.0, 1920, 1080), [1.0, 1.0]);
        // Values ≥1.0 clamp to the no-op too (no super-sampling).
        assert_eq!(scene_uv_scale_for(1.5, 1920, 1080), [1.0, 1.0]);

        // Half scale on 1920×1080: floor(960)=960, floor(540)=540 → exact halves.
        assert_eq!(scene_uv_scale_for(0.5, 1920, 1080), [0.5, 0.5]);

        // 0.7 on an odd width (1921): floor(1921*0.7)=floor(1344.7)=1344.
        // 1344/1921 ≈ 0.6996 — strictly less than 0.7 (never exceeds the floor).
        let [sx, _] = scene_uv_scale_for(0.7, 1921, 1080);
        assert!(
            sx < 0.7,
            "scene_uv_scale.x {sx} must not exceed the raw scale"
        );
        assert!(sx > 0.69, "scene_uv_scale.x {sx} lost too much precision");
        assert_eq!(sx, 1344.0 / 1921.0);

        // Zero-size surface (before first resize) must not panic or produce NaN.
        let z = scene_uv_scale_for(0.5, 0, 0);
        assert!(z[0].is_finite() && z[1].is_finite());
    }
}
