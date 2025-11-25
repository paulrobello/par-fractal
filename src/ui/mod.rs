// Module declarations
mod command;
mod history;
mod monitor;
mod overlays;
mod toast;
mod toast_ui;

// Re-exports
pub use monitor::MonitorInfo;
pub use toast::Toast;

use crate::command_palette::CommandPalette;
use crate::fractal::{
    BookmarkGallery, CameraBookmark, CustomPalette, CustomPaletteGallery, FractalParams,
    FractalType, Preset, PresetCategory, PresetGallery, ShadingModel, UIState,
};
use egui::Context;
use glam::Vec3;

use history::HistoryEntry;

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
    pub ui_state: UIState,
    // Command palette
    pub command_palette: CommandPalette,
    // Performance tracking
    frame_times: Vec<f32>, // Last N frame times in milliseconds
    max_frame_history: usize,
    // Preset UI state
    preset_name: String,
    preset_description: String,
    preset_category: PresetCategory,
    preset_search: String,
    preset_category_filter: PresetCategory,
    user_presets: Vec<String>,
    last_preset_list_update: web_time::Instant,
    // Undo/Redo system
    history: Vec<HistoryEntry>,
    history_index: usize,
    max_history_size: usize,
    last_saved_params: Option<FractalParams>,
    // Camera bookmarks
    bookmark_name: String,
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
    // GPU selection
    pub available_gpus: Vec<super::renderer::GpuInfo>,
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
            show_ui: true,
            show_fps: false,
            show_camera_info: false,
            show_performance_overlay: false,
            ui_state: UIState::default(),
            command_palette: CommandPalette::new(),
            frame_times: Vec::with_capacity(120),
            max_frame_history: 120,
            preset_name: String::new(),
            preset_description: String::new(),
            preset_category: PresetCategory::All,
            preset_search: String::new(),
            preset_category_filter: PresetCategory::All,
            user_presets: PresetGallery::list_user_presets().unwrap_or_default(),
            last_preset_list_update: web_time::Instant::now(),
            history: Vec::new(),
            history_index: 0,
            max_history_size: 50,
            last_saved_params: None,
            bookmark_name: String::new(),
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
        eprintln!("DEBUG: Scanning for monitors...");
        self.available_monitors.clear();

        // Get primary monitor
        let primary_monitor = window.primary_monitor();
        eprintln!(
            "DEBUG: Primary monitor: {:?}",
            primary_monitor.as_ref().and_then(|m| m.name())
        );

        // Get all available monitors
        let mut count = 0;
        for (index, monitor) in window.available_monitors().enumerate() {
            count += 1;
            eprintln!("DEBUG: Found monitor {}: {:?}", index, monitor.name());
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

        eprintln!("DEBUG: Total monitors found: {}", count);
        eprintln!("DEBUG: Monitors in list: {}", self.available_monitors.len());

        // Update scan time
        self.last_monitor_scan = web_time::Instant::now();

        log::info!("Scanned {} monitor(s)", self.available_monitors.len());
    }

    pub fn load_ui_state(&mut self, ui_state: UIState) {
        self.show_fps = ui_state.show_fps;
        self.show_camera_info = ui_state.show_camera_info;
        self.ui_state = ui_state;
    }

    pub fn get_ui_state(&self) -> &UIState {
        &self.ui_state
    }

    #[allow(clippy::type_complexity)]
    pub fn render(
        &mut self,
        ctx: &Context,
        params: &mut FractalParams,
        camera_pos: Vec3,
        camera_target: Vec3,
        is_recording: bool,
    ) -> (
        bool,
        bool,
        bool,
        bool,
        bool,
        Option<Preset>,
        Option<(u32, u32)>,
        Option<CameraBookmark>,
        bool,
        bool,
        bool,
    ) {
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
            return (
                false, false, false, false, false, None, None, None, false, false, false,
            );
        }

        let mut changed = false;
        let mut screenshot_requested = false;
        let mut reset_requested = false;
        let mut reset_camera_requested = false;
        let mut point_at_fractal_requested = false;
        let mut preset_to_load: Option<Preset> = None;
        let mut hires_render_resolution: Option<(u32, u32)> = None;
        let mut randomize_requested = false;
        let mut start_recording = false;
        let mut stop_recording = false;
        let mut bookmark_to_load: Option<CameraBookmark> = None;
        let mut gpu_scan_requested = false;
        let mut from_history = false; // Don't save to history if change came from undo/redo

        egui::Window::new("Fractal Controls")
            .default_width(320.0)
            .default_height(600.0)
            .resizable(true)
            .vscroll(true)
            .show(ctx, |ui| {
                // Quick Actions at the top
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.can_undo(), egui::Button::new("↶ Undo"))
                        .on_hover_text("Undo last parameter change (Ctrl+Z)")
                        .clicked() {
                        if let Some(prev_params) = self.undo() {
                            *params = prev_params;
                            changed = true;
                            from_history = true;  // Don't save to history
                        }
                    }
                    if ui.add_enabled(self.can_redo(), egui::Button::new("↷ Redo"))
                        .on_hover_text("Redo parameter change (Ctrl+Y)")
                        .clicked() {
                        if let Some(next_params) = self.redo() {
                            *params = next_params;
                            changed = true;
                            from_history = true;  // Don't save to history
                        }
                    }
                    if ui.button("🎲 Randomize")
                        .on_hover_text("Generate random fractal settings for creative exploration")
                        .clicked() {
                        randomize_requested = true;
                    }
                });

                // UI Control Actions
                ui.horizontal(|ui| {
                    if ui.button("⏏ Hide UI")
                        .on_hover_text("Hide the control panel [H]")
                        .clicked() {
                        self.show_ui = false;
                    }

                    let theme_icon = if self.dark_theme { "☀" } else { "🌙" };
                    let theme_text = format!("{} Theme", theme_icon);
                    if ui.button(theme_text)
                        .on_hover_text("Toggle between dark and light themes")
                        .clicked() {
                        self.dark_theme = !self.dark_theme;
                    }

                    if ui.button("📷 Capture")
                        .on_hover_text("Open capture & recording panel")
                        .clicked() {
                        self.ui_state.capture_window_open = !self.ui_state.capture_window_open;
                    }

                    if ui.button("ℹ About")
                        .on_hover_text("About Par Fractal")
                        .clicked() {
                        self.ui_state.about_window_open = !self.ui_state.about_window_open;
                    }
                });
                ui.separator();

                let response = egui::CollapsingHeader::new("Fractal Type")
                    .default_open(self.ui_state.fractal_type_open)
                    .show(ui, |ui| {
                        let old_type = params.fractal_type;
                        ui.label("2D Fractals:");
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut params.fractal_type, FractalType::Mandelbrot2D, "Mandelbrot")
                                .on_hover_text("Classic Mandelbrot set - infinite detail fractal [1]");
                            ui.selectable_value(&mut params.fractal_type, FractalType::Julia2D, "Julia")
                                .on_hover_text("Julia set - beautiful variations with complex parameter [2]");
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut params.fractal_type, FractalType::Sierpinski2D, "Sierpinski Carpet")
                                .on_hover_text("Sierpinski carpet - recursive square pattern [3]");
                            ui.selectable_value(&mut params.fractal_type, FractalType::SierpinskiTriangle2D, "Sierpinski Triangle")
                                .on_hover_text("Sierpinski triangle - classic recursive triangle pattern");
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut params.fractal_type, FractalType::BurningShip2D, "Burning Ship")
                                .on_hover_text("Burning Ship fractal - variant with absolute values [4]");
                            ui.selectable_value(&mut params.fractal_type, FractalType::Tricorn2D, "Tricorn")
                                .on_hover_text("Tricorn - Mandelbrot with conjugate iteration [5]");
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut params.fractal_type, FractalType::Phoenix2D, "Phoenix")
                                .on_hover_text("Phoenix fractal - dynamic iteration algorithm [6]");
                            ui.selectable_value(&mut params.fractal_type, FractalType::Celtic2D, "Celtic")
                                .on_hover_text("Celtic fractal - alternative complex iteration [7]");
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut params.fractal_type, FractalType::Newton2D, "Newton")
                                .on_hover_text("Newton fractal - polynomial root-finding visualization [8]");
                            ui.selectable_value(&mut params.fractal_type, FractalType::Lyapunov2D, "Lyapunov")
                                .on_hover_text("Lyapunov fractal - stability diagram patterns [9]");
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut params.fractal_type, FractalType::Nova2D, "Nova")
                                .on_hover_text("Nova fractal - Newton-Mandelbrot hybrid [0]");
                            ui.selectable_value(&mut params.fractal_type, FractalType::Magnet2D, "Magnet")
                                .on_hover_text("Magnet Type 1 - physics-inspired fractal");
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut params.fractal_type, FractalType::Collatz2D, "Collatz")
                                .on_hover_text("Collatz fractal - based on Collatz conjecture");
                        });

                        ui.separator();
                        ui.label("2D Strange Attractors:");
                        // Helper macro-like closure to create attractor buttons that auto-enable accumulation
                        let mut attractor_button = |ui: &mut egui::Ui, fractal: FractalType, label: &str, hover: &str| {
                            let selected = params.fractal_type == fractal;
                            if ui.selectable_label(selected, label).on_hover_text(hover).clicked() {
                                params.fractal_type = fractal;
                                params.attractor_accumulation_enabled = true;
                                params.attractor_pending_clear = true;
                                params.attractor_total_iterations = 0;
                                changed = true;
                            }
                        };
                        ui.horizontal(|ui| {
                            attractor_button(ui, FractalType::Hopalong2D, "Hopalong", "Hopalong attractor - intricate web patterns");
                            attractor_button(ui, FractalType::Martin2D, "Martin", "Martin attractor - spiral/flower patterns");
                        });
                        ui.horizontal(|ui| {
                            attractor_button(ui, FractalType::Gingerbreadman2D, "Gingerbread", "Gingerbreadman - simple formula, complex output");
                            attractor_button(ui, FractalType::Chip2D, "Chip", "Chip - log/cos/atan hopalong variant");
                        });
                        ui.horizontal(|ui| {
                            attractor_button(ui, FractalType::Quadruptwo2D, "Quadruptwo", "Quadruptwo - log/sin/atan hopalong variant");
                            attractor_button(ui, FractalType::Threeply2D, "Threeply", "Threeply - trigonometric hopalong variant");
                        });

                        ui.separator();
                        ui.label("3D Fractals:");
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut params.fractal_type, FractalType::Mandelbulb3D, "Mandelbulb")
                                .on_hover_text("3D Mandelbrot with adjustable power [F1]");
                            ui.selectable_value(&mut params.fractal_type, FractalType::MengerSponge3D, "Menger Sponge")
                                .on_hover_text("Recursive cubic structure with infinite holes [F2]");
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut params.fractal_type, FractalType::SierpinskiPyramid3D, "Sierpinski Pyramid")
                                .on_hover_text("3D Sierpinski pyramid - recursive tetrahedron [F3]");
                            ui.selectable_value(&mut params.fractal_type, FractalType::SierpinskiGasket3D, "Sierpinski Gasket")
                                .on_hover_text("3D Sierpinski gasket - sphere packing structure");
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut params.fractal_type, FractalType::JuliaSet3D, "Julia 3D")
                                .on_hover_text("3D Julia set with quaternion math [F4]");
                            ui.selectable_value(&mut params.fractal_type, FractalType::Mandelbox3D, "Mandelbox")
                                .on_hover_text("Cubic folding fractal with sharp edges [F5]");
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut params.fractal_type, FractalType::OctahedralIFS3D, "Octahedron IFS")
                                .on_hover_text("Kaleidoscopic IFS with 8-fold symmetry [F6]");
                            ui.selectable_value(&mut params.fractal_type, FractalType::IcosahedralIFS3D, "Icosahedron IFS")
                                .on_hover_text("Kaleidoscopic IFS with 20-fold symmetry [F7]");
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut params.fractal_type, FractalType::ApollonianGasket3D, "Apollonian Gasket")
                                .on_hover_text("Beautiful sphere-packing fractal [F8]");
                            ui.selectable_value(&mut params.fractal_type, FractalType::Kleinian3D, "Kleinian")
                                .on_hover_text("Kleinian group fractal [F9]");
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut params.fractal_type, FractalType::HybridMandelbulbJulia3D, "Hybrid Bulb-Julia")
                                .on_hover_text("Mandelbulb and Julia set hybrid [F10]");
                            ui.selectable_value(&mut params.fractal_type, FractalType::QuaternionCubic3D, "Quaternion Cubic")
                                .on_hover_text("Cubic quaternion Julia set (z³+c)");
                        });

                        // NOTE: 3D Strange Attractors disabled - ray marching point clouds
                        // is too expensive (causes GPU timeout). Requires different rendering
                        // approach (instanced points or volumetric). See todos.md.

                        if old_type != params.fractal_type {
                            params.switch_fractal(params.fractal_type);
                            changed = true;
                        }
                    });
                self.ui_state.fractal_type_open = response.openness > 0.0;

                // Preset management section
                let response = egui::CollapsingHeader::new("Presets")
                    .default_open(self.ui_state.presets_open)
                    .show(ui, |ui| {
                        // Category filter buttons
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Category:");
                            for category in PresetCategory::all_categories() {
                                if ui.selectable_label(
                                    self.preset_category_filter == category,
                                    category.as_str()
                                ).clicked() {
                                    self.preset_category_filter = category;
                                }
                            }
                        });

                        // Search/filter box
                        ui.horizontal(|ui| {
                            ui.label("🔍 Search:");
                            ui.text_edit_singleline(&mut self.preset_search)
                                .on_hover_text("Filter presets by name or description");
                            if ui.small_button("✖")
                                .on_hover_text("Clear search")
                                .clicked() {
                                self.preset_search.clear();
                            }
                        });
                        ui.separator();

                        ui.heading("Built-in Presets");

                        let builtin_presets = PresetGallery::get_builtin_presets();
                        let search_lower = self.preset_search.to_lowercase();
                        let filtered_builtin: Vec<&Preset> = builtin_presets.iter()
                            .filter(|p| {
                                // Filter by category
                                let category_match = self.preset_category_filter == PresetCategory::All ||
                                                    p.category == self.preset_category_filter;

                                // Filter by search text
                                let search_match = if search_lower.is_empty() {
                                    true
                                } else {
                                    p.name.to_lowercase().contains(&search_lower) ||
                                    p.description.to_lowercase().contains(&search_lower)
                                };

                                category_match && search_match
                            })
                            .collect();

                        if filtered_builtin.is_empty() && !search_lower.is_empty() {
                            ui.label("No matching presets found");
                        } else {
                            egui::ScrollArea::vertical()
                                .id_salt("builtin_presets_scroll")
                                .max_height(400.0)
                                .show(ui, |ui| {
                                    for preset in filtered_builtin.iter() {
                                        ui.horizontal(|ui| {
                                            if ui.button(&preset.name).clicked() {
                                                preset_to_load = Some((*preset).clone());
                                            }
                                            ui.label(format!("- {}", preset.description));
                                        });
                                    }
                                });
                        }

                        ui.separator();
                        ui.heading("Save Current as Preset");

                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.preset_name);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Description:");
                            ui.text_edit_singleline(&mut self.preset_description);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Category:");
                            egui::ComboBox::from_id_salt("preset_category_combo")
                                .selected_text(self.preset_category.as_str())
                                .show_ui(ui, |ui| {
                                    for category in PresetCategory::all_categories() {
                                        if category != PresetCategory::All {
                                            ui.selectable_value(
                                                &mut self.preset_category,
                                                category,
                                                category.as_str()
                                            );
                                        }
                                    }
                                });
                        });

                        if ui.button("Save Preset").clicked() && !self.preset_name.is_empty() {
                            let preset = Preset::from_current(
                                self.preset_name.clone(),
                                self.preset_description.clone(),
                                self.preset_category,
                                params,
                                camera_pos,
                                camera_target
                            );

                            // Sanitize filename
                            let filename = self.preset_name.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");

                            if let Err(e) = PresetGallery::save_preset(&preset, &filename) {
                                eprintln!("Failed to save preset: {}", e);
                            } else {
                                // Refresh user presets list
                                self.user_presets = PresetGallery::list_user_presets().unwrap_or_default();
                                self.preset_name.clear();
                                self.preset_description.clear();
                            }
                        }

                        // Refresh user presets list periodically
                        if self.last_preset_list_update.elapsed().as_secs() > 2 {
                            self.user_presets = PresetGallery::list_user_presets().unwrap_or_default();
                            self.last_preset_list_update = web_time::Instant::now();
                        }

                        if !self.user_presets.is_empty() {
                            ui.separator();
                            ui.heading("User Presets");

                            let filtered_user: Vec<&String> = self.user_presets.iter()
                                .filter(|p| {
                                    if search_lower.is_empty() {
                                        true
                                    } else {
                                        p.to_lowercase().contains(&search_lower)
                                    }
                                })
                                .collect();

                            if filtered_user.is_empty() && !search_lower.is_empty() {
                                ui.label("No matching user presets found");
                            } else {
                                let mut refresh_presets = false;
                                egui::ScrollArea::vertical()
                                    .id_salt("user_presets_scroll")
                                    .max_height(150.0)
                                    .show(ui, |ui| {
                                        for preset_name in filtered_user.iter() {
                                            ui.horizontal(|ui| {
                                                if ui.button(*preset_name).clicked() {
                                                    println!("User preset button clicked: {}", preset_name);
                                                    match PresetGallery::load_preset(preset_name) {
                                                        Ok(preset) => {
                                                            println!("Preset loaded successfully: {}", preset.name);
                                                            preset_to_load = Some(preset);
                                                        }
                                                        Err(e) => {
                                                            eprintln!("Failed to load preset '{}': {}", preset_name, e);
                                                        }
                                                    }
                                                }
                                                // Add delete button
                                                if ui.small_button("🗑").on_hover_text("Delete preset").clicked() {
                                                    #[cfg(not(target_arch = "wasm32"))]
                                                    if let Err(e) = PresetGallery::delete_preset(preset_name) {
                                                        eprintln!("Failed to delete preset: {}", e);
                                                    } else {
                                                        refresh_presets = true;
                                                    }
                                                }
                                            });
                                        }
                                    });
                                if refresh_presets {
                                    self.user_presets = PresetGallery::list_user_presets().unwrap_or_default();
                                }
                            }
                        }

                        ui.separator();
                        ui.heading("Import / Export");
                        ui.horizontal(|ui| {
                            if ui.button("📥 Export to JSON")
                                .on_hover_text("Export current settings to a JSON file")
                                .clicked()
                            {
                                let settings = params.to_settings();
                                if let Err(e) = PresetGallery::export_to_json(&settings, camera_pos.to_array(), camera_target.to_array()) {
                                    eprintln!("Failed to export settings: {}", e);
                                } else {
                                    println!("Settings exported successfully");
                                }
                            }

                            if ui.button("📤 Import from JSON")
                                .on_hover_text("Import settings from a JSON file")
                                .clicked()
                            {
                                match PresetGallery::import_from_json() {
                                    Ok(preset) => {
                                        preset_to_load = Some(preset);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to import settings: {}", e);
                                    }
                                }
                            }
                        });
                    });
                self.ui_state.presets_open = response.openness > 0.0;

                let response = egui::CollapsingHeader::new("Color & Visualization")
                    .default_open(self.ui_state.color_viz_open)
                    .show(ui, |ui| {
                        ui.label("Color Mode:")
                            .on_hover_text("Choose how colors are applied to the fractal");
                        changed |= egui::ComboBox::from_id_salt("color_mode")
                            .selected_text(match params.color_mode {
                                crate::fractal::ColorMode::Palette => "Palette",
                                crate::fractal::ColorMode::RaySteps => "Ray Steps / Iterations",
                                crate::fractal::ColorMode::Normals => "Normals (3D)",
                                crate::fractal::ColorMode::OrbitTrapXYZ => "Orbit Trap XYZ",
                                crate::fractal::ColorMode::OrbitTrapRadial => "Orbit Trap Radial",
                                crate::fractal::ColorMode::WorldPosition => "World Position",
                                crate::fractal::ColorMode::LocalPosition => "Local Position",
                                crate::fractal::ColorMode::AmbientOcclusion => "Ambient Occlusion (3D)",
                                crate::fractal::ColorMode::PerChannel => "Per-Channel (Custom RGB)",
                                crate::fractal::ColorMode::DistanceField => "🔍 Distance Field (Debug)",
                                crate::fractal::ColorMode::Depth => "🔍 Depth (Debug)",
                                crate::fractal::ColorMode::Convergence => "🔍 Convergence (Debug)",
                                crate::fractal::ColorMode::LightingOnly => "🔍 Lighting Only (Debug)",
                                crate::fractal::ColorMode::ShadowMap => "🔍 Shadow Map (Debug)",
                                crate::fractal::ColorMode::CameraDistanceLOD => "🔍 Camera Distance LOD (Debug)",
                                crate::fractal::ColorMode::DistanceGrayscale => "🔍 Distance Grayscale (Debug)",
                            })
                            .show_ui(ui, |ui| {
                                let mut changed_local = false;
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::Palette, "Palette").changed();
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::RaySteps, "Ray Steps / Iterations").changed();
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::Normals, "Normals (3D)").changed();
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::OrbitTrapXYZ, "Orbit Trap XYZ").changed();
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::OrbitTrapRadial, "Orbit Trap Radial").changed();
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::WorldPosition, "World Position").changed();
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::LocalPosition, "Local Position").changed();
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::AmbientOcclusion, "Ambient Occlusion (3D)").changed();
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::PerChannel, "Per-Channel (Custom RGB)")
                                    .on_hover_text("Map different data sources to R, G, and B channels independently")
                                    .changed();
                                ui.separator();
                                ui.label("Debug Modes:");
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::DistanceField, "🔍 Distance Field")
                                    .on_hover_text("Visualize distance field complexity - shows ray marching step density")
                                    .changed();
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::Depth, "🔍 Depth")
                                    .on_hover_text("Visualize distance from camera")
                                    .changed();
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::Convergence, "🔍 Convergence")
                                    .on_hover_text("Visualize escape time / convergence speed")
                                    .changed();
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::LightingOnly, "🔍 Lighting Only")
                                    .on_hover_text("Show only lighting without fractal coloring")
                                    .changed();
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::ShadowMap, "🔍 Shadow Map")
                                    .on_hover_text("Visualize shadow values (3D)")
                                    .changed();
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::CameraDistanceLOD, "🔍 Camera Distance LOD")
                                    .on_hover_text("Visualize distance from camera using LOD zone colors (3D)")
                                    .changed();
                                changed_local |= ui.selectable_value(&mut params.color_mode, crate::fractal::ColorMode::DistanceGrayscale, "🔍 Distance Grayscale")
                                    .on_hover_text("Visualize raw distance from camera as brightness (3D)")
                                    .changed();
                                changed_local
                            })
                            .inner.unwrap_or(false);

                        // Show color key for debug visualization modes
                        match params.color_mode {
                            crate::fractal::ColorMode::DistanceField => {
                                ui.separator();
                                ui.label("🔍 Color Key - Distance Field:");
                                ui.horizontal(|ui| {
                                    // Draw gradient bar
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(200.0, 20.0),
                                        egui::Sense::hover()
                                    );
                                    let n = 50;
                                    for i in 0..n {
                                        let t = i as f32 / n as f32;
                                        let color = egui::Color32::from_rgb(
                                            (t * 255.0) as u8,
                                            (t * 0.5 * 255.0) as u8,
                                            ((1.0 - t) * 255.0) as u8,
                                        );
                                        let x = rect.min.x + (rect.width() * i as f32 / n as f32);
                                        let segment_rect = egui::Rect::from_min_size(
                                            egui::pos2(x, rect.min.y),
                                            egui::vec2(rect.width() / n as f32, rect.height())
                                        );
                                        ui.painter().rect_filled(segment_rect, 0.0, color);
                                    }
                                });
                                ui.label("<- Simple/Open Areas (Blue) | Complex/Tight Areas (Red) ->");
                            }
                            crate::fractal::ColorMode::Depth => {
                                ui.separator();
                                ui.label("🔍 Color Key - Depth:");
                                ui.horizontal(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(200.0, 20.0),
                                        egui::Sense::hover()
                                    );
                                    let n = 50;
                                    for i in 0..n {
                                        let t = i as f32 / n as f32;
                                        let color = egui::Color32::from_rgb(
                                            ((1.0 - t) * 255.0) as u8,
                                            (t * 0.5 * 255.0) as u8,
                                            (t * 255.0) as u8,
                                        );
                                        let x = rect.min.x + (rect.width() * i as f32 / n as f32);
                                        let segment_rect = egui::Rect::from_min_size(
                                            egui::pos2(x, rect.min.y),
                                            egui::vec2(rect.width() / n as f32, rect.height())
                                        );
                                        ui.painter().rect_filled(segment_rect, 0.0, color);
                                    }
                                });
                                ui.label("<- Near Camera (Bright) | Far from Camera (Dark) ->");
                            }
                            crate::fractal::ColorMode::Convergence => {
                                ui.separator();
                                ui.label("🔍 Color Key - Convergence:");
                                ui.horizontal(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(200.0, 20.0),
                                        egui::Sense::hover()
                                    );
                                    let n = 50;
                                    for i in 0..n {
                                        let t = i as f32 / n as f32;
                                        let color = egui::Color32::from_rgb(
                                            (t * 255.0) as u8,
                                            ((1.0 - t) * 255.0) as u8,
                                            ((t * (1.0 - t) * 4.0) * 255.0) as u8,
                                        );
                                        let x = rect.min.x + (rect.width() * i as f32 / n as f32);
                                        let segment_rect = egui::Rect::from_min_size(
                                            egui::pos2(x, rect.min.y),
                                            egui::vec2(rect.width() / n as f32, rect.height())
                                        );
                                        ui.painter().rect_filled(segment_rect, 0.0, color);
                                    }
                                });
                                ui.label("<- Slow Convergence (Green) | Fast Convergence (Red) ->");
                            }
                            crate::fractal::ColorMode::ShadowMap => {
                                ui.separator();
                                ui.label("🔍 Color Key - Shadow Map:");
                                ui.horizontal(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(200.0, 20.0),
                                        egui::Sense::hover()
                                    );
                                    let n = 50;
                                    for i in 0..n {
                                        let t = i as f32 / n as f32;
                                        let gray = (t * 255.0) as u8;
                                        let color = egui::Color32::from_rgb(gray, gray, gray);
                                        let x = rect.min.x + (rect.width() * i as f32 / n as f32);
                                        let segment_rect = egui::Rect::from_min_size(
                                            egui::pos2(x, rect.min.y),
                                            egui::vec2(rect.width() / n as f32, rect.height())
                                        );
                                        ui.painter().rect_filled(segment_rect, 0.0, color);
                                    }
                                });
                                ui.label("<- In Shadow (Dark) | Fully Lit (Bright) ->");
                            }
                            crate::fractal::ColorMode::LightingOnly => {
                                ui.separator();
                                ui.label("🔍 Lighting Only Mode:");
                                ui.label("Shows pure lighting/shadows on neutral gray surface");
                            }
                            _ => {}
                        }

                        // Show palette controls for modes that use the palette
                        if params.color_mode == crate::fractal::ColorMode::Palette ||
                           params.color_mode == crate::fractal::ColorMode::OrbitTrapXYZ ||
                           params.color_mode == crate::fractal::ColorMode::OrbitTrapRadial {
                            ui.separator();
                            ui.label("Palette Selection:")
                                .on_hover_text("Choose from 6 built-in color palettes [P to cycle]");
                            ui.horizontal(|ui| {
                                if ui.button("◀ Previous").on_hover_text("Switch to previous palette").clicked() {
                                    params.prev_palette();
                                    self.show_toast(format!("Palette: {}", params.palette.name));
                                    changed = true;
                                }
                                ui.label(params.palette.name);
                                if ui.button("Next ▶").on_hover_text("Switch to next palette [P]").clicked() {
                                    params.next_palette();
                                    self.show_toast(format!("Palette: {}", params.palette.name));
                                    changed = true;
                                }
                            });

                            // Show palette colors
                            ui.horizontal(|ui| {
                                for color in &params.palette.colors {
                                    let color32 = egui::Color32::from_rgb(
                                        (color.x * 255.0) as u8,
                                        (color.y * 255.0) as u8,
                                        (color.z * 255.0) as u8,
                                    );
                                    let (rect, _response) = ui.allocate_exact_size(
                                        egui::vec2(20.0, 20.0),
                                        egui::Sense::hover()
                                    );
                                    ui.painter().rect_filled(rect, 2.0, color32);
                                }
                            });

                            // Palette Animation Controls
                            ui.separator();
                            ui.horizontal(|ui| {
                                changed |= ui.checkbox(&mut self.palette_animation_enabled, "Animate Palette")
                                    .on_hover_text("Slowly rotate through palette colors for mesmerizing effects")
                                    .changed();
                            });

                            if self.palette_animation_enabled {
                                ui.horizontal(|ui| {
                                    ui.label("Speed:");
                                    changed |= ui.add(egui::Slider::new(&mut self.palette_animation_speed, 0.01..=1.0)
                                        .text(""))
                                        .on_hover_text("Animation speed - higher values rotate faster")
                                        .changed();
                                });

                                ui.horizontal(|ui| {
                                    changed |= ui.checkbox(&mut self.palette_animation_reverse, "Reverse Direction")
                                        .on_hover_text("Reverse the animation direction")
                                        .changed();
                                });
                            }

                            // Show orbit trap scale slider for orbit trap modes
                            if params.color_mode == crate::fractal::ColorMode::OrbitTrapXYZ ||
                               params.color_mode == crate::fractal::ColorMode::OrbitTrapRadial {
                                ui.separator();
                                changed |= ui.add(egui::Slider::new(&mut params.orbit_trap_scale, 0.1..=5.0)
                                    .text("Orbit Trap Scale"))
                                    .on_hover_text("Scale factor for orbit trap coloring - affects color variation")
                                    .changed();
                            }

                            // Per-Channel Controls
                            if params.color_mode == crate::fractal::ColorMode::PerChannel {
                                ui.separator();
                                ui.label("Channel Mapping:")
                                    .on_hover_text("Map different data sources to R, G, and B channels");

                                // Red channel
                                ui.horizontal(|ui| {
                                    ui.label("R:");
                                    changed |= egui::ComboBox::from_id_salt("channel_r")
                                        .selected_text(format!("{:?}", params.channel_r))
                                        .show_ui(ui, |ui| {
                                            let mut ch = false;
                                            ch |= ui.selectable_value(&mut params.channel_r, crate::fractal::ChannelSource::Iterations, "Iterations").changed();
                                            ch |= ui.selectable_value(&mut params.channel_r, crate::fractal::ChannelSource::Distance, "Distance").changed();
                                            ch |= ui.selectable_value(&mut params.channel_r, crate::fractal::ChannelSource::PositionX, "Position X").changed();
                                            ch |= ui.selectable_value(&mut params.channel_r, crate::fractal::ChannelSource::PositionY, "Position Y").changed();
                                            ch |= ui.selectable_value(&mut params.channel_r, crate::fractal::ChannelSource::PositionZ, "Position Z").changed();
                                            ch |= ui.selectable_value(&mut params.channel_r, crate::fractal::ChannelSource::Normal, "Normal").changed();
                                            ch |= ui.selectable_value(&mut params.channel_r, crate::fractal::ChannelSource::AO, "AO").changed();
                                            ch |= ui.selectable_value(&mut params.channel_r, crate::fractal::ChannelSource::Constant, "Constant (0)").changed();
                                            ch
                                        })
                                        .inner.unwrap_or(false);
                                });

                                // Green channel
                                ui.horizontal(|ui| {
                                    ui.label("G:");
                                    changed |= egui::ComboBox::from_id_salt("channel_g")
                                        .selected_text(format!("{:?}", params.channel_g))
                                        .show_ui(ui, |ui| {
                                            let mut ch = false;
                                            ch |= ui.selectable_value(&mut params.channel_g, crate::fractal::ChannelSource::Iterations, "Iterations").changed();
                                            ch |= ui.selectable_value(&mut params.channel_g, crate::fractal::ChannelSource::Distance, "Distance").changed();
                                            ch |= ui.selectable_value(&mut params.channel_g, crate::fractal::ChannelSource::PositionX, "Position X").changed();
                                            ch |= ui.selectable_value(&mut params.channel_g, crate::fractal::ChannelSource::PositionY, "Position Y").changed();
                                            ch |= ui.selectable_value(&mut params.channel_g, crate::fractal::ChannelSource::PositionZ, "Position Z").changed();
                                            ch |= ui.selectable_value(&mut params.channel_g, crate::fractal::ChannelSource::Normal, "Normal").changed();
                                            ch |= ui.selectable_value(&mut params.channel_g, crate::fractal::ChannelSource::AO, "AO").changed();
                                            ch |= ui.selectable_value(&mut params.channel_g, crate::fractal::ChannelSource::Constant, "Constant (0)").changed();
                                            ch
                                        })
                                        .inner.unwrap_or(false);
                                });

                                // Blue channel
                                ui.horizontal(|ui| {
                                    ui.label("B:");
                                    changed |= egui::ComboBox::from_id_salt("channel_b")
                                        .selected_text(format!("{:?}", params.channel_b))
                                        .show_ui(ui, |ui| {
                                            let mut ch = false;
                                            ch |= ui.selectable_value(&mut params.channel_b, crate::fractal::ChannelSource::Iterations, "Iterations").changed();
                                            ch |= ui.selectable_value(&mut params.channel_b, crate::fractal::ChannelSource::Distance, "Distance").changed();
                                            ch |= ui.selectable_value(&mut params.channel_b, crate::fractal::ChannelSource::PositionX, "Position X").changed();
                                            ch |= ui.selectable_value(&mut params.channel_b, crate::fractal::ChannelSource::PositionY, "Position Y").changed();
                                            ch |= ui.selectable_value(&mut params.channel_b, crate::fractal::ChannelSource::PositionZ, "Position Z").changed();
                                            ch |= ui.selectable_value(&mut params.channel_b, crate::fractal::ChannelSource::Normal, "Normal").changed();
                                            ch |= ui.selectable_value(&mut params.channel_b, crate::fractal::ChannelSource::AO, "AO").changed();
                                            ch |= ui.selectable_value(&mut params.channel_b, crate::fractal::ChannelSource::Constant, "Constant (0)").changed();
                                            ch
                                        })
                                        .inner.unwrap_or(false);
                                });
                            }

                            // Custom Palette Editor
                            ui.separator();
                            ui.collapsing("Custom Palette Editor", |ui| {
                                ui.label("Create your own color palettes")
                                    .on_hover_text("Design custom 8-color gradients");

                                // Color picker for each of the 8 palette colors
                                for i in 0..8 {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("Color {}:", i + 1));
                                        if ui.color_edit_button_rgb(&mut self.custom_palette_colors[i])
                                            .on_hover_text(format!("Edit color {} of the palette", i + 1))
                                            .changed() {
                                            // Color changed, could auto-update preview if needed
                                        }
                                    });
                                }

                                ui.separator();
                                ui.label("Preview:");
                                ui.horizontal(|ui| {
                                    for color in &self.custom_palette_colors {
                                        let color32 = egui::Color32::from_rgb(
                                            (color[0] * 255.0) as u8,
                                            (color[1] * 255.0) as u8,
                                            (color[2] * 255.0) as u8,
                                        );
                                        let (rect, _response) = ui.allocate_exact_size(
                                            egui::vec2(20.0, 20.0),
                                            egui::Sense::hover()
                                        );
                                        ui.painter().rect_filled(rect, 2.0, color32);
                                    }
                                });

                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label("Palette Name:");
                                    ui.text_edit_singleline(&mut self.custom_palette_name);
                                });

                                ui.horizontal(|ui| {
                                    if ui.button("💾 Save Custom Palette")
                                        .on_hover_text("Save this palette for later use")
                                        .clicked()
                                        && !self.custom_palette_name.is_empty()
                                    {
                                        let colors = [
                                            Vec3::from_array(self.custom_palette_colors[0]),
                                            Vec3::from_array(self.custom_palette_colors[1]),
                                            Vec3::from_array(self.custom_palette_colors[2]),
                                            Vec3::from_array(self.custom_palette_colors[3]),
                                            Vec3::from_array(self.custom_palette_colors[4]),
                                            Vec3::from_array(self.custom_palette_colors[5]),
                                            Vec3::from_array(self.custom_palette_colors[6]),
                                            Vec3::from_array(self.custom_palette_colors[7]),
                                        ];
                                        let custom_palette = CustomPalette::new(
                                            self.custom_palette_name.clone(),
                                            colors,
                                        );

                                        // Sanitize filename
                                        let filename = self.custom_palette_name.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");

                                        if let Err(e) = CustomPaletteGallery::save_palette(&custom_palette, &filename) {
                                            eprintln!("Failed to save custom palette: {}", e);
                                        } else {
                                            // Refresh custom palette list
                                            self.custom_palettes = CustomPaletteGallery::list_palettes().unwrap_or_default();
                                            self.custom_palette_name.clear();
                                        }
                                    }

                                    if ui.button("📋 Copy from Current")
                                        .on_hover_text("Copy colors from the currently selected palette")
                                        .clicked() {
                                        for i in 0..8 {
                                            self.custom_palette_colors[i] = params.palette.colors[i].to_array();
                                        }
                                    }
                                });

                                // Import palette from file
                                ui.separator();
                                ui.label("Import Palette:")
                                    .on_hover_text("Import from .pal (JASC-PAL) or plain text RGB files");
                                ui.horizontal(|ui| {
                                    ui.label("File path:");
                                    ui.text_edit_singleline(&mut self.palette_import_path)
                                        .on_hover_text("Enter path to .pal file or text file with RGB values (one per line)");
                                });

                                ui.horizontal(|ui| {
                                    if ui.button("📥 Import")
                                        .on_hover_text("Import palette from the specified file")
                                        .clicked() {
                                        let path = std::path::Path::new(&self.palette_import_path);
                                        match CustomPalette::from_pal_file(path) {
                                            Ok(imported) => {
                                                // Apply imported colors to editor
                                                let (_name, colors) = imported.to_color_palette();
                                                for (i, color) in colors.iter().enumerate() {
                                                    self.custom_palette_colors[i] = color.to_array();
                                                }
                                                // Pre-fill the name
                                                if self.custom_palette_name.is_empty() {
                                                    self.custom_palette_name = imported.name.clone();
                                                }
                                                self.palette_import_message = Some(format!("✓ Imported '{}' successfully!", imported.name));
                                            }
                                            Err(e) => {
                                                self.palette_import_message = Some(format!("✗ Error: {}", e));
                                            }
                                        }
                                    }

                                    if ui.small_button("Clear")
                                        .on_hover_text("Clear the import path")
                                        .clicked() {
                                        self.palette_import_path.clear();
                                        self.palette_import_message = None;
                                    }
                                });

                                // Show import status message
                                if let Some(ref msg) = self.palette_import_message {
                                    ui.label(msg);
                                }

                                ui.label("Supported formats:")
                                    .on_hover_text("JASC-PAL (.pal), plain text RGB (0-255 or 0.0-1.0)");
                                ui.label("  • JASC-PAL: Standard .pal format")
                                    .on_hover_text("First line: JASC-PAL, Second: 0100, Third: color count, then RGB values (0-255)");
                                ui.label("  • Plain text: One RGB per line")
                                    .on_hover_text("Format: R G B (space or comma separated, 0-255 or 0.0-1.0)");

                                // Refresh custom palette list periodically
                                if self.last_custom_palette_list_update.elapsed().as_secs() > 2 {
                                    self.custom_palettes = CustomPaletteGallery::list_palettes().unwrap_or_default();
                                    self.last_custom_palette_list_update = web_time::Instant::now();
                                }

                                if !self.custom_palettes.is_empty() {
                                    ui.separator();
                                    ui.label("Saved Custom Palettes:")
                                        .on_hover_text("Click to load, right-click to delete");

                                    egui::ScrollArea::vertical()
                                        .id_salt("custom_palettes_scroll")
                                        .max_height(120.0)
                                        .show(ui, |ui| {
                                            let custom_palettes_clone = self.custom_palettes.clone();
                                            for palette_name in custom_palettes_clone.iter() {
                                                ui.horizontal(|ui| {
                                                    if ui.button(palette_name)
                                                        .on_hover_text("Click to load this custom palette")
                                                        .clicked() {
                                                        if let Ok(custom_palette) = CustomPaletteGallery::load_palette(palette_name) {
                                                            // Apply the custom palette to the current params
                                                            let (_name, colors) = custom_palette.to_color_palette();
                                                            params.palette.colors = colors;
                                                            changed = true;
                                                        }
                                                    }
                                                    if ui.small_button("🗑")
                                                        .on_hover_text("Delete this custom palette")
                                                        .clicked() {
                                                        self.custom_palette_to_delete = Some(palette_name.clone());
                                                    }
                                                });
                                            }
                                        });
                                }

                                // Handle custom palette deletion
                                if let Some(ref palette_name) = self.custom_palette_to_delete {
                                    if let Err(e) = CustomPaletteGallery::delete_palette(palette_name) {
                                        eprintln!("Failed to delete custom palette: {}", e);
                                    }
                                    self.custom_palettes = CustomPaletteGallery::list_palettes().unwrap_or_default();
                                    self.custom_palette_to_delete = None;
                                }
                            });
                        }
                    });
                self.ui_state.color_viz_open = response.openness > 0.0;

                match params.render_mode {
                    crate::fractal::RenderMode::TwoD => {
                        let response = egui::CollapsingHeader::new("2D Parameters")
                            .default_open(self.ui_state.params_2d_open)
                            .show(ui, |ui| {
                                // Hide iterations slider for Collatz (doesn't affect it)
                                // Hide max iterations for strange attractors (they use accumulation mode)
                                // and for Collatz which doesn't use iterations
                                if params.fractal_type != FractalType::Collatz2D && !params.fractal_type.is_2d_attractor() {
                                    changed |= ui.add(egui::Slider::new(&mut params.max_iterations, 1..=1024)
                                        .text("Max Iterations")
                                        .logarithmic(true))
                                        .on_hover_text("Number of iterations before considering a point escaped\nHigher = more detail but slower")
                                        .changed();
                                }

                                if params.fractal_type == FractalType::Julia2D {
                                    ui.label("Julia Constant (C):")
                                        .on_hover_text("The complex constant used in Julia set formula");
                                    changed |= ui.add(egui::Slider::new(&mut params.julia_c[0], -2.0..=2.0)
                                        .text("Real"))
                                        .on_hover_text("Real component of Julia constant")
                                        .changed();
                                    changed |= ui.add(egui::Slider::new(&mut params.julia_c[1], -2.0..=2.0)
                                        .text("Imaginary"))
                                        .on_hover_text("Imaginary component of Julia constant")
                                        .changed();
                                }

                                ui.label(format!("Center: ({:.6}, {:.6})", params.center_2d[0], params.center_2d[1]))
                                    .on_hover_text("Current view center (drag to pan)");
                                ui.label(format!("Zoom: {:.4}", params.zoom_2d))
                                    .on_hover_text("Current zoom level (scroll to zoom)");
                                if ui.button("Reset View").on_hover_text("Reset center and zoom [R]").clicked() {
                                    params.center_2d = [0.0, 0.0];
                                    params.zoom_2d = 1.0;
                                    changed = true;
                                }

                                // Accumulation controls for strange attractors (always enabled)
                                if params.fractal_type.is_2d_attractor() {
                                    // Ensure accumulation is always enabled for strange attractors
                                    params.attractor_accumulation_enabled = true;

                                    ui.separator();
                                    ui.label("🎯 Accumulation Settings");

                                    changed |= ui.add(egui::Slider::new(&mut params.attractor_iterations_per_frame, 10_000..=1_000_000)
                                        .text("Iterations/Frame")
                                        .logarithmic(true))
                                        .on_hover_text("Number of orbit iterations per frame\nHigher = faster accumulation, lower FPS")
                                        .changed();

                                    changed |= ui.add(egui::Slider::new(&mut params.attractor_log_scale, 0.5..=5.0)
                                        .text("Density Scale"))
                                        .on_hover_text("Controls saturation point (hits needed for white)\n0.5 = ~30 hits, 1.0 = ~100, 2.0 = ~1000, 3.0 = ~10k")
                                        .changed();

                                    ui.label(format!("Total Iterations: {}", params.attractor_total_iterations));

                                    if ui.button("Clear Accumulation").on_hover_text("Reset accumulated density").clicked() {
                                        params.attractor_pending_clear = true;
                                        params.attractor_total_iterations = 0;
                                        changed = true;
                                    }

                                    // Attractor-specific parameter controls
                                    ui.separator();
                                    ui.label("🔧 Attractor Parameters");

                                    match params.fractal_type {
                                        FractalType::Hopalong2D => {
                                            // Hopalong: a, b, c parameters (range 0-10 typical)
                                            changed |= ui.add(egui::Slider::new(&mut params.julia_c[0], -10.0..=10.0)
                                                .text("a"))
                                                .on_hover_text("Hopalong parameter a")
                                                .changed();
                                            changed |= ui.add(egui::Slider::new(&mut params.julia_c[1], -10.0..=10.0)
                                                .text("b"))
                                                .on_hover_text("Hopalong parameter b")
                                                .changed();
                                            changed |= ui.add(egui::Slider::new(&mut params.power, -10.0..=10.0)
                                                .text("c"))
                                                .on_hover_text("Hopalong parameter c")
                                                .changed();
                                        }
                                        FractalType::Martin2D => {
                                            // Martin: just parameter a
                                            changed |= ui.add(egui::Slider::new(&mut params.julia_c[0], -10.0..=10.0)
                                                .text("a"))
                                                .on_hover_text("Martin parameter a (pi produces classic pattern)")
                                                .changed();
                                        }
                                        FractalType::Gingerbreadman2D => {
                                            ui.label("No adjustable parameters");
                                        }
                                        FractalType::Chip2D => {
                                            // Chip: a, b, c parameters
                                            changed |= ui.add(egui::Slider::new(&mut params.julia_c[0], -100.0..=100.0)
                                                .text("a"))
                                                .on_hover_text("Chip parameter a")
                                                .changed();
                                            changed |= ui.add(egui::Slider::new(&mut params.julia_c[1], -100.0..=100.0)
                                                .text("b"))
                                                .on_hover_text("Chip parameter b")
                                                .changed();
                                            changed |= ui.add(egui::Slider::new(&mut params.power, -100.0..=100.0)
                                                .text("c"))
                                                .on_hover_text("Chip parameter c")
                                                .changed();
                                        }
                                        FractalType::Quadruptwo2D => {
                                            // Quadruptwo: a, b, c parameters
                                            changed |= ui.add(egui::Slider::new(&mut params.julia_c[0], -100.0..=100.0)
                                                .text("a"))
                                                .on_hover_text("Quadruptwo parameter a (default: 34)")
                                                .changed();
                                            changed |= ui.add(egui::Slider::new(&mut params.julia_c[1], -100.0..=100.0)
                                                .text("b"))
                                                .on_hover_text("Quadruptwo parameter b (default: 1)")
                                                .changed();
                                            changed |= ui.add(egui::Slider::new(&mut params.power, -100.0..=100.0)
                                                .text("c"))
                                                .on_hover_text("Quadruptwo parameter c (default: 5)")
                                                .changed();
                                        }
                                        FractalType::Threeply2D => {
                                            // Threeply: a, b, c parameters
                                            changed |= ui.add(egui::Slider::new(&mut params.julia_c[0], -100.0..=100.0)
                                                .text("a"))
                                                .on_hover_text("Threeply parameter a (default: -55)")
                                                .changed();
                                            changed |= ui.add(egui::Slider::new(&mut params.julia_c[1], -100.0..=100.0)
                                                .text("b"))
                                                .on_hover_text("Threeply parameter b (default: -1)")
                                                .changed();
                                            changed |= ui.add(egui::Slider::new(&mut params.power, -100.0..=100.0)
                                                .text("c"))
                                                .on_hover_text("Threeply parameter c (default: -42)")
                                                .changed();
                                        }
                                        _ => {}
                                    }

                                    // Reset to defaults button
                                    if ui.button("Reset Parameters").on_hover_text("Reset attractor parameters to defaults").clicked() {
                                        params.switch_fractal(params.fractal_type);
                                        params.attractor_pending_clear = true;
                                        params.attractor_total_iterations = 0;
                                        changed = true;
                                    }
                                }
                            });
                        self.ui_state.params_2d_open = response.openness > 0.0;
                    }
                    crate::fractal::RenderMode::ThreeD => {
                        let response = egui::CollapsingHeader::new("3D Parameters")
                            .default_open(self.ui_state.params_3d_open)
                            .show(ui, |ui| {
                                // Scale control for all 3D fractals
                                ui.label("Fractal Shape:")
                                    .on_hover_text("Control the size and proportions of the fractal");
                                changed |= ui.add(egui::Slider::new(&mut params.fractal_scale, 0.5..=5.0)
                                    .text("Scale"))
                                    .on_hover_text("Overall size of the fractal structure")
                                    .changed();

                                // Mandelbulb-specific parameters
                                if params.fractal_type == FractalType::Mandelbulb3D {
                                    changed |= ui.add(egui::Slider::new(&mut params.power, 2.0..=16.0)
                                        .text("Power"))
                                        .on_hover_text("Mandelbulb power (8 is classic, higher = more detail)")
                                        .changed();
                                }

                                // Julia 3D-specific parameters
                                if params.fractal_type == FractalType::JuliaSet3D {
                                    ui.label("Julia Constant (C):")
                                        .on_hover_text("Quaternion constant for 3D Julia set");
                                    changed |= ui.add(egui::Slider::new(&mut params.julia_c[0], -2.0..=2.0)
                                        .text("Real"))
                                        .on_hover_text("Real component of quaternion constant")
                                        .changed();
                                    changed |= ui.add(egui::Slider::new(&mut params.julia_c[1], -2.0..=2.0)
                                        .text("Imaginary"))
                                        .on_hover_text("Imaginary component of quaternion constant")
                                        .changed();
                                }

                                // Iterations control for specific 3D fractals
                                if matches!(params.fractal_type,
                                    FractalType::MengerSponge3D |
                                    FractalType::SierpinskiPyramid3D) {
                                    changed |= ui.add(egui::Slider::new(&mut params.max_iterations, 1..=20)
                                        .text("Iterations"))
                                        .on_hover_text("Recursion depth (higher = more detail and smaller features)")
                                        .changed();
                                }

                                if params.fractal_type == FractalType::QuaternionCubic3D {
                                    changed |= ui.add(egui::Slider::new(&mut params.max_iterations, 1..=64)
                                        .text("Iterations"))
                                        .on_hover_text("Number of quaternion iterations (higher = more detail, slower)")
                                        .changed();
                                }

                                // Advanced fractal shape controls
                                if matches!(params.fractal_type,
                                    FractalType::Mandelbox3D |
                                    FractalType::OctahedralIFS3D |
                                    FractalType::IcosahedralIFS3D |
                                    FractalType::ApollonianGasket3D) {
                                    ui.separator();
                                    ui.label("Advanced Shape:")
                                        .on_hover_text("Fine-tune fractal geometry with folding parameters");
                                    changed |= ui.add(egui::Slider::new(&mut params.fractal_fold, 0.1..=3.0)
                                        .text("Fold"))
                                        .on_hover_text("Fold strength - affects how space is bent")
                                        .changed();

                                    // Min Radius for fractals with sphere folding
                                    if matches!(params.fractal_type,
                                        FractalType::Mandelbox3D |
                                        FractalType::ApollonianGasket3D) {
                                        changed |= ui.add(egui::Slider::new(&mut params.fractal_min_radius, 0.1..=2.0)
                                            .text("Min Radius"))
                                            .on_hover_text("Minimum sphere folding radius - affects inner details")
                                            .changed();
                                    }
                                }
                            });
                        self.ui_state.params_3d_open = response.openness > 0.0;

                        let response = egui::CollapsingHeader::new("Ray Marching")
                            .default_open(self.ui_state.ray_marching_open)
                            .show(ui, |ui| {
                                changed |= ui.checkbox(&mut params.use_adaptive_step, "Adaptive Step Size")
                                    .on_hover_text("Use distance field for step size (recommended)\nDisable for fixed steps")
                                    .changed();

                                if params.use_adaptive_step {
                                    changed |= ui.add(egui::Slider::new(&mut params.step_multiplier, 0.1..=2.0)
                                        .text("Step Multiplier"))
                                        .on_hover_text("Adaptive step multiplier - lower = more accurate but slower")
                                        .changed();
                                } else {
                                    changed |= ui.add(egui::Slider::new(&mut params.fixed_step_size, 0.01..=0.5)
                                        .text("Fixed Step Size"))
                                        .on_hover_text("Fixed step size in world units - smaller = more accurate")
                                        .changed();
                                }

                                changed |= ui.add(egui::Slider::new(&mut params.max_steps, 32..=512)
                                    .text("Max Steps"))
                                    .on_hover_text("Maximum ray marching steps - higher = better quality but slower")
                                    .changed();

                                changed |= ui.add(egui::Slider::new(&mut params.min_distance, 0.0001..=0.01)
                                    .text("Min Distance")
                                    .logarithmic(true))
                                    .on_hover_text("Distance threshold for surface hit detection\nSmaller = finer details")
                                    .changed();

                                changed |= ui.add(egui::Slider::new(&mut params.max_distance, 10.0..=200.0)
                                    .text("Max Distance"))
                                    .on_hover_text("Maximum ray marching distance before giving up")
                                    .changed();
                            });
                        self.ui_state.ray_marching_open = response.openness > 0.0;

                        let response = egui::CollapsingHeader::new("Camera")
                            .default_open(self.ui_state.camera_open)
                            .show(ui, |ui| {
                                changed |= ui.add(egui::Slider::new(&mut params.camera_speed, 0.1..=10.0)
                                    .text("Movement Speed"))
                                    .on_hover_text("Camera movement speed for WASD controls")
                                    .changed();

                                // Camera speed presets
                                ui.horizontal(|ui| {
                                    ui.label("Speed presets:");
                                    if ui.small_button("Slow")
                                        .on_hover_text("Set camera speed to 1.0")
                                        .clicked() {
                                        params.camera_speed = 1.0;
                                        changed = true;
                                    }
                                    if ui.small_button("Normal")
                                        .on_hover_text("Set camera speed to 3.0")
                                        .clicked() {
                                        params.camera_speed = 3.0;
                                        changed = true;
                                    }
                                    if ui.small_button("Fast")
                                        .on_hover_text("Set camera speed to 6.0")
                                        .clicked() {
                                        params.camera_speed = 6.0;
                                        changed = true;
                                    }
                                });

                                ui.add_space(5.0);
                                changed |= ui.add(egui::Slider::new(&mut params.camera_fov, 20.0..=120.0)
                                    .text("Field of View (FOV)"))
                                    .on_hover_text("Camera field of view in degrees\n45° = normal, higher = wide angle")
                                    .changed();

                                // FOV presets
                                ui.horizontal(|ui| {
                                    ui.label("FOV presets:");
                                    if ui.small_button("Wide")
                                        .on_hover_text("Set FOV to 90° (wide angle)")
                                        .clicked() {
                                        params.camera_fov = 90.0;
                                        changed = true;
                                    }
                                    if ui.small_button("Normal")
                                        .on_hover_text("Set FOV to 45° (normal)")
                                        .clicked() {
                                        params.camera_fov = 45.0;
                                        changed = true;
                                    }
                                    if ui.small_button("Tele")
                                        .on_hover_text("Set FOV to 30° (telephoto/zoomed)")
                                        .clicked() {
                                        params.camera_fov = 30.0;
                                        changed = true;
                                    }
                                });

                                ui.separator();
                                ui.label("Auto Orbit:")
                                    .on_hover_text("Automatically rotate camera around the fractal [O]");
                                changed |= ui.checkbox(&mut params.auto_orbit, "Enable Auto Orbit")
                                    .on_hover_text("Toggle auto-orbit mode [O]")
                                    .changed();
                                if params.auto_orbit {
                                    changed |= ui.add(egui::Slider::new(&mut params.orbit_speed, 0.1..=3.0)
                                        .text("Orbit Speed"))
                                        .on_hover_text("Rotation speed in auto-orbit mode [ and ] to adjust")
                                        .changed();
                                }

                                ui.separator();
                                if ui.checkbox(&mut self.show_camera_info, "Show Camera Info Overlay")
                                    .on_hover_text("Display camera position and direction on screen")
                                    .changed() {
                                    self.ui_state.show_camera_info = self.show_camera_info;
                                }

                                ui.separator();
                                ui.horizontal(|ui| {
                                    if ui.button("🔄 Reset Camera")
                                        .on_hover_text("Reset camera to default position")
                                        .clicked() {
                                        reset_camera_requested = true;
                                    }
                                    if ui.button("🎯 Point at Fractal")
                                        .on_hover_text("Aim camera at fractal center")
                                        .clicked() {
                                        point_at_fractal_requested = true;
                                    }
                                });

                                ui.separator();
                                ui.label("Camera Bookmarks:")
                                    .on_hover_text("Save and restore camera viewpoints");

                                ui.horizontal(|ui| {
                                    ui.label("Name:");
                                    ui.text_edit_singleline(&mut self.bookmark_name);
                                });

                                if ui.button("📌 Save Bookmark")
                                    .on_hover_text("Save current camera position")
                                    .clicked()
                                    && !self.bookmark_name.is_empty()
                                {
                                    let bookmark = CameraBookmark::new(
                                        self.bookmark_name.clone(),
                                        camera_pos,
                                        camera_target,
                                        params.camera_fov,
                                    );

                                    // Sanitize filename
                                    let filename = self.bookmark_name.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");

                                    if let Err(e) = BookmarkGallery::save_bookmark(&bookmark, &filename) {
                                        eprintln!("Failed to save bookmark: {}", e);
                                    } else {
                                        // Refresh bookmark list
                                        self.bookmarks = BookmarkGallery::list_bookmarks().unwrap_or_default();
                                        self.bookmark_name.clear();
                                    }
                                }

                                // Refresh bookmark list periodically
                                if self.last_bookmark_list_update.elapsed().as_secs() > 2 {
                                    self.bookmarks = BookmarkGallery::list_bookmarks().unwrap_or_default();
                                    self.last_bookmark_list_update = web_time::Instant::now();
                                }

                                if !self.bookmarks.is_empty() {
                                    ui.separator();
                                    ui.label("Saved Bookmarks:")
                                        .on_hover_text("Click to load, right-click to delete");

                                    egui::ScrollArea::vertical()
                                        .id_salt("bookmarks_scroll")
                                        .max_height(120.0)
                                        .show(ui, |ui| {
                                            let bookmarks_clone = self.bookmarks.clone();
                                            for bookmark_name in bookmarks_clone.iter() {
                                                ui.horizontal(|ui| {
                                                    if ui.button(bookmark_name)
                                                        .on_hover_text("Click to restore this camera position")
                                                        .clicked() {
                                                        if let Ok(bookmark) = BookmarkGallery::load_bookmark(bookmark_name) {
                                                            bookmark_to_load = Some(bookmark);
                                                        }
                                                    }
                                                    if ui.small_button("🗑")
                                                        .on_hover_text("Delete this bookmark")
                                                        .clicked() {
                                                        self.bookmark_to_delete = Some(bookmark_name.clone());
                                                    }
                                                });
                                            }
                                        });
                                }

                                // Handle bookmark deletion
                                if let Some(ref bookmark_name) = self.bookmark_to_delete {
                                    if let Err(e) = BookmarkGallery::delete_bookmark(bookmark_name) {
                                        eprintln!("Failed to delete bookmark: {}", e);
                                    }
                                    self.bookmarks = BookmarkGallery::list_bookmarks().unwrap_or_default();
                                    self.bookmark_to_delete = None;
                                }
                            });
                        self.ui_state.camera_open = response.openness > 0.0;

                        let response = egui::CollapsingHeader::new("Shading")
                            .default_open(self.ui_state.shading_open)
                            .show(ui, |ui| {
                                changed |= ui.radio_value(&mut params.shading_model, ShadingModel::BlinnPhong, "Blinn-Phong")
                                    .on_hover_text("Classic Blinn-Phong shading - fast and simple")
                                    .changed();
                                changed |= ui.radio_value(&mut params.shading_model, ShadingModel::PBR, "PBR")
                                    .on_hover_text("Physically Based Rendering - more realistic materials")
                                    .changed();

                                if params.shading_model == ShadingModel::PBR {
                                    ui.separator();
                                    ui.label("Material Properties:")
                                        .on_hover_text("Control surface appearance with PBR");
                                    changed |= ui.add(egui::Slider::new(&mut params.roughness, 0.0..=1.0)
                                        .text("Roughness"))
                                        .on_hover_text("Surface roughness: 0 = smooth/shiny, 1 = rough/matte")
                                        .changed();
                                    changed |= ui.add(egui::Slider::new(&mut params.metallic, 0.0..=1.0)
                                        .text("Metallic"))
                                        .on_hover_text("Metalness: 0 = dielectric, 1 = metal")
                                        .changed();
                                }
                            });
                        self.ui_state.shading_open = response.openness > 0.0;

                        let response = egui::CollapsingHeader::new("Lighting")
                            .default_open(self.ui_state.lighting_open)
                            .show(ui, |ui| {
                                ui.label("Light Settings:")
                                    .on_hover_text("Configure lighting intensity and ambience");
                                changed |= ui.add(egui::Slider::new(&mut params.light_intensity, 0.5..=10.0)
                                    .text("Light Intensity"))
                                    .on_hover_text("Brightness of the main directional light")
                                    .changed();
                                changed |= ui.add(egui::Slider::new(&mut params.ambient_light, 0.0..=1.0)
                                    .text("Ambient Light"))
                                    .on_hover_text("Base illumination level - prevents pure black shadows")
                                    .changed();

                                ui.separator();
                                ui.label("Light Direction:")
                                    .on_hover_text("Control the direction of the main light");
                                changed |= ui.add(egui::Slider::new(&mut params.light_azimuth, 0.0..=360.0)
                                    .text("Azimuth"))
                                    .on_hover_text("Horizontal angle of the light (0-360°)")
                                    .changed();
                                changed |= ui.add(egui::Slider::new(&mut params.light_elevation, 5.0..=90.0)
                                    .text("Elevation"))
                                    .on_hover_text("Vertical angle of the light (5-90°, where 90° is directly above)")
                                    .changed();

                                ui.separator();
                                ui.label("Shadows & AO:")
                                    .on_hover_text("Shadow and occlusion effects [B to cycle shadows]");
                                ui.horizontal(|ui| {
                                    ui.label("Shadows [B]:");
                                    let shadow_names = ["Off", "Hard", "Soft"];
                                    egui::ComboBox::from_id_salt("shadow_mode")
                                        .selected_text(shadow_names[params.shadow_mode as usize])
                                        .show_ui(ui, |ui| {
                                            for (i, name) in shadow_names.iter().enumerate() {
                                                if ui.selectable_value(&mut params.shadow_mode, i as u32, *name).changed() {
                                                    changed = true;
                                                }
                                            }
                                        });
                                });
                                if params.shadow_mode > 0 {
                                    changed |= ui.add(egui::Slider::new(&mut params.shadow_max_distance, 1.0..=20.0)
                                        .text("Shadow Distance"))
                                        .on_hover_text("Maximum distance for shadow ray marching")
                                        .changed();
                                    changed |= ui.add(egui::Slider::new(&mut params.shadow_samples, 32..=256)
                                        .text("Shadow Samples"))
                                        .on_hover_text("Number of ray marching steps for shadows - higher = more accurate but slower")
                                        .changed();
                                    changed |= ui.add(egui::Slider::new(&mut params.shadow_step_factor, 0.3..=1.0)
                                        .text("Shadow Accuracy"))
                                        .on_hover_text("Step size factor for shadow rays - lower = more accurate but slower (0.6 is good default)")
                                        .changed();
                                }
                                if params.shadow_mode == 2 {
                                    changed |= ui.add(egui::Slider::new(&mut params.shadow_softness, 1.0..=32.0)
                                        .text("Shadow Softness"))
                                        .on_hover_text("Shadow penumbra softness - higher = softer edges")
                                        .changed();
                                }

                                changed |= ui.checkbox(&mut params.ambient_occlusion, "Ambient Occlusion")
                                    .on_hover_text("Enable ambient occlusion for contact shadows [L]")
                                    .changed();
                                if params.ambient_occlusion {
                                    changed |= ui.add(egui::Slider::new(&mut params.ao_intensity, 0.0..=10.0)
                                        .text("AO Intensity"))
                                        .on_hover_text("Strength of ambient occlusion darkening")
                                        .changed();
                                    changed |= ui.add(egui::Slider::new(&mut params.ao_step_size, 0.01..=0.5)
                                        .text("AO Step Size"))
                                        .on_hover_text("Step size for AO sampling - smaller = finer detail")
                                        .changed();
                                }
                            });
                        self.ui_state.lighting_open = response.openness > 0.0;

                        let response = egui::CollapsingHeader::new("Effects")
                            .default_open(self.ui_state.effects_open)
                            .show(ui, |ui| {
                                changed |= ui.checkbox(&mut params.depth_of_field, "Depth of Field")
                                    .on_hover_text("Blur based on distance from focus [T]")
                                    .changed();

                                if params.depth_of_field {
                                    changed |= ui.add(egui::Slider::new(&mut params.dof_focal_length, 1.0..=20.0)
                                        .text("Focal Length"))
                                        .on_hover_text("Distance to the focus plane - objects at this distance are sharp")
                                        .changed();
                                    changed |= ui.add(egui::Slider::new(&mut params.dof_aperture, 0.01..=1.0)
                                        .text("Aperture"))
                                        .on_hover_text("Aperture size - larger = more blur, smaller = sharper")
                                        .changed();
                                    changed |= ui.add(egui::Slider::new(&mut params.dof_samples, 1..=16)
                                        .text("Samples (quality vs speed)"))
                                        .on_hover_text("Number of samples per pixel - higher = smoother but slower")
                                        .changed();
                                }

                                ui.separator();
                                changed |= ui.checkbox(&mut params.fog_enabled, "Fog")
                                    .on_hover_text("Add atmospheric fog effect")
                                    .changed();

                                if params.fog_enabled {
                                    changed |= egui::ComboBox::from_id_salt("fog_mode")
                                        .selected_text(match params.fog_mode {
                                            crate::fractal::FogMode::Linear => "Linear",
                                            crate::fractal::FogMode::Exponential => "Exponential",
                                            crate::fractal::FogMode::Quadratic => "Quadratic",
                                        })
                                        .show_ui(ui, |ui| {
                                            let mut changed_local = false;
                                            changed_local |= ui.selectable_value(&mut params.fog_mode, crate::fractal::FogMode::Linear, "Linear")
                                                .on_hover_text("Linear fog falloff")
                                                .changed();
                                            changed_local |= ui.selectable_value(&mut params.fog_mode, crate::fractal::FogMode::Exponential, "Exponential")
                                                .on_hover_text("Exponential fog falloff - more realistic")
                                                .changed();
                                            changed_local |= ui.selectable_value(&mut params.fog_mode, crate::fractal::FogMode::Quadratic, "Quadratic")
                                                .on_hover_text("Quadratic fog falloff - dense atmosphere")
                                                .changed();
                                            changed_local
                                        })
                                        .inner.unwrap_or(false);

                                    changed |= ui.add(egui::Slider::new(&mut params.fog_density, 0.0..=0.2)
                                        .text("Fog Density"))
                                        .on_hover_text("How thick the fog is - higher = denser")
                                        .changed();

                                    ui.label("Fog Color:")
                                        .on_hover_text("Color of the fog");
                                    let mut fog_color = [params.fog_color.x, params.fog_color.y, params.fog_color.z];
                                    if ui.color_edit_button_rgb(&mut fog_color)
                                        .on_hover_text("Click to change fog color")
                                        .changed() {
                                        params.fog_color = glam::Vec3::from_array(fog_color);
                                        changed = true;
                                    }
                                }

                                // Post-Processing Section
                                ui.separator();
                                ui.label("Post-Processing:")
                                    .on_hover_text("Visual effects applied after rendering");

                                // Color Grading
                                ui.label("Color Grading:")
                                    .on_hover_text("Adjust the overall look and color of the image");
                                changed |= ui.add(egui::Slider::new(&mut params.brightness, 0.0..=2.0)
                                    .text("Brightness"))
                                    .on_hover_text("Overall image brightness (1.0 = normal)")
                                    .changed();
                                changed |= ui.add(egui::Slider::new(&mut params.contrast, 0.0..=2.0)
                                    .text("Contrast"))
                                    .on_hover_text("Contrast between light and dark areas (1.0 = normal)")
                                    .changed();
                                changed |= ui.add(egui::Slider::new(&mut params.saturation, 0.0..=2.0)
                                    .text("Saturation"))
                                    .on_hover_text("Color intensity (0.0 = grayscale, 1.0 = normal, 2.0 = vivid)")
                                    .changed();
                                changed |= ui.add(egui::Slider::new(&mut params.hue_shift, -1.0..=1.0)
                                    .text("Hue Shift"))
                                    .on_hover_text("Shift colors around the color wheel (-1.0 to 1.0)")
                                    .changed();

                                ui.separator();

                                // Vignette
                                changed |= ui.checkbox(&mut params.vignette_enabled, "Vignette")
                                    .on_hover_text("Darken the edges of the image")
                                    .changed();
                                if params.vignette_enabled {
                                    changed |= ui.add(egui::Slider::new(&mut params.vignette_intensity, 0.0..=1.0)
                                        .text("Vignette Intensity"))
                                        .on_hover_text("How dark the edges become (0.0 = no effect, 1.0 = very dark)")
                                        .changed();
                                    changed |= ui.add(egui::Slider::new(&mut params.vignette_radius, 0.1..=2.0)
                                        .text("Vignette Radius"))
                                        .on_hover_text("Size of the vignette effect (smaller = larger dark area)")
                                        .changed();
                                }

                                ui.separator();

                                // Bloom
                                changed |= ui.checkbox(&mut params.bloom_enabled, "Bloom")
                                    .on_hover_text("Glow effect around bright areas - extracts and blurs bright pixels using multi-pass rendering")
                                    .changed();
                                if params.bloom_enabled {
                                    changed |= ui.add(egui::Slider::new(&mut params.bloom_threshold, 0.0..=1.0)
                                        .text("Threshold"))
                                        .on_hover_text("Minimum brightness for bloom (0.0 = all pixels, 1.0 = only brightest)")
                                        .changed();
                                    changed |= ui.add(egui::Slider::new(&mut params.bloom_intensity, 0.0..=2.0)
                                        .text("Intensity"))
                                        .on_hover_text("Strength of the bloom glow (0.3-0.8 recommended)")
                                        .changed();
                                }

                                ui.separator();

                                // FXAA Anti-aliasing
                                changed |= ui.checkbox(&mut params.fxaa_enabled, "FXAA Anti-aliasing")
                                    .on_hover_text("Fast approximate anti-aliasing to smooth jagged edges")
                                    .changed();
                            });
                        self.ui_state.effects_open = response.openness > 0.0;

                        let response = egui::CollapsingHeader::new("Floor")
                            .default_open(self.ui_state.floor_open)
                            .show(ui, |ui| {
                                changed |= ui.checkbox(&mut params.show_floor, "Show Floor")
                                    .on_hover_text("Display checkered floor plane [G]")
                                    .changed();

                                if params.show_floor {
                                    changed |= ui.add(egui::Slider::new(&mut params.floor_height, -10.0..=10.0)
                                        .text("Floor Height"))
                                        .on_hover_text("Vertical position of the floor plane")
                                        .changed();

                                    ui.separator();
                                    ui.label("Floor Colors:")
                                        .on_hover_text("Checkerboard pattern colors");

                                    let mut color1 = [params.floor_color1.x, params.floor_color1.y, params.floor_color1.z];
                                    if ui.color_edit_button_rgb(&mut color1)
                                        .on_hover_text("First checkerboard color")
                                        .changed() {
                                        params.floor_color1 = glam::Vec3::from_array(color1);
                                        changed = true;
                                    }

                                    let mut color2 = [params.floor_color2.x, params.floor_color2.y, params.floor_color2.z];
                                    if ui.color_edit_button_rgb(&mut color2)
                                        .on_hover_text("Second checkerboard color")
                                        .changed() {
                                        params.floor_color2 = glam::Vec3::from_array(color2);
                                        changed = true;
                                    }

                                    ui.separator();
                                    changed |= ui.checkbox(&mut params.floor_reflections, "Floor Reflections")
                                        .on_hover_text("Enable screen-space reflections on the floor - reflects the fractal onto the floor surface with Fresnel effect")
                                        .changed();

                                    if params.floor_reflections {
                                        changed |= ui.add(egui::Slider::new(&mut params.floor_reflection_strength, 0.0..=1.0)
                                            .text("Reflection Strength"))
                                            .on_hover_text("Adjust reflection intensity: 0.0 = no reflections, 0.5 = moderate, 1.0 = maximum reflections")
                                            .changed();
                                    }
                                }
                            });
                        self.ui_state.floor_open = response.openness > 0.0;

                        // LOD System
                        let response = egui::CollapsingHeader::new("LOD System")
                            .default_open(self.ui_state.lod_open)
                            .show(ui, |ui| {
                                ui.label("Adaptive quality system for smooth performance")
                                    .on_hover_text("Automatically adjusts rendering quality based on distance, motion, and performance");

                                ui.separator();

                                // Main Controls
                                changed |= ui.checkbox(&mut params.lod_config.enabled, "Enable LOD System")
                                    .on_hover_text("Enable adaptive quality adjustment (disabled by default)")
                                    .changed();

                                if params.lod_config.enabled {
                                    ui.separator();

                                    // Profile Selection
                                    ui.label("Profile:");
                                    let profile_changed = egui::ComboBox::from_id_salt("lod_profile")
                                        .selected_text(params.lod_config.profile_name())
                                        .show_ui(ui, |ui| {
                                            use crate::lod::LODProfile;
                                            let mut changed_local = false;

                                            changed_local |= ui.selectable_value(&mut params.lod_config.profile, LODProfile::Balanced, "Balanced")
                                                .on_hover_text("Good mix of quality and performance (default)")
                                                .changed();
                                            changed_local |= ui.selectable_value(&mut params.lod_config.profile, LODProfile::QualityFirst, "Quality First")
                                                .on_hover_text("Prioritize visual quality, less aggressive LOD")
                                                .changed();
                                            changed_local |= ui.selectable_value(&mut params.lod_config.profile, LODProfile::PerformanceFirst, "Performance First")
                                                .on_hover_text("Prioritize performance, aggressive LOD")
                                                .changed();
                                            changed_local |= ui.selectable_value(&mut params.lod_config.profile, LODProfile::DistanceOnly, "Distance Only")
                                                .on_hover_text("Only use distance-based LOD")
                                                .changed();
                                            changed_local |= ui.selectable_value(&mut params.lod_config.profile, LODProfile::MotionOnly, "Motion Only")
                                                .on_hover_text("Only reduce quality during camera movement")
                                                .changed();
                                            changed_local |= ui.selectable_value(&mut params.lod_config.profile, LODProfile::Custom, "Custom")
                                                .on_hover_text("User-defined configuration")
                                                .changed();

                                            changed_local
                                        })
                                        .inner.unwrap_or(false);

                                    // Apply profile if changed
                                    if profile_changed && params.lod_config.profile != crate::lod::LODProfile::Custom {
                                        params.lod_config.apply_profile(params.lod_config.profile);
                                        changed = true;
                                    }
                                    changed |= profile_changed;

                                    ui.separator();

                                    // Strategy Selection
                                    ui.label("Strategy:");
                                    changed |= egui::ComboBox::from_id_salt("lod_strategy")
                                        .selected_text(match params.lod_config.strategy {
                                            crate::lod::LODStrategy::Distance => "Distance-based",
                                            crate::lod::LODStrategy::Motion => "Motion-based",
                                            crate::lod::LODStrategy::Performance => "Performance-based",
                                            crate::lod::LODStrategy::Hybrid => "Hybrid (All)",
                                        })
                                        .show_ui(ui, |ui| {
                                            let mut changed_local = false;
                                            changed_local |= ui.selectable_value(&mut params.lod_config.strategy, crate::lod::LODStrategy::Distance, "Distance-based")
                                                .on_hover_text("Reduce quality based on distance from camera")
                                                .changed();
                                            changed_local |= ui.selectable_value(&mut params.lod_config.strategy, crate::lod::LODStrategy::Motion, "Motion-based")
                                                .on_hover_text("Reduce quality during camera movement")
                                                .changed();
                                            changed_local |= ui.selectable_value(&mut params.lod_config.strategy, crate::lod::LODStrategy::Performance, "Performance-based")
                                                .on_hover_text("Adjust quality to maintain target FPS")
                                                .changed();
                                            changed_local |= ui.selectable_value(&mut params.lod_config.strategy, crate::lod::LODStrategy::Hybrid, "Hybrid (All)")
                                                .on_hover_text("Intelligently combine all strategies")
                                                .changed();
                                            changed_local
                                        })
                                        .inner.unwrap_or(false);

                                    // Target FPS
                                    changed |= ui.add(egui::Slider::new(&mut params.lod_config.target_fps, 30.0..=120.0)
                                        .text("Target FPS"))
                                        .on_hover_text("Target framerate for performance-based LOD")
                                        .changed();

                                    // Debug Visualization
                                    changed |= ui.checkbox(&mut params.lod_config.debug_visualization, "Debug Visualization")
                                        .on_hover_text("Show current LOD level and performance metrics")
                                        .changed();

                                    ui.separator();

                                    // Distance-based Controls
                                    if params.lod_config.strategy == crate::lod::LODStrategy::Distance ||
                                       params.lod_config.strategy == crate::lod::LODStrategy::Hybrid {
                                        ui.collapsing("Distance Zones", |ui| {
                                            ui.label("Define distance thresholds for quality levels:")
                                                .on_hover_text("Closer = higher quality, farther = lower quality");

                                            ui.add_space(4.0);

                                            // Ultra zone (< zone 0)
                                            ui.horizontal(|ui| {
                                                let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                                                ui.painter().rect_filled(rect, 2.0, egui::Color32::from_rgb(0, 255, 0));
                                                changed |= ui.add(egui::Slider::new(&mut params.lod_config.distance_zones[0], 1.0..=50.0)
                                                    .text("Near -> Mid"))
                                                    .on_hover_text("Distance where quality drops from Ultra to High")
                                                    .changed();
                                            });

                                            // High zone (zone 0 to zone 1)
                                            ui.horizontal(|ui| {
                                                let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                                                ui.painter().rect_filled(rect, 2.0, egui::Color32::from_rgb(100, 255, 100));
                                                changed |= ui.add(egui::Slider::new(&mut params.lod_config.distance_zones[1], 10.0..=100.0)
                                                    .text("Mid -> Far"))
                                                    .on_hover_text("Distance where quality drops from High to Medium")
                                                    .changed();
                                            });

                                            // Medium zone (zone 1 to zone 2)
                                            ui.horizontal(|ui| {
                                                let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                                                ui.painter().rect_filled(rect, 2.0, egui::Color32::from_rgb(255, 200, 0));
                                                changed |= ui.add(egui::Slider::new(&mut params.lod_config.distance_zones[2], 25.0..=150.0)
                                                    .text("Far -> Distant"))
                                                    .on_hover_text("Distance where quality drops from Medium to Low")
                                                    .changed();
                                            });

                                            // Low zone (> zone 2) - indicator only
                                            ui.horizontal(|ui| {
                                                let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                                                ui.painter().rect_filled(rect, 2.0, egui::Color32::from_rgb(255, 100, 0));
                                                ui.label("Far (Low quality beyond last zone)");
                                            });

                                            // Ensure zones are ordered correctly
                                            if params.lod_config.distance_zones[0] > params.lod_config.distance_zones[1] {
                                                params.lod_config.distance_zones[0] = params.lod_config.distance_zones[1];
                                                changed = true;
                                            }
                                            if params.lod_config.distance_zones[1] > params.lod_config.distance_zones[2] {
                                                params.lod_config.distance_zones[1] = params.lod_config.distance_zones[2];
                                                changed = true;
                                            }
                                        });
                                    }

                                    // Motion-based Controls
                                    if params.lod_config.strategy == crate::lod::LODStrategy::Motion ||
                                       params.lod_config.strategy == crate::lod::LODStrategy::Hybrid {
                                        ui.collapsing("Motion Settings", |ui| {
                                            changed |= ui.add(egui::Slider::new(&mut params.lod_config.motion_sensitivity, 0.0..=5.0)
                                                .text("Motion Sensitivity"))
                                                .on_hover_text("Higher = more sensitive to camera movement (1.0 = normal)")
                                                .changed();

                                            changed |= ui.add(egui::Slider::new(&mut params.lod_config.motion_threshold, 0.01..=1.0)
                                                .text("Motion Threshold"))
                                                .on_hover_text("Minimum velocity to trigger quality reduction")
                                                .changed();

                                            changed |= ui.add(egui::Slider::new(&mut params.lod_config.restore_delay, 0.1..=2.0)
                                                .text("Restore Delay"))
                                                .on_hover_text("Seconds to wait after stopping before restoring quality")
                                                .changed();
                                        });
                                    }

                                    // Quality Level Presets
                                    ui.collapsing("Quality Presets", |ui| {
                                        ui.horizontal(|ui| {
                                            if ui.button("Ultra")
                                                .on_hover_text("Maximum quality preset")
                                                .clicked() {
                                                params.lod_config.quality_presets[0] = crate::lod::QualityLevel::ultra();
                                                changed = true;
                                            }
                                            if ui.button("High")
                                                .on_hover_text("High quality preset")
                                                .clicked() {
                                                params.lod_config.quality_presets[1] = crate::lod::QualityLevel::high();
                                                changed = true;
                                            }
                                            if ui.button("Medium")
                                                .on_hover_text("Medium quality preset")
                                                .clicked() {
                                                params.lod_config.quality_presets[2] = crate::lod::QualityLevel::medium();
                                                changed = true;
                                            }
                                            if ui.button("Low")
                                                .on_hover_text("Low quality preset")
                                                .clicked() {
                                                params.lod_config.quality_presets[3] = crate::lod::QualityLevel::low();
                                                changed = true;
                                            }
                                        });

                                        ui.horizontal(|ui| {
                                            if ui.button("Reset All to Defaults")
                                                .on_hover_text("Reset all quality presets to default values")
                                                .clicked() {
                                                params.lod_config.quality_presets = [
                                                    crate::lod::QualityLevel::ultra(),
                                                    crate::lod::QualityLevel::high(),
                                                    crate::lod::QualityLevel::medium(),
                                                    crate::lod::QualityLevel::low(),
                                                ];
                                                changed = true;
                                            }
                                        });

                                        ui.separator();

                                        // Custom quality editor
                                        ui.label("Edit Quality Levels:")
                                            .on_hover_text("Fine-tune each quality preset");

                                        for (i, preset) in params.lod_config.quality_presets.iter_mut().enumerate() {
                                            let level_name = match i {
                                                0 => "Ultra",
                                                1 => "High",
                                                2 => "Medium",
                                                3 => "Low",
                                                _ => "Unknown",
                                            };

                                            ui.collapsing(level_name, |ui| {
                                                changed |= ui.add(egui::Slider::new(&mut preset.max_steps, 50..=500)
                                                    .text("Max Steps"))
                                                    .on_hover_text("Ray marching iterations")
                                                    .changed();

                                                changed |= ui.add(egui::Slider::new(&mut preset.min_distance, 0.0001..=0.01)
                                                    .text("Min Distance")
                                                    .logarithmic(true))
                                                    .on_hover_text("Surface precision threshold")
                                                    .changed();

                                                changed |= ui.add(egui::Slider::new(&mut preset.shadow_samples, 4..=256)
                                                    .text("Shadow Samples"))
                                                    .on_hover_text("Shadow quality")
                                                    .changed();

                                                changed |= ui.add(egui::Slider::new(&mut preset.shadow_step_factor, 0.3..=1.0)
                                                    .text("Shadow Step Factor"))
                                                    .on_hover_text("Shadow ray step size (higher = faster, less precise)")
                                                    .changed();

                                                changed |= ui.add(egui::Slider::new(&mut preset.ao_step_size, 0.05..=0.5)
                                                    .text("AO Step Size"))
                                                    .on_hover_text("Ambient occlusion step size")
                                                    .changed();

                                                changed |= ui.add(egui::Slider::new(&mut preset.dof_samples, 1..=16)
                                                    .text("DOF Samples"))
                                                    .on_hover_text("Depth of field sample count")
                                                    .changed();

                                                changed |= ui.add(egui::Slider::new(&mut preset.render_scale, 0.25..=1.0)
                                                    .text("Render Scale"))
                                                    .on_hover_text("Resolution multiplier (1.0 = native)")
                                                    .changed();
                                            });
                                        }
                                    });

                                    // Advanced Settings
                                    ui.collapsing("Advanced", |ui| {
                                        changed |= ui.checkbox(&mut params.lod_config.smooth_transitions, "Smooth Transitions")
                                            .on_hover_text("Interpolate between quality levels for seamless changes")
                                            .changed();

                                        if params.lod_config.smooth_transitions {
                                            changed |= ui.add(egui::Slider::new(&mut params.lod_config.transition_duration, 0.0..=1.0)
                                                .text("Transition Duration"))
                                                .on_hover_text("Time to transition between quality levels (seconds)")
                                                .changed();
                                        }

                                        changed |= ui.checkbox(&mut params.lod_config.aggressive_mode, "Aggressive Mode")
                                            .on_hover_text("More aggressive quality reduction for better performance")
                                            .changed();

                                        ui.label("Minimum Quality Level:");
                                        changed |= egui::ComboBox::from_id_salt("min_quality_level")
                                            .selected_text(match params.lod_config.min_quality_level {
                                                0 => "Ultra",
                                                1 => "High",
                                                2 => "Medium",
                                                3 => "Low",
                                                _ => "Unknown",
                                            })
                                            .show_ui(ui, |ui| {
                                                let mut changed_local = false;
                                                changed_local |= ui.selectable_value(&mut params.lod_config.min_quality_level, 0, "Ultra")
                                                    .on_hover_text("Never reduce quality below Ultra")
                                                    .changed();
                                                changed_local |= ui.selectable_value(&mut params.lod_config.min_quality_level, 1, "High")
                                                    .on_hover_text("Never reduce quality below High")
                                                    .changed();
                                                changed_local |= ui.selectable_value(&mut params.lod_config.min_quality_level, 2, "Medium")
                                                    .on_hover_text("Never reduce quality below Medium")
                                                    .changed();
                                                changed_local |= ui.selectable_value(&mut params.lod_config.min_quality_level, 3, "Low")
                                                    .on_hover_text("Allow all quality levels")
                                                    .changed();
                                                changed_local
                                            })
                                            .inner.unwrap_or(false);
                                    });

                                    // Status Display
                                    ui.separator();
                                    ui.collapsing("Status", |ui| {
                                        // Current LOD Level
                                        let level_name = match params.lod_state.current_level {
                                            0 => ("Ultra", egui::Color32::from_rgb(0, 255, 0)),
                                            1 => ("High", egui::Color32::from_rgb(100, 255, 100)),
                                            2 => ("Medium", egui::Color32::from_rgb(255, 200, 0)),
                                            3 => ("Low", egui::Color32::from_rgb(255, 100, 0)),
                                            _ => ("Unknown", egui::Color32::GRAY),
                                        };

                                        ui.horizontal(|ui| {
                                            ui.label("Current Level:");
                                            ui.colored_label(level_name.1, level_name.0);
                                        });

                                        // FPS Display
                                        ui.horizontal(|ui| {
                                            ui.label(format!("Current FPS: {:.1}", params.lod_state.current_fps));
                                        });

                                        // Motion Status
                                        ui.horizontal(|ui| {
                                            ui.label("Motion:");
                                            if params.lod_state.is_moving {
                                                ui.colored_label(egui::Color32::YELLOW, "Moving");
                                            } else {
                                                ui.colored_label(egui::Color32::GREEN, "Stationary");
                                            }
                                        });

                                        // Transition Progress
                                        if params.lod_state.transition_progress < 1.0 {
                                            ui.horizontal(|ui| {
                                                ui.label("Transitioning:");
                                                ui.add(egui::ProgressBar::new(params.lod_state.transition_progress)
                                                    .show_percentage());
                                            });
                                        }

                                        // Active Quality Parameters (showing what's currently being used)
                                        ui.collapsing("Active Parameters", |ui| {
                                            let quality = &params.lod_state.active_quality;
                                            ui.label(format!("Max Steps: {}", quality.max_steps));
                                            ui.label(format!("Min Distance: {:.6}", quality.min_distance));
                                            ui.label(format!("Shadow Samples: {}", quality.shadow_samples));
                                            ui.label(format!("Shadow Step: {:.2}", quality.shadow_step_factor));
                                            ui.label(format!("AO Step: {:.2}", quality.ao_step_size));
                                            ui.label(format!("DOF Samples: {}", quality.dof_samples));
                                            ui.label(format!("Render Scale: {:.2}", quality.render_scale));
                                        });
                                    });
                                }
                            });
                        self.ui_state.lod_open = response.openness > 0.0;
                    }
                }

                let response = egui::CollapsingHeader::new("Settings")
                    .default_open(self.ui_state.settings_open)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("💾 Save Settings")
                                .on_hover_text("Manually save current settings to disk")
                                .clicked() {
                                if let Err(e) = params.save_to_file() {
                                    eprintln!("Failed to save settings: {}", e);
                                }
                            }
                            if ui.button("🔄 Reset to Defaults")
                                .on_hover_text("Reset all settings to default values")
                                .clicked() {
                                reset_requested = true;
                            }
                        });

                        ui.separator();
                        ui.heading("GPU Selection");

                        // Load GPU list if not already loaded
                        if self.available_gpus.is_empty() {
                            // Note: We can't call async function here, so we'll show a button to load GPUs
                            if ui.button("🔍 Detect Available GPUs")
                                .on_hover_text("Scan for available graphics adapters")
                                .clicked()
                            {
                                // This will be handled in the app to call the async function
                                gpu_scan_requested = true;
                                self.gpu_selection_message = Some("Scanning for GPUs...".to_string());
                            }
                        } else {
                            // Load current preference
                            let prefs = crate::fractal::AppPreferences::load();
                            let mut current_selection = prefs.preferred_gpu_index.unwrap_or(0);

                            ui.label(format!("Available GPUs ({}):", self.available_gpus.len()));

                            egui::ComboBox::from_label("Select GPU")
                                .selected_text(if current_selection < self.available_gpus.len() {
                                    format!("#{}: {}", current_selection, self.available_gpus[current_selection].name)
                                } else {
                                    "Default (Auto-select)".to_string()
                                })
                                .show_ui(ui, |ui| {
                                    for (idx, gpu_info) in self.available_gpus.iter().enumerate() {
                                        let label = format!("#{}: {} ({}, {})",
                                            idx, gpu_info.name, gpu_info.backend, gpu_info.device_type);
                                        if ui.selectable_value(&mut current_selection, idx, label).clicked() {
                                            // Save preference
                                            let mut prefs = crate::fractal::AppPreferences::load();
                                            prefs.preferred_gpu_index = Some(current_selection);
                                            prefs.preferred_gpu_name = Some(gpu_info.name.clone());
                                            if let Err(e) = prefs.save() {
                                                self.gpu_selection_message = Some(format!("Failed to save preference: {}", e));
                                            } else {
                                                self.gpu_selection_message = Some("GPU preference saved. Please restart the application for changes to take effect.".to_string());
                                            }
                                        }
                                    }
                                });

                            if let Some(msg) = &self.gpu_selection_message {
                                ui.colored_label(egui::Color32::YELLOW, msg);
                            }

                            if ui.button("🔄 Refresh GPU List").clicked() {
                                self.available_gpus.clear();
                                self.gpu_selection_message = None;
                            }
                        }

                        ui.separator();
                        ui.label("Settings: ~/.config/par-fractal/settings.yaml")
                            .on_hover_text("Configuration file location");
                    });
                self.ui_state.settings_open = response.openness > 0.0;

                let response = egui::CollapsingHeader::new("Controls")
                    .default_open(self.ui_state.controls_open)
                    .show(ui, |ui| {
                        ui.label("General:");
                        ui.label("• H: Toggle UI");
                        ui.label("• F: Toggle FPS counter");
                        ui.label("• V: Toggle performance overlay");
                        ui.label("• F12: Save screenshot");
                        ui.label("• R: Reset view");
                        ui.label("• P: Next color palette");
                        ui.separator();

                        ui.label("2D Fractals (Number Keys):");
                        ui.label("• 1: Mandelbrot");
                        ui.label("• 2: Julia");
                        ui.label("• 3: Sierpinski Carpet");
                        ui.label("• 4: Burning Ship");
                        ui.label("• 5: Tricorn");
                        ui.label("• 6: Phoenix");
                        ui.label("• 7: Celtic");
                        ui.label("• 8: Newton");
                        ui.label("• 9: Lyapunov");
                        ui.label("• 0: Nova");
                        ui.label("• (Magnet, Collatz: use UI buttons)");
                        ui.separator();

                        ui.label("3D Fractals (Function Keys):");
                        ui.label("• F1: Mandelbulb");
                        ui.label("• F2: Menger Sponge");
                        ui.label("• F3: Sierpinski Pyramid");
                        ui.label("• F4: Julia Set 3D");
                        ui.label("• F5: Mandelbox");
                        ui.label("• F6: Tglad Formula");
                        ui.label("• F7: Octahedral IFS");
                        ui.label("• F8: Icosahedral IFS");
                        ui.label("• F9: Apollonian Gasket");
                        ui.label("• F10: Kleinian");
                        ui.label("• F11: Hybrid Bulb-Julia");
                        ui.label("• (Others: use UI buttons)");
                        ui.separator();

                        ui.label("Parameters:");
                        ui.label("• -/=: Decrease/increase iterations/steps");
                        ui.label("• ,/.: Decrease/increase fractal power");
                        ui.separator();

                        ui.label("Effects (3D):");
                        ui.label("• L: Toggle ambient occlusion");
                        ui.label("• T: Toggle depth of field");
                        ui.label("• G: Toggle floor");
                        ui.label("• B: Cycle shadow mode (Off/Hard/Soft)");
                        ui.separator();

                        ui.label("Camera (3D):");
                        ui.label("• WASD: Move forward/left/back/right");
                        ui.label("• Q/E: Move down/up");
                        ui.label("• Mouse Drag: Look around");
                        ui.label("• O: Toggle auto-orbit");
                        ui.label("• [/]: Decrease/increase orbit speed");
                        ui.separator();

                        match params.render_mode {
                            crate::fractal::RenderMode::TwoD => {
                                ui.label("Mouse (2D Mode):");
                                ui.label("• Drag: Pan view");
                                ui.label("• Wheel: Zoom in/out");
                            }
                            crate::fractal::RenderMode::ThreeD => {
                                ui.label("Mouse (3D Mode):");
                                ui.label("• Drag: Rotate camera view");
                                ui.label("• Wheel: Adjust move speed");
                            }
                        }
                    });
                self.ui_state.controls_open = response.openness > 0.0;
            });

        // Handle randomization request
        if randomize_requested {
            self.save_to_history(params);
            params.randomize();
            changed = true;
        }

        // Save to history when parameters change (but not if change came from undo/redo)
        if changed && !from_history {
            self.save_to_history(params);
        }

        // Capture & Recording Window
        if self.ui_state.capture_window_open {
            egui::Window::new("📷 Capture & Recording")
                .default_width(400.0)
                .resizable(true)
                .vscroll(true)
                .open(&mut self.ui_state.capture_window_open)
                .show(ctx, |ui| {
                    ui.heading("Screenshot");

                    if ui
                        .button("📷 Screen Resolution")
                        .on_hover_text("Capture current view at screen resolution [F12]")
                        .clicked()
                    {
                        screenshot_requested = true;
                    }

                    ui.label("Output: {fractal}_YYYYMMDD_HHMMSS.png")
                        .on_hover_text("Saved to current directory. {fractal} = fractal type name");

                    ui.separator();
                    ui.heading("🖥 Desktop Wallpaper")
                        .on_hover_text("Render at your monitor's native resolution for wallpapers");

                    // Monitor selection UI
                    if self.available_monitors.is_empty() {
                        if ui
                            .button("🔍 Detect Monitors")
                            .on_hover_text("Scan for available monitors")
                            .clicked()
                        {
                            // Signal to scan monitors - will be handled in app.rs
                            gpu_scan_requested = true; // Reuse this flag temporarily
                        }
                        ui.label("⚠ No monitors detected. Click to scan.")
                            .on_hover_text("Scan for connected displays");
                    } else {
                        ui.label("Select monitor:");

                        for (i, monitor) in self.available_monitors.iter().enumerate() {
                            let label = if monitor.is_primary {
                                format!(
                                    "🌟 {} ({}x{}) - Primary",
                                    monitor.name, monitor.width, monitor.height
                                )
                            } else {
                                format!(
                                    "   {} ({}x{})",
                                    monitor.name, monitor.width, monitor.height
                                )
                            };

                            if ui
                                .selectable_value(&mut self.selected_monitor_index, i, label)
                                .on_hover_text("Select this monitor for wallpaper rendering")
                                .clicked()
                            {
                                // Monitor selection changed
                            }
                        }

                        ui.add_space(4.0);
                        if let Some(monitor) =
                            self.available_monitors.get(self.selected_monitor_index)
                        {
                            if ui
                                .button(format!(
                                    "📐 Render {}x{} Wallpaper",
                                    monitor.width, monitor.height
                                ))
                                .on_hover_text(format!(
                                    "Render at {}'s native resolution",
                                    monitor.name
                                ))
                                .clicked()
                            {
                                hires_render_resolution = Some((monitor.width, monitor.height));
                            }
                        }

                        ui.horizontal(|ui| {
                            if ui
                                .button("🔄 Rescan Monitors")
                                .on_hover_text("Refresh monitor list")
                                .clicked()
                            {
                                gpu_scan_requested = true; // Reuse this flag
                            }
                            ui.label(format!(
                                "{} monitor(s) detected",
                                self.available_monitors.len()
                            ));
                        });
                    }

                    ui.label("Output: {fractal}_WxH_YYYYMMDD_HHMMSS.png")
                        .on_hover_text("Saved to current directory. {fractal} = fractal type name");

                    ui.separator();
                    ui.heading("High-Resolution Render")
                        .on_hover_text("Render at custom resolutions");
                    ui.label("Standard resolutions:");

                    // Classic 4:3 resolutions
                    ui.horizontal(|ui| {
                        if ui
                            .button("640x480 (VGA)")
                            .on_hover_text("Render at VGA resolution (4:3)")
                            .clicked()
                        {
                            hires_render_resolution = Some((640, 480));
                        }
                        if ui
                            .button("800x600 (SVGA)")
                            .on_hover_text("Render at SVGA resolution (4:3)")
                            .clicked()
                        {
                            hires_render_resolution = Some((800, 600));
                        }
                        if ui
                            .button("1024x768 (XGA)")
                            .on_hover_text("Render at XGA resolution (4:3)")
                            .clicked()
                        {
                            hires_render_resolution = Some((1024, 768));
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui
                            .button("1280x960")
                            .on_hover_text("Render at 1280x960 (4:3)")
                            .clicked()
                        {
                            hires_render_resolution = Some((1280, 960));
                        }
                        if ui
                            .button("1400x1050")
                            .on_hover_text("Render at 1400x1050 (4:3)")
                            .clicked()
                        {
                            hires_render_resolution = Some((1400, 1050));
                        }
                        if ui
                            .button("1600x1200 (UXGA)")
                            .on_hover_text("Render at UXGA resolution (4:3)")
                            .clicked()
                        {
                            hires_render_resolution = Some((1600, 1200));
                        }
                    });

                    ui.add_space(4.0);
                    ui.label("HD & modern resolutions:");

                    // 16:9 HD resolutions
                    ui.horizontal(|ui| {
                        if ui
                            .button("1280x720 (HD)")
                            .on_hover_text("Render at 720p HD resolution")
                            .clicked()
                        {
                            hires_render_resolution = Some((1280, 720));
                        }
                        if ui
                            .button("1920x1080 (Full HD)")
                            .on_hover_text("Render at 1080p Full HD resolution")
                            .clicked()
                        {
                            hires_render_resolution = Some((1920, 1080));
                        }
                        if ui
                            .button("2560x1440 (2K)")
                            .on_hover_text("Render at 1440p 2K resolution")
                            .clicked()
                        {
                            hires_render_resolution = Some((2560, 1440));
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui
                            .button("3840x2160 (4K)")
                            .on_hover_text("Render at 4K UHD resolution (may take time)")
                            .clicked()
                        {
                            hires_render_resolution = Some((3840, 2160));
                        }
                        if ui
                            .button("7680x4320 (8K)")
                            .on_hover_text("Render at 8K resolution (will take significant time)")
                            .clicked()
                        {
                            hires_render_resolution = Some((7680, 4320));
                        }
                    });

                    ui.add_space(4.0);
                    ui.label("Square & social media:");

                    // Square and other formats
                    ui.horizontal(|ui| {
                        if ui
                            .button("800x800")
                            .on_hover_text("Small square format")
                            .clicked()
                        {
                            hires_render_resolution = Some((800, 800));
                        }
                        if ui
                            .button("1080x1080 (Square)")
                            .on_hover_text("Square format for Instagram")
                            .clicked()
                        {
                            hires_render_resolution = Some((1080, 1080));
                        }
                        if ui
                            .button("2048x2048")
                            .on_hover_text("Large square format")
                            .clicked()
                        {
                            hires_render_resolution = Some((2048, 2048));
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui
                            .button("1080x1920 (Portrait)")
                            .on_hover_text("Portrait format for mobile/stories")
                            .clicked()
                        {
                            hires_render_resolution = Some((1080, 1920));
                        }
                    });

                    ui.add_space(4.0);
                    ui.label("Custom resolution:");

                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        let width_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.custom_width)
                                .desired_width(80.0)
                                .hint_text("1920"),
                        );
                        ui.label("Height:");
                        let height_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.custom_height)
                                .desired_width(80.0)
                                .hint_text("1080"),
                        );
                        if width_resp.changed() || height_resp.changed() {
                            changed = true;
                        }
                        if ui
                            .button("Render")
                            .on_hover_text("Render at custom resolution")
                            .clicked()
                        {
                            // Parse width and height
                            if let (Ok(width), Ok(height)) = (
                                self.custom_width.trim().parse::<u32>(),
                                self.custom_height.trim().parse::<u32>(),
                            ) {
                                if width > 0 && height > 0 && width <= 16384 && height <= 16384 {
                                    hires_render_resolution = Some((width, height));
                                } else {
                                    // Show error toast for invalid dimensions
                                    eprintln!(
                                        "Invalid resolution: {}x{} (must be 1-16384)",
                                        width, height
                                    );
                                }
                            } else {
                                eprintln!("Failed to parse resolution");
                            }
                        }
                    });

                    ui.label("Output: {fractal}_WxH_YYYYMMDD_HHMMSS.png")
                        .on_hover_text("Saved to current directory. {fractal} = fractal type name");

                    ui.add_space(4.0);
                    let prev_auto_open = self.auto_open_captures;
                    ui.checkbox(&mut self.auto_open_captures, "Auto-open captured images")
                        .on_hover_text("Automatically open captured images/videos after saving");
                    if self.auto_open_captures != prev_auto_open {
                        changed = true;
                    }

                    // Video recording section - native only
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        ui.separator();
                        ui.heading("Video & GIF Recording")
                            .on_hover_text("Record animated videos or GIFs of your fractals");

                        ui.horizontal(|ui| {
                            ui.label("Format:");
                            ui.add_enabled(
                                !is_recording,
                                egui::RadioButton::new(self.video_format == VideoFormat::MP4, "MP4"),
                            )
                            .clicked()
                            .then(|| self.video_format = VideoFormat::MP4);
                            ui.add_enabled(
                                !is_recording,
                                egui::RadioButton::new(
                                    self.video_format == VideoFormat::WebM,
                                    "WebM",
                                ),
                            )
                            .clicked()
                            .then(|| self.video_format = VideoFormat::WebM);
                            ui.add_enabled(
                                !is_recording,
                                egui::RadioButton::new(self.video_format == VideoFormat::GIF, "GIF"),
                            )
                            .clicked()
                            .then(|| self.video_format = VideoFormat::GIF);
                        });

                        ui.horizontal(|ui| {
                            ui.label("FPS:");
                            let fps_range = if self.video_format == VideoFormat::GIF {
                                10..=30 // GIFs typically use lower FPS
                            } else {
                                24..=60
                            };
                            ui.add_enabled(
                                !is_recording,
                                egui::Slider::new(&mut self.video_fps, fps_range).text("fps"),
                            );
                        });

                        // Clamp FPS when switching to GIF
                        if self.video_format == VideoFormat::GIF && self.video_fps > 30 {
                            self.video_fps = 30;
                        } else if self.video_format != VideoFormat::GIF && self.video_fps < 24 {
                            self.video_fps = 24;
                        }

                        ui.horizontal(|ui| {
                            if !is_recording {
                                if ui
                                    .button("🔴 Start Recording")
                                    .on_hover_text("Begin recording (requires ffmpeg)")
                                    .clicked()
                                {
                                    start_recording = true;
                                }
                            } else if ui
                                .button("⏹ Stop Recording")
                                .on_hover_text("Stop recording and save")
                                .clicked()
                            {
                                stop_recording = true;
                            }
                        });

                        ui.label("Output: {fractal}_YYYYMMDD_HHMMSS.{mp4,webm,gif}")
                            .on_hover_text(
                                "Saved to current directory. {fractal} = fractal type name",
                            );

                        if self.video_format == VideoFormat::GIF {
                            ui.label("ℹ GIF: Optimized palette, looped, great for social media")
                                .on_hover_text(
                                    "GIFs use palette-based encoding with dithering for best quality",
                                );
                        }

                        if !is_recording {
                            ui.label("⚠ Requires ffmpeg to be installed").on_hover_text(
                                "Install ffmpeg from your package manager or ffmpeg.org",
                            );
                        }
                    }
                });
        }

        // About Window
        if self.ui_state.about_window_open {
            egui::Window::new("ℹ About Par Fractal")
                .default_width(400.0)
                .resizable(false)
                .collapsible(false)
                .open(&mut self.ui_state.about_window_open)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Par Fractal");
                        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.label(env!("CARGO_PKG_DESCRIPTION"));

                    ui.add_space(8.0);

                    egui::Grid::new("about_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Author:");
                            ui.label("Paul Robello");
                            ui.end_row();

                            ui.label("License:");
                            ui.label("MIT");
                            ui.end_row();

                            ui.label("GitHub:");
                            ui.hyperlink_to(
                                "paulrobello/par-fractal",
                                "https://github.com/paulrobello/par-fractal",
                            );
                            ui.end_row();

                            ui.label("Crates.io:");
                            ui.hyperlink_to("par-fractal", "https://crates.io/crates/par-fractal");
                            ui.end_row();
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.collapsing("What's New in v0.3.0", |ui| {
                        ui.label("• 12 new 2D strange attractor fractals");
                        ui.label("• 27 new color palettes from xfractint");
                        ui.label("• Hit-based rendering for attractors");
                        ui.label("• Increased iteration limits (100k for attractors)");
                        ui.label("• Hotkey tooltips on all buttons");
                        ui.label("• About panel with version info");
                    });

                    ui.collapsing("Features", |ui| {
                        ui.label("• 13 classic 2D fractals + 12 strange attractors");
                        ui.label("• 13 ray-marched 3D fractals");
                        ui.label("• 33+ color palettes with animation");
                        ui.label("• PBR shading, AO, soft shadows, DoF");
                        ui.label("• Screenshot & video recording");
                        ui.label("• Preset system with import/export");
                        ui.label("• Cross-platform (Windows, macOS, Linux, Web)");
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Built with:");
                        ui.hyperlink_to("Rust", "https://www.rust-lang.org/");
                        ui.label("+");
                        ui.hyperlink_to("wgpu", "https://wgpu.rs/");
                        ui.label("+");
                        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                    });
                });
        }

        // Render toast notifications
        self.render_toasts(ctx);
        (
            changed,
            screenshot_requested,
            reset_requested,
            reset_camera_requested,
            point_at_fractal_requested,
            preset_to_load,
            hires_render_resolution,
            bookmark_to_load,
            gpu_scan_requested,
            start_recording,
            stop_recording,
        )
    }
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
