//! Game state and the fixed-timestep simulation loop.
//!
//! # Update model
//!
//! - Simulation runs at a *fixed* timestep [`SIM_DT`] = 1/[`SIM_HZ`] s,
//!   independent of render rate.
//! - Each render frame, the host calls [`Game::tick`] with the wall-clock
//!   elapsed since the previous frame.
//!   `tick` accumulates time, runs zero or more sim sub-steps, then updates
//!   the camera (which is a render-rate concern, not a sim-rate concern).
//! - To prevent the spiral of death on a slow frame, the accumulator is
//!   capped at [`MAX_FRAME_DT`].
//!
//! Render code reads [`Vehicle::body_transform`](crate::vehicle::Vehicle::body_transform)
//! and [`Vehicle::wheel_transforms`](crate::vehicle::Vehicle::wheel_transforms)
//! after `tick` returns.

use crate::camera::{Camera, CameraMode, CameraSpec};
use crate::track::Track;
use crate::vehicle::{Controls, Vehicle, VehicleSpec};
use glam::Vec3;

/// Simulation rate in Hz.
pub const SIM_HZ: u32 = 120;

/// Simulation timestep, seconds.
pub const SIM_DT: f32 = 1.0 / SIM_HZ as f32;

/// Maximum render-frame delta the loop will integrate in one call.
/// Anything beyond this is dropped to avoid runaway sub-stepping.
pub const MAX_FRAME_DT: f32 = 0.25;

/// Top-level game state: vehicle + track + camera + driver inputs.
pub struct Game {
    /// The player's vehicle.
    pub vehicle: Vehicle,

    /// The track.
    pub track: Track,

    /// Active camera.
    pub camera: Camera,

    /// Latest driver inputs. The host writes this each frame; the sim reads
    /// it once per tick (so within a single render frame, all sub-steps
    /// share the same inputs — appropriate for slow-changing analog inputs).
    pub controls: Controls,

    accumulator: f32,
}

impl Game {
    /// Construct a new game state with default specs.
    ///
    /// # Arguments
    ///
    /// * `track` - The track to drive on.
    /// * `start_arc_length` - Where on the centerline to spawn (metres).
    /// * `camera_mode` - Initial camera mode.
    #[must_use]
    pub fn new(track: Track, start_arc_length: f32, camera_mode: CameraMode) -> Self {
        let spec = VehicleSpec::arcade();
        let pos = track.to_world(start_arc_length, 0.0) + Vec3::Y * spec.suspension_rest;
        let fwd = track.forward_at(start_arc_length);
        let yaw = libm::atan2f(fwd.x, fwd.z);
        let vehicle = Vehicle::new(spec, pos, yaw);
        let camera = Camera::new(camera_mode, CameraSpec::default_arcade(), &vehicle);
        Self {
            vehicle,
            track,
            camera,
            controls: Controls::default(),
            accumulator: 0.0,
        }
    }

    /// Advance the game by one render-frame's worth of wall-clock time.
    ///
    /// # Arguments
    ///
    /// * `frame_dt` - Wall-clock seconds since the previous render frame.
    pub fn tick(&mut self, frame_dt: f32) {
        self.controls.sanitize();
        self.accumulator = (self.accumulator + frame_dt).min(MAX_FRAME_DT);
        while self.accumulator >= SIM_DT {
            self.vehicle.step(SIM_DT, &self.controls, &self.track);
            self.accumulator -= SIM_DT;
        }
        self.camera.update(&self.vehicle, frame_dt);
    }
}
