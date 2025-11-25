use super::{
    AccumulationTexture, AttractorComputePipeline, BloomUniforms, BlurUniforms, GpuInfo,
    PostProcessUniforms, Renderer, Uniforms,
};
use wgpu::util::DeviceExt;

/// GPU initialization and setup methods
impl Renderer {
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn enumerate_gpus() -> Vec<GpuInfo> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapters = instance.enumerate_adapters(wgpu::Backends::all());
        adapters
            .into_iter()
            .map(|adapter| {
                let info = adapter.get_info();
                GpuInfo {
                    name: info.name,
                    backend: format!("{:?}", info.backend),
                    device_type: format!("{:?}", info.device_type),
                }
            })
            .collect()
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn enumerate_gpus() -> Vec<GpuInfo> {
        // GPU enumeration not available on web - browser handles GPU selection
        Vec::new()
    }

    pub async fn new(
        window: std::sync::Arc<winit::window::Window>,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> Self {
        Self::new_with_gpu_preference(window, size, None).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn new_with_gpu_preference(
        window: std::sync::Arc<winit::window::Window>,
        size: winit::dpi::PhysicalSize<u32>,
        preferred_gpu_index: Option<usize>,
    ) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        // Select adapter based on preference or fallback to default
        let adapter = if let Some(gpu_index) = preferred_gpu_index {
            // Try to get the adapter at the specified index
            let adapters = instance.enumerate_adapters(wgpu::Backends::all());

            if gpu_index < adapters.len() {
                let selected = adapters.into_iter().nth(gpu_index).unwrap();
                let info = selected.get_info();
                println!(
                    "Using selected GPU #{}: {} ({:?}, {:?})",
                    gpu_index, info.name, info.backend, info.device_type
                );
                selected
            } else {
                println!(
                    "Preferred GPU index {} not found, falling back to default",
                    gpu_index
                );
                instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::HighPerformance,
                        compatible_surface: Some(&surface),
                        force_fallback_adapter: false,
                    })
                    .await
                    .unwrap()
            }
        } else {
            // Use default selection
            instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await
                .unwrap()
        };

        Self::initialize_with_adapter(surface, adapter, size).await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_with_gpu_preference(
        window: std::sync::Arc<winit::window::Window>,
        size: winit::dpi::PhysicalSize<u32>,
        _preferred_gpu_index: Option<usize>,
    ) -> Self {
        // On web, browser handles GPU selection - ignore preference
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        Self::initialize_with_adapter(surface, adapter, size).await
    }

    async fn initialize_with_adapter(
        surface: wgpu::Surface<'static>,
        adapter: wgpu::Adapter,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> Self {
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                experimental_features: Default::default(),
                trace: Default::default(),
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/fractal.wgsl").into()),
        });

