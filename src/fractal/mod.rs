// Module declarations
mod palettes;
mod presets;
mod settings;
mod types;
mod ui_state;

// Re-exports
pub use palettes::*;
pub use presets::*;
pub use settings::*;
pub use types::*;
pub use ui_state::*;

use glam::Vec3;

use crate::lod::{LODConfig, LODState};

#[derive(Clone)]
pub struct FractalParams {
    pub fractal_type: FractalType,
    pub render_mode: RenderMode,
    pub shading_model: ShadingModel,
    pub color_mode: ColorMode,
    pub palette: ColorPalette,
    pub palette_index: usize,
    pub palette_offset: f32,
    pub orbit_trap_scale: f32,
    pub channel_r: ChannelSource,
    pub channel_g: ChannelSource,
    pub channel_b: ChannelSource,

    // 2D specific
    pub center_2d: [f64; 2],
    pub zoom_2d: f32,
    pub julia_c: [f32; 2],
    pub max_iterations: u32,

    // 3D specific
    pub power: f32,
    pub max_steps: u32,
    pub min_distance: f32,
    pub ambient_occlusion: bool,
    pub ao_intensity: f32,
    pub ao_step_size: f32,
    pub shadow_mode: u32, // 0=off, 1=hard, 2=soft
    pub shadow_softness: f32,
    pub shadow_max_distance: f32,
    pub shadow_samples: u32,
    pub shadow_step_factor: f32,
    pub depth_of_field: bool,
    pub dof_focal_length: f32,
    pub dof_aperture: f32,
    pub dof_samples: u32,

    // 3D fractal parameters
    pub fractal_scale: f32,
    pub fractal_fold: f32,
    pub fractal_min_radius: f32,

    // Material properties
    pub roughness: f32,
    pub metallic: f32,
    pub albedo: Vec3,

    // Lighting
    pub light_intensity: f32,
    pub ambient_light: f32,
    pub light_azimuth: f32,
    pub light_elevation: f32,

    // Floor
    pub show_floor: bool,
    pub floor_height: f32,
    pub floor_color1: Vec3,
    pub floor_color2: Vec3,
    pub floor_reflections: bool,
    pub floor_reflection_strength: f32,

    // Fog
    pub fog_enabled: bool,
    pub fog_mode: FogMode,
    pub fog_density: f32,
    pub fog_color: Vec3,

    // Ray marching
    pub use_adaptive_step: bool,
    pub fixed_step_size: f32,
    pub step_multiplier: f32,
    pub max_distance: f32,

    // Camera (3D mode)
    pub camera_speed: f32,
    pub camera_fov: f32,
    pub auto_orbit: bool,
    pub orbit_speed: f32,

    // Post-processing
    // Color grading
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub hue_shift: f32,

    // Vignette
    pub vignette_enabled: bool,
    pub vignette_intensity: f32,
    pub vignette_radius: f32,

    // Bloom
    pub bloom_enabled: bool,
    pub bloom_threshold: f32,
    pub bloom_intensity: f32,
    pub bloom_radius: f32,

    // Anti-aliasing
    pub fxaa_enabled: bool,

    // LOD (Level of Detail) System
    pub lod_config: LODConfig,
    pub lod_state: LODState,

    // Strange Attractor Accumulation Mode
    /// Enable compute shader accumulation for strange attractors
    pub attractor_accumulation_enabled: bool,
    /// Number of orbit iterations per frame (higher = more detail but slower)
    pub attractor_iterations_per_frame: u32,
    /// Total accumulated iterations (display only)
    pub attractor_total_iterations: u64,
    /// Log scale factor for density display
    pub attractor_log_scale: f32,
    /// Flag to clear accumulation on next frame
    pub attractor_pending_clear: bool,
    /// Last view center for detecting pan (triggers auto-clear)
    pub attractor_last_center: [f64; 2],
    /// Last zoom level for detecting zoom (triggers auto-clear)
    pub attractor_last_zoom: f32,
    /// Last julia_c parameters (triggers auto-clear on change)
    pub attractor_last_julia_c: [f32; 2],
}

