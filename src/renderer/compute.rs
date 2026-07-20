//! Compute shader infrastructure for texture-based effects and simulations.
//!
//! This module provides a modular system for GPU compute operations including:
//! - Accumulation textures for iterative effects (strange attractors, particle systems)
//! - Storage buffer management for compute data
//! - Flexible compute pipeline creation
//!
//! # Design
//!
//! The system is designed to be reusable for various texture-based effects:
//! - Strange attractor density accumulation
//! - Particle simulations
//! - Image processing pipelines
//! - Reaction-diffusion systems
//!
//! # Status
//!
//! ARC-016: this module IS integrated with the main renderer. The attractor and
//! Buddhabot compute pipelines (`AttractorComputePipeline`, `BuddhabotComputePipeline`)
//! are owned by `Renderer` and dispatched every accumulation frame from
//! `app/render.rs::dispatch_accumulation` when `attractor_accumulation_enabled`
//! is true and the active fractal is a 2D attractor or Buddhabrot. The four
//! compute/copy shaders live in `src/shaders/` (`attractor_compute.wgsl`,
//! `attractor_display.wgsl`, `buddhabot_compute.wgsl`, `buddhabot_copy.wgsl`).

use super::uniforms::write_uniform_bytes;
use encase::ShaderType;
use wgpu::util::DeviceExt;

/// Uniforms for the accumulation display shader
#[derive(Debug, Clone, Copy, encase::ShaderType)]
pub struct AccumulationDisplayUniforms {
    pub log_scale: f32,
    pub gamma: f32,
    pub palette_offset: f32,
    /// 8 palette colors; maps to the WGSL `array<vec4<f32>, 8>`.
    pub palette: [glam::Vec4; 8],
}

impl Default for AccumulationDisplayUniforms {
    fn default() -> Self {
        Self {
            log_scale: 1.0,
            gamma: 0.6,
            palette_offset: 0.0,
            // Default fire palette (8 colors)
            palette: [
                glam::Vec4::new(0.0, 0.0, 0.0, 1.0),   // Black
                glam::Vec4::new(0.25, 0.0, 0.25, 1.0), // Deep purple
                glam::Vec4::new(0.5, 0.0, 0.5, 1.0),   // Purple
                glam::Vec4::new(0.75, 0.0, 0.25, 1.0), // Magenta
                glam::Vec4::new(1.0, 0.0, 0.0, 1.0),   // Red
                glam::Vec4::new(1.0, 0.5, 0.0, 1.0),   // Orange
                glam::Vec4::new(1.0, 0.75, 0.0, 1.0),  // Light orange
                glam::Vec4::new(1.0, 1.0, 0.0, 1.0),   // Yellow
            ],
        }
    }
}

/// Uniforms for the attractor compute shader
#[derive(Debug, Clone, Copy, encase::ShaderType)]
pub struct AttractorComputeUniforms {
    // Attractor parameters (from julia_c, power, etc.)
    pub param_a: f32,
    pub param_b: f32,
    pub param_c: f32,
    pub param_d: f32,

    // View transform
    pub center_x: f32,
    pub center_y: f32,
    pub zoom: f32,
    pub aspect_ratio: f32,

    // Rendering parameters
    pub width: u32,
    pub height: u32,
    pub iterations_per_frame: u32,
    pub attractor_type: u32,

    // Accumulation control
    pub total_iterations: u32,
    pub clear_accumulation: u32,
}

/// Uniforms for the Buddhabrot compute shader
#[derive(Debug, Clone, Copy, encase::ShaderType)]
pub struct BuddhabrotComputeUniforms {
    // View transform
    pub center_x: f32,
    pub center_y: f32,
    pub zoom: f32,
    pub aspect_ratio: f32,

    // Rendering parameters
    pub width: u32,
    pub height: u32,
    pub iterations_per_frame: u32,
    pub max_iterations: u32,

    // Accumulation control
    pub total_iterations: u32,
    pub clear_accumulation: u32,
    pub min_iterations: u32, // Minimum iterations for trajectory to be plotted
}

impl Default for BuddhabrotComputeUniforms {
    fn default() -> Self {
        Self {
            center_x: -0.4,
            center_y: 0.0,
            zoom: 0.4,
            aspect_ratio: 16.0 / 9.0,
            width: 1920,
            height: 1080,
            iterations_per_frame: 20_000,
            max_iterations: 2000,
            total_iterations: 0,
            clear_accumulation: 1,
            min_iterations: 20, // Filter out short trajectories
        }
    }
}

impl Default for AttractorComputeUniforms {
    fn default() -> Self {
        Self {
            param_a: 0.4,
            param_b: 1.0,
            param_c: 0.0,
            param_d: 0.0,
            center_x: 0.0,
            center_y: 0.0,
            zoom: 1.0,
            aspect_ratio: 16.0 / 9.0,
            width: 1920,
            height: 1080,
            iterations_per_frame: 100_000,
            attractor_type: 0, // Hopalong
            total_iterations: 0,
            clear_accumulation: 1,
        }
    }
}

