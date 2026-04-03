// ============================================================================
// Bezier2D.swift — Quadratic and Cubic Bézier Curves
// ============================================================================
//
// A Bézier curve of degree n is defined by n+1 control points P0…Pn.
// The curve traces a smooth path that passes through P0 and Pn, and is
// "attracted" to the intermediate control points without passing through them.
//
// Evaluation — De Casteljau's Algorithm
// --------------------------------------
// De Casteljau's algorithm evaluates B(t) by repeated linear interpolation:
//
//   Start with the control points P0, P1, …, Pn.
//   At each step, lerp each consecutive pair: P_i' = lerp(P_i, P_{i+1}, t).
//   After n rounds, a single point remains — that is B(t).
//
// This is numerically superior to the explicit Bernstein polynomial expansion
// because it avoids large intermediate coefficients that can cancel.
//
// Splitting — De Casteljau at an Arbitrary t
// -------------------------------------------
// The intermediate points from De Casteljau not only give us B(t), they also
// give us the control points of the two sub-curves [0,t] and [t,1]:
//   - Left sub-curve control points: the first point at each level.
//   - Right sub-curve control points: the last point at each level.
//
// Bounding Box
// ------------
// The bounding box of a Bézier curve is not simply the bounding box of its
// control points (those form a convex hull, not the tight bbox). The curve
// reaches extrema where its derivative is zero. We solve each coordinate
// component independently.
//
// Layer: G2D02 — depends on point2d and trig
// ============================================================================

import Trig
import Point2D

// ============================================================================
// Internal helper — find t values where a scalar cubic derivative is zero
// ============================================================================

/// Find t ∈ (0,1) where the derivative of a scalar cubic component is zero.
///
/// The derivative of a cubic Bézier component f(t) = c0(1-t)³ + 3c1t(1-t)² +
/// 3c2t²(1-t) + c3t³ is quadratic:
///   f'(t) = a·t² + b·t + c = 0
/// where:
///   a = -3c0 + 9c1 - 9c2 + 3c3
///   b =  6c0 - 12c1 + 6c2
///   c = -3c0 + 3c1
private func cubicExtrema(_ c0: Double, _ c1: Double, _ c2: Double, _ c3: Double) -> [Double] {
    let a = -3*c0 + 9*c1 - 9*c2 + 3*c3
    let b =  6*c0 - 12*c1 + 6*c2
    let c = -3*c0 + 3*c1

    var ts: [Double] = []

    if Swift.abs(a) < 1e-12 {
        // Derivative is linear: b*t + c = 0 → t = -c/b
        if Swift.abs(b) > 1e-12 {
            let t = -c / b
            if t > 0 && t < 1 { ts.append(t) }
        }
        return ts
    }

    let disc = b*b - 4*a*c
    guard disc >= 0 else { return ts }
    let sqrtDisc = Trig.sqrt(disc)
    let t1 = (-b + sqrtDisc) / (2*a)
    let t2 = (-b - sqrtDisc) / (2*a)
    if t1 > 0 && t1 < 1 { ts.append(t1) }
    if t2 > 0 && t2 < 1 { ts.append(t2) }
    return ts
}

// ============================================================================
// QuadraticBezier
// ============================================================================

/// A quadratic Bézier curve with control points P0, P1, P2.
///
/// The curve starts at P0, ends at P2, and is pulled toward P1.
public struct QuadraticBezier {
    public let p0: Point
    public let p1: Point
    public let p2: Point

    public init(_ p0: Point, _ p1: Point, _ p2: Point) {
        self.p0 = p0; self.p1 = p1; self.p2 = p2
    }

    // -------------------------------------------------------------------------
    // Evaluation
    // -------------------------------------------------------------------------

    /// Evaluate the curve at parameter t ∈ [0,1] using De Casteljau.
    ///
    /// De Casteljau step for quadratic (2 rounds):
    ///   Round 1: q0 = lerp(P0, P1, t),  q1 = lerp(P1, P2, t)
    ///   Round 2: result = lerp(q0, q1, t)
    public func eval(_ t: Double) -> Point {
        let q0 = p0.lerp(p1, t)
        let q1 = p1.lerp(p2, t)
        return q0.lerp(q1, t)
    }

