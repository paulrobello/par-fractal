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
use crate::renderer::{GpuInfo, Renderer};
use crate::ui::UI;
use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::window::Window;

#[cfg(feature = "native")]
use crate::video_recorder::{VideoFormat, VideoRecorder};

/// Top-level application state and event-loop owner.
///
/// Holds the window, renderer, cameras, fractal parameters, egui UI, and (on
/// native targets) the video recorder. The per-frame work is split across the
/// private submodules `input`, `update`, `render`, `capture`/`capture_web`,
/// `persistence`, and `camera_transition`; `App` is the shared state those
/// modules operate on. Construct with `new` (native) or `new_async` (web).
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
    /// ARC-005: tracks whether the bloom output texture currently holds defined
    /// (cleared-to-black) contents. The composite pass samples `bloom_view`
    /// unconditionally, so when bloom is disabled we must record one cheap clear
    /// of the bloom target (`LoadOp::Clear(BLACK)` + no draw) to avoid sampling
    /// stale/garbage memory after an enabled→disabled transition. Reset to
    /// `false` whenever the bloom passes run, since they overwrite the texture.
    bloom_texture_cleared: bool,
    /// ARC-006: dirty flag for render-on-demand. Set true by every image-
    /// affecting state change (input handlers, UI actions, LOD transitions,
    /// camera moves, palette animation, etc.). Cleared at the end of `render()`
    /// when no continuous animation source is active. The event loop consults
    /// `should_render_next_frame()` in `AboutToWait` to decide whether to
    /// request another redraw; when clean and idle the native loop sets
    /// `ControlFlow::Wait` and the app sleeps until the next OS event.
    /// Progressive refinement while idle is ENH-002, NOT this flag.
    scene_dirty: bool,
    /// ARC-018: in-flight background GPU enumeration. `Some` while a worker
    /// thread is scanning adapters; the result is drained from the receiver
    /// in `App::update` each frame (no blocking). Native only — on wasm,
    /// `enumerate_gpus` returns an empty `Vec`, so the receiver stays `None`
    /// and the UI's "Scanning…" label is set/cleared synchronously.
    #[cfg(not(target_arch = "wasm32"))]
    gpu_scan_receiver: Option<std::sync::mpsc::Receiver<Vec<GpuInfo>>>,
}

impl App {
    /// Create a new App instance (native version)
    #[cfg(feature = "native")]
    pub async fn new(
        window: Window,
        screenshot_delay: Option<f32>,
        exit_delay: Option<f32>,
        preset_name: Option<String>,
        quality_level: Option<usize>,
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
            // First try built-in presets
            if let Some(preset_data) = crate::fractal::PresetGallery::get_builtin_preset(&preset) {
                println!("Loaded built-in preset: {}", preset);
                FractalParams::from_settings(preset_data.settings.clone())
            } else {
                // Try to load user preset from file
                match crate::fractal::PresetGallery::load_preset(&preset) {
                    Ok(preset_data) => {
                        println!("Loaded user preset: {}", preset);
                        FractalParams::from_settings(preset_data.settings)
                    }
                    Err(e) => {
                        eprintln!("Failed to load preset '{}': {}", preset, e);
                        eprintln!("Falling back to saved settings or defaults");
                        FractalParams::load_from_file().unwrap_or_default()
                    }
                }
            }
        } else {
            FractalParams::load_from_file().unwrap_or_default()
        };

        // Apply quality level from CLI if specified
        let mut fractal_params = fractal_params;
        if let Some(level) = quality_level {
            let level = level.min(3); // Clamp to valid range 0-3
            let quality_name = match level {
                0 => "Ultra",
                1 => "High",
                2 => "Medium",
                _ => "Low",
            };
            println!("Setting quality level: {}", quality_name);

            // Enable LOD system and set to use the specified quality level
            fractal_params.lod_config.enabled = true;
            fractal_params.lod_state.current_level = level;
            fractal_params.lod_state.target_level = level;
            fractal_params.lod_state.transition_progress = 1.0;
            fractal_params.lod_state.active_quality =
                fractal_params.lod_config.quality_presets[level];

            // Set min_quality_level so LOD won't drop below the requested level
            fractal_params.lod_config.min_quality_level = level;

            // Apply the quality settings to fractal params immediately
            fractal_params.max_steps = fractal_params.lod_state.active_quality.max_steps;
            fractal_params.min_distance = fractal_params.lod_state.active_quality.min_distance;
        }

