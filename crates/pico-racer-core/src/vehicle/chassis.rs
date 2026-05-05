//! Chassis dynamics: bicycle model + cosmetic body motion.
//!
//! The simulation tracks pose and velocity of the chassis origin in world
//! space and integrates them under driver inputs.
//! The model is intentionally simplified for arcade feel:
//!
//! - Steering is a kinematic bicycle: `yaw_rate = (v · tan(steer)) / wheelbase`,
//!   modulated by speed so low-speed turning is sharp.
//! - Lateral velocity is damped exponentially (`lateral_grip`), which lets
//!   the car drift when grip is overwhelmed.
//! - Pitch and roll are *cosmetic* — driven by smoothed accelerations, not
//!   by simulated forces — and do not feed back into motion.

use glam::{Mat4, Vec3};
use libm::{cosf, sinf, tanf};

use super::{Controls, VehicleSpec};

/// Chassis state in world space.
#[derive(Clone, Copy, Debug)]
pub struct Chassis {
    /// Position of the chassis origin.
    pub position: Vec3,

    /// Heading in radians around `+Y`. 0 = facing `+Z`.
    pub yaw: f32,

    /// Cosmetic pitch (rotation around the chassis-local X axis), radians.
    pub pitch: f32,

    /// Cosmetic roll (rotation around the chassis-local Z axis), radians.
    pub roll: f32,

    /// World-space linear velocity.
    pub velocity: Vec3,

    /// Yaw rate, radians/second.
    pub yaw_rate: f32,

    /// Current front-wheel steering angle (filtered toward target), radians.
    pub steering_angle: f32,
}

impl Chassis {
    /// Create a chassis at rest at the given pose.
    #[must_use]
    pub fn new(position: Vec3, yaw: f32) -> Self {
        Self {
            position,
            yaw,
            pitch: 0.0,
            roll: 0.0,
            velocity: Vec3::ZERO,
            yaw_rate: 0.0,
            steering_angle: 0.0,
        }
    }

    /// Forward direction in world space (unit length).
    #[must_use]
    pub fn forward(&self) -> Vec3 {
        Vec3::new(sinf(self.yaw), 0.0, cosf(self.yaw))
    }

    /// Right direction in world space (unit length).
    #[must_use]
    pub fn right(&self) -> Vec3 {
        Vec3::new(cosf(self.yaw), 0.0, -sinf(self.yaw))
    }

    /// Signed forward speed (m/s). Positive = moving forward.
    #[must_use]
    pub fn forward_speed(&self) -> f32 {
        self.velocity.dot(self.forward())
    }

    /// Signed lateral speed (m/s). Positive = sliding to chassis-right.
    #[must_use]
    pub fn lateral_speed(&self) -> f32 {
        self.velocity.dot(self.right())
    }

