use super::App;
use crate::fractal::FractalParams;
use crate::renderer::compute::{
    AccumulationDisplayUniforms, AttractorComputeUniforms, BuddhabrotComputeUniforms,
};
use crate::ui::UiActions;

#[cfg(not(target_arch = "wasm32"))]
use crate::video_recorder::VideoRecorder;

/// Render methods.
///
/// ARC-004: the original ~900-line `App::render` was a God method mixing six
/// responsibilities. It is now an orchestration shell that delegates to four
/// behavior-preserving extractions:
///
/// - `dispatch_accumulation` — attractor/Buddhabrot compute + display pass.
/// - `run_post_chain` — bloom/blur/composite/FXAA passes (bloom-gated per
///   ARC-005; one-shot clear of the bloom target on enabled→disabled
///   transitions).
/// - `render_ui` — the egui frame (UI panels, command palette, overlays) and
///   the egui render pass into the frame's encoder. Returns the `UiActions`
///   built inside the egui closure plus a flag telling the caller whether any
///   UI action mutated scene state (so it can mark the scene dirty).
/// - `handle_ui_actions` — preset/bookmark/reset/recorder/etc. state side
///   effects, plus the non-blocking GPU scan kickoff (ARC-018).
///
/// What stays inline in `render`: surface acquisition, encoder creation, the
/// non-accumulation scene render pass (a single 25-line block), the
/// screenshot/video capture block (which forks the encoder mid-function —
/// moving it would risk reordering `queue.submit` and break capture
/// correctness), and the final submit/present.
impl App {
    pub fn render(&mut self) {
        let output = match self.renderer.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                // Reconfigure surface for next frame, but still present the current one
                self.renderer
                    .surface
                    .configure(&self.renderer.device, &self.renderer.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                // Skip this frame
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                // Reconfigure the surface and try again next frame
                self.renderer
                    .surface
                    .configure(&self.renderer.device, &self.renderer.config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                log::error!("Surface get_current_texture returned a validation error");
                return;
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // Check if we should use accumulation mode for strange attractors or Buddhabrot
        let is_attractor = self.fractal_params.settings.fractal_type.is_2d_attractor();
        let is_buddhabrot = self.fractal_params.settings.fractal_type.is_buddhabrot();
        let use_accumulation = self.fractal_params.settings.attractor_accumulation_enabled
            && (is_attractor || is_buddhabrot);

        // Pass 1: fractal pass (accumulation compute chain OR the standard scene render).
        if use_accumulation {
            self.dispatch_accumulation(&mut encoder);
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
                    multiview_mask: None,
                });

                let mut render_pass = render_pass.forget_lifetime();

                // ARC-009: select 2D vs 3D pipeline by fractal type. Both
                // pipelines share one layout and uniform bind group.
                let pipeline = if self.fractal_params.settings.fractal_type.is_3d() {
                    &self.renderer.pipeline_3d
                } else {
                    &self.renderer.pipeline_2d
                };
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(0, &self.renderer.uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.renderer.vertex_buffer.slice(..));
                render_pass.draw(0..4, 0..1);
            }
        }

        // Passes 2-6: bloom (gated), composite, FXAA/final.
        self.run_post_chain(&mut encoder, &view, use_accumulation);

        // If screenshot requested or recording, capture fractal before UI is rendered.
        // ARC-004: this block stays inline because it forks the encoder
        // mid-function (submits the fractal passes, then opens a fresh
        // encoder for the UI). Moving it would risk reordering `queue.submit`
        // and breaking screenshot/video correctness.
        let should_screenshot = self.save_screenshot;
        #[cfg(not(target_arch = "wasm32"))]
        let is_recording = self.video_recorder.is_recording();
        #[cfg(target_arch = "wasm32")]
        let is_recording = false; // Video recording not supported on web

