// ============================================================================
// bezier2d — Quadratic and Cubic Bezier Curves
// ============================================================================
//
// This crate provides the mathematical machinery for smooth 2D curves.
// Bezier curves are the universal primitive: SVG, PDF, HTML Canvas, Core
// Graphics, Direct2D, Skia, and every TrueType/OpenType font outline use
// them.
//
// ## No dependency on trig
//
// Bezier evaluation is pure polynomial arithmetic: weighted sums of control
// points, where the weights (Bernstein polynomials) involve only addition,
// subtraction, and multiplication of the parameter t ∈ [0, 1]. No
// trigonometric functions are needed. We depend on `point2d` for the Point
// type (and its `lerp` method), but NOT on the `trig` crate.
//
// We DO use trig::sqrt for bounding box computation where we need square roots.
//
// ## De Casteljau's Algorithm
//
// The most numerically stable way to evaluate a Bezier curve is de
// Casteljau's algorithm: repeatedly lerp between adjacent control points
// until a single point remains. At level 0, the control points are the
// inputs. At each subsequent level, each point is lerp(prev[i], prev[i+1], t).
// After n levels, the single remaining point is B(t).
//
// For splitting: the intermediate points from de Casteljau form the control
// points of the two sub-curves that together cover [0,t] and [t,1].

use point2d::{Point, Rect};

// We use trig::sqrt for bounding box computation (discriminant square root).

/// Version of the neutral G2D02 flattening fixture consumed by this crate.
pub const FLATTENING_FIXTURE_VERSION: u32 = 1;
/// Maximum permitted subdivision depth for one curve fragment.
pub const FLATTENING_MAX_DEPTH: usize = 32;
/// Maximum number of splits permitted for one atomic flattening call.
pub const FLATTENING_MAX_SUBDIVISIONS: usize = 65_535;

/// Deterministic failures produced by conservative polyline flattening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlatteningError {
    /// Tolerance was zero, negative, NaN, or infinite.
    InvalidTolerance,
    /// Input or derived control geometry was non-finite.
    NonFiniteGeometry,
    /// The pinned depth or subdivision ceiling was exhausted.
    BudgetExceeded,
}

impl std::fmt::Display for FlatteningError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidTolerance => "tolerance must be finite and positive",
            Self::NonFiniteGeometry => "non-finite geometry",
            Self::BudgetExceeded => "flattening budget exceeded",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for FlatteningError {}

// ============================================================================
// Quadratic Bezier
// ============================================================================

/// A quadratic Bezier curve defined by three control points.
///
/// The curve starts at `p0`, is attracted toward `p1` (the off-curve control
/// point), and ends at `p2`. The mathematical formula:
///
///   B(t) = (1-t)²·p0 + 2(1-t)t·p1 + t²·p2,   t ∈ [0,1]
///
/// TrueType font outlines use quadratic Beziers exclusively (they call them
/// "conic" segments).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadraticBezier {
    /// The start point (on the curve, t=0).
    pub p0: Point,
    /// The control point (off the curve — the curve is *attracted* toward it).
    pub p1: Point,
    /// The end point (on the curve, t=1).
    pub p2: Point,
}

impl QuadraticBezier {
    /// Create a quadratic Bezier from three control points.
    pub fn new(p0: Point, p1: Point, p2: Point) -> Self {
        Self { p0, p1, p2 }
    }

    /// Evaluate the curve at parameter t ∈ [0, 1].
    ///
    /// Uses de Casteljau's algorithm: two rounds of lerp.
    ///
    /// Round 1: q0 = lerp(p0, p1, t),  q1 = lerp(p1, p2, t)
    /// Round 2: B(t) = lerp(q0, q1, t)
    ///
    /// This is algebraically equivalent to the Bernstein formula but more
    /// numerically stable for extreme values of t.
    pub fn evaluate(&self, t: f64) -> Point {
        // First round of de Casteljau: interpolate between adjacent pairs.
        let q0 = self.p0.lerp(self.p1, t);
        let q1 = self.p1.lerp(self.p2, t);
        // Second round: final interpolation yields the curve point.
        q0.lerp(q1, t)
    }