    /// First derivative B'(t) — the tangent direction at t.
    ///
    ///   B'(t) = 2[(P1-P0)(1-t) + (P2-P1)t]
    public func derivative(_ t: Double) -> Point {
        let d0 = p1.subtract(p0)   // P1 - P0
        let d1 = p2.subtract(p1)   // P2 - P1
        let lx = d0.x + t*(d1.x - d0.x)
        let ly = d0.y + t*(d1.y - d0.y)
        return Point(2*lx, 2*ly)
    }

    // -------------------------------------------------------------------------
    // Splitting
    // -------------------------------------------------------------------------

    /// Split the curve at t into two quadratic sub-curves.
    ///
    /// Returns (left, right) where left covers [0,t] and right covers [t,1].
    public func split(_ t: Double) -> (QuadraticBezier, QuadraticBezier) {
        let q0 = p0.lerp(p1, t)
        let q1 = p1.lerp(p2, t)
        let m  = q0.lerp(q1, t)
        return (QuadraticBezier(p0, q0, m),
                QuadraticBezier(m, q1, p2))
    }

    // -------------------------------------------------------------------------
    // Adaptive polyline
    // -------------------------------------------------------------------------

    /// Recursively subdivide until the chord error is below `tolerance`.
    ///
    /// The chord midpoint approximation: for a segment from A to B, the
    /// midpoint of the chord is (A+B)/2. We compare this to the true curve
    /// midpoint B(0.5). If the distance is within `tolerance`, we treat the
    /// segment as flat and stop subdividing.
    public func polyline(tolerance: Double = 0.5) -> [Point] {
        var pts: [Point] = [p0]
        subdivide(tolerance: tolerance, pts: &pts)
        return pts
    }

    private func subdivide(tolerance: Double, pts: inout [Point]) {
        let start = p0
        let end   = p2
        let mid   = eval(0.5)
        let chordMid = Point((start.x+end.x)/2, (start.y+end.y)/2)
        if mid.distance(to: chordMid) <= tolerance {
            pts.append(end)
        } else {
            let (left, right) = split(0.5)
            left.subdivide(tolerance: tolerance, pts: &pts)
            right.subdivide(tolerance: tolerance, pts: &pts)
        }
    }

    // -------------------------------------------------------------------------
    // Bounding box
    // -------------------------------------------------------------------------

    /// Tight axis-aligned bounding box.
    ///
    /// The quadratic derivative B'(t) = 2[(P1-P0)(1-t) + (P2-P1)t] is linear
    /// in t. Setting each component to zero:
    ///   tx = (P1.x-P0.x) / (P0.x - 2P1.x + P2.x)   (if denominator ≠ 0)
    ///   ty = (P1.y-P0.y) / (P0.y - 2P1.y + P2.y)   (if denominator ≠ 0)
    public var boundingBox: Rect {
        var xs = [p0.x, p2.x]
        var ys = [p0.y, p2.y]

        // X extremum: B'x(t) = 2[(P1.x-P0.x) + t(P0.x-2P1.x+P2.x)] = 0
        //   → t = -(P1.x-P0.x) / (P0.x-2P1.x+P2.x) = (P0.x-P1.x) / (P0.x-2P1.x+P2.x)
        let dxDenom = p0.x - 2*p1.x + p2.x
        if Swift.abs(dxDenom) > 1e-12 {
            let tx = (p0.x - p1.x) / dxDenom
            if tx > 0 && tx < 1 { xs.append(eval(tx).x) }
        }

        // Y extremum: same formula for y component
        let dyDenom = p0.y - 2*p1.y + p2.y
        if Swift.abs(dyDenom) > 1e-12 {
            let ty = (p0.y - p1.y) / dyDenom
            if ty > 0 && ty < 1 { ys.append(eval(ty).y) }
        }

        let minX = xs.min()!
        let maxX = xs.max()!
        let minY = ys.min()!
        let maxY = ys.max()!
        return Rect(minX, minY, maxX-minX, maxY-minY)
    }

    // -------------------------------------------------------------------------
    // Degree elevation
    // -------------------------------------------------------------------------

    /// Elevate to a cubic Bézier that traces the same curve.
    ///
    /// Any quadratic can be represented as a cubic. The control point formula:
    ///   Q0 = P0
    ///   Q1 = P0/3 + 2P1/3
    ///   Q2 = 2P1/3 + P2/3
    ///   Q3 = P2
    public func elevate() -> CubicBezier {
        let q1 = Point(p0.x/3 + 2*p1.x/3, p0.y/3 + 2*p1.y/3)
        let q2 = Point(2*p1.x/3 + p2.x/3, 2*p1.y/3 + p2.y/3)
        return CubicBezier(p0, q1, q2, p2)
    }
}

