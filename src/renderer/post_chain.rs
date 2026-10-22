//! ARC-023: one post-processing encoder shared by the interactive surface
//! path and both capture paths (native hi-res, web).
//!
//! Previously App::render, capture.rs and capture_web.rs each hand-encoded
//! the same pass sequence and drifted: exports skipped FXAA, ran bloom at
//! full width while the screen ran it at half (ENH-005), and composited in
//! accumulation mode where the screen copies directly. [`Renderer::
//! encode_post_chain`] is now the single sequence; callers only supply the
//! target-sized bind groups/views.

use super::Renderer;

/// The per-target bind groups and views the post chain reads and writes.
/// The interactive path borrows the renderer's own (surface-sized) set via
/// [`Renderer::surface_post_targets`]; capture paths build a capture-sized
/// set with the same layout.
pub struct PostChainTargets<'a> {
    /// Reads the scene texture (bloom extract; accumulation copy-out).
    pub scene_bind_group: &'a wgpu::BindGroup,
    pub bright_view: &'a wgpu::TextureView,
    pub blur_temp_view: &'a wgpu::TextureView,
    pub bloom_view: &'a wgpu::TextureView,
    pub composite_view: &'a wgpu::TextureView,
    /// Reads the bright pass (blur H).
    pub bright_bind_group: &'a wgpu::BindGroup,
    /// Reads the blur-temp texture (blur V).
    pub blur_temp_bind_group: &'a wgpu::BindGroup,
    /// Composite's scene+bloom input.
    pub composite_bind_group: &'a wgpu::BindGroup,
    /// Final pass input (reads composite).
    pub composite_final_bind_group: &'a wgpu::BindGroup,
}

/// Per-frame toggles for the post chain.
#[derive(Debug, Clone, Copy)]
pub struct PostOptions {
    /// Run the three bloom passes (extract + H + V). Mirrors
    /// `settings.bloom_enabled`; accumulation mode forces it off.
    pub bloom_enabled: bool,
    /// Final pass uses FXAA instead of a direct copy. Capture paths now set
    /// this from the same setting as the screen (ARC-023 criterion: exports
    /// match the on-screen render).
    pub fxaa_enabled: bool,
    /// Accumulation mode: skip bloom + composite, copy scene straight to the
    /// output (what the screen does; exports previously composited anyway).
    pub use_accumulation: bool,
    /// Whether the bloom output texture is known-clear (the interactive
    /// path's one-shot-clear bookkeeping). Capture targets are freshly
    /// created, so callers pass `true` there.
    pub bloom_already_cleared: bool,
}

impl Renderer {
    /// The renderer's own surface-sized target set.
    pub(crate) fn surface_post_targets(&self) -> PostChainTargets<'_> {
        PostChainTargets {
            scene_bind_group: &self.scene_bind_group,
            bright_view: &self.bright_view,
            blur_temp_view: &self.blur_temp_view,
            bloom_view: &self.bloom_view,
            composite_view: &self.composite_view,
            bright_bind_group: &self.bright_bind_group,
            blur_temp_bind_group: &self.blur_temp_bind_group,
            composite_bind_group: &self.composite_bind_group,
            composite_final_bind_group: &self.composite_final_bind_group,
        }
    }

    /// Encode bloom (extract → blur H → blur V), composite, and the final
    /// FXAA/copy pass into `output`. The parameters bind groups
    /// (bloom/blur directions/composite grading) are always the renderer's
    /// own — the blur shader derives its texel size from
    /// `textureDimensions`, so one param set serves any target size.
    pub(crate) fn encode_post_chain(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        t: &PostChainTargets<'_>,
        output: &wgpu::TextureView,
        opts: &PostOptions,
    ) {
        let bloom_active = opts.bloom_enabled && !opts.use_accumulation;
        if bloom_active {
            // Extract bright pixels.
            {
                let mut pass = encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Bloom Extract Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: t.bright_view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: self.profiler.pass_writes("bloom_extract"),
                        multiview_mask: None,
                    })
                    .forget_lifetime();
                pass.set_pipeline(&self.bloom_extract_pipeline);
                pass.set_bind_group(0, t.scene_bind_group, &[]);
                pass.set_bind_group(1, &self.bloom_params_bind_group, &[]);
                pass.set_vertex_buffer(0, self.postprocess_vertex_buffer.slice(..));
                pass.draw(0..4, 0..1);
            }

            // Blur horizontally into the temp target.
            {
                let mut pass = encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Blur Horizontal Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: t.blur_temp_view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: self.profiler.pass_writes("bloom_h"),
                        multiview_mask: None,
                    })
                    .forget_lifetime();
                pass.set_pipeline(&self.blur_pipeline);
                pass.set_bind_group(0, t.bright_bind_group, &[]);
                pass.set_bind_group(1, &self.blur_h_params_bind_group, &[]);
                pass.set_vertex_buffer(0, self.postprocess_vertex_buffer.slice(..));
                pass.draw(0..4, 0..1);
            }

            // Blur vertically into the bloom output (its bind group carries
            // the (0,1) direction buffer — ARC-022).
            {
                let mut pass = encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Blur Vertical Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: t.bloom_view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: self.profiler.pass_writes("bloom_v"),
                        multiview_mask: None,
                    })
                    .forget_lifetime();
                pass.set_pipeline(&self.blur_pipeline);
                pass.set_bind_group(0, t.blur_temp_bind_group, &[]);
                pass.set_bind_group(1, &self.blur_v_params_bind_group, &[]);
                pass.set_vertex_buffer(0, self.postprocess_vertex_buffer.slice(..));
                pass.draw(0..4, 0..1);
            }
        } else if !opts.use_accumulation && !opts.bloom_already_cleared {
            // Composite samples `bloom_view` even when bloom is disabled, so
            // the texture must hold defined memory. One cheap clear pass.
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Bloom Output One-Shot Clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: t.bloom_view,
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
        }

        // Composite (scene + bloom + grading + vignette) — skipped in
        // accumulation mode, exactly like the screen path.
        if !opts.use_accumulation {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Composite Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: t.composite_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: self.profiler.pass_writes("composite"),
                    multiview_mask: None,
                })
                .forget_lifetime();
            pass.set_pipeline(&self.composite_pipeline);
            pass.set_bind_group(0, t.composite_bind_group, &[]);
            pass.set_bind_group(1, &self.composite_params_bind_group, &[]);
            pass.set_vertex_buffer(0, self.postprocess_vertex_buffer.slice(..));
            pass.draw(0..4, 0..1);
        }

        // Final pass: FXAA when enabled, direct copy otherwise; accumulation
        // copies the scene straight out (bloom/composite never ran).
        let mut pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Final Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: self.profiler.pass_writes("fxaa"),
                multiview_mask: None,
            })
            .forget_lifetime();

        if opts.use_accumulation {
            pass.set_pipeline(&self.copy_pipeline);
            pass.set_bind_group(0, t.scene_bind_group, &[]);
        } else if opts.fxaa_enabled {
            pass.set_pipeline(&self.fxaa_pipeline);
            pass.set_bind_group(0, t.composite_final_bind_group, &[]);
        } else {
            pass.set_pipeline(&self.copy_pipeline);
            pass.set_bind_group(0, t.composite_final_bind_group, &[]);
        }

        pass.set_vertex_buffer(0, self.postprocess_vertex_buffer.slice(..));
        pass.draw(0..4, 0..1);
    }
}
