//! "Capture & Recording" window — screenshot, hi-res render, video recording.

use super::super::{UI, UiActions};
use egui::Context;

// VideoFormat is cfg-gated in the parent module (native uses the real type from
// `video_recorder`; web has a stub). Reach it via `super::` so this panel picks
// up whichever definition is in scope.
#[cfg(not(target_arch = "wasm32"))]
use super::super::VideoFormat;
#[cfg(target_arch = "wasm32")]
use super::super::VideoFormat;

impl UI {
    /// Render the Capture & Recording window. Owned by this module per AUDIT QA-009.
    pub fn render_capture_window(
        &mut self,
        ctx: &Context,
        actions: &mut UiActions,
        is_recording: bool,
    ) {
        if !self.ui_state.capture_window_open {
            return;
        }
        egui::Window::new("📷 Capture & Recording")
            .default_width(400.0)
            .resizable(true)
            .vscroll(true)
            .open(&mut self.ui_state.capture_window_open)
            .show(ctx, |ui| {
                ui.heading("Screenshot");

                if ui
                    .button("📷 Screen Resolution")
                    .on_hover_text("Capture current view at screen resolution [F12]")
                    .clicked()
                {
                    actions.screenshot_requested = true;
                }

                ui.label("Output: {fractal}_YYYYMMDD_HHMMSS.png")
                    .on_hover_text("Saved to current directory. {fractal} = fractal type name");

                ui.separator();
                ui.heading("🖥 Desktop Wallpaper")
                    .on_hover_text("Render at your monitor's native resolution for wallpapers");

                // Monitor selection UI
                if self.available_monitors.is_empty() {
                    if ui
                        .button("🔍 Detect Monitors")
                        .on_hover_text("Scan for available monitors")
                        .clicked()
                    {
                        // Signal to scan monitors - will be handled in app.rs
                        actions.gpu_scan_requested = true; // Reuse this flag temporarily
                    }
                    ui.label("⚠ No monitors detected. Click to scan.")
                        .on_hover_text("Scan for connected displays");
                } else {
                    ui.label("Select monitor:");

                    for (i, monitor) in self.available_monitors.iter().enumerate() {
                        let label = if monitor.is_primary {
                            format!(
                                "🌟 {} ({}x{}) - Primary",
                                monitor.name, monitor.width, monitor.height
                            )
                        } else {
                            format!("   {} ({}x{})", monitor.name, monitor.width, monitor.height)
                        };

                        if ui
                            .selectable_value(&mut self.selected_monitor_index, i, label)
                            .on_hover_text("Select this monitor for wallpaper rendering")
                            .clicked()
                        {
                            // Monitor selection changed
                        }
                    }

                    ui.add_space(4.0);
                    if let Some(monitor) = self.available_monitors.get(self.selected_monitor_index)
                        && ui
                            .button(format!(
                                "📐 Render {}x{} Wallpaper",
                                monitor.width, monitor.height
                            ))
                            .on_hover_text(format!(
                                "Render at {}'s native resolution",
                                monitor.name
                            ))
                            .clicked()
                    {
                        actions.hires_render_resolution = Some((monitor.width, monitor.height));
                    }

                    ui.horizontal(|ui| {
                        if ui
                            .button("🔄 Rescan Monitors")
                            .on_hover_text("Refresh monitor list")
                            .clicked()
                        {
                            actions.gpu_scan_requested = true; // Reuse this flag
                        }
                        ui.label(format!(
                            "{} monitor(s) detected",
                            self.available_monitors.len()
                        ));
                    });
                }

                ui.label("Output: {fractal}_WxH_YYYYMMDD_HHMMSS.png")
                    .on_hover_text("Saved to current directory. {fractal} = fractal type name");

                ui.separator();
                ui.heading("High-Resolution Render")
                    .on_hover_text("Render at custom resolutions");
                ui.label("Standard resolutions:");

                // Classic 4:3 resolutions
                ui.horizontal(|ui| {
                    if ui
                        .button("640x480 (VGA)")
                        .on_hover_text("Render at VGA resolution (4:3)")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((640, 480));
                    }
                    if ui
                        .button("800x600 (SVGA)")
                        .on_hover_text("Render at SVGA resolution (4:3)")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((800, 600));
                    }
                    if ui
                        .button("1024x768 (XGA)")
                        .on_hover_text("Render at XGA resolution (4:3)")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((1024, 768));
                    }
                });