    /// Derivative (tangent vector) at parameter t.
    ///
    /// The derivative of the quadratic Bezier is a linear Bezier:
    ///   B'(t) = 2 · ((1-t)·(p1 - p0) + t·(p2 - p1))
    ///
    /// The derivative is NOT normalized — it is the actual velocity vector.
    /// To get the unit tangent, normalize the result.
    pub fn derivative(&self, t: f64) -> Point {
        // The linear Bezier of the differences, scaled by 2.
        let d0 = self.p1.subtract(self.p0); // p1 - p0
        let d1 = self.p2.subtract(self.p1); // p2 - p1
                                            // Lerp between d0 and d1, then scale by 2.
        d0.lerp(d1, t).scale(2.0)
    }

    /// Split the curve at parameter t into two sub-curves.
    ///
    /// Returns (left, right) where:
    ///   - left covers [0, t] of the original curve
    ///   - right covers [t, 1] of the original curve
    ///
    /// The control points come directly from de Casteljau's triangle:
    ///   q0 = lerp(p0, p1, t)
    ///   q1 = lerp(p1, p2, t)
    ///   m  = lerp(q0, q1, t)   ← the split point, shared between both halves
    ///
    ///   left  = (p0, q0, m)
    ///   right = (m, q1, p2)
    pub fn split(&self, t: f64) -> (QuadraticBezier, QuadraticBezier) {
        let q0 = self.p0.lerp(self.p1, t);
        let q1 = self.p1.lerp(self.p2, t);
        let m = q0.lerp(q1, t); // the exact curve point at parameter t
        let left = QuadraticBezier::new(self.p0, q0, m);
        let right = QuadraticBezier::new(m, q1, self.p2);
        (left, right)
    }

    /// Conservatively flatten this curve or return an atomic error.
    ///
    /// Midpoint-only tests miss symmetric curves whose midpoint lies on the
    /// chord. The quadratic convex-hull property supplies a stronger bound:
    /// the curve stays within the control point's distance to the finite chord
    /// segment. An explicit stack avoids host recursion limits; right is pushed
    /// before left so output remains ordered by increasing parameter.
    pub fn to_polyline(&self, tolerance: f64) -> Result<Vec<Point>, FlatteningError> {
        validate_tolerance(tolerance)?;
        validate_points(&[self.p0, self.p1, self.p2])?;

        let mut stack = vec![(*self, 0usize)];
        let mut points = vec![self.p0];
        let mut subdivisions = 0usize;
        while let Some((curve, depth)) = stack.pop() {
            let error_bound = point_to_segment_distance(curve.p1, curve.p0, curve.p2)?;
            if error_bound <= tolerance {
                points.push(curve.p2);
                continue;
            }
            if depth >= FLATTENING_MAX_DEPTH || subdivisions >= FLATTENING_MAX_SUBDIVISIONS {
                return Err(FlatteningError::BudgetExceeded);
            }
            let (left, right) = curve.split(0.5);
            validate_points(&[left.p0, left.p1, left.p2, right.p0, right.p1, right.p2])?;
            subdivisions += 1;
            stack.push((right, depth + 1));
            stack.push((left, depth + 1));
        }
        Ok(points)
    }

