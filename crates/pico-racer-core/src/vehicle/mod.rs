//! Vehicle simulation: chassis dynamics, suspension, and visual presentation.
//!
//! # Coordinate convention
//!
//! World axes are right-handed with `+Y` up.
//! A vehicle with yaw = 0 faces along `+Z` (its forward direction).
//! Yaw is positive turning toward `+X` when viewed from above (clockwise looking down).
//!
//! # Simulation contract
//!
//! [`Vehicle::step`] is intended to be driven from a fixed-timestep loop
//! (see [`crate::game`]).
//! Visual presentation is read with [`Vehicle::body_transform`] and
//! [`Vehicle::wheel_transforms`] once per render frame.

pub mod chassis;
pub mod controls;
pub mod spec;
pub mod suspension;
pub mod visual;

use glam::{Mat4, Vec3};

use crate::track::Track;

pub use chassis::Chassis;
pub use controls::Controls;
pub use spec::VehicleSpec;
pub use suspension::{Suspension, WheelIndex, NUM_WHEELS};

/// A drivable vehicle: dynamic chassis + suspension + immutable spec.
pub struct Vehicle {
    /// Chassis dynamics state (position, orientation, velocity).
    pub chassis: Chassis,

    /// Per-wheel suspension state.
    pub suspension: Suspension,

    /// Tunable parameters describing this vehicle.
    pub spec: VehicleSpec,
}

impl Vehicle {
    /// Construct a vehicle at rest at the given world position with the given heading.
    ///
    /// # Arguments
    ///
    /// * `spec` - Vehicle tuning parameters.
    /// * `position` - Initial world position of the chassis origin.
    /// * `yaw` - Initial heading in radians (0 = facing `+Z`).
    ///
    /// # Returns
    ///
    /// A new [`Vehicle`] at rest.
    pub fn new(spec: VehicleSpec, position: Vec3, yaw: f32) -> Self {
        let suspension = Suspension::new(&spec);
        let chassis = Chassis::new(position, yaw);
        Self {
            chassis,
            suspension,
            spec,
        }
    }

    /// Advance the simulation by one fixed timestep.
    ///
    /// # Arguments
    ///
    /// * `dt` - Timestep in seconds (expected: [`crate::game::SIM_DT`]).
    /// * `controls` - Driver inputs for this tick.
    /// * `track` - The track, used for terrain height under each wheel.
    pub fn step(&mut self, dt: f32, controls: &Controls, track: &Track) {
        self.chassis.step(dt, controls, &self.spec);
        self.suspension
            .step(dt, &self.chassis, &self.spec, controls, track);
    }

    /// World-space transform of the chassis body, suitable as a model matrix.
    ///
    /// Includes cosmetic pitch and roll driven by accelerations.
    pub fn body_transform(&self) -> Mat4 {
        self.chassis.body_transform()
    }

    /// World-space transforms for each wheel, in [`WheelIndex`] order.
    ///
    /// Each transform is the chassis transform composed with the wheel's
    /// local rest position, current suspension travel, steering angle (front
    /// wheels only), and accumulated spin.
    pub fn wheel_transforms(&self) -> [Mat4; NUM_WHEELS] {
        self.suspension.wheel_transforms(&self.chassis, &self.spec)
    }

    /// Current world-space position of the chassis origin.
    pub fn position(&self) -> Vec3 {
        self.chassis.position
    }

    /// Current heading in radians.
    pub fn yaw(&self) -> f32 {
        self.chassis.yaw
    }

    /// Forward direction of the chassis in world space (unit length).
    pub fn forward(&self) -> Vec3 {
        self.chassis.forward()
    }

    /// World-space velocity vector of the chassis origin.
    pub fn velocity(&self) -> Vec3 {
        self.chassis.velocity
    }

    /// Signed forward speed in m/s (positive = moving forward).
    pub fn forward_speed(&self) -> f32 {
        self.chassis.forward_speed()
    }
}
