// Module declarations
mod command;
mod history;
mod monitor;
mod overlays;
mod panels;
mod toast;
mod toast_ui;

// Re-exports
pub use monitor::MonitorInfo;
pub use toast::Toast;

use crate::command_palette::CommandPalette;
use crate::fractal::{
    BookmarkGallery, CameraBookmark, CustomPaletteGallery, FractalParams, Preset, PresetCategory,
    PresetGallery, RenderSettings, UIState,
};
use egui::Context;
use glam::Vec3;

use history::HistoryEntry;

use crate::fractal::ProceduralPalette;

/// Outputs from `UI::render` consumed by the app layer (ARC-003).
///
/// Replaces an 11-element tuple so consumers destructure by name — prevents
/// the "two adjacent bools swapped" class of bug. Field order matches the
/// historical tuple positional order; names are derived from how the
/// call site in `app/render.rs` used each position.
#[derive(Default)]
pub struct UiActions {
    /// Any fractal parameter changed this frame (triggers settings save).
    pub changed: bool,
    /// User clicked the screenshot button.
    pub screenshot_requested: bool,
    /// User requested a full parameter reset to defaults.
    pub reset_requested: bool,
    /// User requested a camera-only reset.
    pub reset_camera_requested: bool,
    /// User clicked "point camera at fractal center".
    pub point_at_fractal_requested: bool,
    /// User selected a preset to load from the gallery.
    pub preset_to_load: Option<Preset>,
    /// User requested a high-resolution render at this resolution.
    pub hires_render_resolution: Option<(u32, u32)>,
    /// User selected a camera bookmark to apply.
    pub bookmark_to_load: Option<CameraBookmark>,
    /// User clicked the GPU rescan button.
    pub gpu_scan_requested: bool,
    /// User clicked the video-record start button.
    pub start_recording: bool,
    /// User clicked the video-record stop button.
    pub stop_recording: bool,
}

