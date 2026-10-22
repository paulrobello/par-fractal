// ============================================================================
// Main Fragment Shader
// ============================================================================

@fragment
fn fs_main_2d(input: VertexOutput) -> @location(0) vec4<f32> {
    // 2D Mode
    // Use the aspect ratio supplied by the host (window or capture target)
    let aspect = uniforms.aspect_ratio.x;

    var t: f32;
    var coord: vec2<f32>;

    // Perturbation-theory path (ENH-001): engaged only when the CPU driver
    // has uploaded a reference orbit AND flipped `perturbation_enabled`.
    // Gated by `orbit_len > 0` so a stale enabled flag with an empty buffer
    // can never index into unpopulated storage. Phase A covered Mandelbrot;
    // Phase B step 7 extends the gate to all 2D escape-time types that map
    // to a perturbation recurrence (0=Mandelbrot, 1=Julia, 4=BurningShip,
    // 5=Tricorn).
    let perturb_eligible_type = uniforms.fractal_type == 0u
                             || uniforms.fractal_type == 1u
                             || uniforms.fractal_type == 4u
                             || uniforms.fractal_type == 5u;
    if (uniforms.perturbation_enabled == 1u && uniforms.orbit_len > 0u && perturb_eligible_type) {
        // Per-pixel Δc derived from screen-space offset × pixel→complex scale.
        // The driver sets `delta_c_scale ≈ (2.0/zoom * aspect, 2.0/zoom)` and
        // `delta_c_origin ≈ (0, 0)` (the screen-center delta from the reference).
        // All f32, all small by construction — never subtract two large coords.
        // For Julia this same value is the INITIAL dz (per-pixel z_0 offset
        // from the reference Z_0); the recurrence adds no Δc term.
        let delta_c = uniforms.delta_c_origin + vec2<f32>(
            input.uv.x * uniforms.delta_c_scale.x,
            input.uv.y * uniforms.delta_c_scale.y,
        );
        // Absolute coordinate for position-based color modes (3–6). center_hi is
        // the f32 high word of the view center; at deep zoom the per-pixel delta
        // is below f32 precision relative to center_hi, so coord is approximately
        // constant across the view — fine for the aux color modes, which degrade
        // gracefully. The primary palette path (default) colors by `t`, not coord.
        coord = uniforms.center_hi + delta_c;
        switch uniforms.fractal_type {
            case 0u: { t = mandelbrot_perturb(delta_c); }
            case 1u: { t = julia_perturb(delta_c); }
            case 4u: { t = burning_ship_perturb(delta_c); }
            case 5u: { t = tricorn_perturb(delta_c); }
            default: {
                // Unreachable: `perturb_eligible_type` restricted to {0,1,4,5}.
                // Fall back to the Mandelbrot recurrence as a safe default.
                t = mandelbrot_perturb(delta_c);
            }
        }
    } else if (uniforms.high_precision == 1u && uniforms.fractal_type <= 5u) {
        // Check if high-precision mode is enabled and fractal supports it.
        // Gate covers types 0..=5 (Mandelbrot, Julia, Sierpinski, SierpinskiTriangle,
        // BurningShip, Tricorn). Tricorn (type 5) was previously excluded here, which
        // made tricorn_hp unreachable — the silent dispatch through the final `else`
        // never fired because the gate short-circuited first. (QA-001)
        // High-precision coordinate calculation
        // offset = uv * 2.0 / zoom * aspect (for x) or uv * 2.0 / zoom (for y)
        let offset_x = input.uv.x * 2.0 / uniforms.zoom * aspect;
        let offset_y = input.uv.y * 2.0 / uniforms.zoom;

        // Add offset to high-precision center using double-float arithmetic
        let coord_x = df_add_full(uniforms.center_hi.x, uniforms.center_lo.x, offset_x, 0.0);
        let coord_y = df_add_full(uniforms.center_hi.y, uniforms.center_lo.y, offset_y, 0.0);
        let coord_hi = vec2<f32>(coord_x.x, coord_y.x);
        let coord_lo = vec2<f32>(coord_x.y, coord_y.y);
        coord = coord_hi; // Use high part for color modes

        // Use high-precision fractal functions (QA-030: switch on fractal_type)
        switch uniforms.fractal_type {
            case 0u: { t = mandelbrot_hp(coord_hi, coord_lo); }
            case 1u: { t = julia_hp(coord_hi, coord_lo); }
            case 2u: { t = sierpinski_hp(coord_hi, coord_lo); }
            case 3u: { t = sierpinski_triangle_hp(coord_hi, coord_lo); }
            case 4u: { t = burning_ship_hp(coord_hi, coord_lo); }
            case 5u: { t = tricorn_hp(coord_hi, coord_lo); }
            default: {
                // Defensive fallback (unreachable: gate ensures type in 0..=5).
                // Use standard-precision coord + mandelbrot as a safe default —
                // never silently dispatch through an hp function.
                coord = vec2<f32>(
                    uniforms.center.x + (input.uv.x * 2.0 / uniforms.zoom) * aspect,
                    uniforms.center.y + (input.uv.y * 2.0 / uniforms.zoom)
                );
                t = mandelbrot(coord);
            }
        }
    } else {
        // Standard precision coordinate
        coord = vec2<f32>(
            uniforms.center.x + (input.uv.x * 2.0 / uniforms.zoom) * aspect,
            uniforms.center.y + (input.uv.y * 2.0 / uniforms.zoom)
        );

        // QA-030: switch replaces the 21-arm if/else-if ladder.
        switch uniforms.fractal_type {
            case 0u: { t = mandelbrot(coord); }
            case 1u: { t = julia(coord); }
            case 2u: { t = sierpinski(coord); }
            case 3u: { t = sierpinski_triangle(coord); }
            case 4u: { t = burning_ship(coord); }
            case 5u: { t = tricorn(coord); }
            case 6u: { t = phoenix(coord); }
            case 7u: { t = celtic(coord); }
            case 8u: { t = newton_fractal(coord); }
            case 9u: { t = lyapunov_fractal(coord); }
            case 10u: { t = nova_fractal(coord); }
            case 11u: { t = magnet_fractal(coord); }
            case 12u: { t = collatz_fractal(coord); }
            // Strange Attractors (types 26-31; QA-031: these MUST match the
            // FractalType discriminants — the pinned dispatch test in
            // src/fractal/types.rs cross-checks this switch against the enum)
            case 26u: { t = hopalong_attractor(coord); }
            case 27u: { t = martin_attractor(coord); }
            case 28u: { t = gingerbreadman_attractor(coord); }
            case 29u: { t = chip_attractor(coord); }
            case 30u: { t = quadruptwo_attractor(coord); }
            case 31u: { t = threeply_attractor(coord); }
            default: { t = collatz_fractal(coord); }
        }
    }

    if (t < 0.0) {
        // Sentinel: fractal function returned -1.0 to signal "inside the set"
        // (escape-time interior, sierpinski removed-cell, magnet convergence, etc.).
        // QA-023: previously `t == 0.0`, which conflated a legitimate first-iteration
        // smooth value of exactly 0.0 with the interior sentinel.
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    var color: vec3<f32>;
    if (uniforms.color_mode == 1u) {
        // Iteration visualization (similar to ray steps)
        color = vec3<f32>(t, t * 0.5, 1.0 - t);
    } else if (uniforms.color_mode == 2u) {
        // Grayscale iteration count
        color = vec3<f32>(t);
    } else if (uniforms.color_mode == 3u) {
        // Orbit Trap XYZ - color based on coordinate components using palette
        let xy_sum = abs(fract(coord.x * uniforms.orbit_trap_scale * 2.0)) + abs(fract(coord.y * uniforms.orbit_trap_scale * 2.0));
        let trap_t = fract(xy_sum / 2.0);
        color = get_palette_color(trap_t);
    } else if (uniforms.color_mode == 4u) {
        // Orbit Trap Radial - color based on distance from origin using palette
        let dist = length(coord);
        let radial_t = fract(dist * uniforms.orbit_trap_scale * 3.0);
        color = get_palette_color(radial_t);
    } else if (uniforms.color_mode == 5u || uniforms.color_mode == 6u) {
        // Position-based coloring for 2D
        color = vec3<f32>(abs(fract(coord.x)), abs(fract(coord.y)), abs(fract(coord.x + coord.y)));
    } else {
        // Palette mode (default)
        color = get_palette_color(t);
    }

    // No post-processing - render raw fractal (post-FX done in multi-pass pipeline)
    return vec4<f32>(color, 1.0);

}
