//! "About Par Fractal" window — version, author, what's new, feature list.

use super::super::UI;
use egui::Context;

impl UI {
    /// Render the About window. Owned by this module per AUDIT QA-009.
    pub fn render_about_window(&mut self, ctx: &Context) {
        if !self.ui_state.about_window_open {
            return;
        }
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

                ui.collapsing("What's New in v0.9.0", |ui| {
                    ui.label("• Upgraded egui 0.34 → 0.35");
                    ui.label("• Migrated to Rust edition 2024");
                    ui.label("• Updated all dependencies to latest versions");
                });

                ui.collapsing("What's New in v0.8.2", |ui| {
                    ui.label("• Upgraded wgpu 27 → 29 (major API migration)");
                    ui.label("• Upgraded egui 0.33 → 0.34");
                    ui.label("• Upgraded glam 0.31 → 0.32, rand 0.9 → 0.10");
                    ui.label("• Updated all web/wasm dependencies");
                });

                ui.collapsing("What's New in v0.8.0", |ui| {
                    ui.label("• Quality level CLI parameter (--quality / -q)");
                    ui.label("• URL parameters for web (quality, preset)");
                    ui.label("• Updated dependencies to latest versions");
                });

                ui.collapsing("What's New in v0.7.x", |ui| {
                    ui.label("• Fixed Buddhabrot high-resolution screenshot capture");
                    ui.label("• Buddhabrot - density visualization of escape trajectories");
                    ui.label("• Compute shader accumulation for Buddhabrot rendering");
                    ui.label("• New preset: Buddhabrot Classic");
                });

                ui.collapsing("Features", |ui| {
                    ui.label("• 20 2D fractals (13 escape-time + 1 density + 6 attractors)");
                    ui.label("• 15 3D fractals (12 ray-marched + 3 attractors)");
                    ui.label("• Variable power for 6 escape-time fractals");
                    ui.label("• 48 static + 12 procedural color palettes");
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
}
