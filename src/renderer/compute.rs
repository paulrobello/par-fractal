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
//! This module provides the infrastructure for compute shader-based accumulation.
//! Integration with the main renderer is pending - the UI controls are available
//! but the actual compute passes are not yet wired into the render loop.

#![allow(dead_code)] // Infrastructure code - will be used when integrated

use bytemuck::{Pod, Zeroable};

/// Uniforms for the accumulation display shader
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct AccumulationDisplayUniforms {
    pub log_scale: f32,
    pub gamma: f32,
    pub palette_offset: f32,
    pub _padding: f32,
    /// 8 palette colors, each as [r, g, b, a] (alpha unused but needed for alignment)
    pub palette: [[f32; 4]; 8],
}

impl Default for AccumulationDisplayUniforms {
    fn default() -> Self {
        Self {
            log_scale: 1.0,
            gamma: 0.6,
            palette_offset: 0.0,
            _padding: 0.0,
            // Default fire palette (8 colors)
            palette: [
                [0.0, 0.0, 0.0, 1.0],   // Black
                [0.25, 0.0, 0.25, 1.0], // Deep purple
                [0.5, 0.0, 0.5, 1.0],   // Purple
                [0.75, 0.0, 0.25, 1.0], // Magenta
                [1.0, 0.0, 0.0, 1.0],   // Red
                [1.0, 0.5, 0.0, 1.0],   // Orange
                [1.0, 0.75, 0.0, 1.0],  // Light orange
                [1.0, 1.0, 0.0, 1.0],   // Yellow
            ],
        }
    }
}

/// Uniforms for the attractor compute shader
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
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
    pub _padding: [u32; 2],
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
            _padding: [0; 2],
        }
    }
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
    /// Texture dimensions
    pub width: u32,
    pub height: u32,
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
            // STORAGE_BINDING for compute write, TEXTURE_BINDING for fragment read
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST, // For clearing
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
            width,
            height,
        }
    }

    /// Clear the accumulation texture to zeros.
    ///
    /// This queues a buffer copy to zero out the texture.
    pub fn clear(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        // bytes_per_row must be aligned to COPY_BYTES_PER_ROW_ALIGNMENT (256 bytes)
        const COPY_BYTES_PER_ROW_ALIGNMENT: u32 = 256;
        let unpadded_bytes_per_row = self.width * 4; // 1 u32 * 4 bytes (R32Uint)
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(COPY_BYTES_PER_ROW_ALIGNMENT)
            * COPY_BYTES_PER_ROW_ALIGNMENT;

        let buffer_size = (padded_bytes_per_row * self.height) as u64;
        let zeros = vec![0u8; buffer_size as usize];

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Accumulation Clear Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: true,
        });

        staging_buffer
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(&zeros);
        staging_buffer.unmap();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Accumulation Clear Encoder"),
        });

        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer: &staging_buffer,
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

        queue.submit(std::iter::once(encoder.finish()));
    }

    /// Resize the accumulation texture.
    ///
    /// This creates a new texture with the new dimensions and recreates bind groups.
    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        compute_bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        if width == self.width && height == self.height {
            return;
        }

        *self = Self::new(
            device,
            width,
            height,
            compute_bind_group_layout,
            "Accumulation Texture",
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
    /// Random state for orbit starting points (persists between frames)
    pub random_state: [u32; 4],
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
            bind_group_layouts: &[&storage_layout, &uniform_layout],
            push_constant_ranges: &[],
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
            size: std::mem::size_of::<AttractorComputeUniforms>() as u64,
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

        // Initialize random state with some seed values
        let random_state = [0x12345678u32, 0x9ABCDEF0, 0xDEADBEEF, 0xCAFEBABE];

        Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            storage_layout,
            uniforms,
            random_state,
        }
    }

    /// Update the uniform buffer with current parameters.
    pub fn update_uniforms(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
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
    ) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Attractor Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, accumulation_bind_group, &[]);
        compute_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        // Each workgroup handles multiple orbits
        // Dispatch enough workgroups to generate iterations_per_frame points
        compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
    }
}

/// Configuration for attractor accumulation rendering
#[derive(Debug, Clone)]
pub struct AttractorAccumulationConfig {
    /// Number of orbit iterations to compute per frame
    pub iterations_per_frame: u32,
    /// Whether accumulation mode is enabled
    pub enabled: bool,
    /// Total accumulated iterations
    pub total_iterations: u64,
    /// Whether to clear on next frame
    pub pending_clear: bool,
}

impl Default for AttractorAccumulationConfig {
    fn default() -> Self {
        Self {
            iterations_per_frame: 100_000,
            enabled: false,
            total_iterations: 0,
            pending_clear: false,
        }
    }
}
