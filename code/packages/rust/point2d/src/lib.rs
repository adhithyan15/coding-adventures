// ============================================================================
// point2d — 2D Point/Vector and Axis-Aligned Bounding Box
// ============================================================================
//
// This crate defines two fundamental data types for 2D geometry:
//
//   - `Point` — a 2D position *and* a 2D vector. In the Cartesian plane, a
//     position (where is something?) and a direction+magnitude (how far and
//     which way?) are both described by exactly two real numbers (x, y).
//     Treating them as the same type eliminates entire classes of
//     representation-mismatch bugs. Context determines whether you interpret
//     the pair as a position or a vector.
//
//   - `Rect` — an axis-aligned bounding box (AABB), given by an origin
//     corner (x, y) and a size (width, height). Bounding boxes appear
//     everywhere: hit-testing, dirty-region tracking, clipping, conservative
//     intersection tests.
//
// ## Dependency
//
// The `angle()` method calls `trig::atan2` from the PHY00 `trig` crate. All
// other operations are pure arithmetic requiring no external dependencies.
//
// ## Design Philosophy
//
// All operations are **immutable**: they return new values and leave the
// originals unchanged. This makes concurrent use safe and eliminates aliasing
// bugs. Both `Point` and `Rect` derive `Copy`, so they are stack-allocated
// and cheap to copy — just 16 bytes and 32 bytes respectively.

use trig;

// ============================================================================
// Point
// ============================================================================

/// A 2D point (position) and 2D vector (direction + magnitude).
///
/// The two interpretations share the same underlying representation:
/// a pair of 64-bit floats `(x, y)`. Which interpretation applies depends
/// on context: subtracting two positions gives a displacement vector;
/// adding a displacement vector to a position gives a new position.
///
/// All methods are immutable — they return new `Point` values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// The horizontal coordinate (positive = right).
    pub x: f64,
    /// The vertical coordinate (positive = down in screen space, up in math).
    pub y: f64,
}

impl Point {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    /// Create a point at coordinates (x, y).
    ///
    /// This is the primary constructor. Use `origin()` for the zero point.
    ///
    /// ```
    /// let p = point2d::Point::new(3.0, 4.0);
    /// assert_eq!(p.x, 3.0);
    /// assert_eq!(p.y, 4.0);
    /// ```
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// The additive identity: the point at (0, 0).
    ///
    /// This is the origin of the coordinate system — the reference point
    /// from which all other positions are measured. Adding the origin to
    /// any point leaves it unchanged.
    pub fn origin() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    // -----------------------------------------------------------------------
    // Arithmetic
    // -----------------------------------------------------------------------

    /// Add another point (or vector) component-wise.
    ///
    /// Geometrically: start at `self`, walk by the displacement `other`.
    /// Result = (x1 + x2, y1 + y2).
    pub fn add(&self, other: Point) -> Point {
        // Vector addition: each component adds independently.
        Point::new(self.x + other.x, self.y + other.y)
    }

    /// Subtract another point (or vector) component-wise.
    ///
    /// The vector from point B to point A is `A.subtract(B)`.
    /// Result = (x1 - x2, y1 - y2).
    pub fn subtract(&self, other: Point) -> Point {
        // Vector subtraction: displacement from other to self.
        Point::new(self.x - other.x, self.y - other.y)
    }

    /// Scale this point/vector by a scalar.
    ///
    /// If s > 1, the vector grows longer. If 0 < s < 1, it shrinks.
    /// If s < 0, it reverses direction. If s = 0, it becomes the origin.
    /// Result = (s*x, s*y).
    pub fn scale(&self, s: f64) -> Point {
        // Scalar multiplication: stretch or shrink each component.
        Point::new(self.x * s, self.y * s)
    }

    /// Negate this point/vector: reverse both components.
    ///
    /// Geometrically: the vector pointing in the opposite direction with the
    /// same magnitude. Equivalent to `scale(-1.0)`.
    /// Result = (-x, -y).
    pub fn negate(&self) -> Point {
        // Additive inverse: flip sign of each component.
        Point::new(-self.x, -self.y)
    }

    // -----------------------------------------------------------------------
    // Vector Operations
    // -----------------------------------------------------------------------

