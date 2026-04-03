// ============================================================================
// Affine2D.swift — 2D Affine Transformation Matrix
// ============================================================================
//
// An affine transformation in 2D is a 3×3 homogeneous matrix that preserves
// parallel lines. It can represent translation, rotation, scaling, and skew
// in a single matrix that combines with other transforms by multiplication.
//
// The matrix layout we use (column-major, row vectors on the right):
//
//   | a  c  e |
//   | b  d  f |
//   | 0  0  1 |
//
// Applied to a point (x, y):
//   x' = a·x + c·y + e
//   y' = b·x + d·y + f
//
// Applied to a vector (dx, dy) — no translation, since vectors represent
// direction/displacement, not position:
//   dx' = a·dx + c·dy
//   dy' = b·dx + d·dy
//
// This layout matches the SVG/CSS matrix() function:
//   matrix(a, b, c, d, e, f)
//
// Composition ("then"):
//   To apply transform A first, then B, multiply B·A (right-to-left).
//   We expose this as `self.then(other)` which returns B·A = other composed with self.
//
// Why homogeneous coordinates?
// ----------------------------
// Translation is not a linear operation in 2D — you can't express "move by
// (tx, ty)" as a 2×2 matrix multiplication. By lifting points to 3D as
// (x, y, 1) and matrices to 3×3, translation becomes a linear operation.
// This lets us combine any sequence of transforms by matrix multiplication.
//
// Layer: G2D01 — depends on point2d and trig
// ============================================================================

import Trig
import Point2D

// ============================================================================
// Affine2D
// ============================================================================

/// A 2D affine transformation stored as a 3×3 homogeneous matrix.
///
/// Only the top two rows vary; the bottom row is always [0, 0, 1].
///
///   | a  c  e |
///   | b  d  f |
///   | 0  0  1 |
///
public struct Affine2D: Equatable {

    // The six independent matrix entries.
    // Named to match the SVG matrix(a, b, c, d, e, f) convention.
    public let a: Double   // x-scale / cos component
    public let b: Double   // y-shear / sin component
    public let c: Double   // x-shear / -sin component
    public let d: Double   // y-scale / cos component
    public let e: Double   // x-translation
    public let f: Double   // y-translation

    /// Designated initializer. Builds the matrix from its six entries.
    public init(_ a: Double, _ b: Double, _ c: Double,
                _ d: Double, _ e: Double, _ f: Double) {
        self.a = a; self.b = b; self.c = c
        self.d = d; self.e = e; self.f = f
    }

    // =========================================================================
    // Factory methods — standard transforms
    // =========================================================================

    /// The identity transform: no translation, no rotation, no scale.
    ///
    ///   | 1  0  0 |
    ///   | 0  1  0 |
    ///   | 0  0  1 |
    public static var identity: Affine2D {
        Affine2D(1, 0, 0, 1, 0, 0)
    }

    /// Translate by (tx, ty).
    ///
    ///   | 1  0  tx |
    ///   | 0  1  ty |
    ///   | 0  0   1 |
    public static func translate(_ tx: Double, _ ty: Double) -> Affine2D {
        Affine2D(1, 0, 0, 1, tx, ty)
    }

    /// Rotate counter-clockwise by `angle` radians about the origin.
    ///
    ///   | cos  -sin  0 |
    ///   | sin   cos  0 |
    ///   |   0     0  1 |
    ///
    /// Note: in screen coordinates (y-axis pointing down), this appears as a
    /// clockwise rotation. In mathematical coordinates (y-axis up) it is CCW.
    public static func rotate(_ angle: Double) -> Affine2D {
        let c = Trig.cos(angle)
        let s = Trig.sin(angle)
        return Affine2D(c, s, -s, c, 0, 0)
    }

    /// Rotate counter-clockwise by `angle` radians about point `pivot`.
    ///
    /// Algorithm:
    ///   1. Translate pivot to origin: translate(-px, -py)
    ///   2. Rotate about origin
    ///   3. Translate back: translate(+px, +py)
    public static func rotateAround(_ angle: Double, pivot: Point) -> Affine2D {
        translate(-pivot.x, -pivot.y)
            .then(rotate(angle))
            .then(translate(pivot.x, pivot.y))
    }

    /// Scale by (sx, sy) about the origin.
    ///
    ///   | sx   0  0 |
    ///   |  0  sy  0 |
    ///   |  0   0  1 |
    public static func scale(_ sx: Double, _ sy: Double) -> Affine2D {
        Affine2D(sx, 0, 0, sy, 0, 0)
    }

    /// Uniform scale by `s` in both axes.
    public static func scaleUniform(_ s: Double) -> Affine2D {
        scale(s, s)
    }

