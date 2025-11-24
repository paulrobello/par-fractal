use super::App;

/// Capture and recording methods
impl App {
    pub(super) fn capture_screenshot(&mut self, texture: &wgpu::Texture) {
        let width = self.renderer.config.width;
        let height = self.renderer.config.height;

        // Calculate buffer size with proper alignment
        let bytes_per_row = (width * 4 + 255) & !255; // Align to 256 bytes
        let buffer_size = (bytes_per_row * height) as wgpu::BufferAddress;

        // Create buffer to copy texture to
        let buffer = self.renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Screenshot Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Create encoder for copy operation
        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Screenshot Encoder"),
                });

        // Copy texture to buffer
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.renderer
            .queue
            .submit(std::iter::once(encoder.finish()));

        // Map buffer and save to file
        let buffer_slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        // Wait for GPU to finish
        self.renderer
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .ok();

        if receiver.recv().unwrap().is_ok() {
            let data = buffer_slice.get_mapped_range();

            // Convert from padded buffer to image
            let mut image_data = Vec::with_capacity((width * height * 4) as usize);
            for row in 0..height {
                let row_start = (row * bytes_per_row) as usize;
                let row_data = &data[row_start..row_start + (width * 4) as usize];
                image_data.extend_from_slice(row_data);
            }

            drop(data);
            buffer.unmap();

            // Generate filename with fractal type and timestamp
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let fractal_name = self.fractal_params.fractal_type.filename_safe_name();
            let filename = format!("{}_{}.png", fractal_name, timestamp);

            // Save as PNG
            if let Some(img) = image::RgbaImage::from_raw(width, height, image_data) {
                if let Err(e) = img.save(&filename) {
                    eprintln!("Failed to save screenshot: {}", e);
                } else {
                    println!("Screenshot saved to {}", filename);
                    // Convert to absolute path and show in toast
                    let abs_path = std::path::Path::new(&filename)
                        .canonicalize()
                        .unwrap_or_else(|_| std::path::PathBuf::from(&filename));

                    // Auto-open if enabled
                    if self.ui.auto_open_captures {
                        if let Err(e) = open::that(&abs_path) {
                            eprintln!("Failed to open screenshot: {}", e);
                        }
                    }

                    self.ui.show_toast_with_file(
                        format!("📸 Screenshot saved: {} - Click to open", filename),
                        abs_path.to_string_lossy().to_string(),
                    );
                }
            } else {
                eprintln!("Failed to create image from buffer");
            }
        } else {
            eprintln!("Failed to map screenshot buffer");
        }
    }

    pub(super) fn capture_video_frame(&mut self, texture: &wgpu::Texture) {
        let width = self.renderer.config.width;
        let height = self.renderer.config.height;

        // Calculate buffer size with proper alignment
        let bytes_per_row = (width * 4 + 255) & !255; // Align to 256 bytes
        let buffer_size = (bytes_per_row * height) as wgpu::BufferAddress;

        // Create buffer to copy texture to
        let buffer = self.renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Video Frame Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Create encoder for copy operation
        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Video Frame Encoder"),
                });

        // Copy texture to buffer
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.renderer
            .queue
            .submit(std::iter::once(encoder.finish()));

        // Map buffer and add frame to video recorder
        let buffer_slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        // Wait for GPU to finish
        self.renderer
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .ok();

        if receiver.recv().unwrap().is_ok() {
            let data = buffer_slice.get_mapped_range();

            // Convert from padded buffer to unpadded RGBA data
            let mut frame_data = Vec::with_capacity((width * height * 4) as usize);
            for row in 0..height {
                let row_start = (row * bytes_per_row) as usize;
                let row_data = &data[row_start..row_start + (width * 4) as usize];
                frame_data.extend_from_slice(row_data);
            }

            drop(data);
            buffer.unmap();

            // Add frame to video recorder
            if let Err(e) = self.video_recorder.add_frame(frame_data) {
                eprintln!("Failed to add frame to video: {}", e);
            }
        } else {
            eprintln!("Failed to map video frame buffer");
        }
    }

    pub(super) fn render_high_resolution(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let hdr_format = wgpu::TextureFormat::Rgba16Float;

        // Helper to create HDR textures
        let create_hdr_texture = |label: &str| {
            self.renderer
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: hdr_format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
        };

        // Create all intermediate textures for post-processing pipeline
        let scene_texture = create_hdr_texture("High-Res Scene");
        let scene_view = scene_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bright_texture = create_hdr_texture("High-Res Bright");
        let bright_view = bright_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let blur_temp_texture = create_hdr_texture("High-Res Blur Temp");
        let blur_temp_view = blur_temp_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bloom_texture = create_hdr_texture("High-Res Bloom");
        let bloom_view = bloom_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let composite_texture = create_hdr_texture("High-Res Composite");
        let composite_view = composite_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create final output texture (8-bit for saving)
        let output_texture = self
            .renderer
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("High-Res Output"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.renderer.config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
        let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create bind groups for post-processing
        let texture_layout = self.renderer.copy_pipeline.get_bind_group_layout(0);
        let composite_layout = self.renderer.composite_pipeline.get_bind_group_layout(0);

        let scene_bind_group = self
            .renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("HR Scene BG"),
                layout: &texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&scene_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.renderer.sampler),
                    },
                ],
            });

        let bright_bind_group =
            self.renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("HR Bright BG"),
                    layout: &texture_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&bright_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.renderer.sampler),
                        },
                    ],
                });

        let blur_temp_bind_group =
            self.renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("HR Blur Temp BG"),
                    layout: &texture_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&blur_temp_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.renderer.sampler),
                        },
                    ],
                });

        let composite_bind_group =
            self.renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("HR Composite BG"),
                    layout: &composite_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&scene_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.renderer.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&bloom_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::Sampler(&self.renderer.sampler),
                        },
                    ],
                });

        let composite_final_bind_group =
            self.renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("HR Final BG"),
                    layout: &texture_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&composite_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.renderer.sampler),
                        },
                    ],
                });

        // Create temporary camera with correct aspect ratio
        let mut temp_camera = self.camera.clone();
        temp_camera.aspect = width as f32 / height as f32;
        self.renderer.update(&temp_camera, &self.fractal_params);

        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("High-Res Render Encoder"),
                });

        // Pass 1: Render fractal to scene texture
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("HR Fractal Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &scene_view,
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
            pass.set_pipeline(&self.renderer.render_pipeline);
            pass.set_bind_group(0, &self.renderer.uniform_bind_group, &[]);
            pass.set_vertex_buffer(0, self.renderer.vertex_buffer.slice(..));
            pass.draw(0..4, 0..1);
        }

        // Pass 2: Extract bright pixels for bloom
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("HR Bloom Extract"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &bright_view,
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
            pass.set_pipeline(&self.renderer.bloom_extract_pipeline);
            pass.set_bind_group(0, &scene_bind_group, &[]);
            pass.set_bind_group(1, &self.renderer.bloom_params_bind_group, &[]);
            pass.set_vertex_buffer(0, self.renderer.postprocess_vertex_buffer.slice(..));
            pass.draw(0..4, 0..1);
        }

        // Pass 3: Horizontal blur
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("HR Blur H"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &blur_temp_view,
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
            pass.set_pipeline(&self.renderer.blur_pipeline);
            pass.set_bind_group(0, &bright_bind_group, &[]);
            pass.set_bind_group(1, &self.renderer.blur_h_params_bind_group, &[]);
            pass.set_vertex_buffer(0, self.renderer.postprocess_vertex_buffer.slice(..));
            pass.draw(0..4, 0..1);
        }

        // Update blur direction to vertical
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct BlurUniforms {
            direction: [f32; 2],
            _padding: [f32; 2],
        }
        self.renderer.queue.write_buffer(
            &self.renderer.blur_uniform_buffer,
            0,
            bytemuck::cast_slice(&[BlurUniforms {
                direction: [0.0, 1.0],
                _padding: [0.0; 2],
            }]),
        );

        // Pass 4: Vertical blur
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("HR Blur V"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &bloom_view,
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
            pass.set_pipeline(&self.renderer.blur_pipeline);
            pass.set_bind_group(0, &blur_temp_bind_group, &[]);
            pass.set_bind_group(1, &self.renderer.blur_v_params_bind_group, &[]);
            pass.set_vertex_buffer(0, self.renderer.postprocess_vertex_buffer.slice(..));
            pass.draw(0..4, 0..1);
        }

        // Restore blur direction to horizontal for normal rendering
        self.renderer.queue.write_buffer(
            &self.renderer.blur_uniform_buffer,
            0,
            bytemuck::cast_slice(&[BlurUniforms {
                direction: [1.0, 0.0],
                _padding: [0.0; 2],
            }]),
        );

        // Pass 5: Composite (scene + bloom + color grading + vignette)
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("HR Composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &composite_view,
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
            pass.set_pipeline(&self.renderer.composite_pipeline);
            pass.set_bind_group(0, &composite_bind_group, &[]);
            pass.set_bind_group(1, &self.renderer.composite_params_bind_group, &[]);
            pass.set_vertex_buffer(0, self.renderer.postprocess_vertex_buffer.slice(..));
            pass.draw(0..4, 0..1);
        }

        // Pass 6: Final copy to 8-bit output
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("HR Final Copy"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
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
            pass.set_pipeline(&self.renderer.copy_pipeline);
            pass.set_bind_group(0, &composite_final_bind_group, &[]);
            pass.set_vertex_buffer(0, self.renderer.postprocess_vertex_buffer.slice(..));
            pass.draw(0..4, 0..1);
        }

        // Create buffer to copy texture to
        let bytes_per_row = (width * 4 + 255) & !255; // Align to 256 bytes
        let buffer_size = (bytes_per_row * height) as wgpu::BufferAddress;

        let buffer = self.renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("High-Res Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Copy texture to buffer
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.renderer
            .queue
            .submit(std::iter::once(encoder.finish()));

        // Restore original camera uniforms
        self.renderer.update(&self.camera, &self.fractal_params);

        // Map buffer and save to file
        let buffer_slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        // Wait for GPU to finish
        self.renderer
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .ok();

        if receiver.recv()?.is_ok() {
            let data = buffer_slice.get_mapped_range();

            // Convert from padded buffer to image
            let mut image_data = Vec::with_capacity((width * height * 4) as usize);
            for row in 0..height {
                let row_start = (row * bytes_per_row) as usize;
                let row_data = &data[row_start..row_start + (width * 4) as usize];
                image_data.extend_from_slice(row_data);
            }

            drop(data);
            buffer.unmap();

            // Convert BGRA to RGBA (surface format is Bgra8UnormSrgb)
            for pixel in image_data.chunks_exact_mut(4) {
                pixel.swap(0, 2); // Swap B and R
            }

            // Generate filename with fractal type, resolution, and timestamp
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let fractal_name = self.fractal_params.fractal_type.filename_safe_name();
            let filename = format!("{}_{}x{}_{}.png", fractal_name, width, height, timestamp);

            // Save as PNG
            if let Some(img) = image::RgbaImage::from_raw(width, height, image_data) {
                img.save(&filename)?;
                println!("High-resolution image saved to {}", filename);
                // Convert to absolute path and show in toast
                let abs_path = std::path::Path::new(&filename)
                    .canonicalize()
                    .unwrap_or_else(|_| std::path::PathBuf::from(&filename));

                // Auto-open if enabled
                if self.ui.auto_open_captures {
                    if let Err(e) = open::that(&abs_path) {
                        eprintln!("Failed to open high-res image: {}", e);
                    }
                }

                self.ui.show_toast_with_file(
                    format!("🖼️  High-res image saved: {} - Click to open", filename),
                    abs_path.to_string_lossy().to_string(),
                );
            } else {
                return Err("Failed to create image from buffer".into());
            }
        } else {
            return Err("Failed to map buffer".into());
        }

        Ok(())
    }
}
