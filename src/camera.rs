use glam::{Mat4, Vec3};
use winit::event::*;
use winit::keyboard::{KeyCode, PhysicalKey};

#[derive(Clone)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            aspect: width as f32 / height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        }
    }

    pub fn reset_to_default(&mut self) {
        self.position = Vec3::new(0.0, 0.0, 5.0);
        self.target = Vec3::ZERO;
        self.up = Vec3::Y;
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.position, self.target, self.up);
        let proj = Mat4::perspective_rh(self.fovy.to_radians(), self.aspect, self.znear, self.zfar);
        proj * view
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }
}

pub struct CameraController {
    speed: f32,
    rotate_speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_up_pressed: bool,
    is_down_pressed: bool,
    mouse_pressed: bool,
    last_mouse_pos: Option<(f32, f32)>,
    yaw: f32,
    pitch: f32,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        // Initialize yaw to point forward (along -Z axis)
        // Initialize pitch to 0 (looking straight ahead)
        Self {
            speed,
            rotate_speed: 0.003,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_up_pressed: false,
            is_down_pressed: false,
            mouse_pressed: false,
            last_mouse_pos: None,
            yaw: 0.0,   // 0 means looking along -Z
            pitch: 0.0, // 0 means level (no up/down tilt)
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(keycode),
                        state,
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    KeyCode::KeyW | KeyCode::ArrowUp => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyS | KeyCode::ArrowDown => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyA | KeyCode::ArrowLeft => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyD | KeyCode::ArrowRight => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyE => {
                        self.is_up_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyQ => {
                        self.is_down_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
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
                if self.mouse_pressed {
                    let current_pos = (position.x as f32, position.y as f32);
                    if let Some(last_pos) = self.last_mouse_pos {
                        let delta_x = current_pos.0 - last_pos.0;
                        let delta_y = current_pos.1 - last_pos.1;

                        // Negate delta_x so dragging right rotates view right
                        self.yaw -= delta_x * self.rotate_speed;
                        self.pitch -= delta_y * self.rotate_speed;
                        self.pitch = self
                            .pitch
                            .clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
                    }
                    self.last_mouse_pos = Some(current_pos);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn update_camera(&self, camera: &mut Camera, dt: f32) {
        // Calculate orientation from yaw/pitch first
        let yaw_quat = glam::Quat::from_axis_angle(Vec3::Y, self.yaw);
        let pitch_quat = glam::Quat::from_axis_angle(Vec3::X, self.pitch);
        let rotation = yaw_quat * pitch_quat;

        let forward = rotation * Vec3::new(0.0, 0.0, -1.0);
        let right = forward.cross(camera.up).normalize();

        // Move camera based on input
        if self.is_forward_pressed {
            camera.position += forward * self.speed * dt;
        }
        if self.is_backward_pressed {
            camera.position -= forward * self.speed * dt;
        }
        if self.is_right_pressed {
            camera.position += right * self.speed * dt;
        }
        if self.is_left_pressed {
            camera.position -= right * self.speed * dt;
        }
        if self.is_up_pressed {
            camera.position += camera.up * self.speed * dt;
        }
        if self.is_down_pressed {
            camera.position -= camera.up * self.speed * dt;
        }

        // Update camera target to look in the direction defined by yaw/pitch
        camera.target = camera.position + forward;
    }

    // Public testing interface
    // These methods are intended for testing and simulation purposes
    #[allow(dead_code)]
    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    pub fn point_at_target(&mut self, camera_pos: Vec3, target: Vec3) {
        // Calculate direction from camera to target
        let direction = (target - camera_pos).normalize();

        // Calculate yaw (rotation around Y axis) from the horizontal components
        // Note: atan2(x, z) where default forward is -Z
        self.yaw = (-direction.x).atan2(-direction.z);

        // Calculate pitch (rotation around X axis)
        // Use atan2 for better numerical stability than asin
        let horizontal_length = (direction.x * direction.x + direction.z * direction.z).sqrt();
        self.pitch = direction.y.atan2(horizontal_length);

        // Clamp pitch to avoid gimbal lock
        self.pitch = self
            .pitch
            .clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
    }

    #[allow(dead_code)]
    pub fn is_any_key_pressed(&self) -> bool {
        self.is_forward_pressed
            || self.is_backward_pressed
            || self.is_left_pressed
            || self.is_right_pressed
            || self.is_up_pressed
            || self.is_down_pressed
    }

    // Setters for testing camera movement simulation
    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn simulate_forward_press(&mut self, pressed: bool) {
        self.is_forward_pressed = pressed;
    }

    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn simulate_backward_press(&mut self, pressed: bool) {
        self.is_backward_pressed = pressed;
    }

    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn simulate_left_press(&mut self, pressed: bool) {
        self.is_left_pressed = pressed;
    }

    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn simulate_right_press(&mut self, pressed: bool) {
        self.is_right_pressed = pressed;
    }

    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn simulate_up_press(&mut self, pressed: bool) {
        self.is_up_pressed = pressed;
    }

    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn simulate_down_press(&mut self, pressed: bool) {
        self.is_down_pressed = pressed;
    }

    // Test-only getters
    #[cfg(test)]
    pub fn is_forward_pressed(&self) -> bool {
        self.is_forward_pressed
    }

    #[cfg(test)]
    pub fn is_backward_pressed(&self) -> bool {
        self.is_backward_pressed
    }

    #[cfg(test)]
    pub fn is_left_pressed(&self) -> bool {
        self.is_left_pressed
    }

    #[cfg(test)]
    pub fn is_right_pressed(&self) -> bool {
        self.is_right_pressed
    }

    #[cfg(test)]
    pub fn is_up_pressed(&self) -> bool {
        self.is_up_pressed
    }

    #[cfg(test)]
    pub fn is_down_pressed(&self) -> bool {
        self.is_down_pressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_creation() {
        let camera = Camera::new(1280, 720);
        assert_eq!(camera.position, Vec3::new(0.0, 0.0, 5.0));
        assert_eq!(camera.target, Vec3::ZERO);
        assert_eq!(camera.up, Vec3::Y);
        assert!((camera.aspect - 1280.0 / 720.0).abs() < 0.001);
        assert_eq!(camera.fovy, 45.0);
        assert_eq!(camera.znear, 0.1);
        assert_eq!(camera.zfar, 100.0);
    }

    #[test]
    fn test_camera_resize() {
        let mut camera = Camera::new(1280, 720);
        let original_aspect = camera.aspect;

        // Resize to a different aspect ratio (4:3 instead of 16:9)
        camera.resize(1024, 768);
        assert!((camera.aspect - 1024.0 / 768.0).abs() < 0.001);
        assert_ne!(camera.aspect, original_aspect);
    }

    #[test]
    fn test_camera_resize_square() {
        let mut camera = Camera::new(1280, 720);
        camera.resize(800, 800);
        assert!((camera.aspect - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_camera_view_projection_matrix() {
        let camera = Camera::new(1280, 720);
        let matrix = camera.build_view_projection_matrix();

        // Matrix should not be all zeros
        let all_zeros = matrix.to_cols_array().iter().all(|&x| x == 0.0);
        assert!(!all_zeros);

        // Matrix should not contain NaN
        let has_nan = matrix.to_cols_array().iter().any(|&x| x.is_nan());
        assert!(!has_nan);
    }

    #[test]
    fn test_camera_controller_creation() {
        let controller = CameraController::new(2.0);
        assert_eq!(controller.speed(), 2.0);
        assert!(!controller.is_forward_pressed());
        assert!(!controller.is_backward_pressed());
        assert!(!controller.is_left_pressed());
        assert!(!controller.is_right_pressed());
        assert!(!controller.is_up_pressed());
        assert!(!controller.is_down_pressed());
    }

    #[test]
    fn test_camera_controller_custom_speed() {
        let controller = CameraController::new(5.0);
        assert_eq!(controller.speed(), 5.0);
    }

    #[test]
    fn test_camera_movement_forward() {
        let mut camera = Camera::new(1280, 720);
        let original_pos = camera.position;

        // Create controller with forward pressed
        let mut active_controller = CameraController::new(2.0);
        active_controller.simulate_forward_press(true);

        active_controller.update_camera(&mut camera, 1.0);

        // Camera should have moved
        assert_ne!(camera.position, original_pos);
    }

    #[test]
    fn test_camera_movement_backward() {
        let mut controller = CameraController::new(2.0);
        let mut camera = Camera::new(1280, 720);
        let original_pos = camera.position;

        controller.simulate_backward_press(true);
        controller.update_camera(&mut camera, 1.0);

        assert_ne!(camera.position, original_pos);
    }

    #[test]
    fn test_camera_movement_strafe() {
        let mut controller = CameraController::new(2.0);
        let mut camera = Camera::new(1280, 720);
        let original_pos = camera.position;

        controller.simulate_right_press(true);
        controller.update_camera(&mut camera, 1.0);

        assert_ne!(camera.position, original_pos);
    }

    #[test]
    fn test_camera_movement_vertical() {
        let mut controller = CameraController::new(2.0);
        let mut camera = Camera::new(1280, 720);
        let original_pos = camera.position;

        controller.simulate_up_press(true);
        controller.update_camera(&mut camera, 1.0);

        assert_ne!(camera.position, original_pos);
        assert!(camera.position.y > original_pos.y);
    }

    #[test]
    fn test_camera_no_movement_when_idle() {
        let controller = CameraController::new(2.0);
        let mut camera = Camera::new(1280, 720);
        let original_pos = camera.position;

        controller.update_camera(&mut camera, 1.0);

        // Position should remain the same (within floating point precision)
        assert!((camera.position - original_pos).length() < 0.01);
    }

    #[test]
    fn test_camera_movement_scales_with_delta_time() {
        let mut controller = CameraController::new(2.0);
        let mut camera1 = Camera::new(1280, 720);
        let mut camera2 = Camera::new(1280, 720);

        controller.simulate_forward_press(true);

        controller.update_camera(&mut camera1, 0.5);
        controller.update_camera(&mut camera2, 1.0);

        // Camera2 should have moved further than camera1
        let dist1 = (camera1.position - Vec3::new(0.0, 0.0, 5.0)).length();
        let dist2 = (camera2.position - Vec3::new(0.0, 0.0, 5.0)).length();
        assert!(dist2 > dist1);
    }

    #[test]
    fn test_camera_target_updates_with_rotation() {
        let mut controller = CameraController::new(2.0);
        let mut camera = Camera::new(1280, 720);
        let original_target = camera.target;

        controller.yaw = 0.5;
        controller.update_camera(&mut camera, 1.0);

        assert_ne!(camera.target, original_target);
    }

    #[test]
    fn test_camera_aspect_ratios() {
        // Test various common aspect ratios
        let test_cases = vec![
            (1920, 1080, 16.0 / 9.0),
            (1280, 720, 16.0 / 9.0),
            (1024, 768, 4.0 / 3.0),
            (800, 600, 4.0 / 3.0),
            (2560, 1440, 16.0 / 9.0),
        ];

        for (width, height, expected_aspect) in test_cases {
            let camera = Camera::new(width, height);
            assert!(
                (camera.aspect - expected_aspect).abs() < 0.01,
                "Failed for {}x{}: expected {}, got {}",
                width,
                height,
                expected_aspect,
                camera.aspect
            );
        }
    }

    #[test]
    fn test_camera_position_range() {
        let camera = Camera::new(1280, 720);

        // Check that initial position is reasonable
        assert!(camera.position.x.abs() < 100.0);
        assert!(camera.position.y.abs() < 100.0);
        assert!(camera.position.z.abs() < 100.0);
    }

    #[test]
    fn test_camera_field_of_view_range() {
        let camera = Camera::new(1280, 720);

        // FOV should be reasonable (between 1 and 180 degrees)
        assert!(camera.fovy > 0.0 && camera.fovy < 180.0);
    }

    #[test]
    fn test_camera_near_far_planes() {
        let camera = Camera::new(1280, 720);

        // Near plane should be less than far plane
        assert!(camera.znear < camera.zfar);

        // Near plane should be positive
        assert!(camera.znear > 0.0);
    }
}
