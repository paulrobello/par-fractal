// LOD (Level of Detail) System
// Adaptive quality system that adjusts rendering parameters based on distance,
// camera movement, and performance metrics to maintain smooth framerates.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// QA-025: Single source of truth for the default Balanced LOD distance-zone
/// boundaries (`distance_zones` field of `LODConfig`). Shared by
/// `LODConfig::default()` / `apply_profile()` and `Uniforms::new()`'s
/// `lod_zoneN` defaults so a saved `preferred_gpu_index`-style stale value
/// can't drift the CPU and GPU sides apart.
pub const DEFAULT_LOD_ZONES: [f32; 3] = [10.0, 25.0, 50.0];

/// QA-025: EMA smoothing factor for `LODState::camera_velocity`. Shared by
/// the 3D (`update_motion`) and 2D (`update_motion_2d`) motion paths so both
/// respond identically to a given motion magnitude.
const MOTION_EMA_ALPHA: f32 = 0.3;

/// QA-025: Weight of rotational velocity in the combined motion metric,
/// relative to translation speed. Tuned so a moderate orbit registers above
/// `motion_threshold` at the default sensitivity.
const ROTATION_WEIGHT: f32 = 5.0;

/// LOD strategy for quality adjustment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LODStrategy {
    /// Reduce quality based on distance from camera
    Distance,
    /// Reduce quality during camera movement
    Motion,
    /// Adjust quality to maintain target FPS
    Performance,
    /// Combine all strategies intelligently
    #[default]
    Hybrid,
}

/// Predefined LOD profiles for different use cases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LODProfile {
    /// Balanced approach - good mix of quality and performance
    #[default]
    Balanced,
    /// Prioritize visual quality, less aggressive LOD
    QualityFirst,
    /// Prioritize performance, aggressive LOD
    PerformanceFirst,
    /// Only use distance-based LOD
    DistanceOnly,
    /// Only reduce quality during motion
    MotionOnly,
    /// Custom user-defined configuration
    Custom,
}

/// Quality level preset with all rendering parameters
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct QualityLevel {
    /// Ray marching max iterations
    pub max_steps: u32,
    /// Surface precision threshold
    pub min_distance: f32,
    /// Shadow sample count
    pub shadow_samples: u32,
    /// Shadow ray step factor (higher = less precise, faster)
    pub shadow_step_factor: f32,
    /// Ambient occlusion step size
    pub ao_step_size: f32,
    /// Depth of field sample count
    pub dof_samples: u32,
    /// Resolution multiplier (1.0 = native, 0.5 = half res)
    ///
    /// NOTE: `render_scale` is a UI-exposed no-op today — the renderer always
    /// renders at native resolution. The field is retained so existing
    /// settings.yaml files keep parsing and the slider remains visible behind
    /// a "(not yet applied)" label (see ENH-003 for the wiring plan).
    pub render_scale: f32,
    /// ARC-007: 2D iteration multiplier applied to the zoom-bonus-adjusted
    /// iteration count. LOD previously had no lever for the (often dominant)
    /// 2D escape-time cost; deep-zoom interaction collapsed FPS before any
    /// other LOD knob could help. Ultra keeps the authored value; lower
    /// presets scale it down so 2D panning stays responsive during deep zoom.
    /// Applied in `Uniforms::update` AFTER the `+log2(zoom)×15` bonus so deep
    /// zoom does not re-lose detail it just paid for.
    #[serde(default = "default_iteration_scale")]
    pub iteration_scale: f32,
}

/// Serde default for `QualityLevel::iteration_scale`. Old settings.yaml files
/// predating ARC-007 lack the field; they parse with `1.0` (no scaling), which
/// preserves their pre-ARC-007 behavior exactly.
fn default_iteration_scale() -> f32 {
    1.0
}

