// Package bezier2d provides quadratic and cubic Bezier curves.
//
// All evaluation uses de Casteljau's algorithm for numerical stability.
// No trig dependency for curve evaluation (trig.Sqrt used for bounding box).
package bezier2d

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/point2d"
	"github.com/adhithyan15/coding-adventures/code/packages/go/trig"
)

// ============================================================================
// QuadraticBezier
// ============================================================================

// QuadraticBezier is a degree-2 Bezier curve with three control points.
type QuadraticBezier struct {
	P0, P1, P2 point2d.Point
}

// EvalQuad evaluates the quadratic Bezier at t ∈ [0,1] using de Casteljau.
func EvalQuad(q QuadraticBezier, t float64) point2d.Point {
	q0 := q.P0.Lerp(q.P1, t)
	q1 := q.P1.Lerp(q.P2, t)
	return q0.Lerp(q1, t)
}

// DerivQuad returns the tangent vector at t.
func DerivQuad(q QuadraticBezier, t float64) point2d.Point {
	d0 := q.P1.Subtract(q.P0)
	d1 := q.P2.Subtract(q.P1)
	return d0.Lerp(d1, t).Scale(2)
}

// SplitQuad splits the curve at t into (left, right).
func SplitQuad(q QuadraticBezier, t float64) (QuadraticBezier, QuadraticBezier) {
	q0 := q.P0.Lerp(q.P1, t)
	q1 := q.P1.Lerp(q.P2, t)
	m := q0.Lerp(q1, t)
	return QuadraticBezier{q.P0, q0, m}, QuadraticBezier{m, q1, q.P2}
}

// PolylineQuad adaptively subdivides into a polyline within tolerance.
func PolylineQuad(q QuadraticBezier, tolerance float64) []point2d.Point {
	chordMid := q.P0.Lerp(q.P2, 0.5)
	curveMid := EvalQuad(q, 0.5)
	if chordMid.Distance(curveMid) <= tolerance {
		return []point2d.Point{q.P0, q.P2}
	}
	left, right := SplitQuad(q, 0.5)
	pts := PolylineQuad(left, tolerance)
	rpts := PolylineQuad(right, tolerance)
	return append(pts, rpts[1:]...)
}

// BboxQuad returns the tight bounding box of the quadratic Bezier.
func BboxQuad(q QuadraticBezier) point2d.Rect {
	minX, maxX := minf(q.P0.X, q.P2.X), maxf(q.P0.X, q.P2.X)
	minY, maxY := minf(q.P0.Y, q.P2.Y), maxf(q.P0.Y, q.P2.Y)

	if dx := q.P0.X - 2*q.P1.X + q.P2.X; absf(dx) > 1e-12 {
		if tx := (q.P0.X - q.P1.X) / dx; tx > 0 && tx < 1 {
			px := EvalQuad(q, tx)
			minX, maxX = minf(minX, px.X), maxf(maxX, px.X)
		}
	}
	if dy := q.P0.Y - 2*q.P1.Y + q.P2.Y; absf(dy) > 1e-12 {
		if ty := (q.P0.Y - q.P1.Y) / dy; ty > 0 && ty < 1 {
			py := EvalQuad(q, ty)
			minY, maxY = minf(minY, py.Y), maxf(maxY, py.Y)
		}
	}
	return point2d.NewRect(minX, minY, maxX-minX, maxY-minY)
}

// ElevateQuad converts a quadratic to an equivalent cubic.
func ElevateQuad(q QuadraticBezier) CubicBezier {
	q1 := q.P0.Scale(1.0 / 3).Add(q.P1.Scale(2.0 / 3))
	q2 := q.P1.Scale(2.0 / 3).Add(q.P2.Scale(1.0 / 3))
	return CubicBezier{q.P0, q1, q2, q.P2}
}

// ============================================================================
// CubicBezier
// ============================================================================

// CubicBezier is a degree-3 Bezier curve with four control points.
type CubicBezier struct {
	P0, P1, P2, P3 point2d.Point
}

