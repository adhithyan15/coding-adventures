// ============================================================================
// arc2d — Elliptical Arcs
// ============================================================================
//
// This crate provides two parameterizations of elliptical arcs:
//
//   - `SvgArc` (endpoint form): from, to, rx, ry, x_rotation, large_arc, sweep.
//     This is the form used by SVG's `A` path command — you know where the
//     pen is and where you want the arc to end.
//
//   - `CenterArc` (center form): center, rx, ry, start_angle, sweep_angle,
//     x_rotation. This form is natural for computation: evaluating a point
//     on the arc, finding tangents, and generating cubic Bezier approximations
//     are all much cleaner in center form.
//
// The W3C SVG specification defines the exact algorithm for converting between
// the two forms. This crate implements it faithfully.
//
// ## Converting to Cubic Beziers
//
// Most rendering backends (PDF, CoreGraphics, Skia) do not have a native arc
// command — they use only cubic Bezier curves. We approximate each arc segment
// (up to 90° of sweep) with a cubic Bezier using the standard formula that
// achieves very low error (< 0.027% of the radius for a full circle).

use trig;
use point2d::{Point, Rect};
use bezier2d::CubicBezier;

// ============================================================================
// CenterArc
// ============================================================================

/// An elliptical arc in center form.
///
/// The ellipse is centered at `center`, with semi-axes `rx` (along the
/// rotated X axis) and `ry` (along the rotated Y axis). The ellipse's X axis
/// is rotated by `x_rotation` radians from the coordinate system's X axis.
///
/// The arc sweeps from angle `start_angle` to `start_angle + sweep_angle`
/// (measured in the rotated ellipse frame).
///
/// - Positive `sweep_angle` → counterclockwise in math convention.
/// - Negative `sweep_angle` → clockwise in math convention.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CenterArc {
    /// Center of the ellipse.
    pub center: Point,
    /// Semi-axis in the (rotated) X direction.
    pub rx: f64,
    /// Semi-axis in the (rotated) Y direction.
    pub ry: f64,
    /// Starting angle in radians (measured in the rotated ellipse frame).
    pub start_angle: f64,
    /// Angular span in radians. Positive = CCW, negative = CW.
    pub sweep_angle: f64,
    /// Rotation of the ellipse's X axis from the coordinate X axis, in radians.
    pub x_rotation: f64,
}

impl CenterArc {
    /// Create a CenterArc from its components.
    pub fn new(
        center: Point,
        rx: f64,
        ry: f64,
        start_angle: f64,
        sweep_angle: f64,
        x_rotation: f64,
    ) -> Self {
        Self { center, rx, ry, start_angle, sweep_angle, x_rotation }
    }

    /// Evaluate the arc at normalized parameter t ∈ [0, 1].
    ///
    /// t=0 gives the start point, t=1 gives the end point.
    ///
    /// Algorithm:
    ///   1. Compute the angle: θ = start_angle + t · sweep_angle
    ///   2. Compute the unrotated ellipse point: (rx·cos θ, ry·sin θ)
    ///   3. Apply x_rotation: rotate (x', y') by x_rotation
    ///   4. Translate by center
    pub fn evaluate(&self, t: f64) -> Point {
        let angle = self.start_angle + t * self.sweep_angle;
        // Unrotated ellipse point in the ellipse's local frame.
        let xp = self.rx * trig::cos(angle);
        let yp = self.ry * trig::sin(angle);
        // Apply x_rotation to bring into the coordinate system.
        let cos_r = trig::cos(self.x_rotation);
        let sin_r = trig::sin(self.x_rotation);
        Point::new(
            cos_r * xp - sin_r * yp + self.center.x,
            sin_r * xp + cos_r * yp + self.center.y,
        )
    }

    /// Tangent vector at normalized parameter t (not normalized to unit length).
    ///
    /// The derivative of `evaluate(t)` with respect to t:
    ///   dx'/dt = -rx · sin(angle) · sweep_angle
    ///   dy'/dt =  ry · cos(angle) · sweep_angle
    /// Then apply the x_rotation to the vector.
    pub fn tangent(&self, t: f64) -> Point {
        let angle = self.start_angle + t * self.sweep_angle;
        // Derivative in the ellipse's local frame.
        let dxp = -self.rx * trig::sin(angle) * self.sweep_angle;
        let dyp = self.ry * trig::cos(angle) * self.sweep_angle;
        // Apply x_rotation to the tangent vector (no translation for vectors).
        let cos_r = trig::cos(self.x_rotation);
        let sin_r = trig::sin(self.x_rotation);
        Point::new(
            cos_r * dxp - sin_r * dyp,
            sin_r * dxp + cos_r * dyp,
        )
    }

