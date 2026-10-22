use super::App;
use crate::fractal::{FractalType, RenderMode};
use winit::event::*;
use winit::keyboard::{KeyCode, PhysicalKey};

/// Input handling methods
impl App {
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        // ARC-006: any interactive input event must force a redraw, so egui (and
        // the fractal) actually process it. See `App::input_pending`.
        if matches!(
            event,
            WindowEvent::KeyboardInput { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::CursorMoved { .. }
                | WindowEvent::Touch { .. }
                | WindowEvent::ModifiersChanged(_)
        ) {
            self.input_pending = true;
        }
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
            ctx.egui_wants_pointer_input()
                || ctx.is_pointer_over_egui()
                || ctx.egui_is_using_pointer()
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
            && self.handle_key_shortcut(*keycode)
        {
            return true;
        }

        // Handle mode-specific input only if egui doesn't block it
        // For touch: always handle (egui consumed flag already checked)
        // For mouse: only if egui doesn't want pointer input
        // Also skip camera input during auto-orbit to prevent state accumulation
        if !egui_blocks_mouse {
            if matches!(event, WindowEvent::Touch(_)) {
                log::info!(
                    "Routing touch to mode handler: {:?}",
                    self.fractal_params.settings.render_mode
                );
            }
            let handled = match self.fractal_params.settings.render_mode {
                RenderMode::TwoD => self.handle_2d_input(event),
                RenderMode::ThreeD => {
                    // Don't process camera events during auto-orbit to prevent state accumulation
                    if !self.fractal_params.settings.auto_orbit {
                        self.camera_controller.process_events(event)
                    } else {
                        false
                    }
                }
            };
            // ARC-006: 2D pan/zoom or 3D camera events mutate the image.
            if handled {
                self.mark_scene_dirty();
            }
            handled
        } else {
            false
        }
    }

