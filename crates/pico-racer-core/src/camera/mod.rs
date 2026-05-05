//! Game cameras: chase and bumper.
//!
//! Cameras are updated each render frame (not each sim tick) — they're
//! presentation, not simulation.
//! Position is smoothed with an exponential filter so the camera glides
//! rather than snapping.

use glam::{Mat4, Vec3};

use crate::vehicle::Vehicle;

/// Camera viewpoint mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraMode {
    /// Third-person chase camera, behind and above the vehicle.
    Chase,
    /// First-person bumper-mounted camera looking forward.
    Bumper,
}

/// Tunable parameters for camera behaviour.
#[derive(Clone, Copy, Debug)]
pub struct CameraSpec {
    /// Chase: distance behind the vehicle, metres.
    pub chase_distance: f32,

    /// Chase: height above the vehicle, metres.
    pub chase_height: f32,

    /// Chase: how far ahead of the vehicle the camera looks, metres.
    pub chase_lookahead: f32,

    /// Bumper: forward offset from the chassis origin, metres.
    pub bumper_forward: f32,

    /// Bumper: vertical offset from the chassis origin, metres.
    pub bumper_height: f32,

    /// Position smoothing rate, 1/s. Higher = stiffer follow.
    pub follow_response: f32,

    /// Vertical field of view, radians.
    pub fov_y: f32,
}

impl CameraSpec {
    /// Sensible defaults for an arcade chase/bumper feel.
    #[must_use]
    pub const fn default_arcade() -> Self {
        Self {
            chase_distance: 5.5,
            chase_height: 2.2,
            chase_lookahead: 6.0,
            bumper_forward: 1.7,
            bumper_height: 0.9,
            follow_response: 5.0,
            fov_y: 1.10, // ~63 degrees
        }
    }
}

impl Default for CameraSpec {
    fn default() -> Self {
        Self::default_arcade()
    }
}

/// A game camera that follows a vehicle.
pub struct Camera {
    /// Active mode.
    pub mode: CameraMode,

    /// Tuning.
    pub spec: CameraSpec,

    /// Current camera position (smoothed).
    pub position: Vec3,

    /// Current look-at target (smoothed).
    pub target: Vec3,
}

impl Camera {
    /// Create a camera following the given vehicle in the given mode.
    /// Position and target are initialised at their unsmoothed values
    /// (no flyby on first frame).
    #[must_use]
    pub fn new(mode: CameraMode, spec: CameraSpec, vehicle: &Vehicle) -> Self {
        let (position, target) = compute_pose(mode, &spec, vehicle);
        Self {
            mode,
            spec,
            position,
            target,
        }
    }

    /// Update camera position and look-at by one render-frame timestep.
    pub fn update(&mut self, vehicle: &Vehicle, dt: f32) {
        let (target_pos, target_look) = compute_pose(self.mode, &self.spec, vehicle);
        let alpha = libm::expf(-self.spec.follow_response * dt);
        self.position = target_pos + (self.position - target_pos) * alpha;
        self.target = target_look + (self.target - target_look) * alpha;
    }

    /// View matrix for this camera (right-handed look-at, world `+Y` up).
    #[must_use]
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, Vec3::Y)
    }

    /// Perspective projection matrix for the given aspect ratio.
    ///
    /// # Arguments
    ///
    /// * `aspect` - Viewport width / height.
    /// * `near` - Near clip distance, metres.
    /// * `far` - Far clip distance, metres.
    #[must_use]
    pub fn projection(&self, aspect: f32, near: f32, far: f32) -> Mat4 {
        Mat4::perspective_rh(self.spec.fov_y, aspect, near, far)
    }
}

/// Compute the un-smoothed (position, look-target) for a vehicle and mode.
fn compute_pose(mode: CameraMode, spec: &CameraSpec, vehicle: &Vehicle) -> (Vec3, Vec3) {
    let car_pos = vehicle.position();
    let fwd = vehicle.forward();
    match mode {
        CameraMode::Chase => {
            let pos = car_pos - fwd * spec.chase_distance + Vec3::Y * spec.chase_height;
            let look = car_pos + fwd * spec.chase_lookahead;
            (pos, look)
        }
        CameraMode::Bumper => {
            let pos = car_pos + fwd * spec.bumper_forward + Vec3::Y * spec.bumper_height;
            let look = pos + fwd * 20.0;
            (pos, look)
        }
    }
}
