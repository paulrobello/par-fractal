//! Web capture implementation using WebGPU buffer readback

use crate::platform::web::WebCapture;
use crate::platform::Capture;
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