    /// Bounding box computed by sampling 100 points.
    ///
    /// Computing the exact analytical bounding box of a rotated ellipse arc
    /// requires solving for angles where the tangent is horizontal/vertical,
    /// which becomes complex for arbitrary x_rotation. Sampling 100 points
    /// gives a result accurate to about 1% of the arc length for typical arcs.
    pub fn bounding_box(&self) -> Rect {
        let n = 100;
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for i in 0..=n {
            let t = i as f64 / n as f64;
            let p = self.evaluate(t);
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Approximate this arc with a sequence of cubic Bezier curves.
    ///
    /// The arc is split into segments of at most 90° (π/2 radians). Each
    /// segment is approximated by a cubic Bezier using the standard formula
    /// that minimizes maximum radial error.
    ///
    /// For a segment sweeping angle `s` (≤ π/2):
    ///   k = (4/3) · tan(s/4)
    ///
    /// The four cubic control points for a unit-circle arc from angle α to α+s:
    ///   p0 = (cos α, sin α)              [on the arc]
    ///   p1 = p0 + k · (-sin α, cos α)   [tangent direction]
    ///   p2 = p3 - k · (-sin β, cos β)   [tangent direction at β]
    ///   p3 = (cos β, sin β)              [on the arc]
    ///
    /// Then scale by (rx, ry), apply x_rotation, and translate by center.
    pub fn to_cubic_beziers(&self) -> Vec<CubicBezier> {
        // Maximum sweep per segment: 90° = π/2.
        let max_seg = trig::PI / 2.0;
        let n_segs = (self.sweep_angle.abs() / max_seg).ceil() as usize;
        let n_segs = n_segs.max(1); // at least one segment

        let seg_sweep = self.sweep_angle / n_segs as f64;
        let cos_r = trig::cos(self.x_rotation);
        let sin_r = trig::sin(self.x_rotation);

        let mut beziers = Vec::with_capacity(n_segs);

        for i in 0..n_segs {
            let alpha = self.start_angle + i as f64 * seg_sweep;
            let beta = alpha + seg_sweep;

            // The magic constant for the cubic approximation of a circular arc.
            // Derived from minimizing the max radial error of the cubic approximation.
            // For a segment of sweep s: k = (4/3) * tan(s/4).
            let k = (4.0 / 3.0) * trig::tan(seg_sweep / 4.0);

            // Compute the four points in the ellipse's local (unrotated) frame.
            let cos_a = trig::cos(alpha);
            let sin_a = trig::sin(alpha);
            let cos_b = trig::cos(beta);
            let sin_b = trig::sin(beta);

            // p0 and p3 are on the ellipse.
            let p0_local = (self.rx * cos_a, self.ry * sin_a);
            let p3_local = (self.rx * cos_b, self.ry * sin_b);

            // p1 is p0 displaced along the tangent at alpha.
            // Tangent at alpha in local frame: (-rx*sin_a, ry*cos_a).
            let p1_local = (
                p0_local.0 + k * (-self.rx * sin_a),
                p0_local.1 + k * (self.ry * cos_a),
            );

            // p2 is p3 displaced backward along the tangent at beta.
            // Tangent at beta in local frame: (-rx*sin_b, ry*cos_b).
            let p2_local = (
                p3_local.0 - k * (-self.rx * sin_b),
                p3_local.1 - k * (self.ry * cos_b),
            );

            // Apply x_rotation and translate by center to get world coordinates.
            let rotate_translate = |lx: f64, ly: f64| -> Point {
                Point::new(
                    cos_r * lx - sin_r * ly + self.center.x,
                    sin_r * lx + cos_r * ly + self.center.y,
                )
            };

            beziers.push(CubicBezier::new(
                rotate_translate(p0_local.0, p0_local.1),
                rotate_translate(p1_local.0, p1_local.1),
                rotate_translate(p2_local.0, p2_local.1),
                rotate_translate(p3_local.0, p3_local.1),
            ));
        }

        beziers
    }
}

// ============================================================================
// SvgArc
// ============================================================================

/// An elliptical arc in SVG endpoint form.
///
/// This is the form used in SVG's `A` path command:
///   A rx ry x-rotation large-arc-flag sweep-flag x y
///
/// The arc connects `from` to `to` along an ellipse with semi-axes `rx`, `ry`,
/// rotated by `x_rotation` radians. The `large_arc` and `sweep` flags select
/// which of the four possible arcs is intended.
///
/// - `sweep=true`:  counterclockwise in math convention (Y-up).
/// - `large_arc=true`: use the arc spanning more than 180°.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvgArc {
    /// Start point of the arc (current pen position).
    pub from: Point,
    /// End point of the arc.
    pub to: Point,
    /// X-axis semi-radius (≥ 0).
    pub rx: f64,
    /// Y-axis semi-radius (≥ 0).
    pub ry: f64,
    /// Rotation of the ellipse's X axis, in radians.
    pub x_rotation: f64,
    /// True: use the arc spanning > 180°. False: use the arc spanning ≤ 180°.
    pub large_arc: bool,
    /// True: counterclockwise sweep. False: clockwise sweep.
    pub sweep: bool,
}

impl SvgArc {
    /// Create an SvgArc.
    pub fn new(
        from: Point,
        to: Point,
        rx: f64,
        ry: f64,
        x_rotation: f64,
        large_arc: bool,
        sweep: bool,
    ) -> Self {
        Self { from, to, rx, ry, x_rotation, large_arc, sweep }
    }

