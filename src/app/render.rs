use super::App;
use crate::fractal::FractalParams;
use crate::renderer::compute::{AccumulationDisplayUniforms, AttractorComputeUniforms};

#[cfg(not(target_arch = "wasm32"))]
use crate::video_recorder::VideoRecorder;

/// Render methods
impl App {
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.renderer.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // Check if we should use accumulation mode for strange attractors
        let use_accumulation = self.fractal_params.attractor_accumulation_enabled
            && self.fractal_params.fractal_type.is_2d_attractor();

        if use_accumulation {
            // Check if texture needs recreation (None or wrong size)
            let texture_needs_recreation = match &self.renderer.accumulation_texture {
                None => true,
                Some(tex) => {
                    tex.width != self.renderer.size.width || tex.height != self.renderer.size.height
                }
            };

            // Initialize compute infrastructure if needed (handles resize too)
            self.renderer.init_accumulation_compute();

            // Reset iteration counter if texture was just recreated
            if texture_needs_recreation {
                self.fractal_params.attractor_total_iterations = 0;
            }

            // Auto-clear when view parameters change (zoom, pan, or attractor params)
            let view_changed = self.fractal_params.center_2d
                != self.fractal_params.attractor_last_center
                || self.fractal_params.zoom_2d != self.fractal_params.attractor_last_zoom
                || self.fractal_params.julia_c != self.fractal_params.attractor_last_julia_c;

            if view_changed {
                self.fractal_params.attractor_pending_clear = true;
                // Update last values
                self.fractal_params.attractor_last_center = self.fractal_params.center_2d;
                self.fractal_params.attractor_last_zoom = self.fractal_params.zoom_2d;
                self.fractal_params.attractor_last_julia_c = self.fractal_params.julia_c;
            }

            // Handle clear request
            if self.fractal_params.attractor_pending_clear {
                if let Some(ref accum_tex) = self.renderer.accumulation_texture {
                    accum_tex.clear(&self.renderer.device, &self.renderer.queue);
                }
                self.fractal_params.attractor_pending_clear = false;
                self.fractal_params.attractor_total_iterations = 0;
            }

            // Update compute uniforms
            if let Some(ref mut compute) = self.renderer.attractor_compute {
                compute.uniforms = AttractorComputeUniforms {
                    param_a: self.fractal_params.julia_c[0],
                    param_b: self.fractal_params.julia_c[1],
                    param_c: 0.0, // Could expose more params
                    param_d: 0.0,
                    center_x: self.fractal_params.center_2d[0] as f32,
                    center_y: self.fractal_params.center_2d[1] as f32,
                    zoom: self.fractal_params.zoom_2d,
                    aspect_ratio: self.renderer.size.width as f32
                        / self.renderer.size.height as f32,
                    width: self.renderer.size.width,
                    height: self.renderer.size.height,
                    iterations_per_frame: self.fractal_params.attractor_iterations_per_frame,
                    attractor_type: self.fractal_params.fractal_type.attractor_index(),
                    total_iterations: self.fractal_params.attractor_total_iterations as u32,
                    clear_accumulation: 0,
                    _padding: [0; 2],
                };
                compute.update_uniforms(&self.renderer.queue);

                // Dispatch compute shader
                if let Some(ref accum_tex) = self.renderer.accumulation_texture {
                    // Number of workgroups: iterations_per_frame / (256 threads * iterations_per_thread)
                    // Each thread does iterations_per_frame / 256 iterations
                    // We want ~iterations_per_frame total, so dispatch (iterations / 256) / per_thread
                    // Simplify: dispatch enough to cover all iterations
                    let num_workgroups =
                        (self.fractal_params.attractor_iterations_per_frame / 256).max(1);
                    compute.dispatch(&mut encoder, &accum_tex.compute_bind_group, num_workgroups);
                }

                // Update total iterations counter
                self.fractal_params.attractor_total_iterations +=
                    self.fractal_params.attractor_iterations_per_frame as u64;
            }

            // Update accumulation display uniforms with palette from fractal params
            let palette_colors = self.fractal_params.palette.colors;
            let display_uniforms = AccumulationDisplayUniforms {
                log_scale: self.fractal_params.attractor_log_scale,
                gamma: 0.6,
                palette_offset: self.fractal_params.palette_offset,
                _padding: 0.0,
                palette: [
                    [
                        palette_colors[0].x,
                        palette_colors[0].y,
                        palette_colors[0].z,
                        1.0,
                    ],
                    [
                        palette_colors[1].x,
                        palette_colors[1].y,
                        palette_colors[1].z,
                        1.0,
                    ],
                    [
                        palette_colors[2].x,
                        palette_colors[2].y,
                        palette_colors[2].z,
                        1.0,
                    ],
                    [
                        palette_colors[3].x,
                        palette_colors[3].y,
                        palette_colors[3].z,
                        1.0,
                    ],
                    [
                        palette_colors[4].x,
                        palette_colors[4].y,
                        palette_colors[4].z,
                        1.0,
                    ],
                ],
            };
            self.renderer.queue.write_buffer(
                &self.renderer.accumulation_display_uniform_buffer,
                0,
                bytemuck::cast_slice(&[display_uniforms]),
            );

            // Render accumulation texture to scene_texture with log scaling
            {
                let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Accumulation Display Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.renderer.scene_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                let mut render_pass: wgpu::RenderPass<'static> =
                    unsafe { std::mem::transmute(render_pass) };

                // Use the accumulation display pipeline with log scaling visualization
                if let Some(ref bind_group) = self.renderer.accumulation_display_bind_group {
                    render_pass.set_pipeline(&self.renderer.accumulation_display_pipeline);
                    render_pass.set_bind_group(0, bind_group, &[]);
                    render_pass.set_bind_group(
                        1,
                        &self.renderer.accumulation_display_uniform_bind_group,
                        &[],
                    );
                    render_pass
                        .set_vertex_buffer(0, self.renderer.postprocess_vertex_buffer.slice(..));
                    render_pass.draw(0..4, 0..1);
                }
            }
        } else {
            // Multi-pass rendering pipeline
            // Pass 1: Render fractal to scene_texture
            {
                let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Scene Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.renderer.scene_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                // SAFETY: We drop the render_pass before using encoder again, so this is safe.
                let mut render_pass: wgpu::RenderPass<'static> =
                    unsafe { std::mem::transmute(render_pass) };

                render_pass.set_pipeline(&self.renderer.render_pipeline);
                render_pass.set_bind_group(0, &self.renderer.uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.renderer.vertex_buffer.slice(..));
                render_pass.draw(0..4, 0..1);
            }
        }

