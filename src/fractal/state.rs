//! Substruct field grouping for `FractalParams` (ARC-015).
//!
//! Prior to ARC-015, `FractalParams` was a single ~100-field God object that
//! mixed three concerns: authored render settings, LOD runtime state (FPS
//! ring buffer, motion tracking), and strange-attractor accumulation
//! bookkeeping (counters, last-view snapshots, pending-clear flags). Undo
//! cloned the whole struct every frame, including the multi-KB FPS deque and
//! the transient accumulation counters — restoring them on undo was both
//! wasteful and semantically wrong (redo of an old view should not resurrect
//! stale FPS history).
//!
//! The three structs below split those concerns. `FractalParams` composes
//! them as `pub settings: RenderSettings`, `pub lod: LodRuntime`,
//! `pub accum: AccumulationState`. The serialization boundary (`Settings` /
//! `to_settings` / `from_settings`) maps flat into `RenderSettings` plus
//! `lod.lod_config`; the on-disk YAML schema is byte-identical to the
//! pre-refactor layout (pinned by `tests_default_settings.yaml`).

use crate::lod::{LODConfig, LODState};
use glam::Vec3;

use super::{
    ChannelSource, ColorMode, ColorPalette, FogMode, FractalType, ProceduralPalette, RenderMode,
    ShadingModel,
};

/// Authored rendering parameters — every user-facing knob that the UI exposes,
/// that round-trips through `Settings` to YAML, and that undo/redo restores.
///
/// This is the largest of the three `FractalParams` substructs because almost
/// every field on the old God object was an authored value. Fields retain the
/// same names, types, and grouping comments they had on `FractalParams`
/// pre-refactor, so the call-site migration is a mechanical path-segment
/// insertion (`params.max_iterations` → `params.settings.max_iterations`).
///
/// `palette` and `render_mode` are derived (from `palette_index` and
/// `fractal_type` respectively) at load time and are not directly serialized
/// by `Settings`; they live here, alongside the values they mirror, because
/// they are stable authored state restored on undo.
/// Range the "Scale" slider exposes for [`RenderSettings::fractal_scale`].
///
/// An egui slider clamps the value it is bound to as a side effect of being
/// drawn, so a per-type default outside this range is silently rewritten the
/// first time the 3D Parameters panel renders — the fractal then renders at the
/// wrong size with nothing in the settings to explain it. Every value
/// `FractalParams::switch_fractal` assigns must therefore lie inside this
/// range; `fractal_scale_defaults_survive_the_ui_slider` pins that.
pub const FRACTAL_SCALE_RANGE: std::ops::RangeInclusive<f32> = 0.05..=5.0;

#[derive(Clone)]
pub struct RenderSettings {
    /// Which fractal is rendered; determines whether 2D or 3D shaders run.
    pub fractal_type: FractalType,
    /// 2D vs 3D pipeline. Derived from `fractal_type` — do not set manually;
    /// `switch_fractal` keeps the two in sync.
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

    // Procedural palette settings
    pub procedural_palette: ProceduralPalette,
    /// Custom procedural palette parameters (for Custom type)
    /// Format: [brightness_r, brightness_g, brightness_b, _]
    pub procedural_brightness: [f32; 3],
    /// Format: [contrast_r, contrast_g, contrast_b, _]
    pub procedural_contrast: [f32; 3],
    /// Format: [frequency_r, frequency_g, frequency_b, _]
    pub procedural_frequency: [f32; 3],
    /// Format: [phase_r, phase_g, phase_b, _]
    pub procedural_phase: [f32; 3],

    // 2D specific
    pub center_2d: [f64; 2],
    /// Optional high-precision 2D center as decimal strings (ENH-001 Phase C).
    /// When `Some`, the perturbation reference orbit parses these to `FBig`
    /// instead of using `center_2d`, so zoom past ~1e15 stays correct. Cleared
    /// by pan/zoom navigation; set by the precise-center UI / preset entry.
    pub center_2d_precise: Option<[String; 2]>,
    /// 2D zoom factor. Stored as f64 (ARC-001) so per-frame `* factor` accumulation
    /// in `zoom_at` does not round at f32 precision across long zoom sequences.
    /// The GPU uniform receives this as f32 (cast at the boundary in `Uniforms::update`),
    /// which is the remaining precision limiter; the double-float center (hi/lo) is what
    /// actually extends the on-GPU ceiling to ~1e11.
    pub zoom_2d: f64,
    pub julia_c: [f32; 2],
    pub max_iterations: u32,

