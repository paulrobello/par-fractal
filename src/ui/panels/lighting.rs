//! "Lighting", "Effects", and "Floor" panels — visual look (3D mode only).

use super::super::{UI, UiActions};
use crate::fractal::FractalParams;

impl UI {
    /// Render the "Floor" collapsing header.
    pub fn render_floor_panel(
        &mut self,
        ui: &mut egui::Ui,
        params: &mut FractalParams,
        actions: &mut UiActions,
    ) {
        let response = egui::CollapsingHeader::new("Floor")
                                    .default_open(self.ui_state.floor_open)
                                    .show(ui, |ui| {
                                        actions.changed |= ui.checkbox(&mut params.settings.show_floor, "Show Floor")
                                            .on_hover_text("Display checkered floor plane [G]")
                                            .changed();

                                        if params.settings.show_floor {
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.floor_height, -10.0..=10.0)
                                                .text("Floor Height"))
                                                .on_hover_text("Vertical position of the floor plane")
                                                .changed();

                                            ui.separator();
                                            ui.label("Floor Colors:")
                                                .on_hover_text("Checkerboard pattern colors");

                                            let mut color1 = [params.settings.floor_color1.x, params.settings.floor_color1.y, params.settings.floor_color1.z];
                                            if ui.color_edit_button_rgb(&mut color1)
                                                .on_hover_text("First checkerboard color")
                                                .changed() {
                                                params.settings.floor_color1 = glam::Vec3::from_array(color1);
                                                actions.changed = true;
                                            }

                                            let mut color2 = [params.settings.floor_color2.x, params.settings.floor_color2.y, params.settings.floor_color2.z];
                                            if ui.color_edit_button_rgb(&mut color2)
                                                .on_hover_text("Second checkerboard color")
                                                .changed() {
                                                params.settings.floor_color2 = glam::Vec3::from_array(color2);
                                                actions.changed = true;
                                            }

                                            ui.separator();
                                            actions.changed |= ui.checkbox(&mut params.settings.floor_reflections, "Floor Reflections")
                                                .on_hover_text("Enable screen-space reflections on the floor - reflects the fractal onto the floor surface with Fresnel effect")
                                                .changed();

                                            if params.settings.floor_reflections {
                                                actions.changed |= ui.add(egui::Slider::new(&mut params.settings.floor_reflection_strength, 0.0..=1.0)
                                                    .text("Reflection Strength"))
                                                    .on_hover_text("Adjust reflection intensity: 0.0 = no reflections, 0.5 = moderate, 1.0 = maximum reflections")
                                                    .changed();
                                            }
                                        }
                                    });
        self.ui_state.floor_open = response.openness > 0.0;
    }
}

