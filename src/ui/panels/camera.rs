//! "Camera" panel — view, orbit, speed, bookmarks (3D mode only).

use super::super::{UI, UiActions};
use crate::fractal::{BookmarkGallery, CameraBookmark, FractalParams};
use glam::Vec3;

impl UI {
    /// Render the "Camera" collapsing header.
    /// Needs `camera_pos` / `camera_target` because bookmark save captures them.
    pub fn render_camera_panel(
        &mut self,
        ui: &mut egui::Ui,
        params: &mut FractalParams,
        actions: &mut UiActions,
        camera_pos: Vec3,
        camera_target: Vec3,
    ) {
        let response = egui::CollapsingHeader::new("Camera")
            .default_open(self.ui_state.camera_open)
            .show(ui, |ui| {
                actions.changed |= ui
                    .add(
                        egui::Slider::new(&mut params.settings.camera_speed, 0.1..=10.0)
                            .text("Movement Speed"),
                    )
                    .on_hover_text("Camera movement speed for WASD controls")
                    .changed();

                // Camera speed presets
                ui.horizontal(|ui| {
                    ui.label("Speed presets:");
                    if ui
                        .small_button("Slow")
                        .on_hover_text("Set camera speed to 1.0")
                        .clicked()
                    {
                        params.settings.camera_speed = 1.0;
                        actions.changed = true;
                    }
                    if ui
                        .small_button("Normal")
                        .on_hover_text("Set camera speed to 3.0")
                        .clicked()
                    {
                        params.settings.camera_speed = 3.0;
                        actions.changed = true;
                    }
                    if ui
                        .small_button("Fast")
                        .on_hover_text("Set camera speed to 6.0")
                        .clicked()
                    {
                        params.settings.camera_speed = 6.0;
                        actions.changed = true;
                    }
                });

                ui.add_space(5.0);
                actions.changed |= ui
                    .add(
                        egui::Slider::new(&mut params.settings.camera_fov, 20.0..=120.0)
                            .text("Field of View (FOV)"),
                    )
                    .on_hover_text(
                        "Camera field of view in degrees\n45° = normal, higher = wide angle",
                    )
                    .changed();

                // FOV presets
                ui.horizontal(|ui| {
                    ui.label("FOV presets:");
                    if ui
                        .small_button("Wide")
                        .on_hover_text("Set FOV to 90° (wide angle)")
                        .clicked()
                    {
                        params.settings.camera_fov = 90.0;
                        actions.changed = true;
                    }
                    if ui
                        .small_button("Normal")
                        .on_hover_text("Set FOV to 45° (normal)")
                        .clicked()
                    {
                        params.settings.camera_fov = 45.0;
                        actions.changed = true;
                    }
                    if ui
                        .small_button("Tele")
                        .on_hover_text("Set FOV to 30° (telephoto/zoomed)")
                        .clicked()
                    {
                        params.settings.camera_fov = 30.0;
                        actions.changed = true;
                    }
                });

                ui.separator();
                ui.label("Auto Orbit:")
                    .on_hover_text("Automatically rotate camera around the fractal [O]");
                actions.changed |= ui
                    .checkbox(&mut params.settings.auto_orbit, "Enable Auto Orbit")
                    .on_hover_text("Toggle auto-orbit mode [O]")
                    .changed();
                if params.settings.auto_orbit {
                    actions.changed |= ui
                        .add(
                            egui::Slider::new(&mut params.settings.orbit_speed, 0.1..=3.0)
                                .text("Orbit Speed"),
                        )
                        .on_hover_text("Rotation speed in auto-orbit mode [ and ] to adjust")
                        .changed();
                }

                ui.separator();
                if ui
                    .checkbox(&mut self.show_camera_info, "Show Camera Info Overlay")
                    .on_hover_text("Display camera position and direction on screen")
                    .changed()
                {
                    self.ui_state.show_camera_info = self.show_camera_info;
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .button("🔄 Reset Camera")
                        .on_hover_text("Reset camera to default position")
                        .clicked()
                    {
                        actions.reset_camera_requested = true;
                    }
                    if ui
                        .button("🎯 Point at Fractal")
                        .on_hover_text("Aim camera at fractal center")
                        .clicked()
                    {
                        actions.point_at_fractal_requested = true;
                    }
                });

                ui.separator();
                ui.label("Camera Bookmarks:")
                    .on_hover_text("Save and restore camera viewpoints");

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.bookmark_name);
                });

                if ui
                    .button("📌 Save Bookmark")
                    .on_hover_text("Save current camera position")
                    .clicked()
                    && !self.bookmark_name.is_empty()
                {
                    let bookmark = CameraBookmark::new(
                        self.bookmark_name.clone(),
                        camera_pos,
                        camera_target,
                        params.settings.camera_fov,
                    );

                    // Sanitize filename
                    let filename = self
                        .bookmark_name
                        .replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");

                    if let Err(e) = BookmarkGallery::save_bookmark(&bookmark, &filename) {
                        log::error!("Failed to save bookmark: {}", e);
                    } else {
                        // Refresh bookmark list
                        self.bookmarks = BookmarkGallery::list_bookmarks().unwrap_or_default();
                        self.bookmark_name.clear();
                    }
                }

                // Refresh bookmark list periodically
                if self.last_bookmark_list_update.elapsed().as_secs() > 2 {
                    self.bookmarks = BookmarkGallery::list_bookmarks().unwrap_or_default();
                    self.last_bookmark_list_update = web_time::Instant::now();
                }

                if !self.bookmarks.is_empty() {
                    ui.separator();
                    ui.label("Saved Bookmarks:")
                        .on_hover_text("Click to load, right-click to delete");

                    egui::ScrollArea::vertical()
                        .id_salt("bookmarks_scroll")
                        .max_height(120.0)
                        .show(ui, |ui| {
                            let bookmarks_clone = self.bookmarks.clone();
                            for bookmark_name in bookmarks_clone.iter() {
                                ui.horizontal(|ui| {
                                    if ui
                                        .button(bookmark_name)
                                        .on_hover_text("Click to restore this camera position")
                                        .clicked()
                                        && let Ok(bookmark) =
                                            BookmarkGallery::load_bookmark(bookmark_name)
                                    {
                                        actions.bookmark_to_load = Some(bookmark);
                                    }
                                    if ui
                                        .small_button("🗑")
                                        .on_hover_text("Delete this bookmark")
                                        .clicked()
                                    {
                                        self.bookmark_to_delete = Some(bookmark_name.clone());
                                    }
                                });
                            }
                        });
                }

                // Handle bookmark deletion
                if let Some(ref bookmark_name) = self.bookmark_to_delete {
                    if let Err(e) = BookmarkGallery::delete_bookmark(bookmark_name) {
                        log::error!("Failed to delete bookmark: {}", e);
                    }
                    self.bookmarks = BookmarkGallery::list_bookmarks().unwrap_or_default();
                    self.bookmark_to_delete = None;
                }
            });
        self.ui_state.camera_open = response.openness > 0.0;
    }
}
