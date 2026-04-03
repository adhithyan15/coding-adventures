// Package point2d provides 2D point/vector and axis-aligned bounding box types.
//
// # Overview
//
// Two fundamental types for 2D geometry:
//
//   - Point — a 2D position and 2D vector. A position (where is something?)
//     and a direction+magnitude (how far and which way?) are both described by
//     two floats (x, y). The same type serves both purposes.
//
//   - Rect — an axis-aligned bounding box (AABB) given by origin (x, y)
//     and dimensions (width, height).
//
// All operations produce new values — value-type semantics, no aliasing.
//
// # Dependency
//
// The Angle() method calls trig.Atan2 from PHY00. No other trig functions are
// needed in this package.
package point2d

import (
	"math"

	"github.com/adhithyan15/coding-adventures/code/packages/go/trig"
)

// ============================================================================
// Point
// ============================================================================

// Point is a 2D position (or 2D vector). Both interpretations share the same
// underlying pair of float64 values (X, Y). Which interpretation applies
// depends on context.
type Point struct {
	// X is the horizontal coordinate.
	X float64
	// Y is the vertical coordinate.
	Y float64
}

// NewPoint creates a point at (x, y).
func NewPoint(x, y float64) Point {
	return Point{X: x, Y: y}
}

// Origin returns the point at (0, 0) — the additive identity.
func Origin() Point {
	return Point{X: 0, Y: 0}
}

// Add returns the component-wise sum: (x1+x2, y1+y2).
func (p Point) Add(other Point) Point {
	return Point{X: p.X + other.X, Y: p.Y + other.Y}
}

// Subtract returns the component-wise difference: (x1-x2, y1-y2).
func (p Point) Subtract(other Point) Point {
	return Point{X: p.X - other.X, Y: p.Y - other.Y}
}

// Scale returns the scalar multiplication: (s*x, s*y).
func (p Point) Scale(s float64) Point {
	return Point{X: p.X * s, Y: p.Y * s}
}

// Negate returns the additive inverse: (-x, -y).
func (p Point) Negate() Point {
	return Point{X: -p.X, Y: -p.Y}
}

// Dot returns the dot product: x1*x2 + y1*y2.
// Encodes the angle θ between vectors: u·v = |u||v|cos(θ).
func (p Point) Dot(other Point) float64 {
	return p.X*other.X + p.Y*other.Y
}

// Cross returns the 2D cross product scalar: x1*y2 - y1*x2.
// Positive means other is to the left of p (CCW turn).
func (p Point) Cross(other Point) float64 {
	return p.X*other.Y - p.Y*other.X
}

// Magnitude returns the Euclidean length: sqrt(x²+y²).
// Uses trig.Sqrt from PHY00.
func (p Point) Magnitude() float64 {
	return trig.Sqrt(p.X*p.X + p.Y*p.Y)
}

// MagnitudeSquared returns x²+y². No square root — cheaper for comparisons.
func (p Point) MagnitudeSquared() float64 {
	return p.X*p.X + p.Y*p.Y
}

// Normalize returns the unit vector in the same direction.
// Returns Origin() if the magnitude is zero.
func (p Point) Normalize() Point {
	m := p.Magnitude()
	if m < 1e-12 {
		return Origin()
	}
	return Point{X: p.X / m, Y: p.Y / m}
}

// Distance returns the Euclidean distance to another point.
func (p Point) Distance(other Point) float64 {
	return p.Subtract(other).Magnitude()
}

// DistanceSquared returns the squared Euclidean distance. No sqrt.
func (p Point) DistanceSquared(other Point) float64 {
	return p.Subtract(other).MagnitudeSquared()
}

// Lerp returns the linear interpolation: p + t*(other-p).
// t=0 → p; t=1 → other; t=0.5 → midpoint.
func (p Point) Lerp(other Point, t float64) Point {
	dx := other.X - p.X
	dy := other.Y - p.Y
	return Point{X: p.X + t*dx, Y: p.Y + t*dy}
}

// Perpendicular returns the 90° CCW rotation: (-y, x).
// Same magnitude as p. Calling twice gives Negate().
func (p Point) Perpendicular() Point {
	return Point{X: -p.Y, Y: p.X}
}

// Angle returns the direction angle in radians: atan2(y, x).
// Counterclockwise from positive X axis. Result in (-π, π].
// Always calls trig.Atan2 from PHY00.
func (p Point) Angle() float64 {
	return trig.Atan2(p.Y, p.X)
}

// ============================================================================
// Rect
// ============================================================================

// Rect is an axis-aligned bounding box (AABB).
// (X, Y) is the top-left corner; Width and Height are the extents.
type Rect struct {
	X      float64
	Y      float64
	Width  float64
	Height float64
}

// NewRect creates a rect with the given origin and dimensions.
func NewRect(x, y, width, height float64) Rect {
	return Rect{X: x, Y: y, Width: width, Height: height}
}

// RectFromPoints constructs a rect from two corner points.
func RectFromPoints(min, max Point) Rect {
	return Rect{X: min.X, Y: min.Y, Width: max.X - min.X, Height: max.Y - min.Y}
}

// ZeroRect returns the empty rect at the origin.
func ZeroRect() Rect {
	return Rect{}
}

// MinPoint returns the top-left corner.
func (r Rect) MinPoint() Point {
	return Point{X: r.X, Y: r.Y}
}

// MaxPoint returns the bottom-right corner.
func (r Rect) MaxPoint() Point {
	return Point{X: r.X + r.Width, Y: r.Y + r.Height}
}

// Center returns the center point.
func (r Rect) Center() Point {
	return Point{X: r.X + r.Width/2, Y: r.Y + r.Height/2}
}

// IsEmpty returns true if Width <= 0 or Height <= 0.
func (r Rect) IsEmpty() bool {
	return r.Width <= 0 || r.Height <= 0
}

// ContainsPoint returns true if p is inside this rect.
// Half-open interval: [x, x+width) × [y, y+height).
func (r Rect) ContainsPoint(p Point) bool {
	return p.X >= r.X && p.X < r.X+r.Width &&
		p.Y >= r.Y && p.Y < r.Y+r.Height
}

// Union returns the smallest rect containing both r and other.
func (r Rect) Union(other Rect) Rect {
	if r.IsEmpty() {
		return other
	}
	if other.IsEmpty() {
		return r
	}
	minX := math.Min(r.X, other.X)
	minY := math.Min(r.Y, other.Y)
	maxX := math.Max(r.X+r.Width, other.X+other.Width)
	maxY := math.Max(r.Y+r.Height, other.Y+other.Height)
	return Rect{X: minX, Y: minY, Width: maxX - minX, Height: maxY - minY}
}

// Intersection returns the overlap region and true, or a zero rect and false.
func (r Rect) Intersection(other Rect) (Rect, bool) {
	ix := math.Max(r.X, other.X)
	iy := math.Max(r.Y, other.Y)
	iw := math.Min(r.X+r.Width, other.X+other.Width) - ix
	ih := math.Min(r.Y+r.Height, other.Y+other.Height) - iy
	if iw <= 0 || ih <= 0 {
		return ZeroRect(), false
	}
	return Rect{X: ix, Y: iy, Width: iw, Height: ih}, true
}

// ExpandBy grows all four edges outward by amount.
func (r Rect) ExpandBy(amount float64) Rect {
	return Rect{
		X:      r.X - amount,
		Y:      r.Y - amount,
		Width:  r.Width + 2*amount,
		Height: r.Height + 2*amount,
	}
}