impl UI {
    /// Render the "Effects" collapsing header.
    pub fn render_effects_panel(
        &mut self,
        ui: &mut egui::Ui,
        params: &mut FractalParams,
        actions: &mut UiActions,
    ) {
        let response = egui::CollapsingHeader::new("Effects")
                                    .default_open(self.ui_state.effects_open)
                                    .show(ui, |ui| {
                                        actions.changed |= ui.checkbox(&mut params.settings.depth_of_field, "Depth of Field")
                                            .on_hover_text("Blur based on distance from focus [T]")
                                            .changed();

                                        if params.settings.depth_of_field {
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.dof_focal_length, 1.0..=20.0)
                                                .text("Focal Length"))
                                                .on_hover_text("Distance to the focus plane - objects at this distance are sharp")
                                                .changed();
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.dof_aperture, 0.01..=1.0)
                                                .text("Aperture"))
                                                .on_hover_text("Aperture size - larger = more blur, smaller = sharper")
                                                .changed();
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.dof_samples, 1..=16)
                                                .text("Samples (quality vs speed)"))
                                                .on_hover_text("Number of samples per pixel - higher = smoother but slower")
                                                .changed();
                                        }

                                        ui.separator();
                                        actions.changed |= ui.checkbox(&mut params.settings.fog_enabled, "Fog")
                                            .on_hover_text("Add atmospheric fog effect")
                                            .changed();

                                        if params.settings.fog_enabled {
                                            actions.changed |= egui::ComboBox::from_id_salt("fog_mode")
                                                .selected_text(match params.settings.fog_mode {
                                                    crate::fractal::FogMode::Linear => "Linear",
                                                    crate::fractal::FogMode::Exponential => "Exponential",
                                                    crate::fractal::FogMode::Quadratic => "Quadratic",
                                                })
                                                .show_ui(ui, |ui| {
                                                    let mut changed_local = false;
                                                    changed_local |= ui.selectable_value(&mut params.settings.fog_mode, crate::fractal::FogMode::Linear, "Linear")
                                                        .on_hover_text("Linear fog falloff")
                                                        .changed();
                                                    changed_local |= ui.selectable_value(&mut params.settings.fog_mode, crate::fractal::FogMode::Exponential, "Exponential")
                                                        .on_hover_text("Exponential fog falloff - more realistic")
                                                        .changed();
                                                    changed_local |= ui.selectable_value(&mut params.settings.fog_mode, crate::fractal::FogMode::Quadratic, "Quadratic")
                                                        .on_hover_text("Quadratic fog falloff - dense atmosphere")
                                                        .changed();
                                                    changed_local
                                                })
                                                .inner.unwrap_or(false);

                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.fog_density, 0.0..=0.2)
                                                .text("Fog Density"))
                                                .on_hover_text("How thick the fog is - higher = denser")
                                                .changed();

                                            ui.label("Fog Color:")
                                                .on_hover_text("Color of the fog");
                                            let mut fog_color = [params.settings.fog_color.x, params.settings.fog_color.y, params.settings.fog_color.z];
                                            if ui.color_edit_button_rgb(&mut fog_color)
                                                .on_hover_text("Click to change fog color")
                                                .changed() {
                                                params.settings.fog_color = glam::Vec3::from_array(fog_color);
                                                actions.changed = true;
                                            }
                                        }

                                        // Post-Processing Section
                                        ui.separator();
                                        ui.label("Post-Processing:")
                                            .on_hover_text("Visual effects applied after rendering");

                                        // Color Grading
                                        ui.label("Color Grading:")
                                            .on_hover_text("Adjust the overall look and color of the image");
                                        actions.changed |= ui.add(egui::Slider::new(&mut params.settings.brightness, 0.0..=2.0)
                                            .text("Brightness"))
                                            .on_hover_text("Overall image brightness (1.0 = normal)")
                                            .changed();
                                        actions.changed |= ui.add(egui::Slider::new(&mut params.settings.contrast, 0.0..=2.0)
                                            .text("Contrast"))
                                            .on_hover_text("Contrast between light and dark areas (1.0 = normal)")
                                            .changed();
                                        actions.changed |= ui.add(egui::Slider::new(&mut params.settings.saturation, 0.0..=2.0)
                                            .text("Saturation"))
                                            .on_hover_text("Color intensity (0.0 = grayscale, 1.0 = normal, 2.0 = vivid)")
                                            .changed();
                                        actions.changed |= ui.add(egui::Slider::new(&mut params.settings.hue_shift, -1.0..=1.0)
                                            .text("Hue Shift"))
                                            .on_hover_text("Shift colors around the color wheel (-1.0 to 1.0)")
                                            .changed();

                                        ui.separator();

                                        // Vignette
                                        actions.changed |= ui.checkbox(&mut params.settings.vignette_enabled, "Vignette")
                                            .on_hover_text("Darken the edges of the image")
                                            .changed();
                                        if params.settings.vignette_enabled {
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.vignette_intensity, 0.0..=1.0)
                                                .text("Vignette Intensity"))
                                                .on_hover_text("How dark the edges become (0.0 = no effect, 1.0 = very dark)")
                                                .changed();
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.vignette_radius, 0.1..=2.0)
                                                .text("Vignette Radius"))
                                                .on_hover_text("Size of the vignette effect (smaller = larger dark area)")
                                                .changed();
                                        }

                                        ui.separator();

                                        // Bloom
                                        actions.changed |= ui.checkbox(&mut params.settings.bloom_enabled, "Bloom")
                                            .on_hover_text("Glow effect around bright areas - extracts and blurs bright pixels using multi-pass rendering")
                                            .changed();
                                        if params.settings.bloom_enabled {
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.bloom_threshold, 0.0..=1.0)
                                                .text("Threshold"))
                                                .on_hover_text("Minimum brightness for bloom (0.0 = all pixels, 1.0 = only brightest)")
                                                .changed();
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.bloom_intensity, 0.0..=2.0)
                                                .text("Intensity"))
                                                .on_hover_text("Strength of the bloom glow (0.3-0.8 recommended)")
                                                .changed();
                                        }

                                        ui.separator();

                                        // FXAA Anti-aliasing
                                        actions.changed |= ui.checkbox(&mut params.settings.fxaa_enabled, "FXAA Anti-aliasing")
                                            .on_hover_text("Fast approximate anti-aliasing to smooth jagged edges")
                                            .changed();
                                    });
        self.ui_state.effects_open = response.openness > 0.0;
    }
}