    /// Compute the tight axis-aligned bounding box of this curve.
    ///
    /// The bounding box includes:
    ///   1. The two endpoints (t=0 and t=1).
    ///   2. Any interior extrema: parameter values where the derivative is zero.
    ///
    /// For the quadratic Bezier, the derivative in each coordinate is linear.
    /// Setting it to zero:
    ///   B'_x(t) = 2·((p1.x - p0.x) + t·(p0.x - 2·p1.x + p2.x)) = 0
    ///   → t = (p0.x - p1.x) / (p0.x - 2·p1.x + p2.x)
    ///
    /// We clamp t to [0, 1] and only include valid (in-range) roots.
    pub fn bounding_box(&self) -> Rect {
        // Start with the bounding box of just the two endpoints.
        let mut min_x = self.p0.x.min(self.p2.x);
        let mut max_x = self.p0.x.max(self.p2.x);
        let mut min_y = self.p0.y.min(self.p2.y);
        let mut max_y = self.p0.y.max(self.p2.y);

        // Find the X extremum: t where the X derivative is zero.
        let denom_x = self.p0.x - 2.0 * self.p1.x + self.p2.x;
        if denom_x.abs() > 1e-12 {
            let t_x = (self.p0.x - self.p1.x) / denom_x;
            if t_x > 0.0 && t_x < 1.0 {
                let px = self.evaluate(t_x);
                min_x = min_x.min(px.x);
                max_x = max_x.max(px.x);
            }
        }

        // Find the Y extremum similarly.
        let denom_y = self.p0.y - 2.0 * self.p1.y + self.p2.y;
        if denom_y.abs() > 1e-12 {
            let t_y = (self.p0.y - self.p1.y) / denom_y;
            if t_y > 0.0 && t_y < 1.0 {
                let py = self.evaluate(t_y);
                min_y = min_y.min(py.y);
                max_y = max_y.max(py.y);
            }
        }

        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Degree elevation: convert to an equivalent cubic Bezier.
    ///
    /// Any quadratic Bezier can be represented exactly as a cubic. The
    /// formula for the new control points:
    ///   q0 = p0
    ///   q1 = (1/3)·p0 + (2/3)·p1
    ///   q2 = (2/3)·p1 + (1/3)·p2
    ///   q3 = p2
    ///
    /// This is useful when a rendering engine only accepts cubics (PDF, most
    /// OpenType renderers).
    pub fn elevate(&self) -> CubicBezier {
        let q1 = self.p0.scale(1.0 / 3.0).add(self.p1.scale(2.0 / 3.0));
        let q2 = self.p1.scale(2.0 / 3.0).add(self.p2.scale(1.0 / 3.0));
        CubicBezier::new(self.p0, q1, q2, self.p2)
    }
}

// ============================================================================
// Cubic Bezier
// ============================================================================

/// A cubic Bezier curve defined by four control points.
///
/// The curve starts at `p0` (tangent toward `p1`), ends at `p3` (tangent from
/// `p2`). The mathematical formula:
///
///   B(t) = (1-t)³·p0 + 3(1-t)²t·p1 + 3(1-t)t²·p2 + t³·p3
///
/// Cubic Beziers are the primitive used in PostScript, PDF, SVG `C` command,
/// HTML Canvas bezierCurveTo, and OpenType/PostScript font outlines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubicBezier {
    /// The start point (on the curve, t=0). Tangent direction: p0→p1.
    pub p0: Point,
    /// First control point (off the curve). Controls the outgoing tangent.
    pub p1: Point,
    /// Second control point (off the curve). Controls the incoming tangent.
    pub p2: Point,
    /// The end point (on the curve, t=1). Tangent direction: p2→p3.
    pub p3: Point,
}

impl CubicBezier {
    /// Create a cubic Bezier from four control points.
    pub fn new(p0: Point, p1: Point, p2: Point, p3: Point) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// Evaluate the curve at parameter t ∈ [0, 1] using de Casteljau.
    ///
    /// Three rounds of lerp:
    ///   p01 = lerp(p0,p1,t);  p12 = lerp(p1,p2,t);  p23 = lerp(p2,p3,t)
    ///   p012 = lerp(p01,p12,t);  p123 = lerp(p12,p23,t)
    ///   B(t) = lerp(p012,p123,t)
    pub fn evaluate(&self, t: f64) -> Point {
        // Level 1: adjacent pairs.
        let p01 = self.p0.lerp(self.p1, t);
        let p12 = self.p1.lerp(self.p2, t);
        let p23 = self.p2.lerp(self.p3, t);
        // Level 2: pairs of level-1 results.
        let p012 = p01.lerp(p12, t);
        let p123 = p12.lerp(p23, t);
        // Level 3: the final curve point.
        p012.lerp(p123, t)
    }

    /// Derivative (tangent vector) at parameter t.
    ///
    /// The derivative of the cubic is a quadratic Bezier scaled by 3:
    ///   B'(t) = 3·((1-t)²·(p1-p0) + 2(1-t)t·(p2-p1) + t²·(p3-p2))
    pub fn derivative(&self, t: f64) -> Point {
        // The three difference vectors (the "hodograph" control points).
        let d0 = self.p1.subtract(self.p0); // p1 - p0
        let d1 = self.p2.subtract(self.p1); // p2 - p1
        let d2 = self.p3.subtract(self.p2); // p3 - p2
                                            // Evaluate the quadratic Bezier through d0, d1, d2 at t.
        let one_t = 1.0 - t;
        let r = d0
            .scale(one_t * one_t)
            .add(d1.scale(2.0 * one_t * t))
            .add(d2.scale(t * t));
        // Scale by 3 (degree of the cubic).
        r.scale(3.0)
    }

