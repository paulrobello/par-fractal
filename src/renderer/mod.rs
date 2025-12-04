// Module declarations
pub mod compute;
mod initialization;
pub mod uniforms;
mod update;

use compute::{
    AccumulationDisplayUniforms, AccumulationTexture, AttractorComputePipeline,
    BuddhabrotAccumulationBuffer, BuddhabrotComputePipeline,
};
use uniforms::*;

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub backend: String,
    pub device_type: String,
}

pub struct Renderer {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,

    // Main fractal rendering
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    uniforms: Uniforms,
    pub start_time: web_time::Instant,

    // Multi-pass post-processing
    pub scene_texture: wgpu::Texture,
    pub scene_view: wgpu::TextureView,
    pub bright_texture: wgpu::Texture,
    pub bright_view: wgpu::TextureView,
    pub blur_temp_texture: wgpu::Texture,
    pub blur_temp_view: wgpu::TextureView,
    pub bloom_texture: wgpu::Texture,
    pub bloom_view: wgpu::TextureView,
    pub composite_texture: wgpu::Texture,
    pub composite_view: wgpu::TextureView,

    pub sampler: wgpu::Sampler,
    pub postprocess_vertex_buffer: wgpu::Buffer,

    // Post-processing pipelines
    pub bloom_extract_pipeline: wgpu::RenderPipeline,
    pub blur_pipeline: wgpu::RenderPipeline,
    pub composite_pipeline: wgpu::RenderPipeline,
    pub fxaa_pipeline: wgpu::RenderPipeline,
    pub copy_pipeline: wgpu::RenderPipeline,

    // Post-processing uniforms
    pub bloom_uniform_buffer: wgpu::Buffer,
    pub blur_uniform_buffer: wgpu::Buffer,
    pub composite_uniform_buffer: wgpu::Buffer,

    // Bind groups
    pub scene_bind_group: wgpu::BindGroup,
    pub bright_bind_group: wgpu::BindGroup,
    pub blur_temp_bind_group: wgpu::BindGroup,
    pub composite_bind_group: wgpu::BindGroup,
    pub composite_final_bind_group: wgpu::BindGroup, // For final pass (FXAA or copy)
    pub bloom_params_bind_group: wgpu::BindGroup,
    pub blur_h_params_bind_group: wgpu::BindGroup,
    pub blur_v_params_bind_group: wgpu::BindGroup,
    pub composite_params_bind_group: wgpu::BindGroup,

    // Compute shader infrastructure for accumulation-based fractals
    pub attractor_compute: Option<AttractorComputePipeline>,
    pub buddhabrot_compute: Option<BuddhabrotComputePipeline>,
    pub accumulation_texture: Option<AccumulationTexture>,
    /// Atomic storage buffer for Buddhabrot accumulation (separate from texture-based attractors)
    pub buddhabrot_accumulation_buffer: Option<BuddhabrotAccumulationBuffer>,
    /// Compute pipeline to copy from Buddhabrot buffer to texture for display
    pub buddhabrot_copy_pipeline: Option<wgpu::ComputePipeline>,
    pub buddhabrot_copy_bind_group: Option<wgpu::BindGroup>,
    pub accumulation_display_pipeline: wgpu::RenderPipeline, // Uses fs_accumulation_display
    pub accumulation_display_bind_group: Option<wgpu::BindGroup>,
    pub accumulation_display_uniform_buffer: wgpu::Buffer,
    pub accumulation_display_uniform_bind_group: wgpu::BindGroup,
}