impl QualityLevel {
    /// Ultra quality preset - maximum visual fidelity
    pub fn ultra() -> Self {
        Self {
            max_steps: 325,
            min_distance: 0.00035,
            shadow_samples: 128,
            shadow_step_factor: 0.6,
            ao_step_size: 0.12,
            dof_samples: 8,
            render_scale: 1.0,
            iteration_scale: 1.0,
        }
    }

    /// High quality preset - excellent visuals, good performance
    pub fn high() -> Self {
        Self {
            max_steps: 250,
            min_distance: 0.0007,
            shadow_samples: 64,
            shadow_step_factor: 0.7,
            ao_step_size: 0.18,
            dof_samples: 4,
            render_scale: 0.85,
            iteration_scale: 0.85,
        }
    }

    /// Medium quality preset - balanced quality/performance
    pub fn medium() -> Self {
        Self {
            max_steps: 175,
            min_distance: 0.0015,
            shadow_samples: 32,
            shadow_step_factor: 0.8,
            ao_step_size: 0.24,
            dof_samples: 2,
            render_scale: 0.7,
            iteration_scale: 0.6,
        }
    }

    /// Low quality preset - prioritize performance
    pub fn low() -> Self {
        Self {
            max_steps: 100,
            min_distance: 0.003,
            shadow_samples: 16,
            shadow_step_factor: 0.9,
            ao_step_size: 0.35,
            dof_samples: 1,
            render_scale: 0.5,
            iteration_scale: 0.35,
        }
    }

    /// Interpolate between two quality levels
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            max_steps: lerp_u32(self.max_steps, other.max_steps, t),
            min_distance: lerp_f32(self.min_distance, other.min_distance, t),
            shadow_samples: lerp_u32(self.shadow_samples, other.shadow_samples, t),
            shadow_step_factor: lerp_f32(self.shadow_step_factor, other.shadow_step_factor, t),
            ao_step_size: lerp_f32(self.ao_step_size, other.ao_step_size, t),
            dof_samples: lerp_u32(self.dof_samples, other.dof_samples, t),
            render_scale: lerp_f32(self.render_scale, other.render_scale, t),
            iteration_scale: lerp_f32(self.iteration_scale, other.iteration_scale, t),
        }
    }
}

impl Default for QualityLevel {
    fn default() -> Self {
        Self::ultra()
    }
}

/// Main LOD configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LODConfig {
    /// Enable/disable LOD system
    pub enabled: bool,

    /// Active LOD profile
    pub profile: LODProfile,

    /// LOD strategy to use
    pub strategy: LODStrategy,

    /// Target FPS for performance-based LOD
    pub target_fps: f32,

    /// Distance zone boundaries [near/mid, mid/far, far/distant]
    pub distance_zones: [f32; 3],

    /// Camera velocity threshold for motion detection (units/sec)
    pub motion_threshold: f32,

    /// Time to wait after stopping before restoring quality (seconds)
    pub restore_delay: f32,

    /// Quality presets for each LOD level [0=ultra, 1=high, 2=medium, 3=low]
    pub quality_presets: [QualityLevel; 4],

    /// Enable debug visualization
    pub debug_visualization: bool,

    /// Smooth transitions between quality levels
    pub smooth_transitions: bool,

    /// Transition duration in seconds
    pub transition_duration: f32,

    /// Motion detection sensitivity multiplier
    pub motion_sensitivity: f32,

    /// Minimum quality level (won't drop below this)
    pub min_quality_level: usize,

    /// Aggressive mode - more aggressive quality reduction
    pub aggressive_mode: bool,
}

impl Default for LODConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Opt-in by default
            profile: LODProfile::Balanced,
            strategy: LODStrategy::Hybrid,
            target_fps: 60.0,
            distance_zones: DEFAULT_LOD_ZONES,
            motion_threshold: 0.1,
            restore_delay: 0.5,
            quality_presets: [
                QualityLevel::ultra(),
                QualityLevel::high(),
                QualityLevel::medium(),
                QualityLevel::low(),
            ],
            debug_visualization: false,
            smooth_transitions: true,
            transition_duration: 0.3,
            motion_sensitivity: 1.0,
            min_quality_level: 0,
            aggressive_mode: false,
        }
    }
}