    /// Split the curve at parameter t into two sub-curves.
    ///
    /// De Casteljau's triangle for cubics:
    ///   Level 1: p01, p12, p23
    ///   Level 2: p012, p123
    ///   Level 3: p0123   ← the split point
    ///
    ///   left  = (p0, p01, p012, p0123)
    ///   right = (p0123, p123, p23, p3)
    pub fn split(&self, t: f64) -> (CubicBezier, CubicBezier) {
        // Level 1
        let p01 = self.p0.lerp(self.p1, t);
        let p12 = self.p1.lerp(self.p2, t);
        let p23 = self.p2.lerp(self.p3, t);
        // Level 2
        let p012 = p01.lerp(p12, t);
        let p123 = p12.lerp(p23, t);
        // Level 3 — the exact curve point at parameter t.
        let p0123 = p012.lerp(p123, t);
        let left = CubicBezier::new(self.p0, p01, p012, p0123);
        let right = CubicBezier::new(p0123, p123, p23, self.p3);
        (left, right)
    }

    /// Conservatively flatten this curve or return an atomic error.
    /// Both off-curve controls participate in the finite-segment error bound.
    pub fn to_polyline(&self, tolerance: f64) -> Result<Vec<Point>, FlatteningError> {
        validate_tolerance(tolerance)?;
        validate_points(&[self.p0, self.p1, self.p2, self.p3])?;

        let mut stack = vec![(*self, 0usize)];
        let mut points = vec![self.p0];
        let mut subdivisions = 0usize;
        while let Some((curve, depth)) = stack.pop() {
            let first_bound = point_to_segment_distance(curve.p1, curve.p0, curve.p3)?;
            let second_bound = point_to_segment_distance(curve.p2, curve.p0, curve.p3)?;
            if first_bound.max(second_bound) <= tolerance {
                points.push(curve.p3);
                continue;
            }
            if depth >= FLATTENING_MAX_DEPTH || subdivisions >= FLATTENING_MAX_SUBDIVISIONS {
                return Err(FlatteningError::BudgetExceeded);
            }
            let (left, right) = curve.split(0.5);
            validate_points(&[
                left.p0, left.p1, left.p2, left.p3, right.p0, right.p1, right.p2, right.p3,
            ])?;
            subdivisions += 1;
            stack.push((right, depth + 1));
            stack.push((left, depth + 1));
        }
        Ok(points)
    }

