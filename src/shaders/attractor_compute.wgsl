// Strange Attractor Compute Shader
//
// This shader iterates strange attractor orbits and accumulates hit counts
// into a storage texture. Each workgroup processes multiple independent orbits,
// enabling millions of iterations per frame at 60 FPS.
//
// The accumulation texture stores:
// - R: hit count (incremented each time an orbit lands in a pixel)
// - G: minimum distance to orbit (for anti-aliasing)
// - B: reserved for future use (e.g., color from orbit position)
// - A: reserved

struct Uniforms {
    // Attractor parameters
    param_a: f32,
    param_b: f32,
    param_c: f32,
    param_d: f32,

    // View transform
    center_x: f32,
    center_y: f32,
    zoom: f32,
    aspect_ratio: f32,

    // Rendering parameters
    width: u32,
    height: u32,
    iterations_per_frame: u32,
    attractor_type: u32,

    // Accumulation control
    total_iterations: u32,
    clear_accumulation: u32,
    _padding: vec2<u32>,
}

@group(0) @binding(0)
var accumulation_texture: texture_storage_2d<rgba32float, read_write>;

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

// Hopalong attractor: x' = y - sign(x)*sqrt(|b*x - c|), y' = a - x
fn hopalong_step(x: f32, y: f32, a: f32, b: f32, c: f32) -> vec2<f32> {
    let x_new = y - sign(x) * sqrt(abs(b * x - c));
    let y_new = a - x;
    return vec2<f32>(x_new, y_new);
}

// Hénon attractor: x' = 1 + y - a*x², y' = b*x
fn henon_step(x: f32, y: f32, a: f32, b: f32) -> vec2<f32> {
    let x_new = 1.0 + y - a * x * x;
    let y_new = b * x;
    return vec2<f32>(x_new, y_new);
}

// Martin attractor: x' = y - sin(x), y' = a - x
fn martin_step(x: f32, y: f32, a: f32) -> vec2<f32> {
    let x_new = y - sin(x);
    let y_new = a - x;
    return vec2<f32>(x_new, y_new);
}

// Gingerbreadman: x' = 1 - y + |x|, y' = x
fn gingerbreadman_step(x: f32, y: f32) -> vec2<f32> {
    let x_new = 1.0 - y + abs(x);
    let y_new = x;
    return vec2<f32>(x_new, y_new);
}

// Latoocarfian: x' = sin(y*b) + c*sin(x*b), y' = sin(x*a) + d*sin(y*a)
fn latoocarfian_step(x: f32, y: f32, a: f32, b: f32, c: f32, d: f32) -> vec2<f32> {
    let x_new = sin(y * b) + c * sin(x * b);
    let y_new = sin(x * a) + d * sin(y * a);
    return vec2<f32>(x_new, y_new);
}

// Chip: x' = y - sign(x)*cos(log²(|b*x - c|))*arctan(log²(|c*x - b|)), y' = a - x
fn chip_step(x: f32, y: f32, a: f32, b: f32, c: f32) -> vec2<f32> {
    let log1 = log(max(abs(b * x - c), 0.001));
    let log2 = log(max(abs(c * x - b), 0.001));
    let x_new = y - sign(x) * cos(log1 * log1) * atan(log2 * log2);
    let y_new = a - x;
    return vec2<f32>(x_new, y_new);
}

// Quadruptwo: x' = y - sign(x)*sin(log(|b*x - c|))*arctan(log²(|c*x - b|)), y' = a - x
fn quadruptwo_step(x: f32, y: f32, a: f32, b: f32, c: f32) -> vec2<f32> {
    let log1 = log(max(abs(b * x - c), 0.001));
    let log2 = log(max(abs(c * x - b), 0.001));
    let x_new = y - sign(x) * sin(log1) * atan(log2 * log2);
    let y_new = a - x;
    return vec2<f32>(x_new, y_new);
}

// Threeply: x' = y - sign(x)*|sin(x)*cos(b) + c - x*sin(a+b+c)|, y' = a - x
fn threeply_step(x: f32, y: f32, a: f32, b: f32, c: f32) -> vec2<f32> {
    let term = sin(x) * cos(b) + c - x * sin(a + b + c);
    let x_new = y - sign(x) * abs(term);
    let y_new = a - x;
    return vec2<f32>(x_new, y_new);
}

// Icon fractal with rotational symmetry
fn icon_step(x: f32, y: f32, lambda: f32, alpha: f32, beta: f32, gamma: f32, omega: f32, degree: i32) -> vec2<f32> {
    // Compute z^(degree-2) where z = x + iy
    var zn_real = 1.0;
    var zn_imag = 0.0;
    for (var j = 0; j < degree - 2; j = j + 1) {
        let temp = zn_real * x - zn_imag * y;
        zn_imag = zn_real * y + zn_imag * x;
        zn_real = temp;
    }

    let r_sq = x * x + y * y;
    let x_new = lambda * x + alpha * (x * x - y * y) - beta * (x * zn_real - y * zn_imag) + gamma * r_sq * x + omega;
    let y_new = lambda * y + 2.0 * alpha * x * y - beta * (x * zn_imag + y * zn_real) + gamma * r_sq * y;

    return vec2<f32>(x_new, y_new);
}

