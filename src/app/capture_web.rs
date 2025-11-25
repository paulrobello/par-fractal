//! Web capture implementation using WebGPU buffer readback

use crate::camera::Camera;
use crate::fractal::FractalParams;
use crate::platform::web::WebCapture;
use crate::platform::Capture;
use crate::renderer::Renderer;
use std::sync::{Arc, Mutex};

/// State shared between the map_async callback and the async task
struct CaptureState {
    mapping_complete: bool,
    mapping_result: Option<Result<(), wgpu::BufferAsyncError>>,
}

/// Captures a screenshot from the GPU texture on web.
///
/// This creates a staging buffer, copies the texture data, and triggers
/// an async download once the buffer is mapped.
pub fn capture_screenshot_web(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    fractal_name: String,
    show_toast: Box<dyn Fn(String) + Send + 'static>,
) {
    // Calculate buffer size with proper alignment
    let bytes_per_row = (width * 4 + 255) & !255; // Align to 256 bytes
    let buffer_size = (bytes_per_row * height) as wgpu::BufferAddress;

    // Create buffer to copy texture to
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Screenshot Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Create encoder for copy operation
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Screenshot Encoder"),
    });

    // Copy texture to buffer
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(std::iter::once(encoder.finish()));

    // Use Arc<Mutex<>> for thread-safe state (required for Send bound on callback)
    let buffer = Arc::new(buffer);
    let buffer_for_async = Arc::clone(&buffer);
    let state = Arc::new(Mutex::new(CaptureState {
        mapping_complete: false,
        mapping_result: None,
    }));
    let state_for_callback = Arc::clone(&state);
    let show_toast = Arc::new(show_toast);
    let show_toast_for_async = Arc::clone(&show_toast);

    // Start the async buffer mapping
    let buffer_slice = buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        if let Ok(mut state) = state_for_callback.lock() {
            state.mapping_result = Some(result);
            state.mapping_complete = true;
        }
    });

    // Spawn async task to poll and wait for mapping completion
    wasm_bindgen_futures::spawn_local(async move {
        // Poll until mapping is complete
        // Use gloo-timers for async delay on web
        loop {
            {
                let state_guard = state.lock().unwrap();
                if state_guard.mapping_complete {
                    break;
                }
            }
            // Small delay to avoid busy-waiting
            gloo_timers::future::TimeoutFuture::new(1).await;
        }

        let result = {
            let mut state_guard = state.lock().unwrap();
            state_guard.mapping_result.take()
        };

        match result {
            Some(Ok(())) => {
                let buffer_slice = buffer_for_async.slice(..);
                let data = buffer_slice.get_mapped_range();

                // Convert from padded buffer to image data
                let mut image_data = Vec::with_capacity((width * height * 4) as usize);
                for row in 0..height {
                    let row_start = (row * bytes_per_row) as usize;
                    let row_data = &data[row_start..row_start + (width * 4) as usize];
                    image_data.extend_from_slice(row_data);
                }

                drop(data);
                buffer_for_async.unmap();

                // Use the platform capture to save the screenshot
                let capture = WebCapture::new();
                match capture.save_screenshot(width, height, &image_data, &fractal_name) {
                    Ok(filename) => {
                        log::info!("Screenshot saved: {}", filename);
                        show_toast_for_async(format!("📸 Screenshot downloaded: {}", filename));
                    }
                    Err(e) => {
                        log::error!("Failed to save screenshot: {}", e);
                        show_toast_for_async(format!("❌ Screenshot failed: {}", e));
                    }
                }
            }
            Some(Err(e)) => {
                log::error!("Buffer mapping error: {:?}", e);
                show_toast_for_async("❌ Screenshot failed: buffer mapping error".to_string());
            }
            None => {
                log::error!("Buffer mapping result not available");
                show_toast_for_async("❌ Screenshot failed: mapping error".to_string());
            }
        }
    });
}

