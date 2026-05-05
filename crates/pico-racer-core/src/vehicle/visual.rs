//! Bundle of render transforms produced by a vehicle each frame.
//!
//! A vehicle's visual state is just the chassis body matrix plus per-wheel
//! matrices.
//! This module exists so renderers can request the bundle once per frame
//! without separately calling [`super::Vehicle::body_transform`] and
//! [`super::Vehicle::wheel_transforms`].

use glam::Mat4;

use super::{suspension::NUM_WHEELS, Vehicle};

/// All transforms needed to draw a vehicle.
#[derive(Clone, Copy, Debug)]
pub struct VehicleVisual {
    /// World transform for the chassis body mesh.
    pub body: Mat4,

    /// World transforms for the four wheel meshes, in [`super::WheelIndex`] order.
    pub wheels: [Mat4; NUM_WHEELS],
}

impl VehicleVisual {
    /// Snapshot the visual transforms of the given vehicle.
    #[must_use]
    pub fn from_vehicle(vehicle: &Vehicle) -> Self {
        Self {
            body: vehicle.body_transform(),
            wheels: vehicle.wheel_transforms(),
        }
    }
}
