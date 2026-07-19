use super::{BloomUniforms, PostProcessUniforms, Renderer, write_uniform_bytes};
use crate::camera::Camera;
use crate::deep_zoom::{ReferenceOrbit, perturbation_eligible};
use crate::fractal::FractalParams;
use crate::renderer::uniforms::scene_uv_scale_for;

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

    /// ENH-005: bloom extract + both blur passes run at half the surface
    /// resolution. Blur is a low-pass filter, so downsampling first is visually
    /// near-lossless and cuts bloom bandwidth/fragment work ~4×; composite
    /// upsamples bilinearly. The blur shader derives its texel offset from
    /// `textureDimensions` of the (now half-size) bound texture, so no texel-
    /// size uniform needs updating. Halved dims clamped to ≥1 keep odd/tiny
    /// window sizes valid as GPU texture dimensions.
    pub(super) fn bloom_size(width: u32, height: u32) -> (u32, u32) {
        ((width / 2).max(1), (height / 2).max(1))
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

        // ENH-005: bloom extract + both blur passes run at half resolution
        // (see `bloom_size`); scene + composite stay full-res.
        let (bw, bh) = Self::bloom_size(self.size.width, self.size.height);

        let (bright_texture, bright_view) =
            Self::create_render_texture(&self.device, bw, bh, "Bright Texture");
        self.bright_texture = bright_texture;
        self.bright_view = bright_view;

        let (blur_temp_texture, blur_temp_view) =
            Self::create_render_texture(&self.device, bw, bh, "Blur Temp Texture");
        self.blur_temp_texture = blur_temp_texture;
        self.blur_temp_view = blur_temp_view;

        let (bloom_texture, bloom_view) =
            Self::create_render_texture(&self.device, bw, bh, "Bloom Texture");
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

    /// ENH-003: force the render scale used by the next `update()` call —
    /// `Some(1.0)` makes high-resolution capture / video render at full quality
    /// regardless of LOD's motion-driven scale; `None` reverts to LOD-driven.
    /// The override is read once in `update()`; callers restore `None` right
    /// after so subsequent normal frames are unaffected.
    pub fn set_render_scale_override(&mut self, scale: Option<f32>) {
        self.render_scale_override = scale;
    }

    /// ENH-003: write the bloom + composite post-uniform buffers at full
    /// resolution (`scene_uv_scale = [1,1]`) for a capture path that renders
    /// its own full-size `scene_texture` but reuses the shared post-uniform
    /// bind groups (the wasm high-res path, which does not call `update`).
    /// Without this, a capture triggered mid-LOD-motion would inherit the last
    /// interactive frame's `scene_uv_scale < 1` and the post passes would
    /// sample a sub-rect of the capture texture. Does NOT touch the
    /// change-detection cache — a capture is one-shot, and the next normal
    /// `update()` re-writes with the correct interactive value.
    ///
    /// Wasm-only: its sole caller is `capture_web::render_high_resolution_web`.
    #[cfg(target_arch = "wasm32")]
    pub fn write_full_quality_post_uniforms(&self, params: &FractalParams) {
        let bloom = BloomUniforms::from_params(params, [1.0, 1.0]);
        let bloom_bytes = write_uniform_bytes(&bloom);
        self.queue
            .write_buffer(&self.bloom_uniform_buffer, 0, &bloom_bytes);
        let composite = PostProcessUniforms::from_params(params, [1.0, 1.0]);
        let composite_bytes = write_uniform_bytes(&composite);
        self.queue
            .write_buffer(&self.composite_uniform_buffer, 0, &composite_bytes);
    }

    pub fn update(&mut self, camera: &Camera, params: &FractalParams) {
        let time = self.start_time.elapsed().as_secs_f32();
        self.uniforms.update(camera, params, time);

        // ENH-001 Phase A step 5: overlay perturbation uniforms when an
        // active orbit has been uploaded AND the gate is met. `Uniforms::update`
        // just zeroed these; we repopulate them here so the shader's delta
        // path renders. While the worker is in flight (no active orbit yet),
        // while the gate isn't met (non-Mandelbrot, 3D, or below the zoom
        // threshold), or after the view drops below the gate, the uniforms
        // stay at zero and the HP path renders — perturbation is purely
        // additive and the DF / f32 paths remain intact below the gate.
        if let Some(active) = self.active_orbit
            && perturbation_eligible(
                params.settings.zoom_2d,
                params.settings.fractal_type,
                params.settings.render_mode,
            )
        {
            self.uniforms.activate_perturbation(
                active.len,
                active.escaped_at,
                params.settings.zoom_2d,
                camera.aspect,
                active.reference_offset,
            );
        }

        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );

        // ENH-003: resolve the render scale for this frame. LOD's active
        // QualityLevel.render_scale drives it (so it tracks motion
        // automatically — lower presets render the scene pass into a sub-rect
        // of scene_texture and the post chain upsamples). Forced to 1.0 when:
        //   - a capture path set `render_scale_override` (full-quality output), or
        //   - accumulation display is active (its display pass writes scene_texture
        //     at full resolution, so the sub-rect sampling must be a no-op).
        // `App::render` reads `scene_render_scale` back to set the scene viewport.
        let scene_render_scale = self.render_scale_override.unwrap_or_else(|| {
            // ENH-002 v2: tile refinement rasterizes full-resolution tiles, so
            // force full res (and thus scene_uv_scale = [1,1]) while refining.
            if self.refining {
                return 1.0;
            }
            let is_accumulation = params.settings.attractor_accumulation_enabled
                && (params.settings.fractal_type.is_2d_attractor()
                    || params.settings.fractal_type.is_buddhabrot());
            if is_accumulation {
                1.0
            } else {
                params.effective_quality().render_scale.clamp(0.25, 1.0)
            }
        });
        self.scene_render_scale = scene_render_scale;
        let scene_uv_scale =
            scene_uv_scale_for(scene_render_scale, self.config.width, self.config.height);

        // ARC-017: gate the post-processing uniform uploads behind change
        // detection. With ARC-006's dirty-flag redraw skipping most frames
        // while idle, the `time`-driven main uniform still has to be rewritten
        // each rendered frame (palette/animation fields change); but the
        // bloom and composite params are static across typical interaction,
        // so skipping the `write_buffer` when the value matches the cache
        // saves two <1KB uploads per rendered frame. PartialEq on the structs
        // is field-wise (padding is consistent because both derive Pod).
        let bloom_uniforms = BloomUniforms::from_params(params, scene_uv_scale);
        if self.cached_bloom_uniforms != Some(bloom_uniforms) {
            self.queue.write_buffer(
                &self.bloom_uniform_buffer,
                0,
                &write_uniform_bytes(&bloom_uniforms),
            );
            self.cached_bloom_uniforms = Some(bloom_uniforms);
        }

        // Blur uniforms don't change (direction is fixed)
        // We use the same buffer for both H and V passes, just different bind groups

        let composite_uniforms = PostProcessUniforms::from_params(params, scene_uv_scale);
        if self.cached_composite_uniforms != Some(composite_uniforms) {
            self.queue.write_buffer(
                &self.composite_uniform_buffer,
                0,
                &write_uniform_bytes(&composite_uniforms),
            );
            self.cached_composite_uniforms = Some(composite_uniforms);
        }
    }

    /// ENH-001 Phase A step 5: upload a freshly-computed reference orbit and
    /// mark it active.
    ///
    /// Grows the storage buffer when the new orbit exceeds the current
    /// capacity, and — critically — rebuilds `uniform_bind_group` in that
    /// case so binding(1) points at the new buffer. The bind group held the
    /// placeholder buffer from initialization; without this rebuild the
    /// shader would keep reading the (zeroed) placeholder and the delta path
    /// would render as if `orbit_len == 0`.
    ///
    /// GPU-only side of the driver: the CPU computation happened on the
    /// worker thread (`deep_zoom::driver`); this method runs on the
    /// render-thread owner of `device`/`queue` (the only thread allowed to
    /// touch GPU state). After this returns, `Renderer::update` will see
    /// `active_orbit = Some(...)` and populate the perturbation uniforms on
    /// the next frame (and every frame until the view changes again).
    pub fn set_reference_orbit(&mut self, orbit: &ReferenceOrbit) {
        log::info!(
            "deep-zoom reference orbit uploaded: {} entries, escaped_at={:?}, precision_bits={}",
            orbit.z.len(),
            orbit.escaped_at,
            orbit.precision_bits,
        );
        let grew = self
            .orbit_buffer
            .ensure_capacity(&self.device, orbit.z.len());
        if grew {
            // Re-create the bind group so binding(1) references the new
            // (larger) orbit buffer. Layout comes from the render pipeline,
            // which was built against `uniform_bind_group_layout` in
            // `initialization.rs` — both pipelines share that one layout.
            let layout = self.pipeline_2d.get_bind_group_layout(0);
            self.uniform_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Uniform Bind Group (post-orbit-realloc)"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.orbit_buffer.buffer.as_entire_binding(),
                    },
                ],
            });
        }
        self.orbit_buffer.write(&self.queue, &orbit.z);
        self.active_orbit = Some(super::orbit_buffer::ActiveOrbit {
            len: orbit.z.len() as u32,
            escaped_at: orbit.escaped_at.unwrap_or(0),
            reference_offset: orbit.reference_offset,
        });
    }

    /// ENH-001 Phase A step 5: drop the active orbit so the shader's
    /// perturbation branch turns off on the next frame.
    ///
    /// Called when the view drops below the activation gate (zoom too low,
    /// fractal type changed away from Mandelbrot, or render mode switched to
    /// 3D). The GPU buffer contents are left in place — `orbit_len == 0` in
    /// the uniforms (re-zeroed by `Uniforms::update` next frame)
    /// short-circuits any read, so stale buffer contents are never sampled.
    pub fn clear_reference_orbit(&mut self) {
        self.active_orbit = None;
    }
}

#[cfg(test)]
mod enh_005_tests {
    use super::Renderer;

    /// ENH-005: `bloom_size` must integer-divide by 2 and never return 0
    /// (a 0-dim GPU texture panics at creation). Locks the floor, the
    /// truncation-on-odd behavior, and the w/h pairing against a future edit
    /// that drops `.max(1)` or swaps the components.
    #[test]
    fn bloom_size_floors_and_halves() {
        // Zero / one px → floored to 1 (the crash guard).
        assert_eq!(Renderer::bloom_size(0, 0), (1, 1));
        assert_eq!(Renderer::bloom_size(1, 1), (1, 1));
        assert_eq!(Renderer::bloom_size(2, 2), (1, 1));
        // Odd dims truncate down (integer division), never round past half.
        assert_eq!(Renderer::bloom_size(3, 3), (1, 1));
        assert_eq!(Renderer::bloom_size(4, 4), (2, 2));
        // Real-world odd window: 1101×733 → 550×366 (truncated, not floored).
        assert_eq!(Renderer::bloom_size(1101, 733), (550, 366));
        // Width and height are independent (no swap).
        assert_eq!(Renderer::bloom_size(1920, 1080), (960, 540));
    }
}