impl LODConfig {
    /// Apply a predefined profile to the LOD configuration
    pub fn apply_profile(&mut self, profile: LODProfile) {
        self.profile = profile;

        match profile {
            LODProfile::Balanced => {
                self.strategy = LODStrategy::Hybrid;
                self.target_fps = 60.0;
                self.motion_threshold = 0.1;
                self.motion_sensitivity = 1.0;
                self.restore_delay = 0.5;
                self.smooth_transitions = true;
                self.transition_duration = 0.3;
                self.aggressive_mode = false;
                self.min_quality_level = 0;
                self.distance_zones = DEFAULT_LOD_ZONES;
            }
            LODProfile::QualityFirst => {
                self.strategy = LODStrategy::Hybrid;
                self.target_fps = 45.0; // Lower target = less aggressive
                self.motion_threshold = 0.2; // Higher threshold = less sensitive
                self.motion_sensitivity = 0.7; // Lower sensitivity
                self.restore_delay = 0.3; // Faster restore
                self.smooth_transitions = true;
                self.transition_duration = 0.5; // Longer transitions
                self.aggressive_mode = false;
                self.min_quality_level = 1; // Never drop below High quality
                self.distance_zones = [15.0, 35.0, 70.0]; // Larger zones = better quality
            }
            LODProfile::PerformanceFirst => {
                self.strategy = LODStrategy::Hybrid;
                self.target_fps = 75.0; // Higher target = more aggressive
                self.motion_threshold = 0.05; // Lower threshold = more sensitive
                self.motion_sensitivity = 1.5; // Higher sensitivity
                self.restore_delay = 1.0; // Slower restore
                self.smooth_transitions = false; // Instant transitions
                self.transition_duration = 0.1;
                self.aggressive_mode = true;
                self.min_quality_level = 0; // Can drop to any level
                self.distance_zones = [8.0, 18.0, 35.0]; // Smaller zones = lower quality faster
            }
            LODProfile::DistanceOnly => {
                self.strategy = LODStrategy::Distance;
                self.target_fps = 60.0;
                self.motion_threshold = 0.1;
                self.motion_sensitivity = 1.0;
                self.restore_delay = 0.5;
                self.smooth_transitions = true;
                self.transition_duration = 0.3;
                self.aggressive_mode = false;
                self.min_quality_level = 0;
                self.distance_zones = DEFAULT_LOD_ZONES;
            }
            LODProfile::MotionOnly => {
                self.strategy = LODStrategy::Motion;
                self.target_fps = 60.0;
                self.motion_threshold = 0.08; // Slightly more sensitive
                self.motion_sensitivity = 1.2;
                self.restore_delay = 0.4; // Quick restore
                self.smooth_transitions = true;
                self.transition_duration = 0.2; // Fast transitions
                self.aggressive_mode = false;
                self.min_quality_level = 0;
                self.distance_zones = DEFAULT_LOD_ZONES;
            }
            LODProfile::Custom => {
                // Don't change anything for custom profile
            }
        }
    }

    /// Get the name of the current profile
    pub fn profile_name(&self) -> &'static str {
        match self.profile {
            LODProfile::Balanced => "Balanced",
            LODProfile::QualityFirst => "Quality First",
            LODProfile::PerformanceFirst => "Performance First",
            LODProfile::DistanceOnly => "Distance Only",
            LODProfile::MotionOnly => "Motion Only",
            LODProfile::Custom => "Custom",
        }
    }
}

/// Runtime LOD state tracking
#[derive(Debug, Clone)]
pub struct LODState {
    /// Current active LOD level (0-3)
    pub current_level: usize,

    /// Target LOD level we're transitioning to
    pub target_level: usize,

