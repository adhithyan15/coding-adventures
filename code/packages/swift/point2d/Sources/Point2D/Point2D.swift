// ============================================================================
// Point2D.swift — Immutable 2D Point/Vector and Axis-Aligned Bounding Rect
// ============================================================================
//
// A Point is an (x, y) pair used as both a position and a 2D vector.
// A Rect  is (x, y, width, height) — the SVG/CSS bounding box convention.
//
// All operations are pure: they return new values rather than mutating.
// ============================================================================

import Trig

// ----------------------------------------------------------------------------
// Point
// ----------------------------------------------------------------------------

/// A 2D point or direction vector.
public struct Point: Equatable {
    public let x: Double
    public let y: Double

    public init(_ x: Double, _ y: Double) {
        self.x = x; self.y = y
    }

    /// Add two points: (x1+x2, y1+y2).
    public func add(_ other: Point) -> Point { Point(x+other.x, y+other.y) }

    /// Subtract: self - other.
    public func subtract(_ other: Point) -> Point { Point(x-other.x, y-other.y) }

    /// Scale by scalar s.
    public func scale(_ s: Double) -> Point { Point(x*s, y*s) }

    /// Negate: (-x, -y).
    public func negate() -> Point { Point(-x, -y) }

    /// Dot product: x1*x2 + y1*y2.
    public func dot(_ other: Point) -> Double { x*other.x + y*other.y }

    /// 2D cross product: x1*y2 - y1*x2.
    public func cross(_ other: Point) -> Double { x*other.y - y*other.x }

    /// Squared magnitude: x^2 + y^2.
    public var magnitudeSquared: Double { x*x + y*y }

    /// Euclidean magnitude.
    public var magnitude: Double { Trig.sqrt(magnitudeSquared) }

    /// Unit vector in the same direction (identity if zero).
    public func normalize() -> Point {
        let m = magnitude
        guard m > 1e-15 else { return self }
        return scale(1.0 / m)
    }

    /// Squared distance to another point.
    public func distanceSquared(to other: Point) -> Double {
        subtract(other).magnitudeSquared
    }

    /// Euclidean distance to another point.
    public func distance(to other: Point) -> Double {
        Trig.sqrt(distanceSquared(to: other))
    }

    /// Linear interpolation to `other` at t ∈ [0,1].
    public func lerp(_ other: Point, _ t: Double) -> Point {
        Point(x + t*(other.x - x), y + t*(other.y - y))
    }

    /// Perpendicular vector (90° CCW): (-y, x).
    public var perpendicular: Point { Point(-y, x) }

    /// Angle from the +X axis (radians, in (-π, π]).
    public var angle: Double { Trig.atan2(y, x) }
}

// ----------------------------------------------------------------------------
// Rect
// ----------------------------------------------------------------------------

/// Axis-aligned bounding box stored as (x, y, width, height).
public struct Rect: Equatable {
    public let x: Double
    public let y: Double
    public let width: Double
    public let height: Double

    public init(_ x: Double, _ y: Double, _ width: Double, _ height: Double) {
        self.x = x; self.y = y; self.width = width; self.height = height
    }

    /// True if `pt` is in the half-open interval [x, x+w) × [y, y+h).
    public func containsPoint(_ pt: Point) -> Bool {
        pt.x >= x && pt.x < x+width && pt.y >= y && pt.y < y+height
    }

    /// Smallest Rect that contains both self and other.
    public func union(_ other: Rect) -> Rect {
        let x0 = Swift.min(x, other.x)
        let y0 = Swift.min(y, other.y)
        let x1 = Swift.max(x+width, other.x+other.width)
        let y1 = Swift.max(y+height, other.y+other.height)
        return Rect(x0, y0, x1-x0, y1-y0)
    }

    /// Intersection of self and other, or nil if disjoint.
    public func intersection(_ other: Rect) -> Rect? {
        let x0 = Swift.max(x, other.x)
        let y0 = Swift.max(y, other.y)
        let x1 = Swift.min(x+width, other.x+other.width)
        let y1 = Swift.min(y+height, other.y+other.height)
        guard x1 > x0 && y1 > y0 else { return nil }
        return Rect(x0, y0, x1-x0, y1-y0)
    }

    /// Enlarge by `margin` on all four sides.
    public func expandedBy(_ margin: Double) -> Rect {
        Rect(x-margin, y-margin, width+2*margin, height+2*margin)
    }
}