    /// QA-035: fractal-type hotkeys (digits + function keys).
    fn handle_fractal_hotkey(&mut self, keycode: KeyCode) -> Option<bool> {
        match keycode {
            KeyCode::Digit1 => {
                self.fractal_params
                    .switch_fractal(FractalType::Mandelbrot2D);
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::Digit2 => {
                self.fractal_params.switch_fractal(FractalType::Julia2D);
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::Digit3 => {
                self.fractal_params
                    .switch_fractal(FractalType::Sierpinski2D);
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::Digit4 => {
                self.fractal_params
                    .switch_fractal(FractalType::BurningShip2D);
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::Digit5 => {
                self.fractal_params.switch_fractal(FractalType::Tricorn2D);
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::Digit6 => {
                self.fractal_params.switch_fractal(FractalType::Phoenix2D);
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::Digit7 => {
                self.fractal_params.switch_fractal(FractalType::Celtic2D);
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::Digit8 => {
                self.fractal_params.switch_fractal(FractalType::Newton2D);
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::Digit9 => {
                self.fractal_params.switch_fractal(FractalType::Lyapunov2D);
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::Digit0 => {
                self.fractal_params.switch_fractal(FractalType::Nova2D);
                self.mark_scene_dirty();
                Some(true)
            }
            // Magnet and Collatz: use UI (no hotkey due to limited keys)
            // 3D Fractals (F1-F9)
            KeyCode::F1 => {
                self.fractal_params
                    .switch_fractal(FractalType::Mandelbulb3D);
                self.reset_view();
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::F2 => {
                self.fractal_params
                    .switch_fractal(FractalType::MengerSponge3D);
                self.reset_view();
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::F3 => {
                self.fractal_params
                    .switch_fractal(FractalType::SierpinskiPyramid3D);
                self.reset_view();
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::F4 => {
                self.fractal_params.switch_fractal(FractalType::JuliaSet3D);
                self.reset_view();
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::F5 => {
                self.fractal_params.switch_fractal(FractalType::Mandelbox3D);
                self.reset_view();
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::F6 => {
                self.fractal_params
                    .switch_fractal(FractalType::OctahedralIFS3D);
                self.reset_view();
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::F7 => {
                self.fractal_params
                    .switch_fractal(FractalType::IcosahedralIFS3D);
                self.reset_view();
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::F8 => {
                self.fractal_params
                    .switch_fractal(FractalType::ApollonianGasket3D);
                self.reset_view();
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::F9 => {
                self.fractal_params.switch_fractal(FractalType::Kleinian3D);
                self.reset_view();
                self.mark_scene_dirty();
                Some(true)
            }
            KeyCode::F10 => {
                self.fractal_params
                    .switch_fractal(FractalType::HybridMandelbulbJulia3D);
                self.reset_view();
                self.mark_scene_dirty();
                Some(true)
            }
            // QuaternionCubic3D: accessible via UI only (no F12 - reserved for screenshot)
            // Every arm above diverges; the catch-all IS the fall-through.
            _ => None,
        }
    }

    /// QA-035: parameter-mutation hotkeys (the ARC-025 commit_change arms).
    fn handle_parameter_hotkey(&mut self, keycode: KeyCode) -> Option<bool> {
        match keycode {
            KeyCode::KeyO => {
                let pre = self.begin_change();
                self.fractal_params.settings.auto_orbit = !self.fractal_params.settings.auto_orbit;
                self.commit_change(pre);
                log::info!(
                    "Auto-orbit: {}",
                    if self.fractal_params.settings.auto_orbit {
                        "ON"
                    } else {
                        "OFF"
                    }
                );
                return Some(true);
            }
            KeyCode::BracketLeft => {
                let pre = self.begin_change();
                let value = self.fractal_params.adjust_orbit_speed(-0.1);
                self.commit_change(pre);
                log::info!("Orbit speed: {value:.2}");
                return Some(true);
            }
            KeyCode::BracketRight => {
                let pre = self.begin_change();
                let value = self.fractal_params.adjust_orbit_speed(0.1);
                self.commit_change(pre);
                log::info!("Orbit speed: {value:.2}");
                return Some(true);
            }
            KeyCode::Minus => {
                let pre = self.begin_change();
                let value = self.fractal_params.adjust_iterations(-32);
                let label = if self.fractal_params.settings.render_mode == RenderMode::TwoD {
                    format!("Max iterations: {value}")
                } else {
                    format!("Max steps: {value}")
                };
                log::info!("{label}");
                self.commit_change(pre);
                return Some(true);
            }
            KeyCode::Equal => {
                let pre = self.begin_change();
                let value = self.fractal_params.adjust_iterations(32);
                let label = if self.fractal_params.settings.render_mode == RenderMode::TwoD {
                    format!("Max iterations: {value}")
                } else {
                    format!("Max steps: {value}")
                };
                log::info!("{label}");
                self.commit_change(pre);
                return Some(true);
            }
            KeyCode::Comma => {
                let pre = self.begin_change();
                let value = self.fractal_params.adjust_power(-0.5);
                self.commit_change(pre);
                log::info!("Power: {value:.1}");
                return Some(true);
            }
            KeyCode::Period => {
                let pre = self.begin_change();
                let value = self.fractal_params.adjust_power(0.5);
                self.commit_change(pre);
                log::info!("Power: {value:.1}");
                return Some(true);
            }
            KeyCode::KeyL => {
                let pre = self.begin_change();
                self.fractal_params.settings.ambient_occlusion =
                    !self.fractal_params.settings.ambient_occlusion;
                self.commit_change(pre);
                log::info!(
                    "Ambient Occlusion: {}",
                    if self.fractal_params.settings.ambient_occlusion {
                        "ON"
                    } else {
                        "OFF"
                    }
                );
                return Some(true);
            }
            KeyCode::KeyT => {
                let pre = self.begin_change();
                self.fractal_params.settings.depth_of_field =
                    !self.fractal_params.settings.depth_of_field;
                self.commit_change(pre);
                log::info!(
                    "Depth of Field: {}",
                    if self.fractal_params.settings.depth_of_field {
                        "ON"
                    } else {
                        "OFF"
                    }
                );
                return Some(true);
            }
            KeyCode::KeyG => {
                let pre = self.begin_change();
                if self.shift_pressed {
                    // Shift+G: toggle ENH-006 GPU profile HUD overlay
                    // (mirrors the Shift+D LOD-debug-visualization pattern).
                    self.ui.show_gpu_profile = !self.ui.show_gpu_profile;
                    self.ui.ui_state.show_gpu_profile = self.ui.show_gpu_profile;
                    log::info!(
                        "GPU Profile Overlay: {}",
                        if self.ui.show_gpu_profile {
                            "ON"
                        } else {
                            "OFF"
                        }
                    );
                    return Some(true);
                }
                self.fractal_params.settings.show_floor = !self.fractal_params.settings.show_floor;
                self.commit_change(pre);
                log::info!(
                    "Floor: {}",
                    if self.fractal_params.settings.show_floor {
                        "ON"
                    } else {
                        "OFF"
                    }
                );
                return Some(true);
            }
            KeyCode::KeyB => {
                let pre = self.begin_change();
                // Cycle shadow mode: 0 -> 1 -> 2 -> 0
                self.fractal_params.settings.shadow_mode =
                    (self.fractal_params.settings.shadow_mode + 1) % 3;
                let mode_name = match self.fractal_params.settings.shadow_mode {
                    0 => "OFF",
                    1 => "HARD",
                    _ => "SOFT",
                };
                self.commit_change(pre);
                log::info!("Shadow Mode: {}", mode_name);
                return Some(true);
            }
            KeyCode::KeyI => {
                // Toggle LOD system on/off
                self.fractal_params.lod.lod_config.enabled =
                    !self.fractal_params.lod.lod_config.enabled;
                self.mark_scene_dirty();
                log::info!(
                    "LOD System: {}",
                    if self.fractal_params.lod.lod_config.enabled {
                        "ON"
                    } else {
                        "OFF"
                    }
                );
                return Some(true);
            }
            KeyCode::KeyD => {
                // Shift+D toggles LOD debug visualization
                if self.shift_pressed {
                    self.fractal_params.lod.lod_config.debug_visualization =
                        !self.fractal_params.lod.lod_config.debug_visualization;
                    self.mark_scene_dirty();
                    log::info!(
                        "LOD Debug Visualization: {}",
                        if self.fractal_params.lod.lod_config.debug_visualization {
                            "ON"
                        } else {
                            "OFF"
                        }
                    );
                    return Some(true);
                }
            }
            _ => return None,
        }
        Some(true)
    }

    /// QA-035: remaining global shortcuts (UI toggles, palette,
    /// undo/redo, command palette). Returns `true` when consumed.
    fn handle_key_shortcut(&mut self, keycode: KeyCode) -> bool {
        if let Some(consumed) = self.handle_fractal_hotkey(keycode) {
            return consumed;
        }
        if let Some(consumed) = self.handle_parameter_hotkey(keycode) {
            return consumed;
        }
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
            KeyCode::KeyZ => {
                // ENH-002: Ctrl/Cmd+Z undo, Ctrl/Cmd+Shift+Z redo. Cmd on
                // macOS, Ctrl elsewhere (same convention as Ctrl+K).
                #[cfg(target_os = "macos")]
                let modifier_pressed = self.egui_state.egui_ctx().input(|i| i.modifiers.command);
                #[cfg(not(target_os = "macos"))]
                let modifier_pressed = self.egui_state.egui_ctx().input(|i| i.modifiers.ctrl);

                if modifier_pressed {
                    let shift = self.egui_state.egui_ctx().input(|i| i.modifiers.shift);
                    let applied = if shift {
                        self.apply_redo()
                    } else {
                        self.apply_undo()
                    };
                    if applied {
                        log::info!("{} via keyboard", if shift { "Redo" } else { "Undo" });
                    }
                    return true;
                }
            }
            KeyCode::KeyY => {
                // ENH-002: Ctrl/Cmd+Y is the canonical redo binding.
                #[cfg(target_os = "macos")]
                let modifier_pressed = self.egui_state.egui_ctx().input(|i| i.modifiers.command);
                #[cfg(not(target_os = "macos"))]
                let modifier_pressed = self.egui_state.egui_ctx().input(|i| i.modifiers.ctrl);

                if modifier_pressed && self.apply_redo() {
                    log::info!("Redo via keyboard");
                    return true;
                }
            }
            KeyCode::KeyR => {
                self.reset_view();
                self.mark_scene_dirty();
                return true;
            }
            // 2D Fractals (1-7)
            KeyCode::KeyP => {
                let pre = self.begin_change();
                if self.shift_pressed {
                    // Shift+P: Cycle procedural palette
                    use crate::fractal::ProceduralPalette;
                    let all_options: Vec<ProceduralPalette> =
                        std::iter::once(ProceduralPalette::None)
                            .chain(ProceduralPalette::ALL.iter().copied())
                            .collect();
                    let current_idx = all_options
                        .iter()
                        .position(|p| *p == self.fractal_params.settings.procedural_palette)
                        .unwrap_or(0);
                    let next_idx = (current_idx + 1) % all_options.len();
                    self.fractal_params.settings.procedural_palette = all_options[next_idx];
                    self.ui.show_toast(format!(
                        "Procedural: {}",
                        self.fractal_params.settings.procedural_palette.name()
                    ));
                } else {
                    // P: Cycle static palette
                    self.fractal_params.next_palette();
                    self.ui.show_toast(format!(
                        "Palette: {}",
                        self.fractal_params.settings.palette.name
                    ));
                }
                self.commit_change(pre);
                return true;
            }
            KeyCode::F12 => {
                self.save_screenshot = true;
                self.mark_scene_dirty();
                log::info!("Screenshot queued...");
                return true;
            }
            KeyCode::Slash => {
                // Open command palette with '/'
                self.ui.command_palette.open();
                log::info!("Command Palette opened");
                return true;
            }
            KeyCode::KeyK => {
                // Ctrl+K also opens command palette (VS Code style)
                #[cfg(target_os = "macos")]
                let modifier_pressed = self.egui_state.egui_ctx().input(|i| i.modifiers.command);
                #[cfg(not(target_os = "macos"))]
                let modifier_pressed = self.egui_state.egui_ctx().input(|i| i.modifiers.ctrl);

                if modifier_pressed {
                    self.ui.command_palette.open();
                    log::info!("Command Palette opened");
                    return true;
                }
            }
            _ => {}
        }
        false
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
                    let pressed = *state == ElementState::Pressed;
                    // ENH-002: double-click-to-center-zoom. winit 0.30 has no
                    // click_count, so detect it manually: a second press on
                    // the same spot within 400 ms. cursor_pos is the physical
                    // pixel position tracked by CursorMoved.
                    if pressed {
                        let now = web_time::Instant::now();
                        let pos = self.cursor_pos;
                        let is_double = self
                            .last_click_at
                            .is_some_and(|t| now.duration_since(t).as_millis() < 400)
                            && self.last_click_pos.is_some_and(|p| {
                                ((p.0 - pos.0).powi(2) + (p.1 - pos.1).powi(2)) < 36.0
                            });
                        self.last_click_at = Some(now);
                        self.last_click_pos = Some(pos);
                        if is_double {
                            // Require a fresh press pair for the next double-click.
                            self.last_click_at = None;
                            self.double_click_center_zoom(pos);
                            return true;
                        }
                    }
                    self.mouse_pressed = pressed;
                    if !pressed {
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
                        let now = web_time::Instant::now();

                        // Clear stale touches only if we have 2+ already
                        // This prevents phantom touches while allowing valid 2-finger pinch
                        if self.active_touches.is_empty() {
                            self.last_touch_time = Some(now);
                        } else if self.active_touches.len() >= 2 {
                            self.active_touches.clear();
                            self.initial_pinch_distance = None;
                            self.mouse_pressed = false;
                            self.last_mouse_pos = None;
                            self.last_touch_time = Some(now);
                        }

                        self.active_touches.insert(touch.id, current_pos);

                        // If this is the first touch, enable panning
                        if self.active_touches.len() == 1 {
                            self.mouse_pressed = true;
                            self.cursor_pos = current_pos;
                            self.last_mouse_pos = Some(current_pos);
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

                                // Convert pinch center from screen coords to NDC
                                // ([-1,1], y-up, before aspect correction — same convention
                                // as zoom_at: matches continuous-zoom and scroll-wheel paths).
                                let screen_x =
                                    (center_x / self.renderer.size.width as f32) * 2.0 - 1.0;
                                let screen_y =
                                    1.0 - (center_y / self.renderer.size.height as f32) * 2.0;
                                let aspect = self.renderer.size.width as f64
                                    / self.renderer.size.height as f64;

                                // Shared seam: zoom_at keeps the fractal point under the
                                // pinch center fixed. (Replaces a bespoke computation that
                                // differed from the reference by a factor of 2 in the
                                // center correction, causing drift toward a corner.)
                                self.fractal_params.zoom_at(
                                    (screen_x as f64, screen_y as f64),
                                    zoom_factor as f64,
                                    aspect,
                                );

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
                                self.mouse_pressed = true;
                                self.last_mouse_pos = Some(current_pos);
                            }

                            if let Some(last_pos) = self.last_mouse_pos {
                                let delta_x = (current_pos.0 - last_pos.0) as f64
                                    / self.renderer.size.width as f64;
                                let delta_y = (current_pos.1 - last_pos.1) as f64
                                    / self.renderer.size.height as f64;

                                let aspect = self.renderer.size.width as f64
                                    / self.renderer.size.height as f64;
                                self.fractal_params.settings.center_2d[0] -=
                                    delta_x * 2.0 / self.fractal_params.settings.zoom_2d * aspect;
                                self.fractal_params.settings.center_2d[1] +=
                                    delta_y * 2.0 / self.fractal_params.settings.zoom_2d;
                                // Pan moves the center → drop any precise decimal override
                                // (ENH-001 Phase C); the f64 center is now the source of truth.
                                self.fractal_params.settings.center_2d_precise = None;
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

                        // If all touches ended, reset mouse state and timing
                        if self.active_touches.is_empty() {
                            self.mouse_pressed = false;
                            self.last_mouse_pos = None;
                            self.last_touch_time = None; // Reset timing for next touch sequence
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
                        self.fractal_params.settings.center_2d[0] -=
                            delta_x * 2.0 / self.fractal_params.settings.zoom_2d * aspect;
                        self.fractal_params.settings.center_2d[1] +=
                            delta_y * 2.0 / self.fractal_params.settings.zoom_2d;
                        // Pan moves the center → drop any precise decimal override
                        // (ENH-001 Phase C); the f64 center is now the source of truth.
                        self.fractal_params.settings.center_2d_precise = None;
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
                if self.fractal_params.settings.render_mode == RenderMode::TwoD {
                    let width = self.renderer.size.width as f64;
                    let height = self.renderer.size.height as f64;
                    let aspect = width / height;

                    // Convert cursor position to normalized coordinates [-1, 1]
                    // (y-up, before aspect correction — matches the zoom_at seam).
                    let norm_x = (self.cursor_pos.0 as f64 / width) * 2.0 - 1.0;
                    let norm_y = 1.0 - (self.cursor_pos.1 as f64 / height) * 2.0; // Flip Y

                    self.fractal_params
                        .zoom_at((norm_x, norm_y), zoom_factor as f64, aspect);
                } else {
                    self.fractal_params.settings.zoom_2d *= zoom_factor as f64;
                }
                true
            }
            _ => false,
        }
    }
}
