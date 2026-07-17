//! "LOD System" panel — adaptive quality configuration (3D mode only).

use super::super::{UI, UiActions};
use crate::fractal::FractalParams;

impl UI {
    /// Render the "LOD System" collapsing header.
    /// 3D-only — wrapped by the `match render_mode { ThreeD => ... }` in the orchestrator.
    pub fn render_lod_panel(
        &mut self,
        ui: &mut egui::Ui,
        params: &mut FractalParams,
        actions: &mut UiActions,
    ) {
        let response = egui::CollapsingHeader::new("LOD System")
                                    .default_open(self.ui_state.lod_open)
                                    .show(ui, |ui| {
                                        ui.label("Adaptive quality system for smooth performance")
                                            .on_hover_text("Automatically adjusts rendering quality based on distance, motion, and performance");

                                        ui.separator();

                                        // Main Controls
                                        actions.changed |= ui.checkbox(&mut params.lod.lod_config.enabled, "Enable LOD System")
                                            .on_hover_text("Enable adaptive quality adjustment (disabled by default)")
                                            .changed();

                                        if params.lod.lod_config.enabled {
                                            ui.separator();

                                            // Profile Selection
                                            ui.label("Profile:");
                                            let profile_changed = egui::ComboBox::from_id_salt("lod_profile")
                                                .selected_text(params.lod.lod_config.profile_name())
                                                .show_ui(ui, |ui| {
                                                    use crate::lod::LODProfile;
                                                    let mut changed_local = false;

                                                    changed_local |= ui.selectable_value(&mut params.lod.lod_config.profile, LODProfile::Balanced, "Balanced")
                                                        .on_hover_text("Good mix of quality and performance (default)")
                                                        .changed();
                                                    changed_local |= ui.selectable_value(&mut params.lod.lod_config.profile, LODProfile::QualityFirst, "Quality First")
                                                        .on_hover_text("Prioritize visual quality, less aggressive LOD")
                                                        .changed();
                                                    changed_local |= ui.selectable_value(&mut params.lod.lod_config.profile, LODProfile::PerformanceFirst, "Performance First")
                                                        .on_hover_text("Prioritize performance, aggressive LOD")
                                                        .changed();
                                                    changed_local |= ui.selectable_value(&mut params.lod.lod_config.profile, LODProfile::DistanceOnly, "Distance Only")
                                                        .on_hover_text("Only use distance-based LOD")
                                                        .changed();
                                                    changed_local |= ui.selectable_value(&mut params.lod.lod_config.profile, LODProfile::MotionOnly, "Motion Only")
                                                        .on_hover_text("Only reduce quality during camera movement")
                                                        .changed();
                                                    changed_local |= ui.selectable_value(&mut params.lod.lod_config.profile, LODProfile::Custom, "Custom")
                                                        .on_hover_text("User-defined configuration")
                                                        .changed();

                                                    changed_local
                                                })
                                                .inner.unwrap_or(false);

                                            // Apply profile if changed
                                            if profile_changed && params.lod.lod_config.profile != crate::lod::LODProfile::Custom {
                                                params.lod.lod_config.apply_profile(params.lod.lod_config.profile);
                                                actions.changed = true;
                                            }
                                            actions.changed |= profile_changed;

                                            ui.separator();

                                            // Strategy Selection
                                            ui.label("Strategy:");
                                            actions.changed |= egui::ComboBox::from_id_salt("lod_strategy")
                                                .selected_text(match params.lod.lod_config.strategy {
                                                    crate::lod::LODStrategy::Distance => "Distance-based",
                                                    crate::lod::LODStrategy::Motion => "Motion-based",
                                                    crate::lod::LODStrategy::Performance => "Performance-based",
                                                    crate::lod::LODStrategy::Hybrid => "Hybrid (All)",
                                                })
                                                .show_ui(ui, |ui| {
                                                    let mut changed_local = false;
                                                    changed_local |= ui.selectable_value(&mut params.lod.lod_config.strategy, crate::lod::LODStrategy::Distance, "Distance-based")
                                                        .on_hover_text("Reduce quality based on distance from camera")
                                                        .changed();
                                                    changed_local |= ui.selectable_value(&mut params.lod.lod_config.strategy, crate::lod::LODStrategy::Motion, "Motion-based")
                                                        .on_hover_text("Reduce quality during camera movement")
                                                        .changed();
                                                    changed_local |= ui.selectable_value(&mut params.lod.lod_config.strategy, crate::lod::LODStrategy::Performance, "Performance-based")
                                                        .on_hover_text("Adjust quality to maintain target FPS")
                                                        .changed();
                                                    changed_local |= ui.selectable_value(&mut params.lod.lod_config.strategy, crate::lod::LODStrategy::Hybrid, "Hybrid (All)")
                                                        .on_hover_text("Intelligently combine all strategies")
                                                        .changed();
                                                    changed_local
                                                })
                                                .inner.unwrap_or(false);

                                            // Target FPS
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.lod.lod_config.target_fps, 30.0..=120.0)
                                                .text("Target FPS"))
                                                .on_hover_text("Target framerate for performance-based LOD")
                                                .changed();

                                            // Debug Visualization
                                            actions.changed |= ui.checkbox(&mut params.lod.lod_config.debug_visualization, "Debug Visualization")
                                                .on_hover_text("Show current LOD level and performance metrics")
                                                .changed();

                                            ui.separator();

                                            // Distance-based Controls
                                            if params.lod.lod_config.strategy == crate::lod::LODStrategy::Distance ||
                                               params.lod.lod_config.strategy == crate::lod::LODStrategy::Hybrid {
                                                ui.collapsing("Distance Zones", |ui| {
                                                    ui.label("Define distance thresholds for quality levels:")
                                                        .on_hover_text("Closer = higher quality, farther = lower quality");

                                                    ui.add_space(4.0);

                                                    // Ultra zone (< zone 0)
                                                    ui.horizontal(|ui| {
                                                        let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                                                        ui.painter().rect_filled(rect, 2.0, egui::Color32::from_rgb(0, 255, 0));
                                                        actions.changed |= ui.add(egui::Slider::new(&mut params.lod.lod_config.distance_zones[0], 1.0..=50.0)
                                                            .text("Near -> Mid"))
                                                            .on_hover_text("Distance where quality drops from Ultra to High")
                                                            .changed();
                                                    });

                                                    // High zone (zone 0 to zone 1)
                                                    ui.horizontal(|ui| {
                                                        let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                                                        ui.painter().rect_filled(rect, 2.0, egui::Color32::from_rgb(100, 255, 100));
                                                        actions.changed |= ui.add(egui::Slider::new(&mut params.lod.lod_config.distance_zones[1], 10.0..=100.0)
                                                            .text("Mid -> Far"))
                                                            .on_hover_text("Distance where quality drops from High to Medium")
                                                            .changed();
                                                    });

                                                    // Medium zone (zone 1 to zone 2)
                                                    ui.horizontal(|ui| {
                                                        let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                                                        ui.painter().rect_filled(rect, 2.0, egui::Color32::from_rgb(255, 200, 0));
                                                        actions.changed |= ui.add(egui::Slider::new(&mut params.lod.lod_config.distance_zones[2], 25.0..=150.0)
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
                                                    if params.lod.lod_config.distance_zones[0] > params.lod.lod_config.distance_zones[1] {
                                                        params.lod.lod_config.distance_zones[0] = params.lod.lod_config.distance_zones[1];
                                                        actions.changed = true;
                                                    }
                                                    if params.lod.lod_config.distance_zones[1] > params.lod.lod_config.distance_zones[2] {
                                                        params.lod.lod_config.distance_zones[1] = params.lod.lod_config.distance_zones[2];
                                                        actions.changed = true;
                                                    }
                                                });
                                            }

                                            // Motion-based Controls
                                            if params.lod.lod_config.strategy == crate::lod::LODStrategy::Motion ||
                                               params.lod.lod_config.strategy == crate::lod::LODStrategy::Hybrid {
                                                ui.collapsing("Motion Settings", |ui| {
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.lod.lod_config.motion_sensitivity, 0.0..=5.0)
                                                        .text("Motion Sensitivity"))
                                                        .on_hover_text("Higher = more sensitive to camera movement (1.0 = normal)")
                                                        .changed();

                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.lod.lod_config.motion_threshold, 0.01..=1.0)
                                                        .text("Motion Threshold"))
                                                        .on_hover_text("Minimum velocity to trigger quality reduction")
                                                        .changed();

                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.lod.lod_config.restore_delay, 0.1..=2.0)
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
                                                        params.lod.lod_config.quality_presets[0] = crate::lod::QualityLevel::ultra();
                                                        actions.changed = true;
                                                    }
                                                    if ui.button("High")
                                                        .on_hover_text("High quality preset")
                                                        .clicked() {
                                                        params.lod.lod_config.quality_presets[1] = crate::lod::QualityLevel::high();
                                                        actions.changed = true;
                                                    }
                                                    if ui.button("Medium")
                                                        .on_hover_text("Medium quality preset")
                                                        .clicked() {
                                                        params.lod.lod_config.quality_presets[2] = crate::lod::QualityLevel::medium();
                                                        actions.changed = true;
                                                    }
                                                    if ui.button("Low")
                                                        .on_hover_text("Low quality preset")
                                                        .clicked() {
                                                        params.lod.lod_config.quality_presets[3] = crate::lod::QualityLevel::low();
                                                        actions.changed = true;
                                                    }
                                                });