    // 3D specific
    /// Fractal power exponent (e.g. `2.0` for the classic `z^2 + c` 2D
    /// fractals, `8.0` for the classic Mandelbulb).
    pub power: f32,
    pub max_steps: u32,
    pub min_distance: f32,
    pub ambient_occlusion: bool,
    pub ao_intensity: f32,
    pub ao_step_size: f32,
    /// Shadow mode: `0` = off, `1` = hard, `2` = soft.
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

    // Strange-attractor accumulation — authored knobs serialized via `Settings`.
    /// Enable compute shader accumulation for strange attractors
    pub attractor_accumulation_enabled: bool,
    /// Number of orbit iterations per frame (higher = more detail but slower)
    pub attractor_iterations_per_frame: u32,
    /// Log scale factor for density display
    pub attractor_log_scale: f32,
}

impl Default for RenderSettings {
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

            // Procedural palette defaults
            procedural_palette: ProceduralPalette::None,
            // Default custom parameters create a rainbow-like gradient
            procedural_brightness: [0.5, 0.5, 0.5],
            procedural_contrast: [0.5, 0.5, 0.5],
            procedural_frequency: [1.0, 1.0, 1.0],
            procedural_phase: [0.0, 0.333, 0.667],

            center_2d: [0.0f64, 0.0f64],
            center_2d_precise: None,
            zoom_2d: 1.0,
            julia_c: [-0.7, 0.27015],
            max_iterations: 80,

            power: 2.0, // Default for Mandelbrot2D/Julia2D (z^2 + c)
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

            // Strange attractor accumulation (disabled by default)
            attractor_accumulation_enabled: false,
            attractor_iterations_per_frame: 10_000,
            attractor_log_scale: 4.0,
        }
    }
}

/// LOD (Level of Detail) runtime subsystem — configuration plus transient state.
///
/// `lod_config` is serialized in `Settings`; `lod_state` is pure runtime (FPS
/// ring buffer, motion tracking EMA, transition progress). Grouping them under
/// one substruct means undo clones `RenderSettings` only — the multi-KB FPS
/// deque and the motion-tracking history are never restored on undo (which
/// would resurrect stale FPS measurements for a now-different scene).
#[derive(Clone, Default)]
pub struct LodRuntime {
    pub lod_config: LODConfig,
    pub lod_state: LODState,
}

/// Strange-attractor accumulation runtime bookkeeping.
///
/// These fields track accumulation progress and detect view changes that
/// should trigger a buffer clear. They are NOT serialized in `Settings`
/// (except indirectly: the user-facing accumulation knobs live in
/// `RenderSettings`). Storing them separately means undo restores the user's
/// accumulation preferences (enabled flag, log scale, iterations-per-frame)
/// without resurrecting stale counters or view-change snapshots.
#[derive(Clone)]
pub struct AccumulationState {
    /// Total accumulated iterations (display only)
    pub total_iterations: u64,
    /// Flag to clear accumulation on next frame
    pub pending_clear: bool,
    /// Flag to pause accumulation
    pub paused: bool,
    /// Maximum iterations before auto-pause (0 = unlimited). Not serialized;
    /// reset to the default on load.
    pub max_iterations: u64,
    /// Last view center for detecting pan (triggers auto-clear)
    pub last_center: [f64; 2],
    /// Last zoom level for detecting zoom (triggers auto-clear). f64 to match
    /// `RenderSettings::zoom_2d` (ARC-001) so change detection isn't tripped
    /// by f32 rounding.
    pub last_zoom: f64,
    /// Last julia_c parameters (triggers auto-clear on change)
    pub last_julia_c: [f32; 2],
}

impl Default for AccumulationState {
    fn default() -> Self {
        Self {
            total_iterations: 0,
            pending_clear: false,
            paused: false,
            max_iterations: 8_000_000,
            last_center: [0.0, 0.0],
            last_zoom: 1.0,
            // Mirrors the `julia_c` default in `RenderSettings`.
            last_julia_c: [-0.7, 0.27015],
        }
    }
}