// EvalCubic evaluates the cubic Bezier at t ∈ [0,1] using de Casteljau.
func EvalCubic(c CubicBezier, t float64) point2d.Point {
	p01 := c.P0.Lerp(c.P1, t)
	p12 := c.P1.Lerp(c.P2, t)
	p23 := c.P2.Lerp(c.P3, t)
	p012 := p01.Lerp(p12, t)
	p123 := p12.Lerp(p23, t)
	return p012.Lerp(p123, t)
}

// DerivCubic returns the tangent vector at t.
func DerivCubic(c CubicBezier, t float64) point2d.Point {
	d0 := c.P1.Subtract(c.P0)
	d1 := c.P2.Subtract(c.P1)
	d2 := c.P3.Subtract(c.P2)
	oneT := 1 - t
	r := d0.Scale(oneT * oneT).Add(d1.Scale(2 * oneT * t)).Add(d2.Scale(t * t))
	return r.Scale(3)
}

// SplitCubic splits the cubic at t into (left, right).
func SplitCubic(c CubicBezier, t float64) (CubicBezier, CubicBezier) {
	p01 := c.P0.Lerp(c.P1, t)
	p12 := c.P1.Lerp(c.P2, t)
	p23 := c.P2.Lerp(c.P3, t)
	p012 := p01.Lerp(p12, t)
	p123 := p12.Lerp(p23, t)
	p0123 := p012.Lerp(p123, t)
	return CubicBezier{c.P0, p01, p012, p0123}, CubicBezier{p0123, p123, p23, c.P3}
}

// PolylineCubic adaptively subdivides into a polyline within tolerance.
func PolylineCubic(c CubicBezier, tolerance float64) []point2d.Point {
	chordMid := c.P0.Lerp(c.P3, 0.5)
	curveMid := EvalCubic(c, 0.5)
	if chordMid.Distance(curveMid) <= tolerance {
		return []point2d.Point{c.P0, c.P3}
	}
	left, right := SplitCubic(c, 0.5)
	pts := PolylineCubic(left, tolerance)
	return append(pts, PolylineCubic(right, tolerance)[1:]...)
}

// BboxCubic returns the tight bounding box of the cubic Bezier.
func BboxCubic(c CubicBezier) point2d.Rect {
	minX, maxX := minf(c.P0.X, c.P3.X), maxf(c.P0.X, c.P3.X)
	minY, maxY := minf(c.P0.Y, c.P3.Y), maxf(c.P0.Y, c.P3.Y)

	for _, tx := range extrema(c.P0.X, c.P1.X, c.P2.X, c.P3.X) {
		px := EvalCubic(c, tx)
		minX, maxX = minf(minX, px.X), maxf(maxX, px.X)
	}
	for _, ty := range extrema(c.P0.Y, c.P1.Y, c.P2.Y, c.P3.Y) {
		py := EvalCubic(c, ty)
		minY, maxY = minf(minY, py.Y), maxf(maxY, py.Y)
	}
	return point2d.NewRect(minX, minY, maxX-minX, maxY-minY)
}

// extrema finds t in (0,1) where the cubic's derivative in one coordinate is zero.
func extrema(v0, v1, v2, v3 float64) []float64 {
	a := -3*v0 + 9*v1 - 9*v2 + 3*v3
	b := 6*v0 - 12*v1 + 6*v2
	c := -3*v0 + 3*v1
	var roots []float64
	if absf(a) < 1e-12 {
		if absf(b) > 1e-12 {
			if tx := -c / b; tx > 0 && tx < 1 {
				roots = append(roots, tx)
			}
		}
	} else {
		disc := b*b - 4*a*c
		if disc >= 0 {
			sq := trig.Sqrt(disc)
			for _, tx := range []float64{(-b + sq) / (2 * a), (-b - sq) / (2 * a)} {
				if tx > 0 && tx < 1 {
					roots = append(roots, tx)
				}
			}
		}
	}
	return roots
}

func minf(a, b float64) float64 {
	if a < b {
		return a
	}
	return b
}
func maxf(a, b float64) float64 {
	if a > b {
		return a
	}
	return b
}
func absf(x float64) float64 {
	if x < 0 {
		return -x
	}
	return x
}
