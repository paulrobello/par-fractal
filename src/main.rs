mod app;
mod camera;
mod command_palette;
mod fractal;
mod lod;
mod renderer;
mod ui;
mod video_recorder;

use app::App;
use std::env;
use winit::{event::*, event_loop::EventLoop};

fn print_help() {
    println!("Par Fractal - GPU Accelerated Fractal Renderer");
    println!();
    println!("Usage: par-fractal [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --clear-settings         Clear all saved settings and start fresh");
    println!("  --preset <name>          Load a specific preset on startup");
    println!("  --list-presets           List all available presets and exit");
    println!("  --screenshot-delay <s>   Take a screenshot after N seconds");
    println!("  --exit-delay <s>         Exit application after N seconds");
    println!("  --help, -h               Show this help message");
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
    if let Ok(user_presets) = PresetGallery::list_user_presets() {
        if !user_presets.is_empty() {
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
    }

    println!("Total: {} built-in presets", builtin_presets.len());
    println!("\nUsage: par-fractal --preset \"<preset name>\"");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut screenshot_delay: Option<f32> = None;
    let mut exit_delay: Option<f32> = None;
    let mut preset_name: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--clear-settings" => {
                clear_settings();
                i += 1;
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

    // Load window size preference (default to 960x540 if none saved)
    let prefs = fractal::AppPreferences::load();
    let (initial_width, initial_height) = prefs.window_size_or_default();

    let window_attributes = winit::window::Window::default_attributes()
        .with_title("Par Fractal - GPU Accelerated Fractal Renderer")
        .with_inner_size(winit::dpi::PhysicalSize::new(initial_width, initial_height));
    #[allow(deprecated)]
    let window = event_loop.create_window(window_attributes).unwrap();

    let mut app = pollster::block_on(App::new(window, screenshot_delay, exit_delay, preset_name));

    #[allow(deprecated)]
    event_loop
        .run(move |event, target| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == app.window().id() => {
                if !app.input(event) {
                    match event {
                        WindowEvent::CloseRequested => target.exit(),
                        WindowEvent::Resized(physical_size) => {
                            app.resize(*physical_size);
                        }
                        WindowEvent::RedrawRequested => {
                            app.update();
                            match app.render() {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => app.resize(app.size()),
                                Err(wgpu::SurfaceError::OutOfMemory) => target.exit(),
                                Err(e) => eprintln!("{:?}", e),
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::AboutToWait => {
                // Check if app should exit (from CLI delay option)
                if app.should_exit() {
                    target.exit();
                }
                app.window().request_redraw();
            }
            _ => {}
        })
        .unwrap();
}