        // Create uniform buffer
        let uniforms = Uniforms::new();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            std::num::NonZeroU64::new(std::mem::size_of::<Uniforms>() as u64)
                                .unwrap(),
                        ),
                    },
                    count: None,
                }],
                label: Some("uniform_bind_group_layout"),
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float, // Render to HDR intermediate texture
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // Fullscreen quad vertices
        let vertices: &[f32] = &[-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // ============================================================================
        // Multi-pass Post-Processing Setup
        // ============================================================================

        // Load post-processing shader
        let postprocess_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Post-Process Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/postprocess.wgsl").into()),
        });

        // Create sampler for texture sampling
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Post-Process Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create intermediate render textures
        let (scene_texture, scene_view) =
            Self::create_render_texture(&device, size.width, size.height, "Scene Texture");
        let (bright_texture, bright_view) =
            Self::create_render_texture(&device, size.width, size.height, "Bright Texture");
        let (blur_temp_texture, blur_temp_view) =
            Self::create_render_texture(&device, size.width, size.height, "Blur Temp Texture");
        let (bloom_texture, bloom_view) =
            Self::create_render_texture(&device, size.width, size.height, "Bloom Texture");
        let (composite_texture, composite_view) =
            Self::create_render_texture(&device, size.width, size.height, "Composite Texture");

        // Create post-processing vertex buffer (fullscreen quad with tex coords)
        // Format: [x, y, u, v] for each vertex
        let postprocess_vertices: &[f32] = &[
            -1.0, -1.0, 0.0, 1.0, // Bottom-left
            1.0, -1.0, 1.0, 1.0, // Bottom-right
            -1.0, 1.0, 0.0, 0.0, // Top-left
            1.0, 1.0, 1.0, 0.0, // Top-right
        ];
        let postprocess_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Post-Process Vertex Buffer"),
                contents: bytemuck::cast_slice(postprocess_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        // Create post-processing uniform buffers
        let bloom_uniforms = BloomUniforms {
            threshold: 0.7,
            intensity: 0.5,
            _padding: [0.0; 2],
        };
        let bloom_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Bloom Uniform Buffer"),
            contents: bytemuck::cast_slice(&[bloom_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let blur_h_uniforms = BlurUniforms {
            direction: [1.0, 0.0], // Horizontal
            _padding: [0.0; 2],
        };
        let _blur_v_uniforms = BlurUniforms {
            direction: [0.0, 1.0], // Vertical
            _padding: [0.0; 2],
        };
        let blur_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Blur Uniform Buffer"),
            contents: bytemuck::cast_slice(&[blur_h_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let composite_uniforms = PostProcessUniforms {
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            hue_shift: 0.0,
            vignette_enabled: 0,
            vignette_intensity: 0.5,
            vignette_radius: 0.8,
            _padding1: 0.0,
            bloom_enabled: 0,
            bloom_intensity: 0.5,
            _padding2: [0.0; 2],
            _padding3: [0.0; 4],
        };
        let composite_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Composite Uniform Buffer"),
                contents: bytemuck::cast_slice(&[composite_uniforms]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group layouts
        // Layout for texture + sampler (group 0)
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Layout for composite pass (scene + bloom textures)
        let composite_texture_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Composite Texture Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Layout for uniforms (group 1)
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Post-Process Uniform Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Create post-processing render pipelines
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: (std::mem::size_of::<f32>() * 4) as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x2,  // position (x, y)
                1 => Float32x2,  // tex_coords (u, v)
            ],
        };

        // Bloom extract pipeline
        let bloom_extract_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Bloom Extract Layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_layout],
            push_constant_ranges: &[],
        });

        let bloom_extract_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some("Bloom Extract Pipeline"),
                layout: Some(&bloom_extract_layout),
                vertex: wgpu::VertexState {
                    module: &postprocess_shader,
                    entry_point: Some("vs_main"),
                    buffers: std::slice::from_ref(&vertex_buffer_layout),
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &postprocess_shader,
                    entry_point: Some("fs_bloom_extract"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

        // Blur pipeline
        let blur_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blur Layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_layout],
            push_constant_ranges: &[],
        });

        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("Blur Pipeline"),
            layout: Some(&blur_layout),
            vertex: wgpu::VertexState {
                module: &postprocess_shader,
                entry_point: Some("vs_main"),
                buffers: std::slice::from_ref(&vertex_buffer_layout),
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &postprocess_shader,
                entry_point: Some("fs_blur"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Composite pipeline
        let composite_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Composite Layout"),
            bind_group_layouts: &[&composite_texture_layout, &uniform_layout],
            push_constant_ranges: &[],
        });

        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("Composite Pipeline"),
            layout: Some(&composite_layout),
            vertex: wgpu::VertexState {
                module: &postprocess_shader,
                entry_point: Some("vs_main"),
                buffers: std::slice::from_ref(&vertex_buffer_layout),
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &postprocess_shader,
                entry_point: Some("fs_composite"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // FXAA pipeline
        let fxaa_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("FXAA Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let fxaa_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("FXAA Pipeline"),
            layout: Some(&fxaa_layout),
            vertex: wgpu::VertexState {
                module: &postprocess_shader,
                entry_point: Some("vs_main"),
                buffers: std::slice::from_ref(&vertex_buffer_layout),
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &postprocess_shader,
                entry_point: Some("fs_fxaa"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Copy/passthrough pipeline (for when FXAA is disabled)
        let copy_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Copy Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let copy_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("Copy Pipeline"),
            layout: Some(&copy_layout),
            vertex: wgpu::VertexState {
                module: &postprocess_shader,
                entry_point: Some("vs_main"),
                buffers: std::slice::from_ref(&vertex_buffer_layout),
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &postprocess_shader,
                entry_point: Some("fs_copy"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Accumulation display pipeline (for visualizing accumulated attractor data)
        let accumulation_display_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Accumulation Display Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let accumulation_display_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some("Accumulation Display Pipeline"),
                layout: Some(&accumulation_display_layout),
                vertex: wgpu::VertexState {
                    module: &postprocess_shader,
                    entry_point: Some("vs_main"),
                    buffers: std::slice::from_ref(&vertex_buffer_layout),
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &postprocess_shader,
                    entry_point: Some("fs_accumulation_display"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float, // Output to scene_texture
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

        // Create bind groups for textures
        let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let bright_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bright Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&bright_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let blur_temp_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blur Temp Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&blur_temp_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Composite Bind Group"),
            layout: &composite_texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&bloom_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Create bind groups for uniforms
        let bloom_params_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bloom Params Bind Group"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: bloom_uniform_buffer.as_entire_binding(),
            }],
        });

        let blur_h_params_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blur H Params Bind Group"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: blur_uniform_buffer.as_entire_binding(),
            }],
        });

        // For vertical blur, we'll need to update the buffer before use
        let blur_v_params_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blur V Params Bind Group"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: blur_uniform_buffer.as_entire_binding(),
            }],
        });

        let composite_params_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Composite Params Bind Group"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: composite_uniform_buffer.as_entire_binding(),
            }],
        });

        // Final bind group for composite texture (for FXAA/copy to screen)
        let composite_final_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Composite Final Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&composite_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            uniform_buffer,
            uniform_bind_group,
            uniforms,
            start_time: web_time::Instant::now(),

            // Multi-pass post-processing
            scene_texture,
            scene_view,
            bright_texture,
            bright_view,
            blur_temp_texture,
            blur_temp_view,
            bloom_texture,
            bloom_view,
            composite_texture,
            composite_view,

            sampler,
            postprocess_vertex_buffer,

            bloom_extract_pipeline,
            blur_pipeline,
            composite_pipeline,
            fxaa_pipeline,
            copy_pipeline,

            bloom_uniform_buffer,
            blur_uniform_buffer,
            composite_uniform_buffer,

            scene_bind_group,
            bright_bind_group,
            blur_temp_bind_group,
            composite_bind_group,
            composite_final_bind_group,
            bloom_params_bind_group,
            blur_h_params_bind_group,
            blur_v_params_bind_group,
            composite_params_bind_group,

            // Compute shader infrastructure (initialized lazily when needed)
            attractor_compute: None,
            accumulation_texture: None,
            accumulation_display_pipeline,
            accumulation_display_bind_group: None,
            accumulation_sample_layout: None,
        }
    }

    /// Initialize the compute shader infrastructure for strange attractor accumulation.
    /// This is called lazily when accumulation mode is first enabled.
    pub fn init_accumulation_compute(&mut self) {
        if self.attractor_compute.is_some() {
            return; // Already initialized
        }

        // Create compute pipeline
        let attractor_compute = AttractorComputePipeline::new(&self.device);

        // Create sample bind group layout for the accumulation texture
        let accumulation_sample_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Accumulation Sample Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        // Create accumulation texture
        let accumulation_texture = AccumulationTexture::new(
            &self.device,
            self.size.width,
            self.size.height,
            &attractor_compute.storage_layout,
            &accumulation_sample_layout,
            &self.sampler,
            "Attractor Accumulation Texture",
        );

        // Create bind group for sampling the accumulation texture
        // Use the layout from the accumulation_display_pipeline which was created in initialization
        let accumulation_display_bind_group =
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Accumulation Display Bind Group"),
                layout: &self.accumulation_display_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&accumulation_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

        self.attractor_compute = Some(attractor_compute);
        self.accumulation_texture = Some(accumulation_texture);
        self.accumulation_display_bind_group = Some(accumulation_display_bind_group);
        self.accumulation_sample_layout = Some(accumulation_sample_layout);
    }

    // Helper: Create a render texture
}
