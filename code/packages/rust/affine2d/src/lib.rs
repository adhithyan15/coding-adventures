// ============================================================================
// affine2d — 2D Affine Transformation Matrix
// ============================================================================
//
// This crate provides `Affine2D`: a six-float representation of any 2D affine
// transformation. It is the same six numbers used by every major 2D graphics
// API in the world:
//
//   SVG `matrix(a,b,c,d,e,f)`,  HTML Canvas `setTransform(a,b,c,d,e,f)`,
//   PDF `cm` operator,  Cairo `cairo_matrix_t`,  Core Graphics `CGAffineTransform`
//
// ## The 3×3 Matrix
//
// To combine translation (addition) with rotation/scale (multiplication), we
// embed 2D points in homogeneous coordinates:
//
//   point (x, y) → column vector [x, y, 1]ᵀ
//
// A general 2D affine transform is then:
//
//   ┌ a  c  e ┐   ┌ x ┐   ┌ a·x + c·y + e ┐
//   │ b  d  f │ × │ y │ = │ b·x + d·y + f │
//   └ 0  0  1 ┘   └ 1 ┘   └       1       ┘
//
// The bottom row is always [0, 0, 1] for affine transforms, so we never store
// it. This gives us the compact 6-float representation: [a, b, c, d, e, f].
//
// ## Field Mapping
//
// | Our field | SVG matrix() | Cairo    | Core Graphics |
// |-----------|--------------|----------|---------------|
// | a         | a            | xx       | a             |
// | b         | b            | yx       | b             |
// | c         | c            | xy       | c             |
// | d         | d            | yy       | d             |
// | e         | e            | x0       | tx            |
// | f         | f            | y0       | ty            |
//
// ## Dependencies
//
// - `trig` (PHY00) for sin, cos, tan in rotation and skew factories
// - `point2d` (G2D00) for Point as input/output type

use trig;
use point2d::Point;

// ============================================================================
// Affine2D
// ============================================================================

/// A 2D affine transformation matrix stored as 6 floats [a, b, c, d, e, f].
///
/// The transform formula is:
///   x' = a·x + c·y + e
///   y' = b·x + d·y + f
///
/// This encodes any combination of translation, rotation, uniform/non-uniform
/// scaling, and shearing. The matrix is composable: `self.multiply(other)`
/// produces a new transform that first applies `other`, then `self`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine2D {
    /// Horizontal scaling / cosine of rotation angle.
    pub a: f64,
    /// Vertical shear / sine of rotation angle.
    pub b: f64,
    /// Horizontal shear / negative sine of rotation angle.
    pub c: f64,
    /// Vertical scaling / cosine of rotation angle.
    pub d: f64,
    /// Horizontal translation.
    pub e: f64,
    /// Vertical translation.
    pub f: f64,
}

impl Affine2D {
    // -----------------------------------------------------------------------
    // Factory Functions
    // -----------------------------------------------------------------------

    /// Create an Affine2D directly from its six components.
    ///
    /// Components follow the SVG/Canvas convention: [a, b, c, d, e, f].
    pub fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// The identity transform: leaves every point unchanged.
    ///
    /// The identity matrix is:
    ///   ┌ 1  0  0 ┐
    ///   │ 0  1  0 │
    ///   └ 0  0  1 ┘
    ///
    /// Multiplying any matrix by identity returns the same matrix.
    pub fn identity() -> Self {
        // a=1, b=0, c=0, d=1, e=0, f=0
        Self::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    }

    /// A pure translation by (tx, ty).
    ///
    /// Adds (tx, ty) to every transformed point without changing direction.
    /// Matrix: [1, 0, 0, 1, tx, ty]
    pub fn translate(tx: f64, ty: f64) -> Self {
        // Translation lives in the last column (e, f).
        Self::new(1.0, 0.0, 0.0, 1.0, tx, ty)
    }

