// ============================================================================
// Arc2D.swift — Elliptical Arcs and SVG Arc Parameterization
// ============================================================================
//
// An elliptical arc is a portion of an ellipse. In 2D graphics (SVG, fonts,
// CAD), arcs are commonly expressed in two different forms:
//
//   1. Center form (CenterArc):
//      • Center point (cx, cy)
//      • Semi-axes rx, ry
//      • X-axis rotation xRotation (tilts the ellipse)
//      • Start angle startAngle (where the arc begins on the ellipse)
//      • Sweep angle sweepAngle (how far around to go; positive = CCW)
//
//   2. Endpoint form (SvgArc):
//      • Start point `from`, end point `to`
//      • Semi-axes rx, ry
//      • X-axis rotation xRotation
//      • Two flags: largeArc (choose the longer or shorter arc)
//                   sweep (choose CCW or CW arc)
//
// SVG uses the endpoint form. Rendering engines prefer the center form.
// We implement conversion both ways (SvgArc → CenterArc) following the
// W3C SVG specification §B.2.4.
//
// Cubic Bézier Approximation
// --------------------------
// Arcs cannot be represented exactly with polynomial Bézier curves, but a
// single cubic Bézier can approximate a quarter-circle arc with error < 0.03%.
//
// The key formula for a circular arc of sweep angle s:
//   k = (4/3) · tan(s/4)
//
// For an arc from angle θ to θ+s on the unit circle:
//   P0 = (cos θ,  sin θ)
//   P1 = P0 + k · (-sin θ, cos θ)    — tangent direction at start
//   P2 = P3 + k · (sin(θ+s), -cos(θ+s))  — tangent direction at end (reversed)
//   P3 = (cos(θ+s), sin(θ+s))
//
// For arcs larger than 90°, we split into multiple segments.
//
// Layer: G2D03 — depends on trig, point2d, bezier2d
// ============================================================================

import Trig
import Point2D
import Bezier2D

// ============================================================================
// CenterArc
// ============================================================================

/// An elliptical arc in center parameterization.
///
/// The parametric equation for a point at angle θ on the arc:
///   x' = cx + rx·cos(xRotation)·cos(θ) - ry·sin(xRotation)·sin(θ)
///   y' = cy + rx·sin(xRotation)·cos(θ) + ry·cos(xRotation)·sin(θ)
///
/// where `xRotation` is the angle the ellipse's major axis makes with the x-axis.
public struct CenterArc {
    public let center: Point
    public let rx: Double
    public let ry: Double
    public let startAngle: Double   // radians
    public let sweepAngle: Double   // radians; positive = CCW
    public let xRotation: Double    // radians

    public init(center: Point, rx: Double, ry: Double,
                startAngle: Double, sweepAngle: Double, xRotation: Double) {
        self.center = center
        self.rx = rx; self.ry = ry
        self.startAngle = startAngle
        self.sweepAngle = sweepAngle
        self.xRotation = xRotation
    }

    // -------------------------------------------------------------------------
    // Evaluation
    // -------------------------------------------------------------------------

    /// Evaluate the arc at parameter t ∈ [0,1].
    ///
    /// t=0 gives the start point; t=1 gives the end point.
    /// Internally, t maps to angle = startAngle + t·sweepAngle.
    public func eval(_ t: Double) -> Point {
        let angle = startAngle + t * sweepAngle
        let cosR = Trig.cos(xRotation)
        let sinR = Trig.sin(xRotation)
        let cosA = Trig.cos(angle)
        let sinA = Trig.sin(angle)
        let x = center.x + rx * cosR * cosA - ry * sinR * sinA
        let y = center.y + rx * sinR * cosA + ry * cosR * sinA
        return Point(x, y)
    }

    // -------------------------------------------------------------------------
    // Bounding box (100-point sampling)
    // -------------------------------------------------------------------------