/// Which bind-group layout an [`AccumulationTexture`]'s `compute_bind_group`
/// was built against. The Buddhabrot and attractor compute paths bind
/// different resource types at binding 0 (a storage *buffer* vs a storage
/// *texture*), so they use incompatible layouts. This tag lets the attractor
/// path detect a Buddhabrot-created placeholder and rebuild the texture before
/// dispatching — the Buddhabrot → attractor switch used to dispatch the
/// Attractor pipeline with the Buddhabrot buffer-layout bind group, failing
/// GPU validation every frame (the reported "lockup").
pub enum AccumulationBindGroupKind {
    /// Real StorageTexture bind group against the attractor compute layout.
    AttractorTexture,
    /// Placeholder built against the Buddhabrot *buffer* layout. Buddhabrot
    /// computes against its atomic buffer and never uses this bind group for
    /// dispatch — but it is incompatible with the Attractor pipeline, so a
    /// texture carrying it must be rebuilt before an attractor dispatch.
    BuddhabrotPlaceholder,
}

/// Manages an accumulation texture for iterative rendering effects.
///
/// This abstraction handles:
/// - Storage texture creation with appropriate usage flags
/// - Bind group management for compute shader access
/// - Clear/reset operations
/// - Read-back for display
pub struct AccumulationTexture {
    /// The storage texture that accumulates values
    pub texture: wgpu::Texture,
    /// View for binding to shaders
    pub view: wgpu::TextureView,
    /// Bind group for compute shader access (read-write)
    pub compute_bind_group: wgpu::BindGroup,
    /// Which layout `compute_bind_group` was built against. See
    /// [`AccumulationBindGroupKind`]: a Buddhabrot placeholder must be rebuilt
    /// before the attractor pipeline can dispatch against this texture.
    pub bind_group_kind: AccumulationBindGroupKind,
    /// Texture dimensions
    pub width: u32,
    pub height: u32,
    /// ARC-012: persistent zeroed staging buffer reused across clears when
    /// `Features::CLEAR_TEXTURE` is unavailable. Allocated lazily on first
    /// fallback clear; lifetime tied to this texture (resize recreates the
    /// `AccumulationTexture` and discards the buffer).
    pub zero_buffer: Option<wgpu::Buffer>,
}

impl AccumulationTexture {
    /// Create a new accumulation texture with the given dimensions.
    ///
    /// # Arguments
    /// * `device` - The wgpu device
    /// * `width` - Texture width in pixels
    /// * `height` - Texture height in pixels
    /// * `compute_bind_group_layout` - Layout for compute shader binding
    /// * `label` - Debug label for the texture
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        compute_bind_group_layout: &wgpu::BindGroupLayout,
        label: &str,
    ) -> Self {
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
            // Use R32Uint for atomic accumulation - widely supported for read-write storage
            // We only need hit count in R channel, other channels unused
            format: wgpu::TextureFormat::R32Uint,
            // STORAGE_BINDING for compute write, TEXTURE_BINDING for fragment read.
            // COPY_DST is kept for the CLEAR_TEXTURE-feature-absent fallback path.
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Compute bind group (read-write storage texture)
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{} Compute Bind Group", label)),
            layout: compute_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });

        Self {
            texture,
            view,
            compute_bind_group,
            bind_group_kind: AccumulationBindGroupKind::AttractorTexture,
            width,
            height,
            zero_buffer: None,
        }
    }

    /// Clear the accumulation texture to zeros.
    ///
    /// ARC-012: records the clear into the frame's existing encoder (no
    /// dedicated encoder + no per-frame multi-MB staging allocation). Uses
    /// `CommandEncoder::clear_texture` when the device supports
    /// `Features::CLEAR_TEXTURE`; otherwise falls back to a persistent,
    /// lazily-allocated zero buffer copied into the texture. The caller MUST
    /// encode this clear BEFORE dispatching this frame's compute pass.
    pub fn clear(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        clear_texture_supported: bool,
    ) {
        if clear_texture_supported {
            encoder.clear_texture(
                &self.texture,
                &wgpu::ImageSubresourceRange {
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: 0,
                    array_layer_count: None,
                },
            );
            return;
        }

        // Fallback: reuse a persistent zeroed buffer (sized to this texture).
        // bytes_per_row must be aligned to COPY_BYTES_PER_ROW_ALIGNMENT (256).
        const COPY_BYTES_PER_ROW_ALIGNMENT: u32 = 256;
        let unpadded_bytes_per_row = self.width * 4; // R32Uint = 1 u32 per pixel
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(COPY_BYTES_PER_ROW_ALIGNMENT)
            * COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer_size = (padded_bytes_per_row * self.height) as u64;

        // Lazily allocate the first time we hit this path; reuse thereafter.
        // If the buffer is the wrong size (shouldn't happen because resize
        // recreates the whole AccumulationTexture), drop and rebuild.
        let needs_init = self
            .zero_buffer
            .as_ref()
            .is_none_or(|b| b.size() != buffer_size);
        if needs_init {
            self.zero_buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Accumulation Zero Buffer"),
                    contents: &vec![0u8; buffer_size as usize],
                    usage: wgpu::BufferUsages::COPY_SRC,
                }),
            );
        }

        let zero_buffer = self.zero_buffer.as_ref().unwrap();
        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer: zero_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }
}

