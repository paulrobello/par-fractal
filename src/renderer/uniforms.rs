use crate::camera::Camera;
use crate::fractal::FractalParams;
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
///     length gives a deterministic shader loop. Perturbation engages at
///     zoom > ~1e4 (PERTURBATION_LOG2_GATE; below it f32 renders and this isn't
///     called), so not matching the LOD-scaled value at shallow zoom is irrelevant.
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

/// ENH-008: serialize a `ShaderType` value into WGSL-uniform-layout bytes via
/// encase. Replaces `bytemuck::cast_slice(&[value])` at every uniform upload
/// site for structs that have migrated to `#[derive(encase::ShaderType)]`.
///
/// One-shot `Vec<u8>` per call. The per-frame uploads (main `Uniforms`,
/// bloom/composite post uniforms) run at most once per rendered frame and
/// ENH-002 skips re-render while idle, so this is not on a tight inner loop;
/// if profiling later flags it, switch the hot callers to a reused scratch
/// `UniformBuffer`.
pub(crate) fn write_uniform_bytes<T>(value: &T) -> Vec<u8>
where
    T: encase::ShaderType + encase::internal::WriteInto,
{
    let mut buf = encase::UniformBuffer::new(Vec::<u8>::new());
    buf.write(value)
        .expect("encase uniform layout write failed");
    buf.into_inner()
}

/// ENH-008: derives `encase::ShaderType` — WGSL-correct uniform layout is
/// computed by encase from the field types + declaration order, so the ~14
/// hand-maintained `_padding_*` fields are gone (from both this struct and the
/// `Uniforms` declaration in `shaders/fractal.wgsl`). vec/mat fields use glam
/// types (glam's `encase` feature provides the `ShaderType` impls). Field
/// ORDER must stay identical to the WGSL struct: encase and WGSL derive the
/// same offsets from the same order, so both sides agree. Add new fields at
/// the end (paired with a matching WGSL edit); no manual padding math needed.
#[derive(Copy, Clone, Debug, encase::ShaderType)]
pub struct Uniforms {
    // Camera (3D mode)
    view_proj: glam::Mat4,
    inv_view_proj: glam::Mat4,
    camera_pos: glam::Vec3,

    // 2D fractal parameters
    center: glam::Vec2,
    zoom: f32,
    max_iterations: u32,

    // Julia set parameters
    julia_c: glam::Vec2,
    fractal_type: u32, // 0: Mandelbrot, 1: Julia, 2: Mandelbulb, 3: Menger
    render_mode: u32,  // 0: 2D, 1: 3D

    // 3D fractal parameters
    power: f32,
    max_steps: u32,
    min_distance: f32,
    fractal_scale: f32,
    fractal_fold: f32,
    fractal_min_radius: f32,

    // Color palette
    palette: [glam::Vec4; 8], // 8 colors

    // Rendering flags
    ambient_occlusion: u32,
    soft_shadows: u32,
    depth_of_field: u32,
    shading_model: u32, // 0: Blinn-Phong, 1: PBR
    color_mode: u32,    // Color visualization mode
    orbit_trap_scale: f32,
    palette_offset: f32,
    channel_r: u32, // Red channel source
    channel_g: u32, // Green channel source
    channel_b: u32, // Blue channel source

    // Material properties
    roughness: f32,
    metallic: f32,
    albedo: glam::Vec3,

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
    light_azimuth: f32,   // Horizontal angle in degrees (0-360)
    light_elevation: f32, // Vertical angle in degrees (5-90)

    // Floor
    show_floor: u32,
    floor_height: f32,
    floor_color1: glam::Vec3,
    floor_color2: glam::Vec3,
    floor_reflections: u32,
    floor_reflection_strength: f32,

    // Ray marching
    use_adaptive_step: u32,
    fixed_step_size: f32,
    step_multiplier: f32,
    max_distance: f32,

    // Fog
    fog_enabled: u32,
    fog_mode: u32, // 0: Linear, 1: Exponential, 2: Quadratic
    fog_density: f32,
    fog_color: glam::Vec3,

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

    // High-precision center for deep zoom (double-float emulation).
    // Each coordinate stored as a (hi, lo) pair where value = hi + lo.
    center_hi: glam::Vec2, // High part of center (x, y)
    center_lo: glam::Vec2, // Low part of center (x, y)
    high_precision: u32,   // Flag: 1 = use high precision

    // LOD debug visualization
    lod_debug_enabled: u32, // Flag: 1 = show LOD zones as colors
    lod_zone1: f32,         // Distance threshold: Ultra -> High
    lod_zone2: f32,         // Distance threshold: High -> Medium
    lod_zone3: f32,         // Distance threshold: Medium -> Low