    /// A counterclockwise rotation by `angle` radians.
    ///
    /// For a rotation by θ (CCW, math convention):
    ///   a = cos θ,  b = sin θ,  c = -sin θ,  d = cos θ
    ///
    /// This is the standard rotation matrix in homogeneous coordinates:
    ///   ┌ cos θ  -sin θ  0 ┐
    ///   │ sin θ   cos θ  0 │
    ///   └   0       0    1 ┘
    ///
    /// Note on SVG convention: SVG has Y pointing downward, so a positive
    /// angle produces a clockwise visual rotation on screen. The math is
    /// still CCW in the mathematical (Y-up) sense.
    pub fn rotate(angle: f64) -> Self {
        let c = trig::cos(angle);
        let s = trig::sin(angle);
        // a=cos, b=sin, c=-sin, d=cos, e=0, f=0
        Self::new(c, s, -s, c, 0.0, 0.0)
    }

    /// Rotation about an arbitrary center point.
    ///
    /// Equivalent to: translate(-center) → rotate(angle) → translate(center).
    /// Composes three transforms in order.
    pub fn rotate_around(center: Point, angle: f64) -> Self {
        // Translate center to origin, rotate, translate back.
        Self::translate(-center.x, -center.y)
            .then(&Self::rotate(angle))
            .then(&Self::translate(center.x, center.y))
    }

    /// Non-uniform scale: (sx, sy) in X and Y.
    ///
    /// Matrix: [sx, 0, 0, sy, 0, 0]
    /// A scale factor of 1.0 leaves that axis unchanged.
    /// Negative values mirror across the axis.
    pub fn scale(sx: f64, sy: f64) -> Self {
        // Scaling lives in the diagonal (a, d).
        Self::new(sx, 0.0, 0.0, sy, 0.0, 0.0)
    }

    /// Uniform scale: same factor in both axes.
    pub fn scale_uniform(s: f64) -> Self {
        Self::scale(s, s)
    }

    /// Horizontal skew (shear along X) by `angle` radians.
    ///
    /// Shears points horizontally: x' = x + tan(angle)·y, y' = y.
    /// Matrix: [1, 0, tan(angle), 1, 0, 0]
    pub fn skew_x(angle: f64) -> Self {
        // The off-diagonal c element produces horizontal shear.
        Self::new(1.0, 0.0, trig::tan(angle), 1.0, 0.0, 0.0)
    }

    /// Vertical skew (shear along Y) by `angle` radians.
    ///
    /// Shears points vertically: x' = x, y' = tan(angle)·x + y.
    /// Matrix: [1, tan(angle), 0, 1, 0, 0]
    pub fn skew_y(angle: f64) -> Self {
        // The off-diagonal b element produces vertical shear.
        Self::new(1.0, trig::tan(angle), 0.0, 1.0, 0.0, 0.0)
    }

    // -----------------------------------------------------------------------
    // Composition Helpers
    // -----------------------------------------------------------------------

    /// Apply `next` after `self`. Returns the composed transform.
    ///
    /// This is a convenience alias for `next.multiply(self)`, which means
    /// `self.then(next)` first applies `self`, then `next`.
    pub fn then(&self, next: &Affine2D) -> Affine2D {
        next.multiply(self)
    }

    // -----------------------------------------------------------------------
    // Operations
    // -----------------------------------------------------------------------

    /// Compose two transforms: `self` applied after `other`.
    ///
    /// Given:
    ///   A = [a1, b1, c1, d1, e1, f1]  (self)
    ///   B = [a2, b2, c2, d2, e2, f2]  (other)
    ///
    /// The 3×3 product A·B gives:
    ///   result.a = a1·a2 + c1·b2
    ///   result.b = b1·a2 + d1·b2
    ///   result.c = a1·c2 + c1·d2
    ///   result.d = b1·c2 + d1·d2
    ///   result.e = a1·e2 + c1·f2 + e1
    ///   result.f = b1·e2 + d1·f2 + f1
    ///
    /// This applies `other` first, then `self` — standard left-to-right
    /// function composition: (self ∘ other)(p) = self(other(p)).
    pub fn multiply(&self, other: &Affine2D) -> Affine2D {
        Affine2D::new(
            self.a * other.a + self.c * other.b,  // result.a
            self.b * other.a + self.d * other.b,  // result.b
            self.a * other.c + self.c * other.d,  // result.c
            self.b * other.c + self.d * other.d,  // result.d
            self.a * other.e + self.c * other.f + self.e, // result.e
            self.b * other.e + self.d * other.f + self.f, // result.f
        )
    }