    /// Tight axis-aligned bounding box.
    ///
    /// The derivative of the cubic is a quadratic in t. Setting it to zero
    /// for each coordinate separately gives us up to two roots per axis.
    ///
    /// For the X component of the derivative:
    ///   a·t² + b·t + c = 0   where:
    ///     a = -3·p0.x + 9·p1.x - 9·p2.x + 3·p3.x
    ///     b =  6·p0.x - 12·p1.x + 6·p2.x
    ///     c = -3·p0.x + 3·p1.x
    ///
    /// We solve via the quadratic formula and clamp roots to [0, 1].
    pub fn bounding_box(&self) -> Rect {
        let mut min_x = self.p0.x.min(self.p3.x);
        let mut max_x = self.p0.x.max(self.p3.x);
        let mut min_y = self.p0.y.min(self.p3.y);
        let mut max_y = self.p0.y.max(self.p3.y);

        // Solve for X extrema.
        for t in extrema_of_cubic_derivative(self.p0.x, self.p1.x, self.p2.x, self.p3.x) {
            let px = self.evaluate(t);
            min_x = min_x.min(px.x);
            max_x = max_x.max(px.x);
        }

        // Solve for Y extrema.
        for t in extrema_of_cubic_derivative(self.p0.y, self.p1.y, self.p2.y, self.p3.y) {
            let py = self.evaluate(t);
            min_y = min_y.min(py.y);
            max_y = max_y.max(py.y);
        }

        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

fn validate_tolerance(tolerance: f64) -> Result<(), FlatteningError> {
    if tolerance.is_finite() && tolerance > 0.0 {
        Ok(())
    } else {
        Err(FlatteningError::InvalidTolerance)
    }
}

fn validate_points(points: &[Point]) -> Result<(), FlatteningError> {
    if points
        .iter()
        .all(|point| point.x.is_finite() && point.y.is_finite())
    {
        Ok(())
    } else {
        Err(FlatteningError::NonFiniteGeometry)
    }
}

/// Scale before squaring so tiny curved geometry cannot underflow into a false
/// zero-distance chord. Non-finite derived arithmetic fails closed.
fn point_to_segment_distance(
    point: Point,
    start: Point,
    end: Point,
) -> Result<f64, FlatteningError> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let px = point.x - start.x;
    let py = point.y - start.y;
    if ![dx, dy, px, py].iter().all(|value| value.is_finite()) {
        return Err(FlatteningError::NonFiniteGeometry);
    }

    let scale = dx.abs().max(dy.abs()).max(px.abs()).max(py.abs());
    if scale == 0.0 {
        return Ok(0.0);
    }
    let nx = dx / scale;
    let ny = dy / scale;
    let npx = px / scale;
    let npy = py / scale;
    let denominator = nx * nx + ny * ny;
    let projection = if denominator > 0.0 {
        ((npx * nx + npy * ny) / denominator).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let distance = scale * (npx - projection * nx).hypot(npy - projection * ny);
    if distance.is_finite() {
        Ok(distance)
    } else {
        Err(FlatteningError::NonFiniteGeometry)
    }
}

// ============================================================================
// Helper: solve for extrema of the cubic derivative in one coordinate
// ============================================================================

/// Find parameter values t ∈ (0, 1) where the cubic's derivative is zero
/// in the given coordinate dimension.
///
/// The cubic's derivative in one coordinate is a quadratic:
///   a·t² + b·t + c = 0
/// where:
///   a = -3·v0 + 9·v1 - 9·v2 + 3·v3
///   b =  6·v0 - 12·v1 + 6·v2
///   c = -3·v0 + 3·v1
///
/// Returns up to two t values in (0, 1) (exclusive endpoints — we already
/// evaluated at t=0 and t=1).
fn extrema_of_cubic_derivative(v0: f64, v1: f64, v2: f64, v3: f64) -> Vec<f64> {
    let a = -3.0 * v0 + 9.0 * v1 - 9.0 * v2 + 3.0 * v3;
    let b = 6.0 * v0 - 12.0 * v1 + 6.0 * v2;
    let c = -3.0 * v0 + 3.0 * v1;

    let mut roots = Vec::new();

    if a.abs() < 1e-12 {
        // Degenerate quadratic → linear equation: b·t + c = 0 → t = -c/b
        if b.abs() > 1e-12 {
            let t = -c / b;
            if t > 0.0 && t < 1.0 {
                roots.push(t);
            }
        }
        // If both a and b are zero, the derivative is constant — no extrema.
    } else {
        // Full quadratic formula: t = (-b ± sqrt(b²-4ac)) / (2a)
        let disc = b * b - 4.0 * a * c;
        if disc >= 0.0 {
            let sq = trig::sqrt(disc);
            let t1 = (-b + sq) / (2.0 * a);
            let t2 = (-b - sq) / (2.0 * a);
            if t1 > 0.0 && t1 < 1.0 {
                roots.push(t1);
            }
            if t2 > 0.0 && t2 < 1.0 {
                roots.push(t2);
            }
        }
    }

    roots
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    fn point_approx_eq(a: Point, b: Point) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y)
    }

    // -----------------------------------------------------------------------
    // QuadraticBezier
    // -----------------------------------------------------------------------