                ui.horizontal(|ui| {
                    if ui
                        .button("1280x960")
                        .on_hover_text("Render at 1280x960 (4:3)")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((1280, 960));
                    }
                    if ui
                        .button("1400x1050")
                        .on_hover_text("Render at 1400x1050 (4:3)")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((1400, 1050));
                    }
                    if ui
                        .button("1600x1200 (UXGA)")
                        .on_hover_text("Render at UXGA resolution (4:3)")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((1600, 1200));
                    }
                });

                ui.add_space(4.0);
                ui.label("HD & modern resolutions:");

                // 16:9 HD resolutions
                ui.horizontal(|ui| {
                    if ui
                        .button("1280x720 (HD)")
                        .on_hover_text("Render at 720p HD resolution")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((1280, 720));
                    }
                    if ui
                        .button("1920x1080 (Full HD)")
                        .on_hover_text("Render at 1080p Full HD resolution")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((1920, 1080));
                    }
                    if ui
                        .button("2560x1440 (2K)")
                        .on_hover_text("Render at 1440p 2K resolution")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((2560, 1440));
                    }
                });

                ui.horizontal(|ui| {
                    if ui
                        .button("3840x2160 (4K)")
                        .on_hover_text("Render at 4K UHD resolution (may take time)")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((3840, 2160));
                    }
                    if ui
                        .button("7680x4320 (8K)")
                        .on_hover_text("Render at 8K resolution (will take significant time)")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((7680, 4320));
                    }
                });

                ui.add_space(4.0);
                ui.label("Square & social media:");

                // Square and other formats
                ui.horizontal(|ui| {
                    if ui
                        .button("800x800")
                        .on_hover_text("Small square format")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((800, 800));
                    }
                    if ui
                        .button("1080x1080 (Square)")
                        .on_hover_text("Square format for Instagram")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((1080, 1080));
                    }
                    if ui
                        .button("2048x2048")
                        .on_hover_text("Large square format")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((2048, 2048));
                    }
                });

                ui.horizontal(|ui| {
                    if ui
                        .button("1080x1920 (Portrait)")
                        .on_hover_text("Portrait format for mobile/stories")
                        .clicked()
                    {
                        actions.hires_render_resolution = Some((1080, 1920));
                    }
                });

                ui.add_space(4.0);
                ui.label("Custom resolution:");

                ui.horizontal(|ui| {
                    ui.label("Width:");
                    let width_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.custom_width)
                            .desired_width(80.0)
                            .hint_text("1920"),
                    );
                    ui.label("Height:");
                    let height_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.custom_height)
                            .desired_width(80.0)
                            .hint_text("1080"),
                    );
                    if width_resp.changed() || height_resp.changed() {
                        actions.changed = true;
                    }
                    if ui
                        .button("Render")
                        .on_hover_text("Render at custom resolution")
                        .clicked()
                    {
                        // Parse width and height
                        if let (Ok(width), Ok(height)) = (
                            self.custom_width.trim().parse::<u32>(),
                            self.custom_height.trim().parse::<u32>(),
                        ) {
                            if width > 0 && height > 0 && width <= 16384 && height <= 16384 {
                                actions.hires_render_resolution = Some((width, height));
                            } else {
                                // Show error toast for invalid dimensions
                                log::error!(
                                    "Invalid resolution: {}x{} (must be 1-16384)",
                                    width,
                                    height
                                );
                            }
                        } else {
                            log::error!("Failed to parse resolution");
                        }
                    }
                });

                ui.label("Output: {fractal}_WxH_YYYYMMDD_HHMMSS.png")
                    .on_hover_text("Saved to current directory. {fractal} = fractal type name");

                ui.add_space(4.0);
                let prev_auto_open = self.auto_open_captures;
                ui.checkbox(&mut self.auto_open_captures, "Auto-open captured images")
                    .on_hover_text("Automatically open captured images/videos after saving");
                if self.auto_open_captures != prev_auto_open {
                    actions.changed = true;
                }

                // Video recording section - native only
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.separator();
                    ui.heading("Video & GIF Recording")
                        .on_hover_text("Record animated videos or GIFs of your fractals");

                    ui.horizontal(|ui| {
                        ui.label("Format:");
                        ui.add_enabled(
                            !is_recording,
                            egui::RadioButton::new(self.video_format == VideoFormat::MP4, "MP4"),
                        )
                        .clicked()
                        .then(|| self.video_format = VideoFormat::MP4);
                        ui.add_enabled(
                            !is_recording,
                            egui::RadioButton::new(self.video_format == VideoFormat::WebM, "WebM"),
                        )
                        .clicked()
                        .then(|| self.video_format = VideoFormat::WebM);
                        ui.add_enabled(
                            !is_recording,
                            egui::RadioButton::new(self.video_format == VideoFormat::GIF, "GIF"),
                        )
                        .clicked()
                        .then(|| self.video_format = VideoFormat::GIF);
                    });

                    ui.horizontal(|ui| {
                        ui.label("FPS:");
                        let fps_range = if self.video_format == VideoFormat::GIF {
                            10..=30 // GIFs typically use lower FPS
                        } else {
                            24..=60
                        };
                        ui.add_enabled(
                            !is_recording,
                            egui::Slider::new(&mut self.video_fps, fps_range).text("fps"),
                        );
                    });

                    // Clamp FPS when switching to GIF
                    if self.video_format == VideoFormat::GIF && self.video_fps > 30 {
                        self.video_fps = 30;
                    } else if self.video_format != VideoFormat::GIF && self.video_fps < 24 {
                        self.video_fps = 24;
                    }

                    ui.horizontal(|ui| {
                        if !is_recording {
                            if ui
                                .button("🔴 Start Recording")
                                .on_hover_text("Begin recording (requires ffmpeg)")
                                .clicked()
                            {
                                actions.start_recording = true;
                            }
                        } else if ui
                            .button("⏹ Stop Recording")
                            .on_hover_text("Stop recording and save")
                            .clicked()
                        {
                            actions.stop_recording = true;
                        }
                    });

                    ui.label("Output: {fractal}_YYYYMMDD_HHMMSS.{mp4,webm,gif}")
                        .on_hover_text("Saved to current directory. {fractal} = fractal type name");

                    if self.video_format == VideoFormat::GIF {
                        ui.label("ℹ GIF: Optimized palette, looped, great for social media")
                            .on_hover_text(
                                "GIFs use palette-based encoding with dithering for best quality",
                            );
                    }

                    if !is_recording {
                        ui.label("⚠ Requires ffmpeg to be installed").on_hover_text(
                            "Install ffmpeg from your package manager or ffmpeg.org",
                        );
                    }
                }
            });
    }
}
