use super::App;
use crate::fractal::{FractalType, RenderMode};
use winit::event::*;
use winit::keyboard::{KeyCode, PhysicalKey};

/// Input handling methods
impl App {
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        // Let egui handle input first
        let response = self.egui_state.on_window_event(self.window.as_ref(), event);
        if response.consumed {
            return true;
        }

        // For touch events, we rely solely on egui's consumed flag (checked above)
        // Don't check egui_wants_pointer for touches because:
        // 1. egui-winit may not update pointer position from Touch events on web
        // 2. is_pointer_over_area() would return stale/incorrect data
        // 3. If egui consumed the touch, we already returned early
        // 4. If not consumed, the touch is for fractal interaction
        let is_touch = matches!(event, WindowEvent::Touch(_));

        // For mouse events, check if egui wants pointer input
        // This prevents camera movement during UI interactions like window resizing
        let egui_blocks_mouse = if !is_touch {
            let ctx = self.egui_state.egui_ctx();
            ctx.wants_pointer_input()
                || ctx.is_pointer_over_area()
                || ctx.is_using_pointer()
                || ctx.dragged_id().is_some()
        } else {
            false // Touch events not blocked by egui (already checked consumed flag)
        };

        // Track shift key for continuous zoom
        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight),
                    state,
                    ..
                },
            ..
        } = event
        {
            self.shift_pressed = *state == ElementState::Pressed;
        }

        // Handle keyboard shortcuts
        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(keycode),
                    state: ElementState::Pressed,
                    repeat: false,
                    ..
                },
            ..
        } = event
        {
            match keycode {
                KeyCode::KeyH => {
                    self.ui.show_ui = !self.ui.show_ui;
                    return true;
                }
                KeyCode::KeyF => {
                    self.ui.show_fps = !self.ui.show_fps;
                    self.ui.ui_state.show_fps = self.ui.show_fps;
                    return true;
                }
                KeyCode::KeyV => {
                    self.ui.show_performance_overlay = !self.ui.show_performance_overlay;
                    return true;
                }
                KeyCode::KeyR => {
                    self.reset_view();
                    return true;
                }
                // 2D Fractals (1-7)
                KeyCode::Digit1 => {
                    self.fractal_params
                        .switch_fractal(FractalType::Mandelbrot2D);
                    return true;
                }
                KeyCode::Digit2 => {
                    self.fractal_params.switch_fractal(FractalType::Julia2D);
                    return true;
                }
                KeyCode::Digit3 => {
                    self.fractal_params
                        .switch_fractal(FractalType::Sierpinski2D);
                    return true;
                }
                KeyCode::Digit4 => {
                    self.fractal_params
                        .switch_fractal(FractalType::BurningShip2D);
                    return true;
                }
                KeyCode::Digit5 => {
                    self.fractal_params.switch_fractal(FractalType::Tricorn2D);
                    return true;
                }
                KeyCode::Digit6 => {
                    self.fractal_params.switch_fractal(FractalType::Phoenix2D);
                    return true;
                }
                KeyCode::Digit7 => {
                    self.fractal_params.switch_fractal(FractalType::Celtic2D);
                    return true;
                }
                KeyCode::Digit8 => {
                    self.fractal_params.switch_fractal(FractalType::Newton2D);
                    return true;
                }
                KeyCode::Digit9 => {
                    self.fractal_params.switch_fractal(FractalType::Lyapunov2D);
                    return true;
                }
                KeyCode::Digit0 => {
                    self.fractal_params.switch_fractal(FractalType::Nova2D);
                    return true;
                }
                // Magnet and Collatz: use UI (no hotkey due to limited keys)
                // 3D Fractals (F1-F9)
                KeyCode::F1 => {
                    self.fractal_params
                        .switch_fractal(FractalType::Mandelbulb3D);
                    self.reset_view();
                    return true;
                }
                KeyCode::F2 => {
                    self.fractal_params
                        .switch_fractal(FractalType::MengerSponge3D);
                    self.reset_view();
                    return true;
                }
                KeyCode::F3 => {
                    self.fractal_params
                        .switch_fractal(FractalType::SierpinskiPyramid3D);
                    self.reset_view();
                    return true;
                }
                KeyCode::F4 => {
                    self.fractal_params.switch_fractal(FractalType::JuliaSet3D);
                    self.reset_view();
                    return true;
                }
                KeyCode::F5 => {
                    self.fractal_params.switch_fractal(FractalType::Mandelbox3D);
                    self.reset_view();
                    return true;
                }
                KeyCode::F6 => {
                    self.fractal_params
                        .switch_fractal(FractalType::OctahedralIFS3D);
                    self.reset_view();
                    return true;
                }
                KeyCode::F7 => {
                    self.fractal_params
                        .switch_fractal(FractalType::IcosahedralIFS3D);
                    self.reset_view();
                    return true;
                }
                KeyCode::F8 => {
                    self.fractal_params
                        .switch_fractal(FractalType::ApollonianGasket3D);
                    self.reset_view();
                    return true;
                }
                KeyCode::F9 => {
                    self.fractal_params.switch_fractal(FractalType::Kleinian3D);
                    self.reset_view();
                    return true;
                }
                KeyCode::F10 => {
                    self.fractal_params
                        .switch_fractal(FractalType::HybridMandelbulbJulia3D);
                    self.reset_view();
                    return true;
                }
                // QuaternionCubic3D: accessible via UI only (no F12 - reserved for screenshot)
                KeyCode::KeyP => {
                    if self.shift_pressed {
                        // Shift+P: Cycle procedural palette
                        use crate::fractal::ProceduralPalette;
                        let all_options: Vec<ProceduralPalette> =
                            std::iter::once(ProceduralPalette::None)
                                .chain(ProceduralPalette::ALL.iter().copied())
                                .collect();
                        let current_idx = all_options
                            .iter()
                            .position(|p| *p == self.fractal_params.procedural_palette)
                            .unwrap_or(0);
                        let next_idx = (current_idx + 1) % all_options.len();
                        self.fractal_params.procedural_palette = all_options[next_idx];
                        self.ui.show_toast(format!(
                            "Procedural: {}",
                            self.fractal_params.procedural_palette.name()
                        ));
                    } else {
                        // P: Cycle static palette
                        self.fractal_params.next_palette();
                        self.ui
                            .show_toast(format!("Palette: {}", self.fractal_params.palette.name));
                    }
                    return true;
                }
                KeyCode::F12 => {
                    self.save_screenshot = true;
                    println!("Screenshot queued...");
                    return true;
                }
                KeyCode::KeyO => {
                    self.fractal_params.auto_orbit = !self.fractal_params.auto_orbit;
                    println!(
                        "Auto-orbit: {}",
                        if self.fractal_params.auto_orbit {
                            "ON"
                        } else {
                            "OFF"
                        }
                    );
                    return true;
                }
                KeyCode::BracketLeft => {
                    self.fractal_params.orbit_speed =
                        (self.fractal_params.orbit_speed - 0.1).max(0.1);
                    println!("Orbit speed: {:.2}", self.fractal_params.orbit_speed);
                    return true;
                }
                KeyCode::BracketRight => {
                    self.fractal_params.orbit_speed =
                        (self.fractal_params.orbit_speed + 0.1).min(5.0);
                    println!("Orbit speed: {:.2}", self.fractal_params.orbit_speed);
                    return true;
                }
                KeyCode::Minus => {
                    match self.fractal_params.render_mode {
                        RenderMode::TwoD => {
                            self.fractal_params.max_iterations = self
                                .fractal_params
                                .max_iterations
                                .saturating_sub(32)
                                .max(32);
                            println!("Max iterations: {}", self.fractal_params.max_iterations);
                        }
                        RenderMode::ThreeD => {
                            self.fractal_params.max_steps =
                                self.fractal_params.max_steps.saturating_sub(10).max(30);
                            println!("Max steps: {}", self.fractal_params.max_steps);
                        }
                    }
                    return true;
                }
                KeyCode::Equal => {
                    match self.fractal_params.render_mode {
                        RenderMode::TwoD => {
                            self.fractal_params.max_iterations =
                                (self.fractal_params.max_iterations + 32).min(2048);
                            println!("Max iterations: {}", self.fractal_params.max_iterations);
                        }
                        RenderMode::ThreeD => {
                            self.fractal_params.max_steps =
                                (self.fractal_params.max_steps + 10).min(500);
                            println!("Max steps: {}", self.fractal_params.max_steps);
                        }
                    }
                    return true;
                }
                KeyCode::Comma => {
                    self.fractal_params.power = (self.fractal_params.power - 0.5).max(2.0);
                    println!("Power: {:.1}", self.fractal_params.power);
                    return true;
                }
                KeyCode::Period => {
                    self.fractal_params.power = (self.fractal_params.power + 0.5).min(16.0);
                    println!("Power: {:.1}", self.fractal_params.power);
                    return true;
                }
                KeyCode::KeyL => {
                    self.fractal_params.ambient_occlusion = !self.fractal_params.ambient_occlusion;
                    println!(
                        "Ambient Occlusion: {}",
                        if self.fractal_params.ambient_occlusion {
                            "ON"
                        } else {
                            "OFF"
                        }
                    );
                    return true;
                }
                KeyCode::KeyT => {
                    self.fractal_params.depth_of_field = !self.fractal_params.depth_of_field;
                    println!(
                        "Depth of Field: {}",
                        if self.fractal_params.depth_of_field {
                            "ON"
                        } else {
                            "OFF"
                        }
                    );
                    return true;
                }
                KeyCode::KeyG => {
                    self.fractal_params.show_floor = !self.fractal_params.show_floor;
                    println!(
                        "Floor: {}",
                        if self.fractal_params.show_floor {
                            "ON"
                        } else {
                            "OFF"
                        }
                    );
                    return true;
                }
                KeyCode::KeyB => {
                    // Cycle shadow mode: 0 -> 1 -> 2 -> 0
                    self.fractal_params.shadow_mode = (self.fractal_params.shadow_mode + 1) % 3;
                    let mode_name = match self.fractal_params.shadow_mode {
                        0 => "OFF",
                        1 => "HARD",
                        _ => "SOFT",
                    };
                    println!("Shadow Mode: {}", mode_name);
                    return true;
                }
                KeyCode::KeyI => {
                    // Toggle LOD system on/off
                    self.fractal_params.lod_config.enabled =
                        !self.fractal_params.lod_config.enabled;
                    println!(
                        "LOD System: {}",
                        if self.fractal_params.lod_config.enabled {
                            "ON"
                        } else {
                            "OFF"
                        }
                    );
                    return true;
                }
                KeyCode::KeyD => {
                    // Shift+D toggles LOD debug visualization
                    if self.shift_pressed {
                        self.fractal_params.lod_config.debug_visualization =
                            !self.fractal_params.lod_config.debug_visualization;
                        println!(
                            "LOD Debug Visualization: {}",
                            if self.fractal_params.lod_config.debug_visualization {
                                "ON"
                            } else {
                                "OFF"
                            }
                        );
                        return true;
                    }
                }
                KeyCode::Slash => {
                    // Open command palette with '/'
                    self.ui.command_palette.open();
                    println!("Command Palette opened");
                    return true;
                }
                KeyCode::KeyK => {
                    // Ctrl+K also opens command palette (VS Code style)
                    #[cfg(target_os = "macos")]
                    let modifier_pressed =
                        self.egui_state.egui_ctx().input(|i| i.modifiers.command);
                    #[cfg(not(target_os = "macos"))]
                    let modifier_pressed = self.egui_state.egui_ctx().input(|i| i.modifiers.ctrl);

                    if modifier_pressed {
                        self.ui.command_palette.open();
                        println!("Command Palette opened");
                        return true;
                    }
                }
                _ => {}
            }
        }

        // Handle mode-specific input only if egui doesn't block it
        // For touch: always handle (egui consumed flag already checked)
        // For mouse: only if egui doesn't want pointer input
        // Also skip camera input during auto-orbit to prevent state accumulation
        if !egui_blocks_mouse {
            if matches!(event, WindowEvent::Touch(_)) {
                log::info!(
                    "Routing touch to mode handler: {:?}",
                    self.fractal_params.render_mode
                );
            }
            match self.fractal_params.render_mode {
                RenderMode::TwoD => self.handle_2d_input(event),
                RenderMode::ThreeD => {
                    // Don't process camera events during auto-orbit to prevent state accumulation
                    if !self.fractal_params.auto_orbit {
                        self.camera_controller.process_events(event)
                    } else {
                        false
                    }
                }
            }
        } else {
            false
        }
    }

    fn handle_2d_input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                // Don't handle mouse input if we have active touches
                // (touch events set their own mouse_pressed state)
                if self.active_touches.is_empty() {
                    self.mouse_pressed = *state == ElementState::Pressed;
                    if !self.mouse_pressed {
                        self.last_mouse_pos = None;
                    }
                }
                true
            }
            WindowEvent::Touch(touch) => {
                // Handle touch events for mobile 2D panning and pinch-to-zoom
                let current_pos = (touch.location.x as f32, touch.location.y as f32);

                match touch.phase {
                    TouchPhase::Started => {
                        // Clear stale touches if we have too many (Ended events might have been lost)
                        if self.active_touches.len() >= 2 {
                            log::info!("🔧 Touch: Clearing {} stale touches", self.active_touches.len());
                            self.active_touches.clear();
                            self.initial_pinch_distance = None;
                        }

                        self.active_touches.insert(touch.id, current_pos);
                        log::info!("🔧 Touch Started: id={}, active={}, mouse_pressed={}, last_mouse_pos={:?}",
                            touch.id, self.active_touches.len(), self.mouse_pressed, self.last_mouse_pos.is_some());

                        // If this is the first touch, enable panning
                        if self.active_touches.len() == 1 {
                            self.mouse_pressed = true;
                            self.cursor_pos = current_pos;
                            self.last_mouse_pos = Some(current_pos);
                            log::info!("🔧 Touch: Enabled panning for first touch at ({:.1}, {:.1})", current_pos.0, current_pos.1);
                        }
                        // If we now have 2 touches, start pinch gesture
                        else if self.active_touches.len() == 2 {
                            self.mouse_pressed = false; // Disable panning during pinch
                            self.last_mouse_pos = None;
                            // Calculate initial distance between two fingers
                            let touches: Vec<&(f32, f32)> = self.active_touches.values().collect();
                            let dx = touches[0].0 - touches[1].0;
                            let dy = touches[0].1 - touches[1].1;
                            let distance = (dx * dx + dy * dy).sqrt();
                            self.initial_pinch_distance = Some(distance);
                        }
                        true
                    }
                    TouchPhase::Moved => {
                        self.active_touches.insert(touch.id, current_pos);

                        // Handle pinch-to-zoom (2 fingers)
                        if self.active_touches.len() == 2 {
                            let touches: Vec<&(f32, f32)> = self.active_touches.values().collect();
                            let dx = touches[0].0 - touches[1].0;
                            let dy = touches[0].1 - touches[1].1;
                            let current_distance = (dx * dx + dy * dy).sqrt();

                            if let Some(initial_distance) = self.initial_pinch_distance {
                                // Calculate zoom factor based on distance change
                                let zoom_delta = current_distance / initial_distance;
                                // Apply zoom with smoothing (50% sensitivity for responsive zoom)
                                let zoom_factor = 1.0 + (zoom_delta - 1.0) * 0.5;

                                // Calculate center point between two fingers (pinch center)
                                let center_x = (touches[0].0 + touches[1].0) / 2.0;
                                let center_y = (touches[0].1 + touches[1].1) / 2.0;

                                // Convert pinch center from screen coords to fractal coords
                                let screen_x = (center_x / self.renderer.size.width as f32) * 2.0 - 1.0;
                                let screen_y = 1.0 - (center_y / self.renderer.size.height as f32) * 2.0;
                                let aspect = self.renderer.size.width as f64 / self.renderer.size.height as f64;

                                // Calculate where the pinch center is in fractal coordinates
                                let fractal_x = self.fractal_params.center_2d[0] + (screen_x as f64) * aspect / self.fractal_params.zoom_2d as f64;
                                let fractal_y = self.fractal_params.center_2d[1] + (screen_y as f64) / self.fractal_params.zoom_2d as f64;

                                // Apply zoom
                                let old_zoom = self.fractal_params.zoom_2d;
                                self.fractal_params.zoom_2d *= zoom_factor;

                                // Adjust center so the pinch point stays in the same place
                                // new_center = old_center + (point - old_center) * (1 - old_zoom/new_zoom)
                                let zoom_ratio = old_zoom / self.fractal_params.zoom_2d;
                                self.fractal_params.center_2d[0] += (fractal_x - self.fractal_params.center_2d[0]) * (1.0 - zoom_ratio as f64);
                                self.fractal_params.center_2d[1] += (fractal_y - self.fractal_params.center_2d[1]) * (1.0 - zoom_ratio as f64);

                                // Update initial distance for next frame
                                self.initial_pinch_distance = Some(current_distance);
                                self.cursor_pos = (center_x, center_y);
                            }
                            true
                        }
                        // Handle single-finger pan (simplified - like 3D camera)
                        else if self.active_touches.len() == 1 {
                            self.cursor_pos = current_pos;

                            // Ensure panning is enabled for single touch
                            if !self.mouse_pressed {
                                log::info!("🔧 Touch Move: Re-enabling mouse_pressed (was off!)");
                                self.mouse_pressed = true;
                                self.last_mouse_pos = Some(current_pos);
                            }

                            if let Some(last_pos) = self.last_mouse_pos {
                                let delta_x = (current_pos.0 - last_pos.0) as f64
                                    / self.renderer.size.width as f64;
                                let delta_y = (current_pos.1 - last_pos.1) as f64
                                    / self.renderer.size.height as f64;

                                log::info!("🔧 Touch Pan: delta=({:.3}, {:.3}), mouse_pressed={}, zoom={:.2}",
                                    delta_x, delta_y, self.mouse_pressed, self.fractal_params.zoom_2d);

                                let aspect = self.renderer.size.width as f64
                                    / self.renderer.size.height as f64;
                                self.fractal_params.center_2d[0] -=
                                    delta_x * 2.0 / self.fractal_params.zoom_2d as f64 * aspect;
                                self.fractal_params.center_2d[1] +=
                                    delta_y * 2.0 / self.fractal_params.zoom_2d as f64;
                            } else {
                                log::info!("🔧 Touch Pan: last_mouse_pos is None! mouse_pressed={}", self.mouse_pressed);
                            }
                            self.last_mouse_pos = Some(current_pos);
                            true
                        } else {
                            false
                        }
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        self.active_touches.remove(&touch.id);

                        // If we're down to 1 or 0 touches, reset pinch state
                        if self.active_touches.len() < 2 {
                            self.initial_pinch_distance = None;
                        }

                        // If all touches ended, reset mouse state
                        if self.active_touches.is_empty() {
                            self.mouse_pressed = false;
                            self.last_mouse_pos = None;
                        }
                        // If we're down to 1 touch, re-enable panning
                        else if self.active_touches.len() == 1 {
                            let remaining_touch = self.active_touches.values().next().unwrap();
                            self.mouse_pressed = true;
                            self.cursor_pos = *remaining_touch;
                            self.last_mouse_pos = Some(*remaining_touch);
                        }
                        true
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let current_pos = (position.x as f32, position.y as f32);
                self.cursor_pos = current_pos; // Always track cursor position

                // Don't handle CursorMoved if we have active touches
                // (touch events generate their own cursor movements on web)
                if self.active_touches.is_empty() && self.mouse_pressed && !self.shift_pressed {
                    // Pan when dragging without shift (shift+drag is continuous zoom)
                    if let Some(last_pos) = self.last_mouse_pos {
                        let delta_x =
                            (current_pos.0 - last_pos.0) as f64 / self.renderer.size.width as f64;
                        let delta_y =
                            (current_pos.1 - last_pos.1) as f64 / self.renderer.size.height as f64;

                        let aspect =
                            self.renderer.size.width as f64 / self.renderer.size.height as f64;
                        // Scale factor matches shader: world = screen * 2 / (zoom * height)
                        // delta is already normalized by width/height, so multiply by 2
                        self.fractal_params.center_2d[0] -=
                            delta_x * 2.0 / self.fractal_params.zoom_2d as f64 * aspect;
                        self.fractal_params.center_2d[1] +=
                            delta_y * 2.0 / self.fractal_params.zoom_2d as f64;
                    }
                    self.last_mouse_pos = Some(current_pos);
                    true
                } else {
                    self.last_mouse_pos = None; // Reset when shift is held
                    false
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let zoom_delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01,
                };

                let zoom_factor = 1.1f32.powf(zoom_delta);

                // Zoom at cursor position (for 2D mode)
                if self.fractal_params.render_mode == RenderMode::TwoD {
                    let width = self.renderer.size.width as f64;
                    let height = self.renderer.size.height as f64;
                    let aspect = width / height;

                    // Convert cursor position to normalized coordinates [-1, 1]
                    let norm_x = (self.cursor_pos.0 as f64 / width) * 2.0 - 1.0;
                    let norm_y = 1.0 - (self.cursor_pos.1 as f64 / height) * 2.0; // Flip Y

                    // Convert to fractal coordinates
                    let zoom = self.fractal_params.zoom_2d as f64;
                    let fractal_x =
                        self.fractal_params.center_2d[0] + (norm_x * 2.0 / zoom) * aspect;
                    let fractal_y = self.fractal_params.center_2d[1] + norm_y * 2.0 / zoom;

                    // Apply zoom
                    self.fractal_params.zoom_2d *= zoom_factor;

                    // Adjust center so the point under cursor stays in place
                    let new_zoom = self.fractal_params.zoom_2d as f64;
                    let new_fractal_x =
                        self.fractal_params.center_2d[0] + (norm_x * 2.0 / new_zoom) * aspect;
                    let new_fractal_y = self.fractal_params.center_2d[1] + norm_y * 2.0 / new_zoom;

                    self.fractal_params.center_2d[0] += fractal_x - new_fractal_x;
                    self.fractal_params.center_2d[1] += fractal_y - new_fractal_y;
                } else {
                    self.fractal_params.zoom_2d *= zoom_factor;
                }
                true
            }
            _ => false,
        }
    }
}
