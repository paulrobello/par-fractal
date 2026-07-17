//! "2D Parameters", "3D Parameters", "Ray Marching", and "Shading" panels —
//! the core per-fractal/per-mode rendering knobs.

use super::super::{UI, UiActions};
use crate::fractal::{FractalParams, FractalType, ShadingModel};

impl UI {
    /// Render the "Shading" collapsing header (3D mode).
    pub fn render_shading_panel(
        &mut self,
        ui: &mut egui::Ui,
        params: &mut FractalParams,
        actions: &mut UiActions,
    ) {
        let response = egui::CollapsingHeader::new("Shading")
            .default_open(self.ui_state.shading_open)
            .show(ui, |ui| {
                actions.changed |= ui
                    .radio_value(
                        &mut params.settings.shading_model,
                        ShadingModel::BlinnPhong,
                        "Blinn-Phong",
                    )
                    .on_hover_text("Classic Blinn-Phong shading - fast and simple")
                    .changed();
                actions.changed |= ui
                    .radio_value(&mut params.settings.shading_model, ShadingModel::PBR, "PBR")
                    .on_hover_text("Physically Based Rendering - more realistic materials")
                    .changed();

                if params.settings.shading_model == ShadingModel::PBR {
                    ui.separator();
                    ui.label("Material Properties:")
                        .on_hover_text("Control surface appearance with PBR");
                    actions.changed |= ui
                        .add(
                            egui::Slider::new(&mut params.settings.roughness, 0.0..=1.0)
                                .text("Roughness"),
                        )
                        .on_hover_text("Surface roughness: 0 = smooth/shiny, 1 = rough/matte")
                        .changed();
                    actions.changed |= ui
                        .add(
                            egui::Slider::new(&mut params.settings.metallic, 0.0..=1.0)
                                .text("Metallic"),
                        )
                        .on_hover_text("Metalness: 0 = dielectric, 1 = metal")
                        .changed();
                }
            });
        self.ui_state.shading_open = response.openness > 0.0;
    }
}

impl UI {
    /// Render the "Ray Marching" collapsing header (3D mode).
    pub fn render_ray_marching_panel(
        &mut self,
        ui: &mut egui::Ui,
        params: &mut FractalParams,
        actions: &mut UiActions,
    ) {
        let response = egui::CollapsingHeader::new("Ray Marching")
            .default_open(self.ui_state.ray_marching_open)
            .show(ui, |ui| {
                actions.changed |= ui
                    .checkbox(&mut params.settings.use_adaptive_step, "Adaptive Step Size")
                    .on_hover_text(
                        "Use distance field for step size (recommended)\nDisable for fixed steps",
                    )
                    .changed();

                if params.settings.use_adaptive_step {
                    actions.changed |= ui
                        .add(
                            egui::Slider::new(&mut params.settings.step_multiplier, 0.1..=2.0)
                                .text("Step Multiplier"),
                        )
                        .on_hover_text(
                            "Adaptive step multiplier - lower = more accurate but slower",
                        )
                        .changed();
                } else {
                    actions.changed |= ui
                        .add(
                            egui::Slider::new(&mut params.settings.fixed_step_size, 0.01..=0.5)
                                .text("Fixed Step Size"),
                        )
                        .on_hover_text("Fixed step size in world units - smaller = more accurate")
                        .changed();
                }

                actions.changed |= ui
                    .add(
                        egui::Slider::new(&mut params.settings.max_steps, 32..=512)
                            .text("Max Steps"),
                    )
                    .on_hover_text(
                        "Maximum ray marching steps - higher = better quality but slower",
                    )
                    .changed();

                actions.changed |= ui
                    .add(
                        egui::Slider::new(&mut params.settings.min_distance, 0.0001..=0.01)
                            .text("Min Distance")
                            .logarithmic(true),
                    )
                    .on_hover_text(
                        "Distance threshold for surface hit detection\nSmaller = finer details",
                    )
                    .changed();

                actions.changed |= ui
                    .add(
                        egui::Slider::new(&mut params.settings.max_distance, 10.0..=200.0)
                            .text("Max Distance"),
                    )
                    .on_hover_text("Maximum ray marching distance before giving up")
                    .changed();
            });
        self.ui_state.ray_marching_open = response.openness > 0.0;
    }
}

