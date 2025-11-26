// Module declarations
mod camera_transition;
mod input;
mod render;
mod update;

#[cfg(feature = "native")]
mod capture;
#[cfg(target_arch = "wasm32")]
mod capture_web;
#[cfg(feature = "native")]
mod persistence;

use camera_transition::CameraTransition;

use crate::camera::{Camera, CameraController};
use crate::fractal::{FractalParams, RenderMode};
use crate::renderer::Renderer;
use crate::ui::UI;
use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::window::Window;

#[cfg(feature = "native")]
use crate::video_recorder::{VideoFormat, VideoRecorder};

pub struct App {
    window: Arc<Window>,
    renderer: Renderer,
    camera: Camera,
    camera_controller: CameraController,
    fractal_params: FractalParams,
    ui: UI,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    last_frame_time: web_time::Instant,
    mouse_pressed: bool,
    last_mouse_pos: Option<(f32, f32)>,
    cursor_pos: (f32, f32), // Current cursor position for zoom-at-cursor
    shift_pressed: bool,    // Track shift key for continuous zoom
    // Multi-touch pinch-to-zoom tracking
    active_touches: std::collections::HashMap<u64, (f32, f32)>, // touch_id -> (x, y)
    initial_pinch_distance: Option<f32>, // Distance between two fingers at pinch start
    last_touch_time: Option<web_time::Instant>, // Time of last touch start (for phantom detection)
    frame_count: u32,
    fps_timer: web_time::Instant,
    current_fps: f32,
    save_screenshot: bool,
    save_hires_render: Option<(u32, u32)>, // Optional (width, height) for high-res render
    camera_last_moved: web_time::Instant,
    camera_needs_save: bool,
    settings_last_changed: web_time::Instant,
    settings_need_save: bool,
    was_auto_orbiting: bool, // Track if we were auto-orbiting in previous frame
    start_time: web_time::Instant, // Track elapsed time for palette animation
    camera_transition: CameraTransition,
    smooth_transitions_enabled: bool,
    #[cfg(feature = "native")]
    video_recorder: VideoRecorder,
    screenshot_delay: Option<f32>, // CLI option: take screenshot after N seconds
    exit_delay: Option<f32>,       // CLI option: exit after N seconds
    screenshot_taken: bool,        // Track if delayed screenshot was taken
    should_exit: bool,             // Track if app should exit
}

impl App {
    /// Create a new App instance (native version)
    #[cfg(feature = "native")]
    pub async fn new(
        window: Window,
        screenshot_delay: Option<f32>,
        exit_delay: Option<f32>,
        preset_name: Option<String>,
    ) -> Self {
        let window = Arc::new(window);
        let size = window.inner_size();

        // Load GPU preferences
        let prefs = crate::fractal::AppPreferences::load();
        let renderer = if let Some(gpu_index) = prefs.preferred_gpu_index {
            println!("Using preferred GPU index: {}", gpu_index);
            Renderer::new_with_gpu_preference(window.clone(), size, Some(gpu_index)).await
        } else {
            Renderer::new(window.clone(), size).await
        };

        // Load fractal params from preset if specified, otherwise from saved settings
        let fractal_params = if let Some(preset) = preset_name {
            // Try to load the specified preset
            match crate::fractal::PresetGallery::load_preset(&preset) {
                Ok(preset_data) => {
                    println!("Loaded preset: {}", preset);
                    FractalParams::from_settings(preset_data.settings)
                }
                Err(e) => {
                    eprintln!("Failed to load preset '{}': {}", preset, e);
                    eprintln!("Falling back to saved settings or defaults");
                    FractalParams::load_from_file().unwrap_or_default()
                }
            }
        } else {
            FractalParams::load_from_file().unwrap_or_default()
        };

        let mut camera = Camera::new(size.width, size.height);
        camera.fovy = fractal_params.camera_fov;
        let mut camera_controller = CameraController::new(fractal_params.camera_speed);

        // Load camera position and UI state from settings if available
        let mut ui = UI::new();
        if let Ok(content) = std::fs::read_to_string(
            directories::ProjectDirs::from("com", "fractal", "par-fractal")
                .map(|dirs| dirs.config_dir().join("settings.yaml"))
                .unwrap_or_else(|| std::path::PathBuf::from("settings.yaml")),
        ) {
            if let Ok(settings) = serde_yaml::from_str::<crate::fractal::Settings>(&content) {
                camera.position = glam::Vec3::from_array(settings.camera_position);
                camera.target = glam::Vec3::from_array(settings.camera_target);
                // Update controller's yaw/pitch to match the loaded camera direction
                camera_controller.point_at_target(camera.position, camera.target);
                ui.load_ui_state(settings.ui_state);
                ui.auto_open_captures = settings.auto_open_captures;
                ui.custom_width = settings.custom_width;
                ui.custom_height = settings.custom_height;
            }
        }

        let egui_ctx = egui::Context::default();
        let egui_state =
            egui_winit::State::new(egui_ctx, egui::ViewportId::ROOT, &window, None, None, None);

        let egui_renderer = egui_wgpu::Renderer::new(
            &renderer.device,
            renderer.config.format,
            egui_wgpu::RendererOptions {
                msaa_samples: 1,
                ..Default::default()
            },
        );

        let video_recorder = VideoRecorder::new(size.width, size.height, 60, VideoFormat::MP4);

        Self {
            window,
            renderer,
            camera,
            camera_controller,
            fractal_params,
            ui,
            egui_state,
            egui_renderer,
            last_frame_time: web_time::Instant::now(),
            mouse_pressed: false,
            last_mouse_pos: None,
            cursor_pos: (0.0, 0.0),
            shift_pressed: false,
            active_touches: std::collections::HashMap::new(),
            initial_pinch_distance: None,
            last_touch_time: None,
            frame_count: 0,
            fps_timer: web_time::Instant::now(),
            current_fps: 0.0,
            save_screenshot: false,
            save_hires_render: None,
            camera_last_moved: web_time::Instant::now(),
            camera_needs_save: false,
            settings_last_changed: web_time::Instant::now(),
            settings_need_save: false,
            was_auto_orbiting: false,
            start_time: web_time::Instant::now(),
            camera_transition: CameraTransition::new(),
            smooth_transitions_enabled: true,
            video_recorder,
            screenshot_delay,
            exit_delay,
            screenshot_taken: false,
            should_exit: false,
        }
    }