/// Creates the bind group layout for compute shader storage texture access.
pub fn create_compute_storage_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Compute Storage Texture Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::StorageTexture {
                access: wgpu::StorageTextureAccess::ReadWrite,
                format: wgpu::TextureFormat::R32Uint, // Widely supported for read-write
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        }],
    })
}

/// Creates the bind group layout for Buddhabrot atomic storage buffer access.
/// This uses a storage buffer with atomics instead of a texture for race-free accumulation.
pub fn create_buddhabrot_storage_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Buddhabrot Storage Buffer Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None, // Dynamic size based on resolution
            },
            count: None,
        }],
    })
}

/// Manages an atomic storage buffer for thread-safe Buddhabrot accumulation.
///
/// Uses a storage buffer with atomic operations instead of a texture
/// to prevent race conditions during concurrent accumulation.
pub struct BuddhabrotAccumulationBuffer {
    /// The storage buffer for atomic accumulation
    pub buffer: wgpu::Buffer,
    /// Bind group for compute shader access
    pub compute_bind_group: wgpu::BindGroup,
    /// Buffer dimensions
    pub width: u32,
    pub height: u32,
}

impl BuddhabrotAccumulationBuffer {
    /// Create a new atomic accumulation buffer.
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        compute_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let buffer_size = (width * height * std::mem::size_of::<u32>() as u32) as u64;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buddhabrot Accumulation Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Buddhabrot Accumulation Compute Bind Group"),
            layout: compute_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            buffer,
            compute_bind_group,
            width,
            height,
        }
    }

    /// Clear the buffer to zeros.
    ///
    /// ARC-012: `CommandEncoder::clear_buffer` is always available (no feature
    /// gate) and avoids building a multi-MB zeroed `Vec` every clear. The
    /// caller MUST encode this clear BEFORE dispatching this frame's compute
    /// pass.
    pub fn clear(&self, encoder: &mut wgpu::CommandEncoder) {
        let size_bytes = (self.width * self.height * std::mem::size_of::<u32>() as u32) as u64;
        encoder.clear_buffer(&self.buffer, 0, Some(size_bytes));
    }
}

/// Creates the bind group layout for uniform buffer access in compute shaders.
pub fn create_compute_uniform_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Compute Uniform Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(
                    std::num::NonZeroU64::new(
                        std::mem::size_of::<AttractorComputeUniforms>() as u64
                    )
                    .unwrap(),
                ),
            },
            count: None,
        }],
    })
}

/// Manages the compute pipeline for strange attractor accumulation.
pub struct AttractorComputePipeline {
    /// The compute pipeline
    pub pipeline: wgpu::ComputePipeline,
    /// Uniform buffer for compute parameters
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group for uniforms
    pub uniform_bind_group: wgpu::BindGroup,
    /// Layout for storage texture binding
    pub storage_layout: wgpu::BindGroupLayout,
    /// Current uniform values
    pub uniforms: AttractorComputeUniforms,
}

impl AttractorComputePipeline {
    /// Create a new attractor compute pipeline.
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Attractor Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/attractor_compute.wgsl").into(),
            ),
        });

        let storage_layout = create_compute_storage_layout(device);
        let uniform_layout = create_compute_uniform_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Attractor Compute Pipeline Layout"),
            bind_group_layouts: &[Some(&storage_layout), Some(&uniform_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Attractor Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let uniforms = AttractorComputeUniforms::default();
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Attractor Compute Uniform Buffer"),
            size: AttractorComputeUniforms::min_size().get(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Attractor Compute Uniform Bind Group"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            storage_layout,
            uniforms,
        }
    }

    /// Update the uniform buffer with current parameters.
    pub fn update_uniforms(&mut self, queue: &wgpu::Queue) {
        let bytes = write_uniform_bytes(&self.uniforms);
        queue.write_buffer(&self.uniform_buffer, 0, &bytes);
    }

    /// Dispatch the compute shader to accumulate attractor points.
    ///
    /// # Arguments
    /// * `encoder` - Command encoder to record to
    /// * `accumulation_bind_group` - Bind group for the accumulation texture
    /// * `num_workgroups` - Number of workgroups to dispatch (each processes points independently)
    pub fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        accumulation_bind_group: &wgpu::BindGroup,
        num_workgroups: u32,
        timestamp_writes: Option<wgpu::ComputePassTimestampWrites<'_>>,
    ) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Attractor Compute Pass"),
            timestamp_writes,
        });

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, accumulation_bind_group, &[]);
        compute_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        // Each workgroup handles multiple orbits
        // Dispatch enough workgroups to generate iterations_per_frame points
        compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
    }
}