// Transform world coordinates to screen coordinates
fn world_to_screen(world: vec2<f32>) -> vec2<i32> {
    // Apply view transform: center and zoom
    let view_x = (world.x - uniforms.center_x) * uniforms.zoom * f32(uniforms.height) / 2.0;
    let view_y = (world.y - uniforms.center_y) * uniforms.zoom * f32(uniforms.height) / 2.0;

    // Convert to screen coordinates (center of screen is origin)
    let screen_x = i32(view_x + f32(uniforms.width) / 2.0);
    let screen_y = i32(view_y + f32(uniforms.height) / 2.0);

    return vec2<i32>(screen_x, screen_y);
}

// Check if screen coordinates are within bounds
fn is_in_bounds(screen: vec2<i32>) -> bool {
    return screen.x >= 0 && screen.x < i32(uniforms.width) &&
           screen.y >= 0 && screen.y < i32(uniforms.height);
}

// Iterate a single attractor step based on type
fn attractor_step(pos: vec2<f32>) -> vec2<f32> {
    let a = uniforms.param_a;
    let b = uniforms.param_b;
    let c = uniforms.param_c;
    let d = uniforms.param_d;

    switch uniforms.attractor_type {
        case 0u: { // Hopalong
            return hopalong_step(pos.x, pos.y, a, b, c);
        }
        case 1u: { // Henon
            return henon_step(pos.x, pos.y, a, b);
        }
        case 2u: { // Martin
            return martin_step(pos.x, pos.y, a);
        }
        case 3u: { // Gingerbreadman
            return gingerbreadman_step(pos.x, pos.y);
        }
        case 4u: { // Latoocarfian
            return latoocarfian_step(pos.x, pos.y, a, b, c, d);
        }
        case 5u: { // Chip
            return chip_step(pos.x, pos.y, a, b, c);
        }
        case 6u: { // Quadruptwo
            return quadruptwo_step(pos.x, pos.y, a, b, c);
        }
        case 7u: { // Threeply
            return threeply_step(pos.x, pos.y, a, b, c);
        }
        case 8u: { // Icon
            return icon_step(pos.x, pos.y, a, b, c, d, 0.0, 5);
        }
        default: {
            return hopalong_step(pos.x, pos.y, a, b, c);
        }
    }
}

// Workgroup size: 256 threads per workgroup
// Each thread processes its own independent orbit
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let thread_id = global_id.x;

    // Generate unique seed for this thread using thread ID and total iterations
    let seed = hash(thread_id ^ (uniforms.total_iterations * 0x9E3779B9u));

    // Initialize orbit position with random starting point
    // Use hash to get different starting points for each thread
    var pos = vec2<f32>(
        hash_to_float(hash(seed)) * 2.0 - 1.0,
        hash_to_float(hash(seed ^ 0x12345678u)) * 2.0 - 1.0
    );

    // Skip transient iterations to reach the attractor
    for (var i = 0u; i < 100u; i = i + 1u) {
        pos = attractor_step(pos);

        // Check for divergence
        if (abs(pos.x) > 100000.0 || abs(pos.y) > 100000.0) {
            // Reset to a new starting point
            let reset_seed = hash(seed ^ i);
            pos = vec2<f32>(
                hash_to_float(hash(reset_seed)) * 2.0 - 1.0,
                hash_to_float(hash(reset_seed ^ 0x87654321u)) * 2.0 - 1.0
            );
        }
    }

    // Number of iterations this thread will compute
    // Distribute iterations_per_frame across all threads
    let iterations_per_thread = uniforms.iterations_per_frame / 256u;

    // Main accumulation loop
    for (var i = 0u; i < iterations_per_thread; i = i + 1u) {
        pos = attractor_step(pos);

        // Check for divergence and reset if needed
        if (abs(pos.x) > 100000.0 || abs(pos.y) > 100000.0) {
            let reset_seed = hash(seed ^ (i * 0xDEADBEEFu));
            pos = vec2<f32>(
                hash_to_float(hash(reset_seed)) * 2.0 - 1.0,
                hash_to_float(hash(reset_seed ^ 0xCAFEBABEu)) * 2.0 - 1.0
            );
            continue;
        }

        // Convert to screen coordinates
        let screen = world_to_screen(pos);

        // Accumulate if within bounds
        if (is_in_bounds(screen)) {
            let coord = vec2<u32>(u32(screen.x), u32(screen.y));

            // Read current value
            let current = textureLoad(accumulation_texture, coord);

            // Increment hit count (R channel)
            let new_value = vec4<f32>(
                current.r + 1.0,
                current.g,
                current.b,
                current.a
            );

            // Write back
            textureStore(accumulation_texture, coord, new_value);
        }
    }
}
