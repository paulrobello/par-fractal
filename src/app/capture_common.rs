//! Shared GPU-readback helpers for the native (`capture.rs`) and web
//! (`capture_web.rs`) capture paths (ARC-014 dedup).
//!
//! Both targets perform the same texture→buffer readback and row-padding
//! strip; only the *wait* (native blocks on `device.poll(Wait)`; web polls an
//! async `map_async` callback) and the *save* (filesystem path vs platform
//! `Capture` download) differ. Those stay per-target; the pure wgpu setup and
//! post-processing live here so the two modules stop carrying ~5 near-identical
//! copies of it.
//!
//! Compiled on every target — these are plain `wgpu` calls with no platform
//! divergence.

/// Create a `MAP_READ` staging buffer, encode a whole-texture → buffer copy,
/// and submit it. Returns the buffer (to map) and the 256-byte-aligned
/// `bytes_per_row` the data was laid out with (needed to strip padding).
///
/// `buffer_label` / `encoder_label` carry the per-call diagnostic labels so the
/// native and web paths keep their existing "Screenshot"/"Video Frame"/"High-Res"
/// debug names.
pub(super) fn copy_texture_to_readback_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    buffer_label: &str,
    encoder_label: &str,
) -> (wgpu::Buffer, u32) {
    // 256-byte alignment is wgpu's COPY_BYTES_PER_ROW_ALIGNMENT on every backend.
    let bytes_per_row = (width * 4 + 255) & !255;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(buffer_label),
        size: (bytes_per_row * height) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some(encoder_label),
    });
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
    (buffer, bytes_per_row)
}

/// Strip 256-byte row padding from a mapped readback, yielding tightly-packed
/// RGBA8 (`width * height * 4` bytes). Does not touch channel order — call
/// [`swap_bgra_channels`] afterwards if the source surface is BGRA.
pub(super) fn strip_row_padding(
    padded: &[u8],
    width: u32,
    height: u32,
    bytes_per_row: u32,
) -> Vec<u8> {
    let mut out = Vec::with_capacity((width * height * 4) as usize);
    let row_bytes = (width * 4) as usize;
    for row in 0..height {
        let start = (row * bytes_per_row) as usize;
        out.extend_from_slice(&padded[start..start + row_bytes]);
    }
    out
}

/// Swap the B and R channels of every RGBA pixel in place when `format` is a
/// BGRA surface format (macOS/Windows `Bgra8Unorm[Srgb]`). RGBA surfaces are a
/// no-op. Mirrors what `fs_copy`/the composited surface write.
pub(super) fn swap_bgra_channels(rgba: &mut [u8], format: wgpu::TextureFormat) {
    if matches!(
        format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    ) {
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
    }
}
