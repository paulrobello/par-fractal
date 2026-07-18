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
    println!("  --window-size <WxH>      Force the window to WxH physical pixels");
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
        let settings_file = config_path.join("settings.yaml");

        // Only delete settings.yaml, preserve presets and other user data
        if settings_file.exists() {
            match std::fs::remove_file(&settings_file) {
                Ok(_) => println!("Settings cleared: {}", settings_file.display()),
                Err(e) => eprintln!("Failed to clear settings: {}", e),
            }
        } else {
            println!("No settings to clear");
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
    let mut window_size: Option<(u32, u32)> = None;

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
        initial_window_size,
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
    initial_window_size: (u32, u32),
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
        ));
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
        let Some(app) = self.app.as_mut() else {
            return;
        };
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
        if app.should_render_next_frame() || app.has_pending_cli_timer() {
            app.window().request_redraw();
        }
    }
}
