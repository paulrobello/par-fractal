//! Fractal parameter model and supporting catalogs.
//!
//! The central type is `FractalParams` — the single source of truth for every
//! rendering parameter across both 2D escape-time and 3D ray-marched fractals.
//! Its values are uploaded to the GPU uniform buffer by the renderer each frame
//! (see `crate::renderer::uniforms`), and it serializes to and from `Settings`
//! for YAML persistence and JSON import/export.
//!
//! This module re-exports the fractal-type enum, render/shading/color models,
//! color palettes, presets, and serializable settings from its private
//! submodules (`types`, `palettes`, `presets`, `settings`, `ui_state`).
//!
//! Note on precision: `center_2d` and `zoom_2d` are held in `f64` so that deep
//! zooms accumulate without f32 rounding (the GPU receives an f32 cast plus a
//! double-float center decomposition, extending the usable zoom ceiling). See
//! the ARC-001 notes on the individual fields.

// Module declarations
mod palettes;
mod presets;
mod settings;
mod state;
mod types;
mod ui_state;

// Re-exports
pub use palettes::*;
pub use presets::*;
pub use settings::*;
pub use state::*;
pub use types::*;
pub use ui_state::*;

use glam::Vec3;

use crate::lod::LODState;

/// Clamp a finite f32 to `[min, max]`; NaN/Inf inputs return `default`.
/// Used at the `from_settings` trust boundary because presets/settings arrive
/// from YAML, JSON import, and web localStorage — all bypassing the bounded UI
/// sliders. `f32::clamp` alone propagates NaN (NaN order comparisons are always
/// false), so the `is_finite()` guard is mandatory.
fn clamp_finite_f32(v: f32, min: f32, max: f32, default: f32) -> f32 {
    if v.is_finite() {
        v.clamp(min, max)
    } else {
        default
    }
}

/// Clamp a finite f64 to `[min, max]`; NaN/Inf inputs return `default`.
fn clamp_finite_f64(v: f64, min: f64, max: f64, default: f64) -> f64 {
    if v.is_finite() {
        v.clamp(min, max)
    } else {
        default
    }
}

/// The CPU-side source of truth for every fractal rendering parameter.
///
/// One `FractalParams` drives the whole pipeline: the renderer reads its
/// fields each frame to build the GPU uniform buffer (via
/// `crate::renderer::uniforms::Uniforms`), the UI mutates it through sliders
/// and the command palette, and it round-trips to disk through [`to_settings`]
/// / [`from_settings`] as YAML. `center_2d` and `zoom_2d` are `f64` to preserve
/// precision on deep zoom (see the module-level ARC-001 note).
///
/// ARC-015: the previously flat ~100-field struct is now a composition of
/// three focused substructs (see [`state`]):
///
/// - [`settings: RenderSettings`](RenderSettings) — authored, serialized,
///   restored on undo. The vast majority of fields live here.
/// - [`lod: LodRuntime`](LodRuntime) — LOD config (serialized) plus the
///   transient FPS ring buffer and motion-tracking state (NOT restored on undo;
///   undoing back to a slow scene should not resurrect the FPS measurements
///   of the scene you just left).
/// - [`accum: AccumulationState`](AccumulationState) — strange-attractor
///   accumulation bookkeeping (counters, last-view snapshots, pending-clear
///   flag). NOT restored on undo; the accumulation buffer is explicitly
///   cleared on parameter rollback instead.
///
/// Migration note for callers: pre-ARC-015 field paths like
/// `params.max_iterations` are now `params.settings.max_iterations`,
/// `params.lod_state` is `params.lod.lod_state`, and
/// `params.attractor_total_iterations` is `params.accum.total_iterations`.
/// Methods on `FractalParams` itself (`switch_fractal`, `zoom_at`,
/// `effective_quality`, `update_lod`, …) keep the same signatures.
///
/// [`to_settings`]: FractalParams::to_settings
/// [`from_settings`]: FractalParams::from_settings
#[derive(Clone, Default)]
pub struct FractalParams {
    pub settings: RenderSettings,
    pub lod: LodRuntime,
    pub accum: AccumulationState,
}

impl FractalParams {
    /// Zoom by `factor` keeping the fractal point under `cursor_ndc` fixed.
    ///
    /// `cursor_ndc` is the cursor position in normalized device coordinates:
    /// `[-1, 1]` on both axes, y-up (i.e. `norm_y = 1.0 - 2.0 * cursor_y / height`),
    /// BEFORE aspect correction. `aspect` is `width / height`.
    ///
    /// This is the single seam for zoom-at-cursor in 2D mode; the three input paths
    /// (continuous shift+drag, pinch, scroll wheel) all converge here so the f64
    /// representation (ARC-001) and any future precision work flows through one place.
    /// Reference behavior: matches `src/app/update.rs`'s continuous-zoom path exactly.
    pub fn zoom_at(&mut self, cursor_ndc: (f64, f64), factor: f64, aspect: f64) {
        let old_zoom = self.settings.zoom_2d;
        // Fractal point under the cursor in old coordinates.
        let fx = self.settings.center_2d[0] + (cursor_ndc.0 * 2.0 / old_zoom) * aspect;
        let fy = self.settings.center_2d[1] + cursor_ndc.1 * 2.0 / old_zoom;
        let new_zoom = old_zoom * factor;
        // Re-center so the same fractal point stays under the cursor at the new zoom.
        self.settings.center_2d[0] = fx - (cursor_ndc.0 * 2.0 / new_zoom) * aspect;
        self.settings.center_2d[1] = fy - cursor_ndc.1 * 2.0 / new_zoom;
        self.settings.zoom_2d = new_zoom;
    }