/// Renders a high-resolution image on web.
///
/// This creates temporary textures at the target resolution, renders the fractal
/// with full post-processing pipeline, and triggers an async download.
#[allow(clippy::too_many_arguments)]
pub fn render_high_resolution_web(
    renderer: &Renderer,
    camera: &Camera,
    fractal_params: &FractalParams,
    width: u32,
    height: u32,
    fractal_name: String,
    show_toast: Box<dyn Fn(String) + Send + 'static>,
) {
    log::info!(
        "Starting high-resolution web render at {}x{}...",
        width,
        height
    );

    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let hdr_format = wgpu::TextureFormat::Rgba16Float;

    // Helper to create HDR textures
    let create_hdr_texture = |label: &str| {
        renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: hdr_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    };

    // Create all intermediate textures for post-processing pipeline
    let scene_texture = create_hdr_texture("High-Res Scene");
    let scene_view = scene_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let bright_texture = create_hdr_texture("High-Res Bright");
    let bright_view = bright_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let blur_temp_texture = create_hdr_texture("High-Res Blur Temp");
    let blur_temp_view = blur_temp_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let bloom_texture = create_hdr_texture("High-Res Bloom");
    let bloom_view = bloom_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let composite_texture = create_hdr_texture("High-Res Composite");
    let composite_view = composite_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Create final output texture (8-bit for saving) - use surface format to match copy_pipeline
    let output_format = renderer.config.format;
    let output_texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("High-Res Output"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: output_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Create bind groups for post-processing
    let texture_layout = renderer.copy_pipeline.get_bind_group_layout(0);
    let composite_layout = renderer.composite_pipeline.get_bind_group_layout(0);

    let scene_bind_group = renderer
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("HR Scene BG"),
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&renderer.sampler),
                },
            ],
        });

    let bright_bind_group = renderer
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("HR Bright BG"),
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&bright_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&renderer.sampler),
                },
            ],
        });

    let blur_temp_bind_group = renderer
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("HR Blur Temp BG"),
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&blur_temp_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&renderer.sampler),
                },
            ],
        });

    let composite_bind_group = renderer
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("HR Composite BG"),
            layout: &composite_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&renderer.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&bloom_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&renderer.sampler),
                },
            ],
        });

    let composite_final_bind_group =
        renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("HR Final BG"),
                layout: &texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&composite_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&renderer.sampler),
                    },
                ],
            });

    // Create temporary uniform buffer with correct aspect ratio
    let mut temp_camera = camera.clone();
    temp_camera.aspect = width as f32 / height as f32;

    // Update the main uniform buffer with the temporary camera
    // (we'll restore it after rendering)
    let uniforms = crate::renderer::uniforms::Uniforms::from_camera_and_params(
        &temp_camera,
        fractal_params,
        renderer.start_time.elapsed().as_secs_f32(),
    );
    renderer.queue.write_buffer(
        &renderer.uniform_buffer,
        0,
        bytemuck::cast_slice(&[uniforms]),
    );

    let mut encoder = renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("High-Res Render Encoder"),
        });

    // Pass 1: Render fractal to scene texture
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("HR Fractal Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &scene_view,
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
        });
        pass.set_pipeline(&renderer.render_pipeline);
        pass.set_bind_group(0, &renderer.uniform_bind_group, &[]);
        pass.set_vertex_buffer(0, renderer.vertex_buffer.slice(..));
        pass.draw(0..4, 0..1);
    }

    // Pass 2: Extract bright pixels for bloom
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("HR Bloom Extract"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &bright_view,
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
        });
        pass.set_pipeline(&renderer.bloom_extract_pipeline);
        pass.set_bind_group(0, &scene_bind_group, &[]);
        pass.set_bind_group(1, &renderer.bloom_params_bind_group, &[]);
        pass.set_vertex_buffer(0, renderer.postprocess_vertex_buffer.slice(..));
        pass.draw(0..4, 0..1);
    }

    // Pass 3: Horizontal blur
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("HR Blur H"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &blur_temp_view,
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
        });
        pass.set_pipeline(&renderer.blur_pipeline);
        pass.set_bind_group(0, &bright_bind_group, &[]);
        pass.set_bind_group(1, &renderer.blur_h_params_bind_group, &[]);
        pass.set_vertex_buffer(0, renderer.postprocess_vertex_buffer.slice(..));
        pass.draw(0..4, 0..1);
    }

    // Update blur direction to vertical
    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct BlurUniforms {
        direction: [f32; 2],
        _padding: [f32; 2],
    }
    renderer.queue.write_buffer(
        &renderer.blur_uniform_buffer,
        0,
        bytemuck::cast_slice(&[BlurUniforms {
            direction: [0.0, 1.0],
            _padding: [0.0; 2],
        }]),
    );

    // Pass 4: Vertical blur
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("HR Blur V"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &bloom_view,
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
        });
        pass.set_pipeline(&renderer.blur_pipeline);
        pass.set_bind_group(0, &blur_temp_bind_group, &[]);
        pass.set_bind_group(1, &renderer.blur_v_params_bind_group, &[]);
        pass.set_vertex_buffer(0, renderer.postprocess_vertex_buffer.slice(..));
        pass.draw(0..4, 0..1);
    }

    // Restore blur direction to horizontal for normal rendering
    renderer.queue.write_buffer(
        &renderer.blur_uniform_buffer,
        0,
        bytemuck::cast_slice(&[BlurUniforms {
            direction: [1.0, 0.0],
            _padding: [0.0; 2],
        }]),
    );

    // Pass 5: Composite (scene + bloom + color grading + vignette)
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("HR Composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &composite_view,
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
        });
        pass.set_pipeline(&renderer.composite_pipeline);
        pass.set_bind_group(0, &composite_bind_group, &[]);
        pass.set_bind_group(1, &renderer.composite_params_bind_group, &[]);
        pass.set_vertex_buffer(0, renderer.postprocess_vertex_buffer.slice(..));
        pass.draw(0..4, 0..1);
    }

    // Pass 6: Final copy to 8-bit output
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("HR Final Copy"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output_view,
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
        });
        pass.set_pipeline(&renderer.copy_pipeline);
        pass.set_bind_group(0, &composite_final_bind_group, &[]);
        pass.set_vertex_buffer(0, renderer.postprocess_vertex_buffer.slice(..));
        pass.draw(0..4, 0..1);
    }

    // Create buffer to copy texture to
    let bytes_per_row = (width * 4 + 255) & !255; // Align to 256 bytes
    let buffer_size = (bytes_per_row * height) as wgpu::BufferAddress;

    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("High-Res Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Copy texture to buffer
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &output_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    renderer.queue.submit(std::iter::once(encoder.finish()));

    // Check if we need BGRA to RGBA conversion
    let needs_bgra_swap = output_format == wgpu::TextureFormat::Bgra8Unorm
        || output_format == wgpu::TextureFormat::Bgra8UnormSrgb;

    // Async buffer mapping and download
    let buffer = Arc::new(buffer);
    let buffer_for_async = Arc::clone(&buffer);
    let state = Arc::new(Mutex::new(CaptureState {
        mapping_complete: false,
        mapping_result: None,
    }));
    let state_for_callback = Arc::clone(&state);
    let show_toast = Arc::new(show_toast);
    let show_toast_for_async = Arc::clone(&show_toast);

    let buffer_slice = buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        if let Ok(mut state) = state_for_callback.lock() {
            state.mapping_result = Some(result);
            state.mapping_complete = true;
        }
    });

    wasm_bindgen_futures::spawn_local(async move {
        // Poll until mapping is complete
        loop {
            {
                let state_guard = state.lock().unwrap();
                if state_guard.mapping_complete {
                    break;
                }
            }
            gloo_timers::future::TimeoutFuture::new(1).await;
        }

        let result = {
            let mut state_guard = state.lock().unwrap();
            state_guard.mapping_result.take()
        };

        match result {
            Some(Ok(())) => {
                let buffer_slice = buffer_for_async.slice(..);
                let data = buffer_slice.get_mapped_range();

                // Convert from padded buffer to image data
                let mut image_data = Vec::with_capacity((width * height * 4) as usize);
                for row in 0..height {
                    let row_start = (row * bytes_per_row) as usize;
                    let row_data = &data[row_start..row_start + (width * 4) as usize];
                    image_data.extend_from_slice(row_data);
                }

                drop(data);
                buffer_for_async.unmap();

                // Convert BGRA to RGBA if needed
                if needs_bgra_swap {
                    for pixel in image_data.chunks_exact_mut(4) {
                        pixel.swap(0, 2); // Swap B and R
                    }
                }

                // Generate filename with resolution
                let filename_prefix = format!("{}_{}x{}", fractal_name, width, height);

                // Use the platform capture to save the screenshot
                let capture = WebCapture::new();
                match capture.save_screenshot(width, height, &image_data, &filename_prefix) {
                    Ok(filename) => {
                        log::info!("High-res image saved: {}", filename);
                        show_toast_for_async(format!(
                            "🖼️  High-res image downloaded: {}",
                            filename
                        ));
                    }
                    Err(e) => {
                        log::error!("Failed to save high-res image: {}", e);
                        show_toast_for_async(format!("❌ High-res capture failed: {}", e));
                    }
                }
            }
            Some(Err(e)) => {
                log::error!("Buffer mapping error: {:?}", e);
                show_toast_for_async(
                    "❌ High-res capture failed: buffer mapping error".to_string(),
                );
            }
            None => {
                log::error!("Buffer mapping result not available");
                show_toast_for_async("❌ High-res capture failed: mapping error".to_string());
            }
        }
    });
}
