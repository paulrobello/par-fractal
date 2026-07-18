// Module declarations
mod camera_transition;
mod input;
mod render;
mod update;

// Shared GPU-readback helpers for both capture paths (ARC-014 dedup).
#[cfg(feature = "native")]
mod capture;
mod capture_common;
#[cfg(target_arch = "wasm32")]
mod capture_web;
#[cfg(feature = "native")]
mod persistence;

use camera_transition::CameraTransition;

use crate::camera::{Camera, CameraController};
use crate::deep_zoom::PerturbationDriver;
use crate::fractal::{FractalParams, RenderMode};
use crate::platform::{PlatformContext, category};
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
    /// ENH-007: CLI override for the screenshot output path. When `Some`, the
    /// delayed screenshot is written exactly here (no timestamp/auto-open) so
    /// the visual-regression harness gets a deterministic, predictable file.
    screenshot_path: Option<std::path::PathBuf>,
    screenshot_taken: bool, // Track if delayed screenshot was taken
    should_exit: bool,      // Track if app should exit
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
    /// ARC-006: set by any unprocessed input event (pointer/keyboard/touch/wheel)
    /// and OR'd into the redraw decision. Forces a redraw so egui (and the
    /// fractal) get to process the event. Without this, render-on-demand parks
    /// the loop after a click that egui doesn't flag for repaint — and since
    /// egui only updates its pointer/hover state *during* a render, it can never
    /// request the repaint it would need to resume, freezing the UI.
    input_pending: bool,
    /// ARC-018: in-flight background GPU enumeration. `Some` while a worker
    /// thread is scanning adapters; the result is drained from the receiver
    /// in `App::update` each frame (no blocking). Native only — on wasm,
    /// `enumerate_gpus` returns an empty `Vec`, so the receiver stays `None`
    /// and the UI's "Scanning…" label is set/cleared synchronously.
    #[cfg(not(target_arch = "wasm32"))]
    gpu_scan_receiver: Option<std::sync::mpsc::Receiver<Vec<GpuInfo>>>,
    /// ENH-001 Phase A step 5: schedules the off-render-thread reference
    /// orbit compute at deep zoom. Polled each frame from `App::update`:
    /// `note_view` records the current view, `maybe_spawn` kicks off the
    /// worker when stale + gate-met, and `poll` drains the result. Native
    /// only (no-op stub on wasm — HP path handles deep zoom there).
    perturbation_driver: PerturbationDriver,
}

impl App {
    /// Create a new App instance (native version).
    ///
    /// Thin async wrapper around [`init_common`](Self::init_common); `main.rs`
    /// blocks on it via `pollster::block_on(App::new(...))`. Keeping the body
    /// in a shared `init_common` is the ARC-014 dedup: native and web share
    /// one construction path, including camera-position / UI-state restore.
    #[cfg(feature = "native")]
    pub async fn new(
        window: Window,
        screenshot_delay: Option<f32>,
        exit_delay: Option<f32>,
        preset_name: Option<String>,
        quality_level: Option<usize>,
        screenshot_path: Option<std::path::PathBuf>,
    ) -> Self {
        Self::init_common(
            window,
            screenshot_delay,
            exit_delay,
            preset_name,
            quality_level,
            screenshot_path,
        )
        .await
    }

    /// Create a new App instance (web version).
    ///
    /// Thin async wrapper around [`init_common`](Self::init_common). Returns
    /// `Result` for symmetry with the previous signature; `init_common` itself
    /// is infallible (errors were only ever raised by wgpu during renderer
    /// construction, which now panics inside `Renderer::new` like native).
    #[cfg(target_arch = "wasm32")]
    pub async fn new_async(
        window: Window,
        screenshot_delay: Option<f32>,
        exit_delay: Option<f32>,
        preset_name: Option<String>,
        quality_level: Option<usize>,
        screenshot_path: Option<std::path::PathBuf>,
    ) -> Result<Self, String> {
        Ok(Self::init_common(
            window,
            screenshot_delay,
            exit_delay,
            preset_name,
            quality_level,
            screenshot_path,
        )
        .await)
    }

