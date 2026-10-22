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
pub(crate) mod post_chain;
pub(crate) use post_chain::{PostChainTargets, PostOptions};
/// ENH-006 Task 1: GPU frame profiler. Per-pass timestamp queries with a
/// 3-deep staging ring for ~2-frame-latent, non-blocking readback, exposing
/// `GpuProfiler::timings_ms` for the HUD (Task 2) and CLI dump (Task 3).
pub mod profiler;
/// GPU uniform buffer definitions. The `Uniforms` struct here must stay
/// byte-identical to the `Uniforms` struct in `shaders/fractal/uniforms.wgsl`.
pub mod uniforms;
mod update;

use std::collections::HashMap;

use compute::{
    AccumulationDisplayUniforms, AccumulationTexture, AttractorComputePipeline,
    BuddhabrotAccumulationBuffer, BuddhabrotComputePipeline,
};
use orbit_buffer::{ActiveOrbit, OrbitBuffer};
use profiler::GpuProfiler;
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
/// it is kept in sync with `shaders/fractal/uniforms.wgsl`'s `Uniforms` struct. Most
/// fields are public so other modules can resize buffers, rebind, or read GPU
/// state during frame updates.
pub struct Renderer {
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) size: winit::dpi::PhysicalSize<u32>,

    // Main fractal rendering
    // ARC-009/ENH-004: two pipelines sharing one layout/module — `fs_main_2d`
    // holds only the escape-time + palette path, `fs_main_3d` only the
    // ray-march + lighting path. Selected per frame via
    // `FractalType::is_3d()`; the unused path is dead-code-eliminated per
    // entry point by naga/the backend.
    pub(crate) pipeline_2d: wgpu::RenderPipeline,
    pub(crate) pipeline_3d: wgpu::RenderPipeline,
    /// ENH-004 Stage 2: the shader module and pipeline layout retained past
    /// init so specialized 3D pipelines can be compiled lazily at runtime
    /// (one per fractal type, overriding the WGSL `SPECIALIZED_TYPE` constant).
    pub(crate) shader_module: wgpu::ShaderModule,
    pub(crate) render_pipeline_layout: wgpu::PipelineLayout,
    /// ENH-004 Stage 2: lazily-compiled specialized 3D pipelines keyed by the
    /// fractal type's GPU index (`FractalType as u32`). A miss returns the
    /// generic `pipeline_3d` for that frame while the specialized variant
    /// compiles. Populated by `ensure_specialized_3d_pipeline`.
    pub(crate) pipeline_3d_cache: HashMap<u32, wgpu::RenderPipeline>,
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) uniform_buffer: wgpu::Buffer,
    pub(crate) uniform_bind_group: wgpu::BindGroup,
    /// Issue #1 Julia-under-cursor preview: dedicated uniform buffer + bind
    /// group for the inset pass, sharing the main bind-group layout (the
    /// orbit-buffer binding reads a placeholder; `orbit_len = 0` in the
    /// inset uniforms keeps perturbation off).
    pub(crate) julia_preview_uniform_buffer: wgpu::Buffer,
    pub(crate) julia_preview_bind_group: wgpu::BindGroup,
    /// Inset pipeline: same shader module / 2D entry point as `pipeline_2d`
    /// but targeting the surface format — the inset draws after the post
    /// chain, straight into the swap-chain view.
    pub(crate) julia_preview_pipeline: wgpu::RenderPipeline,
    /// ENH-001 Phase A step 3: storage buffer for the perturbation reference
    /// orbit. Bound at `@group(0) @binding(1)` so it shares group 0 with
    /// the uniforms. Step 5 (CPU driver) uploads a real orbit here; before
    /// the first deep-zoom view it holds a one-entry placeholder so the bind
    /// group validates.
    pub(crate) orbit_buffer: OrbitBuffer,
    /// ENH-001 Phase A step 5: metadata of the currently-uploaded orbit, or
    /// `None` when perturbation is off (no orbit yet, the gate isn't met, or
    /// the view dropped below the gate). The buffer contents may still be
    /// present; this field is the live/not-live switch that
    /// `Renderer::update` reads to populate `perturbation_enabled`.
    pub(crate) active_orbit: Option<ActiveOrbit>,
    uniforms: Uniforms,
    pub(crate) start_time: web_time::Instant,

    // Multi-pass post-processing
    pub(crate) scene_texture: wgpu::Texture,
    pub(crate) scene_view: wgpu::TextureView,
    pub(crate) bright_texture: wgpu::Texture,
    pub(crate) bright_view: wgpu::TextureView,
    pub(crate) blur_temp_texture: wgpu::Texture,
    pub(crate) blur_temp_view: wgpu::TextureView,
    pub(crate) bloom_texture: wgpu::Texture,
    pub(crate) bloom_view: wgpu::TextureView,
    pub(crate) composite_texture: wgpu::Texture,
    pub(crate) composite_view: wgpu::TextureView,

    pub(crate) sampler: wgpu::Sampler,
    pub(crate) postprocess_vertex_buffer: wgpu::Buffer,

    // Post-processing pipelines
    pub(crate) bloom_extract_pipeline: wgpu::RenderPipeline,
    pub(crate) blur_pipeline: wgpu::RenderPipeline,
    pub(crate) composite_pipeline: wgpu::RenderPipeline,
    pub(crate) fxaa_pipeline: wgpu::RenderPipeline,
    pub(crate) copy_pipeline: wgpu::RenderPipeline,

    // Post-processing uniforms
    pub(crate) bloom_uniform_buffer: wgpu::Buffer,
    // ARC-022: the blur direction buffers are not Renderer fields — each
    // bind group keeps its own (1,0)/(0,1) buffer alive from init.
    pub(crate) composite_uniform_buffer: wgpu::Buffer,

    // Bind groups
    pub(crate) scene_bind_group: wgpu::BindGroup,
    pub(crate) bright_bind_group: wgpu::BindGroup,
    pub(crate) blur_temp_bind_group: wgpu::BindGroup,
    pub(crate) composite_bind_group: wgpu::BindGroup,
    pub(crate) composite_final_bind_group: wgpu::BindGroup, // For final pass (FXAA or copy)
    pub(crate) bloom_params_bind_group: wgpu::BindGroup,
    pub(crate) blur_h_params_bind_group: wgpu::BindGroup,
    pub(crate) blur_v_params_bind_group: wgpu::BindGroup,
    pub(crate) composite_params_bind_group: wgpu::BindGroup,

    // Compute shader infrastructure for accumulation-based fractals
    pub(crate) attractor_compute: Option<AttractorComputePipeline>,
    pub(crate) buddhabrot_compute: Option<BuddhabrotComputePipeline>,
    pub(crate) accumulation_texture: Option<AccumulationTexture>,
    /// Atomic storage buffer for Buddhabrot accumulation (separate from texture-based attractors)
    pub(crate) buddhabrot_accumulation_buffer: Option<BuddhabrotAccumulationBuffer>,
    /// Compute pipeline to copy from Buddhabrot buffer to texture for display
    pub(crate) buddhabrot_copy_pipeline: Option<wgpu::ComputePipeline>,
    pub(crate) buddhabrot_copy_bind_group: Option<wgpu::BindGroup>,
    pub(crate) accumulation_display_pipeline: wgpu::RenderPipeline, // Uses fs_accumulation_display
    pub(crate) accumulation_display_bind_group: Option<wgpu::BindGroup>,
    pub(crate) accumulation_display_uniform_buffer: wgpu::Buffer,
    pub(crate) accumulation_display_uniform_bind_group: wgpu::BindGroup,

    /// ARC-012: true when the device supports `Features::CLEAR_TEXTURE`, letting
    /// `AccumulationTexture::clear` use `CommandEncoder::clear_texture` instead
    /// of the persistent-zero-buffer fallback. Always false on wasm (WebGPU
    /// does not expose the feature).
    pub(crate) clear_texture_supported: bool,

    /// ENH-006: GPU frame profiler. Owns the timestamp `QuerySet` (or `None`
    /// when the device lacks `Features::TIMESTAMP_QUERY`), the resolve buffer,
    /// and the 3-deep staging ring. Public so `App::render` can call
    /// `pass_writes` / `end_frame` / `poll_results` directly each frame; read
    /// `timings_ms` for the HUD (Task 2) and CLI dump (Task 3).
    pub(crate) profiler: GpuProfiler,

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
    pub(crate) scene_render_scale: f32,
    /// ENH-003: when `Some`, forces `scene_render_scale` regardless of LOD —
    /// used by high-resolution capture / video paths so a capture taken mid-motion
    /// (LOD at reduced scale) still renders at full quality. `None` in normal use.
    render_scale_override: Option<f32>,
    /// ENH-002 v2: set by `App` while tile-progressive refinement is in flight
    /// so `Renderer::update` forces `scene_render_scale = 1.0` (tiles must
    /// cover the full `scene_texture`, and the post chain must read it at full
    /// resolution — a sub-rect viewport would conflict with the per-tile
    /// scissor).
    pub(crate) refining: bool,
}