    /// Dot product: x1*x2 + y1*y2.
    ///
    /// The dot product encodes the angle θ between two vectors:
    ///   u · v = |u| |v| cos(θ)
    ///
    /// - Positive: vectors point roughly the same direction (θ < 90°)
    /// - Zero: vectors are perpendicular (θ = 90°)
    /// - Negative: vectors point roughly opposite directions (θ > 90°)
    ///
    /// Use cases: projections, checking perpendicularity, computing angles.
    pub fn dot(&self, other: Point) -> f64 {
        // Component-wise multiply and sum.
        self.x * other.x + self.y * other.y
    }

    /// 2D cross product (scalar): x1*y2 - y1*x2.
    ///
    /// In 3D, the cross product of two XY-plane vectors produces a Z-axis
    /// vector. In 2D we only care about its magnitude (the Z component).
    ///
    /// - Positive: `other` is to the LEFT of `self` (counterclockwise turn)
    /// - Negative: `other` is to the RIGHT of `self` (clockwise turn)
    /// - Zero: vectors are parallel (collinear or anti-parallel)
    ///
    /// Use cases: winding number, orientation tests, convex hull.
    pub fn cross(&self, other: Point) -> f64 {
        // The determinant of the 2×2 matrix [self | other].
        self.x * other.y - self.y * other.x
    }

    /// Euclidean magnitude (length) of this vector: sqrt(x² + y²).
    ///
    /// Implements the Pythagorean theorem. Uses `trig::sqrt` from PHY00.
    ///
    /// **Performance tip**: if you only need to *compare* magnitudes,
    /// use `magnitude_squared()` — it avoids the square root entirely.
    pub fn magnitude(&self) -> f64 {
        // Pythagorean theorem: distance from origin.
        trig::sqrt(self.x * self.x + self.y * self.y)
    }

    /// Squared magnitude: x² + y². No square root.
    ///
    /// Cheaper than `magnitude()`. Use this when comparing distances
    /// (e.g., is point A closer than point B?) — the ordering is preserved
    /// after squaring.
    pub fn magnitude_squared(&self) -> f64 {
        // Avoid the sqrt: (x² + y²) preserves relative ordering.
        self.x * self.x + self.y * self.y
    }

    /// Normalize to a unit vector (magnitude = 1).
    ///
    /// Divides each component by the magnitude. The resulting vector has the
    /// same direction as the original, but length 1. Unit vectors represent
    /// *pure direction* with no magnitude information.
    ///
    /// **Edge case**: if the magnitude is zero (or very close to zero), we
    /// return the origin rather than dividing by zero.
    pub fn normalize(&self) -> Point {
        let m = self.magnitude();
        // Guard against division by zero for the zero vector.
        if m < 1e-12 {
            // The zero vector has no direction; return the origin by convention.
            return Point::origin();
        }
        // Scale each component by 1/m to produce a unit vector.
        Point::new(self.x / m, self.y / m)
    }

    /// Euclidean distance to another point.
    ///
    /// This is the length of the displacement vector from `other` to `self`.
    /// Equivalent to `self.subtract(other).magnitude()`.
    pub fn distance(&self, other: Point) -> f64 {
        // Compute displacement, then take its length.
        self.subtract(other).magnitude()
    }

    /// Squared Euclidean distance to another point. No square root.
    ///
    /// Prefer this over `distance()` when comparing distances.
    pub fn distance_squared(&self, other: Point) -> f64 {
        self.subtract(other).magnitude_squared()
    }

    // -----------------------------------------------------------------------
    // Interpolation and Direction
    // -----------------------------------------------------------------------

    /// Linear interpolation between `self` and `other`.
    ///
    /// Formula: self + t * (other - self)
    ///        = (1 - t) * self + t * other
    ///
    /// - t = 0.0 → returns self
    /// - t = 1.0 → returns other
    /// - t = 0.5 → returns the midpoint
    /// - t outside [0, 1] → extrapolates beyond the segment
    ///
    /// Lerp is the building block of de Casteljau's algorithm for Bezier
    /// curves (G2D02), easing functions, and gradient interpolation.
    pub fn lerp(&self, other: Point, t: f64) -> Point {
        // Equivalent to self + t*(other-self), which avoids catastrophic
        // cancellation better than (1-t)*self + t*other for extreme t.
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        Point::new(self.x + t * dx, self.y + t * dy)
    }

    /// Rotate 90° counterclockwise: returns (-y, x).
    ///
    /// Proof: if the original vector has angle θ (x = r·cos θ, y = r·sin θ),
    /// then (-y, x) = (-r·sin θ, r·cos θ). Using the identities
    ///   cos(θ + 90°) = -sin θ   and   sin(θ + 90°) = cos θ
    /// this is the vector at angle θ + 90°.
    ///
    /// Result has the same magnitude as self. Calling twice gives negate().
    /// Use cases: curve normals, stroke boundaries, right-hand directions.
    pub fn perpendicular(&self) -> Point {
        // A 90° CCW rotation in 2D: (x, y) → (-y, x).
        Point::new(-self.y, self.x)
    }