    /// Serialize to the on-disk [`Settings`] representation.
    ///
    /// Camera position/target and UI state are written by the caller (the
    /// `App`), not captured here — they default to placeholders and are
    /// overwritten before serialization.
    pub fn to_settings(&self) -> Settings {
        Settings {
            fractal_type: self.settings.fractal_type,
            shading_model: self.settings.shading_model,
            color_mode: self.settings.color_mode,
            palette_index: self.settings.palette_index,
            orbit_trap_scale: self.settings.orbit_trap_scale,
            channel_r: self.settings.channel_r,
            channel_g: self.settings.channel_g,
            channel_b: self.settings.channel_b,
            procedural_palette: self.settings.procedural_palette,
            procedural_brightness: self.settings.procedural_brightness,
            procedural_contrast: self.settings.procedural_contrast,
            procedural_frequency: self.settings.procedural_frequency,
            procedural_phase: self.settings.procedural_phase,
            ui_state: UIState::default(), // Will be overridden by App if UI state exists
            auto_open_captures: false,    // Will be overridden by App with UI state
            center_2d: self.settings.center_2d,
            zoom_2d: self.settings.zoom_2d,
            julia_c: self.settings.julia_c,
            max_iterations: self.settings.max_iterations,
            power: self.settings.power,
            max_steps: self.settings.max_steps,
            min_distance: self.settings.min_distance,
            ambient_occlusion: self.settings.ambient_occlusion,
            ao_intensity: self.settings.ao_intensity,
            ao_step_size: self.settings.ao_step_size,
            shadow_mode: self.settings.shadow_mode,
            shadow_softness: self.settings.shadow_softness,
            shadow_max_distance: self.settings.shadow_max_distance,
            shadow_samples: self.settings.shadow_samples,
            shadow_step_factor: self.settings.shadow_step_factor,
            depth_of_field: self.settings.depth_of_field,
            dof_focal_length: self.settings.dof_focal_length,
            dof_aperture: self.settings.dof_aperture,
            dof_samples: self.settings.dof_samples,
            fractal_scale: self.settings.fractal_scale,
            fractal_fold: self.settings.fractal_fold,
            fractal_min_radius: self.settings.fractal_min_radius,
            roughness: self.settings.roughness,
            metallic: self.settings.metallic,
            albedo: self.settings.albedo.to_array(),
            light_intensity: self.settings.light_intensity,
            ambient_light: self.settings.ambient_light,
            light_azimuth: self.settings.light_azimuth,
            light_elevation: self.settings.light_elevation,
            show_floor: self.settings.show_floor,
            floor_height: self.settings.floor_height,
            floor_color1: self.settings.floor_color1.to_array(),
            floor_color2: self.settings.floor_color2.to_array(),
            floor_reflections: self.settings.floor_reflections,
            floor_reflection_strength: self.settings.floor_reflection_strength,
            fog_enabled: self.settings.fog_enabled,
            fog_mode: self.settings.fog_mode,
            fog_density: self.settings.fog_density,
            fog_color: self.settings.fog_color.to_array(),
            use_adaptive_step: self.settings.use_adaptive_step,
            fixed_step_size: self.settings.fixed_step_size,
            step_multiplier: self.settings.step_multiplier,
            max_distance: self.settings.max_distance,
            camera_position: [0.0, 0.0, 3.0], // Will be overridden by App
            camera_target: [0.0, 0.0, 0.0],   // Will be overridden by App
            camera_speed: self.settings.camera_speed,
            camera_fov: self.settings.camera_fov,
            auto_orbit: self.settings.auto_orbit,
            orbit_speed: self.settings.orbit_speed,
            brightness: self.settings.brightness,
            contrast: self.settings.contrast,
            saturation: self.settings.saturation,
            hue_shift: self.settings.hue_shift,
            vignette_enabled: self.settings.vignette_enabled,
            vignette_intensity: self.settings.vignette_intensity,
            vignette_radius: self.settings.vignette_radius,
            bloom_enabled: self.settings.bloom_enabled,
            bloom_threshold: self.settings.bloom_threshold,
            bloom_intensity: self.settings.bloom_intensity,
            bloom_radius: self.settings.bloom_radius,
            fxaa_enabled: self.settings.fxaa_enabled,
            lod_config: self.lod.lod_config.clone(),
            custom_width: default_custom_width(),
            custom_height: default_custom_height(),
            attractor_accumulation_enabled: self.settings.attractor_accumulation_enabled,
            attractor_iterations_per_frame: self.settings.attractor_iterations_per_frame,
            attractor_log_scale: self.settings.attractor_log_scale,
        }
    }