    /// Transition progress (0.0 = current, 1.0 = target)
    pub transition_progress: f32,

    /// Recent FPS samples for performance tracking
    pub fps_samples: VecDeque<f32>,

    /// QA-026: combined motion magnitude (translation + rotation, in
    /// units/sec) after EMA smoothing. Was a `Vec3` whose `.y` and `.z`
    /// components were always zero — the LOD path only ever consumed `.x`.
    /// Stored as a scalar so the unused components can't mislead callers or
    /// debug overlays.
    pub camera_velocity: f32,

    /// Previous camera position for velocity calculation
    pub prev_camera_pos: Vec3,

    /// Previous camera forward vector for rotation detection
    pub prev_camera_forward: Vec3,

    /// ARC-007: previous 2D zoom (f64 CPU-side) for 2D motion detection.
    /// Updated each frame by `update_motion_2d`.
    pub prev_zoom_2d: f64,
    /// ARC-007: previous 2D center for 2D motion detection.
    pub prev_center_2d: [f64; 2],

    /// Is camera currently moving
    pub is_moving: bool,

    /// Time since camera stopped moving
    pub time_since_stopped: f32,

    /// Current average FPS
    pub current_fps: f32,

    /// Interpolated quality level (during transitions)
    pub active_quality: QualityLevel,

    /// Performance LOD hysteresis: time spent in current FPS range (seconds)
    pub fps_stable_time: f32,

    /// Last calculated performance LOD level (for detecting changes)
    pub last_performance_level: usize,
}

impl LODState {
    pub fn new() -> Self {
        Self {
            current_level: 0,
            target_level: 0,
            transition_progress: 1.0,
            fps_samples: VecDeque::with_capacity(60),
            camera_velocity: 0.0,
            prev_camera_pos: Vec3::ZERO,
            prev_camera_forward: Vec3::Z,
            prev_zoom_2d: 1.0,
            prev_center_2d: [0.0, 0.0],
            is_moving: false,
            time_since_stopped: 0.0,
            current_fps: 60.0,
            active_quality: QualityLevel::ultra(),
            fps_stable_time: 0.0,
            last_performance_level: 0,
        }
    }

    /// Update motion tracking
    pub fn update_motion(
        &mut self,
        camera_pos: Vec3,
        camera_forward: Vec3,
        delta_time: f32,
        motion_threshold: f32,
        motion_sensitivity: f32,
    ) {
        // Calculate translational velocity
        let translation_velocity = if delta_time > 0.0 {
            (camera_pos - self.prev_camera_pos) / delta_time
        } else {
            Vec3::ZERO
        };

        // Calculate rotational velocity (angle change per second).
        // QA-003: clamp the dot product before `acos`. The inputs are
        // normalized direction vectors, but rounding can push the dot
        // product one ulp past ±1.0 — `acos` of that returns NaN, and the
        // EMA below would then propagate the NaN forever (motion tracking
        // never recovers mid-session). The clamp is a no-op for in-range
        // inputs.
        let dot = self
            .prev_camera_forward
            .dot(camera_forward)
            .clamp(-1.0, 1.0);
        let rotation_change = dot.acos();
        let rotation_velocity = if delta_time > 0.0 {
            rotation_change / delta_time
        } else {
            0.0
        };

        // Combine translation and rotation into single motion metric
        let translation_speed = translation_velocity.length();
        let motion_magnitude = translation_speed + rotation_velocity * ROTATION_WEIGHT;

        // Apply sensitivity multiplier
        let adjusted_magnitude = motion_magnitude * motion_sensitivity;

        // Update velocity with exponential moving average (smoothing)
        self.camera_velocity =
            self.camera_velocity * (1.0 - MOTION_EMA_ALPHA) + adjusted_magnitude * MOTION_EMA_ALPHA;

        // Detect motion
        let was_moving = self.is_moving;
        self.is_moving = self.camera_velocity > motion_threshold;

        // Update time since stopped
        if self.is_moving {
            self.time_since_stopped = 0.0;
        } else if was_moving {
            // Just stopped
            self.time_since_stopped = 0.0;
        } else {
            self.time_since_stopped += delta_time;
        }

        // Update previous values for next frame
        self.prev_camera_pos = camera_pos;
        self.prev_camera_forward = camera_forward;
    }

