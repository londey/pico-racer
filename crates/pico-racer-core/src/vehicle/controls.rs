//! Driver inputs.

/// Per-tick driver inputs to the vehicle simulation.
///
/// All analog axes are normalized; throttle/brake are unsigned `0..=1`,
/// steering is signed `-1..=1` (positive = steer toward `+X` of the chassis).
#[derive(Clone, Copy, Debug, Default)]
pub struct Controls {
    /// Throttle, `0.0` (off) to `1.0` (full).
    pub throttle: f32,

    /// Brake, `0.0` (off) to `1.0` (full).
    pub brake: f32,

    /// Steering, `-1.0` (full left) to `+1.0` (full right).
    pub steering: f32,

    /// Handbrake engaged.
    pub handbrake: bool,

    /// Reverse gear engaged.
    /// When set, throttle accelerates the vehicle backward at
    /// [`crate::vehicle::VehicleSpec::max_reverse_accel`].
    pub reverse: bool,
}

impl Controls {
    /// Clamp all axes into their valid ranges. Useful after summing inputs.
    pub fn sanitize(&mut self) {
        self.throttle = self.throttle.clamp(0.0, 1.0);
        self.brake = self.brake.clamp(0.0, 1.0);
        self.steering = self.steering.clamp(-1.0, 1.0);
    }
}