        // Pass 2-4: Bloom pipeline (always run to keep texture valid)
        if true {
            // Always run bloom passes, composite will decide whether to use it
            // Pass 2: Extract bright pixels
            {
                let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Bloom Extract Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.renderer.bright_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                // SAFETY: We drop the render_pass before using encoder again, so this is safe.
                let mut render_pass: wgpu::RenderPass<'static> =
                    unsafe { std::mem::transmute(render_pass) };

                render_pass.set_pipeline(&self.renderer.bloom_extract_pipeline);
                render_pass.set_bind_group(0, &self.renderer.scene_bind_group, &[]);
                render_pass.set_bind_group(1, &self.renderer.bloom_params_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.renderer.postprocess_vertex_buffer.slice(..));
                render_pass.draw(0..4, 0..1);
            }

            // Pass 3: Horizontal blur
            {
                let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Blur Horizontal Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.renderer.blur_temp_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                // SAFETY: We drop the render_pass before using encoder again, so this is safe.
                let mut render_pass: wgpu::RenderPass<'static> =
                    unsafe { std::mem::transmute(render_pass) };

                render_pass.set_pipeline(&self.renderer.blur_pipeline);
                render_pass.set_bind_group(0, &self.renderer.bright_bind_group, &[]);
                render_pass.set_bind_group(1, &self.renderer.blur_h_params_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.renderer.postprocess_vertex_buffer.slice(..));
                render_pass.draw(0..4, 0..1);
            }

            // Update blur buffer to vertical direction for next pass
            #[repr(C)]
            #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
            struct BlurUniforms {
                direction: [f32; 2],
                _padding: [f32; 2],
            }
            let blur_v_uniforms = BlurUniforms {
                direction: [0.0, 1.0], // Vertical
                _padding: [0.0; 2],
            };
            self.renderer.queue.write_buffer(
                &self.renderer.blur_uniform_buffer,
                0,
                bytemuck::cast_slice(&[blur_v_uniforms]),
            );

            // Pass 4: Vertical blur
            {
                let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Blur Vertical Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.renderer.bloom_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                // SAFETY: We drop the render_pass before using encoder again, so this is safe.
                let mut render_pass: wgpu::RenderPass<'static> =
                    unsafe { std::mem::transmute(render_pass) };

                render_pass.set_pipeline(&self.renderer.blur_pipeline);
                render_pass.set_bind_group(0, &self.renderer.blur_temp_bind_group, &[]);
                render_pass.set_bind_group(1, &self.renderer.blur_v_params_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.renderer.postprocess_vertex_buffer.slice(..));
                render_pass.draw(0..4, 0..1);
            }
        }

