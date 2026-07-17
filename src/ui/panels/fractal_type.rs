//! "Fractal Type" panel — fractal selection grid (2D / density / attractors / 3D).

use super::super::{UI, UiActions};
use crate::fractal::{FractalParams, FractalType};

impl UI {
    /// Render the "Fractal Type" collapsing header.
    pub fn render_fractal_type_panel(
        &mut self,
        ui: &mut egui::Ui,
        params: &mut FractalParams,
        actions: &mut UiActions,
    ) {
        let response = egui::CollapsingHeader::new("Fractal Type")
            .default_open(self.ui_state.fractal_type_open)
            .show(ui, |ui| {
                let old_type = params.settings.fractal_type;
                ui.label("2D Fractals:");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::Mandelbrot2D, "Mandelbrot")
                        .on_hover_text("Classic Mandelbrot set - infinite detail fractal [1]");
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::Julia2D, "Julia")
                        .on_hover_text("Julia set - beautiful variations with complex parameter [2]");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::Sierpinski2D, "Sierpinski Carpet")
                        .on_hover_text("Sierpinski carpet - recursive square pattern [3]");
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::SierpinskiTriangle2D, "Sierpinski Triangle")
                        .on_hover_text("Sierpinski triangle - classic recursive triangle pattern");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::BurningShip2D, "Burning Ship")
                        .on_hover_text("Burning Ship fractal - variant with absolute values [4]");
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::Tricorn2D, "Tricorn")
                        .on_hover_text("Tricorn - Mandelbrot with conjugate iteration [5]");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::Phoenix2D, "Phoenix")
                        .on_hover_text("Phoenix fractal - dynamic iteration algorithm [6]");
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::Celtic2D, "Celtic")
                        .on_hover_text("Celtic fractal - alternative complex iteration [7]");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::Newton2D, "Newton")
                        .on_hover_text("Newton fractal - polynomial root-finding visualization [8]");
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::Lyapunov2D, "Lyapunov")
                        .on_hover_text("Lyapunov fractal - stability diagram patterns [9]");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::Nova2D, "Nova")
                        .on_hover_text("Nova fractal - Newton-Mandelbrot hybrid [0]");
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::Magnet2D, "Magnet")
                        .on_hover_text("Magnet Type 1 - physics-inspired fractal");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::Collatz2D, "Collatz")
                        .on_hover_text("Collatz fractal - based on Collatz conjecture");
                });

                ui.separator();
                ui.label("2D Density Fractals:");
                // Buddhabrot needs accumulation mode enabled
                {
                    let selected = params.settings.fractal_type == FractalType::Buddhabrot2D;
                    if ui.selectable_label(selected, "Buddhabrot").on_hover_text("Buddhabrot - Mandelbrot escape trajectory density visualization (discovered by Melinda Green, 1993)").clicked() {
                        params.settings.fractal_type = FractalType::Buddhabrot2D;
                        params.settings.attractor_accumulation_enabled = true;
                        params.accum.pending_clear = true;
                        params.accum.total_iterations = 0;
                        actions.changed = true;
                    }
                }

                ui.separator();
                ui.label("2D Strange Attractors:");
                // Helper macro-like closure to create attractor buttons that auto-enable accumulation
                let mut attractor_button = |ui: &mut egui::Ui, fractal: FractalType, label: &str, hover: &str| {
                    let selected = params.settings.fractal_type == fractal;
                    if ui.selectable_label(selected, label).on_hover_text(hover).clicked() {
                        params.settings.fractal_type = fractal;
                        params.settings.attractor_accumulation_enabled = true;
                        params.accum.pending_clear = true;
                        params.accum.total_iterations = 0;
                        actions.changed = true;
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
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::Mandelbulb3D, "Mandelbulb")
                        .on_hover_text("3D Mandelbrot with adjustable power [F1]");
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::MengerSponge3D, "Menger Sponge")
                        .on_hover_text("Recursive cubic structure with infinite holes [F2]");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::SierpinskiPyramid3D, "Sierpinski Pyramid")
                        .on_hover_text("3D Sierpinski pyramid - recursive tetrahedron [F3]");
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::SierpinskiGasket3D, "Sierpinski Gasket")
                        .on_hover_text("3D Sierpinski gasket - sphere packing structure");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::JuliaSet3D, "Julia 3D")
                        .on_hover_text("3D Julia set with quaternion math [F4]");
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::Mandelbox3D, "Mandelbox")
                        .on_hover_text("Cubic folding fractal with sharp edges [F5]");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::OctahedralIFS3D, "Octahedron IFS")
                        .on_hover_text("Kaleidoscopic IFS with 8-fold symmetry [F6]");
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::IcosahedralIFS3D, "Icosahedron IFS")
                        .on_hover_text("Kaleidoscopic IFS with 20-fold symmetry [F7]");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::ApollonianGasket3D, "Apollonian Gasket")
                        .on_hover_text("Beautiful sphere-packing fractal [F8]");
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::Kleinian3D, "Kleinian")
                        .on_hover_text("Kleinian group fractal [F9]");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::HybridMandelbulbJulia3D, "Hybrid Bulb-Julia")
                        .on_hover_text("Mandelbulb and Julia set hybrid [F10]");
                    ui.selectable_value(&mut params.settings.fractal_type, FractalType::QuaternionCubic3D, "Quaternion Cubic")
                        .on_hover_text("Cubic quaternion Julia set (z³+c)");
                });

                // NOTE: 3D Strange Attractors disabled - ray marching point clouds
                // is too expensive (causes GPU timeout). Requires different rendering
                // approach (instanced points or volumetric). See todos.md.

                if old_type != params.settings.fractal_type {
                    params.switch_fractal(params.settings.fractal_type);
                    actions.changed = true;
                }
            });
        self.ui_state.fractal_type_open = response.openness > 0.0;
    }
}