impl Default for FractalParams {
    fn default() -> Self {
        Self {
            fractal_type: FractalType::Mandelbrot2D,
            render_mode: RenderMode::TwoD,
            shading_model: ShadingModel::PBR,
            color_mode: ColorMode::Palette,
            palette: ColorPalette::FIRE,
            palette_index: 0,
            palette_offset: 0.0,
            orbit_trap_scale: 1.0,
            channel_r: ChannelSource::Iterations,
            channel_g: ChannelSource::Distance,
            channel_b: ChannelSource::PositionZ,

            center_2d: [0.0f64, 0.0f64],
            zoom_2d: 1.0,
            julia_c: [-0.7, 0.27015],
            max_iterations: 80,

            power: 8.0,
            max_steps: 200,
            min_distance: 0.00035,
            ambient_occlusion: true,
            ao_intensity: 1.0,
            ao_step_size: 0.12,
            shadow_mode: 2, // soft
            shadow_softness: 8.0,
            shadow_max_distance: 5.0,
            shadow_samples: 128,
            shadow_step_factor: 0.6,
            depth_of_field: false,
            dof_focal_length: 6.0,
            dof_aperture: 0.01,
            dof_samples: 2,

            fractal_scale: 2.0,
            fractal_fold: 1.0,
            fractal_min_radius: 0.5,

            roughness: 0.4,
            metallic: 0.20,
            albedo: Vec3::new(0.8, 0.8, 0.8),

            light_intensity: 3.0,
            ambient_light: 0.15,
            light_azimuth: 45.0,
            light_elevation: 35.0,

            show_floor: true,
            floor_height: -2.0,
            floor_color1: Vec3::new(1.0, 1.0, 1.0), // White
            floor_color2: Vec3::new(0.0, 0.0, 0.0), // Black
            floor_reflections: false,
            floor_reflection_strength: 0.5,

            fog_enabled: true,
            fog_mode: FogMode::Quadratic,
            fog_density: 0.005,
            fog_color: Vec3::new(0.0, 0.0, 0.0), // Black

            use_adaptive_step: true,
            fixed_step_size: 0.1,
            step_multiplier: 1.0,
            max_distance: 100.0,

            camera_speed: 2.0,
            camera_fov: 45.0,
            auto_orbit: false,
            orbit_speed: 0.2,

            // Post-processing defaults
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            hue_shift: 0.0,

            vignette_enabled: false,
            vignette_intensity: 0.5,
            vignette_radius: 0.8,

            bloom_enabled: false,
            bloom_threshold: 0.75,
            bloom_intensity: 0.1,
            bloom_radius: 0.005,

            fxaa_enabled: false,

            // LOD system (disabled by default)
            lod_config: LODConfig::default(),
            lod_state: LODState::default(),

            // Strange attractor accumulation (disabled by default)
            attractor_accumulation_enabled: false,
            attractor_iterations_per_frame: 100_000,
            attractor_total_iterations: 0,
            attractor_log_scale: 4.0,
            attractor_pending_clear: false,
            attractor_last_center: [0.0, 0.0],
            attractor_last_zoom: 1.0,
            attractor_last_julia_c: [-0.7, 0.27015],
        }
    }
}