    /// Load the user's [`crate::fractal::Settings`] via the platform storage
    /// abstraction (ARC-014).
    ///
    /// Goes through [`PlatformContext`] / [`Storage`] so the same code path
    /// works on native (filesystem) and web (localStorage). Native first
    /// queries the platform-storage location (`<config_dir>/settings/settings.yaml`);
    /// if that is empty it falls back to the pre-ARC-014 location
    /// (`<config_dir>/settings.yaml`) so existing user files keep loading
    /// non-destructively while the save path migrates over.
    fn load_settings_via_platform() -> Option<crate::fractal::Settings> {
        let storage = PlatformContext::new().storage;
        if let Ok(Some(bytes)) = storage.load(category::SETTINGS, "settings") {
            if let Ok(yaml) = std::str::from_utf8(&bytes) {
                if let Ok(settings) = serde_yaml::from_str::<crate::fractal::Settings>(yaml) {
                    return Some(settings);
                }
            }
        }

        // Legacy native fallback: pre-ARC-014 settings lived at
        // `<config_dir>/settings.yaml` (no category subdir). Keep loading
        // these so the migration to platform-storage is non-destructive
        // until the save path is also fully migrated.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let legacy = config_dir.config_dir().join("settings.yaml");
            if let Ok(yaml) = std::fs::read_to_string(legacy)
                && let Ok(settings) = serde_yaml::from_str::<crate::fractal::Settings>(&yaml)
            {
                return Some(settings);
            }
        }