    /// Approximate bounding box by sampling 100 points along the arc.
    ///
    /// An exact analytical bbox requires finding where the parametric derivative
    /// is zero, which involves solving transcendental equations through the
    /// x-rotation. Sampling 100 evenly-spaced points gives sufficient accuracy
    /// for all practical rendering purposes.
    public var boundingBox: Rect {
        var minX = Double.greatestFiniteMagnitude
        var maxX = -Double.greatestFiniteMagnitude
        var minY = Double.greatestFiniteMagnitude
        var maxY = -Double.greatestFiniteMagnitude

        let n = 100
        for i in 0...n {
            let t = Double(i) / Double(n)
            let p = eval(t)
            if p.x < minX { minX = p.x }
            if p.x > maxX { maxX = p.x }
            if p.y < minY { minY = p.y }
            if p.y > maxY { maxY = p.y }
        }

        return Rect(minX, minY, maxX - minX, maxY - minY)
    }

    // -------------------------------------------------------------------------
    // Cubic Bézier approximation
    // -------------------------------------------------------------------------

    /// Convert this arc to one or more cubic Bézier curves.
    ///
    /// Each segment covers at most 90° (π/2 radians) of sweep.
    ///
    /// The formula for k is k = (4/3)·tan(s/4) where s is the per-segment
    /// sweep angle. This gives the control point offset along the tangent at
    /// each endpoint that produces the best cubic approximation to a circular
    /// arc. For ellipses, we scale by rx and ry respectively.
    public func toCubicBeziers() -> [CubicBezier] {
        let twoPI = 2.0 * PI
        let halfPI = PI / 2.0

        let absSwp = Swift.abs(sweepAngle)
        // ceil without Foundation: convert to Int, then add 1 if not exact
        let ratio = absSwp / halfPI
        let ratioInt = Int(ratio)
        let nRaw = (Double(ratioInt) < ratio) ? ratioInt + 1 : ratioInt
        let n = Swift.max(1, nRaw)
        let segSwp = sweepAngle / Double(n)

        let k = (4.0 / 3.0) * Trig.tan(segSwp / 4.0)
        let cosR = Trig.cos(xRotation)
        let sinR = Trig.sin(xRotation)

        var curves: [CubicBezier] = []
        var theta = startAngle

        for _ in 0..<n {
            let theta2 = theta + segSwp
            let cos1 = Trig.cos(theta)
            let sin1 = Trig.sin(theta)
            let cos2 = Trig.cos(theta2)
            let sin2 = Trig.sin(theta2)

            // P0 — start of this segment
            let p0x = center.x + rx * cosR * cos1 - ry * sinR * sin1
            let p0y = center.y + rx * sinR * cos1 + ry * cosR * sin1

            // P3 — end of this segment
            let p3x = center.x + rx * cosR * cos2 - ry * sinR * sin2
            let p3y = center.y + rx * sinR * cos2 + ry * cosR * sin2

            // P1 = P0 + k · tangent-at-start (rotated into ellipse space)
            // Tangent at θ on the ellipse (unrotated): (-rx·sin θ, ry·cos θ)
            let t1x = -rx * sin1
            let t1y =  ry * cos1
            let p1x = p0x + k * (cosR * t1x - sinR * t1y)
            let p1y = p0y + k * (sinR * t1x + cosR * t1y)

            // P2 = P3 - k · tangent-at-end
            let t2x = -rx * sin2
            let t2y =  ry * cos2
            let p2x = p3x - k * (cosR * t2x - sinR * t2y)
            let p2y = p3y - k * (sinR * t2x + cosR * t2y)

            curves.append(CubicBezier(
                Point(p0x, p0y), Point(p1x, p1y),
                Point(p2x, p2y), Point(p3x, p3y)
            ))

            theta = theta2
        }

        return curves
    }
}

// ============================================================================
// SvgArc
// ============================================================================

/// An elliptical arc in SVG endpoint parameterization.
///
/// This matches the SVG `A` path command:
///   A rx ry xRotation largeArc sweep x y
///
/// where (x, y) is `to` and the current path position is `from`.
public struct SvgArc {
    public let from: Point
    public let to: Point
    public let rx: Double
    public let ry: Double
    public let xRotation: Double   // radians
    public let largeArc: Bool
    public let sweep: Bool

    public init(from: Point, to: Point, rx: Double, ry: Double,
                xRotation: Double, largeArc: Bool, sweep: Bool) {
        self.from = from; self.to = to
        self.rx = rx; self.ry = ry
        self.xRotation = xRotation
        self.largeArc = largeArc; self.sweep = sweep
    }

