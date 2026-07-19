//! "Settings" and "Controls" panels — application preferences, GPU selection,
//! and keybindings help.

use super::super::{UI, UiActions};
use crate::fractal::FractalParams;

impl UI {
    /// Render the "Controls" keybindings help collapsing header.
    pub fn render_controls_panel(
        &mut self,
        ui: &mut egui::Ui,
        params: &mut FractalParams,
        _actions: &mut UiActions,
    ) {
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

                match params.settings.render_mode {
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
    }
}

impl UI {
    /// Render the "Settings" collapsing header — save/reset, GPU selection.
    pub fn render_settings_panel(
        &mut self,
        ui: &mut egui::Ui,
        params: &mut FractalParams,
        actions: &mut UiActions,
    ) {
        let response = egui::CollapsingHeader::new("Settings")
                            .default_open(self.ui_state.settings_open)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if ui.button("💾 Save Settings")
                                        .on_hover_text("Manually save current settings to disk")
                                        .clicked()
                                        && let Err(e) = params.save_to_file() {
                                            log::error!("Failed to save settings: {}", e);
                                        }
                                    if ui.button("🔄 Reset to Defaults")
                                        .on_hover_text("Reset all settings to default values")
                                        .clicked() {
                                        actions.reset_requested = true;
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
                                        actions.gpu_scan_requested = true;
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

                                // ENH-006: GPU profile HUD overlay toggle.
                                // Global (any render mode); the checkbox is
                                // mirrored to `UIState` for persistence.
                                ui.separator();
                                ui.heading("Diagnostics");
                                if ui
                                    .checkbox(&mut self.show_gpu_profile, "Show GPU Profile Overlay")
                                    .on_hover_text(
                                        "Display per-scope GPU timings and CPU frame ms [Shift+G]",
                                    )
                                    .changed()
                                {
                                    self.ui_state.show_gpu_profile = self.show_gpu_profile;
                                }
                            });
        self.ui_state.settings_open = response.openness > 0.0;
    }
}