    // Aspect ratio stored in a vec4 slot to guarantee 16-byte alignment
    aspect_ratio: glam::Vec4, // .x = width/height, others unused

    // Procedural palette parameters
    procedural_palette_type: u32, // 0=None (use static), 1=Firestrm, 2=Rainbow, etc.
    /// Custom procedural palette: brightness (a), contrast (b), frequency (c), phase (d)
    /// color(t) = a + b * cos(2π * (c * t + d))
    procedural_brightness: glam::Vec4, // [r, g, b, _]
    procedural_contrast: glam::Vec4, // [r, g, b, _]
    procedural_frequency: glam::Vec4, // [r, g, b, _]
    procedural_phase: glam::Vec4, // [r, g, b, _]

    // Perturbation uniforms (ENH-001 Phase A step 3 — plumbing only;
    // perturbation stays OFF, step 5 populates these and uploads a real
    // orbit). Mirrored by the WGSL `Uniforms` declaration.
    perturbation_enabled: u32, // 0 = OFF (default), 1 = use perturbation delta path
    orbit_len: u32,            // entries of ref_orbit actually populated
    ref_escaped_at: u32,       // index where the reference escaped (0 if bounded)
    delta_c_scale: glam::Vec2, // pixel → Δc mapping (per-pixel delta magnitude)
    delta_c_origin: glam::Vec2, // screen-center Δc (normally 0)
}

impl Default for Uniforms {
    fn default() -> Self {
        Self::new()
    }
}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY,
            inv_view_proj: Mat4::IDENTITY,
            camera_pos: glam::Vec3::new(0.0, 0.0, 3.0),
            center: glam::Vec2::new(0.0, 0.0),
            zoom: 1.0,
            max_iterations: 80,
            julia_c: glam::Vec2::new(-0.7, 0.27015),
            fractal_type: 0,
            render_mode: 0,
            power: 8.0,
            max_steps: 325,
            min_distance: 0.00035,
            fractal_scale: 2.0,
            fractal_fold: 1.0,
            fractal_min_radius: 0.5,
            palette: [glam::Vec4::ZERO; 8],
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
            roughness: 0.3,
            metallic: 0.15,
            albedo: glam::Vec3::new(0.8, 0.8, 0.8),
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

            show_floor: 1,
            floor_height: -2.0,
            floor_color1: glam::Vec3::new(1.0, 1.0, 1.0), // White
            floor_color2: glam::Vec3::new(0.0, 0.0, 0.0), // Black
            floor_reflections: 0,
            floor_reflection_strength: 0.7,

            use_adaptive_step: 1,
            fixed_step_size: 0.1,
            step_multiplier: 0.7,
            max_distance: 100.0,

            fog_enabled: 0,
            fog_mode: 1, // Exponential
            fog_density: 0.001,
            fog_color: glam::Vec3::new(0.2, 0.2, 0.2), // Dark grey

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

            center_hi: glam::Vec2::ZERO,
            center_lo: glam::Vec2::ZERO,
            high_precision: 0,

            lod_debug_enabled: 0,
            // QA-025: shared with `LODConfig::default()` via `DEFAULT_LOD_ZONES`
            // so the CPU-side defaults and the GPU-side uniform defaults cannot
            // drift apart.
            lod_zone1: crate::lod::DEFAULT_LOD_ZONES[0],
            lod_zone2: crate::lod::DEFAULT_LOD_ZONES[1],
            lod_zone3: crate::lod::DEFAULT_LOD_ZONES[2],

            aspect_ratio: glam::Vec4::new(16.0 / 9.0, 0.0, 0.0, 0.0), // Default aspect ratio

            // Procedural palette defaults
            procedural_palette_type: 0, // None (use static palette)
            procedural_brightness: glam::Vec4::new(0.5, 0.5, 0.5, 0.0),
            procedural_contrast: glam::Vec4::new(0.5, 0.5, 0.5, 0.0),
            procedural_frequency: glam::Vec4::new(1.0, 1.0, 1.0, 0.0),
            procedural_phase: glam::Vec4::new(0.0, 0.333, 0.667, 0.0),

            // ENH-001 Phase A step 3: perturbation OFF by default. Step 5
            // will populate these from a computed reference orbit.
            perturbation_enabled: 0,
            orbit_len: 0,
            ref_escaped_at: 0,
            delta_c_scale: glam::Vec2::ZERO,
            delta_c_origin: glam::Vec2::ZERO,
        }
    }

    pub fn update(&mut self, camera: &Camera, params: &FractalParams, time: f32) {
        let view_proj = camera.build_view_projection_matrix();
        self.view_proj = view_proj;
        self.inv_view_proj = view_proj.inverse();
        self.camera_pos = camera.position;

        self.center = glam::Vec2::new(
            params.settings.center_2d[0] as f32,
            params.settings.center_2d[1] as f32,
        );
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
        self.center_hi = glam::Vec2::new(hi_x, hi_y);
        self.center_lo = glam::Vec2::new(lo_x, lo_y);

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
        self.julia_c = params.settings.julia_c.into();

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
            self.palette[i] = glam::Vec4::new(color.x, color.y, color.z, 1.0);
        }

        // Update procedural palette
        self.procedural_palette_type = params.settings.procedural_palette.shader_index();
        self.procedural_brightness = glam::Vec4::new(
            params.settings.procedural_brightness[0],
            params.settings.procedural_brightness[1],
            params.settings.procedural_brightness[2],
            0.0,
        );
        self.procedural_contrast = glam::Vec4::new(
            params.settings.procedural_contrast[0],
            params.settings.procedural_contrast[1],
            params.settings.procedural_contrast[2],
            0.0,
        );
        self.procedural_frequency = glam::Vec4::new(
            params.settings.procedural_frequency[0],
            params.settings.procedural_frequency[1],
            params.settings.procedural_frequency[2],
            0.0,
        );
        self.procedural_phase = glam::Vec4::new(
            params.settings.procedural_phase[0],
            params.settings.procedural_phase[1],
            params.settings.procedural_phase[2],
            0.0,
        );

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
        self.albedo = params.settings.albedo;

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
        self.floor_color1 = params.settings.floor_color1;
        self.floor_color2 = params.settings.floor_color2;
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
        self.fog_color = params.settings.fog_color;

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
        self.delta_c_scale = glam::Vec2::ZERO;
        self.delta_c_origin = glam::Vec2::ZERO;
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
        self.delta_c_scale =
            glam::Vec2::new((inv_zoom_f64 * aspect_f64) as f32, inv_zoom_f64 as f32);
        self.delta_c_origin = reference_offset.into();
    }
}