/// Generate a preview color for procedural palettes (CPU-side approximation of shader code)
fn get_procedural_preview_color(
    palette_type: ProceduralPalette,
    t: f32,
    brightness: &[f32; 3],
    contrast: &[f32; 3],
    frequency: &[f32; 3],
    phase: &[f32; 3],
) -> [f32; 3] {
    const TWO_PI: f32 = std::f32::consts::PI * 2.0;

    // Generic cosine palette formula
    let cosine_palette = |t: f32, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3]| -> [f32; 3] {
        [
            (a[0] + b[0] * (TWO_PI * (c[0] * t + d[0])).cos()).clamp(0.0, 1.0),
            (a[1] + b[1] * (TWO_PI * (c[1] * t + d[1])).cos()).clamp(0.0, 1.0),
            (a[2] + b[2] * (TWO_PI * (c[2] * t + d[2])).cos()).clamp(0.0, 1.0),
        ]
    };

    match palette_type {
        ProceduralPalette::None => [0.0, 0.0, 0.0],
        ProceduralPalette::Firestrm => {
            let angle = t * TWO_PI;
            [
                (angle.cos() + 1.0) * 0.5,
                ((angle + TWO_PI / 3.0).cos() + 1.0) * 0.5,
                ((angle + TWO_PI * 2.0 / 3.0).cos() + 1.0) * 0.5,
            ]
        }
        ProceduralPalette::Rainbow => {
            // HSV hue rotation (red -> yellow -> green -> cyan -> blue -> magenta -> red)
            let h = t;
            let c = 1.0_f32; // s * v where s=1, v=1
            let x = c * (1.0 - ((h * 6.0).fract() * 2.0 - 1.0).abs());
            let m = 0.0_f32; // v - c where v=1, c=1
            let h6 = h * 6.0;
            let (r, g, b) = if h6 < 1.0 {
                (c, x, 0.0)
            } else if h6 < 2.0 {
                (x, c, 0.0)
            } else if h6 < 3.0 {
                (0.0, c, x)
            } else if h6 < 4.0 {
                (0.0, x, c)
            } else if h6 < 5.0 {
                (x, 0.0, c)
            } else {
                (c, 0.0, x)
            };
            [r + m, g + m, b + m]
        }
        ProceduralPalette::Electric => cosine_palette(
            t,
            [0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [1.0, 1.0, 1.0],
            [0.5, 0.6, 0.7],
        ),
        ProceduralPalette::Sunset => cosine_palette(
            t,
            [0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [1.0, 1.0, 1.0],
            [0.0, 0.1, 0.2],
        ),
        ProceduralPalette::Forest => cosine_palette(
            t,
            [0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [1.0, 1.0, 0.5],
            [0.3, 0.2, 0.2],
        ),
        ProceduralPalette::Ocean => cosine_palette(
            t,
            [0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [1.0, 1.0, 1.0],
            [0.6, 0.7, 0.8],
        ),
        ProceduralPalette::Grayscale => [t, t, t],
        ProceduralPalette::Hot => cosine_palette(
            t,
            [0.5, 0.5, 0.5],
            [0.5, 0.5, 0.4],
            [1.0, 1.0, 1.0],
            [0.0, 0.15, 0.4],
        ),
        ProceduralPalette::Cool => cosine_palette(
            t,
            [0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [1.0, 1.0, 1.0],
            [0.8, 0.9, 0.3],
        ),
        ProceduralPalette::Plasma => cosine_palette(
            t,
            [0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [1.0, 1.0, 1.0],
            [0.8, 0.9, 0.1],
        ),
        ProceduralPalette::Viridis => cosine_palette(
            t,
            [0.5, 0.5, 0.5],
            [0.4, 0.5, 0.4],
            [0.8, 0.8, 0.5],
            [0.7, 0.5, 0.0],
        ),
        ProceduralPalette::Custom => cosine_palette(t, *brightness, *contrast, *frequency, *phase),
    }
}

// Video format - use actual type on native, stub on web
#[cfg(not(target_arch = "wasm32"))]
use crate::video_recorder::VideoFormat;

/// Stub video format for web builds
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoFormat {
    #[default]
    MP4,
    WebM,
    GIF,
}

#[cfg(target_arch = "wasm32")]
impl VideoFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            VideoFormat::MP4 => "mp4",
            VideoFormat::WebM => "webm",
            VideoFormat::GIF => "gif",
        }
    }
}

pub struct UI {
    pub show_ui: bool,
    pub show_fps: bool,
    pub show_camera_info: bool,
    pub show_performance_overlay: bool,
    /// ENH-006: GPU profile HUD overlay visibility. Mirrored to
    /// `UIState::show_gpu_profile` for persistence.
    pub show_gpu_profile: bool,
    pub ui_state: UIState,
    // Command palette
    pub command_palette: CommandPalette,
    // Performance tracking
    frame_times: Vec<f32>, // Last N frame times in milliseconds
    max_frame_history: usize,
    /// ENH-002: mirrors `App::scene_converged` each frame so the performance
    /// overlay can report whether the fractal pass ran (Active) or was skipped
    /// because the cached full-quality frame was reused (Idle/Converged).
    pub scene_converged: bool,
    // Preset UI state
    preset_name: String,
    preset_description: String,
    preset_category: PresetCategory,
    preset_search: String,
    preset_category_filter: PresetCategory,
    user_presets: Vec<String>,
    last_preset_list_update: web_time::Instant,
    // Undo/Redo system. ARC-019: VecDeque so oldest-entry eviction is O(1)
    // (`pop_front`) instead of the previous O(n) `Vec::remove(0)`.
    history: std::collections::VecDeque<HistoryEntry>,
    history_index: usize,
    max_history_size: usize,
    /// ARC-015: history tracks `RenderSettings` snapshots only — transient
    /// LOD/accumulation state is not part of the undo contract.
    last_saved_settings: Option<RenderSettings>,
    // Camera bookmarks
    bookmark_name: String,
    /// Text buffer for the 2D precise-center "Go to" entry (ENH-001 Phase C).
    precise_center_input: String,
    bookmarks: Vec<String>,
    last_bookmark_list_update: web_time::Instant,
    bookmark_to_delete: Option<String>,
    // Custom palette editor
    custom_palette_name: String,
    custom_palette_colors: [[f32; 3]; 8],
    custom_palettes: Vec<String>,
    last_custom_palette_list_update: web_time::Instant,
    custom_palette_to_delete: Option<String>,
    palette_import_path: String,
    palette_import_message: Option<String>,
    // Theme
    pub dark_theme: bool,
    // Palette animation
    pub palette_animation_enabled: bool,
    pub palette_animation_speed: f32,
    pub palette_animation_reverse: bool,
    palette_animation_offset: f32, // Current accumulated offset
    // GPU selection
    pub available_gpus: Vec<super::renderer::GpuInfo>,
    // QA-020: retained from the GPU-picker dialog. The dialog now surfaces
    // its choice via `gpu_selection_message` and writes through to settings,
    // so this field is currently write-only; kept because the upcoming
    // "remember last-used GPU" UX (AUDIT ARC-014 follow-up) reads it.
    #[allow(dead_code)]
    pub selected_gpu_index: Option<usize>,
    pub gpu_selection_message: Option<String>,
    // Video recording
    pub video_format: VideoFormat,
    pub video_fps: u32,
    // Monitor/wallpaper support
    pub available_monitors: Vec<MonitorInfo>,
    // Toast notifications
    toasts: Vec<Toast>,
    pub selected_monitor_index: usize,
    pub last_monitor_scan: web_time::Instant,
    // Custom resolution input
    pub custom_width: String,
    pub custom_height: String,
    // Auto-open captured images
    pub auto_open_captures: bool,
}

impl UI {
    pub fn new() -> Self {
        Self {
            show_ui: cfg!(not(target_arch = "wasm32")), // Hidden by default on web for mobile testing
            show_fps: false,
            show_camera_info: false,
            show_performance_overlay: false,
            show_gpu_profile: false,
            ui_state: UIState::default(),
            command_palette: CommandPalette::new(),
            frame_times: Vec::with_capacity(120),
            max_frame_history: 120,
            scene_converged: false,
            preset_name: String::new(),
            preset_description: String::new(),
            preset_category: PresetCategory::All,
            preset_search: String::new(),
            preset_category_filter: PresetCategory::All,
            user_presets: PresetGallery::list_user_presets().unwrap_or_default(),
            last_preset_list_update: web_time::Instant::now(),
            history: std::collections::VecDeque::new(),
            history_index: 0,
            max_history_size: 50,
            last_saved_settings: None,
            bookmark_name: String::new(),
            precise_center_input: String::new(),
            bookmarks: BookmarkGallery::list_bookmarks().unwrap_or_default(),
            last_bookmark_list_update: web_time::Instant::now(),
            bookmark_to_delete: None,
            custom_palette_name: String::new(),
            custom_palette_colors: [
                [0.0, 0.0, 0.0], // Black
                [1.0, 0.0, 0.0], // Red
                [1.0, 0.5, 0.0], // Orange
                [1.0, 1.0, 0.0], // Yellow
                [0.0, 1.0, 0.0], // Green
                [0.0, 1.0, 1.0], // Cyan
                [0.0, 0.0, 1.0], // Blue
                [1.0, 0.0, 1.0], // Magenta
            ],
            custom_palettes: CustomPaletteGallery::list_palettes().unwrap_or_default(),
            last_custom_palette_list_update: web_time::Instant::now(),
            custom_palette_to_delete: None,
            palette_import_path: String::new(),
            palette_import_message: None,
            dark_theme: true,
            palette_animation_enabled: false,
            palette_animation_speed: 0.1,
            palette_animation_reverse: false,
            palette_animation_offset: 0.0,
            available_gpus: Vec::new(),
            selected_gpu_index: None,
            gpu_selection_message: None,
            video_format: VideoFormat::MP4,
            video_fps: 60,
            available_monitors: Vec::new(),
            toasts: Vec::new(),
            selected_monitor_index: 0,
            last_monitor_scan: web_time::Instant::now(),
            custom_width: String::from("1920"),
            custom_height: String::from("1080"),
            auto_open_captures: false,
        }
    }

    /// Scan for available monitors and populate the list
    pub fn scan_monitors(&mut self, window: &winit::window::Window) {
        log::debug!("Scanning for monitors...");
        self.available_monitors.clear();

        // Get primary monitor
        let primary_monitor = window.primary_monitor();
        log::debug!(
            "Primary monitor: {:?}",
            primary_monitor.as_ref().and_then(|m| m.name())
        );

        // Get all available monitors
        let mut count = 0;
        for (index, monitor) in window.available_monitors().enumerate() {
            count += 1;
            log::debug!("Found monitor {}: {:?}", index, monitor.name());
            let is_primary = if let Some(ref primary) = primary_monitor {
                monitor::monitors_equal(&monitor, primary)
            } else {
                index == 0 // Fallback: treat first as primary if we can't determine
            };

            // Get monitor name
            let name = monitor
                .name()
                .unwrap_or_else(|| format!("Monitor {}", index + 1));

            // Prefer the monitor's reported current resolution. If unavailable (e.g., web),
            // fall back to the largest advertised video mode to avoid tiny default modes.
            let size = monitor.size();
            let (width, height) = if size.width > 0 && size.height > 0 {
                (size.width, size.height)
            } else {
                monitor
                    .video_modes()
                    .max_by_key(|mode| {
                        let s = mode.size();
                        (s.width as u64) * (s.height as u64)
                    })
                    .map(|mode| {
                        let s = mode.size();
                        (s.width, s.height)
                    })
                    .unwrap_or((0, 0))
            };

            self.available_monitors.push(MonitorInfo {
                name,
                width,
                height,
                is_primary,
            });
        }

        // Sort so primary is first
        self.available_monitors
            .sort_by(|a, b| match (b.is_primary, a.is_primary) {
                (true, false) => std::cmp::Ordering::Greater,
                (false, true) => std::cmp::Ordering::Less,
                _ => std::cmp::Ordering::Equal,
            });

        log::debug!("Total monitors found: {}", count);
        log::debug!("Monitors in list: {}", self.available_monitors.len());

        // Update scan time
        self.last_monitor_scan = web_time::Instant::now();

        log::info!("Scanned {} monitor(s)", self.available_monitors.len());
    }

    pub fn load_ui_state(&mut self, ui_state: UIState) {
        self.show_fps = ui_state.show_fps;
        self.show_camera_info = ui_state.show_camera_info;
        self.show_gpu_profile = ui_state.show_gpu_profile;
        self.ui_state = ui_state;
    }

    pub fn get_ui_state(&self) -> &UIState {
        &self.ui_state
    }

    pub fn render(
        &mut self,
        ctx: &Context,
        params: &mut FractalParams,
        camera_pos: Vec3,
        camera_target: Vec3,
        is_recording: bool,
    ) -> UiActions {
        // Apply theme
        if self.dark_theme {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        if !self.show_ui {
            // Show a small floating button to restore the UI
            egui::Window::new("show_ui_toggle")
                .title_bar(false)
                .resizable(false)
                .fixed_pos([10.0, 10.0])
                .show(ctx, |ui| {
                    if ui
                        .button("☰ Show UI")
                        .on_hover_text("Show the control panel [H]")
                        .clicked()
                    {
                        self.show_ui = true;
                    }
                });
            return UiActions::default();
        }

        // QA-009: per-panel actions accumulator (replaces 11 local flags).
        // Panels write to these fields via `&mut UiActions`; the value is
        // returned at the end of the frame. `randomize_requested` and
        // `from_history` stay as locals — they are render-internal and do
        // not appear in the `UiActions` API contract.
        let mut actions = UiActions::default();
        let mut randomize_requested = false;
        let mut from_history = false; // Don't save to history if change came from undo/redo

        egui::Window::new("Fractal Controls")
            .default_width(320.0)
            .default_height(600.0)
            .resizable(true)
            .vscroll(true)
            .show(ctx, |ui| {
                // Quick Actions at the top
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(self.can_undo(), egui::Button::new("↶ Undo"))
                        .on_hover_text("Undo last parameter change (Ctrl+Z)")
                        .clicked()
                        && let Some(prev_settings) = self.undo()
                    {
                        // ARC-015: only authored RenderSettings rolls back;
                        // params.lod (FPS deque) and params.accum (counters,
                        // pending-clear) are left untouched on undo/redo.
                        params.settings = prev_settings;
                        actions.changed = true;
                        from_history = true; // Don't save to history
                    }
                    if ui
                        .add_enabled(self.can_redo(), egui::Button::new("↷ Redo"))
                        .on_hover_text("Redo parameter change (Ctrl+Y)")
                        .clicked()
                        && let Some(next_settings) = self.redo()
                    {
                        params.settings = next_settings;
                        actions.changed = true;
                        from_history = true; // Don't save to history
                    }
                    if ui
                        .button("🎲 Randomize")
                        .on_hover_text("Generate random fractal settings for creative exploration")
                        .clicked()
                    {
                        randomize_requested = true;
                    }
                });

                // UI Control Actions
                ui.horizontal(|ui| {
                    if ui
                        .button("⏏ Hide UI")
                        .on_hover_text("Hide the control panel [H]")
                        .clicked()
                    {
                        self.show_ui = false;
                    }

                    let theme_icon = if self.dark_theme { "☀" } else { "🌙" };
                    let theme_text = format!("{} Theme", theme_icon);
                    if ui
                        .button(theme_text)
                        .on_hover_text("Toggle between dark and light themes")
                        .clicked()
                    {
                        self.dark_theme = !self.dark_theme;
                    }

                    if ui
                        .button("📷 Capture")
                        .on_hover_text("Open capture & recording panel")
                        .clicked()
                    {
                        self.ui_state.capture_window_open = !self.ui_state.capture_window_open;
                    }

                    if ui
                        .button("ℹ About")
                        .on_hover_text("About Par Fractal")
                        .clicked()
                    {
                        self.ui_state.about_window_open = !self.ui_state.about_window_open;
                    }
                });
                ui.separator();

                self.render_fractal_type_panel(ui, params, &mut actions);

                // Preset management section
                self.render_presets_panel(ui, params, &mut actions, camera_pos, camera_target);

                self.render_color_viz_panel(ui, params, &mut actions);

                match params.settings.render_mode {
                    crate::fractal::RenderMode::TwoD => {
                        self.render_params_2d_panel(ui, params, &mut actions);
                    }
                    crate::fractal::RenderMode::ThreeD => {
                        self.render_params_3d_panel(ui, params, &mut actions);

                        self.render_ray_marching_panel(ui, params, &mut actions);

                        self.render_camera_panel(
                            ui,
                            params,
                            &mut actions,
                            camera_pos,
                            camera_target,
                        );

                        self.render_shading_panel(ui, params, &mut actions);

                        self.render_lighting_panel(ui, params, &mut actions);

                        self.render_effects_panel(ui, params, &mut actions);

                        self.render_floor_panel(ui, params, &mut actions);

                        // LOD System
                        self.render_lod_panel(ui, params, &mut actions);
                    }
                }

                self.render_settings_panel(ui, params, &mut actions);

                self.render_controls_panel(ui, params, &mut actions);
            });

        // Handle randomization request
        if randomize_requested {
            self.save_to_history(params);
            params.randomize();
            actions.changed = true;
        }

        // Save to history when parameters change (but not if change came from undo/redo)
        if actions.changed && !from_history {
            self.save_to_history(params);
        }

        // Capture & Recording Window
        self.render_capture_window(ctx, &mut actions, is_recording);
        // About Window
        self.render_about_window(ctx);

        // Render toast notifications
        self.render_toasts(ctx);
        actions
    }
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