impl UI {
    /// Render the "3D Parameters" collapsing header (3D mode).
    pub fn render_params_3d_panel(
        &mut self,
        ui: &mut egui::Ui,
        params: &mut FractalParams,
        actions: &mut UiActions,
    ) {
        let response = egui::CollapsingHeader::new("3D Parameters")
            .default_open(self.ui_state.params_3d_open)
            .show(ui, |ui| {
                // Scale control for all 3D fractals
                ui.label("Fractal Shape:")
                    .on_hover_text("Control the size and proportions of the fractal");
                actions.changed |= ui
                    .add(
                        egui::Slider::new(&mut params.settings.fractal_scale, 0.5..=5.0)
                            .text("Scale"),
                    )
                    .on_hover_text("Overall size of the fractal structure")
                    .changed();

                // Mandelbulb-specific parameters
                if params.settings.fractal_type == FractalType::Mandelbulb3D {
                    actions.changed |= ui
                        .add(
                            egui::Slider::new(&mut params.settings.power, 2.0..=16.0).text("Power"),
                        )
                        .on_hover_text("Mandelbulb power (8 is classic, higher = more detail)")
                        .changed();
                }

                // Julia 3D-specific parameters
                if params.settings.fractal_type == FractalType::JuliaSet3D {
                    ui.label("Julia Constant (C):")
                        .on_hover_text("Quaternion constant for 3D Julia set");
                    actions.changed |= ui
                        .add(
                            egui::Slider::new(&mut params.settings.julia_c[0], -2.0..=2.0)
                                .text("Real"),
                        )
                        .on_hover_text("Real component of quaternion constant")
                        .changed();
                    actions.changed |= ui
                        .add(
                            egui::Slider::new(&mut params.settings.julia_c[1], -2.0..=2.0)
                                .text("Imaginary"),
                        )
                        .on_hover_text("Imaginary component of quaternion constant")
                        .changed();
                }

                // Iterations control for specific 3D fractals
                if matches!(
                    params.settings.fractal_type,
                    FractalType::MengerSponge3D | FractalType::SierpinskiPyramid3D
                ) {
                    actions.changed |= ui
                        .add(
                            egui::Slider::new(&mut params.settings.max_iterations, 1..=20)
                                .text("Iterations"),
                        )
                        .on_hover_text(
                            "Recursion depth (higher = more detail and smaller features)",
                        )
                        .changed();
                }

                if params.settings.fractal_type == FractalType::QuaternionCubic3D {
                    actions.changed |= ui
                        .add(
                            egui::Slider::new(&mut params.settings.max_iterations, 1..=64)
                                .text("Iterations"),
                        )
                        .on_hover_text(
                            "Number of quaternion iterations (higher = more detail, slower)",
                        )
                        .changed();
                }

                // Advanced fractal shape controls
                if matches!(
                    params.settings.fractal_type,
                    FractalType::Mandelbox3D
                        | FractalType::OctahedralIFS3D
                        | FractalType::IcosahedralIFS3D
                        | FractalType::ApollonianGasket3D
                ) {
                    ui.separator();
                    ui.label("Advanced Shape:")
                        .on_hover_text("Fine-tune fractal geometry with folding parameters");
                    actions.changed |= ui
                        .add(
                            egui::Slider::new(&mut params.settings.fractal_fold, 0.1..=3.0)
                                .text("Fold"),
                        )
                        .on_hover_text("Fold strength - affects how space is bent")
                        .changed();

                    // Min Radius for fractals with sphere folding
                    if matches!(
                        params.settings.fractal_type,
                        FractalType::Mandelbox3D | FractalType::ApollonianGasket3D
                    ) {
                        actions.changed |= ui
                            .add(
                                egui::Slider::new(
                                    &mut params.settings.fractal_min_radius,
                                    0.1..=2.0,
                                )
                                .text("Min Radius"),
                            )
                            .on_hover_text("Minimum sphere folding radius - affects inner details")
                            .changed();
                    }
                }
            });
        self.ui_state.params_3d_open = response.openness > 0.0;
    }
}