    /// Reconstruct from untrusted [`Settings`].
    ///
    /// This is the trust boundary for preset/loading (SEC-001): `Settings`
    /// arrives from YAML, JSON import, and web `localStorage`, so every
    /// resource-driving integer is clamped to a safe maximum and every float
    /// reaching a GPU uniform is run through `clamp_finite_f32` / `_f64` to
    /// reject NaN/Inf. Hostile or corrupt presets cannot smuggle bad values
    /// past this constructor. `render_mode` is re-derived from `fractal_type`.
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
            | FractalType::Threeply2D
            | FractalType::Buddhabrot2D => RenderMode::TwoD,
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

        // SEC-001: Trust-boundary clamps. Clamp every resource-driving field to a
        // sane maximum (mirroring UI slider bounds where they exist) and reject
        // NaN/Inf on every float that reaches a GPU uniform. The clamped values
        // are also reused for the attractor_last_* bookkeeping below so a hostile
        // preset cannot smuggle bad values in through those either.
        let max_iterations = settings.max_iterations.clamp(1, 100_000);
        let max_steps = settings.max_steps.clamp(1, 2_000);
        let attractor_iterations_per_frame =
            settings.attractor_iterations_per_frame.clamp(1, 2_000_000);
        let shadow_samples = settings.shadow_samples.min(512);
        let dof_samples = settings.dof_samples.min(64);
        let zoom_2d = clamp_finite_f64(settings.zoom_2d, 1e-6, 1e30, 1.0);
        let min_distance = clamp_finite_f32(settings.min_distance, 1e-7, 1.0, 0.00035);
        let orbit_trap_scale = clamp_finite_f32(settings.orbit_trap_scale, 0.0, 1e6, 1.0);
        let power = clamp_finite_f32(settings.power, -1e4, 1e4, 2.0);
        let julia_c = [
            clamp_finite_f32(settings.julia_c[0], -1e6, 1e6, -0.7),
            clamp_finite_f32(settings.julia_c[1], -1e6, 1e6, 0.27015),
        ];
        let ao_intensity = clamp_finite_f32(settings.ao_intensity, 0.0, 1e6, 1.0);
        let ao_step_size = clamp_finite_f32(settings.ao_step_size, 1e-7, 1e6, 0.12);
        let shadow_softness = clamp_finite_f32(settings.shadow_softness, 0.0, 1e6, 8.0);
        let shadow_max_distance = clamp_finite_f32(settings.shadow_max_distance, 0.0, 1e6, 5.0);
        let shadow_step_factor = clamp_finite_f32(settings.shadow_step_factor, 0.0, 1e6, 0.6);
        let dof_focal_length = clamp_finite_f32(settings.dof_focal_length, 0.0, 1e6, 6.0);
        let dof_aperture = clamp_finite_f32(settings.dof_aperture, 0.0, 1e6, 0.01);
        let fractal_scale = clamp_finite_f32(settings.fractal_scale, -1e6, 1e6, 2.0);
        let fractal_fold = clamp_finite_f32(settings.fractal_fold, 0.0, 1e6, 1.0);
        let fractal_min_radius = clamp_finite_f32(settings.fractal_min_radius, 0.0, 1e6, 0.5);
        let roughness = clamp_finite_f32(settings.roughness, 0.0, 1e6, 0.4);
        let metallic = clamp_finite_f32(settings.metallic, 0.0, 1e6, 0.20);
        let light_intensity = clamp_finite_f32(settings.light_intensity, 0.0, 1e6, 3.0);
        let ambient_light = clamp_finite_f32(settings.ambient_light, 0.0, 1e6, 0.15);
        let light_azimuth = clamp_finite_f32(settings.light_azimuth, -1e6, 1e6, 45.0);
        let light_elevation = clamp_finite_f32(settings.light_elevation, -1e6, 1e6, 35.0);
        let floor_height = clamp_finite_f32(settings.floor_height, -1e6, 1e6, -2.0);
        let floor_reflection_strength =
            clamp_finite_f32(settings.floor_reflection_strength, 0.0, 1e6, 0.5);
        let fog_density = clamp_finite_f32(settings.fog_density, 0.0, 1e6, 0.005);
        let fixed_step_size = clamp_finite_f32(settings.fixed_step_size, 1e-7, 1e6, 0.1);
        let step_multiplier = clamp_finite_f32(settings.step_multiplier, 0.0, 1e6, 1.0);
        let max_distance = clamp_finite_f32(settings.max_distance, 0.0, 1e6, 100.0);
        let camera_speed = clamp_finite_f32(settings.camera_speed, 0.0, 1e6, 2.0);
        let camera_fov = clamp_finite_f32(settings.camera_fov, 1e-3, 180.0, 45.0);
        let orbit_speed = clamp_finite_f32(settings.orbit_speed, -1e6, 1e6, 0.2);
        let brightness = clamp_finite_f32(settings.brightness, 0.0, 1e6, 1.0);
        let contrast = clamp_finite_f32(settings.contrast, 0.0, 1e6, 1.0);
        let saturation = clamp_finite_f32(settings.saturation, 0.0, 1e6, 1.0);
        let hue_shift = clamp_finite_f32(settings.hue_shift, -1e6, 1e6, 0.0);
        let vignette_intensity = clamp_finite_f32(settings.vignette_intensity, 0.0, 1e6, 0.5);
        let vignette_radius = clamp_finite_f32(settings.vignette_radius, 0.0, 1e6, 0.8);
        let bloom_threshold = clamp_finite_f32(settings.bloom_threshold, 0.0, 1e6, 0.75);
        let bloom_intensity = clamp_finite_f32(settings.bloom_intensity, 0.0, 1e6, 0.1);
        let bloom_radius = clamp_finite_f32(settings.bloom_radius, 0.0, 1e6, 0.005);
        let attractor_log_scale = clamp_finite_f32(settings.attractor_log_scale, 0.0, 1e6, 4.0);
        let center_2d = [
            clamp_finite_f64(settings.center_2d[0], -1e15, 1e15, 0.0),
            clamp_finite_f64(settings.center_2d[1], -1e15, 1e15, 0.0),
        ];
        let procedural_brightness = [
            clamp_finite_f32(settings.procedural_brightness[0], -1e6, 1e6, 0.5),
            clamp_finite_f32(settings.procedural_brightness[1], -1e6, 1e6, 0.5),
            clamp_finite_f32(settings.procedural_brightness[2], -1e6, 1e6, 0.5),
        ];
        let procedural_contrast = [
            clamp_finite_f32(settings.procedural_contrast[0], -1e6, 1e6, 0.5),
            clamp_finite_f32(settings.procedural_contrast[1], -1e6, 1e6, 0.5),
            clamp_finite_f32(settings.procedural_contrast[2], -1e6, 1e6, 0.5),
        ];
        let procedural_frequency = [
            clamp_finite_f32(settings.procedural_frequency[0], -1e6, 1e6, 1.0),
            clamp_finite_f32(settings.procedural_frequency[1], -1e6, 1e6, 1.0),
            clamp_finite_f32(settings.procedural_frequency[2], -1e6, 1e6, 1.0),
        ];
        let procedural_phase = [
            clamp_finite_f32(settings.procedural_phase[0], -1e6, 1e6, 0.0),
            clamp_finite_f32(settings.procedural_phase[1], -1e6, 1e6, 0.333),
            clamp_finite_f32(settings.procedural_phase[2], -1e6, 1e6, 0.667),
        ];
        let albedo = Vec3::from_array([
            clamp_finite_f32(settings.albedo[0], 0.0, 1e6, 0.8),
            clamp_finite_f32(settings.albedo[1], 0.0, 1e6, 0.8),
            clamp_finite_f32(settings.albedo[2], 0.0, 1e6, 0.8),
        ]);
        let floor_color1 = Vec3::from_array([
            clamp_finite_f32(settings.floor_color1[0], 0.0, 1e6, 1.0),
            clamp_finite_f32(settings.floor_color1[1], 0.0, 1e6, 1.0),
            clamp_finite_f32(settings.floor_color1[2], 0.0, 1e6, 1.0),
        ]);
        let floor_color2 = Vec3::from_array([
            clamp_finite_f32(settings.floor_color2[0], 0.0, 1e6, 0.0),
            clamp_finite_f32(settings.floor_color2[1], 0.0, 1e6, 0.0),
            clamp_finite_f32(settings.floor_color2[2], 0.0, 1e6, 0.0),
        ]);
        let fog_color = Vec3::from_array([
            clamp_finite_f32(settings.fog_color[0], 0.0, 1e6, 0.0),
            clamp_finite_f32(settings.fog_color[1], 0.0, 1e6, 0.0),
            clamp_finite_f32(settings.fog_color[2], 0.0, 1e6, 0.0),
        ]);

