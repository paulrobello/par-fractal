use super::{BloomUniforms, PostProcessUniforms, Renderer};
use crate::camera::Camera;
use crate::fractal::FractalParams;

/// Update and helper methods
impl Renderer {
    pub(super) fn create_render_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: &str,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    // Helper: Recreate all intermediate textures (for resize)
    fn recreate_textures(&mut self) {
        let (scene_texture, scene_view) = Self::create_render_texture(
            &self.device,
            self.size.width,
            self.size.height,
            "Scene Texture",
        );
        self.scene_texture = scene_texture;
        self.scene_view = scene_view;

        let (bright_texture, bright_view) = Self::create_render_texture(
            &self.device,
            self.size.width,
            self.size.height,
            "Bright Texture",
        );
        self.bright_texture = bright_texture;
        self.bright_view = bright_view;

        let (blur_temp_texture, blur_temp_view) = Self::create_render_texture(
            &self.device,
            self.size.width,
            self.size.height,
            "Blur Temp Texture",
        );
        self.blur_temp_texture = blur_temp_texture;
        self.blur_temp_view = blur_temp_view;

        let (bloom_texture, bloom_view) = Self::create_render_texture(
            &self.device,
            self.size.width,
            self.size.height,
            "Bloom Texture",
        );
        self.bloom_texture = bloom_texture;
        self.bloom_view = bloom_view;

        let (composite_texture, composite_view) = Self::create_render_texture(
            &self.device,
            self.size.width,
            self.size.height,
            "Composite Texture",
        );
        self.composite_texture = composite_texture;
        self.composite_view = composite_view;

        // Recreate bind groups that use these textures
        // We need to get the bind group layouts from the pipelines
        let texture_bind_group_layout = self.bloom_extract_pipeline.get_bind_group_layout(0);
        let composite_texture_layout = self.composite_pipeline.get_bind_group_layout(0);

        self.scene_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.bright_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bright Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.bright_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.blur_temp_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blur Temp Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.blur_temp_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.composite_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Composite Bind Group"),
            layout: &composite_texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.bloom_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.composite_final_bind_group =
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Composite Final Bind Group"),
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.composite_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // Recreate intermediate textures for multi-pass rendering
            self.recreate_textures();

            // Recreate accumulation texture if it exists (for strange attractors)
            // This ensures the accumulation matches the new window size
            if self.accumulation_texture.is_some() {
                // Clear the existing texture and let it be recreated on next frame
                self.accumulation_texture = None;
                self.accumulation_display_bind_group = None;
            }
        }
    }

    pub fn update(&mut self, camera: &Camera, params: &FractalParams) {
        let time = self.start_time.elapsed().as_secs_f32();
        self.uniforms.update(camera, params, time);
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );

        // Update post-processing uniforms
        let bloom_uniforms = BloomUniforms {
            threshold: params.bloom_threshold,
            intensity: params.bloom_intensity,
            _padding: [0.0; 2],
        };
        self.queue.write_buffer(
            &self.bloom_uniform_buffer,
            0,
            bytemuck::cast_slice(&[bloom_uniforms]),
        );

        // Blur uniforms don't change (direction is fixed)
        // We use the same buffer for both H and V passes, just different bind groups

        let composite_uniforms = PostProcessUniforms {
            brightness: params.brightness,
            contrast: params.contrast,
            saturation: params.saturation,
            hue_shift: params.hue_shift,
            vignette_enabled: if params.vignette_enabled { 1 } else { 0 },
            vignette_intensity: params.vignette_intensity,
            vignette_radius: params.vignette_radius,
            _padding1: 0.0,
            bloom_enabled: if params.bloom_enabled { 1 } else { 0 },
            bloom_intensity: params.bloom_intensity,
            _padding2: [0.0; 2],
            _padding3: [0.0; 4],
        };
        self.queue.write_buffer(
            &self.composite_uniform_buffer,
            0,
            bytemuck::cast_slice(&[composite_uniforms]),
        );
    }
}
