//! Per-wheel damped-spring suspension and visual wheel transforms.
//!
//! Each wheel anchors to a fixed chassis-local rest position.
//! At each tick, the terrain height under the wheel is queried and the
//! suspension tries to settle so the wheel touches the ground.
//! Compression travel is integrated as a damped spring response, which gives
//! a believable visual bob over bumps and during braking/acceleration.
//!
//! Suspension does not currently feed back into chassis dynamics; the chassis
//! follows the kinematic bicycle model regardless of wheel contact.
//! That choice keeps the model arcade-simple and stable; per-wheel traction
//! effects can be layered on later.

use glam::{Mat4, Vec3};

use super::{Chassis, Controls, VehicleSpec};
use crate::track::Track;

/// Number of wheels per vehicle.
pub const NUM_WHEELS: usize = 4;

/// Index helpers for the four wheels.
#[derive(Clone, Copy, Debug)]
#[repr(usize)]
pub enum WheelIndex {
    /// Front left.
    FrontLeft = 0,
    /// Front right.
    FrontRight = 1,
    /// Rear left.
    RearLeft = 2,
    /// Rear right.
    RearRight = 3,
}

impl WheelIndex {
    /// All wheels in canonical order (FL, FR, RL, RR).
    pub const ALL: [Self; NUM_WHEELS] = [
        Self::FrontLeft,
        Self::FrontRight,
        Self::RearLeft,
        Self::RearRight,
    ];

    /// True if this wheel is on the front axle (steers).
    #[must_use]
    pub const fn is_front(self) -> bool {
        matches!(self, Self::FrontLeft | Self::FrontRight)
    }
}

/// State of a single wheel.
#[derive(Clone, Copy, Debug, Default)]
pub struct WheelState {
    /// Compression of the spring from rest, metres. Positive = compressed.
    pub travel: f32,

    /// Rate of change of compression, m/s.
    pub travel_velocity: f32,

    /// Accumulated wheel spin angle, radians. Used for visual rotation only.
    pub spin: f32,

    /// True if the wheel is in contact with terrain this tick.
    pub on_ground: bool,
}

/// Suspension for one vehicle.
#[derive(Clone, Debug)]
pub struct Suspension {
    /// State of each wheel, indexed by [`WheelIndex`].
    pub wheels: [WheelState; NUM_WHEELS],

    /// Chassis-local rest positions of the wheel anchors (top of suspension travel).
    pub local_offsets: [Vec3; NUM_WHEELS],
}

impl Suspension {
    /// Build suspension for the given vehicle spec.
    ///
    /// Wheel anchors are placed at `±track_width/2` laterally and `±wheelbase/2`
    /// longitudinally, at the chassis vertical origin.
    #[must_use]
    pub fn new(spec: &VehicleSpec) -> Self {
        let half_track = spec.track_width * 0.5;
        let half_base = spec.wheelbase * 0.5;
        let local_offsets = [
            Vec3::new(-half_track, 0.0, half_base),  // FL
            Vec3::new(half_track, 0.0, half_base),   // FR
            Vec3::new(-half_track, 0.0, -half_base), // RL
            Vec3::new(half_track, 0.0, -half_base),  // RR
        ];
        Self {
            wheels: [WheelState::default(); NUM_WHEELS],
            local_offsets,
        }
    }

    /// Advance suspension one fixed timestep.
    pub fn step(
        &mut self,
        dt: f32,
        chassis: &Chassis,
        spec: &VehicleSpec,
        controls: &Controls,
        track: &Track,
    ) {
        let kin = chassis.kinematic_transform();
        let speed = chassis.forward_speed();

        for i in 0..NUM_WHEELS {
            let local_anchor = self.local_offsets[i];
            let anchor_world = kin.transform_point3(local_anchor);

            // Resting wheel-bottom Y is `anchor_world.y - suspension_rest`.
            // If terrain is above that, the spring compresses; below, the
            // wheel hangs at full droop (we cap at 0 — no extension below rest).
            let terrain_y = track.surface_height(anchor_world);
            let target_compression = (terrain_y - (anchor_world.y - spec.suspension_rest)).max(0.0);
            let target_compression = target_compression.min(spec.suspension_max_travel);

            let displacement = target_compression - self.wheels[i].travel;
            let force = spec.suspension_stiffness * displacement
                - spec.suspension_damping * self.wheels[i].travel_velocity;
            self.wheels[i].travel_velocity += force * dt;
            self.wheels[i].travel += self.wheels[i].travel_velocity * dt;
            self.wheels[i].travel = self.wheels[i].travel.clamp(0.0, spec.suspension_max_travel);

            self.wheels[i].on_ground = target_compression > 0.0;

            // Spin: wheels turn proportional to forward speed; brake locks them.
            let spin_rate = if controls.handbrake && WheelIndex::ALL[i].is_front() {
                speed / spec.wheel_radius
            } else if controls.brake > 0.7 {
                0.0
            } else {
                speed / spec.wheel_radius
            };
            self.wheels[i].spin += spin_rate * dt;
        }
    }

    /// Compute world transforms for each wheel for rendering.
    ///
    /// Each transform places the wheel mesh at its sagged ground position,
    /// applies steering (front wheels), and applies accumulated spin.
    /// Wheel meshes are expected to be authored with their axle along the
    /// chassis-local `X` axis.
    #[must_use]
    pub fn wheel_transforms(&self, chassis: &Chassis, spec: &VehicleSpec) -> [Mat4; NUM_WHEELS] {
        let kin = chassis.kinematic_transform();
        let mut out = [Mat4::IDENTITY; NUM_WHEELS];
        for (i, idx) in WheelIndex::ALL.iter().enumerate() {
            let local_y = spec.axle_drop - self.wheels[i].travel;
            let local_pos = Vec3::new(self.local_offsets[i].x, local_y, self.local_offsets[i].z);
            let steer = if idx.is_front() {
                chassis.steering_angle
            } else {
                0.0
            };
            // Local-space wheel transform: translate, then steer (Y), then spin (X).
            let local = Mat4::from_translation(local_pos)
                * Mat4::from_rotation_y(steer)
                * Mat4::from_rotation_x(self.wheels[i].spin);
            out[i] = kin * local;
        }
        out
    }
}