impl UI {
    /// Render the "Lighting" collapsing header.
    pub fn render_lighting_panel(
        &mut self,
        ui: &mut egui::Ui,
        params: &mut FractalParams,
        actions: &mut UiActions,
    ) {
        let response = egui::CollapsingHeader::new("Lighting")
                                    .default_open(self.ui_state.lighting_open)
                                    .show(ui, |ui| {
                                        ui.label("Light Settings:")
                                            .on_hover_text("Configure lighting intensity and ambience");
                                        actions.changed |= ui.add(egui::Slider::new(&mut params.settings.light_intensity, 0.5..=10.0)
                                            .text("Light Intensity"))
                                            .on_hover_text("Brightness of the main directional light")
                                            .changed();
                                        actions.changed |= ui.add(egui::Slider::new(&mut params.settings.ambient_light, 0.0..=1.0)
                                            .text("Ambient Light"))
                                            .on_hover_text("Base illumination level - prevents pure black shadows")
                                            .changed();

                                        ui.separator();
                                        ui.label("Light Direction:")
                                            .on_hover_text("Control the direction of the main light");
                                        actions.changed |= ui.add(egui::Slider::new(&mut params.settings.light_azimuth, 0.0..=360.0)
                                            .text("Azimuth"))
                                            .on_hover_text("Horizontal angle of the light (0-360°)")
                                            .changed();
                                        actions.changed |= ui.add(egui::Slider::new(&mut params.settings.light_elevation, 5.0..=90.0)
                                            .text("Elevation"))
                                            .on_hover_text("Vertical angle of the light (5-90°, where 90° is directly above)")
                                            .changed();

                                        ui.separator();
                                        ui.label("Shadows & AO:")
                                            .on_hover_text("Shadow and occlusion effects [B to cycle shadows]");
                                        ui.horizontal(|ui| {
                                            ui.label("Shadows [B]:");
                                            let shadow_names = ["Off", "Hard", "Soft"];
                                            egui::ComboBox::from_id_salt("shadow_mode")
                                                .selected_text(shadow_names[params.settings.shadow_mode as usize])
                                                .show_ui(ui, |ui| {
                                                    for (i, name) in shadow_names.iter().enumerate() {
                                                        if ui.selectable_value(&mut params.settings.shadow_mode, i as u32, *name).changed() {
                                                            actions.changed = true;
                                                        }
                                                    }
                                                });
                                        });
                                        if params.settings.shadow_mode > 0 {
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.shadow_max_distance, 1.0..=20.0)
                                                .text("Shadow Distance"))
                                                .on_hover_text("Maximum distance for shadow ray marching")
                                                .changed();
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.shadow_samples, 32..=256)
                                                .text("Shadow Samples"))
                                                .on_hover_text("Number of ray marching steps for shadows - higher = more accurate but slower")
                                                .changed();
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.shadow_step_factor, 0.3..=1.0)
                                                .text("Shadow Accuracy"))
                                                .on_hover_text("Step size factor for shadow rays - lower = more accurate but slower (0.6 is good default)")
                                                .changed();
                                        }
                                        if params.settings.shadow_mode == 2 {
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.shadow_softness, 1.0..=32.0)
                                                .text("Shadow Softness"))
                                                .on_hover_text("Shadow penumbra softness - higher = softer edges")
                                                .changed();
                                        }

                                        actions.changed |= ui.checkbox(&mut params.settings.ambient_occlusion, "Ambient Occlusion")
                                            .on_hover_text("Enable ambient occlusion for contact shadows [L]")
                                            .changed();
                                        if params.settings.ambient_occlusion {
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.ao_intensity, 0.0..=10.0)
                                                .text("AO Intensity"))
                                                .on_hover_text("Strength of ambient occlusion darkening")
                                                .changed();
                                            actions.changed |= ui.add(egui::Slider::new(&mut params.settings.ao_step_size, 0.01..=0.5)
                                                .text("AO Step Size"))
                                                .on_hover_text("Step size for AO sampling - smaller = finer detail")
                                                .changed();
                                        }
                                    });
        self.ui_state.lighting_open = response.openness > 0.0;
    }
}
