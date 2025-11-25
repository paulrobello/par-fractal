use super::App;
use crate::fractal::RenderMode;

/// Update loop methods
impl App {
    pub fn update(&mut self) {
        let now = web_time::Instant::now();
        let dt = (now - self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        // Update FPS counter
        self.frame_count += 1;
        let fps_elapsed = (now - self.fps_timer).as_secs_f32();
        if fps_elapsed >= 0.5 {
            self.current_fps = self.frame_count as f32 / fps_elapsed;
            self.frame_count = 0;
            self.fps_timer = now;
        }

        // Update frame time for performance overlay
        let frame_time_ms = dt * 1000.0;
        self.ui.update_frame_time(frame_time_ms);

        // Check for delayed screenshot (CLI option)
        if let Some(delay) = self.screenshot_delay {
            let elapsed = (now - self.start_time).as_secs_f32();
            if !self.screenshot_taken && elapsed >= delay {
                println!("Taking screenshot after {:.1}s delay", delay);
                self.save_screenshot = true;
                self.screenshot_taken = true;
            }
        }

        // Check for delayed exit (CLI option)
        if let Some(delay) = self.exit_delay {
            let elapsed = (now - self.start_time).as_secs_f32();
            if elapsed >= delay {
                println!("Exiting after {:.1}s delay", delay);
                self.should_exit = true;
            }
        }

        // Continuous zoom with shift+left mouse (2D mode)
        if self.shift_pressed
            && self.mouse_pressed
            && self.fractal_params.render_mode == RenderMode::TwoD
        {
            let zoom_speed = 2.0; // Zoom factor per second
            let zoom_factor = (zoom_speed * dt).exp();

            // Zoom at cursor position
            let width = self.renderer.size.width as f64;
            let height = self.renderer.size.height as f64;
            let aspect = width / height;
            let norm_x = (self.cursor_pos.0 as f64 / width) * 2.0 - 1.0;
            let norm_y = 1.0 - (self.cursor_pos.1 as f64 / height) * 2.0;

            let zoom = self.fractal_params.zoom_2d as f64;
            let fractal_x = self.fractal_params.center_2d[0] + (norm_x * 2.0 / zoom) * aspect;
            let fractal_y = self.fractal_params.center_2d[1] + norm_y * 2.0 / zoom;

            self.fractal_params.zoom_2d *= zoom_factor;

            let new_zoom = self.fractal_params.zoom_2d as f64;
            let new_fractal_x =
                self.fractal_params.center_2d[0] + (norm_x * 2.0 / new_zoom) * aspect;
            let new_fractal_y = self.fractal_params.center_2d[1] + norm_y * 2.0 / new_zoom;

            self.fractal_params.center_2d[0] += fractal_x - new_fractal_x;
            self.fractal_params.center_2d[1] += fractal_y - new_fractal_y;
        }

        // Update camera for 3D mode
        if self.fractal_params.render_mode == RenderMode::ThreeD {
            let old_pos = self.camera.position;
            let old_target = self.camera.target;

            // Update camera transition if active
            if self
                .camera_transition
                .update(&mut self.camera, &mut self.camera_controller)
            {
                // Transition is still running, don't allow other camera movements
                self.fractal_params.camera_fov = self.camera.fovy;
            } else if self.camera_transition.active {
                // Transition just finished
                self.camera_transition.active = false;
            } else if self.fractal_params.auto_orbit {
                // Auto-orbit camera around fractal center (only if not transitioning)
                let orbit_center = glam::Vec3::ZERO;
                let to_camera = self.camera.position - orbit_center;

                // Calculate orbit angle based on speed and delta time
                let orbit_angle = self.fractal_params.orbit_speed * dt;

                // Rotate around Y axis
                let rotation = glam::Quat::from_axis_angle(glam::Vec3::Y, orbit_angle);
                let new_offset = rotation * to_camera;

                self.camera.position = orbit_center + new_offset;
                self.camera.target = orbit_center;

                // Update controller to match the new orientation
                self.camera_controller
                    .point_at_target(self.camera.position, self.camera.target);

                self.was_auto_orbiting = true;
            } else {
                // On transition frame (just exited auto-orbit), sync controller one final time
                // This ensures perfect alignment before manual control resumes
                if self.was_auto_orbiting {
                    self.camera_controller
                        .point_at_target(self.camera.position, self.camera.target);
                    self.was_auto_orbiting = false;
                    // Don't call update_camera() this frame - let the sync settle
                } else {
                    // Normal manual camera control
                    self.camera_controller.update_camera(&mut self.camera, dt);
                }
            }

            // Check if camera moved
            if old_pos != self.camera.position || old_target != self.camera.target {
                self.camera_last_moved = web_time::Instant::now();
                self.camera_needs_save = true;
            }
        }

        // Auto-save camera position after 1 second of inactivity (native only)
        #[cfg(not(target_arch = "wasm32"))]
        if self.camera_needs_save
            && self.camera_last_moved.elapsed() >= std::time::Duration::from_secs(1)
        {
            self.save_camera_settings();
            self.camera_needs_save = false;
        }

        // Auto-save settings after 1 second of inactivity (native only)
        #[cfg(not(target_arch = "wasm32"))]
        if self.settings_need_save
            && self.settings_last_changed.elapsed() >= std::time::Duration::from_secs(1)
        {
            self.save_all_settings();
            self.settings_need_save = false;
        }

        // Handle high-resolution render request (native only)
        #[cfg(not(target_arch = "wasm32"))]
        if let Some((width, height)) = self.save_hires_render.take() {
            println!("Starting high-resolution render at {}x{}...", width, height);
            if let Err(e) = self.render_high_resolution(width, height) {
                eprintln!("Failed to render high-resolution image: {}", e);
            } else {
                println!("High-resolution render completed!");
            }
        }
        #[cfg(target_arch = "wasm32")]
        if let Some((width, height)) = self.save_hires_render.take() {
            log::info!("Starting high-resolution render at {}x{}...", width, height);
            let fractal_name = self
                .fractal_params
                .fractal_type
                .filename_safe_name()
                .to_string();
            let show_toast: Box<dyn Fn(String) + Send + 'static> = Box::new(move |msg: String| {
                log::info!("{}", msg);
            });
            super::capture_web::render_high_resolution_web(
                &self.renderer,
                &self.camera,
                &self.fractal_params,
                width,
                height,
                fractal_name,
                show_toast,
            );
        }

        // Update palette animation
        let elapsed_time = self.start_time.elapsed().as_secs_f32();
        self.fractal_params.palette_offset = self.ui.get_palette_animation_offset(elapsed_time);

        // Update LOD system (must be done before renderer.update())
        let camera_forward = (self.camera.target - self.camera.position).normalize();
        self.fractal_params
            .update_lod(self.camera.position, camera_forward, dt);

        // Update renderer uniforms
        self.renderer.update(&self.camera, &self.fractal_params);
    }
}
