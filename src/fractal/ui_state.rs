//! Egui panel and overlay visibility state.
//!
//! `UIState` tracks which `egui` windows and collapsible sections are open
//! plus the FPS / camera-info HUD overlays, so the UI layout survives a
//! restart. It is serialized inside `Settings`. The `pub(super) fn default_*`
//! helpers provide serde defaults shared with the `Settings` struct.

use serde::{Deserialize, Serialize};

/// Tracks which egui panels/windows are open and which HUD overlays (FPS,
/// camera info) are visible. Serialized as part of `Settings` so the UI
/// layout is restored on the next launch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIState {
    pub fractal_type_open: bool,
    pub presets_open: bool,
    pub color_viz_open: bool,
    pub params_2d_open: bool,
    pub params_3d_open: bool,
    pub ray_marching_open: bool,
    pub camera_open: bool,
    pub shading_open: bool,
    pub lighting_open: bool,
    pub effects_open: bool,
    pub floor_open: bool,
    pub lod_open: bool,
    pub settings_open: bool,
    pub controls_open: bool,
    pub capture_window_open: bool,
    pub about_window_open: bool,
    pub show_fps: bool,
    pub show_camera_info: bool,
    /// ENH-006: GPU profile HUD overlay (per-scope GPU timings + CPU frame
    /// ms). `#[serde(default)]` so older `settings.yaml`/presets that predate
    /// the field still deserialize.
    #[serde(default)]
    pub show_gpu_profile: bool,
    /// Issue #1: live Julia-under-cursor preview inset (Mandelbrot2D).
    /// Defaulted (enabled) so older settings without the key load with the
    /// feature on — it is discoverable-by-default and only affects
    /// Mandelbrot2D views.
    #[serde(default = "default_julia_preview_enabled")]
    pub julia_preview_enabled: bool,
}

pub(super) fn default_dof_samples() -> u32 {
    2
}

pub(super) fn default_shadow_mode() -> u32 {
    2 // Soft shadows
}

pub(super) fn default_one() -> f32 {
    1.0
}

pub(super) fn default_reflection_strength() -> f32 {
    0.5
}

pub(super) fn default_light_azimuth() -> f32 {
    45.0 // degrees
}

pub(super) fn default_light_elevation() -> f32 {
    35.0 // degrees — ARC-026: must match RenderSettings::default (state.rs)
}

pub(super) fn default_vignette_intensity() -> f32 {
    0.5
}

pub(super) fn default_vignette_radius() -> f32 {
    0.8
}

pub(super) fn default_bloom_threshold() -> f32 {
    0.75
}

pub(super) fn default_bloom_intensity() -> f32 {
    0.1
}

pub(super) fn default_bloom_radius() -> f32 {
    0.005
}

pub(super) fn default_custom_width() -> String {
    "1920".to_string()
}

pub(super) fn default_custom_height() -> String {
    "1080".to_string()
}

pub(super) fn default_orbit_speed() -> f32 {
    0.2
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            fractal_type_open: true,
            presets_open: false,
            color_viz_open: true,
            params_2d_open: true,
            params_3d_open: true,
            ray_marching_open: false,
            camera_open: false,
            shading_open: true,
            lighting_open: true,
            effects_open: false,
            floor_open: false,
            lod_open: false,
            settings_open: false,
            controls_open: false,
            capture_window_open: false,
            about_window_open: false,
            show_fps: false,
            show_camera_info: false,
            show_gpu_profile: false,
            julia_preview_enabled: true,
        }
    }
}

pub(super) fn default_julia_preview_enabled() -> bool {
    true
}