    /// Horizontal shear (skew along X-axis) by `angle` radians.
    ///
    ///   | 1  tan(angle)  0 |
    ///   | 0       1      0 |
    ///   | 0       0      1 |
    public static func skewX(_ angle: Double) -> Affine2D {
        Affine2D(1, 0, Trig.tan(angle), 1, 0, 0)
    }

    /// Vertical shear (skew along Y-axis) by `angle` radians.
    ///
    ///   |      1   0  0 |
    ///   | tan(a)   1  0 |
    ///   |      0   0  1 |
    public static func skewY(_ angle: Double) -> Affine2D {
        Affine2D(1, Trig.tan(angle), 0, 1, 0, 0)
    }

    // =========================================================================
    // Composition
    // =========================================================================

    /// Return a new transform that applies `self` first, then `other`.
    ///
    /// Matrix multiplication: result = other · self
    ///
    /// Column-major 3×3 product (keeping only the top 2×3 rows):
    ///
    ///   | a2  c2  e2 |   | a1  c1  e1 |
    ///   | b2  d2  f2 | × | b1  d1  f1 |
    ///   |  0   0   1 |   |  0   0   1 |
    ///
    ///   a' = a2·a1 + c2·b1
    ///   b' = b2·a1 + d2·b1
    ///   c' = a2·c1 + c2·d1
    ///   d' = b2·c1 + d2·d1
    ///   e' = a2·e1 + c2·f1 + e2
    ///   f' = b2·e1 + d2·f1 + f2
    public func then(_ other: Affine2D) -> Affine2D {
        let a1 = self.a, b1 = self.b, c1 = self.c
        let d1 = self.d, e1 = self.e, f1 = self.f
        let a2 = other.a, b2 = other.b, c2 = other.c
        let d2 = other.d, e2 = other.e, f2 = other.f

        return Affine2D(
            a2*a1 + c2*b1,   // a'
            b2*a1 + d2*b1,   // b'
            a2*c1 + c2*d1,   // c'
            b2*c1 + d2*d1,   // d'
            a2*e1 + c2*f1 + e2,  // e'
            b2*e1 + d2*f1 + f2   // f'
        )
    }

    // =========================================================================
    // Application
    // =========================================================================

    /// Apply this transform to a point (includes translation).
    ///
    ///   x' = a·x + c·y + e
    ///   y' = b·x + d·y + f
    public func applyToPoint(_ p: Point) -> Point {
        Point(a*p.x + c*p.y + e,
              b*p.x + d*p.y + f)
    }

    /// Apply this transform to a vector (excludes translation).
    ///
    ///   dx' = a·dx + c·dy
    ///   dy' = b·dx + d·dy
    ///
    /// Vectors represent displacement, so translation does not apply.
    public func applyToVector(_ v: Point) -> Point {
        Point(a*v.x + c*v.y,
              b*v.x + d*v.y)
    }

    // =========================================================================
    // Properties
    // =========================================================================

    /// The determinant of the linear (2×2) part: a·d - b·c.
    ///
    /// Interpretation:
    ///   > 0 — orientation-preserving (no flip)
    ///   < 0 — orientation-reversing (includes a reflection)
    ///   = 0 — degenerate (collapses area to zero — not invertible)
    public var determinant: Double { a*d - b*c }

    /// The inverse transform, or nil if the matrix is singular.
    ///
    /// The 2×2 inverse is:
    ///   inv = (1/det) · | d  -c |
    ///                   | -b   a |
    ///
    /// The translation part (e, f) is solved by: -inv · (e, f).
    public var inverted: Affine2D? {
        let det = determinant
        guard Swift.abs(det) > 1e-15 else { return nil }
        let invDet = 1.0 / det
        let ia = d * invDet
        let ib = -b * invDet
        let ic = -c * invDet
        let id = a * invDet
        let ie = -(ia * e + ic * f)
        let if_ = -(ib * e + id * f)
        return Affine2D(ia, ib, ic, id, ie, if_)
    }

    /// True if this is the identity transform (within floating-point epsilon).
    public var isIdentity: Bool {
        Swift.abs(a - 1) < 1e-12 && Swift.abs(b) < 1e-12 &&
        Swift.abs(c) < 1e-12 && Swift.abs(d - 1) < 1e-12 &&
        Swift.abs(e) < 1e-12 && Swift.abs(f) < 1e-12
    }

    /// True if this is a pure translation (a=1, b=0, c=0, d=1).
    public var isTranslationOnly: Bool {
        Swift.abs(a - 1) < 1e-12 && Swift.abs(b) < 1e-12 &&
        Swift.abs(c) < 1e-12 && Swift.abs(d - 1) < 1e-12
    }

    /// Return the six entries as an array: [a, b, c, d, e, f].
    public func toArray() -> [Double] { [a, b, c, d, e, f] }
}