// ENH-008: the old compile-time `size_of::<Uniforms>() == 896` assert is gone —
// the struct no longer carries manual padding, so its WGSL-correct size comes
// from encase (`Uniforms::min_size()`), checked at runtime by the
// `uniforms_byte_layout` test alongside the field-offset sentinels.

// Post-processing uniform structs.
//
// ENH-008: these derive `encase::ShaderType` instead of the prior `#[repr(C)]
// + bytemuck` hand-layout. encase computes WGSL-correct uniform layout
// (alignment + trailing padding) from the field types, so the explicit
// `_padding_*` fields are gone from both Rust and WGSL. vec/mat fields use glam
// types (glam's `encase` feature provides the `ShaderType` impls), because
// encase maps `[f32; N]` to a WGSL `array<f32, N>` (16-byte stride in uniform
// address space), not a `vecN<f32>`. Upload sites call `write_uniform_bytes`
// (encase `UniformBuffer::write`) instead of `bytemuck::cast_slice`.
#[derive(Copy, Clone, Debug, PartialEq, encase::ShaderType)]
pub(super) struct BloomUniforms {
    pub(super) threshold: f32,
    pub(super) intensity: f32,
    /// ENH-003: fraction of `scene_texture` the fractal pass actually wrote
    /// (sub-rect viewport during LOD motion). `[1.0, 1.0]` at full resolution
    /// — the bloom-extract shader's `scene_sample_uv` is a no-op then.
    pub(super) scene_uv_scale: glam::Vec2,
}

#[derive(Copy, Clone, Debug, encase::ShaderType)]
pub(crate) struct BlurUniforms {
    /// `(1,0)` horizontal or `(0,1)` vertical.
    pub(crate) direction: glam::Vec2,
}