    // -------------------------------------------------------------------------
    // Conversion to CenterArc (W3C SVG §B.2.4)
    // -------------------------------------------------------------------------

    /// Convert SVG endpoint form to center form.
    ///
    /// Returns nil for degenerate arcs (from == to, or zero radii, or
    /// endpoints too far apart for the given radii).
    ///
    /// The algorithm follows W3C SVG §B.2.4 exactly:
    ///   Step 1: Compute (x1', y1') in the rotated frame
    ///   Step 2: Compute the center (cx', cy') in the rotated frame
    ///   Step 3: Compute cx, cy from cx' and cy'
    ///   Step 4: Compute startAngle and sweepAngle
    public func toCenterArc() -> CenterArc? {
        // Degenerate: same start and end
        if from.x == to.x && from.y == to.y { return nil }

        var rx = Swift.abs(self.rx)
        var ry = Swift.abs(self.ry)
        if rx < 1e-12 || ry < 1e-12 { return nil }

        let cosR = Trig.cos(xRotation)
        let sinR = Trig.sin(xRotation)

        // Step 1: midpoint in rotated frame
        let dx2 = (from.x - to.x) / 2.0
        let dy2 = (from.y - to.y) / 2.0
        let x1p =  cosR * dx2 + sinR * dy2
        let y1p = -sinR * dx2 + cosR * dy2

        // Step 1b: radius scale-up if endpoints are too far apart
        let x1pSq = x1p * x1p
        let y1pSq = y1p * y1p
        let rxSq  = rx * rx
        let rySq  = ry * ry
        let lambda = x1pSq / rxSq + y1pSq / rySq
        if lambda > 1.0 {
            let scale = Trig.sqrt(lambda)
            rx *= scale
            ry *= scale
        }
        let rxSq2 = rx * rx
        let rySq2 = ry * ry

        // Step 2: compute (cx', cy')
        let num = rxSq2 * rySq2 - rxSq2 * y1pSq - rySq2 * x1pSq
        let den = rxSq2 * y1pSq + rySq2 * x1pSq
        let sqrtArg = Swift.max(0.0, num / den)
        let sq = Trig.sqrt(sqrtArg)

        // The sign of sq depends on the largeArc and sweep flags.
        // If largeArc == sweep, the center is on one side; otherwise the other.
        let sign: Double = (largeArc == sweep) ? -1.0 : 1.0
        let cxp = sign * sq * rx * y1p / ry
        let cyp = sign * sq * (-ry) * x1p / rx

        // Step 3: transform back to original coordinate system
        let midX = (from.x + to.x) / 2.0
        let midY = (from.y + to.y) / 2.0
        let cx = cosR * cxp - sinR * cyp + midX
        let cy = sinR * cxp + cosR * cyp + midY

        // Step 4: compute startAngle and sweepAngle using the angle formula
        //
        // angleBetween(u, v) = signed angle from u to v.
        // We use atan2(cross(u,v), dot(u,v)) for the signed version.
        //
        // acos is computed via atan2(sqrt(1-c²), c) since trig has no acos.
        let ux = (x1p - cxp) / rx
        let uy = (y1p - cyp) / ry
        let vx = (-x1p - cxp) / rx
        let vy = (-y1p - cyp) / ry

        let startAngle = signedAngle(1.0, 0.0, ux, uy)
        var sweepAngle = signedAngle(ux, uy, vx, vy)

        let twoPI = 2.0 * PI

        if !sweep && sweepAngle > 0 {
            sweepAngle -= twoPI
        } else if sweep && sweepAngle < 0 {
            sweepAngle += twoPI
        }

        return CenterArc(center: Point(cx, cy), rx: rx, ry: ry,
                         startAngle: startAngle, sweepAngle: sweepAngle,
                         xRotation: xRotation)
    }

    /// Compute the signed angle from vector (u1,u2) to vector (v1,v2).
    ///
    /// We use atan2(cross, dot) directly for a numerically stable signed angle.
    private func signedAngle(_ u1: Double, _ u2: Double,
                              _ v1: Double, _ v2: Double) -> Double {
        let dot    = u1*v1 + u2*v2
        let cross  = u1*v2 - u2*v1
        return Trig.atan2(cross, dot)
    }
}
