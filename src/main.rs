//! ARC-024: the binary is a thin shim over the `par_fractal` library — the
//! whole module tree compiles ONCE (in the rlib); declaring `mod` here would
//! duplicate it into the binary (the source of the parsight "dead App
//! methods" ghost and the `#[allow(dead_code)] mod platform` silencer).
use par_fractal::app::{self, App};
use par_fractal::fractal;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

fn clear_settings() {
    if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
        let config_path = config_dir.config_dir();

        // Clears every location the loader reads, preserving presets and other
        // user data. Deleting only the legacy path here let a reset silently
        // no-op while the platform-storage copy kept loading.
        match app::clear_settings_files(config_path) {
            Ok(removed) if removed.is_empty() => println!("No settings to clear"),
            Ok(removed) => {
                for path in removed {
                    println!("Settings cleared: {}", path.display());
                }
            }
            Err(e) => eprintln!("Failed to clear settings: {}", e),
        }

        // Note: User presets in {}/presets/ are preserved
        let presets_dir = config_path.join("presets");
        if presets_dir.exists() {
            println!(
                "Note: User presets in {} are preserved",
                presets_dir.display()
            );
        }
    }
}

fn list_presets() {
    use fractal::{PresetCategory, PresetGallery};

    println!("Available Presets:");
    println!("==================\n");

    // Get built-in presets
    let builtin_presets = PresetGallery::get_builtin_presets();

    // Group by category
    let mut categories = std::collections::HashMap::new();
    for preset in &builtin_presets {
        categories
            .entry(preset.category)
            .or_insert_with(Vec::new)
            .push(preset);
    }

    // Print by category
    let category_order = [
        PresetCategory::TwoDFractals,
        PresetCategory::ThreeDFractals,
        PresetCategory::IFS,
        PresetCategory::Apollonian,
    ];

    for category in &category_order {
        if let Some(presets) = categories.get(category) {
            let category_name = match category {
                PresetCategory::TwoDFractals => "2D Fractals",
                PresetCategory::ThreeDFractals => "3D Fractals",
                PresetCategory::IFS => "IFS Fractals",
                PresetCategory::Apollonian => "Apollonian Gasket",
                PresetCategory::All => "All",
            };

            println!("📁 {}:", category_name);
            for preset in presets {
                println!("   • {} - {}", preset.name, preset.description);
            }
            println!();
        }
    }

    // Get user presets
    if let Ok(user_presets) = PresetGallery::list_user_presets()
        && !user_presets.is_empty()
    {
        println!("💾 User Presets:");
        for preset_name in user_presets {
            if let Ok(preset) = PresetGallery::load_preset(&preset_name) {
                println!("   • {} - {}", preset.name, preset.description);
            } else {
                println!("   • {}", preset_name);
            }
        }
        println!();
    }

    println!("Total: {} built-in presets", builtin_presets.len());
    println!("\nUsage: par-fractal --preset \"<preset name>\"");
}

