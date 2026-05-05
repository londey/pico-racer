//! Catmull-Rom centerline spline with arc-length parameterization.
//!
//! A Catmull-Rom spline interpolates its control points (unlike a Bezier),
//! which makes it the natural choice for hand-authored tracks: each control
//! point is a place the road actually goes through.
//!
//! The implementation here is the centripetal Catmull-Rom variant
//! (α = 0.5) which avoids self-intersections that the uniform variant
//! produces near sharp turns.
//!
//! All splines are treated as closed loops (last segment wraps to first).
//!
//! Arc-length tables are built once at construction so position/tangent
//! lookups by arc length are O(log N) with N segments.

use glam::Vec3;

/// Maximum number of control points a track may have.
pub const MAX_CONTROL_POINTS: usize = 128;

/// Number of arc-length samples per segment.
/// More samples = better arc-length accuracy at construction cost.
const SAMPLES_PER_SEGMENT: usize = 16;

/// Closed-loop Catmull-Rom spline through a sequence of control points.
#[derive(Clone, Debug)]
pub struct CenterlineSpline {
    points: heapless::Vec<Vec3, MAX_CONTROL_POINTS>,
    /// Cumulative arc length at each segment boundary.
    /// `segment_arc[i]` is the arc length from segment 0 start to segment `i` start.
    /// Length = `points.len() + 1`; the final entry is the total length.
    segment_arc: heapless::Vec<f32, { MAX_CONTROL_POINTS + 1 }>,
}

impl CenterlineSpline {
    /// Build a closed-loop centripetal Catmull-Rom spline through the given points.
    ///
    /// # Arguments
    ///
    /// * `points` - World-space control points. At least 4 are recommended
    ///   for smooth curvature; fewer is supported but the spline degenerates.
    ///
    /// # Panics
    ///
    /// Panics in debug if `points` is empty.
    #[must_use]
    pub fn from_loop(points: &[Vec3]) -> Self {
        debug_assert!(
            !points.is_empty(),
            "CenterlineSpline requires at least one control point"
        );
        let mut owned: heapless::Vec<Vec3, MAX_CONTROL_POINTS> = heapless::Vec::new();
        for p in points {
            // Truncate silently if oversize; alternative would be Result, but
            // a track of >128 points is a content error caught at authoring.
            if owned.push(*p).is_err() {
                break;
            }
        }
        let mut spline = Self {
            points: owned,
            segment_arc: heapless::Vec::new(),
        };
        spline.rebuild_arc_table();
        spline
    }

    /// All control points.
    #[must_use]
    pub fn control_points(&self) -> &[Vec3] {
        &self.points
    }

    /// Total arc length around the loop.
    #[must_use]
    pub fn total_length(&self) -> f32 {
        *self.segment_arc.last().unwrap_or(&0.0)
    }

    /// World position at the given arc length (wraps).
    #[must_use]
    pub fn position_at(&self, arc_length: f32) -> Vec3 {
        let (seg, t) = self.segment_for_arc(arc_length);
        self.eval_segment(seg, t)
    }

    /// Unit tangent at the given arc length (wraps).
    #[must_use]
    pub fn tangent_at(&self, arc_length: f32) -> Vec3 {
        let (seg, t) = self.segment_for_arc(arc_length);
        self.eval_segment_tangent(seg, t).normalize_or_zero()
    }

    /// Project a world point onto the spline. Returns `(arc_length, segment_t)`.
    ///
    /// `hint_arc_length`, when supplied, restricts the search to a window
    /// of segments around the hint — appropriate for tracking a moving
    /// vehicle frame to frame.
    /// `None` performs a full scan over all segments.
    pub fn project(&self, world: Vec3, hint_arc_length: Option<f32>) -> (f32, f32) {
        if self.points.is_empty() {
            return (0.0, 0.0);
        }
        let total = self.total_length();
        let n = self.points.len();

        let (start_seg, end_seg) = match hint_arc_length {
            Some(h) if total > 0.0 => {
                let (seg, _) = self.segment_for_arc(h);
                let window = (n / 8).max(2);
                (seg as i32 - window as i32, seg as i32 + window as i32)
            }
            _ => (0_i32, n as i32),
        };

        let mut best = ProjectBest::new();
        for s in start_seg..end_seg {
            let seg = ((s % n as i32) + n as i32) as usize % n;
            self.project_into_segment(seg, world, &mut best);
        }
        (best.arc, best.t)
    }