    /// Apply this transform to a point (including translation).
    ///
    /// The full affine transform: x' = a·x + c·y + e,  y' = b·x + d·y + f.
    /// Use this for positions (points that should be translated).
    pub fn apply_to_point(&self, p: Point) -> Point {
        Point::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }

    /// Apply this transform to a vector (ignoring translation).
    ///
    /// The linear part only: x' = a·x + c·y,  y' = b·x + d·y.
    /// Use this for directions and displacements (vectors that should NOT
    /// be translated — only rotated/scaled/sheared).
    ///
    /// Example: a normal vector or a tangent direction should be transformed
    /// with this method, not `apply_to_point`.
    pub fn apply_to_vector(&self, v: Point) -> Point {
        // No translation component — only the 2×2 linear part.
        Point::new(
            self.a * v.x + self.c * v.y,
            self.b * v.x + self.d * v.y,
        )
    }

    /// The determinant of the 2×2 linear part: a·d - b·c.
    ///
    /// The determinant tells you:
    /// - Its absolute value is the scale factor for areas.
    /// - Its sign tells you whether the transform flips orientation.
    /// - If it is zero, the transform collapses space to a line — not invertible.
    pub fn determinant(&self) -> f64 {
        // det = a·d - b·c  (the 2×2 determinant ignoring translation)
        self.a * self.d - self.b * self.c
    }

    /// Compute the inverse of this transform, or `None` if it is not invertible.
    ///
    /// A transform is invertible iff its determinant is non-zero.
    ///
    /// The inverse formula for [a, b, c, d, e, f] with det = a·d - b·c:
    ///   inv_a =  d / det
    ///   inv_b = -b / det
    ///   inv_c = -c / det
    ///   inv_d =  a / det
    ///   inv_e = (c·f - d·e) / det
    ///   inv_f = (b·e - a·f) / det
    ///
    /// This is derived from the standard formula for the inverse of a 2×3
    /// augmented matrix with the implicit third row [0, 0, 1].
    pub fn invert(&self) -> Option<Affine2D> {
        let det = self.determinant();
        if det.abs() < 1e-12 {
            // Singular matrix: cannot be inverted (the transform collapses space).
            return None;
        }
        Some(Affine2D::new(
            self.d / det,                             // inv_a
            -self.b / det,                            // inv_b
            -self.c / det,                            // inv_c
            self.a / det,                             // inv_d
            (self.c * self.f - self.d * self.e) / det, // inv_e
            (self.b * self.e - self.a * self.f) / det, // inv_f
        ))
    }

    /// True if this transform is (approximately) the identity.
    ///
    /// Checks all six components against identity within 1e-10 tolerance.
    pub fn is_identity(&self) -> bool {
        let eps = 1e-10;
        (self.a - 1.0).abs() < eps
            && self.b.abs() < eps
            && self.c.abs() < eps
            && (self.d - 1.0).abs() < eps
            && self.e.abs() < eps
            && self.f.abs() < eps
    }

    /// True if this transform is a pure translation (no rotation/scale/shear).
    ///
    /// Checks that a≈1, b≈0, c≈0, d≈1 within 1e-10.
    pub fn is_translation_only(&self) -> bool {
        let eps = 1e-10;
        (self.a - 1.0).abs() < eps
            && self.b.abs() < eps
            && self.c.abs() < eps
            && (self.d - 1.0).abs() < eps
    }

