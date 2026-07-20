//! "2D Parameters", "3D Parameters", "Ray Marching", and "Shading" panels —
//! the core per-fractal/per-mode rendering knobs.

use super::super::{UI, UiActions};
use crate::deep_zoom::orbit::parse_center_decimal;
use crate::fractal::{FRACTAL_SCALE_RANGE, FractalParams, FractalType, ShadingModel};

/// Parse a "Go to" location string into `(re, im, optional zoom)` for the 2D
/// precise-center entry (ENH-001 Phase C).
///
/// Accepted forms: `re, im` and `re, im @ zoom`. Both coordinates are validated
/// to parse as decimal numbers (via [`parse_center_decimal`]); the returned
/// strings are the trimmed raw decimals (the driver re-parses them at the
/// orbit's zoom-derived precision). Returns `None` on any malformed input.
fn parse_location_input(s: &str) -> Option<(String, String, Option<f64>)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // Optional " @ zoom" suffix.
    let (coords, zoom) = match s.split_once('@') {
        Some((c, z)) => {
            let z = z.trim().parse::<f64>().ok()?;
            (c, Some(z))
        }
        None => (s, None),
    };
    let (re, im) = coords.split_once(',')?;
    let re = re.trim();
    let im = im.trim();
    if re.is_empty() || im.is_empty() {
        return None;
    }
    // Validate both parse as decimal coordinates — any precision suffices here
    // (the driver re-parses at the orbit's zoom-derived precision at view time).
    parse_center_decimal(re, 64).ok()?;
    parse_center_decimal(im, 64).ok()?;
    Some((re.to_string(), im.to_string(), zoom))
}

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
                        egui::Slider::new(&mut params.settings.fractal_scale, FRACTAL_SCALE_RANGE)
                            // Spans two decades now that the 3D attractors (0.05 to
                            // 0.3) are representable; linear would bunch all of them
                            // into the leftmost pixels.
                            .logarithmic(true)
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
                                            // Cap stays below the SEC-001 safety clamp
                                            // (max_iterations.clamp(1, 100_000) in fractal/mod.rs) so the slider
                                            // and the preset/import clamp agree. Logarithmic scale keeps the
                                            // default reachable; deep zoom adds more via the zoom-bonus.
                                            let max_iter_range = 1..=10_000;
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

                                        // Center — high precision; shows the exact decimal center when a
                                        // precise override is active (ENH-001 Phase C).
                                        let precise_active = params.settings.center_2d_precise.is_some();
                                        let center_text = match &params.settings.center_2d_precise {
                                            Some(p) => format!("Center: ({}, {})", p[0], p[1]),
                                            None => format!(
                                                "Center: ({:?}, {:?})",
                                                params.settings.center_2d[0], params.settings.center_2d[1]
                                            ),
                                        };
                                        ui.label(center_text).on_hover_text(format!(
                                            "Current view center (drag to pan).{}\n\
                                             Paste a high-precision coordinate below to render zoom past ~1e15 \
                                             (ENH-001 Phase C perturbation).",
                                            if precise_active {
                                                "\n🔒 Precise decimal center active — the reference orbit uses \
                                                 this exact value; pan / zoom-at-cursor clears it."
                                            } else {
                                                ""
                                            }
                                        ));
                                        let zoom = params.settings.zoom_2d;
                                        ui.label(format!("Zoom: {}", format_zoom(zoom)))
                                            .on_hover_text(format!(
                                                "Current zoom level (scroll / shift-drag to zoom).\n\
                                                 log₁₀ ≈ {:.2}    log₂ ≈ {:.2}{}\n\
                                                 Deep-zoom perturbation (ENH-001) engages past log₂ 13.3 \
                                                 (~1e4) for Mandelbrot / Julia / Burning Ship / Tricorn \
                                                 in 2D mode.",
                                                zoom.log10(),
                                                zoom.log2(),
                                                if crate::deep_zoom::perturbation_eligible(
                                                    zoom,
                                                    params.settings.fractal_type,
                                                    params.settings.render_mode,
                                                ) {
                                                    "\n● Perturbation active."
                                                } else {
                                                    ""
                                                },
                                            ));
                                        // Precise-center "Go to" (ENH-001 Phase C): paste "re, im"
                                        // (optionally "re, im @ zoom") to jump to a high-precision
                                        // coordinate the f64 mirror cannot represent.
                                        ui.horizontal(|ui| {
                                            ui.label("Go to:");
                                            ui.add(
                                                egui::TextEdit::singleline(&mut self.precise_center_input)
                                                    .hint_text("re, im   e.g. -0.743643887037151071, 0.1318259042")
                                                    .desired_width(200.0),
                                            );
                                            if ui
                                                .button("Set")
                                                .on_hover_text(
                                                    "Jump to the pasted coordinate (sets the high-precision center)",
                                                )
                                                .clicked()
                                                && !self.precise_center_input.trim().is_empty()
                                            {
                                                match parse_location_input(&self.precise_center_input) {
                                                    Some((re_s, im_s, zoom)) => {
                                                        // Mirror into the f64 center so the rest of the app
                                                        // (panning, display, non-perturbation rendering) sees
                                                        // the right location; clamp to the f64 band.
                                                        let re_f = parse_center_decimal(&re_s, 64)
                                                            .unwrap()
                                                            .to_f64()
                                                            .value()
                                                            .clamp(-1e15, 1e15);
                                                        let im_f = parse_center_decimal(&im_s, 64)
                                                            .unwrap()
                                                            .to_f64()
                                                            .value()
                                                            .clamp(-1e15, 1e15);
                                                        params.settings.center_2d = [re_f, im_f];
                                                        params.settings.center_2d_precise = Some([re_s, im_s]);
                                                        if let Some(z) = zoom {
                                                            params.settings.zoom_2d = z.max(1e-6);
                                                        }
                                                        actions.changed = true;
                                                        self.precise_center_input.clear();
                                                    }
                                                    None => {
                                                        self.show_toast(
                                                            "Could not parse center. Use: re, im  (e.g. -0.7436, 0.1318)"
                                                                .to_string(),
                                                        );
                                                    }
                                                }
                                            }
                                            if ui
                                                .button("Copy")
                                                .on_hover_text("Copy location as 're, im @ zoom'")
                                                .clicked()
                                            {
                                                let (re, im) = match &params.settings.center_2d_precise {
                                                    Some(p) => (p[0].clone(), p[1].clone()),
                                                    None => (
                                                        format!("{:?}", params.settings.center_2d[0]),
                                                        format!("{:?}", params.settings.center_2d[1]),
                                                    ),
                                                };
                                                ui.ctx().copy_text(format!(
                                                    "{} {}, @ {}",
                                                    re,
                                                    im,
                                                    format_zoom(params.settings.zoom_2d)
                                                ));
                                                self.show_toast("Location copied".to_string());
                                            }
                                            if precise_active
                                                && ui
                                                    .button("Clear")
                                                    .on_hover_text(
                                                        "Drop the precise center; resume f64-precision navigation",
                                                    )
                                                    .clicked()
                                            {
                                                params.settings.center_2d_precise = None;
                                                actions.changed = true;
                                            }
                                        });

                                        if ui.button("Reset View").on_hover_text("Reset center and zoom [R]").clicked() {
                                            params.settings.center_2d = [0.0, 0.0];
                                            params.settings.center_2d_precise = None;
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

/// Render an integer as Unicode superscript digits (e.g. `-45` → `⁻⁴⁵`), for the
/// `10ⁿ` deep-zoom readout. egui's default font includes these codepoints.
fn superscript_int(n: i32) -> String {
    const SUP: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    let mut s = String::new();
    if n < 0 {
        s.push('⁻');
    }
    for d in n.unsigned_abs().to_string().chars() {
        s.push(SUP[d.to_digit(10).unwrap_or(0) as usize]);
    }
    s
}

/// Format a 2D magnification for display. Shallow zoom keeps the readable
/// fixed-point form; deep zoom (≥ 1e4) collapses to a power-of-ten readout
/// (`≈ 1.23×10⁴⁵`) so the level stays legible at extreme depths where the raw
/// f64 prints dozens of digits. ENH-001 Phase C.
fn format_zoom(zoom: f64) -> String {
    if !zoom.is_finite() || zoom < 1e4 {
        format!("{:.4}", zoom)
    } else {
        let exp = zoom.log10().floor() as i32;
        // Guard against powi overflow / NaN at absurd values.
        let mantissa = if (-30..=308).contains(&exp) {
            zoom / 10f64.powi(exp)
        } else {
            1.0
        };
        format!("≈ {:.2}×10{}", mantissa, superscript_int(exp))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn superscript_digits_and_sign() {
        assert_eq!(superscript_int(0), "⁰");
        assert_eq!(superscript_int(7), "⁷");
        assert_eq!(superscript_int(45), "⁴⁵");
        assert_eq!(superscript_int(-3), "⁻³");
    }

    #[test]
    fn zoom_readout_shallow_stays_fixed() {
        // Below the 1e4 threshold the readable fixed-point form is preserved.
        assert_eq!(format_zoom(1.0), "1.0000");
        assert_eq!(format_zoom(256.0), "256.0000");
        assert_eq!(format_zoom(9999.0), "9999.0000");
        // Non-finite doesn't panic.
        assert_eq!(format_zoom(f64::NAN), "NaN"); // {:.4} of NaN is "NaN"
    }

    #[test]
    fn zoom_readout_deep_is_power_of_ten() {
        // At and above 1e4 the readout collapses to ≈ M.MM×10ⁿ.
        let s = format_zoom(1.6e7);
        assert!(s.starts_with("≈ "), "got {s}");
        assert!(s.contains("×10⁷"), "expected 10⁷ term, got {s}");
        // 1e45 renders as ≈ 1.00×10⁴⁵ — readable, not a 46-digit string.
        let deep = format_zoom(1e45);
        assert!(deep.contains("×10⁴⁵"), "got {deep}");
        assert!(
            deep.len() < 20,
            "deep readout should be compact, got {deep}"
        );
    }

    // ---- ENH-001 Phase C: precise-center "Go to" input parsing ----

    #[test]
    fn parse_location_plain_pair() {
        let (re, im, zoom) = parse_location_input("-0.743643887037151071, 0.1318259042").unwrap();
        assert_eq!(re, "-0.743643887037151071");
        assert_eq!(im, "0.1318259042");
        assert!(zoom.is_none(), "no @ suffix → no zoom");
    }

    #[test]
    fn parse_location_with_zoom_suffix() {
        let (re, im, zoom) = parse_location_input("  -0.5 , 0.0 @ 1e20 ").unwrap();
        assert_eq!(re, "-0.5");
        assert_eq!(im, "0.0");
        assert_eq!(zoom, Some(1e20));
    }

    #[test]
    fn parse_location_scientific_coords() {
        // Coordinates in scientific notation parse (DBig accepts `E`).
        let (re, im, _) = parse_location_input("1.5e-3, -2e0").unwrap();
        assert_eq!(re, "1.5e-3");
        assert_eq!(im, "-2e0");
    }

    #[test]
    fn parse_location_rejects_garbage() {
        assert!(parse_location_input("not a coordinate").is_none());
        assert!(
            parse_location_input("-0.7").is_none(),
            "need both re and im"
        );
        assert!(parse_location_input("").is_none());
        // Bad zoom suffix.
        assert!(parse_location_input("-0.5, 0.0 @ huge").is_none());
    }
}
