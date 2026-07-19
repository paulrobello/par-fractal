use super::App;
use crate::fractal::RenderMode;

/// ENH-006 Task 3: warmup window before the `--profile-dump` write fires.
/// 120 frames gives the profiler's EMA (α = 0.1) ~4 time-constants to
/// converge and lets the 2-frame-latent staging ring populate
/// `timings_ms`. Compare with `>=`, not `==`: an exact-equality check
/// could be skipped past if `update` ever runs twice on one frame.
const PROFILE_DUMP_FRAME: u32 = 120;

/// Update loop methods
impl App {
    pub fn update(&mut self) {
        let now = web_time::Instant::now();
        let dt = (now - self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        // ARC-018: drain the background GPU scan if it has completed. Cheap
        // (`try_recv`) and runs every frame so the UI list populates the
        // instant the worker thread finishes, without blocking the loop.
        self.poll_gpu_scan();

        // Update FPS counter
        self.frame_count += 1;
        // ENH-006 Task 3: monotonic counter that does NOT reset (unlike
        // `frame_count`, which rolls over every ~0.5 s for the FPS readout).
        // Drives the `--profile-dump` warmup trigger.
        self.total_frame_count = self.total_frame_count.saturating_add(1);
        let fps_elapsed = (now - self.fps_timer).as_secs_f32();
        if fps_elapsed >= 0.5 {
            self.current_fps = self.frame_count as f32 / fps_elapsed;
            self.frame_count = 0;
            self.fps_timer = now;
        }

        // Update frame time for performance overlay
        let frame_time_ms = dt * 1000.0;
        self.ui.update_frame_time(frame_time_ms);
        // ENH-002 v2: record this frame's wall-clock length for the adaptive
        // tile-count estimate (paired with `last_render_scale` captured at the
        // end of the previous render). `dt` is the duration of the previous
        // frame, which is exactly the signal we want.
        self.last_frame_ms = frame_time_ms;
        // ENH-002: mirror convergence so the performance overlay can show
        // whether the fractal pass was skipped (Idle) on this frame.
        self.ui.scene_converged = self.scene_converged;

        // Check for delayed screenshot (CLI option)
        if let Some(delay) = self.screenshot_delay {
            let elapsed = (now - self.start_time).as_secs_f32();
            if !self.screenshot_taken && elapsed >= delay {
                log::info!("Taking screenshot after {:.1}s delay", delay);
                self.save_screenshot = true;
                self.screenshot_taken = true;
            }
        }

        // Check for delayed exit (CLI option)
        if let Some(delay) = self.exit_delay {
            let elapsed = (now - self.start_time).as_secs_f32();
            if elapsed >= delay {
                log::info!("Exiting after {:.1}s delay", delay);
                self.should_exit = true;
            }
        }

        // ENH-006 Task 3: write the EMA-smoothed per-scope GPU timings to the
        // path requested by `--profile-dump`, once the warmup window has
        // elapsed. Idempotent (`profile_dumped` guard) and never forces exit
        // — pair with `--exit-delay` for that. A disabled profiler / empty
        // `timings_ms` still writes the (empty) map, which is the scriptable
        // "feature unavailable" signal; write errors log + continue so a
        // profile-dump failure never crashes the render loop.
        if let Some(path) = self.profile_dump_path.clone() {
            if !self.profile_dumped && self.total_frame_count >= PROFILE_DUMP_FRAME {
                self.profile_dumped = true;
                match serde_yaml::to_string(&self.renderer.profiler.timings_ms) {
                    Ok(yaml) => match std::fs::write(&path, yaml) {
                        Ok(()) => log::info!("wrote GPU profile to {}", path.display()),
                        Err(e) => {
                            log::error!("failed to write GPU profile to {}: {}", path.display(), e)
                        }
                    },
                    Err(e) => log::error!("failed to serialize GPU profile: {}", e),
                }
            }
        }

        // Continuous zoom with shift+left mouse (2D mode)
        if self.shift_pressed
            && self.mouse_pressed
            && self.fractal_params.settings.render_mode == RenderMode::TwoD
        {
            let zoom_speed = 2.0; // Zoom factor per second
            let zoom_factor = (zoom_speed * dt).exp();

            // Zoom at cursor position via the shared seam (cursor_ndc is y-up,
            // [-1,1], before aspect correction).
            let width = self.renderer.size.width as f64;
            let height = self.renderer.size.height as f64;
            let aspect = width / height;
            let norm_x = (self.cursor_pos.0 as f64 / width) * 2.0 - 1.0;
            let norm_y = 1.0 - (self.cursor_pos.1 as f64 / height) * 2.0;
            self.fractal_params
                .zoom_at((norm_x, norm_y), zoom_factor as f64, aspect);
            // ARC-006: user is actively zooming — keep the redraw loop alive.
            self.mark_scene_dirty();
        }

        // Update camera for 3D mode
        if self.fractal_params.settings.render_mode == RenderMode::ThreeD {
            let old_pos = self.camera.position;
            let old_target = self.camera.target;

            // Update camera transition if active
            if self
                .camera_transition
                .update(&mut self.camera, &mut self.camera_controller)
            {
                // Transition is still running, don't allow other camera movements
                self.fractal_params.settings.camera_fov = self.camera.fovy;
            } else if self.camera_transition.active {
                // Transition just finished
                self.camera_transition.active = false;
            } else if self.fractal_params.settings.auto_orbit {
                // Auto-orbit camera around fractal center (only if not transitioning)
                let orbit_center = glam::Vec3::ZERO;
                let to_camera = self.camera.position - orbit_center;

                // Calculate orbit angle based on speed and delta time
                let orbit_angle = self.fractal_params.settings.orbit_speed * dt;

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
                // ARC-006: camera moved — the image changed, keep rendering.
                self.mark_scene_dirty();
            }
        }

        // Auto-save camera position after 1 second of inactivity (native only).
        // Skipped in headless mode: scripted runs (`--screenshot-delay` /
        // `--exit-delay`, e.g. the visual-regression harness) typically load a
        // preset, and `save_camera_settings` persists the WHOLE settings struct
        // — so it would overwrite the user's saved preferences with the
        // preset's state (root-caused 2026-07-18: harness `--preset` runs
        // clobbered the user's `color_mode`).
        #[cfg(not(target_arch = "wasm32"))]
        if !self.is_headless()
            && self.camera_needs_save
            && self.camera_last_moved.elapsed() >= std::time::Duration::from_secs(1)
        {
            self.save_camera_settings();
            self.camera_needs_save = false;
        }

        // Auto-save settings after 1 second of inactivity (native only).
        // Same headless guard — never persist from an automated run.
        #[cfg(not(target_arch = "wasm32"))]
        if !self.is_headless()
            && self.settings_need_save
            && self.settings_last_changed.elapsed() >= std::time::Duration::from_secs(1)
        {
            self.save_all_settings();
            self.settings_need_save = false;
        }

        // Handle high-resolution render request (native only)
        #[cfg(not(target_arch = "wasm32"))]
        if let Some((width, height)) = self.save_hires_render.take() {
            log::info!("Starting high-resolution render at {}x{}...", width, height);
            if let Err(e) = self.render_high_resolution(width, height) {
                log::error!("Failed to render high-resolution image: {}", e);
            } else {
                log::info!("High-resolution render completed!");
            }
        }
        #[cfg(target_arch = "wasm32")]
        if let Some((width, height)) = self.save_hires_render.take() {
            log::info!("Starting high-resolution render at {}x{}...", width, height);
            let fractal_name = self
                .fractal_params
                .settings
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

        // Update palette animation (uses delta time to avoid jumps when changing speed)
        self.fractal_params.settings.palette_offset = self.ui.update_palette_animation(dt);

        // Update LOD system (must be done before renderer.update())
        let camera_forward = (self.camera.target - self.camera.position).normalize();
        self.fractal_params
            .update_lod(self.camera.position, camera_forward, dt);

        // ENH-001: drive the perturbation worker AFTER `update_lod` and BEFORE
        // `renderer.update`. The orbit's spawn-time max_iter and the shader's
        // uniform both derive from `effective_2d_max_iterations`, which reads
        // LOD's `iteration_scale` — so they MUST see the same LOD tick or the
        // async orbit spawns at a pre-tick scale and lands shorter than the
        // shader's loop (the max_iter desync root-caused 2026-07-18: orbit=89
        // vs shader=329). Running it here (post-LOD, pre-render) keeps them in
        // lockstep; the uploaded orbit is still reflected in this frame's
        // uniform write, and `activate_perturbation` pins the shader to the
        // orbit length as a backstop.
        self.update_perturbation();

        // Update renderer uniforms
        // ENH-002 v2: tell the renderer whether tile refinement is in flight so
        // it forces full-resolution scene rendering (tiles + the post chain
        // must both operate on the whole scene_texture).
        self.renderer.refining = self.refine_state.is_some();
        self.renderer.update(&self.camera, &self.fractal_params);
    }

    /// ENH-001 Phase A step 5: drive the perturbation reference-orbit worker.
    ///
    /// Per-frame sequence (no-ops below the gate, cheap):
    /// 1. Record the current view (`center_2d`, `zoom_2d`, effective
    ///    `max_iter`) so the driver can mark the orbit stale on change.
    /// 2. If stale AND eligible (Mandelbrot2D + 2D + log2(zoom) > PERTURBATION_LOG2_GATE) AND
    ///    no worker is in flight, spawn one. Show a "computing…" toast only
    ///    on the transition (the driver returns true exactly once per spawn).
    /// 3. Drain any completed orbit onto the GPU via
    ///    `Renderer::set_reference_orbit` (handles capacity-grow + bind
    ///    group rebuild + upload). The shader's perturbation path activates
    ///    on this same frame.
    /// 4. If the gate is no longer met (zoom dropped, fractal type
    ///    changed), clear the active orbit so the shader falls back to HP.
    ///
    /// The `max_iter` passed to the worker matches the GPU shader's
    /// effective iteration count (zoom bonus + LOD scale) via the shared
    /// `effective_2d_max_iterations` helper — keeping them in lockstep is
    /// the correctness invariant that lets the shader's `orbit_len` always
    /// index a fully-served iteration budget.
    fn update_perturbation(&mut self) {
        use crate::deep_zoom::perturbation_eligible;
        use crate::renderer::uniforms::perturbation_max_iterations;

        let center = self.fractal_params.settings.center_2d;
        let center_precise = self.fractal_params.settings.center_2d_precise.clone();
        let zoom = self.fractal_params.settings.zoom_2d;
        let aspect = self.camera.aspect;
        let fractal_type = self.fractal_params.settings.fractal_type;
        let render_mode = self.fractal_params.settings.render_mode;
        let julia_c = self.fractal_params.settings.julia_c;
        // Deterministic (non-LOD) budget: see `perturbation_max_iterations`.
        // A fixed budget means the orbit computes once (no LOD-driven
        // recompute) and lands deterministically.
        let effective_max_iter = perturbation_max_iterations(&self.fractal_params);

        // (1) Record view → mark stale on change.
        self.perturbation_driver.note_view(
            center,
            center_precise.clone(),
            zoom,
            aspect,
            effective_max_iter,
            fractal_type,
            julia_c,
        );

        // (2) Spawn worker if eligible + stale + idle.
        if self.perturbation_driver.maybe_spawn(
            center,
            center_precise,
            zoom,
            aspect,
            effective_max_iter,
            fractal_type,
            julia_c,
        ) {
            self.ui
                .show_toast("Computing deep-zoom reference…".to_string());
        }

        // (3) Drain a completed orbit onto the GPU. The upload + bind-group
        //     rebuild + uniform population all happen on this thread (the
        //     render-thread owner of device/queue).
        if let Some(orbit) = self.perturbation_driver.poll() {
            self.renderer.set_reference_orbit(&orbit);
            // Re-render once the orbit lands so the perturbation path
            // actually shows up — otherwise ARC-006's render-on-demand might
            // leave the stale HP frame on screen.
            self.mark_scene_dirty();
        }

        // (4) Drop the active orbit when the gate is no longer met so the
        //     shader falls back to the HP / f32 path. The driver itself
        //     stays ready (a new orbit will be computed if the user zooms
        //     back past the gate).
        if !perturbation_eligible(zoom, fractal_type, render_mode)
            && self.renderer.active_orbit.is_some()
        {
            self.renderer.clear_reference_orbit();
            self.mark_scene_dirty();
        }

        // Keep the render loop alive while the worker is in flight so the
        // orbit lands promptly and the "computing…" toast expires normally.
        if self.perturbation_driver.computing {
            self.mark_scene_dirty();
        }
    }
}