        None
    }

    /// Shared constructor body for both native and web (ARC-014).
    ///
    /// The two public constructors ([`new`](Self::new) on native,
    /// [`new_async`](Self::new_async) on web) are thin wrappers around this;
    /// all the per-target divergence lives here behind narrow `cfg` gates:
    ///
    /// - GPU-index preference runs on native only (the browser handles
    ///   adapter selection on web).
    /// - The 0×0 window-size fallback runs on web only.
    /// - User preset files are loaded from disk on native only; web has no
    ///   file access and falls back to defaults.
    /// - The `video_recorder` field exists on native only.
    ///
    /// **ARC-014 drift fix:** camera position and UI state are restored on
    /// every target via [`load_settings_via_platform`](Self::load_settings_via_platform).
    /// Previously the web constructor skipped this entire block; now it runs
    /// identically, reading from `localStorage` there.
    async fn init_common(
        window: Window,
        screenshot_delay: Option<f32>,
        exit_delay: Option<f32>,
        preset_name: Option<String>,
        quality_level: Option<usize>,
        screenshot_path: Option<std::path::PathBuf>,
    ) -> Self {
        let window = Arc::new(window);
        let size = window.inner_size();

        // Web canvases sometimes start at 0×0 before layout settles; fall
        // back so wgpu surface creation doesn't fail. Native windows are
        // always sized at construction.
        #[cfg(target_arch = "wasm32")]
        let size = if size.width == 0 || size.height == 0 {
            log::warn!(
                "Window size is {}x{}, using fallback 800x600",
                size.width,
                size.height
            );
            winit::dpi::PhysicalSize::new(800, 600)
        } else {
            size
        };
        #[cfg(target_arch = "wasm32")]
        log::info!(
            "Initializing renderer with size {}x{}",
            size.width,
            size.height
        );

        // Renderer. Native honours the saved GPU preference; web leaves it
        // to the browser.
        let renderer = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let prefs = crate::fractal::AppPreferences::load();
                if let Some(gpu_index) = prefs.preferred_gpu_index {
                    log::info!("Using preferred GPU index: {}", gpu_index);
                    Renderer::new_with_gpu_preference(window.clone(), size, Some(gpu_index)).await
                } else {
                    Renderer::new(window.clone(), size).await
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                Renderer::new(window.clone(), size).await
            }
        };

        // Load saved Settings once via the platform trait (camera/UI state
        // restore runs on every target — ARC-014 drift fix). The preset
        // path below overrides fractal parameters but not camera position,
        // preserving the prior native semantics.
        let saved_settings = Self::load_settings_via_platform();

        let mut fractal_params = if let Some(preset) = preset_name.as_deref() {
            // Built-in presets are in-memory and available on every target.
            if let Some(preset_data) = crate::fractal::PresetGallery::get_builtin_preset(preset) {
                #[cfg(not(target_arch = "wasm32"))]
                log::info!("Loaded built-in preset: {}", preset);
                #[cfg(target_arch = "wasm32")]
                log::info!("Loaded preset: {}", preset);
                FractalParams::from_settings(preset_data.settings.clone())
            } else {
                // User preset files: native loads from disk; web has no
                // file access and falls back to saved settings / defaults.
                #[cfg(not(target_arch = "wasm32"))]
                {
                    match crate::fractal::PresetGallery::load_preset(preset) {
                        Ok(preset_data) => {
                            log::info!("Loaded user preset: {}", preset);
                            FractalParams::from_settings(preset_data.settings)
                        }
                        Err(e) => {
                            log::error!("Failed to load preset '{}': {}", preset, e);
                            log::error!("Falling back to saved settings or defaults");
                            saved_settings
                                .as_ref()
                                .map(|s| FractalParams::from_settings(s.clone()))
                                .unwrap_or_default()
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    log::warn!("Preset '{}' not found, using defaults", preset);
                    FractalParams::default()
                }
            }
        } else {
            saved_settings
                .as_ref()
                .map(|s| FractalParams::from_settings(s.clone()))
                .unwrap_or_default()
        };

        // Apply quality level from CLI (native) or URL param (web).
        if let Some(level) = quality_level {
            let level = level.min(3); // Clamp to valid range 0-3
            let quality_name = match level {
                0 => "Ultra",
                1 => "High",
                2 => "Medium",
                _ => "Low",
            };
            #[cfg(not(target_arch = "wasm32"))]
            log::info!("Setting quality level: {}", quality_name);
            #[cfg(target_arch = "wasm32")]
            log::info!("Setting quality level: {}", quality_name);

            // Enable LOD system and set to use the specified quality level
            fractal_params.lod.lod_config.enabled = true;
            fractal_params.lod.lod_state.current_level = level;
            fractal_params.lod.lod_state.target_level = level;
            fractal_params.lod.lod_state.transition_progress = 1.0;
            fractal_params.lod.lod_state.active_quality =
                fractal_params.lod.lod_config.quality_presets[level];

            // Set min_quality_level so LOD won't drop below the requested level
            fractal_params.lod.lod_config.min_quality_level = level;

            // Apply the quality settings to fractal params immediately
            fractal_params.settings.max_steps =
                fractal_params.lod.lod_state.active_quality.max_steps;
            fractal_params.settings.min_distance =
                fractal_params.lod.lod_state.active_quality.min_distance;
        }

        let mut camera = Camera::new(size.width, size.height);
        camera.fovy = fractal_params.settings.camera_fov;
        let mut camera_controller = CameraController::new(fractal_params.settings.camera_speed);

        // ARC-014 drift fix: restore camera position + UI state on EVERY
        // target via the platform storage trait. Web previously skipped this
        // block entirely; now it runs identically, reading from localStorage
        // there (and from the legacy `settings.yaml` path on native until the
        // save side fully migrates).
        let mut ui = UI::new();
        if let Some(settings) = saved_settings {
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

        #[cfg(feature = "native")]
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
            #[cfg(feature = "native")]
            video_recorder,
            screenshot_delay,
            exit_delay,
            screenshot_path,
            screenshot_taken: false,
            should_exit: false,
            bloom_texture_cleared: false,
            scene_dirty: true,
            input_pending: false,
            #[cfg(not(target_arch = "wasm32"))]
            gpu_scan_receiver: None,
            perturbation_driver: PerturbationDriver::new(),
        }
    }

    /// Borrow the application's window.
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Whether the app has requested to exit (e.g. after a CLI `--exit-delay`).
    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    /// Whether a CLI timer (`--screenshot-delay` / `--exit-delay`) is still
    /// pending. These timers are evaluated inside `update()`, which only runs
    /// on `RedrawRequested`, so the event loop must keep ticking until they
    /// fire — otherwise ARC-006's `ControlFlow::Wait` parks the loop and the
    /// timer never gets checked. Normal interactive use sets no CLI timer and
    /// stays parked for power savings.
    pub fn has_pending_cli_timer(&self) -> bool {
        self.screenshot_delay.is_some() || self.exit_delay.is_some()
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
        if self.fractal_params.settings.auto_orbit
            && self.fractal_params.settings.render_mode == RenderMode::ThreeD
        {
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
        if self.fractal_params.settings.procedural_palette != ProceduralPalette::None {
            return true;
        }
        // LOD is smoothly interpolating between quality presets.
        if self.fractal_params.lod.lod_config.enabled
            && self.fractal_params.lod.lod_state.transition_progress < 1.0
        {
            return true;
        }
        // Attractor / Buddhabrot still accumulating samples.
        if self.fractal_params.settings.attractor_accumulation_enabled
            && !self.fractal_params.accum.paused
            && (self.fractal_params.settings.fractal_type.is_2d_attractor()
                || self.fractal_params.settings.fractal_type.is_buddhabrot())
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
        // `input_pending` covers the egui render-on-demand death spiral: egui
        // only updates its pointer/hover state during a render, so without a
        // redraw forced on input it can freeze after a click. See `input_pending`.
        self.scene_dirty
            || self.input_pending
            || self.is_scene_animation_active()
            || self.ui_needs_repaint()
            || self.save_screenshot
            || self.save_hires_render.is_some()
    }

    /// ARC-006: hook called from `App::render` after the fractal pass + UI
    /// submit. Clears the dirty flag when no continuous animation source is
    /// active (animation sources keep the flag set so the next frame also
    /// renders). Called by `render.rs` at the end of `App::render`.
    pub fn after_render_frame(&mut self) {
        // The input event has now been processed by this render (egui `run`
        // consumed it); clear it so the flag is set fresh by the next event.
        self.input_pending = false;
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
                log::error!("Failed to save window size: {}", e);
            }
        }
    }

    fn reset_view(&mut self) {
        match self.fractal_params.settings.render_mode {
            RenderMode::TwoD => {
                // Re-apply fractal defaults (this sets the correct center and zoom for each fractal type)
                let current_type = self.fractal_params.settings.fractal_type;
                self.fractal_params.switch_fractal(current_type);

                // Clear accumulation for strange attractors and sync tracking values
                if self.fractal_params.settings.attractor_accumulation_enabled {
                    self.fractal_params.accum.pending_clear = true;
                    self.fractal_params.accum.total_iterations = 0;
                    // Sync tracking to the reset values
                    self.fractal_params.accum.last_center = self.fractal_params.settings.center_2d;
                    self.fractal_params.accum.last_zoom = self.fractal_params.settings.zoom_2d;
                    self.fractal_params.accum.last_julia_c = self.fractal_params.settings.julia_c;
                }
            }
            RenderMode::ThreeD => {
                let size = self.renderer.size;
                self.camera = Camera::new(size.width, size.height);
                self.camera.fovy = self.fractal_params.settings.camera_fov;
                self.camera_controller =
                    CameraController::new(self.fractal_params.settings.camera_speed);
            }
        }
    }
}