    /// ARC-007: 2D motion detection. The 3D `update_motion` path watches
    /// camera translation/rotation; for 2D fractals the image-affecting
    /// motion is zoom and pan, measured in screen-space units. Without this
    /// path, continuous wheel/pinch zoom never registered as motion, so the
    /// LOD system never engaged its quality levers during deep-zoom
    /// interaction — exactly when FPS collapses.
    ///
    /// Formula (per second):
    ///   zoom_rate = |log2(zoom / prev_zoom)| / dt      (octaves/sec)
    ///   pan_speed = length((center - prev_center) * zoom) / dt
    ///   motion_magnitude = zoom_rate * ZOOM_MOTION_WEIGHT + pan_speed
    ///
    /// The zoom weight (0.5) is tuned so one scroll-wheel notch per frame
    /// (~1.1× zoom ≈ 0.49 octaves/sec at 16ms frame time, scaled back up to
    /// ~30 oct/sec by `/dt`) registers well above the default 0.1 motion
    /// threshold, while sub-ulp jitter from f64 rounding does not.
    pub fn update_motion_2d(
        &mut self,
        zoom: f64,
        center: [f64; 2],
        delta_time: f32,
        motion_threshold: f32,
        motion_sensitivity: f32,
    ) {
        const ZOOM_MOTION_WEIGHT: f64 = 0.5;

        let mut motion_magnitude = 0.0_f64;
        if delta_time > 0.0 && self.prev_zoom_2d > 0.0 {
            let dt = delta_time as f64;
            let zoom_ratio = zoom / self.prev_zoom_2d;
            if zoom_ratio > 0.0 {
                let zoom_rate = zoom_ratio.abs().log2().abs() / dt;
                let dx = (center[0] - self.prev_center_2d[0]) * zoom;
                let dy = (center[1] - self.prev_center_2d[1]) * zoom;
                let pan_speed = (dx * dx + dy * dy).sqrt() / dt;
                motion_magnitude = zoom_rate * ZOOM_MOTION_WEIGHT + pan_speed;
            }
        }

        let adjusted_magnitude = (motion_magnitude as f32) * motion_sensitivity;

        // Reuse the same EMA + hysteresis as the 3D path so the LOD state
        // machine (`is_moving`, `time_since_stopped`) behaves identically.
        self.camera_velocity =
            self.camera_velocity * (1.0 - MOTION_EMA_ALPHA) + adjusted_magnitude * MOTION_EMA_ALPHA;

        let was_moving = self.is_moving;
        self.is_moving = self.camera_velocity > motion_threshold;
        // Moving or just-stopped this frame: reset the idle timer. Otherwise
        // accumulate so restore-delay logic can fire after `restore_delay`.
        if self.is_moving || was_moving {
            self.time_since_stopped = 0.0;
        } else {
            self.time_since_stopped += delta_time;
        }

        self.prev_zoom_2d = zoom;
        self.prev_center_2d = center;
    }

    /// Update FPS tracking
    pub fn update_fps(&mut self, delta_time: f32) {
        if delta_time > 0.0 {
            let fps = 1.0 / delta_time;

            // Add to rolling window
            self.fps_samples.push_back(fps);
            if self.fps_samples.len() > 60 {
                self.fps_samples.pop_front();
            }

            // Calculate average
            if !self.fps_samples.is_empty() {
                let sum: f32 = self.fps_samples.iter().sum();
                self.current_fps = sum / self.fps_samples.len() as f32;
            }
        }
    }

