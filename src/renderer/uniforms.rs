use crate::camera::Camera;
use crate::fractal::{FractalParams, RenderMode};
use bytemuck::{Pod, Zeroable};
use glam::Mat4;

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
            lod_zone1: 10.0, // Default LOD thresholds
            lod_zone2: 25.0,
            lod_zone3: 50.0,

            aspect_ratio: [16.0 / 9.0, 0.0, 0.0, 0.0], // Default aspect ratio

            // Procedural palette defaults
            procedural_palette_type: 0, // None (use static palette)
            _padding_proc_pal: [0; 3],
            procedural_brightness: [0.5, 0.5, 0.5, 0.0],
            procedural_contrast: [0.5, 0.5, 0.5, 0.0],
            procedural_frequency: [1.0, 1.0, 1.0, 0.0],
            procedural_phase: [0.0, 0.333, 0.667, 0.0],

            _padding_end: [0.0; 8],
        }
    }

    pub fn update(&mut self, camera: &Camera, params: &FractalParams, time: f32) {
        let view_proj = camera.build_view_projection_matrix();
        self.view_proj = view_proj.to_cols_array_2d();
        self.inv_view_proj = view_proj.inverse().to_cols_array_2d();
        self.camera_pos = camera.position.into();

        self.center = [params.center_2d[0] as f32, params.center_2d[1] as f32];
        // ARC-001: zoom_2d is f64 CPU-side; the GPU uniform stays f32 (casting here at
        // the boundary). The f32 GPU zoom is the remaining precision limiter; the
        // double-float center (hi/lo) is what actually extends the on-GPU ceiling.
        self.zoom = params.zoom_2d as f32;
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
        let use_high_precision = params.zoom_2d > HP_ZOOM_THRESHOLD;
        self.high_precision = if use_high_precision { 1 } else { 0 };

        // Split center coordinates into double-float pairs
        // hi = value as f32, lo = (value - hi as f64) as f32
        let center_x = params.center_2d[0];
        let center_y = params.center_2d[1];
        self.center_hi = [center_x as f32, center_y as f32];
        self.center_lo = [
            (center_x - self.center_hi[0] as f64) as f32,
            (center_y - self.center_hi[1] as f64) as f32,
        ];

        // Auto-scale iterations with zoom for 2D fractals, combined with user
        // slider. ARC-007: also fold in the LOD `iteration_scale`, applied
        // AFTER the zoom bonus (so deep zoom does not re-lose detail it just
        // paid for) and floored at ≥16 to avoid blank renders at low quality.
        // 2D-only: 3D ray-march cost is dominated by `max_steps`, not iters.
        let q = params.effective_quality();
        if params.render_mode == crate::fractal::RenderMode::TwoD {
            let zoom_bonus = (params.zoom_2d.max(1.0).log2() * 15.0) as u32;
            let scaled = ((params.max_iterations + zoom_bonus) as f32 * q.iteration_scale) as u32;
            self.max_iterations = scaled.max(16);
        } else {
            self.max_iterations = params.max_iterations;
        }
        self.julia_c = params.julia_c;

        self.fractal_type = match params.fractal_type {
            // 2D fractals (0-12)
            crate::fractal::FractalType::Mandelbrot2D => 0,
            crate::fractal::FractalType::Julia2D => 1,
            crate::fractal::FractalType::Sierpinski2D => 2,
            crate::fractal::FractalType::SierpinskiTriangle2D => 3,
            crate::fractal::FractalType::BurningShip2D => 4,
            crate::fractal::FractalType::Tricorn2D => 5,
            crate::fractal::FractalType::Phoenix2D => 6,
            crate::fractal::FractalType::Celtic2D => 7,
            crate::fractal::FractalType::Newton2D => 8,
            crate::fractal::FractalType::Lyapunov2D => 9,
            crate::fractal::FractalType::Nova2D => 10,
            crate::fractal::FractalType::Magnet2D => 11,
            crate::fractal::FractalType::Collatz2D => 12,
            // 2D Density fractals
            crate::fractal::FractalType::Buddhabrot2D => 25, // Rendered via compute shader, not main shader
            // 3D fractals (13-25)
            crate::fractal::FractalType::Mandelbulb3D => 13,
            crate::fractal::FractalType::MengerSponge3D => 14,
            crate::fractal::FractalType::SierpinskiPyramid3D => 15,
            crate::fractal::FractalType::JuliaSet3D => 16,
            crate::fractal::FractalType::Mandelbox3D => 17,
            crate::fractal::FractalType::OctahedralIFS3D => 18,
            crate::fractal::FractalType::IcosahedralIFS3D => 19,
            crate::fractal::FractalType::ApollonianGasket3D => 20,
            crate::fractal::FractalType::Kleinian3D => 21,
            crate::fractal::FractalType::HybridMandelbulbJulia3D => 22,
            crate::fractal::FractalType::QuaternionCubic3D => 23,
            crate::fractal::FractalType::SierpinskiGasket3D => 24,
            // 2D Strange Attractors (26-32)
            crate::fractal::FractalType::Hopalong2D => 26,
            crate::fractal::FractalType::Martin2D => 27,
            crate::fractal::FractalType::Gingerbreadman2D => 28,
            crate::fractal::FractalType::Chip2D => 29,
            crate::fractal::FractalType::Quadruptwo2D => 30,
            crate::fractal::FractalType::Threeply2D => 31,
            // 3D Strange Attractors (35-37)
            crate::fractal::FractalType::Pickover3D => 35,
            crate::fractal::FractalType::Lorenz3D => 36,
            crate::fractal::FractalType::Rossler3D => 37,
        };

        self.render_mode = match params.render_mode {
            RenderMode::TwoD => 0,
            RenderMode::ThreeD => 1,
        };

        self.power = params.power;
        // ARC-008: LOD no longer mutates FractalParams. The user's slider values
        // stay authored; we merge with the LOD-active `QualityLevel` here at
        // uniform-build time. Take the *cheaper* direction on every cost axis:
        //   - smaller sample/step counts → fewer GPU iterations
        //   - larger `min_distance`       → coarser surface precision
        //   - larger `shadow_step_factor` → less precise shadow rays
        //   - larger `ao_step_size`       → coarser AO sampling
        // This is the single place LOD quality influences the GPU.
        // (`q` was computed above, before the 2D iteration scaling.)
        self.max_steps = params.max_steps.min(q.max_steps);
        self.min_distance = params.min_distance.max(q.min_distance);
        // Pass scale parameters directly - each fractal handles them appropriately
        self.fractal_scale = params.fractal_scale;
        self.fractal_fold = params.fractal_fold;
        self.fractal_min_radius = params.fractal_min_radius;

        // Update palette
        for (i, color) in params.palette.colors.iter().enumerate() {
            self.palette[i] = [color.x, color.y, color.z, 1.0];
        }

        // Update procedural palette
        self.procedural_palette_type = params.procedural_palette.shader_index();
        self.procedural_brightness = [
            params.procedural_brightness[0],
            params.procedural_brightness[1],
            params.procedural_brightness[2],
            0.0,
        ];
        self.procedural_contrast = [
            params.procedural_contrast[0],
            params.procedural_contrast[1],
            params.procedural_contrast[2],
            0.0,
        ];
        self.procedural_frequency = [
            params.procedural_frequency[0],
            params.procedural_frequency[1],
            params.procedural_frequency[2],
            0.0,
        ];
        self.procedural_phase = [
            params.procedural_phase[0],
            params.procedural_phase[1],
            params.procedural_phase[2],
            0.0,
        ];

        self.ambient_occlusion = if params.ambient_occlusion { 1 } else { 0 };
        // shadow_mode: 0=off,1=hard,2=soft; pass through for shader
        self.soft_shadows = params.shadow_mode;
        self.depth_of_field = if params.depth_of_field { 1 } else { 0 };
        self.shading_model = match params.shading_model {
            crate::fractal::ShadingModel::BlinnPhong => 0,
            crate::fractal::ShadingModel::PBR => 1,
        };

        self.color_mode = match params.color_mode {
            crate::fractal::ColorMode::Palette => 0,
            crate::fractal::ColorMode::RaySteps => 1,
            crate::fractal::ColorMode::Normals => 2,
            crate::fractal::ColorMode::OrbitTrapXYZ => 3,
            crate::fractal::ColorMode::OrbitTrapRadial => 4,
            crate::fractal::ColorMode::WorldPosition => 5,
            crate::fractal::ColorMode::LocalPosition => 6,
            crate::fractal::ColorMode::AmbientOcclusion => 7,
            crate::fractal::ColorMode::PerChannel => 8,
            crate::fractal::ColorMode::DistanceField => 9,
            crate::fractal::ColorMode::Depth => 10,
            crate::fractal::ColorMode::Convergence => 11,
            crate::fractal::ColorMode::LightingOnly => 12,
            crate::fractal::ColorMode::ShadowMap => 13,
            crate::fractal::ColorMode::CameraDistanceLOD => 14,
            crate::fractal::ColorMode::DistanceGrayscale => 15,
        };

        self.orbit_trap_scale = params.orbit_trap_scale;
        self.palette_offset = params.palette_offset;

        // Convert channel sources to shader-compatible values
        self.channel_r = match params.channel_r {
            crate::fractal::ChannelSource::Iterations => 0,
            crate::fractal::ChannelSource::Distance => 1,
            crate::fractal::ChannelSource::PositionX => 2,
            crate::fractal::ChannelSource::PositionY => 3,
            crate::fractal::ChannelSource::PositionZ => 4,
            crate::fractal::ChannelSource::Normal => 5,
            crate::fractal::ChannelSource::AO => 6,
            crate::fractal::ChannelSource::Constant => 7,
        };
        self.channel_g = match params.channel_g {
            crate::fractal::ChannelSource::Iterations => 0,
            crate::fractal::ChannelSource::Distance => 1,
            crate::fractal::ChannelSource::PositionX => 2,
            crate::fractal::ChannelSource::PositionY => 3,
            crate::fractal::ChannelSource::PositionZ => 4,
            crate::fractal::ChannelSource::Normal => 5,
            crate::fractal::ChannelSource::AO => 6,
            crate::fractal::ChannelSource::Constant => 7,
        };
        self.channel_b = match params.channel_b {
            crate::fractal::ChannelSource::Iterations => 0,
            crate::fractal::ChannelSource::Distance => 1,
            crate::fractal::ChannelSource::PositionX => 2,
            crate::fractal::ChannelSource::PositionY => 3,
            crate::fractal::ChannelSource::PositionZ => 4,
            crate::fractal::ChannelSource::Normal => 5,
            crate::fractal::ChannelSource::AO => 6,
            crate::fractal::ChannelSource::Constant => 7,
        };

        self.roughness = params.roughness;
        self.metallic = params.metallic;
        self.albedo = params.albedo.into();

        self.dof_focal_length = params.dof_focal_length;
        self.dof_aperture = params.dof_aperture;
        self.dof_samples = params.dof_samples.min(q.dof_samples);
        self.time = time;
        self.light_intensity = params.light_intensity;
        self.ambient_light = params.ambient_light;
        self.ao_intensity = params.ao_intensity;
        self.ao_step_size = params.ao_step_size.max(q.ao_step_size);
        self.shadow_softness = params.shadow_softness;
        self.shadow_max_distance = params.shadow_max_distance;
        self.shadow_samples = params.shadow_samples.min(q.shadow_samples);
        self.shadow_step_factor = params.shadow_step_factor.max(q.shadow_step_factor);

        self.light_azimuth = params.light_azimuth;
        self.light_elevation = params.light_elevation;

        self.show_floor = if params.show_floor { 1 } else { 0 };
        self.floor_height = params.floor_height;
        self.floor_color1 = params.floor_color1.into();
        self.floor_color2 = params.floor_color2.into();
        self.floor_reflections = if params.floor_reflections { 1 } else { 0 };
        self.floor_reflection_strength = params.floor_reflection_strength;

        self.use_adaptive_step = if params.use_adaptive_step { 1 } else { 0 };
        self.fixed_step_size = params.fixed_step_size;
        self.step_multiplier = params.step_multiplier;
        self.max_distance = params.max_distance;

        self.fog_enabled = if params.fog_enabled { 1 } else { 0 };
        self.fog_mode = match params.fog_mode {
            crate::fractal::FogMode::Linear => 0,
            crate::fractal::FogMode::Exponential => 1,
            crate::fractal::FogMode::Quadratic => 2,
        };
        self.fog_density = params.fog_density;
        self.fog_color = params.fog_color.into();

        // Post-processing
        self.brightness = params.brightness;
        self.contrast = params.contrast;
        self.saturation = params.saturation;
        self.hue_shift = params.hue_shift;
        self.vignette_enabled = if params.vignette_enabled { 1 } else { 0 };
        self.vignette_intensity = params.vignette_intensity;
        self.vignette_radius = params.vignette_radius;
        self.bloom_enabled = if params.bloom_enabled { 1 } else { 0 };
        self.bloom_threshold = params.bloom_threshold;
        self.bloom_intensity = params.bloom_intensity;
        self.bloom_radius = params.bloom_radius;
        self.fxaa_enabled = if params.fxaa_enabled { 1 } else { 0 };

        // LOD debug visualization
        let lod_enabled = params.lod_config.enabled && params.lod_config.debug_visualization;
        self.lod_debug_enabled = if lod_enabled { 1 } else { 0 };
        self.lod_zone1 = params.lod_config.distance_zones[0];
        self.lod_zone2 = params.lod_config.distance_zones[1];
        self.lod_zone3 = params.lod_config.distance_zones[2];
    }

    /// Creates a new Uniforms struct populated from camera and fractal parameters.
    /// This is useful for high-resolution rendering where we need immutable access to the renderer.
    #[cfg(target_arch = "wasm32")]
    pub fn from_camera_and_params(camera: &Camera, params: &FractalParams, time: f32) -> Self {
        let mut uniforms = Self::new();
        uniforms.update(camera, params, time);
        uniforms
    }
}

// Compile-time assertion to ensure struct size matches WGSL expectations
const _: () = assert!(
    std::mem::size_of::<Uniforms>() == 864,
    "Uniforms struct must be exactly 864 bytes"
);

// Post-processing uniform structs
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub(super) struct BloomUniforms {
    pub(super) threshold: f32,
    pub(super) intensity: f32,
    pub(super) _padding: [f32; 2],
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

    pub(super) _padding3: [f32; 4], // offset 48 (vec3 + alignment = 16 bytes)
                                    // Total: 64 bytes
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use std::mem::offset_of;

    /// Locks the `Uniforms` Rust struct to the WGSL `Uniforms` declaration in
    /// `src/shaders/fractal.wgsl` (lines 1-134). Any field reorder, add, or
    /// remove breaks this test, forcing the change to be deliberate and paired
    /// with a matching WGSL edit.
    ///
    /// The compile-time `size_of == 864` assert above catches total-size
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
        assert_eq!(std::mem::size_of::<Uniforms>(), 864);

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
    }
}