impl FractalParams {
    pub fn to_settings(&self) -> Settings {
        Settings {
            fractal_type: self.fractal_type,
            shading_model: self.shading_model,
            color_mode: self.color_mode,
            palette_index: self.palette_index,
            orbit_trap_scale: self.orbit_trap_scale,
            channel_r: self.channel_r,
            channel_g: self.channel_g,
            channel_b: self.channel_b,
            ui_state: UIState::default(), // Will be overridden by App if UI state exists
            auto_open_captures: false,    // Will be overridden by App with UI state
            center_2d: self.center_2d,
            zoom_2d: self.zoom_2d,
            julia_c: self.julia_c,
            max_iterations: self.max_iterations,
            power: self.power,
            max_steps: self.max_steps,
            min_distance: self.min_distance,
            ambient_occlusion: self.ambient_occlusion,
            ao_intensity: self.ao_intensity,
            ao_step_size: self.ao_step_size,
            shadow_mode: self.shadow_mode,
            shadow_softness: self.shadow_softness,
            shadow_max_distance: self.shadow_max_distance,
            shadow_samples: self.shadow_samples,
            shadow_step_factor: self.shadow_step_factor,
            depth_of_field: self.depth_of_field,
            dof_focal_length: self.dof_focal_length,
            dof_aperture: self.dof_aperture,
            dof_samples: self.dof_samples,
            fractal_scale: self.fractal_scale,
            fractal_fold: self.fractal_fold,
            fractal_min_radius: self.fractal_min_radius,
            roughness: self.roughness,
            metallic: self.metallic,
            albedo: self.albedo.to_array(),
            light_intensity: self.light_intensity,
            ambient_light: self.ambient_light,
            light_azimuth: self.light_azimuth,
            light_elevation: self.light_elevation,
            show_floor: self.show_floor,
            floor_height: self.floor_height,
            floor_color1: self.floor_color1.to_array(),
            floor_color2: self.floor_color2.to_array(),
            floor_reflections: self.floor_reflections,
            floor_reflection_strength: self.floor_reflection_strength,
            fog_enabled: self.fog_enabled,
            fog_mode: self.fog_mode,
            fog_density: self.fog_density,
            fog_color: self.fog_color.to_array(),
            use_adaptive_step: self.use_adaptive_step,
            fixed_step_size: self.fixed_step_size,
            step_multiplier: self.step_multiplier,
            max_distance: self.max_distance,
            camera_position: [0.0, 0.0, 3.0], // Will be overridden by App
            camera_target: [0.0, 0.0, 0.0],   // Will be overridden by App
            camera_speed: self.camera_speed,
            camera_fov: self.camera_fov,
            auto_orbit: self.auto_orbit,
            orbit_speed: self.orbit_speed,
            brightness: self.brightness,
            contrast: self.contrast,
            saturation: self.saturation,
            hue_shift: self.hue_shift,
            vignette_enabled: self.vignette_enabled,
            vignette_intensity: self.vignette_intensity,
            vignette_radius: self.vignette_radius,
            bloom_enabled: self.bloom_enabled,
            bloom_threshold: self.bloom_threshold,
            bloom_intensity: self.bloom_intensity,
            bloom_radius: self.bloom_radius,
            fxaa_enabled: self.fxaa_enabled,
            lod_config: self.lod_config.clone(),
            custom_width: default_custom_width(),
            custom_height: default_custom_height(),
            attractor_accumulation_enabled: self.attractor_accumulation_enabled,
            attractor_iterations_per_frame: self.attractor_iterations_per_frame,
            attractor_log_scale: self.attractor_log_scale,
        }
    }