    /// Set target LOD level
    pub fn set_target(&mut self, level: usize) {
        if level != self.target_level {
            self.target_level = level.min(3);
            self.transition_progress = 0.0;
        }
    }

    /// Update transition progress
    pub fn update_transition(&mut self, delta_time: f32, duration: f32) {
        if self.transition_progress < 1.0 {
            if duration > 0.0 {
                self.transition_progress += delta_time / duration;
                self.transition_progress = self.transition_progress.min(1.0);
            } else {
                // Instant transition
                self.transition_progress = 1.0;
            }

            // Update current level when transition completes
            if self.transition_progress >= 1.0 {
                self.current_level = self.target_level;
            }
        }
    }

    /// Get current interpolated quality level
    pub fn get_active_quality(&self, presets: &[QualityLevel; 4], smooth: bool) -> QualityLevel {
        if !smooth || self.transition_progress >= 1.0 {
            // No interpolation, return current preset
            presets[self.current_level]
        } else {
            // Interpolate between current and target
            let t = smoothstep(self.transition_progress);
            presets[self.current_level].lerp(&presets[self.target_level], t)
        }
    }
}

impl Default for LODState {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_u32(a: u32, b: u32, t: f32) -> u32 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u32
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_interpolation() {
        let ultra = QualityLevel::ultra();
        let low = QualityLevel::low();

        let mid = ultra.lerp(&low, 0.5);
        assert!(mid.max_steps > low.max_steps);
        assert!(mid.max_steps < ultra.max_steps);
    }

    /// QA-019 / ARC-007: extend the lerp test to the new `iteration_scale`
    /// field. The contract is the same as every other f32 field in
    /// `QualityLevel`: at t=0 the result equals `self`; across the sweep the
    /// value moves monotonically toward `other`; at t=1 it reaches `other`
    /// within 1 ulp (the `lerp_f32` form `a + (b - a) * t` can round 1-ulp
    /// high/low at t=1.0). Pinned as a regression because `iteration_scale`
    /// is the 2D LOD lever most likely to be silently dropped from the lerp
    /// when a future field is added.
    #[test]
    fn quality_lerp_iteration_scale_monotonic() {
        let ultra = QualityLevel::ultra();
        let low = QualityLevel::low();
        // Sanity: ultra keeps authored (1.0), low degrades (0.35) — so the
        // direction of the interpolation is well-defined.
        assert!(ultra.iteration_scale > low.iteration_scale);

        // t=0 endpoint is exact (a + (b-a)*0 == a, no rounding).
        let at_zero = ultra.lerp(&low, 0.0);
        assert_eq!(at_zero.iteration_scale, ultra.iteration_scale);

        // t=1 endpoint: `lerp_f32(a, b, 1.0)` = a + (b - a), which can round
        // 1 ulp away from b for some a/b pairs. Allow ±1 ulp tolerance — same
        // contract every other f32 lerp field already has implicitly.
        let at_one = ultra.lerp(&low, 1.0);
        let one_ulp = (low.iteration_scale * f32::EPSILON).max(f32::MIN_POSITIVE);
        assert!(
            (at_one.iteration_scale - low.iteration_scale).abs() <= one_ulp,
            "iteration_scale at t=1 ({}) not within 1 ulp of low ({})",
            at_one.iteration_scale,
            low.iteration_scale,
        );

        // Midpoint is strictly between the two — monotonicity check.
        let mid = ultra.lerp(&low, 0.5);
        assert!(
            mid.iteration_scale > low.iteration_scale
                && mid.iteration_scale < ultra.iteration_scale,
            "iteration_scale lerp is not monotonic at t=0.5: got {} for [{}, {}]",
            mid.iteration_scale,
            low.iteration_scale,
            ultra.iteration_scale,
        );

        // Sweep: monotonically non-increasing from ultra→low as t goes 0→1.
        let mut prev = ultra.iteration_scale;
        let mut any_strict = false;
        for step in 1..=10 {
            let t = step as f32 / 10.0;
            let q = ultra.lerp(&low, t);
            assert!(
                q.iteration_scale <= prev,
                "iteration_scale increased at t={t}: {} > {prev}",
                q.iteration_scale,
            );
            any_strict |= q.iteration_scale < prev;
            prev = q.iteration_scale;
        }
        assert!(any_strict, "iteration_scale lerp is flat across the sweep");
    }

