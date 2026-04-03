// Package affine2d provides the standard 2D affine transformation matrix.
//
// The 6-float representation [a, b, c, d, e, f] is used by SVG matrix(),
// HTML Canvas setTransform(), PDF, Cairo, and Core Graphics:
//
//	x' = a*x + c*y + e
//	y' = b*x + d*y + f
package affine2d

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/point2d"
	"github.com/adhithyan15/coding-adventures/code/packages/go/trig"
)

// Affine2D is a 2D affine transformation stored as [a, b, c, d, e, f].
type Affine2D struct {
	A, B, C, D, E, F float64
}

// Identity returns the identity transform: leaves every point unchanged.
func Identity() Affine2D { return Affine2D{A: 1, D: 1} }

// Translate returns a pure translation by (tx, ty).
func Translate(tx, ty float64) Affine2D { return Affine2D{A: 1, D: 1, E: tx, F: ty} }

// Rotate returns a CCW rotation by angle radians.
func Rotate(angle float64) Affine2D {
	c := trig.Cos(angle)
	s := trig.Sin(angle)
	return Affine2D{A: c, B: s, C: -s, D: c}
}

// RotateAround returns a rotation about center by angle.
func RotateAround(center point2d.Point, angle float64) Affine2D {
	return Then(Then(Translate(-center.X, -center.Y), Rotate(angle)), Translate(center.X, center.Y))
}

// Scale returns a non-uniform scale.
func Scale(sx, sy float64) Affine2D { return Affine2D{A: sx, D: sy} }

// ScaleUniform returns a uniform scale.
func ScaleUniform(s float64) Affine2D { return Scale(s, s) }

// SkewX returns a horizontal skew by angle radians.
func SkewX(angle float64) Affine2D { return Affine2D{A: 1, C: trig.Tan(angle), D: 1} }

// SkewY returns a vertical skew by angle radians.
func SkewY(angle float64) Affine2D { return Affine2D{A: 1, B: trig.Tan(angle), D: 1} }

// Then returns a composed with b (first a, then b).
func Then(a, b Affine2D) Affine2D { return Multiply(b, a) }

// Multiply returns the composed transform: self applied after other.
func Multiply(self, other Affine2D) Affine2D {
	return Affine2D{
		A: self.A*other.A + self.C*other.B,
		B: self.B*other.A + self.D*other.B,
		C: self.A*other.C + self.C*other.D,
		D: self.B*other.C + self.D*other.D,
		E: self.A*other.E + self.C*other.F + self.E,
		F: self.B*other.E + self.D*other.F + self.F,
	}
}

// ApplyToPoint applies the full affine transform to a point.
func ApplyToPoint(m Affine2D, p point2d.Point) point2d.Point {
	return point2d.NewPoint(m.A*p.X+m.C*p.Y+m.E, m.B*p.X+m.D*p.Y+m.F)
}

// ApplyToVector applies the linear part only (ignores translation).
func ApplyToVector(m Affine2D, v point2d.Point) point2d.Point {
	return point2d.NewPoint(m.A*v.X+m.C*v.Y, m.B*v.X+m.D*v.Y)
}

// Determinant returns a*d - b*c.
func Determinant(m Affine2D) float64 { return m.A*m.D - m.B*m.C }

// Invert returns the inverse of m and true, or zero value and false if singular.
func Invert(m Affine2D) (Affine2D, bool) {
	det := Determinant(m)
	if abs64(det) < 1e-12 {
		return Affine2D{}, false
	}
	return Affine2D{
		A: m.D / det,
		B: -m.B / det,
		C: -m.C / det,
		D: m.A / det,
		E: (m.C*m.F - m.D*m.E) / det,
		F: (m.B*m.E - m.A*m.F) / det,
	}, true
}

// IsIdentity returns true if m is approximately the identity (within 1e-10).
func IsIdentity(m Affine2D) bool {
	const eps = 1e-10
	return abs64(m.A-1) < eps && abs64(m.B) < eps &&
		abs64(m.C) < eps && abs64(m.D-1) < eps &&
		abs64(m.E) < eps && abs64(m.F) < eps
}

// IsTranslationOnly returns true if m is a pure translation.
func IsTranslationOnly(m Affine2D) bool {
	const eps = 1e-10
	return abs64(m.A-1) < eps && abs64(m.B) < eps &&
		abs64(m.C) < eps && abs64(m.D-1) < eps
}

// ToArray returns the six components as [a, b, c, d, e, f].
func ToArray(m Affine2D) [6]float64 {
	return [6]float64{m.A, m.B, m.C, m.D, m.E, m.F}
}

func abs64(x float64) float64 {
	if x < 0 {
		return -x
	}
	return x
}
