@fragment
fn fs_main_3d(input: VertexOutput) -> @location(0) vec4<f32> {
    // 3D Mode
    // UV coordinates are already in NDC space (-1 to 1)
    let ndc_x = input.uv.x;
    let ndc_y = input.uv.y;

    // Unproject a point on the far plane to get initial ray direction
    let far_point = vec4<f32>(ndc_x, ndc_y, 1.0, 1.0);
    var far_world = uniforms.inv_view_proj * far_point;
    far_world = far_world / far_world.w;

    // Initial ray from camera
    let base_ray_origin = uniforms.camera_pos;
    let base_ray_dir = normalize(far_world.xyz - base_ray_origin);

    var final_color: vec3<f32>;

    // Apply depth of field if enabled with multi-sampling
    if (uniforms.depth_of_field == 1u) {
        // Multi-sample DOF - configurable quality vs performance
        let num_samples = uniforms.dof_samples;
        var accumulated_color = vec3<f32>(0.0);

        // Calculate focal point
        let focal_point = base_ray_origin + base_ray_dir * uniforms.dof_focal_length;

        // Calculate camera right and up vectors
        let camera_forward = base_ray_dir;
        let world_up = vec3<f32>(0.0, 1.0, 0.0);
        let camera_right = normalize(cross(camera_forward, world_up));
        let camera_up = cross(camera_right, camera_forward);

        // Take multiple samples
        for (var i = 0u; i < num_samples; i = i + 1u) {
            // Sample aperture with indexed pattern
            let aperture_sample = sample_disk_indexed(input.clip_position.xy, i, num_samples);
            let aperture_offset = (camera_right * aperture_sample.x + camera_up * aperture_sample.y) * uniforms.dof_aperture;

            // Offset ray origin and recalculate direction to focal point
            let ray_origin = base_ray_origin + aperture_offset;
            let ray_dir = normalize(focal_point - ray_origin);

            // Render this ray and accumulate
            accumulated_color = accumulated_color + render_ray(ray_origin, ray_dir, input.uv);
        }

        // Average the samples
        final_color = accumulated_color / f32(num_samples);
    } else {
        // No DOF - single sample
        final_color = render_ray(base_ray_origin, base_ray_dir, input.uv);
    }

    // No post-processing - render raw fractal (post-FX done in multi-pass pipeline)
    return vec4<f32>(final_color, 1.0);
}