    /// Convert to center form using the W3C SVG specification algorithm.
    ///
    /// Returns `None` for degenerate cases:
    ///   - from == to (zero-length arc)
    ///   - rx == 0 or ry == 0 (degenerate ellipse, should be a line)
    ///
    /// The algorithm has 8 steps:
    ///
    /// **Step 1**: Rotate the midpoint into the ellipse's local frame.
    ///
    /// **Step 2**: Ensure the radii are large enough to connect the endpoints.
    ///   If they are too small, scale them up proportionally.
    ///
    /// **Step 3**: Compute the center in the rotated frame.
    ///
    /// **Step 4**: Rotate the center back to the original frame.
    ///
    /// **Step 5–6**: Compute start_angle and sweep_angle using atan2.
    ///
    /// **Step 7**: Adjust sweep_angle for the large_arc and sweep flags.
    pub fn to_center_arc(&self) -> Option<CenterArc> {
        // Degenerate: same start and end point.
        if (self.from.x - self.to.x).abs() < 1e-12
            && (self.from.y - self.to.y).abs() < 1e-12
        {
            return None;
        }
        // Degenerate: zero radius means it's a line, not an arc.
        if self.rx.abs() < 1e-12 || self.ry.abs() < 1e-12 {
            return None;
        }

        let cos_r = trig::cos(self.x_rotation);
        let sin_r = trig::sin(self.x_rotation);

        // Step 1: Compute (x1', y1') in the rotated frame.
        // The midpoint vector (from→to)/2 is rotated by -x_rotation.
        let dx = (self.from.x - self.to.x) / 2.0;
        let dy = (self.from.y - self.to.y) / 2.0;
        let x1p = cos_r * dx + sin_r * dy;   // note: +sin for -rotation
        let y1p = -sin_r * dx + cos_r * dy;

        // Step 2: Ensure radii are large enough.
        // lambda = (x1'/rx)² + (y1'/ry)²
        // If lambda > 1, the endpoints are too far apart for the given radii.
        // Scale up: rx *= sqrt(lambda), ry *= sqrt(lambda).
        let mut rx = self.rx.abs();
        let mut ry = self.ry.abs();
        let lambda = (x1p / rx) * (x1p / rx) + (y1p / ry) * (y1p / ry);
        if lambda > 1.0 {
            let sqrt_lambda = trig::sqrt(lambda);
            rx *= sqrt_lambda;
            ry *= sqrt_lambda;
        }

        // Step 3: Compute the center (cx', cy') in the rotated frame.
        let rx2 = rx * rx;
        let ry2 = ry * ry;
        let x1p2 = x1p * x1p;
        let y1p2 = y1p * y1p;

        // The formula from the W3C spec:
        //   sq = sqrt(max(0, (rx²*ry² - rx²*y1'² - ry²*x1'²) / (rx²*y1'² + ry²*x1'²)))
        // sign = +1 if large_arc ≠ sweep, else -1.
        let num = rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2;
        let den = rx2 * y1p2 + ry2 * x1p2;

        // Clamp num to 0 to handle floating-point rounding near the boundary.
        let sq = if den.abs() < 1e-12 {
            0.0
        } else {
            trig::sqrt((num / den).max(0.0))
        };

        // The sign distinguishes which of the two possible centers to use.
        let sign = if self.large_arc == self.sweep { -1.0 } else { 1.0 };

        // Center in the rotated frame.
        let cxp = sign * sq * (rx * y1p / ry);
        let cyp = sign * sq * -(ry * x1p / rx);

        // Step 4: Rotate back and translate to get the real center.
        let mid_x = (self.from.x + self.to.x) / 2.0;
        let mid_y = (self.from.y + self.to.y) / 2.0;
        let cx = cos_r * cxp - sin_r * cyp + mid_x;
        let cy = sin_r * cxp + cos_r * cyp + mid_y;

        // Step 5–6: Compute start_angle and sweep_angle.
        // The "angle between" function: angle_between(u, v) = atan2(u×v, u·v)
        // where × is the 2D cross product and · is the dot product.
        //
        // start_angle: angle from (1,0) to the vector pointing to the start point
        // in the normalized ellipse frame.
        let ux = (x1p - cxp) / rx;
        let uy = (y1p - cyp) / ry;
        let vx = (-x1p - cxp) / rx;
        let vy = (-y1p - cyp) / ry;

        let start_angle = angle_between(1.0, 0.0, ux, uy);
        let mut sweep_angle = angle_between(ux, uy, vx, vy);

        // Step 7: Adjust sweep_angle for the large_arc and sweep flags.
        if !self.sweep && sweep_angle > 0.0 {
            // Clockwise: make sweep negative.
            sweep_angle -= 2.0 * trig::PI;
        }
        if self.sweep && sweep_angle < 0.0 {
            // Counterclockwise: make sweep positive.
            sweep_angle += 2.0 * trig::PI;
        }

        Some(CenterArc::new(
            Point::new(cx, cy),
            rx,
            ry,
            start_angle,
            sweep_angle,
            self.x_rotation,
        ))
    }

