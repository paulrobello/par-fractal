// ============================================================================
// Ray Rendering Helper (for DOF multi-sampling)
// ============================================================================

fn render_ray(ray_origin: vec3<f32>, ray_dir: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    // Ray march
    let result = ray_march(ray_origin, ray_dir);

    if (!result.hit) {
        // Background - use fog color if fog is enabled, otherwise gradient
        var bg: vec3<f32>;
        if (uniforms.fog_enabled != 0u) {
            bg = uniforms.fog_color;
        } else {
            let t = (uv.y + 1.0) * 0.5;
            bg = mix(vec3<f32>(0.1, 0.1, 0.15), vec3<f32>(0.0, 0.0, 0.0), t);
        }
        return bg;
    }

    let pos = result.position;
    let normal = calculate_normal(pos);
    let view_dir = normalize(ray_origin - pos);

    // Lighting
    let light_dir = calculate_light_direction();

    // Calculate shadow with small offset to avoid self-intersection
    // Use 3x min_distance as offset - just enough to clear the surface
    let shadow_offset = uniforms.min_distance * 3.0;
    let shadow = calculate_soft_shadow(pos + normal * shadow_offset, light_dir, shadow_offset);

    // Calculate AO
    let ao = calculate_ao(pos, normal);

    // Determine albedo based on material and color mode
    var albedo: vec3<f32>;
    var final_color: vec3<f32>;
    var apply_shading = true;

    // Floor always uses checkered pattern regardless of color mode
    if (result.material_id == 1u) {
        albedo = checkered(pos);
    } else {
        // Apply color mode visualization to fractal only
        if (uniforms.color_mode == 1u) {
            // Ray Steps visualization
            let step_t = f32(result.steps) / f32(uniforms.max_steps);
            albedo = vec3<f32>(step_t, step_t * 0.5, 1.0 - step_t);
        } else if (uniforms.color_mode == 2u) {
            // Normals visualization (no shading)
            final_color = normal * 0.5 + 0.5;
            apply_shading = false;
        } else if (uniforms.color_mode == 3u) {
            // Orbit Trap XYZ - color based on position components using palette
            let xyz_sum = abs(fract(pos.x * uniforms.orbit_trap_scale * 1.5)) + abs(fract(pos.y * uniforms.orbit_trap_scale * 1.5)) + abs(fract(pos.z * uniforms.orbit_trap_scale * 1.5));
            let trap_t = fract(xyz_sum / 3.0);
            albedo = get_palette_color(trap_t);
        } else if (uniforms.color_mode == 4u) {
            // Orbit Trap Radial - color based on distance patterns using palette
            let dist = length(pos);
            let radial_t = fract(dist * uniforms.orbit_trap_scale * 2.0);
            albedo = get_palette_color(radial_t);
        } else if (uniforms.color_mode == 5u) {
            // World Position visualization
            albedo = abs(fract(pos * 0.5));
        } else if (uniforms.color_mode == 6u) {
            // Local Position visualization (relative to fractal center)
            albedo = abs(fract(pos * 2.0));
        } else if (uniforms.color_mode == 7u) {
            // Ambient Occlusion only (no shading)
            final_color = vec3<f32>(ao);
            apply_shading = false;
        } else if (uniforms.color_mode == 8u) {
            // Per-Channel mode - map different sources to R,G,B independently
            let iter_value = f32(result.steps) / f32(uniforms.max_steps);
            let dist_value = clamp(result.distance * 10.0, 0.0, 1.0);

            // Get value for each channel based on source
            var r_val = 0.0;
            if (uniforms.channel_r == 0u) { r_val = iter_value; }  // Iterations
            else if (uniforms.channel_r == 1u) { r_val = dist_value; }  // Distance
            else if (uniforms.channel_r == 2u) { r_val = abs(fract(pos.x)); }  // PositionX
            else if (uniforms.channel_r == 3u) { r_val = abs(fract(pos.y)); }  // PositionY
            else if (uniforms.channel_r == 4u) { r_val = abs(fract(pos.z)); }  // PositionZ
            else if (uniforms.channel_r == 5u) { r_val = abs(normal.x); }  // Normal
            else if (uniforms.channel_r == 6u) { r_val = ao; }  // AO
            else if (uniforms.channel_r == 7u) { r_val = 0.0; }  // Constant

            var g_val = 0.0;
            if (uniforms.channel_g == 0u) { g_val = iter_value; }
            else if (uniforms.channel_g == 1u) { g_val = dist_value; }
            else if (uniforms.channel_g == 2u) { g_val = abs(fract(pos.x)); }
            else if (uniforms.channel_g == 3u) { g_val = abs(fract(pos.y)); }
            else if (uniforms.channel_g == 4u) { g_val = abs(fract(pos.z)); }
            else if (uniforms.channel_g == 5u) { g_val = abs(normal.y); }
            else if (uniforms.channel_g == 6u) { g_val = ao; }
            else if (uniforms.channel_g == 7u) { g_val = 0.0; }

            var b_val = 0.0;
            if (uniforms.channel_b == 0u) { b_val = iter_value; }
            else if (uniforms.channel_b == 1u) { b_val = dist_value; }
            else if (uniforms.channel_b == 2u) { b_val = abs(fract(pos.x)); }
            else if (uniforms.channel_b == 3u) { b_val = abs(fract(pos.y)); }
            else if (uniforms.channel_b == 4u) { b_val = abs(fract(pos.z)); }
            else if (uniforms.channel_b == 5u) { b_val = abs(normal.z); }
            else if (uniforms.channel_b == 6u) { b_val = ao; }
            else if (uniforms.channel_b == 7u) { b_val = 0.0; }

            albedo = vec3<f32>(r_val, g_val, b_val);
        } else if (uniforms.color_mode == 9u) {
            // Distance Field visualization - show complexity of distance field
            // Use ray marching steps as proxy for distance field tightness
            // More steps = tighter/more complex distance field
            let steps_t = f32(result.steps) / f32(uniforms.max_steps);
            // Use log scale to emphasize differences in lower step counts
            let dist_t = clamp(log2(1.0 + steps_t * 15.0) / 4.0, 0.0, 1.0);
            final_color = vec3<f32>(dist_t, dist_t * 0.5, 1.0 - dist_t);
            apply_shading = false;
        } else if (uniforms.color_mode == 10u) {
            // Depth visualization - visualize distance from camera
            let depth = length(pos - uniforms.camera_pos);
            // Use a reasonable depth range (0-20 units) instead of max_distance
            let depth_t = clamp(depth / 20.0, 0.0, 1.0);
            final_color = vec3<f32>(1.0 - depth_t, depth_t * 0.5, depth_t);
            apply_shading = false;
        } else if (uniforms.color_mode == 11u) {
            // Convergence visualization - escape time (mainly for 2D fractals)
            let conv_t = f32(result.steps) / f32(max(uniforms.max_iterations, uniforms.max_steps));
            final_color = vec3<f32>(conv_t, 1.0 - conv_t, conv_t * (1.0 - conv_t) * 4.0);
            apply_shading = false;
        } else if (uniforms.color_mode == 12u) {
            // Lighting Only - show only the lighting (no fractal coloring)
            albedo = vec3<f32>(0.8, 0.8, 0.8);  // Neutral gray albedo
        } else if (uniforms.color_mode == 13u) {
            // Shadow Map visualization
            final_color = vec3<f32>(shadow);
            apply_shading = false;
        } else if (uniforms.color_mode == 14u) {
            // Camera Distance (LOD Zones) - shows distance from camera using LOD zone colors
            let distance = length(pos - uniforms.camera_pos);
            final_color = get_lod_debug_color(distance);
            apply_shading = false;
        } else if (uniforms.color_mode == 15u) {
            // Distance Grayscale - visualize raw distance from camera as brightness
            let distance = length(pos - uniforms.camera_pos);
            // Map distance 0-100 units to grayscale
            let gray = clamp(distance / 100.0, 0.0, 1.0);
            final_color = vec3<f32>(gray);
            apply_shading = false;
        } else {
            // Standard palette mode (0)
            let color_t = f32(result.steps) / f32(uniforms.max_steps);
            albedo = get_palette_color(color_t);
        }
    }

    // Apply shading to modes that need it
    if (apply_shading) {
        if (uniforms.shading_model == 0u) {
            final_color = blinn_phong(normal, view_dir, light_dir, albedo);
        } else {
            final_color = pbr(normal, view_dir, light_dir, albedo, uniforms.metallic, uniforms.roughness);
        }

        // Apply shadow
        if (uniforms.soft_shadows != 0u) {
            final_color = final_color * shadow;
        }

        // Apply ambient occlusion
        if (uniforms.ambient_occlusion != 0u) {
            final_color = final_color * ao;
        }
    }

    // Apply screen-space reflections for floor
    if (result.material_id == 1u && uniforms.floor_reflections != 0u) {
        // Reflect the view direction about the floor normal (upward)
        let floor_normal = vec3<f32>(0.0, 1.0, 0.0);
        let reflect_dir = reflect(-view_dir, floor_normal);

        // Cast a reflection ray from the floor position
        let reflect_origin = pos + floor_normal * uniforms.min_distance * 3.0;
        let reflect_result = ray_march(reflect_origin, reflect_dir);

        if (reflect_result.hit && reflect_result.material_id == 0u) {
            // We hit the fractal - calculate its color
            let reflect_pos = reflect_result.position;
            let reflect_normal = calculate_normal(reflect_pos);
            let reflect_view_dir = normalize(reflect_origin - reflect_pos);
            let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));

            // Get fractal albedo based on color mode
            var reflect_albedo: vec3<f32>;
            if (uniforms.color_mode == 1u) {
                let step_t = f32(reflect_result.steps) / f32(uniforms.max_steps);
                reflect_albedo = vec3<f32>(step_t, step_t * 0.5, 1.0 - step_t);
            } else {
                let color_t = f32(reflect_result.steps) / f32(uniforms.max_steps);
                reflect_albedo = get_palette_color(color_t);
            }

            // Apply lighting to reflection
            var reflect_color: vec3<f32>;
            if (uniforms.shading_model == 0u) {
                reflect_color = blinn_phong(reflect_normal, reflect_view_dir, light_dir, reflect_albedo);
            } else {
                reflect_color = pbr(reflect_normal, reflect_view_dir, light_dir, reflect_albedo, uniforms.metallic, uniforms.roughness);
            }

            // Apply AO and shadows to reflection
            if (uniforms.ambient_occlusion != 0u) {
                let reflect_ao = calculate_ao(reflect_pos, reflect_normal);
                reflect_color = reflect_color * reflect_ao;
            }
            if (uniforms.soft_shadows != 0u) {
                let shadow_offset = uniforms.min_distance * 3.0;
                let reflect_shadow = calculate_soft_shadow(reflect_pos + reflect_normal * shadow_offset, light_dir, shadow_offset);
                reflect_color = reflect_color * reflect_shadow;
            }

            // Fresnel effect - more reflection at grazing angles
            let fresnel = pow(1.0 - abs(dot(view_dir, floor_normal)), 2.0);
            // Scale reflection strength by user parameter (0.0 = no reflection, 1.0 = full strength)
            // Base range is 0.3-0.7, scaled by floor_reflection_strength
            let base_strength = mix(0.3, 0.7, fresnel);
            let reflection_strength = base_strength * uniforms.floor_reflection_strength;

            // Mix floor color with reflection
            final_color = mix(final_color, reflect_color, reflection_strength);
        }
    }

    // Apply fog if enabled
    if (uniforms.fog_enabled != 0u) {
        let dist = length(pos - ray_origin);
        var fog_factor: f32;

        if (uniforms.fog_mode == 0u) {
            // Linear fog
            fog_factor = clamp(dist * uniforms.fog_density, 0.0, 1.0);
        } else if (uniforms.fog_mode == 1u) {
            // Exponential fog
            fog_factor = 1.0 - exp(-uniforms.fog_density * dist);
        } else {
            // Quadratic fog
            fog_factor = 1.0 - exp(-uniforms.fog_density * dist * dist);
        }

        fog_factor = clamp(fog_factor, 0.0, 1.0);
        final_color = mix(final_color, uniforms.fog_color, fog_factor);
    }

    // LOD debug visualization removed - use Color Mode "Camera Distance LOD" instead

    return final_color;
}