    /// Direction angle in radians: atan2(y, x).
    ///
    /// The angle is measured counterclockwise from the positive X axis.
    /// Result is in the range (-π, π].
    ///
    /// - (1, 0).angle() =  0.0   (points right)
    /// - (0, 1).angle() =  π/2   (points up in math / down in screen)
    /// - (-1, 0).angle() = π or -π (points left)
    /// - (0, -1).angle() = -π/2  (points down in math)
    ///
    /// Always calls `trig::atan2` from PHY00 — never the standard library.
    pub fn angle(&self) -> f64 {
        // atan2 handles all four quadrants correctly, unlike atan(y/x)
        // which is ambiguous in the third and fourth quadrants.
        trig::atan2(self.y, self.x)
    }
}

// ============================================================================
// Rect
// ============================================================================

/// An axis-aligned bounding box (AABB).
///
/// Defined by an origin corner `(x, y)` and dimensions `(width, height)`.
/// All fields are 64-bit floats. The coordinate convention is:
///   - (x, y) is the **top-left** corner in screen space (Y increases downward)
///   - `width` and `height` are non-negative for a valid rect
///
/// This representation is used by SVG, HTML Canvas, Core Graphics, Direct2D,
/// and every major 2D drawing API.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// X coordinate of the top-left corner.
    pub x: f64,
    /// Y coordinate of the top-left corner.
    pub y: f64,
    /// Width of the rectangle (extent in the X direction).
    pub width: f64,
    /// Height of the rectangle (extent in the Y direction).
    pub height: f64,
}

impl Rect {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    /// Create a rect with the given origin and dimensions.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }

    /// Construct a rect from two corner points.
    ///
    /// `min` is the top-left corner (smaller x, smaller y in screen space).
    /// `max` is the bottom-right corner.
    /// Width = max.x - min.x, Height = max.y - min.y.
    pub fn from_points(min: Point, max: Point) -> Self {
        Self {
            x: min.x,
            y: min.y,
            width: max.x - min.x,
            height: max.y - min.y,
        }
    }

