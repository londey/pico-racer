//! Tunable vehicle parameters.

/// Tunable parameters describing a vehicle's geometry, drivetrain, and feel.
///
/// Units are SI (metres, seconds, kilograms, radians, Newtons) unless noted.
#[derive(Clone, Copy, Debug)]
pub struct VehicleSpec {
    // ---- geometry ----
    /// Distance between front and rear axles.
    pub wheelbase: f32,

    /// Distance between left and right wheels.
    pub track_width: f32,

    /// Wheel radius (used for visual spin).
    pub wheel_radius: f32,

    /// Vertical offset from chassis origin to axle plane (negative = axles below origin).
    pub axle_drop: f32,

    // ---- drivetrain ----
    /// Maximum forward acceleration from full throttle, m/s².
    pub max_accel: f32,

    /// Maximum deceleration from full brake, m/s².
    pub max_brake: f32,

    /// Maximum reverse acceleration, m/s².
    pub max_reverse_accel: f32,

    /// Top forward speed, m/s.
    pub top_speed: f32,

    /// Quadratic drag coefficient: longitudinal deceleration ≈ `drag * v²`.
    pub drag: f32,

    /// Constant rolling resistance, m/s².
    pub rolling_resistance: f32,

    // ---- handling ----
    /// Maximum steering angle of the front wheels, radians.
    pub max_steering: f32,

    /// Steering input low-pass rate, 1/s. Higher = snappier steering.
    pub steering_response: f32,

    /// Lateral velocity damping rate, 1/s. Higher = more grip, less drift.
    pub lateral_grip: f32,

    /// Lateral grip multiplier when handbrake is engaged (typical: 0.2).
    pub handbrake_grip_factor: f32,

    // ---- suspension ----
    /// Suspension rest length: ride height of the chassis above the wheel, m.
    pub suspension_rest: f32,

    /// Suspension spring stiffness, 1/s².
    pub suspension_stiffness: f32,

    /// Suspension damping rate, 1/s.
    pub suspension_damping: f32,

    /// Maximum compression travel, m.
    pub suspension_max_travel: f32,

    // ---- cosmetic body motion ----
    /// Roll angle per unit lateral acceleration, rad·s²/m.
    pub roll_factor: f32,

    /// Pitch angle per unit longitudinal acceleration, rad·s²/m.
    pub pitch_factor: f32,

    /// Cosmetic body angle response rate, 1/s.
    pub body_response: f32,
}

impl VehicleSpec {
    /// A reasonable starting tune for an arcade racer (Ridge-Racer-ish).
    ///
    /// Hand-picked, not derived from any real car.
    /// Tune from here once a track is in place.
    #[must_use]
    pub const fn arcade() -> Self {
        Self {
            wheelbase: 2.6,
            track_width: 1.6,
            wheel_radius: 0.32,
            axle_drop: -0.30,

            max_accel: 12.0,
            max_brake: 18.0,
            max_reverse_accel: 6.0,
            top_speed: 70.0,
            drag: 0.0025,
            rolling_resistance: 0.5,

            max_steering: 0.55,
            steering_response: 6.0,
            lateral_grip: 4.5,
            handbrake_grip_factor: 0.18,

            suspension_rest: 0.45,
            suspension_stiffness: 90.0,
            suspension_damping: 9.0,
            suspension_max_travel: 0.18,

            roll_factor: 0.045,
            pitch_factor: 0.020,
            body_response: 6.0,
        }
    }
}

impl Default for VehicleSpec {
    fn default() -> Self {
        Self::arcade()
    }
}
