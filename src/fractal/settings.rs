use super::ui_state::*;
use super::{ChannelSource, ColorMode, FogMode, FractalType, ShadingModel, UIState};
use crate::lod::LODConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub fractal_type: FractalType,
    pub shading_model: ShadingModel,
    pub color_mode: ColorMode,
    pub palette_index: usize,
    pub orbit_trap_scale: f32,
    pub channel_r: ChannelSource,
    pub channel_g: ChannelSource,
    pub channel_b: ChannelSource,

    // UI state
    #[serde(default)]
    pub ui_state: UIState,
    #[serde(default)]
    pub auto_open_captures: bool,
    #[serde(default = "default_custom_width")]
    pub custom_width: String,
    #[serde(default = "default_custom_height")]
    pub custom_height: String,

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
    #[serde(default = "default_shadow_mode", alias = "soft_shadows")]
    pub shadow_mode: u32, // 0=off, 1=hard, 2=soft; alias preserves old bool field
    pub shadow_softness: f32,
    pub shadow_max_distance: f32,
    pub shadow_samples: u32,
    pub shadow_step_factor: f32,
    pub depth_of_field: bool,
    pub dof_focal_length: f32,
    pub dof_aperture: f32,
    #[serde(default = "default_dof_samples")]
    pub dof_samples: u32,

    // 3D fractal parameters
    pub fractal_scale: f32,
    pub fractal_fold: f32,
    pub fractal_min_radius: f32,

    // Material properties
    pub roughness: f32,
    pub metallic: f32,
    pub albedo: [f32; 3],

    // Lighting
    pub light_intensity: f32,
    pub ambient_light: f32,
    #[serde(default = "default_light_azimuth")]
    pub light_azimuth: f32, // Horizontal angle in degrees (0-360)
    #[serde(default = "default_light_elevation")]
    pub light_elevation: f32, // Vertical angle in degrees (0-90)

    // Floor
    pub show_floor: bool,
    pub floor_height: f32,
    pub floor_color1: [f32; 3],
    pub floor_color2: [f32; 3],
    #[serde(default = "default_true")]
    pub floor_reflections: bool,
    #[serde(default = "default_reflection_strength")]
    pub floor_reflection_strength: f32,

    // Fog
    pub fog_enabled: bool,
    pub fog_mode: FogMode,
    pub fog_density: f32,
    pub fog_color: [f32; 3],

    // Ray marching
    pub use_adaptive_step: bool,
    pub fixed_step_size: f32,
    pub step_multiplier: f32,
    pub max_distance: f32,

    // Camera (3D mode)
    pub camera_position: [f32; 3],
    pub camera_target: [f32; 3],
    pub camera_speed: f32,
    pub camera_fov: f32,

    // Camera orbit
    #[serde(default)]
    pub auto_orbit: bool,
    #[serde(default = "default_orbit_speed")]
    pub orbit_speed: f32,

    // Post-processing
    #[serde(default = "default_one")]
    pub brightness: f32,
    #[serde(default = "default_one")]
    pub contrast: f32,
    #[serde(default = "default_one")]
    pub saturation: f32,
    #[serde(default)]
    pub hue_shift: f32,

    #[serde(default)]
    pub vignette_enabled: bool,
    #[serde(default = "default_vignette_intensity")]
    pub vignette_intensity: f32,
    #[serde(default = "default_vignette_radius")]
    pub vignette_radius: f32,

    #[serde(default)]
    pub bloom_enabled: bool,
    #[serde(default = "default_bloom_threshold")]
    pub bloom_threshold: f32,
    #[serde(default = "default_bloom_intensity")]
    pub bloom_intensity: f32,
    #[serde(default = "default_bloom_radius")]
    pub bloom_radius: f32,

    #[serde(default)]
    pub fxaa_enabled: bool,

    // LOD system
    #[serde(default)]
    pub lod_config: LODConfig,
}

// Preset categories for organization