                                                ui.horizontal(|ui| {
                                                    if ui.button("Reset All to Defaults")
                                                        .on_hover_text("Reset all quality presets to default values")
                                                        .clicked() {
                                                        params.lod.lod_config.quality_presets = [
                                                            crate::lod::QualityLevel::ultra(),
                                                            crate::lod::QualityLevel::high(),
                                                            crate::lod::QualityLevel::medium(),
                                                            crate::lod::QualityLevel::low(),
                                                        ];
                                                        actions.changed = true;
                                                    }
                                                });

                                                ui.separator();

                                                // Custom quality editor
                                                ui.label("Edit Quality Levels:")
                                                    .on_hover_text("Fine-tune each quality preset");

                                                for (i, preset) in params.lod.lod_config.quality_presets.iter_mut().enumerate() {
                                                    let level_name = match i {
                                                        0 => "Ultra",
                                                        1 => "High",
                                                        2 => "Medium",
                                                        3 => "Low",
                                                        _ => "Unknown",
                                                    };

                                                    ui.collapsing(level_name, |ui| {
                                                        actions.changed |= ui.add(egui::Slider::new(&mut preset.max_steps, 50..=500)
                                                            .text("Max Steps"))
                                                            .on_hover_text("Ray marching iterations")
                                                            .changed();

                                                        actions.changed |= ui.add(egui::Slider::new(&mut preset.min_distance, 0.0001..=0.01)
                                                            .text("Min Distance")
                                                            .logarithmic(true))
                                                            .on_hover_text("Surface precision threshold")
                                                            .changed();

                                                        actions.changed |= ui.add(egui::Slider::new(&mut preset.shadow_samples, 4..=256)
                                                            .text("Shadow Samples"))
                                                            .on_hover_text("Shadow quality")
                                                            .changed();

                                                        actions.changed |= ui.add(egui::Slider::new(&mut preset.shadow_step_factor, 0.3..=1.0)
                                                            .text("Shadow Step Factor"))
                                                            .on_hover_text("Shadow ray step size (higher = faster, less precise)")
                                                            .changed();

                                                        actions.changed |= ui.add(egui::Slider::new(&mut preset.ao_step_size, 0.05..=0.5)
                                                            .text("AO Step Size"))
                                                            .on_hover_text("Ambient occlusion step size")
                                                            .changed();

                                                        actions.changed |= ui.add(egui::Slider::new(&mut preset.dof_samples, 1..=16)
                                                            .text("DOF Samples"))
                                                            .on_hover_text("Depth of field sample count")
                                                            .changed();

                                                        // ARC-007: render_scale is a no-op today — the renderer
                                                        // always renders at native resolution. The slider is hidden
                                                        // (not deleted) so existing settings keep parsing; the field
                                                        // is wired up under ENH-003 (dynamic render scale).
                                                        let _ = preset.render_scale;
                                                    });
                                                }
                                            });

                                            // Advanced Settings
                                            ui.collapsing("Advanced", |ui| {
                                                actions.changed |= ui.checkbox(&mut params.lod.lod_config.smooth_transitions, "Smooth Transitions")
                                                    .on_hover_text("Interpolate between quality levels for seamless changes")
                                                    .changed();

                                                if params.lod.lod_config.smooth_transitions {
                                                    actions.changed |= ui.add(egui::Slider::new(&mut params.lod.lod_config.transition_duration, 0.0..=1.0)
                                                        .text("Transition Duration"))
                                                        .on_hover_text("Time to transition between quality levels (seconds)")
                                                        .changed();
                                                }

                                                actions.changed |= ui.checkbox(&mut params.lod.lod_config.aggressive_mode, "Aggressive Mode")
                                                    .on_hover_text("More aggressive quality reduction for better performance")
                                                    .changed();

                                                ui.label("Minimum Quality Level:");
                                                actions.changed |= egui::ComboBox::from_id_salt("min_quality_level")
                                                    .selected_text(match params.lod.lod_config.min_quality_level {
                                                        0 => "Ultra",
                                                        1 => "High",
                                                        2 => "Medium",
                                                        3 => "Low",
                                                        _ => "Unknown",
                                                    })
                                                    .show_ui(ui, |ui| {
                                                        let mut changed_local = false;
                                                        changed_local |= ui.selectable_value(&mut params.lod.lod_config.min_quality_level, 0, "Ultra")
                                                            .on_hover_text("Never reduce quality below Ultra")
                                                            .changed();
                                                        changed_local |= ui.selectable_value(&mut params.lod.lod_config.min_quality_level, 1, "High")
                                                            .on_hover_text("Never reduce quality below High")
                                                            .changed();
                                                        changed_local |= ui.selectable_value(&mut params.lod.lod_config.min_quality_level, 2, "Medium")
                                                            .on_hover_text("Never reduce quality below Medium")
                                                            .changed();
                                                        changed_local |= ui.selectable_value(&mut params.lod.lod_config.min_quality_level, 3, "Low")
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
                                                let level_name = match params.lod.lod_state.current_level {
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
                                                    ui.label(format!("Current FPS: {:.1}", params.lod.lod_state.current_fps));
                                                });

                                                // Motion Status
                                                ui.horizontal(|ui| {
                                                    ui.label("Motion:");
                                                    if params.lod.lod_state.is_moving {
                                                        ui.colored_label(egui::Color32::YELLOW, "Moving");
                                                    } else {
                                                        ui.colored_label(egui::Color32::GREEN, "Stationary");
                                                    }
                                                });

                                                // Transition Progress
                                                if params.lod.lod_state.transition_progress < 1.0 {
                                                    ui.horizontal(|ui| {
                                                        ui.label("Transitioning:");
                                                        ui.add(egui::ProgressBar::new(params.lod.lod_state.transition_progress)
                                                            .show_percentage());
                                                    });
                                                }

                                                // Active Quality Parameters (showing what's currently being used)
                                                ui.collapsing("Active Parameters", |ui| {
                                                    let quality = &params.lod.lod_state.active_quality;
                                                    ui.label(format!("Max Steps: {}", quality.max_steps));
                                                    ui.label(format!("Min Distance: {:.6}", quality.min_distance));
                                                    ui.label(format!("Shadow Samples: {}", quality.shadow_samples));
                                                    ui.label(format!("Shadow Step: {:.2}", quality.shadow_step_factor));
                                                    ui.label(format!("AO Step: {:.2}", quality.ao_step_size));
                                                    ui.label(format!("DOF Samples: {}", quality.dof_samples));
                                                    ui.label(format!("Render Scale: {:.2} (not yet applied — see ENH-003)", quality.render_scale));
                                                });
                                            });
                                        }
                                    });
        self.ui_state.lod_open = response.openness > 0.0;
    }
}