    /// Convert to a sequence of cubic Bezier curves.
    ///
    /// Delegates to `to_center_arc().to_cubic_beziers()`.
    /// Returns an empty vec if the arc is degenerate.
    pub fn to_cubic_beziers(&self) -> Vec<CubicBezier> {
        match self.to_center_arc() {
            Some(ca) => ca.to_cubic_beziers(),
            None => vec![],
        }
    }

    /// Evaluate the arc at parameter t ∈ [0, 1].
    ///
    /// Returns `None` if the arc is degenerate.
    pub fn evaluate(&self, t: f64) -> Option<Point> {
        self.to_center_arc().map(|ca| ca.evaluate(t))
    }

    /// Compute the bounding box of the arc.
    ///
    /// Returns `None` if the arc is degenerate.
    pub fn bounding_box(&self) -> Option<Rect> {
        self.to_center_arc().map(|ca| ca.bounding_box())
    }
}

// ============================================================================
// Helper: angle between two 2D vectors
// ============================================================================

/// Compute the signed angle from vector (ux, uy) to vector (vx, vy).
///
/// Uses atan2(cross, dot) where:
///   cross = ux*vy - uy*vx  (2D cross product — positive = CCW turn)
///   dot   = ux*vx + uy*vy  (dot product)
///
/// Result is in (-π, π].
fn angle_between(ux: f64, uy: f64, vx: f64, vy: f64) -> f64 {
    let cross = ux * vy - uy * vx;
    let dot = ux * vx + uy * vy;
    trig::atan2(cross, dot)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-6;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    fn point_approx_eq(a: Point, b: Point) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y)
    }

    // -----------------------------------------------------------------------
    // CenterArc: evaluate
    // -----------------------------------------------------------------------

    #[test]
    fn test_center_arc_unit_circle_endpoints() {
        // Unit circle arc from 0 to π/2.
        let arc = CenterArc::new(
            Point::origin(),
            1.0,
            1.0,
            0.0,
            trig::PI / 2.0,
            0.0,
        );
        let start = arc.evaluate(0.0);
        let end = arc.evaluate(1.0);
        assert!(approx_eq(start.x, 1.0));
        assert!(approx_eq(start.y, 0.0));
        assert!(approx_eq(end.x, 0.0));
        assert!(approx_eq(end.y, 1.0));
    }

    #[test]
    fn test_center_arc_full_circle_midpoint() {
        // Full circle arc from 0 to 2π.
        let arc = CenterArc::new(
            Point::origin(),
            2.0,
            2.0,
            0.0,
            2.0 * trig::PI,
            0.0,
        );
        let mid = arc.evaluate(0.5); // t=0.5 → angle = π
        assert!(approx_eq(mid.x, -2.0));
        assert!(approx_eq(mid.y, 0.0));
    }

    #[test]
    fn test_center_arc_ellipse_evaluate() {
        // Ellipse with rx=2, ry=1, no rotation.
        let arc = CenterArc::new(
            Point::origin(),
            2.0,
            1.0,
            0.0,
            trig::PI / 2.0,
            0.0,
        );
        let start = arc.evaluate(0.0);
        let end = arc.evaluate(1.0);
        assert!(approx_eq(start.x, 2.0));
        assert!(approx_eq(start.y, 0.0));
        assert!(approx_eq(end.x, 0.0));
        assert!(approx_eq(end.y, 1.0));
    }

    #[test]
    fn test_center_arc_with_center_offset() {
        let arc = CenterArc::new(
            Point::new(5.0, 3.0),
            1.0,
            1.0,
            0.0,
            trig::PI / 2.0,
            0.0,
        );
        let start = arc.evaluate(0.0);
        // Should be center + (rx*cos(0), ry*sin(0)) = (6, 3)
        assert!(approx_eq(start.x, 6.0));
        assert!(approx_eq(start.y, 3.0));
    }

    // -----------------------------------------------------------------------
    // CenterArc: tangent
    // -----------------------------------------------------------------------

    #[test]
    fn test_center_arc_tangent_direction() {
        // Unit circle, quarter arc. Tangent at start (t=0) should point upward.
        let arc = CenterArc::new(
            Point::origin(),
            1.0,
            1.0,
            0.0,
            trig::PI / 2.0,
            0.0,
        );
        let t0 = arc.tangent(0.0);
        // Tangent at angle=0: dx=-rx*sin(0)*sweep = 0, dy=ry*cos(0)*sweep = π/2
        assert!(approx_eq(t0.x, 0.0));
        assert!(t0.y > 0.0); // pointing upward
    }

    // -----------------------------------------------------------------------
    // CenterArc: bounding_box
    // -----------------------------------------------------------------------

    #[test]
    fn test_center_arc_bounding_box_unit_circle() {
        let arc = CenterArc::new(
            Point::origin(),
            1.0,
            1.0,
            0.0,
            2.0 * trig::PI,
            0.0,
        );
        let bb = arc.bounding_box();
        assert!((bb.x + 1.0).abs() < 0.05); // min x ≈ -1
        assert!((bb.y + 1.0).abs() < 0.05); // min y ≈ -1
        assert!((bb.width - 2.0).abs() < 0.05); // width ≈ 2
        assert!((bb.height - 2.0).abs() < 0.05); // height ≈ 2
    }

    // -----------------------------------------------------------------------
    // CenterArc: to_cubic_beziers
    // -----------------------------------------------------------------------

    #[test]
    fn test_center_arc_quarter_circle_one_bezier() {
        let arc = CenterArc::new(
            Point::origin(),
            1.0,
            1.0,
            0.0,
            trig::PI / 2.0,
            0.0,
        );
        let beziers = arc.to_cubic_beziers();
        assert_eq!(beziers.len(), 1);
    }

    #[test]
    fn test_center_arc_full_circle_four_beziers() {
        let arc = CenterArc::new(
            Point::origin(),
            1.0,
            1.0,
            0.0,
            2.0 * trig::PI,
            0.0,
        );
        let beziers = arc.to_cubic_beziers();
        assert_eq!(beziers.len(), 4); // 360° / 90° = 4 segments
    }

    #[test]
    fn test_center_arc_beziers_endpoint_continuity() {
        // Adjacent beziers should share their endpoints.
        let arc = CenterArc::new(
            Point::origin(),
            1.0,
            1.0,
            0.0,
            2.0 * trig::PI,
            0.0,
        );
        let beziers = arc.to_cubic_beziers();
        for i in 0..beziers.len() - 1 {
            let end = beziers[i].p3;
            let start = beziers[i + 1].p0;
            assert!(
                (end.x - start.x).abs() < 1e-6 && (end.y - start.y).abs() < 1e-6,
                "Bezier {} end doesn't match Bezier {} start", i, i + 1
            );
        }
    }

    #[test]
    fn test_center_arc_bezier_approximation_accuracy() {
        // The cubic approximation of a quarter circle should be accurate.
        let arc = CenterArc::new(
            Point::origin(),
            1.0,
            1.0,
            0.0,
            trig::PI / 2.0,
            0.0,
        );
        let beziers = arc.to_cubic_beziers();
        let b = &beziers[0];
        // The midpoint of the bezier should be close to the midpoint of the arc.
        let arc_mid = arc.evaluate(0.5);
        let bez_mid = b.evaluate(0.5);
        let err = arc_mid.distance(bez_mid);
        // Known error for the standard formula is < 0.027% of radius.
        assert!(err < 0.001, "Cubic approximation error too large: {}", err);
    }

    // -----------------------------------------------------------------------
    // SvgArc: degenerate cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_svg_arc_same_start_end_returns_none() {
        let arc = SvgArc::new(
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            1.0, 1.0, 0.0, false, true,
        );
        assert!(arc.to_center_arc().is_none());
    }

    #[test]
    fn test_svg_arc_zero_radius_returns_none() {
        let arc = SvgArc::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            0.0, 1.0, 0.0, false, true,
        );
        assert!(arc.to_center_arc().is_none());
    }

    // -----------------------------------------------------------------------
    // SvgArc: conversion to center form
    // -----------------------------------------------------------------------

    #[test]
    fn test_svg_arc_quarter_circle_conversion() {
        // Quarter circle arc from (1,0) to (0,1) on a unit circle.
        // Center should be at origin.
        let arc = SvgArc::new(
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
            1.0, 1.0, 0.0, false, true,
        );
        let ca = arc.to_center_arc().expect("should not be degenerate");
        assert!(approx_eq(ca.center.x, 0.0));
        assert!(approx_eq(ca.center.y, 0.0));
        assert!(approx_eq(ca.rx, 1.0));
        assert!(approx_eq(ca.ry, 1.0));
    }

    #[test]
    fn test_svg_arc_evaluate_endpoint() {
        let arc = SvgArc::new(
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
            1.0, 1.0, 0.0, false, true,
        );
        let start = arc.evaluate(0.0).expect("should evaluate");
        // The start point of the center arc should be the `from` point.
        assert!((start.x - 1.0).abs() < 0.001);
        assert!((start.y - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_svg_arc_to_cubic_beziers_nonempty() {
        let arc = SvgArc::new(
            Point::new(1.0, 0.0),
            Point::new(-1.0, 0.0),
            1.0, 1.0, 0.0, true, true,
        );
        let beziers = arc.to_cubic_beziers();
        assert!(!beziers.is_empty());
    }

    #[test]
    fn test_svg_arc_bounding_box_some() {
        let arc = SvgArc::new(
            Point::new(1.0, 0.0),
            Point::new(-1.0, 0.0),
            1.0, 1.0, 0.0, true, true,
        );
        assert!(arc.bounding_box().is_some());
    }

    #[test]
    fn test_svg_arc_sweep_flag_matters() {
        // The same endpoints with different sweep flags should give different arcs.
        let arc_ccw = SvgArc::new(
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
            1.0, 1.0, 0.0, false, true, // CCW
        );
        let arc_cw = SvgArc::new(
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
            1.0, 1.0, 0.0, false, false, // CW
        );
        let ca_ccw = arc_ccw.to_center_arc().unwrap();
        let ca_cw = arc_cw.to_center_arc().unwrap();
        // The sweep angles should have opposite signs.
        assert!(ca_ccw.sweep_angle > 0.0);
        assert!(ca_cw.sweep_angle < 0.0);
    }

    #[test]
    fn test_svg_arc_large_arc_flag_matters() {
        let arc_small = SvgArc::new(
            Point::new(1.0, 0.0),
            Point::new(-1.0, 0.0),
            1.0, 1.0, 0.0, false, true, // small arc
        );
        let arc_large = SvgArc::new(
            Point::new(1.0, 0.0),
            Point::new(-1.0, 0.0),
            1.0, 1.0, 0.0, true, true, // large arc
        );
        let ca_small = arc_small.to_center_arc().unwrap();
        let ca_large = arc_large.to_center_arc().unwrap();
        // Large arc should have a sweep_angle > π; small arc ≤ π.
        assert!(ca_small.sweep_angle.abs() <= trig::PI + 1e-6);
        assert!(ca_large.sweep_angle.abs() > trig::PI - 1e-6);
    }
}
