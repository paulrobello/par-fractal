use super::{Toast, UI};
use egui::Context;

/// Toast notification UI methods
impl UI {
    /// Show a simple toast notification without a file path.
    /// Replaces any existing simple toasts (without file paths) to prevent stacking.
    pub fn show_toast(&mut self, message: String) {
        // Remove existing simple toasts (those without file paths) to prevent stacking
        self.toasts.retain(|toast| toast.file_path.is_some());
        self.toasts.push(Toast::new(message, 2.0));
    }

    pub fn show_toast_with_file(&mut self, message: String, file_path: String) {
        self.toasts
            .push(Toast::with_file_path(message, file_path, 4.0));
    }

    /// Remove expired toasts
    fn cleanup_toasts(&mut self) {
        self.toasts.retain(|toast| !toast.is_expired());
    }

    /// Render toast notifications
    pub(super) fn render_toasts(&mut self, ctx: &Context) {
        self.cleanup_toasts();

        let spacing = 10.0;
        let mut y_offset = 0.0;

        for toast in &self.toasts {
            let opacity = toast.opacity();
            let file_path = toast.file_path.clone();

            let area_response =
                egui::Area::new(egui::Id::new(format!("toast_{:?}", toast.created_at)))
                    .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 40.0 + y_offset))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        let frame_response = egui::Frame::NONE
                            .fill(egui::Color32::from_rgba_premultiplied(
                                20,
                                20,
                                20,
                                (230.0 * opacity) as u8,
                            ))
                            .corner_radius(8.0)
                            .inner_margin(egui::Margin::symmetric(20, 14))
                            .stroke(egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgba_premultiplied(
                                    80,
                                    80,
                                    80,
                                    (180.0 * opacity) as u8,
                                ),
                            ))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(&toast.message).color(
                                    egui::Color32::from_rgba_premultiplied(
                                        255,
                                        255,
                                        255,
                                        (255.0 * opacity) as u8,
                                    ),
                                ))
                            });

                        // Make the entire frame clickable by interacting with its rect
                        ui.interact(
                            frame_response.response.rect,
                            egui::Id::new(format!("toast_click_{:?}", toast.created_at)),
                            egui::Sense::click(),
                        )
                    });

            let toast_response = area_response.inner;

            // Show cursor as pointer when hovering
            if toast_response.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            if toast_response.clicked() {
                eprintln!("DEBUG: Toast was clicked!");
                if let Some(path) = &file_path {
                    eprintln!("DEBUG: Attempting to open file: {}", path);

                    // Open file (native only)
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        // Verify file exists before trying to open
                        if std::path::Path::new(path).exists() {
                            eprintln!("DEBUG: File exists at path");
                        } else {
                            eprintln!("DEBUG: WARNING - File does not exist at path!");
                        }

                        if let Err(e) = open::that(path) {
                            eprintln!("ERROR: Failed to open file {}: {}", path, e);
                        } else {
                            eprintln!("DEBUG: Successfully called open::that()");
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        log::info!("Would open file: {}", path);
                    }
                } else {
                    eprintln!("DEBUG: No file_path available");
                }
            }

            y_offset += 60.0 + spacing;
        }
    }
}