    pub fn from_settings(settings: Settings) -> Self {
        let palette_index = settings.palette_index.min(ColorPalette::ALL.len() - 1);
        let palette = ColorPalette::ALL[palette_index];

        let render_mode = match settings.fractal_type {
            FractalType::Mandelbrot2D
            | FractalType::Julia2D
            | FractalType::Sierpinski2D
            | FractalType::SierpinskiTriangle2D
            | FractalType::BurningShip2D
            | FractalType::Tricorn2D
            | FractalType::Phoenix2D
            | FractalType::Celtic2D
            | FractalType::Newton2D
            | FractalType::Lyapunov2D
            | FractalType::Nova2D
            | FractalType::Magnet2D
            | FractalType::Collatz2D
            | FractalType::Hopalong2D
            | FractalType::Martin2D
            | FractalType::Gingerbreadman2D
            | FractalType::Chip2D
            | FractalType::Quadruptwo2D
            | FractalType::Threeply2D => RenderMode::TwoD,
            FractalType::Mandelbulb3D
            | FractalType::MengerSponge3D
            | FractalType::SierpinskiPyramid3D
            | FractalType::JuliaSet3D
            | FractalType::Mandelbox3D
            | FractalType::OctahedralIFS3D
            | FractalType::IcosahedralIFS3D
            | FractalType::ApollonianGasket3D
            | FractalType::Kleinian3D
            | FractalType::HybridMandelbulbJulia3D
            | FractalType::QuaternionCubic3D
            | FractalType::SierpinskiGasket3D
            | FractalType::Pickover3D
            | FractalType::Lorenz3D
            | FractalType::Rossler3D => RenderMode::ThreeD,
        };

        Self {
            fractal_type: settings.fractal_type,
            render_mode,
            shading_model: settings.shading_model,
            color_mode: settings.color_mode,
            palette,
            palette_index,
            palette_offset: 0.0,
            orbit_trap_scale: settings.orbit_trap_scale,
            channel_r: settings.channel_r,
            channel_g: settings.channel_g,
            channel_b: settings.channel_b,
            center_2d: settings.center_2d,
            zoom_2d: settings.zoom_2d,
            julia_c: settings.julia_c,
            max_iterations: settings.max_iterations,
            power: settings.power,
            max_steps: settings.max_steps,
            min_distance: settings.min_distance,
            ambient_occlusion: settings.ambient_occlusion,
            ao_intensity: settings.ao_intensity,
            ao_step_size: settings.ao_step_size,
            shadow_mode: settings.shadow_mode,
            shadow_softness: settings.shadow_softness,
            shadow_max_distance: settings.shadow_max_distance,
            shadow_samples: settings.shadow_samples,
            shadow_step_factor: settings.shadow_step_factor,
            depth_of_field: settings.depth_of_field,
            dof_focal_length: settings.dof_focal_length,
            dof_aperture: settings.dof_aperture,
            dof_samples: settings.dof_samples,
            fractal_scale: settings.fractal_scale,
            fractal_fold: settings.fractal_fold,
            fractal_min_radius: settings.fractal_min_radius,
            roughness: settings.roughness,
            metallic: settings.metallic,
            albedo: Vec3::from_array(settings.albedo),
            light_intensity: settings.light_intensity,
            ambient_light: settings.ambient_light,
            light_azimuth: settings.light_azimuth,
            light_elevation: settings.light_elevation,
            show_floor: settings.show_floor,
            floor_height: settings.floor_height,
            floor_color1: Vec3::from_array(settings.floor_color1),
            floor_color2: Vec3::from_array(settings.floor_color2),
            floor_reflections: settings.floor_reflections,
            floor_reflection_strength: settings.floor_reflection_strength,
            fog_enabled: settings.fog_enabled,
            fog_mode: settings.fog_mode,
            fog_density: settings.fog_density,
            fog_color: Vec3::from_array(settings.fog_color),
            use_adaptive_step: settings.use_adaptive_step,
            fixed_step_size: settings.fixed_step_size,
            step_multiplier: settings.step_multiplier,
            max_distance: settings.max_distance,
            camera_speed: settings.camera_speed,
            camera_fov: settings.camera_fov,
            auto_orbit: settings.auto_orbit,
            orbit_speed: settings.orbit_speed,
            brightness: settings.brightness,
            contrast: settings.contrast,
            saturation: settings.saturation,
            hue_shift: settings.hue_shift,
            vignette_enabled: settings.vignette_enabled,
            vignette_intensity: settings.vignette_intensity,
            vignette_radius: settings.vignette_radius,
            bloom_enabled: settings.bloom_enabled,
            bloom_threshold: settings.bloom_threshold,
            bloom_intensity: settings.bloom_intensity,
            bloom_radius: settings.bloom_radius,
            fxaa_enabled: settings.fxaa_enabled,
            lod_config: settings.lod_config,
            lod_state: LODState::default(),
            attractor_accumulation_enabled: settings.attractor_accumulation_enabled,
            attractor_iterations_per_frame: settings.attractor_iterations_per_frame,
            attractor_total_iterations: 0, // Always reset on load
            attractor_log_scale: settings.attractor_log_scale,
            attractor_pending_clear: false,
            attractor_last_center: settings.center_2d,
            attractor_last_zoom: settings.zoom_2d,
            attractor_last_julia_c: settings.julia_c,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let config_path = config_dir.config_dir();
            std::fs::create_dir_all(config_path)?;

            let settings_file = config_path.join("settings.yaml");
            let settings = self.to_settings();
            let yaml = serde_yaml::to_string(&settings)?;
            std::fs::write(settings_file, yaml)?;

            println!("Settings saved");
            Ok(())
        } else {
            Err("Could not determine config directory".into())
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Settings persistence not yet implemented for web
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_from_file() -> Option<Self> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let settings_file = config_dir.config_dir().join("settings.yaml");

            if let Ok(yaml) = std::fs::read_to_string(settings_file) {
                if let Ok(settings) = serde_yaml::from_str::<Settings>(&yaml) {
                    println!("Settings loaded");
                    return Some(Self::from_settings(settings));
                }
            }
        }
        None
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load_from_file() -> Option<Self> {
        // Settings persistence not yet implemented for web
        None
    }

    pub fn switch_fractal(&mut self, fractal_type: FractalType) {
        self.fractal_type = fractal_type;
        self.render_mode = match fractal_type {
            FractalType::Mandelbrot2D
            | FractalType::Julia2D
            | FractalType::Sierpinski2D
            | FractalType::SierpinskiTriangle2D
            | FractalType::BurningShip2D
            | FractalType::Tricorn2D
            | FractalType::Phoenix2D
            | FractalType::Celtic2D
            | FractalType::Newton2D
            | FractalType::Lyapunov2D
            | FractalType::Nova2D
            | FractalType::Magnet2D
            | FractalType::Collatz2D
            | FractalType::Hopalong2D
            | FractalType::Martin2D
            | FractalType::Gingerbreadman2D
            | FractalType::Chip2D
            | FractalType::Quadruptwo2D
            | FractalType::Threeply2D => RenderMode::TwoD,
            FractalType::Mandelbulb3D
            | FractalType::MengerSponge3D
            | FractalType::SierpinskiPyramid3D
            | FractalType::JuliaSet3D
            | FractalType::Mandelbox3D
            | FractalType::OctahedralIFS3D
            | FractalType::IcosahedralIFS3D
            | FractalType::ApollonianGasket3D
            | FractalType::Kleinian3D
            | FractalType::HybridMandelbulbJulia3D
            | FractalType::QuaternionCubic3D
            | FractalType::SierpinskiGasket3D
            | FractalType::Pickover3D
            | FractalType::Lorenz3D
            | FractalType::Rossler3D => RenderMode::ThreeD,
        };

        // Set fractal-specific defaults
        match fractal_type {
            FractalType::MengerSponge3D => {
                self.fractal_scale = 1.0; // Double the apparent size (half the scale factor)
                self.max_iterations = 7; // Default iterations for Menger Sponge
            }
            FractalType::SierpinskiPyramid3D => {
                self.max_iterations = 12; // Default iterations for Sierpinski Pyramid
            }
            FractalType::Nova2D | FractalType::Lyapunov2D => {
                self.max_iterations = 16;
            }
            FractalType::SierpinskiTriangle2D => {
                self.max_iterations = 30;
            }
            FractalType::Mandelbox3D => {
                self.fractal_scale = 1.0; // Double the apparent size
                self.fractal_fold = 1.0;
                self.fractal_min_radius = 0.5;
                self.roughness = 0.21;
                self.metallic = 0.32;
            }
            FractalType::OctahedralIFS3D | FractalType::IcosahedralIFS3D => {
                self.fractal_fold = 1.7;
            }
            FractalType::ApollonianGasket3D => {
                self.fractal_fold = 1.05;
                self.fractal_min_radius = 0.6;
            }
            FractalType::Kleinian3D => {
                self.fractal_scale = 1.5;
                self.fractal_fold = 1.0; // Results in scale -2.0 (classic Mandelbox)
                self.fractal_min_radius = 0.5;
            }
            FractalType::HybridMandelbulbJulia3D => {
                self.fractal_scale = 1.5;
                self.max_iterations = 8; // Lower for performance
            }
            FractalType::QuaternionCubic3D => {
                self.fractal_scale = 1.5;
                self.max_iterations = 8; // Lower for performance
            }
            // Strange Attractors 2D - set appropriate view bounds and iterations
            FractalType::Hopalong2D => {
                self.center_2d = [0.5, 0.5];
                self.zoom_2d = 0.3;
                self.max_iterations = 1000;
                // julia_c used for parameters: x=a, y=b (c defaults to 0)
                self.julia_c = [0.4, 1.0];
            }
            FractalType::Martin2D => {
                self.center_2d = [0.0, 0.0];
                self.zoom_2d = 0.05;
                self.max_iterations = 1000;
                // julia_c.x = a
                self.julia_c = [std::f32::consts::PI, 0.0];
            }
            FractalType::Gingerbreadman2D => {
                self.center_2d = [2.0, 2.0];
                self.zoom_2d = 0.15;
                self.max_iterations = 1000;
            }
            FractalType::Chip2D => {
                self.center_2d = [0.0, 0.0];
                self.zoom_2d = 0.002;
                self.max_iterations = 1000;
                self.julia_c = [-15.0, -19.0];
                self.power = 1.0;
            }
            FractalType::Quadruptwo2D => {
                self.center_2d = [15.0, 17.0];
                self.zoom_2d = 0.01;
                self.max_iterations = 1000;
                self.julia_c = [34.0, 1.0];
                self.power = 5.0;
            }
            FractalType::Threeply2D => {
                self.center_2d = [0.0, 0.0];
                self.zoom_2d = 1.0;
                self.max_iterations = 1000;
                self.julia_c = [-55.0, -1.0];
                self.power = -42.0;
            }
            // 3D Strange Attractors
            FractalType::Pickover3D => {
                self.fractal_scale = 0.3;
                self.max_iterations = 10000;
                // a, b, c, d via julia_c.x, julia_c.y, power, fractal_fold
                self.julia_c = [2.24, 0.43];
                self.power = -0.65;
                self.fractal_fold = -2.43;
            }
            FractalType::Lorenz3D => {
                self.fractal_scale = 0.05;
                self.max_iterations = 10000;
                // sigma, rho, beta via julia_c.x, julia_c.y, power
                self.julia_c = [10.0, 28.0];
                self.power = 2.666667;
            }
            FractalType::Rossler3D => {
                self.fractal_scale = 0.1;
                self.max_iterations = 10000;
                // a, b, c via julia_c.x, julia_c.y, power
                self.julia_c = [0.2, 0.2];
                self.power = 5.7;
            }
            _ => {}
        }
    }

    pub fn next_palette(&mut self) {
        self.palette_index = (self.palette_index + 1) % ColorPalette::ALL.len();
        self.palette = ColorPalette::ALL[self.palette_index];
    }

    pub fn prev_palette(&mut self) {
        if self.palette_index == 0 {
            self.palette_index = ColorPalette::ALL.len() - 1;
        } else {
            self.palette_index -= 1;
        }
        self.palette = ColorPalette::ALL[self.palette_index];
    }

    /// Randomize fractal parameters for creative exploration
    pub fn randomize(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();

        // Randomly select a fractal type
        let fractal_types = [
            FractalType::Mandelbrot2D,
            FractalType::Julia2D,
            FractalType::BurningShip2D,
            FractalType::Tricorn2D,
            FractalType::Phoenix2D,
            FractalType::Celtic2D,
            FractalType::Newton2D,
            FractalType::Lyapunov2D,
            FractalType::Nova2D,
            FractalType::Magnet2D,
            FractalType::Collatz2D,
            FractalType::Mandelbulb3D,
            FractalType::MengerSponge3D,
            FractalType::JuliaSet3D,
            FractalType::Mandelbox3D,
            FractalType::OctahedralIFS3D,
            FractalType::IcosahedralIFS3D,
            FractalType::ApollonianGasket3D,
        ];
        let new_type = fractal_types[rng.random_range(0..fractal_types.len())];
        self.switch_fractal(new_type);

        // Randomize color palette
        self.palette_index = rng.random_range(0..ColorPalette::ALL.len());
        self.palette = ColorPalette::ALL[self.palette_index];

        // Randomize color mode
        let color_modes = [
            ColorMode::Palette,
            ColorMode::RaySteps,
            ColorMode::Normals,
            ColorMode::OrbitTrapXYZ,
            ColorMode::OrbitTrapRadial,
        ];
        self.color_mode = color_modes[rng.random_range(0..color_modes.len())];

        match self.render_mode {
            RenderMode::TwoD => {
                // Randomize 2D parameters
                self.julia_c = [rng.random_range(-2.0..2.0), rng.random_range(-2.0..2.0)];
                self.max_iterations = rng.random_range(64..512);
            }
            RenderMode::ThreeD => {
                // Randomize 3D parameters
                self.power = rng.random_range(4.0..12.0);
                self.max_steps = rng.random_range(100..350);
                self.fractal_scale = rng.random_range(0.8..3.0);

                if matches!(self.fractal_type, FractalType::Mandelbox3D) {
                    self.fractal_fold = rng.random_range(0.5..2.5);
                    self.fractal_min_radius = rng.random_range(0.2..1.5);
                }

                // Randomize lighting
                self.light_intensity = rng.random_range(1.5..6.0);
                self.ambient_light = rng.random_range(0.05..0.4);

                // Randomize effects
                self.ambient_occlusion = rng.random_bool(0.7); // 70% chance
                if self.ambient_occlusion {
                    self.ao_intensity = rng.random_range(1.0..6.0);
                }

                // 0=off, 1=hard, 2=soft
                self.shadow_mode = if rng.random_bool(0.6) {
                    if rng.random_bool(0.7) {
                        2
                    } else {
                        1
                    } // 70% soft, 30% hard
                } else {
                    0
                };
                if self.shadow_mode == 2 {
                    self.shadow_softness = rng.random_range(4.0..20.0);
                }

                self.fog_enabled = rng.random_bool(0.5); // 50% chance
                if self.fog_enabled {
                    self.fog_density = rng.random_range(0.001..0.05);
                }

                self.show_floor = rng.random_bool(0.5); // 50% chance

                // Randomize material for PBR
                if self.shading_model == ShadingModel::PBR {
                    self.roughness = rng.random_range(0.1..0.9);
                    self.metallic = rng.random_range(0.0..0.7);
                }
            }
        }
    }

    /// Update LOD system state and apply quality adjustments
    pub fn update_lod(&mut self, camera_pos: Vec3, camera_forward: Vec3, delta_time: f32) {
        if !self.lod_config.enabled {
            // LOD disabled, ensure we're at max quality
            if self.lod_state.current_level != 0 {
                self.lod_state.current_level = 0;
                self.lod_state.target_level = 0;
                self.lod_state.transition_progress = 1.0;
                self.lod_state.active_quality = self.lod_config.quality_presets[0];
            }
            return;
        }

        // Update FPS tracking
        self.lod_state.update_fps(delta_time);

        // Update motion tracking
        self.lod_state.update_motion(
            camera_pos,
            camera_forward,
            delta_time,
            self.lod_config.motion_threshold,
            self.lod_config.motion_sensitivity,
        );

        // Determine target LOD level based on strategy
        let target_level = self.calculate_target_lod_level(camera_pos, delta_time);

        // Update transition
        self.lod_state.set_target(target_level);
        self.lod_state
            .update_transition(delta_time, self.lod_config.transition_duration);

        // Get active quality (interpolated if transitioning)
        self.lod_state.active_quality = self.lod_state.get_active_quality(
            &self.lod_config.quality_presets,
            self.lod_config.smooth_transitions,
        );

        // Apply quality to rendering parameters
        self.apply_lod_quality();
    }

    /// Calculate target LOD level based on current strategy
    fn calculate_target_lod_level(&mut self, camera_pos: Vec3, delta_time: f32) -> usize {
        use crate::lod::LODStrategy;

        let target_level = match self.lod_config.strategy {
            LODStrategy::Distance => self.calculate_distance_lod(camera_pos),
            LODStrategy::Motion => self.calculate_motion_lod(),
            LODStrategy::Performance => self.calculate_performance_lod(delta_time),
            LODStrategy::Hybrid => {
                // Combine all strategies, taking the most restrictive (highest LOD level)
                let distance_lod = self.calculate_distance_lod(camera_pos);
                let motion_lod = self.calculate_motion_lod();
                let performance_lod = self.calculate_performance_lod(delta_time);
                distance_lod.max(motion_lod).max(performance_lod)
            }
        };

        // Respect minimum quality level setting
        target_level.min(3).max(self.lod_config.min_quality_level)
    }

    /// Calculate LOD level based on distance from camera to fractal center
    fn calculate_distance_lod(&self, camera_pos: Vec3) -> usize {
        // For 3D fractals, calculate distance from camera to origin (fractal center)
        if self.render_mode == RenderMode::ThreeD {
            let fractal_center = Vec3::ZERO;
            let distance = camera_pos.distance(fractal_center);

            // Determine LOD level based on distance zones
            if distance < self.lod_config.distance_zones[0] {
                0 // Ultra - close up
            } else if distance < self.lod_config.distance_zones[1] {
                1 // High - medium distance
            } else if distance < self.lod_config.distance_zones[2] {
                2 // Medium - far
            } else {
                3 // Low - very far
            }
        } else {
            // For 2D fractals, use zoom level as distance proxy
            // Higher zoom = closer = lower LOD level number
            let zoom = self.zoom_2d;
            if zoom > 100.0 {
                0 // Ultra - zoomed in
            } else if zoom > 10.0 {
                1 // High
            } else if zoom > 1.0 {
                2 // Medium
            } else {
                3 // Low - zoomed out
            }
        }
    }

    /// Calculate LOD level based on camera motion
    fn calculate_motion_lod(&self) -> usize {
        if self.lod_state.is_moving {
            // Camera is moving, reduce quality
            if self.lod_config.aggressive_mode {
                3 // Drop to lowest quality immediately
            } else {
                2 // Drop to medium quality
            }
        } else if self.lod_state.time_since_stopped < self.lod_config.restore_delay {
            // Just stopped, but within restore delay
            2 // Keep at medium quality
        } else {
            // Stationary and past restore delay
            0 // Return to ultra quality
        }
    }

    /// Calculate LOD level based on performance (FPS) with hysteresis
    fn calculate_performance_lod(&mut self, delta_time: f32) -> usize {
        let current_fps = self.lod_state.current_fps;
        let target_fps = self.lod_config.target_fps;

        // Determine suggested level based on FPS thresholds
        // Add hysteresis: increase thresholds when improving, decrease when degrading
        let hysteresis_margin = 0.05; // 5% hysteresis band
        let current_level = self.lod_state.last_performance_level;

        let suggested_level = if current_fps >= target_fps * (1.0 + hysteresis_margin) {
            0 // Ultra - well above target
        } else if current_fps
            >= target_fps
                * (0.8
                    + if current_level <= 1 {
                        hysteresis_margin
                    } else {
                        -hysteresis_margin
                    })
        {
            1 // High - close to target
        } else if current_fps
            >= target_fps
                * (0.6
                    + if current_level <= 2 {
                        hysteresis_margin
                    } else {
                        -hysteresis_margin
                    })
        {
            2 // Medium - below target
        } else {
            3 // Low - significantly below target
        };

        if suggested_level == self.lod_state.last_performance_level {
            // Same as before, accumulate stable time
            self.lod_state.fps_stable_time += delta_time;
            self.lod_state.last_performance_level
        } else {
            // Different suggestion - check if we should switch
            if suggested_level > self.lod_state.last_performance_level {
                // Degrading quality (FPS dropping) - switch immediately for responsiveness
                self.lod_state.fps_stable_time = 0.0;
                self.lod_state.last_performance_level = suggested_level;
                suggested_level
            } else {
                // Improving quality (FPS rising) - require stable time to prevent thrashing
                const STABLE_TIME_REQUIRED: f32 = 0.5; // Half second of stable FPS before upgrading
                self.lod_state.fps_stable_time += delta_time;

                if self.lod_state.fps_stable_time >= STABLE_TIME_REQUIRED {
                    // FPS has been stable and good for long enough, allow upgrade
                    self.lod_state.fps_stable_time = 0.0;
                    self.lod_state.last_performance_level = suggested_level;
                    suggested_level
                } else {
                    // Not stable enough yet, keep current level
                    self.lod_state.last_performance_level
                }
            }
        }
    }

    /// Apply the active LOD quality level to rendering parameters
    fn apply_lod_quality(&mut self) {
        let quality = &self.lod_state.active_quality;

        // Apply quality parameters to fractal params
        self.max_steps = quality.max_steps;
        self.min_distance = quality.min_distance;
        self.shadow_samples = quality.shadow_samples;
        self.shadow_step_factor = quality.shadow_step_factor;
        self.ao_step_size = quality.ao_step_size;
        self.dof_samples = quality.dof_samples;

        // Render scale would require render target resizing, which we'll skip for now
        // This could be added in a future phase
    }
}

#[cfg(test)]
mod tests;