// ============================================================================
// CubicBezier
// ============================================================================

/// A cubic Bézier curve with control points P0, P1, P2, P3.
///
/// The curve starts at P0, ends at P3, and is shaped by P1 and P2.
/// The tangent at P0 points toward P1; the tangent at P3 points from P2.
public struct CubicBezier {
    public let p0: Point
    public let p1: Point
    public let p2: Point
    public let p3: Point

    public init(_ p0: Point, _ p1: Point, _ p2: Point, _ p3: Point) {
        self.p0 = p0; self.p1 = p1; self.p2 = p2; self.p3 = p3
    }

    // -------------------------------------------------------------------------
    // Evaluation
    // -------------------------------------------------------------------------

    /// Evaluate the curve at parameter t ∈ [0,1] using De Casteljau.
    ///
    /// De Casteljau for cubic (3 rounds):
    ///   Round 1: 3 new points (lerp consecutive pairs)
    ///   Round 2: 2 new points
    ///   Round 3: 1 point — the result
    public func eval(_ t: Double) -> Point {
        let q0 = p0.lerp(p1, t)
        let q1 = p1.lerp(p2, t)
        let q2 = p2.lerp(p3, t)
        let r0 = q0.lerp(q1, t)
        let r1 = q1.lerp(q2, t)
        return r0.lerp(r1, t)
    }

    /// First derivative B'(t).
    ///
    ///   B'(t) = 3[(P1-P0)(1-t)² + 2(P2-P1)t(1-t) + (P3-P2)t²]
    public func derivative(_ t: Double) -> Point {
        let d0 = p1.subtract(p0)
        let d1 = p2.subtract(p1)
        let d2 = p3.subtract(p2)
        let u = 1 - t
        let x = 3 * (d0.x*u*u + 2*d1.x*t*u + d2.x*t*t)
        let y = 3 * (d0.y*u*u + 2*d1.y*t*u + d2.y*t*t)
        return Point(x, y)
    }

    // -------------------------------------------------------------------------
    // Splitting
    // -------------------------------------------------------------------------

    /// Split at t into two cubic sub-curves.
    public func split(_ t: Double) -> (CubicBezier, CubicBezier) {
        let q0 = p0.lerp(p1, t)
        let q1 = p1.lerp(p2, t)
        let q2 = p2.lerp(p3, t)
        let r0 = q0.lerp(q1, t)
        let r1 = q1.lerp(q2, t)
        let m  = r0.lerp(r1, t)
        return (CubicBezier(p0, q0, r0, m),
                CubicBezier(m, r1, q2, p3))
    }

    // -------------------------------------------------------------------------
    // Adaptive polyline
    // -------------------------------------------------------------------------

    /// Recursively subdivide until chord error ≤ tolerance.
    public func polyline(tolerance: Double = 0.5) -> [Point] {
        var pts: [Point] = [p0]
        subdivide(tolerance: tolerance, pts: &pts)
        return pts
    }

    private func subdivide(tolerance: Double, pts: inout [Point]) {
        let mid      = eval(0.5)
        let chordMid = Point((p0.x+p3.x)/2, (p0.y+p3.y)/2)
        if mid.distance(to: chordMid) <= tolerance {
            pts.append(p3)
        } else {
            let (left, right) = split(0.5)
            left.subdivide(tolerance: tolerance, pts: &pts)
            right.subdivide(tolerance: tolerance, pts: &pts)
        }
    }

    // -------------------------------------------------------------------------
    // Bounding box
    // -------------------------------------------------------------------------

    /// Tight axis-aligned bounding box.
    ///
    /// The derivative is quadratic, so zeros are found via the quadratic formula.
    public var boundingBox: Rect {
        var xs = [p0.x, p3.x]
        var ys = [p0.y, p3.y]

        for t in cubicExtrema(p0.x, p1.x, p2.x, p3.x) {
            xs.append(eval(t).x)
        }
        for t in cubicExtrema(p0.y, p1.y, p2.y, p3.y) {
            ys.append(eval(t).y)
        }

        let minX = xs.min()!
        let maxX = xs.max()!
        let minY = ys.min()!
        let maxY = ys.max()!
        return Rect(minX, minY, maxX-minX, maxY-minY)
    }
}
