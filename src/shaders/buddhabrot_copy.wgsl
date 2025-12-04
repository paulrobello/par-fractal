// Buddhabrot Buffer-to-Texture Copy Shader
//
// This compute shader copies data from the atomic storage buffer
// to the R32Uint texture for display by the existing accumulation display pipeline.
//
// The Buddhabrot uses a storage buffer with atomics for thread-safe accumulation,
// but the display pipeline expects a texture, so we copy after each compute pass.

struct CopyUniforms {
    width: u32,
    height: u32,
    _padding: vec2<u32>,
}

@group(0) @binding(0)
var<storage, read> source_buffer: array<u32>;

@group(0) @binding(1)
var dest_texture: texture_storage_2d<r32uint, write>;

@group(0) @binding(2)
var<uniform> uniforms: CopyUniforms;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    // Bounds check
    if (x >= uniforms.width || y >= uniforms.height) {
        return;
    }

    // Read from buffer
    let index = y * uniforms.width + x;
    let value = source_buffer[index];

    // Write to texture
    let coord = vec2<u32>(x, y);
    textureStore(dest_texture, coord, vec4<u32>(value, 0u, 0u, 0u));
}