fn main() {
    let cli = <par_fractal::cli::Cli as clap::Parser>::parse();

    // Hidden agent helper: emit completions and exit (ENH-003).
    if let Some(shell) = cli.generate_completions {
        let mut cmd = <par_fractal::cli::Cli as clap::CommandFactory>::command();
        let name = cmd.get_name().to_string();
        clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
        return;
    }

    if cli.list_presets {
        list_presets();
        return;
    }

    // NOTE: --clear-settings clears AND continues launching (historical
    // behavior — preserved identically by ENH-003).
    if cli.clear_settings {
        clear_settings();
    }

    // ENH-003: post-parse validation for the two-token flags (clap's
    // value_parser applies per-value, so their typed checks live here).
    let resize_after = cli.resize_after.as_ref().map(|pair| {
        let secs: f32 = pair[0].parse().unwrap_or_else(|_| {
            eprintln!(
                "Invalid --resize-after delay '{}' (expected seconds)",
                pair[0]
            );
            std::process::exit(2);
        });
        if !secs.is_finite() || secs < 0.0 {
            eprintln!(
                "Invalid --resize-after delay '{}' (must be finite and >= 0)",
                pair[0]
            );
            std::process::exit(2);
        }
        let (w, h) = pair[1].split_once('x').unwrap_or_else(|| {
            eprintln!("Invalid --resize-after size '{}' (expected WxH)", pair[1]);
            std::process::exit(2);
        });
        let (width, height) = match (w.parse::<u32>(), h.parse::<u32>()) {
            (Ok(width), Ok(height)) if width > 0 && height > 0 => (width, height),
            _ => {
                eprintln!("Invalid --resize-after size '{}' (expected WxH)", pair[1]);
                std::process::exit(2);
            }
        };
        (secs, width, height)
    });

    let switch_after = cli.switch_after.as_ref().map(|pair| {
        let ft = serde_yaml::from_str::<fractal::FractalType>(&pair[0]).unwrap_or_else(|_| {
            eprintln!(
                "Unknown fractal type '{}' for --switch-after (use a FractalType variant name, e.g. Hopalong2D)",
                pair[0]
            );
            std::process::exit(2);
        });
        let secs: f32 = pair[1]
            .parse()
            .unwrap_or_else(|_| {
                eprintln!("Invalid --switch-after delay '{}' (expected seconds)", pair[1]);
                std::process::exit(2);
            });
        if !secs.is_finite() {
            eprintln!("Invalid --switch-after delay '{}' (must be finite)", pair[1]);
            std::process::exit(2);
        }
        (ft, secs)
    });

    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    // Load window size preference (default to 960x540 if none saved). The
    // actual window is created inside `AppHandler::resumed` — winit 0.30's
    // contract requires windows to be created from the active event loop,
    // which is only available once the loop is running.
    let prefs = fractal::AppPreferences::load();
    let (initial_width, initial_height) = prefs.window_size_or_default();
    // ENH-007: `--window-size` overrides the saved preference so the visual-
    // regression harness gets a fixed capture resolution.
    let initial_window_size = cli.window_size.unwrap_or((initial_width, initial_height));

    let camera_pos = cli.camera_pos.map(glam::Vec3::from_array);

    let mut handler = AppHandler {
        screenshot_delay: cli.screenshot_delay,
        exit_delay: cli.exit_delay,
        preset_name: cli.preset,
        quality_level: cli.quality,
        screenshot_path: cli.screenshot_path,
        profile_dump_path: cli.profile_dump,
        initial_window_size,
        resize_after,
        start: None,
        resize_taken: false,
        switch_after,
        switch_done: false,
        camera_pos,
        app: None,
    };

    event_loop.run_app(&mut handler).unwrap();
}

/// winit 0.30 `ApplicationHandler` wrapper around [`App`].
///
/// Holds the CLI/startup arguments needed to build `App` and owns the `App`
/// instance as `Option<App>` — `None` until the first `resumed()` fires, where
/// the window and renderer are created (the winit 0.30 contract: windows must
/// be created from the `ActiveEventLoop`, which only exists once the loop is
/// running). `resumed()` is guarded so a second fire (mobile lifecycle / bfcache
/// restore) is a no-op rather than re-creating the window.
struct AppHandler {
    screenshot_delay: Option<f32>,
    exit_delay: Option<f32>,
    preset_name: Option<String>,
    quality_level: Option<usize>,
    screenshot_path: Option<std::path::PathBuf>,
    profile_dump_path: Option<std::path::PathBuf>,
    initial_window_size: (u32, u32),
    /// QA-027: `(delay_secs, width, height)` — request a window resize after
    /// the delay to exercise the resize lifecycle without a human.
    resize_after: Option<(f32, u32, u32)>,
    /// Process start time (set in `resumed`); the resize-delay origin.
    start: Option<std::time::Instant>,
    /// QA-027: tracks that the deferred resize has fired (idempotent).
    resize_taken: bool,
    /// Agent-operability: `(FractalType, delay_secs)` — switch the fractal type
    /// after the delay. Powers scripted type-transition tests (e.g. reproducing
    /// the Buddhabrot → attractor accumulation bind-group switch bug).
    switch_after: Option<(fractal::FractalType, f32)>,
    /// Tracks that the deferred switch has fired (idempotent).
    switch_done: bool,
    /// Agent-operability: `--camera-pos x,y,z`. Applied at startup and re-applied
    /// after `switch_after` fires (which otherwise re-frames to the type default).
    camera_pos: Option<glam::Vec3>,
    app: Option<App>,
}