    /// Return the six components as an array [a, b, c, d, e, f].
    pub fn to_array(&self) -> [f64; 6] {
        [self.a, self.b, self.c, self.d, self.e, self.f]
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

    fn affine_approx_eq(a: &Affine2D, b: &Affine2D) -> bool {
        approx_eq(a.a, b.a)
            && approx_eq(a.b, b.b)
            && approx_eq(a.c, b.c)
            && approx_eq(a.d, b.d)
            && approx_eq(a.e, b.e)
            && approx_eq(a.f, b.f)
    }

    // -----------------------------------------------------------------------
    // Factory functions
    // -----------------------------------------------------------------------

    #[test]
    fn test_identity() {
        let id = Affine2D::identity();
        assert_eq!(id.a, 1.0);
        assert_eq!(id.b, 0.0);
        assert_eq!(id.c, 0.0);
        assert_eq!(id.d, 1.0);
        assert_eq!(id.e, 0.0);
        assert_eq!(id.f, 0.0);
    }

    #[test]
    fn test_identity_leaves_point_unchanged() {
        let id = Affine2D::identity();
        let p = Point::new(3.0, 4.0);
        assert!(point_approx_eq(id.apply_to_point(p), p));
    }

    #[test]
    fn test_translate() {
        let t = Affine2D::translate(5.0, -3.0);
        let p = Point::new(1.0, 2.0);
        let q = t.apply_to_point(p);
        assert!(approx_eq(q.x, 6.0));
        assert!(approx_eq(q.y, -1.0));
    }

    #[test]
    fn test_translate_does_not_move_vector() {
        let t = Affine2D::translate(5.0, -3.0);
        let v = Point::new(1.0, 2.0);
        let w = t.apply_to_vector(v);
        // Vectors are not translated.
        assert!(point_approx_eq(w, v));
    }

    #[test]
    fn test_rotate_90() {
        let r = Affine2D::rotate(trig::PI / 2.0);
        let p = Point::new(1.0, 0.0);
        let q = r.apply_to_point(p);
        // Rotating (1,0) by 90° CCW → (0, 1)
        assert!(approx_eq(q.x, 0.0));
        assert!(approx_eq(q.y, 1.0));
    }

    #[test]
    fn test_rotate_180() {
        let r = Affine2D::rotate(trig::PI);
        let p = Point::new(1.0, 0.0);
        let q = r.apply_to_point(p);
        // 180° rotation: (1, 0) → (-1, 0)
        assert!(approx_eq(q.x, -1.0));
        assert!(approx_eq(q.y, 0.0));
    }

    #[test]
    fn test_rotate_360_is_identity() {
        let r = Affine2D::rotate(2.0 * trig::PI);
        assert!(r.is_identity());
    }

    #[test]
    fn test_rotate_around_center() {
        let center = Point::new(1.0, 0.0);
        // Rotating by 90° around (1,0): point (1,0) stays at (1,0).
        let r = Affine2D::rotate_around(center, trig::PI / 2.0);
        let p = center;
        let q = r.apply_to_point(p);
        assert!(approx_eq(q.x, 1.0));
        assert!(approx_eq(q.y, 0.0));
    }

    #[test]
    fn test_scale() {
        let s = Affine2D::scale(2.0, 3.0);
        let p = Point::new(1.0, 1.0);
        let q = s.apply_to_point(p);
        assert!(approx_eq(q.x, 2.0));
        assert!(approx_eq(q.y, 3.0));
    }

    #[test]
    fn test_scale_uniform() {
        let s = Affine2D::scale_uniform(5.0);
        let p = Point::new(2.0, 3.0);
        let q = s.apply_to_point(p);
        assert!(approx_eq(q.x, 10.0));
        assert!(approx_eq(q.y, 15.0));
    }

    #[test]
    fn test_skew_x() {
        // skew_x(45°): tan(45°) = 1, so x' = x + y, y' = y
        let sk = Affine2D::skew_x(trig::PI / 4.0);
        let p = Point::new(0.0, 1.0);
        let q = sk.apply_to_point(p);
        assert!(approx_eq(q.x, 1.0)); // 0 + tan(45)*1 = 1
        assert!(approx_eq(q.y, 1.0));
    }

    #[test]
    fn test_skew_y() {
        let sk = Affine2D::skew_y(trig::PI / 4.0);
        let p = Point::new(1.0, 0.0);
        let q = sk.apply_to_point(p);
        assert!(approx_eq(q.x, 1.0));
        assert!(approx_eq(q.y, 1.0)); // tan(45)*1 + 0 = 1
    }

    // -----------------------------------------------------------------------
    // Composition (multiply)
    // -----------------------------------------------------------------------

    #[test]
    fn test_multiply_identity() {
        let a = Affine2D::translate(3.0, 4.0);
        let id = Affine2D::identity();
        assert!(affine_approx_eq(&a.multiply(&id), &a));
        assert!(affine_approx_eq(&id.multiply(&a), &a));
    }

    #[test]
    fn test_multiply_scale_then_translate() {
        // Scale by 2, then translate by (10, 0).
        // Point (1,1) → scale → (2,2) → translate → (12, 2).
        let scale = Affine2D::scale_uniform(2.0);
        let translate = Affine2D::translate(10.0, 0.0);
        let composed = translate.multiply(&scale); // translate(scale(p))
        let p = Point::new(1.0, 1.0);
        let q = composed.apply_to_point(p);
        assert!(approx_eq(q.x, 12.0));
        assert!(approx_eq(q.y, 2.0));
    }

    #[test]
    fn test_multiply_rotate_twice() {
        // Two 90° rotations = 180° rotation.
        let r90 = Affine2D::rotate(trig::PI / 2.0);
        let r180 = Affine2D::rotate(trig::PI);
        let composed = r90.multiply(&r90);
        assert!(affine_approx_eq(&composed, &r180));
    }

    // -----------------------------------------------------------------------
    // Determinant and invert
    // -----------------------------------------------------------------------

    #[test]
    fn test_determinant_identity() {
        assert!(approx_eq(Affine2D::identity().determinant(), 1.0));
    }

    #[test]
    fn test_determinant_scale() {
        let s = Affine2D::scale(2.0, 3.0);
        assert!(approx_eq(s.determinant(), 6.0)); // 2*3 - 0*0
    }

    #[test]
    fn test_determinant_rotation() {
        // Rotation preserves area: det should be 1.
        let r = Affine2D::rotate(trig::PI / 3.0);
        assert!(approx_eq(r.determinant(), 1.0));
    }

    #[test]
    fn test_invert_identity() {
        let id = Affine2D::identity();
        let inv = id.invert().unwrap();
        assert!(affine_approx_eq(&inv, &id));
    }

    #[test]
    fn test_invert_translation() {
        let t = Affine2D::translate(3.0, -7.0);
        let inv = t.invert().unwrap();
        let composed = t.multiply(&inv);
        assert!(composed.is_identity());
    }

    #[test]
    fn test_invert_rotate() {
        let r = Affine2D::rotate(trig::PI / 4.0);
        let inv = r.invert().unwrap();
        let composed = r.multiply(&inv);
        assert!(composed.is_identity());
    }

    #[test]
    fn test_invert_singular() {
        // A matrix that collapses to a line is not invertible.
        let singular = Affine2D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        assert!(singular.invert().is_none());
    }

    #[test]
    fn test_invert_scale() {
        let s = Affine2D::scale(2.0, 4.0);
        let inv = s.invert().unwrap();
        let p = Point::new(8.0, 12.0);
        let q = inv.apply_to_point(s.apply_to_point(p));
        assert!(point_approx_eq(q, p));
    }

    // -----------------------------------------------------------------------
    // Predicates
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_identity() {
        assert!(Affine2D::identity().is_identity());
        assert!(!Affine2D::translate(1.0, 0.0).is_identity());
        assert!(!Affine2D::scale(2.0, 1.0).is_identity());
    }

    #[test]
    fn test_is_translation_only() {
        assert!(Affine2D::identity().is_translation_only());
        assert!(Affine2D::translate(5.0, 3.0).is_translation_only());
        assert!(!Affine2D::rotate(0.1).is_translation_only());
        assert!(!Affine2D::scale(2.0, 1.0).is_translation_only());
    }

    #[test]
    fn test_to_array() {
        let m = Affine2D::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        assert_eq!(m.to_array(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    // -----------------------------------------------------------------------
    // apply_to_vector (no translation)
    // -----------------------------------------------------------------------

    #[test]
    fn test_apply_to_vector_rotate() {
        // Rotating vector (1,0) by 90° should give (0,1).
        let r = Affine2D::rotate(trig::PI / 2.0);
        let v = Point::new(1.0, 0.0);
        let w = r.apply_to_vector(v);
        assert!(approx_eq(w.x, 0.0));
        assert!(approx_eq(w.y, 1.0));
    }

    #[test]
    fn test_apply_to_vector_ignores_translation() {
        let t = Affine2D::translate(100.0, 200.0);
        let v = Point::new(1.0, 1.0);
        let w = t.apply_to_vector(v);
        // Translation does not affect vectors.
        assert!(point_approx_eq(w, v));
    }
}
