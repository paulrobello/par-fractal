//! "Presets" panel — built-in/user preset gallery, save, import/export.
//! Includes the SEC-006 import-clamp detection toast.

use super::super::{UI, UiActions};
use crate::fractal::{FractalParams, Preset, PresetCategory, PresetGallery};
use glam::Vec3;

impl UI {
    /// Render the "Presets" collapsing header.
    /// Needs `camera_pos` / `camera_target` because preset save captures them.
    pub fn render_presets_panel(
        &mut self,
        ui: &mut egui::Ui,
        params: &mut FractalParams,
        actions: &mut UiActions,
        camera_pos: Vec3,
        camera_target: Vec3,
    ) {
        let response = egui::CollapsingHeader::new("Presets")
            .default_open(self.ui_state.presets_open)
            .show(ui, |ui| {
                // Category filter buttons
                ui.horizontal_wrapped(|ui| {
                    ui.label("Category:");
                    for category in PresetCategory::all_categories() {
                        if ui
                            .selectable_label(
                                self.preset_category_filter == category,
                                category.as_str(),
                            )
                            .clicked()
                        {
                            self.preset_category_filter = category;
                        }
                    }
                });

                // Search/filter box
                ui.horizontal(|ui| {
                    ui.label("🔍 Search:");
                    ui.text_edit_singleline(&mut self.preset_search)
                        .on_hover_text("Filter presets by name or description");
                    if ui.small_button("✖").on_hover_text("Clear search").clicked() {
                        self.preset_search.clear();
                    }
                });
                ui.separator();

                ui.heading("Built-in Presets");

                let builtin_presets = PresetGallery::get_builtin_presets();
                let search_lower = self.preset_search.to_lowercase();
                let filtered_builtin: Vec<&Preset> = builtin_presets
                    .iter()
                    .filter(|p| {
                        // Filter by category
                        let category_match = self.preset_category_filter == PresetCategory::All
                            || p.category == self.preset_category_filter;

                        // Filter by search text
                        let search_match = if search_lower.is_empty() {
                            true
                        } else {
                            p.name.to_lowercase().contains(&search_lower)
                                || p.description.to_lowercase().contains(&search_lower)
                        };

                        category_match && search_match
                    })
                    .collect();

                if filtered_builtin.is_empty() && !search_lower.is_empty() {
                    ui.label("No matching presets found");
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt("builtin_presets_scroll")
                        .max_height(800.0)
                        .show(ui, |ui| {
                            for preset in filtered_builtin.iter() {
                                ui.horizontal(|ui| {
                                    if ui.button(&preset.name).clicked() {
                                        actions.preset_to_load = Some((*preset).clone());
                                    }
                                    ui.label(format!("- {}", preset.description));

                                    // Add export button
                                    if ui
                                        .small_button("💾")
                                        .on_hover_text("Export this preset to JSON")
                                        .clicked()
                                    {
                                        if let Err(e) = PresetGallery::export_preset_to_json(preset)
                                        {
                                            log::error!("Failed to export preset: {}", e);
                                        } else {
                                            log::info!(
                                                "Preset '{}' exported successfully",
                                                preset.name
                                            );
                                        }
                                    }
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
                                        category.as_str(),
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
                        camera_target,
                    );

                    // Sanitize filename
                    let filename = self
                        .preset_name
                        .replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");

                    if let Err(e) = PresetGallery::save_preset(&preset, &filename) {
                        log::error!("Failed to save preset: {}", e);
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

                    let filtered_user: Vec<&String> = self
                        .user_presets
                        .iter()
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
                                            log::debug!(
                                                "User preset button clicked: {}",
                                                preset_name
                                            );
                                            match PresetGallery::load_preset(preset_name) {
                                                Ok(preset) => {
                                                    log::info!(
                                                        "Preset loaded successfully: {}",
                                                        preset.name
                                                    );
                                                    actions.preset_to_load = Some(preset);
                                                }
                                                Err(e) => {
                                                    log::error!(
                                                        "Failed to load preset '{}': {}",
                                                        preset_name,
                                                        e
                                                    );
                                                }
                                            }
                                        }

                                        // Add export button
                                        if ui
                                            .small_button("💾")
                                            .on_hover_text("Export this preset to JSON")
                                            .clicked()
                                        {
                                            match PresetGallery::load_preset(preset_name) {
                                                Ok(preset) => {
                                                    if let Err(e) =
                                                        PresetGallery::export_preset_to_json(
                                                            &preset,
                                                        )
                                                    {
                                                        log::error!(
                                                            "Failed to export preset: {}",
                                                            e
                                                        );
                                                    } else {
                                                        log::info!(
                                                            "Preset '{}' exported successfully",
                                                            preset.name
                                                        );
                                                    }
                                                }
                                                Err(e) => {
                                                    log::error!(
                                                        "Failed to load preset '{}' for export: {}",
                                                        preset_name,
                                                        e
                                                    );
                                                }
                                            }
                                        }

                                        // Add delete button
                                        if ui
                                            .small_button("🗑")
                                            .on_hover_text("Delete preset")
                                            .clicked()
                                        {
                                            if let Err(e) =
                                                PresetGallery::delete_preset(preset_name)
                                            {
                                                log::error!("Failed to delete preset: {}", e);
                                            } else {
                                                refresh_presets = true;
                                            }
                                        }
                                    });
                                }
                            });
                        if refresh_presets {
                            self.user_presets =
                                PresetGallery::list_user_presets().unwrap_or_default();
                        }
                    }
                }

                ui.separator();
                ui.heading("Import / Export");
                ui.horizontal(|ui| {
                    if ui
                        .button("📥 Export to JSON")
                        .on_hover_text("Export current settings to a JSON file")
                        .clicked()
                    {
                        let settings = params.to_settings();
                        if let Err(e) = PresetGallery::export_to_json(
                            &settings,
                            camera_pos.to_array(),
                            camera_target.to_array(),
                        ) {
                            log::error!("Failed to export settings: {}", e);
                        } else {
                            log::info!("Settings exported successfully");
                        }
                    }

                    if ui
                        .button("📤 Import from JSON")
                        .on_hover_text("Import settings from a JSON file")
                        .clicked()
                    {
                        match PresetGallery::import_from_json() {
                            Ok(preset) => {
                                // SEC-006: detect whether from_settings clamped any
                                // resource-driving field and warn the user. Reuses
                                // SEC-001's clamping path — no duplicated logic, only
                                // a pre/post comparison at the import site.
                                let raw = preset.settings.clone();
                                let clamped = FractalParams::from_settings(raw.clone());
                                let was_clamped = raw.max_iterations
                                    != clamped.settings.max_iterations
                                    || raw.max_steps != clamped.settings.max_steps
                                    || raw.attractor_iterations_per_frame
                                        != clamped.settings.attractor_iterations_per_frame
                                    || raw.shadow_samples != clamped.settings.shadow_samples
                                    || raw.dof_samples != clamped.settings.dof_samples
                                    || raw.zoom_2d != clamped.settings.zoom_2d;
                                if was_clamped {
                                    self.show_toast(
                                        "Preset values out of range were clamped".to_string(),
                                    );
                                }
                                actions.preset_to_load = Some(preset);
                            }
                            Err(e) => {
                                log::error!("Failed to import settings: {}", e);
                            }
                        }
                    }
                });
            });
        self.ui_state.presets_open = response.openness > 0.0;
    }
}