impl BlurUniforms {
    /// `(1,0)` horizontal or `(0,1)` vertical. Single constructor so the
    /// WGSL-mirrored layout lives in one place; reused by the interactive
    /// renderer and the high-res capture paths (ENH-008 dedup — previously
    /// each capture path re-declared its own local `BlurUniforms`).
    pub(crate) fn new(direction: [f32; 2]) -> Self {
        Self {
            direction: direction.into(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, encase::ShaderType)]
pub(crate) struct PostProcessUniforms {
    pub(crate) brightness: f32,
    pub(crate) contrast: f32,
    pub(crate) saturation: f32,
    pub(crate) hue_shift: f32,
    pub(crate) vignette_enabled: u32,
    pub(crate) vignette_intensity: f32,
    pub(crate) vignette_radius: f32,
    pub(crate) bloom_enabled: u32,
    pub(crate) bloom_intensity: f32,
    /// ENH-003: sub-rect of scene_texture the fractal pass wrote ([1,1] = full
    /// res; composite's scene_sample_uv is a no-op then).
    pub(crate) scene_uv_scale: glam::Vec2,
}

impl BloomUniforms {
    /// Build from the user's settings + the frame's `scene_uv_scale`. The
    /// single constructor for the struct so the WGSL-mirrored layout lives in
    /// one place; called by `Renderer::update` (interactive scale) and the
    /// capture paths (full-quality `[1,1]`). (ENH-003.)
    pub(super) fn from_params(params: &FractalParams, scene_uv_scale: [f32; 2]) -> Self {
        Self {
            threshold: params.settings.bloom_threshold,
            intensity: params.settings.bloom_intensity,
            scene_uv_scale: scene_uv_scale.into(),
        }
    }
}

impl PostProcessUniforms {
    /// Build from the user's settings + the frame's `scene_uv_scale`. Single
    /// constructor (see `BloomUniforms::from_params`). (ENH-003.)
    pub(crate) fn from_params(params: &FractalParams, scene_uv_scale: [f32; 2]) -> Self {
        Self {
            brightness: params.settings.brightness,
            contrast: params.settings.contrast,
            saturation: params.settings.saturation,
            hue_shift: params.settings.hue_shift,
            vignette_enabled: if params.settings.vignette_enabled {
                1
            } else {
                0
            },
            vignette_intensity: params.settings.vignette_intensity,
            vignette_radius: params.settings.vignette_radius,
            bloom_enabled: if params.settings.bloom_enabled { 1 } else { 0 },
            bloom_intensity: params.settings.bloom_intensity,
            scene_uv_scale: scene_uv_scale.into(),
        }
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    /// ENH-008: `Uniforms` derives `encase::ShaderType` with no manual padding,
    /// so its WGSL-correct layout (768 bytes) is computed by encase. Lock a
    /// spread of field offsets (early → trailing) by serializing sentinels and
    /// reading them back at the offsets derived from WGSL host-shareable rules:
    ///   - mat4x4<f32>: 64B, align 16
    ///   - vec4<f32>:   16B, align 16
    ///   - vec3<f32>:   12B data + align 16 (a trailing scalar lands at +12, NOT +16)
    ///   - vec2<f32>:   8B,  align 8
    ///   - f32/u32:     4B,  align 4
    ///   - array<vec4<f32>, N>: stride 16, align 16
    ///
    /// A byte-pattern test is stronger than `offset_of!` here: it proves the
    /// bytes the GPU receives match the WGSL struct, not the Rust struct's
    /// in-memory layout (encase decouples the two — e.g. glam::Vec3 is 4-byte
    /// aligned in Rust but gets a 12-byte/16-align slot in the uniform buffer).
    /// Any field reorder/add/remove or a WGSL↔Rust type mismatch breaks a sentinel.
    #[test]
    fn uniforms_byte_layout() {
        let mut u = Uniforms::new();
        u.max_iterations = 0x1234_5678; // u32 @156
        u.palette[2] = glam::Vec4::new(1.5, 2.5, 3.5, 4.5); // palette @208, [2] @240
        u.albedo = glam::Vec3::new(5.5, 6.5, 7.5); // vec3 @384
        u.center_hi = glam::Vec2::new(8.5, 9.5); // @604
        u.aspect_ratio = glam::Vec4::new(10.5, 11.5, 12.5, 13.5); // @640
        u.procedural_phase = glam::Vec4::new(14.5, 15.5, 16.5, 17.5); // @720
        u.delta_c_origin = glam::Vec2::new(18.5, 19.5); // @760 (trailing field)

        let bytes = write_uniform_bytes(&u);
        assert_eq!(bytes.len(), 768);

        assert_eq!(u32_at(&bytes, 156), 0x1234_5678); // max_iterations

        // palette[2] @208 + 2*16
        assert_eq!(f32_at(&bytes, 240), 1.5);
        assert_eq!(f32_at(&bytes, 244), 2.5);
        assert_eq!(f32_at(&bytes, 248), 3.5);
        assert_eq!(f32_at(&bytes, 252), 4.5);

        // albedo @384: vec3 is 12B of data, so .xyz land at 384/388/392.
        assert_eq!(f32_at(&bytes, 384), 5.5);
        assert_eq!(f32_at(&bytes, 388), 6.5);
        assert_eq!(f32_at(&bytes, 392), 7.5);

        assert_eq!(f32_at(&bytes, 592), 8.5); // center_hi.x
        assert_eq!(f32_at(&bytes, 596), 9.5); // center_hi.y

        assert_eq!(f32_at(&bytes, 640), 10.5); // aspect_ratio.x
        assert_eq!(f32_at(&bytes, 644), 11.5); // aspect_ratio.y

        assert_eq!(f32_at(&bytes, 720), 14.5); // procedural_phase.x
        assert_eq!(f32_at(&bytes, 724), 15.5); // procedural_phase.y

        // delta_c_origin is the trailing field; its 8 bytes end exactly at 768.
        assert_eq!(f32_at(&bytes, 760), 18.5);
        assert_eq!(f32_at(&bytes, 764), 19.5);
    }

    /// ENH-003: lock the post-processing uniform layouts. `scene_uv_scale`
    /// repurposes pre-existing padding, so the struct sizes must NOT change
    /// (16B / 64B) and the new field lands at the documented offset.
    /// Read an f32 at a byte offset (little-endian) from serialized uniform
    /// bytes — used by the ENH-008 byte-pattern layout tests.
    fn f32_at(bytes: &[u8], offset: usize) -> f32 {
        f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    /// Read a u32 at a byte offset (little-endian) from serialized uniform bytes.
    fn u32_at(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    /// ENH-008: `BloomUniforms` now derives `encase::ShaderType`. Lock its GPU
    /// layout by serializing sentinel values and checking they land at the
    /// WGSL-documented offsets (threshold@0, intensity@4, scene_uv_scale@8,
    /// total 16B). A byte-pattern test is stronger than `offset_of!` here: it
    /// proves the bytes the GPU receives match the WGSL struct, not just the
    /// Rust struct's in-memory layout (which glam/encase may pad differently).
    #[test]
    fn bloom_uniform_byte_layout() {
        let bloom = BloomUniforms {
            threshold: 1.0,
            intensity: 2.0,
            scene_uv_scale: glam::Vec2::new(3.0, 4.0),
        };
        let bytes = write_uniform_bytes(&bloom);
        assert_eq!(bytes.len(), 16);
        assert_eq!(f32_at(&bytes, 0), 1.0);
        assert_eq!(f32_at(&bytes, 4), 2.0);
        assert_eq!(f32_at(&bytes, 8), 3.0);
        assert_eq!(f32_at(&bytes, 12), 4.0);
    }

    /// ENH-008: `PostProcessUniforms` derives `encase::ShaderType`; its three
    /// former `_padding*` fields are gone from Rust and WGSL. encase lays the
    /// 9 scalars out at natural alignment, so `scene_uv_scale` (vec2, 8-align)
    /// lands at @40 after `bloom_intensity`@32 (4B implicit pad at 36) and the
    /// struct is 48 bytes. Pin the bytes the GPU receives with sentinels.
    #[test]
    fn postprocess_uniform_byte_layout() {
        let p = PostProcessUniforms {
            brightness: 1.0,
            contrast: 2.0,
            saturation: 3.0,
            hue_shift: 4.0,
            vignette_enabled: 5,
            vignette_intensity: 6.0,
            vignette_radius: 7.0,
            bloom_enabled: 8,
            bloom_intensity: 9.0,
            scene_uv_scale: glam::Vec2::new(10.0, 11.0),
        };
        let bytes = write_uniform_bytes(&p);
        assert_eq!(bytes.len(), 48);
        assert_eq!(f32_at(&bytes, 0), 1.0);
        assert_eq!(f32_at(&bytes, 4), 2.0);
        assert_eq!(f32_at(&bytes, 8), 3.0);
        assert_eq!(f32_at(&bytes, 12), 4.0);
        assert_eq!(u32_at(&bytes, 16), 5);
        assert_eq!(f32_at(&bytes, 20), 6.0);
        assert_eq!(f32_at(&bytes, 24), 7.0);
        assert_eq!(u32_at(&bytes, 28), 8);
        assert_eq!(f32_at(&bytes, 32), 9.0);
        // 36..40 implicit padding (vec2 aligns to its 8-byte boundary)
        assert_eq!(f32_at(&bytes, 40), 10.0);
        assert_eq!(f32_at(&bytes, 44), 11.0);
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
