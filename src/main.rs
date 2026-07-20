mod app;
mod camera;
mod command_palette;
/// ENH-001 perturbation-theory deep-zoom subsystem: arbitrary-precision
/// reference orbit (CPU foundation) + the off-render-thread driver that
/// recomputes it on view change.
mod deep_zoom;
mod fractal;
mod lod;
// `platform` is dormant infrastructure being incrementally wired up via the
// `platform::` traits (ARC-014 constructor dedup uses `PlatformContext` +
// `Storage`; the `FileDialog` / `Capture` traits and the unused `category`
// constants await the larger `capture.rs` / `capture_web.rs` dedup noted in
// AUDIT ARC-014). QA-020 audit: silenced with a reason, not deleted, because
// the platform-abstraction layering is intentional infrastructure.
#[allow(dead_code)]
mod platform;
mod renderer;
mod ui;
mod video_recorder;

use app::App;
use std::env;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

fn print_help() {
    println!("Par Fractal - GPU Accelerated Fractal Renderer");
    println!();
    println!("Usage: par-fractal [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --clear-settings         Clear all saved settings and start fresh");
    println!("  --preset <name>          Load a specific preset on startup");
    println!("  --quality <level>        Set quality level: low, medium, high, ultra");
    println!("  -q <level>               Short form of --quality");
    println!("  --list-presets           List all available presets and exit");
    println!("  --screenshot-delay <s>   Take a screenshot after N seconds");
    println!("  --exit-delay <s>         Exit application after N seconds");
    println!("  --screenshot-path <path> Write the screenshot to this exact path (no timestamp)");
    println!(
        "  --profile-dump <path>    Write EMA-smoothed GPU profile timings (YAML) after warmup"
    );
    println!("  --window-size <WxH>      Force the window to WxH physical pixels");
    println!("  --resize-after <s WxH>   Resize the window to WxH after N seconds (QA-027)");
    println!("  --switch-after <type> <s> Switch to FractalType after N seconds (agent testing)");
    println!("  --help, -h               Show this help message");
}