        let mut camera = Camera::new(size.width, size.height);
        camera.fovy = fractal_params.camera_fov;
        let mut camera_controller = CameraController::new(fractal_params.camera_speed);

        // Load camera position and UI state from settings if available
        let mut ui = UI::new();
        if let Ok(content) = std::fs::read_to_string(
            directories::ProjectDirs::from("com", "fractal", "par-fractal")
                .map(|dirs| dirs.config_dir().join("settings.yaml"))
                .unwrap_or_else(|| std::path::PathBuf::from("settings.yaml")),
        ) && let Ok(settings) = serde_yaml::from_str::<crate::fractal::Settings>(&content)
        {
            camera.position = glam::Vec3::from_array(settings.camera_position);
            camera.target = glam::Vec3::from_array(settings.camera_target);
            // Update controller's yaw/pitch to match the loaded camera direction
            camera_controller.point_at_target(camera.position, camera.target);
            ui.load_ui_state(settings.ui_state);
            ui.auto_open_captures = settings.auto_open_captures;
            ui.custom_width = settings.custom_width;
            ui.custom_height = settings.custom_height;
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
            bloom_texture_cleared: false,
            scene_dirty: true,
            gpu_scan_receiver: None,
        }
    }

    /// Create a new App instance (web version with error handling)
    #[cfg(target_arch = "wasm32")]
    pub async fn new_async(
        window: Window,
        screenshot_delay: Option<f32>,
        exit_delay: Option<f32>,
        preset_name: Option<String>,
        quality_level: Option<usize>,
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

        // Apply quality level from URL parameter if specified
        let mut fractal_params = fractal_params;
        if let Some(level) = quality_level {
            let level = level.min(3); // Clamp to valid range 0-3
            let quality_name = match level {
                0 => "Ultra",
                1 => "High",
                2 => "Medium",
                _ => "Low",
            };
            log::info!("Setting quality level: {}", quality_name);

            // Enable LOD system and set to use the specified quality level
            fractal_params.lod_config.enabled = true;
            fractal_params.lod_state.current_level = level;
            fractal_params.lod_state.target_level = level;
            fractal_params.lod_state.transition_progress = 1.0;
            fractal_params.lod_state.active_quality =
                fractal_params.lod_config.quality_presets[level];

            // Set min_quality_level so LOD won't drop below the requested level
            fractal_params.lod_config.min_quality_level = level;

            // Apply the quality settings to fractal params immediately
            fractal_params.max_steps = fractal_params.lod_state.active_quality.max_steps;
            fractal_params.min_distance = fractal_params.lod_state.active_quality.min_distance;
        }

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
            bloom_texture_cleared: false,
            scene_dirty: true,
        })
    }

    /// Borrow the application's window.
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Whether the app has requested to exit (e.g. after a CLI `--exit-delay`).
    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    /// ARC-006: mark the scene as needing a re-render. Called from every
    /// image-affecting state change (input handlers, UI actions, camera moves,
    /// palette animation, etc.). Cheap (one bool write); the gating happens in
    /// `should_render_next_frame`.
    pub fn mark_scene_dirty(&mut self) {
        self.scene_dirty = true;
    }

    /// ARC-006: true when something is animating the scene continuously,
    /// independent of user input. Each of these would re-render every frame
    /// even without the dirty flag, so we OR them in at the redraw decision.
    /// The `time`-uniform path is gated here so a static palette doesn't keep
    /// the loop spinning: only `palette_animation_enabled` (advances
    /// `palette_offset`) or a non-`None` `procedural_palette` (uses `time`)
    /// actually change the image.
    pub fn is_scene_animation_active(&self) -> bool {
        use crate::fractal::{ProceduralPalette, RenderMode};

        // 3D auto-orbit rotates the camera each frame.
        if self.fractal_params.auto_orbit && self.fractal_params.render_mode == RenderMode::ThreeD {
            return true;
        }
        // Smooth camera transition (bookmark load) is interpolating.
        if self.camera_transition.active {
            return true;
        }
        // Palette offset advances each frame when the user enables animation.
        if self.ui.palette_animation_enabled {
            return true;
        }
        // Procedural palette samples the `time` uniform, which advances.
        if self.fractal_params.procedural_palette != ProceduralPalette::None {
            return true;
        }
        // LOD is smoothly interpolating between quality presets.
        if self.fractal_params.lod_config.enabled
            && self.fractal_params.lod_state.transition_progress < 1.0
        {
            return true;
        }
        // Attractor / Buddhabrot still accumulating samples.
        if self.fractal_params.attractor_accumulation_enabled
            && !self.fractal_params.attractor_paused
            && (self.fractal_params.fractal_type.is_2d_attractor()
                || self.fractal_params.fractal_type.is_buddhabrot())
        {
            return true;
        }
        // Video capture needs a fresh frame each render.
        #[cfg(not(target_arch = "wasm32"))]
        if self.video_recorder.is_recording() {
            return true;
        }
        false
    }

    /// ARC-006: whether egui wants another frame (UI animation, hover, drag,
    /// etc.). Without this OR-term, egui panels freeze while the fractal is
    /// idle — the user moves a slider and nothing repaints until they nudge
    /// the fractal.
    pub fn ui_needs_repaint(&self) -> bool {
        self.egui_state.egui_ctx().has_requested_repaint()
    }

    /// ARC-006: the redraw decision consulted by the event loop in
    /// `AboutToWait`. True when the scene changed, something is animating, or
    /// egui requested a repaint. Also forces a frame while a screenshot /
    /// hi-res render / exit is pending so the request doesn't get stuck.
    pub fn should_render_next_frame(&self) -> bool {
        if self.scene_dirty || self.is_scene_animation_active() || self.ui_needs_repaint() {
            return true;
        }
        // One-shot events that need a frame to fire even when nothing else changed.
        if self.save_screenshot || self.save_hires_render.is_some() {
            return true;
        }
        false
    }

    /// ARC-006: hook called from `App::render` after the fractal pass + UI
    /// submit. Clears the dirty flag when no continuous animation source is
    /// active (animation sources keep the flag set so the next frame also
    /// renders). Called by `render.rs` at the end of `App::render`.
    pub fn after_render_frame(&mut self) {
        if !self.is_scene_animation_active() {
            self.scene_dirty = false;
        }
    }

    /// ARC-018: drain the background GPU-enumeration result if it has landed.
    /// Called from `App::update` every frame so the UI list populates as soon
    /// as the worker thread finishes, without ever blocking the render loop.
    /// Native only — on wasm, `enumerate_gpus` is synchronous and returns an
    /// empty `Vec`, so the receiver is never set and this is a no-op stub.
    #[cfg(not(target_arch = "wasm32"))]
    fn poll_gpu_scan(&mut self) {
        let Some(rx) = self.gpu_scan_receiver.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(gpus) => {
                self.ui.available_gpus = gpus;
                self.ui.gpu_selection_message =
                    Some(format!("Found {} GPU(s)", self.ui.available_gpus.len()));
                self.gpu_scan_receiver = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                // Still scanning — the "Scanning…" label set on click stays.
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // Worker thread died before sending (panic in enumerate_gpus).
                // Reset so the user can click "Detect" again.
                self.ui.gpu_selection_message =
                    Some("GPU scan failed — click Detect to retry.".to_string());
                self.gpu_scan_receiver = None;
            }
        }
    }

    /// ARC-018: wasm stub — GPU enumeration isn't available on web (the browser
    /// handles adapter selection), so there's never a receiver to poll.
    #[cfg(target_arch = "wasm32")]
    fn poll_gpu_scan(&mut self) {}

    /// Handle a window resize: reconfigure the renderer surface, update the
    /// camera aspect ratio, and (on native) persist the new window size.
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.renderer.resize(new_size);
        self.camera.resize(new_size.width, new_size.height);

        // ARC-006: a fresh surface + recreated intermediate textures means the
        // next composite samples cleared-to-black bloom output; flag dirty so
        // one redraw fires after the resize event chain settles, and reset the
        // bloom-cleared flag so the post-resize frame re-establishes defined
        // bloom contents if bloom is enabled.
        self.mark_scene_dirty();
        self.bloom_texture_cleared = false;

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