    /// Sample one segment for the projection search and update `best`.
    ///
    /// Sampling is uniform in segment-`t` at [`SAMPLES_PER_SEGMENT`] points;
    /// good enough for our use case, can refine with Newton iteration later.
    fn project_into_segment(&self, seg: usize, world: Vec3, best: &mut ProjectBest) {
        for k in 0..=SAMPLES_PER_SEGMENT {
            let t = k as f32 / SAMPLES_PER_SEGMENT as f32;
            let p = self.eval_segment(seg, t);
            let d2 = (p - world).length_squared();
            if d2 < best.d2 {
                best.d2 = d2;
                best.t = t;
                best.arc = self.arc_for_segment_t(seg, t);
            }
        }
    }

    // ---- internals ----

    fn rebuild_arc_table(&mut self) {
        self.segment_arc.clear();
        let _ = self.segment_arc.push(0.0);
        let mut total = 0.0;
        for seg in 0..self.points.len() {
            let mut prev = self.eval_segment(seg, 0.0);
            for k in 1..=SAMPLES_PER_SEGMENT {
                let t = k as f32 / SAMPLES_PER_SEGMENT as f32;
                let p = self.eval_segment(seg, t);
                total += (p - prev).length();
                prev = p;
            }
            let _ = self.segment_arc.push(total);
        }
    }

    /// Evaluate segment `seg` at parameter `t ∈ [0,1]`.
    /// Segment `seg` runs from `points[seg]` to `points[seg+1]` (wrapping).
    fn eval_segment(&self, seg: usize, t: f32) -> Vec3 {
        let n = self.points.len();
        let p0 = self.points[(seg + n - 1) % n];
        let p1 = self.points[seg % n];
        let p2 = self.points[(seg + 1) % n];
        let p3 = self.points[(seg + 2) % n];
        catmull_rom(p0, p1, p2, p3, t)
    }

    fn eval_segment_tangent(&self, seg: usize, t: f32) -> Vec3 {
        let n = self.points.len();
        let p0 = self.points[(seg + n - 1) % n];
        let p1 = self.points[seg % n];
        let p2 = self.points[(seg + 1) % n];
        let p3 = self.points[(seg + 2) % n];
        catmull_rom_tangent(p0, p1, p2, p3, t)
    }

    /// Find `(segment, t_in_segment)` for a given (wrapped) arc length.
    fn segment_for_arc(&self, arc_length: f32) -> (usize, f32) {
        let total = self.total_length();
        if total <= 0.0 || self.points.is_empty() {
            return (0, 0.0);
        }
        let a = rem_euclid(arc_length, total);
        let n = self.points.len();
        // Linear scan; fine for ≤128 segments.
        for seg in 0..n {
            let lo = self.segment_arc[seg];
            let hi = self.segment_arc[seg + 1];
            if a <= hi {
                return (seg, segment_t_for_arc(a, lo, hi));
            }
        }
        // Should not reach here, but fall back to the last segment.
        (n - 1, 1.0)
    }

    fn arc_for_segment_t(&self, seg: usize, t: f32) -> f32 {
        let lo = self.segment_arc[seg];
        let hi = self.segment_arc[seg + 1];
        lo + (hi - lo) * t
    }
}

/// Running best-match state for [`CenterlineSpline::project`].
struct ProjectBest {
    arc: f32,
    t: f32,
    d2: f32,
}

