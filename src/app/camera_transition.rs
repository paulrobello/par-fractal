use crate::camera::{Camera, CameraController};
use glam::Vec3;

pub(super) struct CameraTransition {
    pub(super) active: bool,
    start_position: Vec3,
    start_target: Vec3,
    start_fov: f32,
    end_position: Vec3,
    end_target: Vec3,
    end_fov: f32,
    start_time: std::time::Instant,
    duration: f32,
}

impl CameraTransition {
    pub(super) fn new() -> Self {
        Self {
            active: false,
            start_position: Vec3::ZERO,
            start_target: Vec3::ZERO,
            start_fov: 45.0,
            end_position: Vec3::ZERO,
            end_target: Vec3::ZERO,
            end_fov: 45.0,
            start_time: std::time::Instant::now(),
            duration: 1.0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn start(
        &mut self,
        start_pos: Vec3,
        start_tgt: Vec3,
        start_fov: f32,
        end_pos: Vec3,
        end_tgt: Vec3,
        end_fov: f32,
        duration: f32,
    ) {
        self.active = true;
        self.start_position = start_pos;
        self.start_target = start_tgt;
        self.start_fov = start_fov;
        self.end_position = end_pos;
        self.end_target = end_tgt;
        self.end_fov = end_fov;
        self.start_time = std::time::Instant::now();
        self.duration = duration;
    }

    pub(super) fn update(&self, camera: &mut Camera, controller: &mut CameraController) -> bool {
        if !self.active {
            return false;
        }

        let elapsed = self.start_time.elapsed().as_secs_f32();
        let t = (elapsed / self.duration).min(1.0);

        // Smooth interpolation using smoothstep
        let t_smooth = t * t * (3.0 - 2.0 * t);

        camera.position = self.start_position.lerp(self.end_position, t_smooth);
        camera.target = self.start_target.lerp(self.end_target, t_smooth);
        camera.fovy = self.start_fov + (self.end_fov - self.start_fov) * t_smooth;

        // Update controller to match
        controller.point_at_target(camera.position, camera.target);

        t < 1.0 // Return true if still animating
    }
}