        let mut encoder = if should_screenshot || is_recording {
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
                        .settings
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
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("UI Render Encoder"),
                })
        } else {
            encoder
        };

        // Render UI into `view` (encodes the egui draw lists into `encoder`),
        // returning the actions the user triggered and whether any of them
        // mutated scene state. Then apply the side effects.
        let (actions, ui_mutated_scene) = self.render_ui(&mut encoder, &view);
        self.handle_ui_actions(actions);

        // ARC-006: a UI action mutated scene state (slider edit, preset load,
        // bookmark, reset, screenshot, hi-res render, recording toggle, or
        // command-palette action). Mark dirty so the next frame renders and
        // (for non-animating changes) the loop returns to idle after that one
        // frame. `render_ui` computes the flag instead of marking dirty
        // in-place because the egui closure holds an immutable borrow on
        // `self.egui_state` for the duration of `run_ui`.
        if ui_mutated_scene {
            self.mark_scene_dirty();
        }

        self.renderer
            .queue
            .submit(std::iter::once(encoder.finish()));

        output.present();

        // ARC-006: we just rendered a frame. Clear the dirty flag unless a
        // continuous animation source (auto-orbit, palette animation, camera
        // transition, LOD interpolation, attractor accumulation, video
        // recording) is still active — those keep the loop spinning by
        // returning `is_scene_animation_active() == true` from
        // `should_render_next_frame()`.
        self.after_render_frame();
    }

    /// Dispatch the attractor/Buddhabrot accumulation compute passes and the
    /// "Accumulation Display Pass" that blits the result to `scene_view`.
    ///
    /// Extracted verbatim from the original `App::render`; behavior unchanged.
    /// The caller has already decided `use_accumulation` is true and created
    /// the encoder; this method encodes into that same encoder (the clear
    /// must precede the dispatch in the same command stream — see ARC-012).
    fn dispatch_accumulation(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let is_buddhabrot = self.fractal_params.settings.fractal_type.is_buddhabrot();

        // Check if texture needs recreation (None or wrong size)
        let texture_needs_recreation = match &self.renderer.accumulation_texture {
            None => true,
            Some(tex) => {
                tex.width != self.renderer.size.width || tex.height != self.renderer.size.height
            }
        };

        // Initialize compute infrastructure if needed (handles resize too)
        if is_buddhabrot {
            self.renderer.init_buddhabrot_compute();
        } else {
            self.renderer.init_accumulation_compute();
        }

        // Reset iteration counter if texture was just recreated
        if texture_needs_recreation {
            self.fractal_params.accum.total_iterations = 0;
        }

        // Auto-clear when view parameters change (zoom, pan, or attractor params)
        let view_changed = self.fractal_params.settings.center_2d
            != self.fractal_params.accum.last_center
            || self.fractal_params.settings.zoom_2d != self.fractal_params.accum.last_zoom
            || (!is_buddhabrot
                && self.fractal_params.settings.julia_c != self.fractal_params.accum.last_julia_c);

        if view_changed {
            self.fractal_params.accum.pending_clear = true;
            self.fractal_params.accum.total_iterations = 0;
            self.fractal_params.accum.paused = false; // Resume accumulation on view change
            // Update last values
            self.fractal_params.accum.last_center = self.fractal_params.settings.center_2d;
            self.fractal_params.accum.last_zoom = self.fractal_params.settings.zoom_2d;
            self.fractal_params.accum.last_julia_c = self.fractal_params.settings.julia_c;
        }

        // Handle clear request
        if self.fractal_params.accum.pending_clear {
            if is_buddhabrot {
                // Clear Buddhabrot buffer
                if let Some(ref buffer) = self.renderer.buddhabrot_accumulation_buffer {
                    buffer.clear(encoder);
                }
            } else {
                // Clear attractor texture
                if let Some(ref mut accum_tex) = self.renderer.accumulation_texture {
                    accum_tex.clear(
                        encoder,
                        &self.renderer.device,
                        self.renderer.clear_texture_supported,
                    );
                }
            }
            self.fractal_params.accum.pending_clear = false;
            self.fractal_params.accum.total_iterations = 0;
        }

        // Dispatch appropriate compute shader based on fractal type (only if not paused)
        if !self.fractal_params.accum.paused && is_buddhabrot {
            // Update Buddhabrot compute uniforms
            if let Some(ref mut compute) = self.renderer.buddhabrot_compute {
                // Filter trajectories by minimum iteration count
                // Short trajectories (outer glow) vs long trajectories (Buddha interior)
                // Higher min = more Buddha detail, lower = more outer structure
                let min_iter = (self.fractal_params.settings.max_iterations / 10).max(20);
                compute.uniforms = BuddhabrotComputeUniforms {
                    center_x: self.fractal_params.settings.center_2d[0] as f32,
                    center_y: self.fractal_params.settings.center_2d[1] as f32,
                    zoom: self.fractal_params.settings.zoom_2d as f32,
                    aspect_ratio: self.renderer.size.width as f32
                        / self.renderer.size.height as f32,
                    width: self.renderer.size.width,
                    height: self.renderer.size.height,
                    iterations_per_frame: self
                        .fractal_params
                        .settings
                        .attractor_iterations_per_frame,
                    max_iterations: self.fractal_params.settings.max_iterations,
                    // QA-022: deliberate u64→u32 wrap. The shader reads this
                    // purely as an RNG stream offset (it XORs the value into
                    // the per-pixel seed; see `buddhabrot_compute.wgsl`).
                    // Auto-pause / progress accounting reads the original
                    // u64 CPU-side (`accum.total_iterations` below), so the
                    // wrap does not lose progress information. Masking keeps
                    // the truncation explicit instead of relying on `as`
                    // semantics.
                    total_iterations: (self.fractal_params.accum.total_iterations & 0xFFFF_FFFF)
                        as u32,
                    clear_accumulation: 0,
                    min_iterations: min_iter,
                    _padding: 0,
                };
                compute.update_uniforms(&self.renderer.queue);

                // Dispatch compute shader using the atomic buffer
                if let Some(ref buffer) = self.renderer.buddhabrot_accumulation_buffer {
                    // Each workgroup (256 threads) tests multiple samples
                    let num_workgroups =
                        (self.fractal_params.settings.attractor_iterations_per_frame / 256).max(1);
                    compute.dispatch(encoder, &buffer.compute_bind_group, num_workgroups);

                    // Copy from atomic buffer to texture for display
                    let has_copy_pipeline = self.renderer.buddhabrot_copy_pipeline.is_some();
                    let has_copy_bind_group = self.renderer.buddhabrot_copy_bind_group.is_some();

                    if has_copy_pipeline && has_copy_bind_group {
                        let copy_pipeline =
                            self.renderer.buddhabrot_copy_pipeline.as_ref().unwrap();
                        let copy_bind_group =
                            self.renderer.buddhabrot_copy_bind_group.as_ref().unwrap();
                        let mut copy_pass =
                            encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                label: Some("Buddhabot Copy Pass"),
                                timestamp_writes: None,
                            });
                        copy_pass.set_pipeline(copy_pipeline);
                        copy_pass.set_bind_group(0, copy_bind_group, &[]);
                        // Dispatch enough workgroups to cover all pixels (16x16 workgroup size)
                        let wg_x = self.renderer.size.width.div_ceil(16);
                        let wg_y = self.renderer.size.height.div_ceil(16);
                        copy_pass.dispatch_workgroups(wg_x, wg_y, 1);
                    }
                }

                // Update total iterations counter
                self.fractal_params.accum.total_iterations +=
                    self.fractal_params.settings.attractor_iterations_per_frame as u64;

                // Auto-pause when max iterations reached
                if self.fractal_params.accum.total_iterations
                    >= self.fractal_params.accum.max_iterations
                {
                    self.fractal_params.accum.paused = true;
                }
            }
        } else if !self.fractal_params.accum.paused {
            // Update attractor compute uniforms (only if not paused)
            if let Some(ref mut compute) = self.renderer.attractor_compute {
                compute.uniforms = AttractorComputeUniforms {
                    param_a: self.fractal_params.settings.julia_c[0],
                    param_b: self.fractal_params.settings.julia_c[1],
                    param_c: 0.0, // Could expose more params
                    param_d: 0.0,
                    center_x: self.fractal_params.settings.center_2d[0] as f32,
                    center_y: self.fractal_params.settings.center_2d[1] as f32,
                    zoom: self.fractal_params.settings.zoom_2d as f32,
                    aspect_ratio: self.renderer.size.width as f32
                        / self.renderer.size.height as f32,
                    width: self.renderer.size.width,
                    height: self.renderer.size.height,
                    iterations_per_frame: self
                        .fractal_params
                        .settings
                        .attractor_iterations_per_frame,
                    attractor_type: self.fractal_params.settings.fractal_type.attractor_index(),
                    // QA-022: deliberate u64→u32 wrap; see the matching note in
                    // the Buddhabrot branch above. `attractor_compute.wgsl`
                    // uses this solely as an RNG seed offset, not for progress.
                    total_iterations: (self.fractal_params.accum.total_iterations & 0xFFFF_FFFF)
                        as u32,
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
                        (self.fractal_params.settings.attractor_iterations_per_frame / 256).max(1);
                    compute.dispatch(encoder, &accum_tex.compute_bind_group, num_workgroups);
                }

                // Update total iterations counter
                self.fractal_params.accum.total_iterations +=
                    self.fractal_params.settings.attractor_iterations_per_frame as u64;

                // Auto-pause when max iterations reached
                if self.fractal_params.accum.total_iterations
                    >= self.fractal_params.accum.max_iterations
                {
                    self.fractal_params.accum.paused = true;
                }
            }
        }

        // Update accumulation display uniforms with palette from fractal params.
        // QA-024: replaced a 50-line hand-unrolled 8-element copy with
        // `array::from_fn`. Each slot is `[r, g, b, 1.0]` from the matching
        // `palette.colors[i]` Vec3 — identical transform to the literal it
        // replaced.
        let palette_colors = self.fractal_params.settings.palette.colors;
        let display_uniforms = AccumulationDisplayUniforms {
            log_scale: self.fractal_params.settings.attractor_log_scale,
            gamma: 0.6,
            palette_offset: self.fractal_params.settings.palette_offset,
            _padding: 0.0,
            palette: std::array::from_fn(|i| {
                let c = palette_colors[i];
                [c.x, c.y, c.z, 1.0]
            }),
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
                multiview_mask: None,
            });

            let mut render_pass = render_pass.forget_lifetime();

            // Use the accumulation display pipeline with log scaling visualization
            if let Some(ref bind_group) = self.renderer.accumulation_display_bind_group {
                render_pass.set_pipeline(&self.renderer.accumulation_display_pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);
                render_pass.set_bind_group(
                    1,
                    &self.renderer.accumulation_display_uniform_bind_group,
                    &[],
                );
                render_pass.set_vertex_buffer(0, self.renderer.postprocess_vertex_buffer.slice(..));
                render_pass.draw(0..4, 0..1);
            }
        }
    }

    /// Encode the post-processing chain: bloom extract + horizontal blur +
    /// vertical blur (gated on `bloom_enabled` per ARC-005; with a one-shot
    /// clear of the bloom target on enabled→disabled transitions so the
    /// composite pass never samples stale memory), the composite pass
    /// (skipped in accumulation mode), and the final pass to the frame's
    /// `view` (FXAA when enabled, else a direct copy).
    ///
    /// Extracted verbatim from the original `App::render`; behavior unchanged.
    fn run_post_chain(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        use_accumulation: bool,
    ) {
        // Pass 2-4: Bloom pipeline.
        // ARC-005: the three full-res Rgba16Float passes are pure waste when
        // bloom is disabled (the default) or when accumulation mode skips the
        // composite pass entirely. Gate them on `bloom_enabled && !use_accumulation`.
        let bloom_active = self.fractal_params.settings.bloom_enabled && !use_accumulation;
        if bloom_active {
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
                    multiview_mask: None,
                });

                let mut render_pass = render_pass.forget_lifetime();

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
                    multiview_mask: None,
                });

                let mut render_pass = render_pass.forget_lifetime();

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
                    multiview_mask: None,
                });

                let mut render_pass = render_pass.forget_lifetime();

                render_pass.set_pipeline(&self.renderer.blur_pipeline);
                render_pass.set_bind_group(0, &self.renderer.blur_temp_bind_group, &[]);
                render_pass.set_bind_group(1, &self.renderer.blur_v_params_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.renderer.postprocess_vertex_buffer.slice(..));
                render_pass.draw(0..4, 0..1);
            }

            // The three passes overwrote `bloom_view` with valid (possibly-black)
            // contents; the next clean-frame skip can rely on that.
            self.bloom_texture_cleared = false;
        } else if !use_accumulation && !self.bloom_texture_cleared {
            // Composite samples `bloom_view` regardless of `bloom_enabled` (the
            // shader-side toggle multiplies the contribution but still reads the
            // texture). After an enabled→disabled transition — or before the
            // first bloom frame — the texture would otherwise hold stale or
            // undefined memory on some backends, leaking garbage pixels into
            // the composite. Record ONE cheap clear with no draw, then mark the
            // texture as cleared so we don't repeat this on every idle frame.
            // (Accumulation mode skips composite entirely, so it never samples
            // `bloom_view`; we still mark it cleared so the first post-accum
            // composite frame doesn't sample uninitialized memory.)
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Bloom Output One-Shot Clear"),
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
                multiview_mask: None,
            });
            self.bloom_texture_cleared = true;
        }

        // Pass 5: Composite (scene + bloom + color grading + vignette)
        // For accumulation mode (attractors/Buddhabrot), skip composite since bloom wasn't rendered
        if !use_accumulation {
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
                multiview_mask: None,
            });

            let mut render_pass = render_pass.forget_lifetime();

            render_pass.set_pipeline(&self.renderer.composite_pipeline);
            render_pass.set_bind_group(0, &self.renderer.composite_bind_group, &[]);
            render_pass.set_bind_group(1, &self.renderer.composite_params_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.renderer.postprocess_vertex_buffer.slice(..));
            render_pass.draw(0..4, 0..1);
        }

        // Pass 6: Final output to screen
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Final Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
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
                multiview_mask: None,
            });

            let mut render_pass = render_pass.forget_lifetime();

            if use_accumulation {
                // For accumulation mode, copy directly from scene to screen (skip composite/bloom)
                render_pass.set_pipeline(&self.renderer.copy_pipeline);
                render_pass.set_bind_group(0, &self.renderer.scene_bind_group, &[]);
            } else if self.fractal_params.settings.fxaa_enabled {
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
    }

    /// Render the egui frame (UI panels, command palette, overlays) and encode
    /// the egui draw lists into `encoder` against `view`. Returns the actions
    /// the user triggered this frame plus a flag telling the caller whether
    /// any of them mutated scene state (so the caller can mark the scene
    /// dirty outside the egui closure, which holds an immutable borrow on
    /// `self.egui_state`).
    ///
    /// ARC-004: extracted verbatim from `App::render`; behavior unchanged.
    /// The `gpu_scan_requested` action is included in `ui_mutated_scene` so a
    /// scan request triggers a redraw (the "Scanning…" label needs to paint).
    fn render_ui(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> (UiActions, bool) {
        let raw_input = self.egui_state.take_egui_input(self.window.as_ref());
        // ARC-006: populated inside the UI closure; consulted after `run_ui`
        // returns (we can't call `self.mark_scene_dirty()` inside the closure
        // — `egui_ctx()` holds an immutable borrow on `self.egui_state`).
        let mut ui_mutated_scene = false;
        let mut actions = UiActions::default();
        let full_output = self.egui_state.egui_ctx().run_ui(raw_input, |ctx| {
            #[cfg(not(target_arch = "wasm32"))]
            let is_rec = self.video_recorder.is_recording();
            #[cfg(target_arch = "wasm32")]
            let is_rec = false; // Video recording not supported on web

            actions = self.ui.render(
                ctx,
                &mut self.fractal_params,
                self.camera.position,
                self.camera.target,
                is_rec,
            );

            // ARC-006: snapshot which UI actions mutate scene state BEFORE the
            // handlers consume the Options, so we can mark the scene dirty in
            // one place after the closure. UI `changed` covers slider/checkbox
            // edits; the rest are explicit action flags. (Command palette's
            // own `changed` is OR'd in below.)
            let UiActions {
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
            } = &actions;
            ui_mutated_scene = *changed
                || preset_to_load.is_some()
                || bookmark_to_load.is_some()
                || *reset_requested
                || *reset_camera_requested
                || *point_at_fractal_requested
                || *screenshot_requested
                || hires_render_resolution.is_some()
                || *gpu_scan_requested
                || *start_recording
                || *stop_recording;

            // Render command palette overlay (always on top). Stays inside the
            // closure because it needs `ctx`.
            if let Some(command_action) = self.ui.render_command_palette(ctx) {
                let (cp_changed, message) = self
                    .ui
                    .execute_command(command_action, &mut self.fractal_params);

                ui_mutated_scene |= cp_changed;

                if cp_changed {
                    self.settings_last_changed = web_time::Instant::now();
                    self.settings_need_save = true;
                }

                if let Some(msg) = message {
                    self.ui.show_toast(msg);
                }
            }

            self.ui.render_fps(ctx, self.current_fps);
            self.ui.render_camera_info(
                ctx,
                self.camera.position,
                self.camera.target,
                &self.fractal_params.lod.lod_config.distance_zones,
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
            encoder,
            &tris,
            &screen_descriptor,
        );

        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
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
                multiview_mask: None,
            });

            let mut render_pass = render_pass.forget_lifetime();

            self.egui_renderer
                .render(&mut render_pass, &tris, &screen_descriptor);
            drop(render_pass);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        (actions, ui_mutated_scene)
    }

    /// Apply the side effects of the UI actions returned from `render_ui`:
    /// preset/bookmark loading, parameter and camera resets, screenshot and
    /// hi-res render kickoff, video recorder lifecycle, settings auto-save
    /// flagging, and the GPU rescan request (ARC-018: native spawns a worker
    /// thread + returns immediately; the result is drained by
    /// `App::poll_gpu_scan` each frame).
    ///
    /// ARC-004 interim placement: this is called immediately after `render_ui`
    /// inside `render()`, NOT from `App::update`. The original code ran these
    /// handlers inside the egui closure at the end of `render()`, so this
    /// placement preserves behavior exactly (same-frame execution). Moving
    /// them to `update()` would apply actions one frame sooner — safe in
    /// principle since screenshot/recording flags are consumed at the top of
    /// the NEXT render — but it is a behavior change and out of scope for
    /// this behavior-preserving extraction.
    ///
    /// Actions that MUST affect the current frame stay here (none of them
    /// do: `save_screenshot` and the recorder state are read at the top of
    /// the next render call, not this one).
    fn handle_ui_actions(&mut self, actions: UiActions) {
        let UiActions {
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
        } = actions;

        // Handle GPU scan request / Monitor scan request
        if gpu_scan_requested {
            // Scan monitors (always do this when the button is clicked)
            self.ui.scan_monitors(&self.window);

            // ARC-018: kick off GPU enumeration off the render path. Native
            // spawns a worker thread (with a fresh `wgpu::Instance` inside
            // `Renderer::enumerate_gpus`) that sends the result through a
            // channel; `App::poll_gpu_scan` (called from `update`) drains it
            // via `try_recv` next frame, so the render loop never blocks on
            // adapter enumeration (which can take hundreds of ms).
            #[cfg(not(target_arch = "wasm32"))]
            {
                if self.gpu_scan_receiver.is_none() {
                    let (tx, rx) = std::sync::mpsc::channel::<Vec<crate::renderer::GpuInfo>>();
                    self.gpu_scan_receiver = Some(rx);
                    std::thread::spawn(move || {
                        let gpus = pollster::block_on(crate::renderer::Renderer::enumerate_gpus());
                        // Ignore send errors: if the receiver was dropped (App
                        // dropped), the result is just unused.
                        let _ = tx.send(gpus);
                    });
                    self.ui.gpu_selection_message = Some("Scanning for GPUs...".to_string());
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                self.ui.gpu_selection_message =
                    Some("GPU selection not available on web".to_string());
            }
        }

        // Handle preset loading
        if let Some(preset) = preset_to_load {
            log::info!("Loading preset: {}", preset.name);
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
            log::info!("Loading camera bookmark: {}", bookmark.name);
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
            self.fractal_params.settings.camera_fov = bookmark.fov;
        }

        if reset_requested {
            self.fractal_params = FractalParams::default();
            // Reset camera to default position and settings
            self.camera.reset_to_default();
            self.camera.fovy = self.fractal_params.settings.camera_fov;
            self.camera_controller
                .set_speed(self.fractal_params.settings.camera_speed);
            // Sync controller with reset camera position
            self.camera_controller
                .point_at_target(self.camera.position, self.camera.target);
            log::info!("Settings and camera reset to defaults");
        }

        if reset_camera_requested {
            self.camera.reset_to_default();
            self.camera.fovy = self.fractal_params.settings.camera_fov;
            // Sync controller with reset camera position
            self.camera_controller
                .point_at_target(self.camera.position, self.camera.target);
            log::info!("Camera reset to default position");
        }

        if point_at_fractal_requested {
            self.camera_controller
                .point_at_target(self.camera.position, glam::Vec3::ZERO);
            log::info!("Camera pointed at fractal");
        }

        if screenshot_requested {
            self.save_screenshot = true;
        }

        if let Some(resolution) = hires_render_resolution {
            self.save_hires_render = Some(resolution);
            log::info!(
                "High-resolution render requested: {}x{}",
                resolution.0,
                resolution.1
            );
        }

        // Handle video recording (native only)
        #[cfg(not(target_arch = "wasm32"))]
        {
            if start_recording {
                // Generate filename with fractal type and timestamp
                let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                let fractal_name = self
                    .fractal_params
                    .settings
                    .fractal_type
                    .filename_safe_name();
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
                    log::error!("Failed to start recording: {}", e);
                } else {
                    log::info!("Started recording to {}", filename);
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
                        if self.ui.auto_open_captures
                            && let Err(e) = open::that(&abs_path)
                        {
                            log::error!("Failed to open video: {}", e);
                        }

                        self.ui.show_toast_with_file(
                            format!("🎬 Video saved: {} - Click to open", filename),
                            abs_path.to_string_lossy().to_string(),
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to stop recording: {}", e);
                    }
                }
            }
        }
        // Video recording not supported on web - UI section is hidden via cfg

        // Mark settings for auto-save (debounced) and sync camera parameters
        // from fractal_params (fovy / camera_speed sliders).
        if changed {
            self.settings_last_changed = web_time::Instant::now();
            self.settings_need_save = true;

            // Update camera parameters from fractal_params
            self.camera.fovy = self.fractal_params.settings.camera_fov;
            self.camera_controller
                .set_speed(self.fractal_params.settings.camera_speed);
        }
    }
}