    #[test]
    fn test_motion_detection() {
        let mut state = LODState::new();
        let pos1 = Vec3::new(0.0, 0.0, 0.0);
        let pos2 = Vec3::new(1.0, 0.0, 0.0);

        state.update_motion(pos1, Vec3::Z, 0.0, 0.1, 1.0);
        assert!(!state.is_moving);

        state.update_motion(pos2, Vec3::Z, 0.1, 0.1, 1.0);
        assert!(state.is_moving);
    }

    /// QA-003 / QA-019: regression test for the LOD NaN-poisoning bug.
    ///
    /// The dot product of two normalized vectors can land one ulp outside
    /// [-1, 1] due to rounding. Before the clamp in `update_motion`, that
    /// made `acos` return NaN, which the EMA then propagated forever —
    /// `is_moving` could never recover and LOD was wedged mid-session.
    ///
    /// The first case uses bit-identical forwards (dot == 1.0 exactly), the
    /// second uses near-parallel vectors whose dot rounds to 1.0+ε. Both
    /// must leave `camera_velocity` finite after repeated updates.
    #[test]
    fn test_motion_tracking_never_nan() {
        let mut state = LODState::new();

        // Identical forwards: dot == 1.0 exactly; acos(1.0) == 0.0, finite.
        let pos = Vec3::new(0.0, 0.0, 5.0);
        state.update_motion(pos, Vec3::Z, 0.016, 0.1, 1.0);
        state.update_motion(pos, Vec3::Z, 0.016, 0.1, 1.0);
        assert!(
            state.camera_velocity.is_finite(),
            "camera_velocity NaN after identical forwards: {}",
            state.camera_velocity
        );

        // Near-parallel forwards whose dot rounds above 1.0. Construct by
        // perturbing one axis by a sub-ulp-of-the-magnitude amount: this is
        // the exact failure mode the in-the-wild bug exhibited.
        let mut state2 = LODState::new();
        let fwd_a = Vec3::Z;
        // Normalized vector almost-but-not-quite parallel to Z. Even when
        // rounding produces dot > 1.0, the clamp keeps acos finite.
        let fwd_b = Vec3::new(1.0e-8, 0.0, 1.0).normalize();
        state2.update_motion(Vec3::ZERO, fwd_a, 0.016, 0.1, 1.0);
        for _ in 0..10 {
            state2.update_motion(Vec3::ZERO, fwd_b, 0.016, 0.1, 1.0);
        }
        assert!(
            state2.camera_velocity.is_finite(),
            "camera_velocity NaN after near-parallel forwards: {} (dot = {})",
            state2.camera_velocity,
            fwd_a.dot(fwd_b)
        );

        // And the orthogonal case (dot == 0.0, acos == π/2) for completeness.
        let mut state3 = LODState::new();
        state3.update_motion(Vec3::ZERO, Vec3::Z, 0.016, 0.1, 1.0);
        state3.update_motion(Vec3::ZERO, Vec3::X, 0.016, 0.1, 1.0);
        assert!(state3.camera_velocity.is_finite());
    }

    #[test]
    fn test_lod_transition() {
        let mut state = LODState::new();
        state.set_target(2);
        assert_eq!(state.target_level, 2);
        assert_eq!(state.transition_progress, 0.0);

        state.update_transition(0.3, 0.3);
        assert_eq!(state.transition_progress, 1.0);
        assert_eq!(state.current_level, 2);
    }
}
