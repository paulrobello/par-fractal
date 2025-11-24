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

        // Check if egui wants pointer input RIGHT NOW (not from previous frame)
        // This prevents camera movement during window resizing from any edge
        let ctx = self.egui_state.egui_ctx();
        // Check multiple conditions to catch all UI interactions including window resizing
        let egui_wants_pointer = ctx.wants_pointer_input()
            || ctx.is_pointer_over_area()
            || ctx.is_using_pointer()
            || ctx.dragged_id().is_some();

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
                        .switch_fractal(FractalType::TgladFormula3D);
                    self.reset_view();
                    return true;
                }
                KeyCode::F7 => {
                    self.fractal_params
                        .switch_fractal(FractalType::OctahedralIFS3D);
                    self.reset_view();
                    return true;
                }
                KeyCode::F8 => {
                    self.fractal_params
                        .switch_fractal(FractalType::IcosahedralIFS3D);
                    self.reset_view();
                    return true;
                }
                KeyCode::F9 => {
                    self.fractal_params
                        .switch_fractal(FractalType::ApollonianGasket3D);
                    self.reset_view();
                    return true;
                }
                KeyCode::F10 => {
                    self.fractal_params.switch_fractal(FractalType::Kleinian3D);
                    self.reset_view();
                    return true;
                }
                KeyCode::F11 => {
                    self.fractal_params
                        .switch_fractal(FractalType::HybridMandelbulbJulia3D);
                    self.reset_view();
                    return true;
                }
                // QuaternionCubic3D: accessible via UI only (no F12 - reserved for screenshot)
                KeyCode::KeyP => {
                    self.fractal_params.next_palette();
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

        // Handle mode-specific input only if egui doesn't want pointer input
        // This prevents camera/view manipulation when interacting with UI (e.g., resizing window)
        // Also skip camera input during auto-orbit to prevent state accumulation
        if !egui_wants_pointer {
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
                self.mouse_pressed = *state == ElementState::Pressed;
                if !self.mouse_pressed {
                    self.last_mouse_pos = None;
                }
                true
            }
            WindowEvent::CursorMoved { position, .. } => {
                let current_pos = (position.x as f32, position.y as f32);
                self.cursor_pos = current_pos; // Always track cursor position

                if self.mouse_pressed && !self.shift_pressed {
                    // Pan when dragging without shift (shift+drag is continuous zoom)
                    if let Some(last_pos) = self.last_mouse_pos {
                        let delta_x =
                            (current_pos.0 - last_pos.0) as f64 / self.renderer.size.width as f64;
                        let delta_y =
                            (current_pos.1 - last_pos.1) as f64 / self.renderer.size.height as f64;

                        let aspect =
                            self.renderer.size.width as f64 / self.renderer.size.height as f64;
                        self.fractal_params.center_2d[0] -=
                            delta_x * 4.0 / self.fractal_params.zoom_2d as f64 * aspect;
                        self.fractal_params.center_2d[1] +=
                            delta_y * 4.0 / self.fractal_params.zoom_2d as f64;
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