/// Creates the bind group layout for Buddhabrot uniform buffer access in compute shaders.
pub fn create_buddhabrot_uniform_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Buddhabrot Compute Uniform Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(
                    std::num::NonZeroU64::new(
                        std::mem::size_of::<BuddhabrotComputeUniforms>() as u64
                    )
                    .unwrap(),
                ),
            },
            count: None,
        }],
    })
}

/// Manages the compute pipeline for Buddhabrot accumulation.
pub struct BuddhabrotComputePipeline {
    /// The compute pipeline
    pub pipeline: wgpu::ComputePipeline,
    /// Uniform buffer for compute parameters
    pub uniform_buffer: wgpu::Buffer,
    /// Bind group for uniforms
    pub uniform_bind_group: wgpu::BindGroup,
    /// Layout for storage texture binding
    pub storage_layout: wgpu::BindGroupLayout,
    /// Current uniform values
    pub uniforms: BuddhabrotComputeUniforms,
}

impl BuddhabrotComputePipeline {
    /// Create a new Buddhabrot compute pipeline.
    /// Uses a storage buffer layout for atomic accumulation instead of texture.
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Buddhabrot Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/buddhabrot_compute.wgsl").into(),
            ),
        });

        // Use storage buffer layout for atomic operations (not texture)
        let storage_layout = create_buddhabrot_storage_layout(device);
        let uniform_layout = create_buddhabrot_uniform_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Buddhabrot Compute Pipeline Layout"),
            bind_group_layouts: &[Some(&storage_layout), Some(&uniform_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Buddhabrot Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let uniforms = BuddhabrotComputeUniforms::default();
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buddhabrot Compute Uniform Buffer"),
            size: BuddhabrotComputeUniforms::min_size().get(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Buddhabrot Compute Uniform Bind Group"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            storage_layout,
            uniforms,
        }
    }

    /// Update the uniform buffer with current parameters.
    pub fn update_uniforms(&mut self, queue: &wgpu::Queue) {
        let bytes = write_uniform_bytes(&self.uniforms);
        queue.write_buffer(&self.uniform_buffer, 0, &bytes);
    }

    /// Dispatch the compute shader to accumulate Buddhabrot points.
    ///
    /// # Arguments
    /// * `encoder` - Command encoder to record to
    /// * `accumulation_bind_group` - Bind group for the accumulation texture
    /// * `num_workgroups` - Number of workgroups to dispatch
    pub fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        accumulation_bind_group: &wgpu::BindGroup,
        num_workgroups: u32,
        timestamp_writes: Option<wgpu::ComputePassTimestampWrites<'_>>,
    ) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Buddhabrot Compute Pass"),
            timestamp_writes,
        });

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, accumulation_bind_group, &[]);
        compute_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        // Each workgroup (256 threads) tests samples_per_thread samples each
        compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ENH-008: `AccumulationDisplayUniforms` derives `encase::ShaderType`. Lock
    /// the layout: 3 scalars, 4B implicit pad, then `array<vec4<f32>, 8>` (16B
    /// stride) starting at @16; total 144B. Pins the palette offset + stride.
    #[test]
    fn accumulation_display_uniform_byte_layout() {
        let u = AccumulationDisplayUniforms {
            log_scale: 1.0,
            gamma: 2.0,
            palette_offset: 3.0,
            palette: std::array::from_fn(|i| glam::Vec4::new(i as f32, 0.0, 0.0, 1.0)),
        };
        let bytes = write_uniform_bytes(&u);
        assert_eq!(bytes.len(), 144);
        let f = |o: usize| f32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
        assert_eq!(f(0), 1.0); // log_scale
        assert_eq!(f(4), 2.0); // gamma
        assert_eq!(f(8), 3.0); // palette_offset
        // 12..16 implicit pad aligns the array<vec4<f32>, 8> to 16B stride
        assert_eq!(f(16), 0.0); // palette[0].x (i=0)
        assert_eq!(f(28), 1.0); // palette[0].w
        assert_eq!(f(32), 1.0); // palette[1].x (i=1)
        assert_eq!(f(128), 7.0); // palette[7].x (16 + 7*16)
        assert_eq!(f(140), 1.0); // palette[7].w
    }
}