impl ApplicationHandler for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // `resumed()` can fire multiple times on some platforms (mobile
        // lifecycle, web bfcache restore). Create the window + renderer once.
        if self.app.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("Par Fractal - GPU Accelerated Fractal Renderer")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                self.initial_window_size.0,
                self.initial_window_size.1,
            ));
        let window = match event_loop.create_window(window_attributes) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to create window: {}", e);
                event_loop.exit();
                return;
            }
        };

        // `App::new` is async (wgpu device/surface init). Native blocks on it
        // synchronously here — the standard winit 0.30 + wgpu pattern.
        let app = pollster::block_on(App::new(
            window,
            self.screenshot_delay,
            self.exit_delay,
            self.preset_name.clone(),
            self.quality_level,
            self.screenshot_path.clone(),
            self.profile_dump_path.clone(),
        ));
        // QA-027: the resize-delay origin is when the app is ready to render,
        // not process start (App::new blocks on GPU init).
        self.start = Some(std::time::Instant::now());
        self.app = Some(app);
        if let (Some(pos), Some(app)) = (self.camera_pos, self.app.as_mut()) {
            app.set_camera_position(pos);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(app) = self.app.as_mut() else {
            return;
        };
        if window_id != app.window().id() {
            return;
        }
        if !app.input(&event) {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(physical_size) => {
                    app.resize(physical_size);
                }
                WindowEvent::RedrawRequested => {
                    app.update();
                    app.render();
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // QA-027: decide the deferred resize from self state first, before the
        // &mut app borrow below, so the field borrows stay disjoint. The resize
        // fires once the delay elapses; until then the loop must keep ticking
        // (a parked ControlFlow::Wait loop would otherwise sleep past it).
        let resize_due = matches!(
            (self.resize_after, self.resize_taken, self.start),
            (Some((delay, _, _)), false, Some(s))
                if s.elapsed() >= std::time::Duration::from_secs_f32(delay)
        );
        let resize_pending = self.resize_after.is_some() && (!self.resize_taken);
        let switch_due = matches!(
            (self.switch_after, self.switch_done, self.start),
            (Some((_, delay)), false, Some(s))
                if s.elapsed() >= std::time::Duration::from_secs_f32(delay)
        );
        let switch_pending = self.switch_after.is_some() && (!self.switch_done);

        let Some(app) = self.app.as_mut() else {
            return;
        };
        if resize_due {
            if let Some((_, w, h)) = self.resize_after {
                // request_inner_size returns the constrained size; the actual
                // size arrives via the Resized event, which drives app.resize.
                let _ = app
                    .window()
                    .request_inner_size(winit::dpi::PhysicalSize::new(w, h));
            }
            self.resize_taken = true;
        }
        if switch_due {
            if let Some((ft, _)) = self.switch_after {
                app.switch_fractal(ft);
                // switch_fractal re-frames to the type default; an explicit
                // --camera-pos must still win.
                if let Some(pos) = self.camera_pos {
                    app.set_camera_position(pos);
                }
            }
            self.switch_done = true;
        }
        // Check if app should exit (from CLI delay option)
        if app.should_exit() {
            event_loop.exit();
        }
        // ARC-006: render-on-demand. Only request a redraw when the
        // scene changed, an animation source is active (auto-orbit,
        // palette animation, LOD transition, attractor accumulation,
        // video recording), or egui wants another frame. Otherwise
        // park the loop in `ControlFlow::Wait` — the OS wakes us on
        // the next input/expose event and idle CPU/GPU goes to ~0.
        // (`Wait` + `request_redraw` is the render-on-demand idiom:
        // winit schedules exactly one RedrawRequested, then sleeps.)
        event_loop.set_control_flow(ControlFlow::Wait);
        // ARC-006: render-on-demand. Only request a redraw when the scene
        // changed, an animation source is active, or egui wants another frame.
        // A pending CLI timer (--screenshot-delay/--exit-delay) also keeps the
        // loop ticking: that timer is evaluated inside update() on the redraw
        // path, so without a redraw it would never fire under ControlFlow::Wait.
        // QA-027: a pending --resize-after does too.
        if app.should_render_next_frame()
            || app.has_pending_cli_timer()
            || resize_pending
            || switch_pending
        {
            app.window().request_redraw();
        }
    }
}