impl ProjectBest {
    fn new() -> Self {
        Self {
            arc: 0.0,
            t: 0.0,
            d2: f32::INFINITY,
        }
    }
}

/// Per-segment `t` for a (wrapped, monotone) arc length given segment bounds.
fn segment_t_for_arc(a: f32, lo: f32, hi: f32) -> f32 {
    let t = if hi > lo { (a - lo) / (hi - lo) } else { 0.0 };
    t.clamp(0.0, 1.0)
}

/// Centripetal Catmull-Rom interpolation between `p1` and `p2`,
/// using `p0` and `p3` to set tangents.
fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    // Centripetal weighting via knot spacing tj = ti + |pi+1 - pi|^α, α = 0.5.
    let alpha = 0.5;
    let t0 = 0.0;
    let t1 = t0 + libm::powf((p1 - p0).length().max(1e-6), alpha);
    let t2 = t1 + libm::powf((p2 - p1).length().max(1e-6), alpha);
    let t3 = t2 + libm::powf((p3 - p2).length().max(1e-6), alpha);

    let tt = t1 + (t2 - t1) * t;

    let a1 = lerp_t(p0, p1, t0, t1, tt);
    let a2 = lerp_t(p1, p2, t1, t2, tt);
    let a3 = lerp_t(p2, p3, t2, t3, tt);
    let b1 = lerp_t(a1, a2, t0, t2, tt);
    let b2 = lerp_t(a2, a3, t1, t3, tt);
    lerp_t(b1, b2, t1, t2, tt)
}

/// Numerical tangent via central difference; cheap, accurate enough for visuals.
fn catmull_rom_tangent(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let h = 1e-3;
    let a = catmull_rom(p0, p1, p2, p3, (t - h).max(0.0));
    let b = catmull_rom(p0, p1, p2, p3, (t + h).min(1.0));
    (b - a) / (2.0 * h)
}

fn lerp_t(a: Vec3, b: Vec3, ta: f32, tb: f32, t: f32) -> Vec3 {
    if (tb - ta).abs() < 1e-9 {
        return a;
    }
    let u = (t - ta) / (tb - ta);
    a * (1.0 - u) + b * u
}

/// Euclidean remainder of `a / b` (always non-negative for positive `b`).
///
/// `core` does not expose `f32::rem_euclid`, so we provide a `no_std`-friendly
/// equivalent here.
pub(crate) fn rem_euclid(a: f32, b: f32) -> f32 {
    let r = a - b * libm::floorf(a / b);
    // Floating-point rounding can push r slightly outside [0, b); clamp.
    if r < 0.0 {
        r + b
    } else if r >= b {
        r - b
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_track() -> CenterlineSpline {
        // Unit square in XZ.
        CenterlineSpline::from_loop(&[
            Vec3::new(10.0, 0.0, 10.0),
            Vec3::new(-10.0, 0.0, 10.0),
            Vec3::new(-10.0, 0.0, -10.0),
            Vec3::new(10.0, 0.0, -10.0),
        ])
    }

    #[test]
    fn loop_has_positive_length() {
        let s = square_track();
        let l = s.total_length();
        assert!(l > 60.0 && l < 100.0, "len = {}", l);
    }

    #[test]
    fn position_wraps() {
        let s = square_track();
        let l = s.total_length();
        let p_a = s.position_at(0.0);
        let p_b = s.position_at(l);
        assert!((p_a - p_b).length() < 1e-3);
    }

    #[test]
    fn tangent_unit_length() {
        let s = square_track();
        let t = s.tangent_at(s.total_length() * 0.25);
        assert!((t.length() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn project_recovers_arc() {
        let s = square_track();
        let arc = s.total_length() * 0.3;
        let p = s.position_at(arc);
        let (recovered, _) = s.project(p, None);
        assert!(
            (recovered - arc).abs() < 2.0,
            "got {} want {}",
            recovered,
            arc
        );
    }
}
