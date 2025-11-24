use crate::camera::Camera;
use crate::fractal::{FractalParams, RenderMode};
use bytemuck::{Pod, Zeroable};
use glam::Mat4;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(super) struct Uniforms {
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
    palette: [[f32; 4]; 5], // 5 colors with padding

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

    // Padding for 16-byte alignment - reduced to account for new fields and vec3 alignment fix
    _padding_end: [f32; 28], // 112 bytes (reduced by 16 for LOD fields)
    _padding_end2: [f32; 4], // 16 bytes (reduced by 16 for aspect_ratio field)
}

impl Uniforms {
    pub(super) fn new() -> Self {
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
            palette: [[0.0; 4]; 5],
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

            _padding_end: [0.0; 28],
            _padding_end2: [0.0; 4],
        }
    }

    pub(super) fn update(&mut self, camera: &Camera, params: &FractalParams, time: f32) {
        let view_proj = camera.build_view_projection_matrix();
        self.view_proj = view_proj.to_cols_array_2d();
        self.inv_view_proj = view_proj.inverse().to_cols_array_2d();
        self.camera_pos = camera.position.into();

        self.center = [params.center_2d[0] as f32, params.center_2d[1] as f32];
        self.zoom = params.zoom_2d;
        self.aspect_ratio[0] = camera.aspect;

        // High-precision center: split f64 into (hi, lo) pair
        // Auto-enable high precision when zoom > 1e6
        let use_high_precision = params.zoom_2d > 1_000_000.0;
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

        // Auto-scale iterations with zoom for 2D fractals, combined with user slider
        if params.render_mode == crate::fractal::RenderMode::TwoD {
            let zoom_bonus = (params.zoom_2d.max(1.0).log2() * 15.0) as u32;
            self.max_iterations = params.max_iterations + zoom_bonus;
        } else {
            self.max_iterations = params.max_iterations;
        }
        self.julia_c = params.julia_c;

        self.fractal_type = match params.fractal_type {
            // 2D fractals (0-11)
            crate::fractal::FractalType::Mandelbrot2D => 0,
            crate::fractal::FractalType::Julia2D => 1,
            crate::fractal::FractalType::Sierpinski2D => 2,
            crate::fractal::FractalType::BurningShip2D => 3,
            crate::fractal::FractalType::Tricorn2D => 4,
            crate::fractal::FractalType::Phoenix2D => 5,
            crate::fractal::FractalType::Celtic2D => 6,
            crate::fractal::FractalType::Newton2D => 7,
            crate::fractal::FractalType::Lyapunov2D => 8,
            crate::fractal::FractalType::Nova2D => 9,
            crate::fractal::FractalType::Magnet2D => 10,
            crate::fractal::FractalType::Collatz2D => 11,
            // 3D fractals (12-20)
            crate::fractal::FractalType::Mandelbulb3D => 12,
            crate::fractal::FractalType::MengerSponge3D => 13,
            crate::fractal::FractalType::SierpinskiPyramid3D => 14,
            crate::fractal::FractalType::JuliaSet3D => 15,
            crate::fractal::FractalType::Mandelbox3D => 16,
            crate::fractal::FractalType::TgladFormula3D => 17,
            crate::fractal::FractalType::OctahedralIFS3D => 18,
            crate::fractal::FractalType::IcosahedralIFS3D => 19,
            crate::fractal::FractalType::ApollonianGasket3D => 20,
            crate::fractal::FractalType::Kleinian3D => 21,
            crate::fractal::FractalType::HybridMandelbulbJulia3D => 22,
            crate::fractal::FractalType::QuaternionCubic3D => 23,
        };

        self.render_mode = match params.render_mode {
            RenderMode::TwoD => 0,
            RenderMode::ThreeD => 1,
        };

        self.power = params.power;
        self.max_steps = params.max_steps;
        self.min_distance = params.min_distance;
        // Pass scale parameters directly - each fractal handles them appropriately
        self.fractal_scale = params.fractal_scale;
        self.fractal_fold = params.fractal_fold;
        self.fractal_min_radius = params.fractal_min_radius;

        // Update palette
        for (i, color) in params.palette.colors.iter().enumerate() {
            self.palette[i] = [color.x, color.y, color.z, 1.0];
        }

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
        self.dof_samples = params.dof_samples;
        self.time = time;
        self.light_intensity = params.light_intensity;
        self.ambient_light = params.ambient_light;
        self.ao_intensity = params.ao_intensity;
        self.ao_step_size = params.ao_step_size;
        self.shadow_softness = params.shadow_softness;
        self.shadow_max_distance = params.shadow_max_distance;
        self.shadow_samples = params.shadow_samples;
        self.shadow_step_factor = params.shadow_step_factor;

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
}

// Compile-time assertion to ensure struct size matches WGSL expectations
const _: () = assert!(
    std::mem::size_of::<Uniforms>() == 832,
    "Uniforms struct must be exactly 832 bytes"
);

// Post-processing uniform structs
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
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
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
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
