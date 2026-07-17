//! "Color & Visualization" panel — palette selection, color modes, channel mixing,
//! procedural palettes, custom palette editor, palette animation.

use super::super::get_procedural_preview_color;
use super::super::{UI, UiActions};
use crate::fractal::{CustomPalette, CustomPaletteGallery, FractalParams};
use glam::Vec3;

impl UI {
    /// Render the "Color & Visualization" collapsing header.
    pub fn render_color_viz_panel(
        &mut self,
        ui: &mut egui::Ui,
        params: &mut FractalParams,
        actions: &mut UiActions,
    ) {
        let response = egui::CollapsingHeader::new("Color & Visualization")
                            .default_open(self.ui_state.color_viz_open)
                            .show(ui, |ui| {
                                ui.label("Color Mode:")
                                    .on_hover_text("Choose how colors are applied to the fractal");
                                actions.changed |= egui::ComboBox::from_id_salt("color_mode")
                                    .selected_text(match params.settings.color_mode {
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
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::Palette, "Palette").changed();
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::RaySteps, "Ray Steps / Iterations").changed();
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::Normals, "Normals (3D)").changed();
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::OrbitTrapXYZ, "Orbit Trap XYZ").changed();
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::OrbitTrapRadial, "Orbit Trap Radial").changed();
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::WorldPosition, "World Position").changed();
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::LocalPosition, "Local Position").changed();
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::AmbientOcclusion, "Ambient Occlusion (3D)").changed();
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::PerChannel, "Per-Channel (Custom RGB)")
                                            .on_hover_text("Map different data sources to R, G, and B channels independently")
                                            .changed();
                                        ui.separator();
                                        ui.label("Debug Modes:");
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::DistanceField, "🔍 Distance Field")
                                            .on_hover_text("Visualize distance field complexity - shows ray marching step density")
                                            .changed();
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::Depth, "🔍 Depth")
                                            .on_hover_text("Visualize distance from camera")
                                            .changed();
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::Convergence, "🔍 Convergence")
                                            .on_hover_text("Visualize escape time / convergence speed")
                                            .changed();
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::LightingOnly, "🔍 Lighting Only")
                                            .on_hover_text("Show only lighting without fractal coloring")
                                            .changed();
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::ShadowMap, "🔍 Shadow Map")
                                            .on_hover_text("Visualize shadow values (3D)")
                                            .changed();
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::CameraDistanceLOD, "🔍 Camera Distance LOD")
                                            .on_hover_text("Visualize distance from camera using LOD zone colors (3D)")
                                            .changed();
                                        changed_local |= ui.selectable_value(&mut params.settings.color_mode, crate::fractal::ColorMode::DistanceGrayscale, "🔍 Distance Grayscale")
                                            .on_hover_text("Visualize raw distance from camera as brightness (3D)")
                                            .changed();
                                        changed_local
                                    })
                                    .inner.unwrap_or(false);

                                // Show color key for debug visualization modes
                                match params.settings.color_mode {
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
                                if params.settings.color_mode == crate::fractal::ColorMode::Palette ||
                                   params.settings.color_mode == crate::fractal::ColorMode::OrbitTrapXYZ ||
                                   params.settings.color_mode == crate::fractal::ColorMode::OrbitTrapRadial {
                                    ui.separator();
                                    // Procedural Palette Selection
                                    ui.label("Procedural Palette:")
                                        .on_hover_text("Choose a mathematically-generated palette for smooth gradients");
                                    actions.changed |= egui::ComboBox::from_id_salt("procedural_palette")
                                        .selected_text(params.settings.procedural_palette.name())
                                        .show_ui(ui, |ui| {
                                            let mut ch = false;
                                            ch |= ui.selectable_value(
                                                &mut params.settings.procedural_palette,
                                                crate::fractal::ProceduralPalette::None,
                                                "None (Static)"
                                            ).on_hover_text("Use the static color palette below").changed();
                                            for palette in crate::fractal::ProceduralPalette::ALL {
                                                ch |= ui.selectable_value(
                                                    &mut params.settings.procedural_palette,
                                                    *palette,
                                                    palette.name()
                                                ).changed();
                                            }
                                            ch
                                        })
                                        .inner.unwrap_or(false);

                                    // Show procedural palette preview (generate 8 sample colors)
                                    if params.settings.procedural_palette != crate::fractal::ProceduralPalette::None {
                                        ui.horizontal(|ui| {
                                            for i in 0..8 {
                                                let t = i as f32 / 7.0;
                                                let color = get_procedural_preview_color(params.settings.procedural_palette, t, &params.settings.procedural_brightness, &params.settings.procedural_contrast, &params.settings.procedural_frequency, &params.settings.procedural_phase);
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

                                        // Custom palette parameters when Custom is selected
                                        if params.settings.procedural_palette == crate::fractal::ProceduralPalette::Custom {
                                            ui.separator();
                                            ui.label("Custom Palette Parameters:")
                                                .on_hover_text("Adjust cosine palette formula: color = a + b * cos(2π * (c * t + d))");

                                            ui.horizontal(|ui| {
                                                ui.label("Brightness:");
                                                actions.changed |= ui.add(egui::DragValue::new(&mut params.settings.procedural_brightness[0]).speed(0.01).range(0.0..=1.0).prefix("R: ")).changed();
                                                actions.changed |= ui.add(egui::DragValue::new(&mut params.settings.procedural_brightness[1]).speed(0.01).range(0.0..=1.0).prefix("G: ")).changed();
                                                actions.changed |= ui.add(egui::DragValue::new(&mut params.settings.procedural_brightness[2]).speed(0.01).range(0.0..=1.0).prefix("B: ")).changed();
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Contrast:");
                                                actions.changed |= ui.add(egui::DragValue::new(&mut params.settings.procedural_contrast[0]).speed(0.01).range(0.0..=1.0).prefix("R: ")).changed();
                                                actions.changed |= ui.add(egui::DragValue::new(&mut params.settings.procedural_contrast[1]).speed(0.01).range(0.0..=1.0).prefix("G: ")).changed();
                                                actions.changed |= ui.add(egui::DragValue::new(&mut params.settings.procedural_contrast[2]).speed(0.01).range(0.0..=1.0).prefix("B: ")).changed();
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Frequency:");
                                                actions.changed |= ui.add(egui::DragValue::new(&mut params.settings.procedural_frequency[0]).speed(0.01).range(0.0..=5.0).prefix("R: ")).changed();
                                                actions.changed |= ui.add(egui::DragValue::new(&mut params.settings.procedural_frequency[1]).speed(0.01).range(0.0..=5.0).prefix("G: ")).changed();
                                                actions.changed |= ui.add(egui::DragValue::new(&mut params.settings.procedural_frequency[2]).speed(0.01).range(0.0..=5.0).prefix("B: ")).changed();
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Phase:");
                                                actions.changed |= ui.add(egui::DragValue::new(&mut params.settings.procedural_phase[0]).speed(0.01).range(0.0..=1.0).prefix("R: ")).changed();
                                                actions.changed |= ui.add(egui::DragValue::new(&mut params.settings.procedural_phase[1]).speed(0.01).range(0.0..=1.0).prefix("G: ")).changed();
                                                actions.changed |= ui.add(egui::DragValue::new(&mut params.settings.procedural_phase[2]).speed(0.01).range(0.0..=1.0).prefix("B: ")).changed();
                                            });
                                        }
                                    }

                                    // Static Palette Selection (only shown when not using procedural)
                                    if params.settings.procedural_palette == crate::fractal::ProceduralPalette::None {
                                        ui.separator();
                                        ui.label("Static Palette:")
                                            .on_hover_text("Choose from built-in color palettes [P to cycle]");
                                        ui.horizontal(|ui| {
                                            if ui.button("◀ Previous").on_hover_text("Switch to previous palette").clicked() {
                                                params.prev_palette();
                                                self.show_toast(format!("Palette: {}", params.settings.palette.name));
                                                actions.changed = true;
                                            }
                                            ui.label(params.settings.palette.name);
                                            if ui.button("Next ▶").on_hover_text("Switch to next palette [P]").clicked() {
                                                params.next_palette();
                                                self.show_toast(format!("Palette: {}", params.settings.palette.name));
                                                actions.changed = true;
                                            }
                                        });

                                        // Show palette colors
                                        ui.horizontal(|ui| {
                                            for color in &params.settings.palette.colors {
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
                                    }

                                    // Palette Animation Controls
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        actions.changed |= ui.checkbox(&mut self.palette_animation_enabled, "Animate Palette")
                                            .on_hover_text("Slowly rotate through palette colors for mesmerizing effects")
                                            .changed();
                                    });

                                    if self.palette_animation_enabled {
                                        ui.horizontal(|ui| {
                                            ui.label("Speed:");
                                            actions.changed |= ui.add(egui::Slider::new(&mut self.palette_animation_speed, 0.01..=1.0)
                                                .text(""))
                                                .on_hover_text("Animation speed - higher values rotate faster")
                                                .changed();
                                        });

                                        ui.horizontal(|ui| {
                                            actions.changed |= ui.checkbox(&mut self.palette_animation_reverse, "Reverse Direction")
                                                .on_hover_text("Reverse the animation direction")
                                                .changed();
                                        });
                                    }

                                    // Show orbit trap scale slider for orbit trap modes
                                    if params.settings.color_mode == crate::fractal::ColorMode::OrbitTrapXYZ ||
                                       params.settings.color_mode == crate::fractal::ColorMode::OrbitTrapRadial {
                                        ui.separator();
                                        actions.changed |= ui.add(egui::Slider::new(&mut params.settings.orbit_trap_scale, 0.1..=5.0)
                                            .text("Orbit Trap Scale"))
                                            .on_hover_text("Scale factor for orbit trap coloring - affects color variation")
                                            .changed();
                                    }

                                    // Per-Channel Controls
                                    if params.settings.color_mode == crate::fractal::ColorMode::PerChannel {
                                        ui.separator();
                                        ui.label("Channel Mapping:")
                                            .on_hover_text("Map different data sources to R, G, and B channels");

                                        // Red channel
                                        ui.horizontal(|ui| {
                                            ui.label("R:");
                                            actions.changed |= egui::ComboBox::from_id_salt("channel_r")
                                                .selected_text(format!("{:?}", params.settings.channel_r))
                                                .show_ui(ui, |ui| {
                                                    let mut ch = false;
                                                    ch |= ui.selectable_value(&mut params.settings.channel_r, crate::fractal::ChannelSource::Iterations, "Iterations").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_r, crate::fractal::ChannelSource::Distance, "Distance").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_r, crate::fractal::ChannelSource::PositionX, "Position X").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_r, crate::fractal::ChannelSource::PositionY, "Position Y").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_r, crate::fractal::ChannelSource::PositionZ, "Position Z").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_r, crate::fractal::ChannelSource::Normal, "Normal").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_r, crate::fractal::ChannelSource::AO, "AO").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_r, crate::fractal::ChannelSource::Constant, "Constant (0)").changed();
                                                    ch
                                                })
                                                .inner.unwrap_or(false);
                                        });

                                        // Green channel
                                        ui.horizontal(|ui| {
                                            ui.label("G:");
                                            actions.changed |= egui::ComboBox::from_id_salt("channel_g")
                                                .selected_text(format!("{:?}", params.settings.channel_g))
                                                .show_ui(ui, |ui| {
                                                    let mut ch = false;
                                                    ch |= ui.selectable_value(&mut params.settings.channel_g, crate::fractal::ChannelSource::Iterations, "Iterations").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_g, crate::fractal::ChannelSource::Distance, "Distance").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_g, crate::fractal::ChannelSource::PositionX, "Position X").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_g, crate::fractal::ChannelSource::PositionY, "Position Y").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_g, crate::fractal::ChannelSource::PositionZ, "Position Z").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_g, crate::fractal::ChannelSource::Normal, "Normal").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_g, crate::fractal::ChannelSource::AO, "AO").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_g, crate::fractal::ChannelSource::Constant, "Constant (0)").changed();
                                                    ch
                                                })
                                                .inner.unwrap_or(false);
                                        });

                                        // Blue channel
                                        ui.horizontal(|ui| {
                                            ui.label("B:");
                                            actions.changed |= egui::ComboBox::from_id_salt("channel_b")
                                                .selected_text(format!("{:?}", params.settings.channel_b))
                                                .show_ui(ui, |ui| {
                                                    let mut ch = false;
                                                    ch |= ui.selectable_value(&mut params.settings.channel_b, crate::fractal::ChannelSource::Iterations, "Iterations").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_b, crate::fractal::ChannelSource::Distance, "Distance").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_b, crate::fractal::ChannelSource::PositionX, "Position X").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_b, crate::fractal::ChannelSource::PositionY, "Position Y").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_b, crate::fractal::ChannelSource::PositionZ, "Position Z").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_b, crate::fractal::ChannelSource::Normal, "Normal").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_b, crate::fractal::ChannelSource::AO, "AO").changed();
                                                    ch |= ui.selectable_value(&mut params.settings.channel_b, crate::fractal::ChannelSource::Constant, "Constant (0)").changed();
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
                                            if ui.button("💾 Save Custom")
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
                                                    log::error!("Failed to save custom palette: {}", e);
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
                                                    self.custom_palette_colors[i] = params.settings.palette.colors[i].to_array();
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
                                                                .clicked()
                                                                && let Ok(custom_palette) = CustomPaletteGallery::load_palette(palette_name) {
                                                                    // Apply the custom palette to the current params
                                                                    let (_name, colors) = custom_palette.to_color_palette();
                                                                    params.settings.palette.colors = colors;
                                                                    actions.changed = true;
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
                                                log::error!("Failed to delete custom palette: {}", e);
                                            }
                                            self.custom_palettes = CustomPaletteGallery::list_palettes().unwrap_or_default();
                                            self.custom_palette_to_delete = None;
                                        }
                                    });
                                }
                            });
        self.ui_state.color_viz_open = response.openness > 0.0;
    }
}