    /// The empty rect at the origin: {0, 0, 0, 0}.
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0, width: 0.0, height: 0.0 }
    }

    // -----------------------------------------------------------------------
    // Corner Accessors
    // -----------------------------------------------------------------------

    /// The top-left corner: Point(x, y).
    pub fn min_point(&self) -> Point {
        Point::new(self.x, self.y)
    }

    /// The bottom-right corner: Point(x + width, y + height).
    pub fn max_point(&self) -> Point {
        Point::new(self.x + self.width, self.y + self.height)
    }

    /// The center point: Point(x + width/2, y + height/2).
    pub fn center(&self) -> Point {
        Point::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    // -----------------------------------------------------------------------
    // Geometric Predicates
    // -----------------------------------------------------------------------

    /// True if width ≤ 0 or height ≤ 0 (zero-area rect).
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// True if point p is inside this rect (half-open interval).
    ///
    /// The interval is [x, x+width) × [y, y+height): the top-left edge is
    /// **inclusive**, the bottom-right edge is **exclusive**. This avoids
    /// double-counting pixels when adjacent rects tile a surface — the
    /// same convention used by Java AWT, Direct2D, and HTML Canvas.
    pub fn contains_point(&self, p: Point) -> bool {
        // Half-open: left and top edges are inside, right and bottom are not.
        p.x >= self.x
            && p.x < self.x + self.width
            && p.y >= self.y
            && p.y < self.y + self.height
    }

    // -----------------------------------------------------------------------
    // Set Operations
    // -----------------------------------------------------------------------

    /// Smallest rect containing both `self` and `other`.
    ///
    /// If either rect is empty, return the other.
    ///
    /// Algorithm: take the min of top-left corners and max of bottom-right
    /// corners. The resulting rect encompasses both.
    pub fn union(&self, other: Rect) -> Rect {
        // Handle empty rects: union with empty returns the other.
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return *self;
        }
        // Expand to cover both rects.
        let min_x = self.x.min(other.x);
        let min_y = self.y.min(other.y);
        let max_x = (self.x + self.width).max(other.x + other.width);
        let max_y = (self.y + self.height).max(other.y + other.height);
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// The overlapping region of `self` and `other`, or `None` if they don't overlap.
    ///
    /// Algorithm:
    ///   - The left edge of the intersection is the rightmost left edge.
    ///   - The top edge of the intersection is the bottommost top edge.
    ///   - Similarly for right and bottom.
    ///   - If the resulting width or height is ≤ 0, there's no overlap.
    pub fn intersection(&self, other: Rect) -> Option<Rect> {
        // The intersection's left edge is the further-right of the two left edges.
        let ix = self.x.max(other.x);
        let iy = self.y.max(other.y);
        // The intersection's right edge is the further-left of the two right edges.
        let iw = (self.x + self.width).min(other.x + other.width) - ix;
        let ih = (self.y + self.height).min(other.y + other.height) - iy;
        // If width or height is non-positive, the rects do not overlap.
        if iw <= 0.0 || ih <= 0.0 {
            None
        } else {
            Some(Rect::new(ix, iy, iw, ih))
        }
    }

    /// Grow all four edges outward by `amount`.
    ///
    /// The origin shifts by (-amount, -amount) and the dimensions grow by
    /// 2*amount in each direction. Negative `amount` shrinks the rect.
    ///
    /// Use cases: stroke bounding boxes (a stroke of width w expands the fill
    /// bbox by w/2 on each side), padding, conservative intersection tests.
    pub fn expand_by(&self, amount: f64) -> Rect {
        // Shift origin by -amount and grow each dimension by 2*amount.
        Rect::new(
            self.x - amount,
            self.y - amount,
            self.width + 2.0 * amount,
            self.height + 2.0 * amount,
        )
    }
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
    // Point construction
    // -----------------------------------------------------------------------

    #[test]
    fn test_origin() {
        let o = Point::origin();
        assert_eq!(o.x, 0.0);
        assert_eq!(o.y, 0.0);
    }

    #[test]
    fn test_new() {
        let p = Point::new(3.0, -5.0);
        assert_eq!(p.x, 3.0);
        assert_eq!(p.y, -5.0);
    }

    // -----------------------------------------------------------------------
    // Arithmetic
    // -----------------------------------------------------------------------

    #[test]
    fn test_add() {
        let a = Point::new(1.0, 2.0);
        let b = Point::new(3.0, 4.0);
        let c = a.add(b);
        assert_eq!(c, Point::new(4.0, 6.0));
    }

    #[test]
    fn test_subtract() {
        let a = Point::new(5.0, 7.0);
        let b = Point::new(2.0, 3.0);
        assert_eq!(a.subtract(b), Point::new(3.0, 4.0));
    }

    #[test]
    fn test_scale() {
        let p = Point::new(3.0, 4.0);
        assert_eq!(p.scale(2.0), Point::new(6.0, 8.0));
        assert_eq!(p.scale(0.0), Point::origin());
        assert_eq!(p.scale(-1.0), Point::new(-3.0, -4.0));
    }

    #[test]
    fn test_negate() {
        let p = Point::new(3.0, -4.0);
        assert_eq!(p.negate(), Point::new(-3.0, 4.0));
    }

    // -----------------------------------------------------------------------
    // Vector operations
    // -----------------------------------------------------------------------

    #[test]
    fn test_dot_perpendicular() {
        // Perpendicular vectors have zero dot product.
        let x = Point::new(1.0, 0.0);
        let y = Point::new(0.0, 1.0);
        assert_eq!(x.dot(y), 0.0);
    }

    #[test]
    fn test_dot_parallel() {
        // Parallel vectors: dot = product of magnitudes.
        let p = Point::new(3.0, 0.0);
        let q = Point::new(5.0, 0.0);
        assert_eq!(p.dot(q), 15.0);
    }

    #[test]
    fn test_cross_ccw() {
        // X-axis cross Y-axis = +1 (CCW turn).
        let x = Point::new(1.0, 0.0);
        let y = Point::new(0.0, 1.0);
        assert_eq!(x.cross(y), 1.0);
    }

    #[test]
    fn test_cross_cw() {
        // Y-axis cross X-axis = -1 (CW turn).
        let x = Point::new(1.0, 0.0);
        let y = Point::new(0.0, 1.0);
        assert_eq!(y.cross(x), -1.0);
    }

    #[test]
    fn test_cross_parallel() {
        // Parallel vectors have zero cross product.
        let a = Point::new(2.0, 0.0);
        let b = Point::new(5.0, 0.0);
        assert_eq!(a.cross(b), 0.0);
    }

    #[test]
    fn test_magnitude_3_4_5() {
        // Classic Pythagorean triple: sqrt(3² + 4²) = 5.
        let p = Point::new(3.0, 4.0);
        assert!(approx_eq(p.magnitude(), 5.0));
    }

    #[test]
    fn test_magnitude_zero() {
        assert_eq!(Point::origin().magnitude(), 0.0);
    }

    #[test]
    fn test_magnitude_squared() {
        let p = Point::new(3.0, 4.0);
        assert_eq!(p.magnitude_squared(), 25.0); // exact, no sqrt
    }

    #[test]
    fn test_normalize_unit() {
        let p = Point::new(3.0, 4.0);
        let n = p.normalize();
        assert!(approx_eq(n.x, 0.6));
        assert!(approx_eq(n.y, 0.8));
        assert!(approx_eq(n.magnitude(), 1.0));
    }

    #[test]
    fn test_normalize_zero_vector() {
        // The zero vector has no direction; return origin by convention.
        let n = Point::origin().normalize();
        assert_eq!(n, Point::origin());
    }

    #[test]
    fn test_normalize_already_unit() {
        let p = Point::new(1.0, 0.0);
        let n = p.normalize();
        assert!(approx_eq(n.x, 1.0));
        assert!(approx_eq(n.y, 0.0));
    }

    #[test]
    fn test_distance() {
        let a = Point::origin();
        let b = Point::new(3.0, 4.0);
        assert!(approx_eq(a.distance(b), 5.0));
    }

    #[test]
    fn test_distance_squared() {
        let a = Point::origin();
        let b = Point::new(3.0, 4.0);
        assert_eq!(a.distance_squared(b), 25.0);
    }

    #[test]
    fn test_lerp_endpoints() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(10.0, 10.0);
        assert!(point_approx_eq(a.lerp(b, 0.0), a));
        assert!(point_approx_eq(a.lerp(b, 1.0), b));
    }

    #[test]
    fn test_lerp_midpoint() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(10.0, 10.0);
        let mid = a.lerp(b, 0.5);
        assert!(point_approx_eq(mid, Point::new(5.0, 5.0)));
    }

    #[test]
    fn test_perpendicular() {
        // (1, 0) rotated 90° CCW = (0, 1)
        let p = Point::new(1.0, 0.0);
        assert_eq!(p.perpendicular(), Point::new(0.0, 1.0));
        // (0, 1) rotated 90° CCW = (-1, 0)
        let q = Point::new(0.0, 1.0);
        assert_eq!(q.perpendicular(), Point::new(-1.0, 0.0));
    }

    #[test]
    fn test_perpendicular_twice_is_negate() {
        let p = Point::new(3.0, 4.0);
        let pp = p.perpendicular().perpendicular();
        assert!(point_approx_eq(pp, p.negate()));
    }

    #[test]
    fn test_angle_right() {
        let p = Point::new(1.0, 0.0);
        assert!(approx_eq(p.angle(), 0.0));
    }

    #[test]
    fn test_angle_up() {
        let p = Point::new(0.0, 1.0);
        assert!(approx_eq(p.angle(), trig::PI / 2.0));
    }

    #[test]
    fn test_angle_left() {
        let p = Point::new(-1.0, 0.0);
        // atan2(0, -1) = π
        assert!(approx_eq(p.angle().abs(), trig::PI));
    }

    #[test]
    fn test_angle_down() {
        let p = Point::new(0.0, -1.0);
        assert!(approx_eq(p.angle(), -trig::PI / 2.0));
    }

    // -----------------------------------------------------------------------
    // Rect construction
    // -----------------------------------------------------------------------

    #[test]
    fn test_rect_new() {
        let r = Rect::new(1.0, 2.0, 10.0, 5.0);
        assert_eq!(r.x, 1.0);
        assert_eq!(r.y, 2.0);
        assert_eq!(r.width, 10.0);
        assert_eq!(r.height, 5.0);
    }

    #[test]
    fn test_rect_from_points() {
        let min = Point::new(1.0, 2.0);
        let max = Point::new(11.0, 7.0);
        let r = Rect::from_points(min, max);
        assert_eq!(r.x, 1.0);
        assert_eq!(r.y, 2.0);
        assert_eq!(r.width, 10.0);
        assert_eq!(r.height, 5.0);
    }

    #[test]
    fn test_rect_zero() {
        let r = Rect::zero();
        assert_eq!(r.x, 0.0);
        assert_eq!(r.width, 0.0);
    }

    // -----------------------------------------------------------------------
    // Rect accessors
    // -----------------------------------------------------------------------

    #[test]
    fn test_min_max_center() {
        let r = Rect::new(2.0, 3.0, 8.0, 4.0);
        assert!(point_approx_eq(r.min_point(), Point::new(2.0, 3.0)));
        assert!(point_approx_eq(r.max_point(), Point::new(10.0, 7.0)));
        assert!(point_approx_eq(r.center(), Point::new(6.0, 5.0)));
    }

    // -----------------------------------------------------------------------
    // Rect predicates
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_empty() {
        assert!(Rect::zero().is_empty());
        assert!(Rect::new(0.0, 0.0, 0.0, 5.0).is_empty());
        assert!(Rect::new(0.0, 0.0, 5.0, -1.0).is_empty());
        assert!(!Rect::new(0.0, 0.0, 5.0, 5.0).is_empty());
    }

    #[test]
    fn test_contains_point_inside() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(r.contains_point(Point::new(5.0, 5.0)));
        assert!(r.contains_point(Point::new(0.0, 0.0))); // top-left inclusive
    }

    #[test]
    fn test_contains_point_boundary_exclusive() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        // Right and bottom edges are exclusive.
        assert!(!r.contains_point(Point::new(10.0, 5.0)));
        assert!(!r.contains_point(Point::new(5.0, 10.0)));
        assert!(!r.contains_point(Point::new(10.0, 10.0)));
    }

    #[test]
    fn test_contains_point_outside() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(!r.contains_point(Point::new(-1.0, 5.0)));
        assert!(!r.contains_point(Point::new(5.0, -1.0)));
        assert!(!r.contains_point(Point::new(15.0, 5.0)));
    }

    // -----------------------------------------------------------------------
    // Rect set operations
    // -----------------------------------------------------------------------

    #[test]
    fn test_union_non_overlapping() {
        let a = Rect::new(0.0, 0.0, 5.0, 5.0);
        let b = Rect::new(10.0, 10.0, 5.0, 5.0);
        let u = a.union(b);
        assert!(approx_eq(u.x, 0.0));
        assert!(approx_eq(u.y, 0.0));
        assert!(approx_eq(u.width, 15.0));
        assert!(approx_eq(u.height, 15.0));
    }

    #[test]
    fn test_union_overlapping() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 10.0, 10.0);
        let u = a.union(b);
        assert!(approx_eq(u.x, 0.0));
        assert!(approx_eq(u.width, 15.0));
        assert!(approx_eq(u.height, 15.0));
    }

    #[test]
    fn test_union_with_empty() {
        let a = Rect::new(1.0, 2.0, 5.0, 5.0);
        let empty = Rect::zero();
        assert_eq!(a.union(empty), a);
        assert_eq!(empty.union(a), a);
    }

    #[test]
    fn test_intersection_overlap() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 10.0, 10.0);
        let i = a.intersection(b).expect("should overlap");
        assert!(approx_eq(i.x, 5.0));
        assert!(approx_eq(i.y, 5.0));
        assert!(approx_eq(i.width, 5.0));
        assert!(approx_eq(i.height, 5.0));
    }

    #[test]
    fn test_intersection_no_overlap() {
        let a = Rect::new(0.0, 0.0, 5.0, 5.0);
        let b = Rect::new(10.0, 10.0, 5.0, 5.0);
        assert!(a.intersection(b).is_none());
    }

    #[test]
    fn test_intersection_touching_edge() {
        // Rects that touch at an edge but don't overlap (width would be 0).
        let a = Rect::new(0.0, 0.0, 5.0, 5.0);
        let b = Rect::new(5.0, 0.0, 5.0, 5.0);
        assert!(a.intersection(b).is_none()); // zero-width intersection
    }

    #[test]
    fn test_expand_by() {
        let r = Rect::new(1.0, 1.0, 8.0, 8.0);
        let e = r.expand_by(1.0);
        assert!(approx_eq(e.x, 0.0));
        assert!(approx_eq(e.y, 0.0));
        assert!(approx_eq(e.width, 10.0));
        assert!(approx_eq(e.height, 10.0));
    }

    #[test]
    fn test_expand_by_negative() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        let s = r.expand_by(-1.0); // shrink by 1 on each side
        assert!(approx_eq(s.x, 1.0));
        assert!(approx_eq(s.width, 8.0));
    }
}