    /// Create a new App instance (web version with error handling)
    #[cfg(target_arch = "wasm32")]
    pub async fn new_async(
        window: Window,
        screenshot_delay: Option<f32>,
        exit_delay: Option<f32>,
        preset_name: Option<String>,
    ) -> Result<Self, String> {
        let window = Arc::new(window);
        let mut size = window.inner_size();

        // Ensure we have valid dimensions (fallback for web where size might be 0x0 initially)
        if size.width == 0 || size.height == 0 {
            log::warn!(
                "Window size is {}x{}, using fallback 800x600",
                size.width,
                size.height
            );
            size = winit::dpi::PhysicalSize::new(800, 600);
        }

        log::info!(
            "Initializing renderer with size {}x{}",
            size.width,
            size.height
        );

        // Create renderer (no GPU preference on web - browser handles this)
        let renderer = Renderer::new(window.clone(), size).await;

        // Use default fractal params for web (no persistent storage yet)
        // TODO: Load from localStorage via platform abstraction
        let fractal_params = if let Some(preset) = preset_name {
            match crate::fractal::PresetGallery::get_builtin_preset(&preset) {
                Some(preset_data) => {
                    log::info!("Loaded preset: {}", preset);
                    FractalParams::from_settings(preset_data.settings.clone())
                }
                None => {
                    log::warn!("Preset '{}' not found, using defaults", preset);
                    FractalParams::default()
                }
            }
        } else {
            FractalParams::default()
        };

        let mut camera = Camera::new(size.width, size.height);
        camera.fovy = fractal_params.camera_fov;
        let camera_controller = CameraController::new(fractal_params.camera_speed);

        let ui = UI::new();

        let egui_ctx = egui::Context::default();
        let egui_state =
            egui_winit::State::new(egui_ctx, egui::ViewportId::ROOT, &window, None, None, None);

        let egui_renderer = egui_wgpu::Renderer::new(
            &renderer.device,
            renderer.config.format,
            egui_wgpu::RendererOptions {
                msaa_samples: 1,
                ..Default::default()
            },
        );

        Ok(Self {
            window,
            renderer,
            camera,
            camera_controller,
            fractal_params,
            ui,
            egui_state,
            egui_renderer,
            last_frame_time: web_time::Instant::now(),
            mouse_pressed: false,
            last_mouse_pos: None,
            cursor_pos: (0.0, 0.0),
            shift_pressed: false,
            active_touches: std::collections::HashMap::new(),
            initial_pinch_distance: None,
            last_touch_time: None,
            frame_count: 0,
            fps_timer: web_time::Instant::now(),
            current_fps: 0.0,
            save_screenshot: false,
            save_hires_render: None,
            camera_last_moved: web_time::Instant::now(),
            camera_needs_save: false,
            settings_last_changed: web_time::Instant::now(),
            settings_need_save: false,
            was_auto_orbiting: false,
            start_time: web_time::Instant::now(),
            camera_transition: CameraTransition::new(),
            smooth_transitions_enabled: true,
            screenshot_delay,
            exit_delay,
            screenshot_taken: false,
            should_exit: false,
        })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.renderer.size
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.renderer.resize(new_size);
        self.camera.resize(new_size.width, new_size.height);

        // Persist window size (native only)
        #[cfg(feature = "native")]
        if new_size.width > 0 && new_size.height > 0 {
            let mut prefs = crate::fractal::AppPreferences::load();
            prefs.set_window_size(new_size.width, new_size.height);
            if let Err(e) = prefs.save() {
                eprintln!("Failed to save window size: {}", e);
            }
        }
    }

    fn reset_view(&mut self) {
        match self.fractal_params.render_mode {
            RenderMode::TwoD => {
                // Re-apply fractal defaults (this sets the correct center and zoom for each fractal type)
                let current_type = self.fractal_params.fractal_type;
                self.fractal_params.switch_fractal(current_type);

                // Clear accumulation for strange attractors and sync tracking values
                if self.fractal_params.attractor_accumulation_enabled {
                    self.fractal_params.attractor_pending_clear = true;
                    self.fractal_params.attractor_total_iterations = 0;
                    // Sync tracking to the reset values
                    self.fractal_params.attractor_last_center = self.fractal_params.center_2d;
                    self.fractal_params.attractor_last_zoom = self.fractal_params.zoom_2d;
                    self.fractal_params.attractor_last_julia_c = self.fractal_params.julia_c;
                }
            }
            RenderMode::ThreeD => {
                let size = self.renderer.size;
                self.camera = Camera::new(size.width, size.height);
                self.camera.fovy = self.fractal_params.camera_fov;
                self.camera_controller = CameraController::new(self.fractal_params.camera_speed);
            }
        }
    }
}
