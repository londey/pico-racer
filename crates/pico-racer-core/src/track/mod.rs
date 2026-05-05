//! Track representation: spline centerline + width/banking/elevation,
//! plus surface queries used by vehicle suspension and game logic.
//!
//! # Layered design
//!
//! - [`spline::CenterlineSpline`] — pure curve math (Catmull-Rom).
//! - [`Track`] — control-point data plus per-CP width, banking, elevation.
//!   Provides high-level queries: arc-length to world position, world position
//!   to track-space coordinates, terrain height under a point.
//!
//! The visual track mesh is **separate** and lives in the asset pipeline;
//! it is referenced by name/index, not embedded here.
//! This split lets us swap a programmatically generated mesh for a
//! Blender-edited one without touching gameplay code.

pub mod spline;

use glam::{Vec2, Vec3};

pub use spline::CenterlineSpline;

/// One Hermite/Catmull-Rom control point on the track.
///
/// Width and banking are sampled at each control point and linearly
/// interpolated between segments.
#[derive(Clone, Copy, Debug)]
pub struct TrackControlPoint {
    /// World-space position of the centerline at this control point.
    pub position: Vec3,

    /// Half-width of the drivable surface to each side, metres.
    pub half_width: f32,

    /// Banking angle, radians. Positive = banks toward `+normal` (left of forward).
    pub banking: f32,
}

/// A full track: spline + per-CP attributes + visual mesh handle.
#[derive(Clone, Debug)]
pub struct Track {
    /// Catmull-Rom spline through the control points (closed loop).
    pub spline: CenterlineSpline,

    /// Per-control-point attributes (length must match `spline.control_points()`).
    pub control_data: &'static [TrackControlPoint],

    /// Identifier for the visual mesh (lookup in asset table).
    /// Game code that draws the track resolves this to a [`crate::render::MeshPatch`] set.
    pub mesh_id: u32,
}

/// Result of projecting a world position onto the track.
#[derive(Clone, Copy, Debug)]
pub struct TrackQuery {
    /// Arc length along the centerline at the projected point, metres.
    pub arc_length: f32,

    /// Signed lateral offset from the centerline at that arc length, metres.
    /// Positive = left of forward direction.
    pub lateral: f32,

    /// Half-width of the track at this arc length.
    pub half_width: f32,

    /// Surface elevation (`y`) at this arc length, including banking lift.
    pub surface_y: f32,

    /// True if `|lateral| > half_width` — vehicle is off the road.
    pub off_track: bool,
}

impl Track {
    /// Build a track from a slice of control points.
    ///
    /// The slice must have at least 4 entries and is treated as a closed loop.
    ///
    /// # Arguments
    ///
    /// * `control_data` - Static slice of control points + per-CP attributes.
    /// * `mesh_id` - Identifier of the visual mesh in the asset table.
    #[must_use]
    pub fn new(control_data: &'static [TrackControlPoint], mesh_id: u32) -> Self {
        let positions: heapless::Vec<Vec3, { spline::MAX_CONTROL_POINTS }> =
            control_data.iter().map(|cp| cp.position).collect();
        let spline = CenterlineSpline::from_loop(&positions);
        Self {
            spline,
            control_data,
            mesh_id,
        }
    }

    /// Total length of the track centerline, metres.
    #[must_use]
    pub fn length(&self) -> f32 {
        self.spline.total_length()
    }

    /// Project a world point onto the track and return track-space coordinates.
    ///
    /// `hint_arc_length` lets callers (e.g. the player car) bound the search
    /// to a window around their last known arc length, which is far cheaper
    /// than a full scan and is the right thing for a moving vehicle.
    /// Pass `None` for a full scan.
    pub fn project(&self, world: Vec3, hint_arc_length: Option<f32>) -> TrackQuery {
        let (arc, _segment_t) = self.spline.project(world, hint_arc_length);
        let centerline = self.spline.position_at(arc);
        let tangent = self.spline.tangent_at(arc);
        // Track-right is tangent × up, project to XZ.
        let right = Vec3::new(tangent.z, 0.0, -tangent.x).normalize_or_zero();
        let to_point = world - centerline;
        let lateral_signed = -to_point.dot(right); // +lateral = left
        let half_width = self.half_width_at(arc);
        TrackQuery {
            arc_length: arc,
            lateral: lateral_signed,
            half_width,
            surface_y: centerline.y,
            off_track: lateral_signed.abs() > half_width,
        }
    }

    /// Half-width of the track at the given arc length.
    #[must_use]
    pub fn half_width_at(&self, arc_length: f32) -> f32 {
        self.lerp_attr(arc_length, |cp| cp.half_width)
    }

    /// Banking angle at the given arc length, radians.
    #[must_use]
    pub fn banking_at(&self, arc_length: f32) -> f32 {
        self.lerp_attr(arc_length, |cp| cp.banking)
    }

    /// Surface height under a world position.
    ///
    /// For a flat-ish track this is the centerline `y` at the projected arc
    /// length, lifted by banking × lateral offset.
    /// Suspension uses this as the terrain height at each wheel.
    #[must_use]
    pub fn surface_height(&self, world: Vec3) -> f32 {
        let q = self.project(world, None);
        let bank = self.banking_at(q.arc_length);
        // Banking tilts the surface around the tangent: surface y rises
        // linearly with lateral on the inside of the bank.
        q.surface_y + libm::sinf(bank) * q.lateral
    }

    /// Linearly interpolate a per-control-point scalar attribute at a given arc length.
    fn lerp_attr<F: Fn(&TrackControlPoint) -> f32>(&self, arc_length: f32, f: F) -> f32 {
        let n = self.control_data.len() as f32;
        if n < 1.0 {
            return 0.0;
        }
        let total = self.length();
        let u = if total > 0.0 {
            spline::rem_euclid(arc_length / total, 1.0) * n
        } else {
            0.0
        };
        let i0 = libm::floorf(u) as usize % self.control_data.len();
        let i1 = (i0 + 1) % self.control_data.len();
        let frac = u - libm::floorf(u);
        let a = f(&self.control_data[i0]);
        let b = f(&self.control_data[i1]);
        a * (1.0 - frac) + b * frac
    }

    /// Convert track-space coordinates back to a world position on the surface.
    ///
    /// Useful for AI racing lines, start grid placement, etc.
    #[must_use]
    pub fn to_world(&self, arc_length: f32, lateral: f32) -> Vec3 {
        let center = self.spline.position_at(arc_length);
        let tangent = self.spline.tangent_at(arc_length);
        let right = Vec3::new(tangent.z, 0.0, -tangent.x).normalize_or_zero();
        let bank = self.banking_at(arc_length);
        let lift = libm::sinf(bank) * lateral;
        center - right * lateral + Vec3::Y * lift
    }

    /// Track-space tangent at an arc length, projected to the XZ plane (unit length).
    #[must_use]
    pub fn forward_at(&self, arc_length: f32) -> Vec3 {
        let t = self.spline.tangent_at(arc_length);
        let flat = Vec2::new(t.x, t.z).normalize_or_zero();
        Vec3::new(flat.x, 0.0, flat.y)
    }
}