        // Pass 5: Composite (scene + bloom + color grading + vignette)
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Composite Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.renderer.composite_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // SAFETY: We drop the render_pass before using encoder again, so this is safe.
            let mut render_pass: wgpu::RenderPass<'static> =
                unsafe { std::mem::transmute(render_pass) };

            render_pass.set_pipeline(&self.renderer.composite_pipeline);
            render_pass.set_bind_group(0, &self.renderer.composite_bind_group, &[]);
            render_pass.set_bind_group(1, &self.renderer.composite_params_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.renderer.postprocess_vertex_buffer.slice(..));
            render_pass.draw(0..4, 0..1);
        }

        // Pass 6: FXAA or direct copy to screen
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Final Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // SAFETY: We drop the render_pass before using encoder again, so this is safe.
            let mut render_pass: wgpu::RenderPass<'static> =
                unsafe { std::mem::transmute(render_pass) };

            if self.fractal_params.fxaa_enabled {
                // Apply FXAA anti-aliasing to composite texture
                render_pass.set_pipeline(&self.renderer.fxaa_pipeline);
                render_pass.set_bind_group(0, &self.renderer.composite_final_bind_group, &[]);
            } else {
                // Direct copy from composite to screen
                render_pass.set_pipeline(&self.renderer.copy_pipeline);
                render_pass.set_bind_group(0, &self.renderer.composite_final_bind_group, &[]);
            }

            render_pass.set_vertex_buffer(0, self.renderer.postprocess_vertex_buffer.slice(..));
            render_pass.draw(0..4, 0..1);
        }

        // If screenshot requested or recording, capture fractal before UI is rendered
        let should_screenshot = self.save_screenshot;
        #[cfg(not(target_arch = "wasm32"))]
        let is_recording = self.video_recorder.is_recording();
        #[cfg(target_arch = "wasm32")]
        let is_recording = false; // Video recording not supported on web

        if should_screenshot || is_recording {
            // Submit the fractal rendering first
            self.renderer
                .queue
                .submit(std::iter::once(encoder.finish()));

            if should_screenshot {
                // Capture the screenshot (fractal only)
                #[cfg(not(target_arch = "wasm32"))]
                self.capture_screenshot(&output.texture);
                #[cfg(target_arch = "wasm32")]
                {
                    let fractal_name = self
                        .fractal_params
                        .fractal_type
                        .filename_safe_name()
                        .to_string();
                    let width = self.renderer.config.width;
                    let height = self.renderer.config.height;
                    // Create a closure that captures what we need for the toast
                    let show_toast: Box<dyn Fn(String) + Send + 'static> =
                        Box::new(move |msg: String| {
                            log::info!("{}", msg);
                        });
                    super::capture_web::capture_screenshot_web(
                        &self.renderer.device,
                        &self.renderer.queue,
                        &output.texture,
                        width,
                        height,
                        fractal_name,
                        show_toast,
                    );
                }
                self.save_screenshot = false;
            }

            #[cfg(not(target_arch = "wasm32"))]
            if is_recording {
                // Capture video frame (fractal only) - native only
                self.capture_video_frame(&output.texture);
            }

            // Create a new encoder for UI rendering
            encoder =
                self.renderer
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("UI Render Encoder"),
                    });
        }

        // Render UI
        let raw_input = self.egui_state.take_egui_input(self.window.as_ref());
        let full_output = self.egui_state.egui_ctx().run(raw_input, |ctx| {
            #[cfg(not(target_arch = "wasm32"))]
            let is_rec = self.video_recorder.is_recording();
            #[cfg(target_arch = "wasm32")]
            let is_rec = false; // Video recording not supported on web

            let (
                changed,
                screenshot_requested,
                reset_requested,
                reset_camera_requested,
                point_at_fractal_requested,
                preset_to_load,
                hires_render_resolution,
                bookmark_to_load,
                gpu_scan_requested,
                start_recording,
                stop_recording,
            ) = self.ui.render(
                ctx,
                &mut self.fractal_params,
                self.camera.position,
                self.camera.target,
                is_rec,
            );

            // Render command palette overlay (always on top)
            if let Some(command_action) = self.ui.render_command_palette(ctx) {
                let (changed, message) = self
                    .ui
                    .execute_command(command_action, &mut self.fractal_params);

                if changed {
                    self.settings_last_changed = web_time::Instant::now();
                    self.settings_need_save = true;
                }

                if let Some(msg) = message {
                    println!("Command executed: {}", msg);
                }
            }

            // Handle GPU scan request / Monitor scan request
            if gpu_scan_requested {
                // Scan monitors (always do this when the button is clicked)
                self.ui.scan_monitors(&self.window);

                // Also scan GPUs for backward compatibility (native only)
                #[cfg(not(target_arch = "wasm32"))]
                {
                    // Spawn async task to enumerate GPUs
                    // Note: We can't easily do async here, so we'll use pollster to block
                    let gpus = pollster::block_on(crate::renderer::Renderer::enumerate_gpus());
                    self.ui.available_gpus = gpus;
                    self.ui.gpu_selection_message =
                        Some(format!("Found {} GPU(s)", self.ui.available_gpus.len()));
                }
                #[cfg(target_arch = "wasm32")]
                {
                    self.ui.gpu_selection_message =
                        Some("GPU selection not available on web".to_string());
                }
            }

            // Handle preset loading
            if let Some(preset) = preset_to_load {
                println!("Loading preset: {}", preset.name);
                self.fractal_params = FractalParams::from_settings(preset.settings.clone());

                // Apply camera settings from preset
                self.camera.position = glam::Vec3::from_array(preset.settings.camera_position);
                self.camera.target = glam::Vec3::from_array(preset.settings.camera_target);
                self.camera.fovy = preset.settings.camera_fov;

                // Update camera controller
                self.camera_controller
                    .set_speed(preset.settings.camera_speed);
                self.camera_controller
                    .point_at_target(self.camera.position, self.camera.target);

                // Mark settings for save
                self.settings_last_changed = web_time::Instant::now();
                self.settings_need_save = true;
            }

            // Handle camera bookmark loading
            if let Some(bookmark) = bookmark_to_load {
                println!("Loading camera bookmark: {}", bookmark.name);
                if self.smooth_transitions_enabled {
                    // Start smooth transition
                    self.camera_transition.start(
                        self.camera.position,
                        self.camera.target,
                        self.camera.fovy,
                        bookmark.get_position(),
                        bookmark.get_target(),
                        bookmark.fov,
                        1.5, // 1.5 second transition
                    );
                } else {
                    // Instant jump
                    self.camera.position = bookmark.get_position();
                    self.camera.target = bookmark.get_target();
                    self.camera.fovy = bookmark.fov;
                    self.camera_controller
                        .point_at_target(self.camera.position, self.camera.target);
                }
                self.fractal_params.camera_fov = bookmark.fov;
            }

            if reset_requested {
                self.fractal_params = FractalParams::default();
                // Reset camera to default position and settings
                self.camera.reset_to_default();
                self.camera.fovy = self.fractal_params.camera_fov;
                self.camera_controller
                    .set_speed(self.fractal_params.camera_speed);
                // Sync controller with reset camera position
                self.camera_controller
                    .point_at_target(self.camera.position, self.camera.target);
                println!("Settings and camera reset to defaults");
            }

            if reset_camera_requested {
                self.camera.reset_to_default();
                self.camera.fovy = self.fractal_params.camera_fov;
                // Sync controller with reset camera position
                self.camera_controller
                    .point_at_target(self.camera.position, self.camera.target);
                println!("Camera reset to default position");
            }

            if point_at_fractal_requested {
                self.camera_controller
                    .point_at_target(self.camera.position, glam::Vec3::ZERO);
                println!("Camera pointed at fractal");
            }

            if screenshot_requested {
                self.save_screenshot = true;
            }

            if let Some(resolution) = hires_render_resolution {
                self.save_hires_render = Some(resolution);
                println!(
                    "High-resolution render requested: {}x{}",
                    resolution.0, resolution.1
                );
            }

            // Handle video recording (native only)
            #[cfg(not(target_arch = "wasm32"))]
            {
                if start_recording {
                    // Generate filename with fractal type and timestamp
                    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                    let fractal_name = self.fractal_params.fractal_type.filename_safe_name();
                    let filename = format!(
                        "{}_{}.{}",
                        fractal_name,
                        timestamp,
                        self.ui.video_format.extension()
                    );

                    // Update video recorder settings
                    self.video_recorder = VideoRecorder::new(
                        self.renderer.config.width,
                        self.renderer.config.height,
                        self.ui.video_fps,
                        self.ui.video_format,
                    );

                    if let Err(e) = self.video_recorder.start_recording(filename.clone()) {
                        eprintln!("Failed to start recording: {}", e);
                    } else {
                        println!("Started recording to {}", filename);
                    }
                }

                if stop_recording {
                    match self.video_recorder.stop_recording() {
                        Ok(filename) => {
                            // Convert to absolute path and show in toast
                            let abs_path = std::path::Path::new(&filename)
                                .canonicalize()
                                .unwrap_or_else(|_| std::path::PathBuf::from(&filename));

                            // Auto-open if enabled
                            if self.ui.auto_open_captures {
                                if let Err(e) = open::that(&abs_path) {
                                    eprintln!("Failed to open video: {}", e);
                                }
                            }

                            self.ui.show_toast_with_file(
                                format!("🎬 Video saved: {} - Click to open", filename),
                                abs_path.to_string_lossy().to_string(),
                            );
                        }
                        Err(e) => {
                            eprintln!("Failed to stop recording: {}", e);
                        }
                    }
                }
            }
            // Video recording not supported on web - UI section is hidden via cfg

            // Mark settings for auto-save (debounced)
            if changed {
                self.settings_last_changed = web_time::Instant::now();
                self.settings_need_save = true;

                // Update camera parameters from fractal_params
                self.camera.fovy = self.fractal_params.camera_fov;
                self.camera_controller
                    .set_speed(self.fractal_params.camera_speed);
            }

            self.ui.render_fps(ctx, self.current_fps);
            self.ui.render_camera_info(
                ctx,
                self.camera.position,
                self.camera.target,
                &self.fractal_params.lod_config.distance_zones,
            );
            self.ui.render_performance_overlay(ctx, self.current_fps);
            #[cfg(not(target_arch = "wasm32"))]
            self.ui.render_recording_indicator(
                ctx,
                self.video_recorder.is_recording(),
                self.video_recorder.frame_count(),
                self.video_recorder.filename(),
            );
            // No recording indicator on web - video recording not supported
            self.ui.render_lod_debug_overlay(ctx, &self.fractal_params);
        });

        self.egui_state
            .handle_platform_output(self.window.as_ref(), full_output.platform_output);

        let tris = self
            .egui_state
            .egui_ctx()
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(
                &self.renderer.device,
                &self.renderer.queue,
                *id,
                image_delta,
            );
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.renderer.config.width, self.renderer.config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        self.egui_renderer.update_buffers(
            &self.renderer.device,
            &self.renderer.queue,
            &mut encoder,
            &tris,
            &screen_descriptor,
        );

        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // SAFETY: We drop the render_pass before using encoder again, so this is safe.
            let mut render_pass: wgpu::RenderPass<'static> =
                unsafe { std::mem::transmute(render_pass) };

            self.egui_renderer
                .render(&mut render_pass, &tris, &screen_descriptor);
            drop(render_pass);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.renderer
            .queue
            .submit(std::iter::once(encoder.finish()));

        output.present();

        Ok(())
    }
}