        Self {
            settings: RenderSettings {
                fractal_type: settings.fractal_type,
                render_mode,
                shading_model: settings.shading_model,
                color_mode: settings.color_mode,
                palette,
                palette_index,
                palette_offset: 0.0,
                orbit_trap_scale,
                channel_r: settings.channel_r,
                channel_g: settings.channel_g,
                channel_b: settings.channel_b,
                procedural_palette: settings.procedural_palette,
                procedural_brightness,
                procedural_contrast,
                procedural_frequency,
                procedural_phase,
                center_2d,
                zoom_2d,
                julia_c,
                max_iterations,
                power,
                max_steps,
                min_distance,
                ambient_occlusion: settings.ambient_occlusion,
                ao_intensity,
                ao_step_size,
                shadow_mode: settings.shadow_mode,
                shadow_softness,
                shadow_max_distance,
                shadow_samples,
                shadow_step_factor,
                depth_of_field: settings.depth_of_field,
                dof_focal_length,
                dof_aperture,
                dof_samples,
                fractal_scale,
                fractal_fold,
                fractal_min_radius,
                roughness,
                metallic,
                albedo,
                light_intensity,
                ambient_light,
                light_azimuth,
                light_elevation,
                show_floor: settings.show_floor,
                floor_height,
                floor_color1,
                floor_color2,
                floor_reflections: settings.floor_reflections,
                floor_reflection_strength,
                fog_enabled: settings.fog_enabled,
                fog_mode: settings.fog_mode,
                fog_density,
                fog_color,
                use_adaptive_step: settings.use_adaptive_step,
                fixed_step_size,
                step_multiplier,
                max_distance,
                camera_speed,
                camera_fov,
                auto_orbit: settings.auto_orbit,
                orbit_speed,
                brightness,
                contrast,
                saturation,
                hue_shift,
                vignette_enabled: settings.vignette_enabled,
                vignette_intensity,
                vignette_radius,
                bloom_enabled: settings.bloom_enabled,
                bloom_threshold,
                bloom_intensity,
                bloom_radius,
                fxaa_enabled: settings.fxaa_enabled,
                attractor_accumulation_enabled: settings.attractor_accumulation_enabled,
                attractor_iterations_per_frame,
                attractor_log_scale,
            },
            lod: LodRuntime {
                lod_config: settings.lod_config,
                lod_state: LODState::default(),
            },
            accum: AccumulationState {
                total_iterations: 0, // Always reset on load
                pending_clear: false,
                paused: false,
                max_iterations: 8_000_000,
                last_center: center_2d,
                last_zoom: zoom_2d,
                last_julia_c: julia_c,
            },
        }
    }

    /// Save to `settings.yaml` under the OS config dir (native).
    ///
    /// Errors if the config directory cannot be determined or writing the
    /// YAML fails.
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

    /// Save to file (web stub — persistence is not yet implemented for WASM).
    #[cfg(target_arch = "wasm32")]
    pub fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Settings persistence not yet implemented for web
        Ok(())
    }

    /// Load from `settings.yaml` under the OS config dir (native).
    ///
    /// Returns `None` if the config dir cannot be determined, the file is
    /// missing, or the YAML fails to parse. Values are trust-boundary clamped
    /// via [`from_settings`](Self::from_settings).
    ///
    /// Unused inside this crate after ARC-014 routed construction through
    /// `App::load_settings_via_platform`; retained as part of the public rlib
    /// API. Slated for removal in QA-020.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(dead_code)]
    pub fn load_from_file() -> Option<Self> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let settings_file = config_dir.config_dir().join("settings.yaml");

            if let Ok(yaml) = std::fs::read_to_string(settings_file)
                && let Ok(settings) = serde_yaml::from_str::<Settings>(&yaml)
            {
                println!("Settings loaded");
                return Some(Self::from_settings(settings));
            }
        }
        None
    }

    /// Load from file (web stub — always returns `None` on WASM).
    #[cfg(target_arch = "wasm32")]
    #[allow(dead_code)]
    pub fn load_from_file() -> Option<Self> {
        // Settings persistence not yet implemented for web
        None
    }

    /// Switch to a different fractal type.
    ///
    /// Sets `fractal_type`, re-derives `render_mode` (2D vs 3D) to match, and
    /// applies fractal-specific defaults (e.g. `power = 8.0` for the classic
    /// Mandelbulb, tuned view bounds and iteration counts for strange
    /// attractors). Use this instead of assigning `fractal_type` directly so
    /// the two fields never disagree.
    pub fn switch_fractal(&mut self, fractal_type: FractalType) {
        self.settings.fractal_type = fractal_type;
        self.settings.render_mode = match fractal_type {
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
            | FractalType::Threeply2D
            | FractalType::Buddhabrot2D => RenderMode::TwoD,
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
            FractalType::Mandelbrot2D
            | FractalType::Julia2D
            | FractalType::BurningShip2D
            | FractalType::Tricorn2D
            | FractalType::Phoenix2D
            | FractalType::Celtic2D => {
                self.settings.power = 2.0; // Classic z^2 + c
            }
            FractalType::Mandelbulb3D => {
                self.settings.power = 8.0; // Classic Mandelbulb
            }
            FractalType::MengerSponge3D => {
                self.settings.fractal_scale = 1.0; // Double the apparent size (half the scale factor)
                self.settings.max_iterations = 7; // Default iterations for Menger Sponge
            }
            FractalType::SierpinskiPyramid3D => {
                self.settings.max_iterations = 12; // Default iterations for Sierpinski Pyramid
            }
            FractalType::Nova2D | FractalType::Lyapunov2D => {
                self.settings.max_iterations = 16;
            }
            FractalType::SierpinskiTriangle2D => {
                self.settings.max_iterations = 30;
            }
            FractalType::Mandelbox3D => {
                self.settings.fractal_scale = 1.0; // Double the apparent size
                self.settings.fractal_fold = 1.0;
                self.settings.fractal_min_radius = 0.5;
                self.settings.roughness = 0.21;
                self.settings.metallic = 0.32;
            }
            FractalType::OctahedralIFS3D | FractalType::IcosahedralIFS3D => {
                self.settings.fractal_fold = 1.7;
            }
            FractalType::ApollonianGasket3D => {
                self.settings.fractal_fold = 1.05;
                self.settings.fractal_min_radius = 0.6;
            }
            FractalType::Kleinian3D => {
                self.settings.fractal_scale = 1.5;
                self.settings.fractal_fold = 1.0; // Results in scale -2.0 (classic Mandelbox)
                self.settings.fractal_min_radius = 0.5;
            }
            FractalType::HybridMandelbulbJulia3D => {
                self.settings.fractal_scale = 1.5;
                self.settings.max_iterations = 8; // Lower for performance
            }
            FractalType::QuaternionCubic3D => {
                self.settings.fractal_scale = 1.5;
                self.settings.max_iterations = 8; // Lower for performance
            }
            // Strange Attractors 2D - set appropriate view bounds and iterations
            FractalType::Hopalong2D => {
                self.settings.center_2d = [0.5, 0.5];
                self.settings.zoom_2d = 0.3;
                self.settings.max_iterations = 1000;
                // julia_c used for parameters: x=a, y=b (c defaults to 0)
                self.settings.julia_c = [0.4, 1.0];
            }
            FractalType::Martin2D => {
                self.settings.center_2d = [0.0, 0.0];
                self.settings.zoom_2d = 0.05;
                self.settings.max_iterations = 1000;
                // julia_c.x = a
                self.settings.julia_c = [std::f32::consts::PI, 0.0];
            }
            FractalType::Gingerbreadman2D => {
                self.settings.center_2d = [2.0, 2.0];
                self.settings.zoom_2d = 0.15;
                self.settings.max_iterations = 1000;
            }
            FractalType::Chip2D => {
                self.settings.center_2d = [0.0, 0.0];
                self.settings.zoom_2d = 0.002;
                self.settings.max_iterations = 1000;
                self.settings.julia_c = [-15.0, -19.0];
                self.settings.power = 1.0;
            }
            FractalType::Quadruptwo2D => {
                self.settings.center_2d = [15.0, 17.0];
                self.settings.zoom_2d = 0.01;
                self.settings.max_iterations = 1000;
                self.settings.julia_c = [34.0, 1.0];
                self.settings.power = 5.0;
            }
            FractalType::Threeply2D => {
                self.settings.center_2d = [0.0, 0.0];
                self.settings.zoom_2d = 1.0;
                self.settings.max_iterations = 1000;
                self.settings.julia_c = [-55.0, -1.0];
                self.settings.power = -42.0;
            }
            // Buddhabrot - density visualization of Mandelbrot escape trajectories
            FractalType::Buddhabrot2D => {
                self.settings.center_2d = [0.4, 0.0]; // Centered for flipped Buddha view
                self.settings.zoom_2d = 0.45; // Zoomed to see full Buddha figure
                self.settings.max_iterations = 5000; // High iterations for detailed Buddha structure
                self.settings.attractor_accumulation_enabled = true; // Requires accumulation
                self.settings.attractor_iterations_per_frame = 50_000; // More samples for faster accumulation
                self.settings.attractor_log_scale = 1.0; // Lower for better contrast
            }
            // 3D Strange Attractors
            FractalType::Pickover3D => {
                self.settings.fractal_scale = 0.3;
                self.settings.max_iterations = 10000;
                // a, b, c, d via julia_c.x, julia_c.y, power, fractal_fold
                self.settings.julia_c = [2.24, 0.43];
                self.settings.power = -0.65;
                self.settings.fractal_fold = -2.43;
            }
            FractalType::Lorenz3D => {
                self.settings.fractal_scale = 0.05;
                self.settings.max_iterations = 10000;
                // sigma, rho, beta via julia_c.x, julia_c.y, power
                self.settings.julia_c = [10.0, 28.0];
                self.settings.power = 2.666667;
            }
            FractalType::Rossler3D => {
                self.settings.fractal_scale = 0.1;
                self.settings.max_iterations = 10000;
                // a, b, c via julia_c.x, julia_c.y, power
                self.settings.julia_c = [0.2, 0.2];
                self.settings.power = 5.7;
            }
            _ => {}
        }
    }

    /// Advance to the next color palette, wrapping around at the end.
    pub fn next_palette(&mut self) {
        self.settings.palette_index = (self.settings.palette_index + 1) % ColorPalette::ALL.len();
        self.settings.palette = ColorPalette::ALL[self.settings.palette_index];
    }

    /// Go to the previous color palette, wrapping around at the start.
    pub fn prev_palette(&mut self) {
        if self.settings.palette_index == 0 {
            self.settings.palette_index = ColorPalette::ALL.len() - 1;
        } else {
            self.settings.palette_index -= 1;
        }
        self.settings.palette = ColorPalette::ALL[self.settings.palette_index];
    }

    /// Randomize fractal parameters for creative exploration
    pub fn randomize(&mut self) {
        use rand::RngExt;
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
        self.settings.palette_index = rng.random_range(0..ColorPalette::ALL.len());
        self.settings.palette = ColorPalette::ALL[self.settings.palette_index];

        // Randomize color mode
        let color_modes = [
            ColorMode::Palette,
            ColorMode::RaySteps,
            ColorMode::Normals,
            ColorMode::OrbitTrapXYZ,
            ColorMode::OrbitTrapRadial,
        ];
        self.settings.color_mode = color_modes[rng.random_range(0..color_modes.len())];

        match self.settings.render_mode {
            RenderMode::TwoD => {
                // Randomize 2D parameters
                self.settings.julia_c = [rng.random_range(-2.0..2.0), rng.random_range(-2.0..2.0)];
                self.settings.max_iterations = rng.random_range(64..512);
            }
            RenderMode::ThreeD => {
                // Randomize 3D parameters
                self.settings.power = rng.random_range(4.0..12.0);
                self.settings.max_steps = rng.random_range(100..350);
                self.settings.fractal_scale = rng.random_range(0.8..3.0);

                if matches!(self.settings.fractal_type, FractalType::Mandelbox3D) {
                    self.settings.fractal_fold = rng.random_range(0.5..2.5);
                    self.settings.fractal_min_radius = rng.random_range(0.2..1.5);
                }

                // Randomize lighting
                self.settings.light_intensity = rng.random_range(1.5..6.0);
                self.settings.ambient_light = rng.random_range(0.05..0.4);

                // Randomize effects
                self.settings.ambient_occlusion = rng.random_bool(0.7); // 70% chance
                if self.settings.ambient_occlusion {
                    self.settings.ao_intensity = rng.random_range(1.0..6.0);
                }

                // 0=off, 1=hard, 2=soft
                self.settings.shadow_mode = if rng.random_bool(0.6) {
                    if rng.random_bool(0.7) { 2 } else { 1 } // 70% soft, 30% hard
                } else {
                    0
                };
                if self.settings.shadow_mode == 2 {
                    self.settings.shadow_softness = rng.random_range(4.0..20.0);
                }

                self.settings.fog_enabled = rng.random_bool(0.5); // 50% chance
                if self.settings.fog_enabled {
                    self.settings.fog_density = rng.random_range(0.001..0.05);
                }

                self.settings.show_floor = rng.random_bool(0.5); // 50% chance

                // Randomize material for PBR
                if self.settings.shading_model == ShadingModel::PBR {
                    self.settings.roughness = rng.random_range(0.1..0.9);
                    self.settings.metallic = rng.random_range(0.0..0.7);
                }
            }
        }
    }

    /// Update LOD system state and apply quality adjustments
    pub fn update_lod(&mut self, camera_pos: Vec3, camera_forward: Vec3, delta_time: f32) {
        if !self.lod.lod_config.enabled {
            // LOD disabled, ensure we're at max quality
            if self.lod.lod_state.current_level != 0 {
                self.lod.lod_state.current_level = 0;
                self.lod.lod_state.target_level = 0;
                self.lod.lod_state.transition_progress = 1.0;
                self.lod.lod_state.active_quality = self.lod.lod_config.quality_presets[0];
            }
            return;
        }

        // Update FPS tracking
        self.lod.lod_state.update_fps(delta_time);

        // Update motion tracking. ARC-007: 2D fractals pan/zoom rather than
        // translating a 3D camera, so feed the LOD motion detector the 2D
        // state instead of constructing a synthetic 3D camera vector (which
        // never registered wheel/pinch zoom as motion).
        if self.settings.render_mode == RenderMode::TwoD {
            self.lod.lod_state.update_motion_2d(
                self.settings.zoom_2d,
                self.settings.center_2d,
                delta_time,
                self.lod.lod_config.motion_threshold,
                self.lod.lod_config.motion_sensitivity,
            );
        } else {
            self.lod.lod_state.update_motion(
                camera_pos,
                camera_forward,
                delta_time,
                self.lod.lod_config.motion_threshold,
                self.lod.lod_config.motion_sensitivity,
            );
        }

        // Determine target LOD level based on strategy
        let target_level = self.calculate_target_lod_level(camera_pos, delta_time);

        // Update transition
        self.lod.lod_state.set_target(target_level);
        self.lod
            .lod_state
            .update_transition(delta_time, self.lod.lod_config.transition_duration);

        // Cache the active (interpolated) quality so `effective_quality()`
        // and the LOD debug overlay can read it. The values are NOT written
        // into the user-visible FractalParams fields — that clobbered slider
        // values and persisted degraded state into settings.yaml (ARC-008/QA-010).
        // The merge with user values happens in `Uniforms::update`.
        self.lod.lod_state.active_quality = self.lod.lod_state.get_active_quality(
            &self.lod.lod_config.quality_presets,
            self.lod.lod_config.smooth_transitions,
        );
    }

    /// The `QualityLevel` currently in effect for rendering.
    ///
    /// When LOD is enabled, this is the LOD-active preset (interpolated during
    /// smooth transitions). When LOD is disabled, it is a snapshot of the
    /// user's own slider values — `FractalParams` no longer carries LOD-derived
    /// quality in its serialized fields, so `to_settings()` persists only
    /// authored values. `Uniforms::update` merges this with the user-authored
    /// fields at uniform-build time; the merge is the only place LOD quality
    /// reaches the GPU. (ARC-008 / QA-010.)
    pub fn effective_quality(&self) -> crate::lod::QualityLevel {
        use crate::lod::QualityLevel;
        if self.lod.lod_config.enabled {
            self.lod.lod_state.get_active_quality(
                &self.lod.lod_config.quality_presets,
                self.lod.lod_config.smooth_transitions,
            )
        } else {
            QualityLevel {
                max_steps: self.settings.max_steps,
                min_distance: self.settings.min_distance,
                shadow_samples: self.settings.shadow_samples,
                shadow_step_factor: self.settings.shadow_step_factor,
                ao_step_size: self.settings.ao_step_size,
                dof_samples: self.settings.dof_samples,
                // `render_scale` is unused by the renderer today (see the
                // deleted `apply_lod_quality` comment in the prior commit);
                // pass through 1.0 to keep the field well-formed.
                render_scale: 1.0,
                // LOD is disabled — never scale 2D iterations.
                iteration_scale: 1.0,
            }
        }
    }

    /// Calculate target LOD level based on current strategy
    fn calculate_target_lod_level(&mut self, camera_pos: Vec3, delta_time: f32) -> usize {
        use crate::lod::LODStrategy;

        let target_level = match self.lod.lod_config.strategy {
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
        target_level
            .min(3)
            .max(self.lod.lod_config.min_quality_level)
    }

    /// Calculate LOD level based on distance from camera to fractal center
    fn calculate_distance_lod(&self, camera_pos: Vec3) -> usize {
        // For 3D fractals, calculate distance from camera to origin (fractal center)
        if self.settings.render_mode == RenderMode::ThreeD {
            let fractal_center = Vec3::ZERO;
            let distance = camera_pos.distance(fractal_center);

            // Determine LOD level based on distance zones
            if distance < self.lod.lod_config.distance_zones[0] {
                0 // Ultra - close up
            } else if distance < self.lod.lod_config.distance_zones[1] {
                1 // High - medium distance
            } else if distance < self.lod.lod_config.distance_zones[2] {
                2 // Medium - far
            } else {
                3 // Low - very far
            }
        } else {
            // For 2D fractals, use zoom level as distance proxy
            // Higher zoom = closer = lower LOD level number
            let zoom = self.settings.zoom_2d;
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
        if self.lod.lod_state.is_moving {
            // Camera is moving, reduce quality
            if self.lod.lod_config.aggressive_mode {
                3 // Drop to lowest quality immediately
            } else {
                2 // Drop to medium quality
            }
        } else if self.lod.lod_state.time_since_stopped < self.lod.lod_config.restore_delay {
            // Just stopped, but within restore delay
            2 // Keep at medium quality
        } else {
            // Stationary and past restore delay
            0 // Return to ultra quality
        }
    }

    /// Calculate LOD level based on performance (FPS) with hysteresis
    fn calculate_performance_lod(&mut self, delta_time: f32) -> usize {
        let current_fps = self.lod.lod_state.current_fps;
        let target_fps = self.lod.lod_config.target_fps;

        // Determine suggested level based on FPS thresholds
        // Add hysteresis: increase thresholds when improving, decrease when degrading
        let hysteresis_margin = 0.05; // 5% hysteresis band
        let current_level = self.lod.lod_state.last_performance_level;

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

        if suggested_level == self.lod.lod_state.last_performance_level {
            // Same as before, accumulate stable time
            self.lod.lod_state.fps_stable_time += delta_time;
            self.lod.lod_state.last_performance_level
        } else {
            // Different suggestion - check if we should switch
            if suggested_level > self.lod.lod_state.last_performance_level {
                // Degrading quality (FPS dropping) - switch immediately for responsiveness
                self.lod.lod_state.fps_stable_time = 0.0;
                self.lod.lod_state.last_performance_level = suggested_level;
                suggested_level
            } else {
                // Improving quality (FPS rising) - require stable time to prevent thrashing
                const STABLE_TIME_REQUIRED: f32 = 0.5; // Half second of stable FPS before upgrading
                self.lod.lod_state.fps_stable_time += delta_time;

                if self.lod.lod_state.fps_stable_time >= STABLE_TIME_REQUIRED {
                    // FPS has been stable and good for long enough, allow upgrade
                    self.lod.lod_state.fps_stable_time = 0.0;
                    self.lod.lod_state.last_performance_level = suggested_level;
                    suggested_level
                } else {
                    // Not stable enough yet, keep current level
                    self.lod.lod_state.last_performance_level
                }
            }
        }
    }

    // ARC-008: `apply_lod_quality` was removed. LOD no longer mutates
    // `FractalParams`; the effective quality is computed at uniform-build time
    // in `Uniforms::update` via `effective_quality()`.
}

#[cfg(test)]
mod tests;