impl UI {
    /// Render the "2D Parameters" collapsing header (2D mode).
    pub fn render_params_2d_panel(
        &mut self,
        ui: &mut egui::Ui,
        params: &mut FractalParams,
        actions: &mut UiActions,
    ) {
        let response = egui::CollapsingHeader::new("2D Parameters")
                                    .default_open(self.ui_state.params_2d_open)
                                    .show(ui, |ui| {
                                        // Hide iterations slider for Collatz (doesn't affect it)
                                        // Hide max iterations for strange attractors (they use accumulation mode)
                                        // Buddhabrot needs higher range for max iterations
                                        if params.settings.fractal_type != FractalType::Collatz2D && !params.settings.fractal_type.is_2d_attractor() {
                                            let max_iter_range = if params.settings.fractal_type.is_buddhabrot() {
                                                1..=10000 // Buddhabrot needs higher iterations for detail
                                            } else {
                                                1..=1024
                                            };
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.max_iterations, max_iter_range)
                                                .text("Max Iterations")
                                                .logarithmic(true))
                                                .on_hover_text("Number of iterations before considering a point escaped\nHigher = more detail but slower")
                                                .changed();
                                        }

                                        // Power control for escape-time fractals
                                        if matches!(params.settings.fractal_type,
                                            FractalType::Mandelbrot2D |
                                            FractalType::Julia2D |
                                            FractalType::BurningShip2D |
                                            FractalType::Tricorn2D |
                                            FractalType::Phoenix2D |
                                            FractalType::Celtic2D
                                        ) {
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.power, -32.0..=32.0)
                                                .step_by(0.1)
                                                .text("Power"))
                                                .on_hover_text("Exponent in z^n + c formula\n2 = classic, 3+ = multi-fold symmetry\nNegative values create inverse fractals")
                                                .changed();
                                        }

                                        if params.settings.fractal_type == FractalType::Julia2D {
                                            ui.label("Julia Constant (C):")
                                                .on_hover_text("The complex constant used in Julia set formula");
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.julia_c[0], -2.0..=2.0)
                                                .text("Real"))
                                                .on_hover_text("Real component of Julia constant")
                                                .changed();
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.julia_c[1], -2.0..=2.0)
                                                .text("Imaginary"))
                                                .on_hover_text("Imaginary component of Julia constant")
                                                .changed();
                                        }

                                        ui.label(format!("Center: ({:.6}, {:.6})", params.settings.center_2d[0], params.settings.center_2d[1]))
                                            .on_hover_text("Current view center (drag to pan)");
                                        ui.label(format!("Zoom: {:.4}", params.settings.zoom_2d))
                                            .on_hover_text("Current zoom level (scroll to zoom)");
                                        if ui.button("Reset View").on_hover_text("Reset center and zoom [R]").clicked() {
                                            params.settings.center_2d = [0.0, 0.0];
                                            params.settings.zoom_2d = 1.0;
                                            actions.changed = true;
                                        }

                                        // Accumulation controls for strange attractors and Buddhabrot
                                        if params.settings.fractal_type.uses_accumulation() {
                                            // Ensure accumulation is always enabled for these fractal types
                                            params.settings.attractor_accumulation_enabled = true;

                                            ui.separator();
                                            ui.label("🎯 Accumulation Settings");

                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.attractor_iterations_per_frame, 1_000..=500_000)
                                                .text("Iterations/Frame")
                                                .logarithmic(true))
                                                .on_hover_text("Number of orbit iterations per frame\nLower = better FPS, Higher = faster accumulation")
                                                .changed();

                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.attractor_log_scale, 0.5..=6.0)
                                                .text("Density Scale"))
                                                .on_hover_text("Controls saturation point (hits needed for white)\n0.5 = ~30 hits, 1.0 = ~100, 2.0 = ~1000, 3.0 = ~10k, 4.0 = ~100k")
                                                .changed();

                                            // Format numbers with commas
                                            let format_with_commas = |n: u64| -> String {
                                                let s = n.to_string();
                                                let mut result = String::new();
                                                for (i, c) in s.chars().rev().enumerate() {
                                                    if i > 0 && i % 3 == 0 {
                                                        result.push(',');
                                                    }
                                                    result.push(c);
                                                }
                                                result.chars().rev().collect()
                                            };

                                            ui.label(format!("Total: {} / {}",
                                                format_with_commas(params.accum.total_iterations),
                                                format_with_commas(params.accum.max_iterations)));

                                            ui.horizontal(|ui| {
                                                ui.label("Max:");
                                                let mut max_millions = (params.accum.max_iterations / 1_000_000) as u32;
                                                if ui.add(egui::DragValue::new(&mut max_millions)
                                                    .range(1..=100)
                                                    .suffix("M"))
                                                    .on_hover_text("Maximum iterations before auto-pause (in millions)")
                                                    .changed() {
                                                    params.accum.max_iterations = max_millions as u64 * 1_000_000;
                                                }
                                            });

                                            ui.horizontal(|ui| {
                                                let pause_text = if params.accum.paused { "▶ Resume" } else { "⏸ Pause" };
                                                if ui.button(pause_text).on_hover_text("Pause/resume accumulation").clicked() {
                                                    params.accum.paused = !params.accum.paused;
                                                }
                                                if ui.button("Clear").on_hover_text("Reset accumulated density").clicked() {
                                                    params.accum.pending_clear = true;
                                                    params.accum.total_iterations = 0;
                                                    actions.changed = true;
                                                }
                                            });

                                            // Attractor-specific parameter controls
                                            ui.separator();
                                            ui.label("🔧 Attractor Parameters");

                                            match params.settings.fractal_type {
                                                FractalType::Hopalong2D => {
                                                    // Hopalong: a, b, c parameters (range 0-10 typical)
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.settings.julia_c[0], -10.0..=10.0)
                                                        .text("a"))
                                                        .on_hover_text("Hopalong parameter a")
                                                        .changed();
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.settings.julia_c[1], -10.0..=10.0)
                                                        .text("b"))
                                                        .on_hover_text("Hopalong parameter b")
                                                        .changed();
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.settings.power, -10.0..=10.0)
                                                        .text("c"))
                                                        .on_hover_text("Hopalong parameter c")
                                                        .changed();
                                                }
                                                FractalType::Martin2D => {
                                                    // Martin: just parameter a
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.settings.julia_c[0], -10.0..=10.0)
                                                        .text("a"))
                                                        .on_hover_text("Martin parameter a (pi produces classic pattern)")
                                                        .changed();
                                                }
                                                FractalType::Gingerbreadman2D => {
                                                    ui.label("No adjustable parameters");
                                                }
                                                FractalType::Chip2D => {
                                                    // Chip: a, b, c parameters
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.settings.julia_c[0], -100.0..=100.0)
                                                        .text("a"))
                                                        .on_hover_text("Chip parameter a")
                                                        .changed();
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.settings.julia_c[1], -100.0..=100.0)
                                                        .text("b"))
                                                        .on_hover_text("Chip parameter b")
                                                        .changed();
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.settings.power, -100.0..=100.0)
                                                        .text("c"))
                                                        .on_hover_text("Chip parameter c")
                                                        .changed();
                                                }
                                                FractalType::Quadruptwo2D => {
                                                    // Quadruptwo: a, b, c parameters
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.settings.julia_c[0], -100.0..=100.0)
                                                        .text("a"))
                                                        .on_hover_text("Quadruptwo parameter a (default: 34)")
                                                        .changed();
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.settings.julia_c[1], -100.0..=100.0)
                                                        .text("b"))
                                                        .on_hover_text("Quadruptwo parameter b (default: 1)")
                                                        .changed();
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.settings.power, -100.0..=100.0)
                                                        .text("c"))
                                                        .on_hover_text("Quadruptwo parameter c (default: 5)")
                                                        .changed();
                                                }
                                                FractalType::Threeply2D => {
                                                    // Threeply: a, b, c parameters
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.settings.julia_c[0], -100.0..=100.0)
                                                        .text("a"))
                                                        .on_hover_text("Threeply parameter a (default: -55)")
                                                        .changed();
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.settings.julia_c[1], -100.0..=100.0)
                                                        .text("b"))
                                                        .on_hover_text("Threeply parameter b (default: -1)")
                                                        .changed();
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.settings.power, -100.0..=100.0)
                                                        .text("c"))
                                                        .on_hover_text("Threeply parameter c (default: -42)")
                                                        .changed();
                                                }
                                                _ => {}
                                            }

                                            // Reset to defaults button
                                            if ui.button("Reset Parameters").on_hover_text("Reset attractor parameters to defaults").clicked() {
                                                params.switch_fractal(params.settings.fractal_type);
                                                params.accum.pending_clear = true;
                                                params.accum.total_iterations = 0;
                                                actions.changed = true;
                                            }
                                        }
                                    });
        self.ui_state.params_2d_open = response.openness > 0.0;
    }
}