/// Parse quality level string to LOD level index (0=ultra, 1=high, 2=medium, 3=low)
fn parse_quality_level(s: &str) -> Option<usize> {
    match s.to_lowercase().as_str() {
        "ultra" | "u" => Some(0),
        "high" | "h" => Some(1),
        "medium" | "med" | "m" => Some(2),
        "low" | "l" => Some(3),
        _ => None,
    }
}

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
    let args: Vec<String> = env::args().collect();

    let mut screenshot_delay: Option<f32> = None;
    let mut exit_delay: Option<f32> = None;
    let mut preset_name: Option<String> = None;
    let mut quality_level: Option<usize> = None;
    let mut screenshot_path: Option<std::path::PathBuf> = None;
    // ENH-006 Task 3: write the EMA-smoothed per-scope GPU timings to YAML
    // after a warmup window so agents/ci can measure optimizations. Runtime
    // only — never persisted (same shape as `--exit-delay` / `--screenshot-*`).
    let mut profile_dump_path: Option<std::path::PathBuf> = None;
    let mut window_size: Option<(u32, u32)> = None;
    // QA-027: resize the window mid-run to exercise the winit 0.30 resize
    // lifecycle (surface reconfigure + redraw) without driving a human input.
    let mut resize_after: Option<(f32, u32, u32)> = None;
    // Agent-operability: `--switch-after <FractalType> <secs>` (see AppHandler).
    let mut switch_after: Option<(fractal::FractalType, f32)> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--clear-settings" => {
                clear_settings();
                i += 1;
            }
            "--screenshot-path" => {
                if i + 1 < args.len() {
                    screenshot_path = Some(std::path::PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("--screenshot-path requires a value");
                    print_help();
                    return;
                }
            }
            "--profile-dump" => {
                if i + 1 < args.len() {
                    profile_dump_path = Some(std::path::PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("--profile-dump requires a value");
                    print_help();
                    return;
                }
            }
            "--window-size" => {
                if i + 1 < args.len() {
                    match args[i + 1].split_once('x') {
                        Some((w, h)) => match (w.parse::<u32>(), h.parse::<u32>()) {
                            (Ok(w), Ok(h)) if w > 0 && h > 0 => {
                                window_size = Some((w, h));
                                i += 2;
                            }
                            _ => {
                                eprintln!(
                                    "Invalid --window-size '{}' (expected WxH, e.g. 256x256)",
                                    args[i + 1]
                                );
                                print_help();
                                return;
                            }
                        },
                        None => {
                            eprintln!(
                                "Invalid --window-size '{}' (expected WxH, e.g. 256x256)",
                                args[i + 1]
                            );
                            print_help();
                            return;
                        }
                    }
                } else {
                    eprintln!("--window-size requires a value (WxH)");
                    print_help();
                    return;
                }
            }
            "--preset" => {
                if i + 1 < args.len() {
                    preset_name = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("--preset requires a preset name");
                    print_help();
                    return;
                }
            }
            "--quality" | "-q" => {
                if i + 1 < args.len() {
                    match parse_quality_level(&args[i + 1]) {
                        Some(level) => {
                            quality_level = Some(level);
                            i += 2;
                        }
                        None => {
                            eprintln!(
                                "Invalid quality level '{}'. Valid options: low, medium, high, ultra",
                                args[i + 1]
                            );
                            print_help();
                            return;
                        }
                    }
                } else {
                    eprintln!("--quality requires a value (low, medium, high, ultra)");
                    print_help();
                    return;
                }
            }
            "--screenshot-delay" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<f32>() {
                        Ok(delay) => {
                            screenshot_delay = Some(delay);
                            i += 2;
                        }
                        Err(_) => {
                            eprintln!(
                                "Invalid delay value for --screenshot-delay: {}",
                                args[i + 1]
                            );
                            print_help();
                            return;
                        }
                    }
                } else {
                    eprintln!("--screenshot-delay requires a value");
                    print_help();
                    return;
                }
            }
            "--exit-delay" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<f32>() {
                        Ok(delay) => {
                            exit_delay = Some(delay);
                            i += 2;
                        }
                        Err(_) => {
                            eprintln!("Invalid delay value for --exit-delay: {}", args[i + 1]);
                            print_help();
                            return;
                        }
                    }
                } else {
                    eprintln!("--exit-delay requires a value");
                    print_help();
                    return;
                }
            }
            "--switch-after" => {
                // Agent-operability: `--switch-after <FractalType> <secs>`
                // switches fractal type after a delay (two tokens). The variant
                // name parses via serde (same wire format as settings.yaml).
                if i + 2 < args.len() {
                    let ft = serde_yaml::from_str::<fractal::FractalType>(&args[i + 1]);
                    let secs = args[i + 2].parse::<f32>();
                    match (ft, secs) {
                        (Ok(ft), Ok(secs)) => {
                            switch_after = Some((ft, secs));
                            i += 3;
                        }
                        (Err(_), _) => {
                            eprintln!(
                                "Unknown fractal type '{}' for --switch-after \
                                 (use a FractalType variant name, e.g. Hopalong2D)",
                                args[i + 1]
                            );
                            print_help();
                            return;
                        }
                        (_, Err(_)) => {
                            eprintln!(
                                "Invalid --switch-after delay '{}' (expected seconds)",
                                args[i + 2]
                            );
                            print_help();
                            return;
                        }
                    }
                } else {
                    eprintln!("--switch-after requires <FractalType> <secs>");
                    print_help();
                    return;
                }
            }
            "--resize-after" => {
                // QA-027: `--resize-after <secs> <WxH>` resizes the window after
                // a delay (two tokens). Powers the lifecycle smoke test.
                if i + 2 < args.len() {
                    match (args[i + 1].parse::<f32>(), args[i + 2].split_once('x')) {
                        (Ok(secs), Some((w, h))) => match (w.parse::<u32>(), h.parse::<u32>()) {
                            (Ok(width), Ok(height)) => {
                                resize_after = Some((secs, width, height));
                                i += 3;
                            }
                            _ => {
                                eprintln!(
                                    "Invalid --resize-after size '{}' (expected WxH)",
                                    args[i + 2]
                                );
                                print_help();
                                return;
                            }
                        },
                        _ => {
                            eprintln!("Invalid --resize-after (expected <secs WxH>)");
                            print_help();
                            return;
                        }
                    }
                } else {
                    eprintln!("--resize-after requires <secs WxH>");
                    print_help();
                    return;
                }
            }
            "--list-presets" => {
                list_presets();
                return;
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_help();
                return;
            }
        }
    }

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
    let initial_window_size = window_size.unwrap_or((initial_width, initial_height));

    let mut handler = AppHandler {
        screenshot_delay,
        exit_delay,
        preset_name,
        quality_level,
        screenshot_path,
        profile_dump_path,
        initial_window_size,
        resize_after,
        start: None,
        resize_taken: false,
        switch_after,
        switch_done: false,
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
