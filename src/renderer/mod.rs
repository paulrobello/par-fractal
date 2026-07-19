// Module declarations
/// Compute pipelines for accumulation-based fractals (strange attractors,
/// Buddhabrot) and their GPU buffers/textures.
pub mod compute;
mod initialization;
/// GPU storage buffer for the perturbation reference orbit (ENH-001 Phase A
/// step 3). Holds `Z_n` f32 pairs uploaded from a CPU-computed
/// [`crate::deep_zoom::ReferenceOrbit`] for the per-pixel delta shader to
/// read in fragment stage.
pub mod orbit_buffer;
/// GPU uniform buffer definitions. The `Uniforms` struct here must stay
/// byte-identical to the `Uniforms` struct in `shaders/fractal.wgsl`.
pub mod uniforms;
mod update;

use compute::{
    AccumulationDisplayUniforms, AccumulationTexture, AttractorComputePipeline,
    BuddhabrotAccumulationBuffer, BuddhabrotComputePipeline,
};
use orbit_buffer::{ActiveOrbit, OrbitBuffer};
use uniforms::*;

/// User-facing description of the selected physical GPU.
///
/// Returned by renderer initialization so the UI can show which adapter was
/// chosen. Strings are formatted for display (not stable identifiers).
#[derive(Debug, Clone)]
pub struct GpuInfo {
    /// Human-readable adapter name reported by wgpu.
    pub name: String,
    /// Backend in use (e.g. `"Metal"`, `"Vulkan"`, `"DX12"`, `"BrowserWebGpu"`).
    pub backend: String,
    /// Device type, e.g. `"DiscreteGpu"`, `"IntegratedGpu"`, `"Cpu"`.
    pub device_type: String,
}

/// Owns the wgpu surface, device, and queue, plus all render and
/// post-processing pipelines, textures, bind groups, and uniform buffers
/// needed to draw a frame.
///
/// The `uniforms` field holds the CPU-side mirror of the GPU uniform buffer;
/// it is kept in sync with `shaders/fractal.wgsl`'s `Uniforms` struct. Most
/// fields are public so other modules can resize buffers, rebind, or read GPU
/// state during frame updates.
pub struct Renderer {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,

    // Main fractal rendering
    // ARC-009/ENH-004: two pipelines sharing one layout/module — `fs_main_2d`
    // holds only the escape-time + palette path, `fs_main_3d` only the
    // ray-march + lighting path. Selected per frame via
    // `FractalType::is_3d()`; the unused path is dead-code-eliminated per
    // entry point by naga/the backend.
    pub pipeline_2d: wgpu::RenderPipeline,
    pub pipeline_3d: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    /// ENH-001 Phase A step 3: storage buffer for the perturbation reference
    /// orbit. Bound at `@group(0) @binding(1)` so it shares group 0 with
    /// the uniforms. Step 5 (CPU driver) uploads a real orbit here; before
    /// the first deep-zoom view it holds a one-entry placeholder so the bind
    /// group validates.
    pub orbit_buffer: OrbitBuffer,
    /// ENH-001 Phase A step 5: metadata of the currently-uploaded orbit, or
    /// `None` when perturbation is off (no orbit yet, the gate isn't met, or
    /// the view dropped below the gate). The buffer contents may still be
    /// present; this field is the live/not-live switch that
    /// `Renderer::update` reads to populate `perturbation_enabled`.
    pub active_orbit: Option<ActiveOrbit>,
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

    /// ARC-012: true when the device supports `Features::CLEAR_TEXTURE`, letting
    /// `AccumulationTexture::clear` use `CommandEncoder::clear_texture` instead
    /// of the persistent-zero-buffer fallback. Always false on wasm (WebGPU
    /// does not expose the feature).
    pub clear_texture_supported: bool,

    /// ARC-017: cached `BloomUniforms` from the last upload; the buffer write
    /// is skipped when the value hasn't changed. `None` until first upload.
    cached_bloom_uniforms: Option<uniforms::BloomUniforms>,
    /// ARC-017: cached `PostProcessUniforms` from the last upload; the buffer
    /// write is skipped when the value hasn't changed. `None` until first upload.
    cached_composite_uniforms: Option<uniforms::PostProcessUniforms>,

    /// ENH-003: the render scale in effect for the most recent `update()`,
    /// read by `App::render` to set the scene-pass viewport. 1.0 = native res;
    /// <1.0 = the fractal pass renders into the top-left sub-rect of
    /// `scene_texture` and the post chain upsamples. Driven by LOD's active
    /// `QualityLevel.render_scale` (so it tracks motion automatically), forced
    /// to 1.0 for accumulation display and high-resolution capture.
    pub scene_render_scale: f32,
    /// ENH-003: when `Some`, forces `scene_render_scale` regardless of LOD —
    /// used by high-resolution capture / video paths so a capture taken mid-motion
    /// (LOD at reduced scale) still renders at full quality. `None` in normal use.
    render_scale_override: Option<f32>,
}