    /// Advance the chassis by one fixed timestep.
    pub fn step(&mut self, dt: f32, controls: &Controls, spec: &VehicleSpec) {
        let target_steering = controls.steering * spec.max_steering;
        self.steering_angle = exp_approach(
            self.steering_angle,
            target_steering,
            spec.steering_response,
            dt,
        );

        let fwd = self.forward();
        let right = self.right();
        let v_fwd_prev = self.velocity.dot(fwd);
        let v_lat_prev = self.velocity.dot(right);

        // Longitudinal acceleration in chassis frame.
        let throttle_accel = if controls.reverse {
            -controls.throttle * spec.max_reverse_accel
        } else {
            controls.throttle * spec.max_accel
        };
        let brake_accel = -v_fwd_prev.signum() * controls.brake * spec.max_brake;
        let drag_accel = -v_fwd_prev.signum() * spec.drag * v_fwd_prev * v_fwd_prev;
        let rolling_accel = if v_fwd_prev.abs() > 0.05 {
            -v_fwd_prev.signum() * spec.rolling_resistance
        } else {
            0.0
        };
        let mut a_fwd = throttle_accel + brake_accel + drag_accel + rolling_accel;

        // Cap at top speed (forward only — reverse uses max_reverse_accel limit).
        let new_v_fwd_unclamped = v_fwd_prev + a_fwd * dt;
        let v_top_signed = if controls.reverse {
            -spec.top_speed * 0.4
        } else {
            spec.top_speed
        };
        let new_v_fwd = if controls.reverse {
            new_v_fwd_unclamped.max(v_top_signed)
        } else {
            new_v_fwd_unclamped.min(v_top_signed)
        };
        // Recompute realised longitudinal acceleration after clamping (used for pitch).
        a_fwd = (new_v_fwd - v_fwd_prev) / dt.max(1e-6);

        // Lateral grip: exponential damping toward zero lateral velocity.
        let grip = if controls.handbrake {
            spec.lateral_grip * spec.handbrake_grip_factor
        } else {
            spec.lateral_grip
        };
        let new_v_lat = v_lat_prev * libm::expf(-grip * dt);
        let a_lat = (new_v_lat - v_lat_prev) / dt.max(1e-6);

        self.velocity = fwd * new_v_fwd + right * new_v_lat;

        // Yaw: kinematic bicycle, scaled so very low speeds still respond.
        let speed_for_yaw = new_v_fwd.abs().max(0.5);
        let target_yaw_rate =
            (speed_for_yaw * tanf(self.steering_angle)) / spec.wheelbase * new_v_fwd.signum();
        // Some yaw bleeds straight through (snappy feel) rather than waiting for grip.
        self.yaw_rate = target_yaw_rate;
        self.yaw += self.yaw_rate * dt;

        // Integrate position with the post-step velocity.
        self.position += self.velocity * dt;

        // Cosmetic body motion. Drive toward target roll/pitch.
        let target_roll = (-a_lat * spec.roll_factor).clamp(-0.35, 0.35);
        let target_pitch = (a_fwd * spec.pitch_factor).clamp(-0.25, 0.25);
        self.roll = exp_approach(self.roll, target_roll, spec.body_response, dt);
        self.pitch = exp_approach(self.pitch, target_pitch, spec.body_response, dt);
    }

    /// World transform of the chassis body for rendering.
    ///
    /// Composition order, outer to inner: translation, yaw, pitch, roll.
    /// Pitch and roll are cosmetic and applied in the chassis-local frame.
    #[must_use]
    pub fn body_transform(&self) -> Mat4 {
        Mat4::from_translation(self.position)
            * Mat4::from_rotation_y(self.yaw)
            * Mat4::from_rotation_x(self.pitch)
            * Mat4::from_rotation_z(self.roll)
    }

    /// World transform of the chassis without cosmetic pitch/roll.
    ///
    /// Use this as the parent transform for wheels — wheels track the
    /// "real" chassis, not the cosmetic body sway.
    #[must_use]
    pub fn kinematic_transform(&self) -> Mat4 {
        Mat4::from_translation(self.position) * Mat4::from_rotation_y(self.yaw)
    }
}

/// Exponential approach: `x_{t+dt} = target - (target - x_t) · exp(-rate · dt)`.
///
/// A continuous-time analog of a low-pass filter; stable for any `dt`.
fn exp_approach(current: f32, target: f32, rate: f32, dt: f32) -> f32 {
    let alpha = libm::expf(-rate * dt);
    target + (current - target) * alpha
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> VehicleSpec {
        VehicleSpec::arcade()
    }

    #[test]
    fn rests_under_no_input() {
        let mut c = Chassis::new(Vec3::ZERO, 0.0);
        let controls = Controls::default();
        for _ in 0..120 {
            c.step(1.0 / 120.0, &controls, &spec());
        }
        assert!(c.velocity.length() < 1e-3);
        assert!(c.position.length() < 1e-3);
    }

    #[test]
    fn accelerates_forward_with_throttle() {
        let mut c = Chassis::new(Vec3::ZERO, 0.0);
        let controls = Controls {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..120 {
            c.step(1.0 / 120.0, &controls, &spec());
        }
        // After 1 s of full throttle, forward speed should be substantial and
        // motion should be along +Z (yaw = 0 → forward = +Z).
        assert!(c.forward_speed() > 5.0);
        assert!(c.position.z > 2.0);
        assert!(c.position.x.abs() < 0.1);
    }

    #[test]
    fn yaws_with_steering() {
        let mut c = Chassis::new(Vec3::ZERO, 0.0);
        let controls = Controls {
            throttle: 0.6,
            steering: 1.0,
            ..Default::default()
        };
        for _ in 0..240 {
            c.step(1.0 / 120.0, &controls, &spec());
        }
        // Right steering with forward throttle should produce positive yaw
        // (turning toward +X).
        assert!(c.yaw > 0.1, "yaw = {}", c.yaw);
    }
}
