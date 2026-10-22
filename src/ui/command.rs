use super::UI;
use crate::command_palette::{CommandAction, EffectType};
use crate::fractal::{FractalParams, PresetGallery};
use egui::Context;

/// Command palette UI methods
impl UI {
    pub fn render_command_palette(&mut self, ctx: &Context) -> Option<CommandAction> {
        if !self.command_palette.open {
            return None;
        }

        let mut command_to_execute = None;
        let mut close_palette = false;

        egui::Window::new("Command Palette")
            .anchor(egui::Align2::CENTER_TOP, [0.0, 100.0])
            .fixed_size([700.0, 400.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("🔍 Quick Command");
                ui.add_space(8.0);

                // Search input
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.command_palette.query)
                        .hint_text("Type to search commands...")
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Heading),
                );

                // Auto-focus the search box
                if response.changed() {
                    self.command_palette
                        .set_query(self.command_palette.query.clone());
                }

                if response.ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close_palette = true;
                }

                if response.ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    self.command_palette.select_next();
                }

                if response.ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    self.command_palette.select_previous();
                }

                if response.ctx.input(|i| i.key_pressed(egui::Key::Enter))
                    && let Some(cmd) = self.command_palette.get_selected_command()
                {
                    command_to_execute = Some(cmd.action.clone());
                    close_palette = true;
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // Results list
                egui::ScrollArea::vertical()
                    .id_salt("command_palette_scroll")
                    .max_height(260.0)
                    .show(ui, |ui| {
                        if self.command_palette.filtered_commands.is_empty() {
                            ui.label(egui::RichText::new("No commands found").italics().weak());
                        } else {
                            for (idx, (score, cmd)) in
                                self.command_palette.filtered_commands.iter().enumerate()
                            {
                                let is_selected = idx == self.command_palette.selected_index;

                                let text = if is_selected {
                                    egui::RichText::new(&cmd.name)
                                        .strong()
                                        .color(egui::Color32::WHITE)
                                } else {
                                    egui::RichText::new(&cmd.name)
                                };

                                let response = ui.selectable_label(is_selected, text);

                                if response.clicked() {
                                    command_to_execute = Some(cmd.action.clone());
                                    close_palette = true;
                                }

                                // Show category, description, and shortcut on same line
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} {}",
                                            cmd.category.icon(),
                                            cmd.category.name()
                                        ))
                                        .small()
                                        .weak(),
                                    );
                                    ui.label(egui::RichText::new(&cmd.description).small().weak());

                                    if let Some(shortcut) = &cmd.shortcut {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    egui::RichText::new(format!("[{}]", shortcut))
                                                        .small()
                                                        .monospace()
                                                        .weak(),
                                                );
                                            },
                                        );
                                    }
                                });

                                if !self.command_palette.query.is_empty() {
                                    ui.label(
                                        egui::RichText::new(format!("Score: {:.0}", score))
                                            .small()
                                            .weak()
                                            .italics(),
                                    );
                                }

                                ui.add_space(4.0);
                            }
                        }
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("[Up/Down]").monospace().weak());
                    ui.label(egui::RichText::new("Navigate").small().weak());
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("[Enter]").monospace().weak());
                    ui.label(egui::RichText::new("Execute").small().weak());
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("[Esc]").monospace().weak());
                    ui.label(egui::RichText::new("Close").small().weak());
                });
            });

        if close_palette {
            self.command_palette.close();
        }

        command_to_execute
    }

    /// Execute a command action and return (changed, result_message)
    /// QA-035: parameter/rendering commands — extracted from
    /// `execute_command` (complexity 52). Returns `None` when the action
    /// is not one of this group's.
    /// QA-035: iteration/power/orbit-speed adjusters and the reset
    /// requests, extracted from `execute_param_command`.
    #[allow(unused_assignments)] // the changed/message match-arm pattern assigns per arm
    fn execute_adjust_command(
        &mut self,
        action: &CommandAction,
        params: &mut FractalParams,
    ) -> Option<(bool, Option<String>)> {
        let mut changed = false;
        let mut message = None;
        let action = action.clone();
        match action {
            CommandAction::IncrementIterations => {
                let value = params.adjust_iterations(32);
                changed = true;
                let label = if params.settings.render_mode == crate::fractal::RenderMode::TwoD {
                    format!("Max iterations: {value}")
                } else {
                    format!("Max steps: {value}")
                };
                message = Some(label);
            }
            CommandAction::DecrementIterations => {
                let value = params.adjust_iterations(-32);
                changed = true;
                let label = if params.settings.render_mode == crate::fractal::RenderMode::TwoD {
                    format!("Max iterations: {value}")
                } else {
                    format!("Max steps: {value}")
                };
                message = Some(label);
            }
            CommandAction::IncrementPower => {
                let value = params.adjust_power(0.5);
                changed = true;
                message = Some(format!("Power: {value:.1}"));
            }
            CommandAction::DecrementPower => {
                let value = params.adjust_power(-0.5);
                changed = true;
                message = Some(format!("Power: {value:.1}"));
            }
            CommandAction::IncrementOrbitSpeed => {
                let value = params.adjust_orbit_speed(0.1);
                message = Some(format!("Orbit speed: {value:.2}"));
            }
            CommandAction::DecrementOrbitSpeed => {
                let value = params.adjust_orbit_speed(-0.1);
                message = Some(format!("Orbit speed: {value:.2}"));
            }
            CommandAction::ResetView => {
                // QA-033: request the reset; App applies it through the same
                // shared `App::reset_view` path the panel buttons and the R
                // key use (per-type 2D view / framed 3D camera, stale
                // precise center cleared).
                self.pending_reset_view = true;
                message = Some("View reset".to_string());
            }
            CommandAction::ResetAll => {
                // QA-033: same routing as ResetView — through
                // `App::reset_all`, not a direct params mutation here.
                self.pending_reset_all = true;
                message = Some("All settings reset to defaults".to_string());
            }
            _ => return None,
        }
        Some((changed, message))
    }

    #[allow(unused_assignments)] // the changed/message match-arm pattern assigns per arm
    fn execute_param_command(
        &mut self,
        action: &CommandAction,
        params: &mut FractalParams,
    ) -> Option<(bool, Option<String>)> {
        if let Some(result) = self.execute_adjust_command(action, params) {
            return Some(result);
        }
        let mut changed = false;
        let mut message = None;
        let action = action.clone();
        match action {
            CommandAction::SetFractalType(ftype) => {
                params.switch_fractal(ftype);
                changed = true;
                message = Some(format!("Switched to {:?}", ftype));
            }
            CommandAction::SetColorMode(mode) => {
                params.settings.color_mode = mode;
                changed = true;
                message = Some(format!("Color mode: {:?}", mode));
            }
            CommandAction::SetShadowMode(mode) => {
                params.settings.shadow_mode = mode;
                changed = true;
                let mode_name = match mode {
                    0 => "OFF",
                    1 => "HARD",
                    _ => "SOFT",
                };
                message = Some(format!("Shadow mode: {}", mode_name));
            }
            CommandAction::SetShadingModel(model) => {
                params.settings.shading_model = model;
                changed = true;
                message = Some(format!("Shading: {:?}", model));
            }
            CommandAction::SetLODProfile(profile) => {
                params.lod.lod_config.apply_profile(profile);
                changed = true;
                message = Some(format!(
                    "LOD Profile: {}",
                    params.lod.lod_config.profile_name()
                ));
            }
            CommandAction::ToggleLOD => {
                params.lod.lod_config.enabled = !params.lod.lod_config.enabled;
                changed = true;
                message = Some(format!(
                    "LOD System: {}",
                    if params.lod.lod_config.enabled {
                        "ON"
                    } else {
                        "OFF"
                    }
                ));
            }
            CommandAction::ToggleLODDebug => {
                params.lod.lod_config.debug_visualization =
                    !params.lod.lod_config.debug_visualization;
                changed = true;
                message = Some(format!(
                    "LOD Debug: {}",
                    if params.lod.lod_config.debug_visualization {
                        "ON"
                    } else {
                        "OFF"
                    }
                ));
            }
            CommandAction::ToggleEffect(effect) => {
                let (new_value, name) = match effect {
                    EffectType::AmbientOcclusion => {
                        params.settings.ambient_occlusion = !params.settings.ambient_occlusion;
                        (params.settings.ambient_occlusion, "Ambient Occlusion")
                    }
                    EffectType::Shadows => {
                        // Cycle shadow mode: 0 -> 1 -> 2 -> 0
                        params.settings.shadow_mode = (params.settings.shadow_mode + 1) % 3;
                        (params.settings.shadow_mode > 0, "Shadows")
                    }
                    EffectType::SoftShadows => {
                        // Toggle soft shadows (mode 0 or 2)
                        params.settings.shadow_mode = if params.settings.shadow_mode == 2 {
                            0
                        } else {
                            2
                        };
                        (params.settings.shadow_mode == 2, "Soft Shadows")
                    }
                    EffectType::DepthOfField => {
                        params.settings.depth_of_field = !params.settings.depth_of_field;
                        (params.settings.depth_of_field, "Depth of Field")
                    }
                    EffectType::Fog => {
                        params.settings.fog_enabled = !params.settings.fog_enabled;
                        (params.settings.fog_enabled, "Fog")
                    }
                    EffectType::Bloom => {
                        params.settings.bloom_enabled = !params.settings.bloom_enabled;
                        (params.settings.bloom_enabled, "Bloom")
                    }
                    EffectType::Vignette => {
                        params.settings.vignette_enabled = !params.settings.vignette_enabled;
                        (params.settings.vignette_enabled, "Vignette")
                    }
                    EffectType::FXAA => {
                        params.settings.fxaa_enabled = !params.settings.fxaa_enabled;
                        (params.settings.fxaa_enabled, "FXAA")
                    }
                    EffectType::SSR => {
                        params.settings.floor_reflections = !params.settings.floor_reflections;
                        (params.settings.floor_reflections, "Floor Reflections")
                    }
                    EffectType::Floor => {
                        params.settings.show_floor = !params.settings.show_floor;
                        (params.settings.show_floor, "Floor")
                    }
                    EffectType::AutoOrbit => {
                        params.settings.auto_orbit = !params.settings.auto_orbit;
                        (params.settings.auto_orbit, "Auto-Orbit")
                    }
                };
                changed = true;
                message = Some(format!(
                    "{}: {}",
                    name,
                    if new_value { "ON" } else { "OFF" }
                ));
            }
            CommandAction::CyclePalette => {
                params.next_palette();
                changed = true;
                message = Some(format!("Static Palette: {}", params.settings.palette.name));
            }
            CommandAction::CycleProceduralPalette => {
                use crate::fractal::ProceduralPalette;
                // Cycle through procedural palettes including None
                let all_options: Vec<ProceduralPalette> = std::iter::once(ProceduralPalette::None)
                    .chain(ProceduralPalette::ALL.iter().copied())
                    .collect();
                let current_idx = all_options
                    .iter()
                    .position(|p| *p == params.settings.procedural_palette)
                    .unwrap_or(0);
                let next_idx = (current_idx + 1) % all_options.len();
                params.settings.procedural_palette = all_options[next_idx];
                changed = true;
                message = Some(format!(
                    "Procedural Palette: {}",
                    params.settings.procedural_palette.name()
                ));
            }
            CommandAction::LoadPreset(name) => match PresetGallery::load_preset(&name) {
                Ok(preset) => {
                    *params = FractalParams::from_settings(preset.settings);
                    changed = true;
                    message = Some(format!("Loaded preset: {}", name));
                }
                Err(e) => {
                    message = Some(format!("Failed to load preset: {}", e));
                }
            },
            // These commands need additional UI dialogs or are handled elsewhere
            CommandAction::SavePreset => {
                message = Some("Open the Presets panel to save".to_string());
            }
            _ => return None,
        }
        Some((changed, message))
    }

    /// QA-035: app/UI-state commands (extracted; see
    /// [`Self::execute_param_command`]).
    #[allow(unused_assignments)] // the changed/message match-arm pattern assigns per arm
    fn execute_app_command(&mut self, action: &CommandAction) -> Option<(bool, Option<String>)> {
        let changed = false;
        let mut message = None;
        let action = action.clone();
        match action {
            CommandAction::ToggleUI => {
                self.show_ui = !self.show_ui;
                message = Some(format!("UI: {}", if self.show_ui { "ON" } else { "OFF" }));
            }
            CommandAction::ToggleStats => {
                self.show_performance_overlay = !self.show_performance_overlay;
                message = Some(format!(
                    "Performance Overlay: {}",
                    if self.show_performance_overlay {
                        "ON"
                    } else {
                        "OFF"
                    }
                ));
            }
            CommandAction::ToggleFPS => {
                self.show_fps = !self.show_fps;
                self.ui_state.show_fps = self.show_fps;
                message = Some(format!(
                    "FPS Counter: {}",
                    if self.show_fps { "ON" } else { "OFF" }
                ));
            }
            CommandAction::CycleTheme => {
                self.dark_theme = !self.dark_theme;
                message = Some(format!(
                    "Theme: {}",
                    if self.dark_theme { "Dark" } else { "Light" }
                ));
            }
            CommandAction::ExportSettings => {
                message = Some("Open the Settings panel to export".to_string());
            }
            CommandAction::ImportSettings => {
                message = Some("Open the Settings panel to import".to_string());
            }
            CommandAction::StartRecording(_) => {
                message = Some("Open the Recording panel to start recording".to_string());
            }
            CommandAction::StopRecording => {
                message = Some("Open the Recording panel to stop recording".to_string());
            }
            CommandAction::Screenshot => {
                message = Some("Screenshot feature not exposed to command palette yet".to_string());
            }
            _ => return None,
        }
        Some((changed, message))
    }

    pub fn execute_command(
        &mut self,
        action: CommandAction,
        params: &mut FractalParams,
    ) -> (bool, Option<String>) {
        // QA-035: three-group dispatch keeps every match under the
        // complexity budget.
        if let Some(result) = self.execute_param_command(&action, params) {
            return result;
        }
        if let Some(result) = self.execute_app_command(&action) {
            return result;
        }
        // Note: History tracking is handled by the UI's existing undo/redo
        // system. Actions outside the two groups above are not palette
        // commands — nothing to do.

        (false, None)
    }
}