    #[test]
    fn test_quad_evaluate_endpoints() {
        let q = QuadraticBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 2.0),
            Point::new(2.0, 0.0),
        );
        assert!(point_approx_eq(q.evaluate(0.0), q.p0));
        assert!(point_approx_eq(q.evaluate(1.0), q.p2));
    }

    #[test]
    fn test_quad_evaluate_midpoint() {
        // A parabola through (0,0), control (1,2), (2,0).
        // B(0.5) = (1-0.5)²*(0,0) + 2*0.5*0.5*(1,2) + 0.5²*(2,0)
        //        = 0.25*(0,0) + 0.5*(1,2) + 0.25*(2,0)
        //        = (0+0.5+0.5, 0+1+0) = (1.0, 1.0)
        let q = QuadraticBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 2.0),
            Point::new(2.0, 0.0),
        );
        let mid = q.evaluate(0.5);
        assert!(approx_eq(mid.x, 1.0));
        assert!(approx_eq(mid.y, 1.0));
    }

    #[test]
    fn test_quad_derivative_at_endpoints() {
        let q = QuadraticBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(2.0, 0.0),
        );
        // B'(0) = 2*(p1 - p0) = 2*(1,1) = (2, 2)
        let d0 = q.derivative(0.0);
        assert!(approx_eq(d0.x, 2.0));
        assert!(approx_eq(d0.y, 2.0));
        // B'(1) = 2*(p2 - p1) = 2*(1,-1) = (2, -2)
        let d1 = q.derivative(1.0);
        assert!(approx_eq(d1.x, 2.0));
        assert!(approx_eq(d1.y, -2.0));
    }

    #[test]
    fn test_quad_split_recombines() {
        let q = QuadraticBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 2.0),
            Point::new(4.0, 0.0),
        );
        let (left, right) = q.split(0.5);
        // The split point should equal the curve at t=0.5.
        let split_pt = q.evaluate(0.5);
        assert!(point_approx_eq(left.p2, split_pt));
        assert!(point_approx_eq(right.p0, split_pt));
        // Endpoints should be preserved.
        assert!(point_approx_eq(left.p0, q.p0));
        assert!(point_approx_eq(right.p2, q.p2));
    }

    #[test]
    fn test_quad_split_evaluate_consistent() {
        // Points on the left sub-curve should match the original at half the parameter.
        let q = QuadraticBezier::new(
            Point::new(0.0, 0.0),
            Point::new(2.0, 4.0),
            Point::new(4.0, 0.0),
        );
        let (left, _) = q.split(0.5);
        // Evaluating left at t=1.0 should give q.evaluate(0.5).
        assert!(point_approx_eq(left.evaluate(1.0), q.evaluate(0.5)));
        // Evaluating left at t=0.5 should give q.evaluate(0.25).
        let left_mid = left.evaluate(0.5);
        let orig_quarter = q.evaluate(0.25);
        assert!((left_mid.x - orig_quarter.x).abs() < 1e-6);
    }

    #[test]
    fn test_quad_to_polyline_straight_line() {
        // A straight-line quadratic (p1 on the segment) should produce just two points.
        let q = QuadraticBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0), // control point ON the line
            Point::new(2.0, 0.0),
        );
        let pts = q
            .to_polyline(0.1)
            .expect("straight quadratic should flatten");
        // Should be two points since the error is zero.
        assert_eq!(pts.len(), 2);
        assert!(point_approx_eq(pts[0], q.p0));
        assert!(point_approx_eq(pts[pts.len() - 1], q.p2));
    }

    #[test]
    fn test_quad_to_polyline_has_endpoints() {
        let q = QuadraticBezier::new(
            Point::new(0.0, 0.0),
            Point::new(0.0, 10.0),
            Point::new(10.0, 0.0),
        );
        let pts = q.to_polyline(0.1).expect("curved quadratic should flatten");
        assert!(!pts.is_empty());
        assert!(point_approx_eq(pts[0], q.p0));
        assert!(point_approx_eq(pts[pts.len() - 1], q.p2));
    }

    #[test]
    fn test_quad_bounding_box_contains_endpoints() {
        let q = QuadraticBezier::new(
            Point::new(0.0, 0.0),
            Point::new(5.0, 10.0),
            Point::new(10.0, 0.0),
        );
        let bb = q.bounding_box();
        assert!(bb.x <= 0.0);
        assert!(bb.y <= 0.0);
        assert!(bb.x + bb.width >= 10.0);
    }

    #[test]
    fn test_quad_bounding_box_axis_aligned() {
        // A curve that reaches a maximum at t=0.5 in Y.
        let q = QuadraticBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 4.0), // peak control point
            Point::new(2.0, 0.0),
        );
        let bb = q.bounding_box();
        // Max Y should be > 0, and > the max of p0.y and p2.y (which are both 0).
        assert!(bb.height > 0.0);
    }

    #[test]
    fn test_quad_elevate() {
        // Elevating a straight-line quadratic should give a straight-line cubic.
        let q = QuadraticBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(2.0, 0.0),
        );
        let c = q.elevate();
        // The elevated cubic should produce the same point at several t values.
        for &t in &[0.0, 0.25, 0.5, 0.75, 1.0] {
            let qp = q.evaluate(t);
            let cp = c.evaluate(t);
            assert!((qp.x - cp.x).abs() < 1e-9, "t={}: x mismatch", t);
            assert!((qp.y - cp.y).abs() < 1e-9, "t={}: y mismatch", t);
        }
    }

    // -----------------------------------------------------------------------
    // CubicBezier
    // -----------------------------------------------------------------------

    #[test]
    fn test_cubic_evaluate_endpoints() {
        let c = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 3.0),
            Point::new(3.0, 3.0),
            Point::new(4.0, 0.0),
        );
        assert!(point_approx_eq(c.evaluate(0.0), c.p0));
        assert!(point_approx_eq(c.evaluate(1.0), c.p3));
    }

    #[test]
    fn test_cubic_evaluate_midpoint_symmetric() {
        // Symmetric cubic: the midpoint should be on the axis of symmetry.
        let c = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 2.0),
            Point::new(3.0, 2.0),
            Point::new(4.0, 0.0),
        );
        let mid = c.evaluate(0.5);
        // By symmetry, x should be 2.0.
        assert!((mid.x - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_cubic_derivative_endpoints() {
        let c = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(3.0, 0.0),
        );
        // Straight line: derivative should be (3, 0) everywhere.
        let d0 = c.derivative(0.0);
        assert!((d0.x - 3.0).abs() < 1e-9);
        assert!(d0.y.abs() < 1e-9);
    }

    #[test]
    fn test_cubic_split_endpoints() {
        let c = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 2.0),
            Point::new(3.0, 2.0),
            Point::new(4.0, 0.0),
        );
        let (left, right) = c.split(0.5);
        let split_pt = c.evaluate(0.5);
        assert!(point_approx_eq(left.p3, split_pt));
        assert!(point_approx_eq(right.p0, split_pt));
        assert!(point_approx_eq(left.p0, c.p0));
        assert!(point_approx_eq(right.p3, c.p3));
    }

    #[test]
    fn test_cubic_to_polyline_straight_line() {
        let c = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(3.0, 0.0),
        );
        let pts = c.to_polyline(0.1).expect("straight cubic should flatten");
        assert_eq!(pts.len(), 2); // straight line → just two points
    }

    #[test]
    fn test_cubic_to_polyline_curved() {
        let c = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(0.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 0.0),
        );
        let pts = c.to_polyline(0.1).expect("curved cubic should flatten");
        // Should have more than 2 points for a curved cubic.
        assert!(pts.len() > 2);
        // Endpoints must be preserved.
        assert!(point_approx_eq(pts[0], c.p0));
        assert!(point_approx_eq(pts[pts.len() - 1], c.p3));
    }

    #[test]
    fn test_cubic_bounding_box_contains_all_evaluated_points() {
        let c = CubicBezier::new(
            Point::new(0.0, 0.0),
            Point::new(0.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 0.0),
        );
        let bb = c.bounding_box();
        // Sample many points and ensure all are within the bounding box.
        for i in 0..=20 {
            let t = i as f64 / 20.0;
            let p = c.evaluate(t);
            assert!(
                p.x >= bb.x - 1e-6 && p.x <= bb.x + bb.width + 1e-6,
                "x out of bounds at t={}",
                t
            );
            assert!(
                p.y >= bb.y - 1e-6 && p.y <= bb.y + bb.height + 1e-6,
                "y out of bounds at t={}",
                t
            );
        }
    }

    #[test]
    fn test_cubic_bounding_box_straight_line() {
        let c = CubicBezier::new(
            Point::new(1.0, 2.0),
            Point::new(2.0, 2.0),
            Point::new(3.0, 2.0),
            Point::new(4.0, 2.0),
        );
        let bb = c.bounding_box();
        assert!((bb.x - 1.0).abs() < 1e-9);
        assert!((bb.y - 2.0).abs() < 1e-9);
        assert!((bb.width - 3.0).abs() < 1e-9);
        assert!(bb.height.abs() < 1e-9);
    }

    #[derive(serde::Deserialize)]
    struct FlatteningFixture {
        version: u32,
        max_depth: usize,
        max_subdivisions: usize,
        cases: Vec<FlatteningCase>,
    }

    #[derive(serde::Deserialize)]
    struct FlatteningCase {
        id: String,
        degree: u32,
        control_points: Vec<[f64; 2]>,
        tolerance: f64,
        expected_disposition: String,
        #[serde(default)]
        expected_termination: String,
    }

    fn flattening_fixture() -> FlatteningFixture {
        serde_json::from_str(include_str!(
            "../../../../specs/fixtures/bezier2d-flattening-v1/cases.json"
        ))
        .expect("the committed flattening fixture should decode")
    }

    fn fixture_point(coordinates: [f64; 2]) -> Point {
        Point::new(coordinates[0], coordinates[1])
    }

    #[test]
    fn flattening_fixture_conformance() {
        let fixture = flattening_fixture();
        assert_eq!(fixture.version, FLATTENING_FIXTURE_VERSION);
        assert_eq!(fixture.max_depth, FLATTENING_MAX_DEPTH);
        assert_eq!(fixture.max_subdivisions, FLATTENING_MAX_SUBDIVISIONS);
        assert_eq!(fixture.cases.len(), 9);

        for case in fixture.cases {
            let result = match case.degree {
                2 => QuadraticBezier::new(
                    fixture_point(case.control_points[0]),
                    fixture_point(case.control_points[1]),
                    fixture_point(case.control_points[2]),
                )
                .to_polyline(case.tolerance),
                3 => CubicBezier::new(
                    fixture_point(case.control_points[0]),
                    fixture_point(case.control_points[1]),
                    fixture_point(case.control_points[2]),
                    fixture_point(case.control_points[3]),
                )
                .to_polyline(case.tolerance),
                degree => panic!("unsupported fixture degree {degree}"),
            };

            if case.expected_disposition == "invalid-tolerance" {
                assert_eq!(
                    result,
                    Err(FlatteningError::InvalidTolerance),
                    "{}",
                    case.id
                );
                continue;
            }
            if case.expected_termination == "budget-error" {
                assert_eq!(result, Err(FlatteningError::BudgetExceeded), "{}", case.id);
                continue;
            }

            let points = result.unwrap_or_else(|error| panic!("{}: {error:?}", case.id));
            assert_eq!(points.first(), Some(&fixture_point(case.control_points[0])));
            assert_eq!(
                points.last(),
                Some(&fixture_point(
                    *case.control_points.last().expect("endpoint")
                ))
            );
            assert!(points
                .iter()
                .all(|point| point.x.is_finite() && point.y.is_finite()));
            if case.expected_disposition == "chord" {
                assert_eq!(points.len(), 2, "{}", case.id);
            } else if case.expected_disposition == "subdivide" {
                assert!(points.len() > 2, "{}", case.id);
            }
        }
    }

    #[test]
    fn flattening_rejects_nonfinite_inputs_atomically() {
        let straight = QuadraticBezier::new(
            Point::new(0.0, 0.0),
            Point::new(0.5, 0.0),
            Point::new(1.0, 0.0),
        );
        for tolerance in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                straight.to_polyline(tolerance),
                Err(FlatteningError::InvalidTolerance)
            );
        }

        let bad_control = QuadraticBezier::new(
            Point::new(0.0, 0.0),
            Point::new(f64::INFINITY, 0.0),
            Point::new(1.0, 0.0),
        );
        assert_eq!(
            bad_control.to_polyline(0.1),
            Err(FlatteningError::NonFiniteGeometry)
        );
    }

    #[test]
    fn flattening_fails_closed_on_derived_overflow_and_subnormal_geometry() {
        let max = f64::MAX;
        let overflow = CubicBezier::new(
            Point::new(-max, 0.0),
            Point::new(0.0, 1.0),
            Point::new(0.0, -1.0),
            Point::new(max, 0.0),
        );
        assert_eq!(
            overflow.to_polyline(0.1),
            Err(FlatteningError::NonFiniteGeometry)
        );

        let tiny = 1e-200;
        let subnormal = QuadraticBezier::new(
            Point::new(0.0, 0.0),
            Point::new(0.0, tiny),
            Point::new(tiny, 0.0),
        );
        assert_eq!(
            subnormal.to_polyline(f64::from_bits(1)),
            Err(FlatteningError::BudgetExceeded)
        );
    }

    #[test]
    fn flattening_error_messages_and_scaled_distance_boundaries() {
        assert_eq!(
            FlatteningError::InvalidTolerance.to_string(),
            "tolerance must be finite and positive"
        );
        assert_eq!(
            FlatteningError::NonFiniteGeometry.to_string(),
            "non-finite geometry"
        );
        assert_eq!(
            FlatteningError::BudgetExceeded.to_string(),
            "flattening budget exceeded"
        );

        let origin = Point::new(0.0, 0.0);
        assert_eq!(point_to_segment_distance(origin, origin, origin), Ok(0.0));
        assert_eq!(
            point_to_segment_distance(Point::new(f64::MAX, f64::MAX), origin, origin),
            Err(FlatteningError::NonFiniteGeometry)
        );
    }
}
