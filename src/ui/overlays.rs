use super::UI;
use crate::fractal::FractalParams;
use egui::Context;
use glam::Vec3;

/// Overlay rendering methods for UI
impl UI {
    pub fn render_fps(&self, ctx: &Context, fps: f32) {
        if !self.show_fps {
            return;
        }

        egui::Area::new(egui::Id::new("fps_counter"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 10.0))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::from_black_alpha(255)) // Increased from 180 (50% more opaque)
                    .inner_margin(8.0)
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.set_min_width(80.0); // Prevent text wrapping
                        ui.label(
                            egui::RichText::new(format!("FPS: {:.1}", fps))
                                .color(egui::Color32::from_rgb(0, 255, 0))
                                .size(16.0),
                        );
                    });
            });
    }

    pub fn render_camera_info(
        &self,
        ctx: &Context,
        camera_pos: Vec3,
        camera_target: Vec3,
        lod_zones: &[f32; 3],
    ) {
        if !self.show_camera_info {
            return;
        }

        let direction = (camera_target - camera_pos).normalize();

        egui::Area::new(egui::Id::new("camera_info"))
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(10.0, -10.0))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::from_black_alpha(220))
                    .inner_margin(8.0)
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Camera Info")
                                .color(egui::Color32::from_rgb(255, 255, 255))
                                .size(14.0)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Pos: ({:.2}, {:.2}, {:.2})",
                                camera_pos.x, camera_pos.y, camera_pos.z
                            ))
                            .color(egui::Color32::from_rgb(200, 200, 255))
                            .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Dir: ({:.2}, {:.2}, {:.2})",
                                direction.x, direction.y, direction.z
                            ))
                            .color(egui::Color32::from_rgb(200, 255, 200))
                            .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Dist to Origin: {:.2}",
                                camera_pos.length()
                            ))
                            .color(egui::Color32::from_rgb(255, 200, 200))
                            .size(12.0),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("LOD Zones:")
                                .color(egui::Color32::from_rgb(150, 150, 150))
                                .size(11.0),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "  Green < {:.1} < LtGrn < {:.1} < Orange < {:.1} < Red",
                                lod_zones[0], lod_zones[1], lod_zones[2]
                            ))
                            .color(egui::Color32::from_rgb(180, 180, 180))
                            .size(10.0),
                        );
                    });
            });
    }

    /// Get the current palette animation offset based on time
    pub fn get_palette_animation_offset(&self, elapsed_time: f32) -> f32 {
        if !self.palette_animation_enabled {
            return 0.0;
        }

        let direction = if self.palette_animation_reverse {
            -1.0
        } else {
            1.0
        };
        (elapsed_time * self.palette_animation_speed * direction) % 1.0
    }

    /// Update frame time history for performance overlay
    pub fn update_frame_time(&mut self, frame_time_ms: f32) {
        self.frame_times.push(frame_time_ms);
        if self.frame_times.len() > self.max_frame_history {
            self.frame_times.remove(0);
        }
    }

    /// Render performance overlay with FPS, frame time, and graph
    pub fn render_performance_overlay(&self, ctx: &Context, fps: f32) {
        if !self.show_performance_overlay {
            return;
        }

        egui::Area::new(egui::Id::new("performance_overlay"))
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(10.0, -10.0))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::from_black_alpha(245))
                    .inner_margin(10.0)
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        // Title
                        ui.label(
                            egui::RichText::new("Performance")
                                .color(egui::Color32::from_rgb(255, 255, 255))
                                .size(16.0)
                                .strong(),
                        );

                        ui.add_space(8.0);

                        // FPS display
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("FPS:")
                                    .color(egui::Color32::from_rgb(180, 180, 180))
                                    .size(14.0),
                            );
                            let fps_color = if fps >= 60.0 {
                                egui::Color32::from_rgb(0, 255, 0)
                            } else if fps >= 30.0 {
                                egui::Color32::from_rgb(255, 255, 0)
                            } else {
                                egui::Color32::from_rgb(255, 100, 100)
                            };
                            ui.label(
                                egui::RichText::new(format!("{:.1}", fps))
                                    .color(fps_color)
                                    .size(14.0)
                                    .strong(),
                            );
                        });

                        // Frame time display
                        if let Some(&last_frame_time) = self.frame_times.last() {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Frame Time:")
                                        .color(egui::Color32::from_rgb(180, 180, 180))
                                        .size(14.0),
                                );
                                let ft_color = if last_frame_time <= 16.67 {
                                    egui::Color32::from_rgb(0, 255, 0)
                                } else if last_frame_time <= 33.33 {
                                    egui::Color32::from_rgb(255, 255, 0)
                                } else {
                                    egui::Color32::from_rgb(255, 100, 100)
                                };
                                ui.label(
                                    egui::RichText::new(format!("{:.2} ms", last_frame_time))
                                        .color(ft_color)
                                        .size(14.0)
                                        .strong(),
                                );
                            });
                        }

                        ui.add_space(8.0);

                        // Frame time graph (similar to three.js stats)
                        if !self.frame_times.is_empty() {
                            let graph_height = 60.0;
                            let graph_width = 200.0;

                            let (rect, _response) = ui.allocate_exact_size(
                                egui::vec2(graph_width, graph_height),
                                egui::Sense::hover(),
                            );

                            // Background
                            ui.painter().rect_filled(
                                rect,
                                2.0,
                                egui::Color32::from_rgb(20, 20, 30),
                            );

                            // Calculate max frame time for scaling (cap at 50ms for better visualization)
                            let max_ft = self
                                .frame_times
                                .iter()
                                .cloned()
                                .fold(0.0f32, f32::max)
                                .min(50.0);
                            let max_display = max_ft.max(16.67); // At least show 60fps line

                            // Draw target frame time lines
                            // 60 FPS line (16.67ms)
                            let y_60fps = rect.max.y - (16.67 / max_display) * graph_height;
                            ui.painter().hline(
                                rect.min.x..=rect.max.x,
                                y_60fps,
                                (1.0, egui::Color32::from_rgb(0, 180, 0)),
                            );

                            // 30 FPS line (33.33ms)
                            if max_display >= 33.33 {
                                let y_30fps = rect.max.y - (33.33 / max_display) * graph_height;
                                ui.painter().hline(
                                    rect.min.x..=rect.max.x,
                                    y_30fps,
                                    (1.0, egui::Color32::from_rgb(180, 180, 0)),
                                );
                            }

                            // Draw frame time graph
                            let num_samples = self.frame_times.len();
                            let bar_width = graph_width / num_samples as f32;

                            for (i, &ft) in self.frame_times.iter().enumerate() {
                                let normalized_height = (ft / max_display).min(1.0) * graph_height;
                                let x = rect.min.x + i as f32 * bar_width;
                                let y = rect.max.y - normalized_height;

                                // Color based on performance
                                let bar_color = if ft <= 16.67 {
                                    egui::Color32::from_rgb(0, 255, 0)
                                } else if ft <= 33.33 {
                                    egui::Color32::from_rgb(255, 255, 0)
                                } else {
                                    egui::Color32::from_rgb(255, 80, 80)
                                };

                                ui.painter().rect_filled(
                                    egui::Rect::from_min_max(
                                        egui::pos2(x, y),
                                        egui::pos2(x + bar_width, rect.max.y),
                                    ),
                                    0.0,
                                    bar_color,
                                );
                            }

                            // Border
                            ui.painter().rect_stroke(
                                rect,
                                0.0,
                                egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
                                egui::epaint::StrokeKind::Middle,
                            );

                            // Labels
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("Max: {:.1} ms", max_ft))
                                        .color(egui::Color32::from_rgb(150, 150, 150))
                                        .size(11.0),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{} frames", num_samples))
                                        .color(egui::Color32::from_rgb(150, 150, 150))
                                        .size(11.0),
                                );
                            });
                        }
                    });
            });
    }

    pub fn render_recording_indicator(
        &self,
        ctx: &Context,
        is_recording: bool,
        frame_count: u32,
        filename: &str,
    ) {
        if !is_recording {
            return;
        }

        egui::Area::new(egui::Id::new("recording_indicator"))
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 10.0))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_unmultiplied(200, 0, 0, 200))
                    .corner_radius(egui::CornerRadius::same(5))
                    .inner_margin(egui::Margin::symmetric(16, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Blinking red dot
                            let blink = (ctx.input(|i| i.time) * 2.0) % 2.0 > 1.0;
                            if blink {
                                ui.colored_label(
                                    egui::Color32::WHITE,
                                    egui::RichText::new("●").size(20.0),
                                );
                            } else {
                                ui.add_space(12.0);
                            }

                            ui.colored_label(
                                egui::Color32::WHITE,
                                egui::RichText::new(format!("REC  {} frames", frame_count))
                                    .size(16.0)
                                    .strong(),
                            );
                        });
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 255, 200),
                            egui::RichText::new(filename).size(12.0),
                        );
                    });
            });
    }

    pub fn render_lod_debug_overlay(&self, ctx: &Context, params: &FractalParams) {
        // Only show if LOD is enabled and debug visualization is on
        if !params.lod_config.enabled || !params.lod_config.debug_visualization {
            return;
        }

        egui::Area::new(egui::Id::new("lod_debug_overlay"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 10.0))
            .show(ctx, |ui| {
                ui.set_max_width(220.0);
                egui::Frame::NONE
                    .fill(egui::Color32::from_black_alpha(245))
                    .inner_margin(10.0)
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.set_max_width(200.0);

                        // Title
                        ui.label(
                            egui::RichText::new("LOD System Debug")
                                .color(egui::Color32::from_rgb(255, 255, 255))
                                .size(16.0)
                                .strong(),
                        );

                        ui.add_space(8.0);

                        // Current LOD Level with large, color-coded display
                        let (level_name, level_color) = match params.lod_state.current_level {
                            0 => ("ULTRA", egui::Color32::from_rgb(0, 255, 0)),
                            1 => ("HIGH", egui::Color32::from_rgb(100, 255, 100)),
                            2 => ("MEDIUM", egui::Color32::from_rgb(255, 200, 0)),
                            3 => ("LOW", egui::Color32::from_rgb(255, 100, 0)),
                            _ => ("UNKNOWN", egui::Color32::GRAY),
                        };

                        ui.label(
                            egui::RichText::new(level_name)
                                .color(level_color)
                                .size(20.0)
                                .strong(),
                        );

                        ui.add_space(4.0);

                        // Show transition progress if transitioning
                        if params.lod_state.transition_progress < 1.0 {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Transitioning:")
                                        .color(egui::Color32::from_rgb(180, 180, 180))
                                        .size(12.0),
                                );
                                let target_name = match params.lod_state.target_level {
                                    0 => "Ultra",
                                    1 => "High",
                                    2 => "Medium",
                                    3 => "Low",
                                    _ => "?",
                                };
                                ui.label(
                                    egui::RichText::new(format!("→ {}", target_name))
                                        .color(egui::Color32::from_rgb(255, 200, 100))
                                        .size(12.0),
                                );
                            });

                            let (rect, _response) = ui
                                .allocate_exact_size(egui::vec2(180.0, 6.0), egui::Sense::hover());

                            // Progress bar background
                            ui.painter().rect_filled(
                                rect,
                                2.0,
                                egui::Color32::from_rgb(50, 50, 50),
                            );

                            // Progress bar fill
                            let fill_width = rect.width() * params.lod_state.transition_progress;
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    rect.min,
                                    egui::vec2(fill_width, rect.height()),
                                ),
                                2.0,
                                egui::Color32::from_rgb(100, 200, 255),
                            );

                            ui.add_space(4.0);
                        }

                        ui.separator();
                        ui.add_space(4.0);

                        // Strategy
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Strategy:")
                                    .color(egui::Color32::from_rgb(180, 180, 180))
                                    .size(13.0),
                            );
                            let strategy_name = match params.lod_config.strategy {
                                crate::lod::LODStrategy::Distance => "Distance",
                                crate::lod::LODStrategy::Motion => "Motion",
                                crate::lod::LODStrategy::Performance => "Performance",
                                crate::lod::LODStrategy::Hybrid => "Hybrid",
                            };
                            ui.label(
                                egui::RichText::new(strategy_name)
                                    .color(egui::Color32::from_rgb(150, 200, 255))
                                    .size(13.0)
                                    .strong(),
                            );
                        });

                        // Motion Status
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Motion:")
                                    .color(egui::Color32::from_rgb(180, 180, 180))
                                    .size(13.0),
                            );
                            if params.lod_state.is_moving {
                                ui.label(
                                    egui::RichText::new("MOVING")
                                        .color(egui::Color32::from_rgb(255, 200, 0))
                                        .size(13.0),
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new("STILL")
                                        .color(egui::Color32::from_rgb(100, 255, 100))
                                        .size(13.0),
                                );
                            }
                        });

                        // FPS
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("FPS:")
                                    .color(egui::Color32::from_rgb(180, 180, 180))
                                    .size(13.0),
                            );
                            let fps = params.lod_state.current_fps;
                            let target = params.lod_config.target_fps;
                            let fps_color = if fps >= target {
                                egui::Color32::from_rgb(0, 255, 0)
                            } else if fps >= target * 0.8 {
                                egui::Color32::from_rgb(255, 255, 0)
                            } else {
                                egui::Color32::from_rgb(255, 100, 100)
                            };
                            ui.label(
                                egui::RichText::new(format!("{:.1}/{:.0}", fps, target))
                                    .color(fps_color)
                                    .size(13.0)
                                    .strong(),
                            );
                        });

                        // Distance (for 3D fractals with Distance or Hybrid strategy)
                        if params.render_mode == crate::fractal::RenderMode::ThreeD
                            && (params.lod_config.strategy == crate::lod::LODStrategy::Distance
                                || params.lod_config.strategy == crate::lod::LODStrategy::Hybrid)
                        {
                            // Calculate distance to show (note: we don't have camera_pos here,
                            // so we'll show the distance zones instead)
                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            ui.label(
                                egui::RichText::new("Distance Zones:")
                                    .color(egui::Color32::from_rgb(200, 200, 200))
                                    .size(12.0)
                                    .strong(),
                            );

                            // Show distance zones with color coding
                            let zones = [
                                (
                                    format!("Ultra: < {:.1}", params.lod_config.distance_zones[0]),
                                    egui::Color32::from_rgb(0, 255, 0),
                                ),
                                (
                                    format!("High: < {:.1}", params.lod_config.distance_zones[1]),
                                    egui::Color32::from_rgb(100, 255, 100),
                                ),
                                (
                                    format!("Medium: < {:.1}", params.lod_config.distance_zones[2]),
                                    egui::Color32::from_rgb(255, 200, 0),
                                ),
                                ("Low: Far".to_string(), egui::Color32::from_rgb(255, 100, 0)),
                            ];

                            for (text, color) in &zones {
                                ui.label(egui::RichText::new(text).color(*color).size(11.0));
                            }
                        }

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Active Quality Parameters
                        ui.label(
                            egui::RichText::new("Active Quality:")
                                .color(egui::Color32::from_rgb(200, 200, 200))
                                .size(12.0)
                                .strong(),
                        );

                        let quality = &params.lod_state.active_quality;
                        let params_text = [
                            format!("Steps: {}", quality.max_steps),
                            format!("Min Dist: {:.6}", quality.min_distance),
                            format!("Shadows: {}", quality.shadow_samples),
                            format!("AO Step: {:.2}", quality.ao_step_size),
                            format!("DOF: {}", quality.dof_samples),
                        ];

                        for text in &params_text {
                            ui.label(
                                egui::RichText::new(text)
                                    .color(egui::Color32::from_rgb(180, 180, 180))
                                    .size(11.0),
                            );
                        }
                    });
            });
    }
}
