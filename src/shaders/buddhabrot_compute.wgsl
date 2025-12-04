// Buddhabrot Compute Shader
//
// The Buddhabrot is a probability distribution visualization of escape trajectories
// from the Mandelbrot set. Unlike standard Mandelbrot which colors based on escape
// time, Buddhabrot traces the full trajectory of escaping points and accumulates
// hit counts at each pixel the trajectory passes through.
//
// Algorithm:
// 1. Sample random c values in the complex plane
// 2. Test if c escapes the Mandelbrot iteration (|z| > 2)
// 3. If it escapes, trace the full escape trajectory
// 4. For each point in the trajectory, increment the pixel counter
//
// This creates an image resembling a seated Buddha figure, discovered by Melinda Green in 1993.
//
// IMPORTANT: Uses atomic storage buffer for correct concurrent accumulation.
// The non-atomic texture approach loses updates due to read-modify-write races.

struct Uniforms {
    // View transform (matching attractor uniforms for compatibility)
    center_x: f32,
    center_y: f32,
    zoom: f32,
    aspect_ratio: f32,

    // Rendering parameters
    width: u32,
    height: u32,
    iterations_per_frame: u32,
    max_iterations: u32,  // Maximum escape iterations to test

    // Accumulation control
    total_iterations: u32,
    clear_accumulation: u32,
    min_iterations: u32,  // Minimum iterations for a trajectory to be plotted
    _padding: u32,
}

// Atomic storage buffer for thread-safe accumulation
// Each element corresponds to a pixel at index: y * width + x
@group(0) @binding(0)
var<storage, read_write> accumulation_buffer: array<atomic<u32>>;

@group(1) @binding(0)
var<uniform> uniforms: Uniforms;

// Simple hash function for generating pseudo-random numbers
fn hash(seed: u32) -> u32 {
    var s = seed;
    s = s ^ (s >> 16u);
    s = s * 0x85ebca6bu;
    s = s ^ (s >> 13u);
    s = s * 0xc2b2ae35u;
    s = s ^ (s >> 16u);
    return s;
}

// Convert hash to float in [0, 1)
fn hash_to_float(h: u32) -> f32 {
    return f32(h) / 4294967296.0;
}

// Transform world coordinates to screen coordinates
// Uses the same formula as the attractor shader for consistency
fn world_to_screen(world: vec2<f32>) -> vec2<i32> {
    // Rotate 90 degrees clockwise to make Buddha upright
    // (swap x and y, negate new y)
    let rotated = vec2<f32>(world.y, -world.x);

    // Scale factor: zoom * height / 2 gives pixels per world unit
    let scale = uniforms.zoom * f32(uniforms.height) / 2.0;

    // Apply view transform: center and zoom
    let view_x = (rotated.x - uniforms.center_x) * scale;
    let view_y = -(rotated.y - uniforms.center_y) * scale;

    // Convert to screen coordinates (center of screen is origin)
    let screen_x = i32(round(view_x + f32(uniforms.width) / 2.0));
    let screen_y = i32(round(view_y + f32(uniforms.height) / 2.0));

    return vec2<i32>(screen_x, screen_y);
}

// Check if screen coordinates are within bounds
fn is_in_bounds(screen: vec2<i32>) -> bool {
    return screen.x >= 0 && screen.x < i32(uniforms.width) &&
           screen.y >= 0 && screen.y < i32(uniforms.height);
}

// Test if a point c escapes and return the escape iteration count
// Returns 0 if the point does not escape (in the Mandelbrot set)
fn test_escape(c: vec2<f32>) -> u32 {
    var z = vec2<f32>(0.0, 0.0);
    let max_iter = uniforms.max_iterations;

    for (var i = 0u; i < max_iter; i = i + 1u) {
        // z = z^2 + c
        let zx = z.x * z.x - z.y * z.y + c.x;
        let zy = 2.0 * z.x * z.y + c.y;
        z = vec2<f32>(zx, zy);

        // Check for escape (|z|^2 > 4)
        if (z.x * z.x + z.y * z.y > 4.0) {
            return i + 1u;
        }
    }

    return 0u;  // Did not escape
}

// Trace the escape trajectory and accumulate points using atomics
fn trace_trajectory(c: vec2<f32>, escape_iter: u32) {
    var z = vec2<f32>(0.0, 0.0);

    for (var i = 0u; i < escape_iter; i = i + 1u) {
        // z = z^2 + c
        let zx = z.x * z.x - z.y * z.y + c.x;
        let zy = 2.0 * z.x * z.y + c.y;
        z = vec2<f32>(zx, zy);

        // Convert to screen coordinates and accumulate
        let screen = world_to_screen(z);

        if (is_in_bounds(screen)) {
            // Calculate buffer index: y * width + x
            let index = u32(screen.y) * uniforms.width + u32(screen.x);

            // Atomic increment - no race condition!
            atomicAdd(&accumulation_buffer[index], 1u);
        }
    }
}

// Generate a random point in the sampling region
// Buddhabrot works best with samples in the range [-2.5, 1] x [-1.5, 1.5]
// which encompasses the entire Mandelbrot set
fn random_sample_point(seed: u32) -> vec2<f32> {
    let h1 = hash(seed);
    let h2 = hash(seed ^ 0x12345678u);

    // Sample in the range that contains the Mandelbrot set
    // X: [-2.5, 1.0] = width 3.5, centered at -0.75
    // Y: [-1.5, 1.5] = width 3.0, centered at 0
    let x = hash_to_float(h1) * 3.5 - 2.5;
    let y = hash_to_float(h2) * 3.0 - 1.5;

    return vec2<f32>(x, y);
}

// DEBUG: Set to true to visualize sample points instead of trajectories
// This helps verify the coordinate mapping is working correctly
const DEBUG_SHOW_SAMPLES: bool = false;

// Workgroup size: 256 threads per workgroup
// Each thread tests multiple random samples
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let thread_id = global_id.x;

    // Number of samples this thread will test
    // Each sample might generate 0 to max_iterations trajectory points
    let samples_per_thread = uniforms.iterations_per_frame / 256u;

    for (var s = 0u; s < samples_per_thread; s = s + 1u) {
        // Generate unique seed for this sample
        let seed = hash(thread_id ^ (uniforms.total_iterations * 0x9E3779B9u) ^ (s * 0xDEADBEEFu));

        // Generate random sample point
        let c = random_sample_point(seed);

        // DEBUG MODE: Just plot c values to verify coordinate mapping
        if (DEBUG_SHOW_SAMPLES) {
            let screen = world_to_screen(c);
            if (is_in_bounds(screen)) {
                let index = u32(screen.y) * uniforms.width + u32(screen.x);
                atomicAdd(&accumulation_buffer[index], 1u);
            }
            continue;
        }

        // Test if this point escapes
        let escape_iter = test_escape(c);

        // Only trace trajectory if the point escapes AND took enough iterations
        // Short trajectories just trace the escape boundary, long ones create the Buddha
        if (escape_iter >= uniforms.min_iterations) {
            trace_trajectory(c, escape_iter);
        }
    }
}
