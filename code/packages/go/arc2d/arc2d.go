// Package arc2d provides elliptical arc representations and operations.
//
// Two representations are supported:
//   - CenterArc: center + radii + angles (natural for rendering)
//   - SvgArc: endpoint + flags (the SVG path "A" command format)
//
// Conversion between them uses the W3C SVG spec §B.2.4 algorithm.
package arc2d

import (
	"errors"
	"math"

	"github.com/adhithyan15/coding-adventures/code/packages/go/bezier2d"
	"github.com/adhithyan15/coding-adventures/code/packages/go/point2d"
	"github.com/adhithyan15/coding-adventures/code/packages/go/trig"
)

// ============================================================================
// CenterArc
// ============================================================================

// CenterArc is an elliptical arc defined by center form.
//
// Fields:
//   - Center: the center of the ellipse
//   - Rx, Ry: semi-major and semi-minor radii (must be > 0)
//   - StartAngle: angle (radians) of the arc start, measured in the ellipse's
//     local un-rotated coordinate system
//   - SweepAngle: signed angular extent; positive = counter-clockwise
//   - XRotation: rotation of the ellipse axes from the X axis (radians, CCW)
type CenterArc struct {
	Center                 point2d.Point
	Rx, Ry                 float64
	StartAngle, SweepAngle float64
	XRotation              float64
}

// ErrInvalidCenterArc is the panic value for invalid or non-finite center-form
// input or derived output. The existing value-returning API has no error slot.
var ErrInvalidCenterArc = errors.New("arc2d: invalid center arc")

// ErrInvalidSweep is the panic value for a finite sweep exceeding one turn.
var ErrInvalidSweep = errors.New("arc2d: sweep exceeds one turn")

func finite(values ...float64) bool {
	for _, value := range values {
		if math.IsNaN(value) || math.IsInf(value, 0) {
			return false
		}
	}
	return true
}

func validateCenterArc(a CenterArc) {
	if !finite(a.Center.X, a.Center.Y, a.Rx, a.Ry, a.StartAngle, a.SweepAngle, a.XRotation) || a.Rx <= 0 || a.Ry <= 0 {
		panic(ErrInvalidCenterArc)
	}
	if math.Abs(a.SweepAngle) > trig.TwoPI {
		panic(ErrInvalidSweep)
	}
}

func centerPoint(a CenterArc, angle float64) point2d.Point {
	if !finite(angle) {
		panic(ErrInvalidCenterArc)
	}
	cosR, sinR := trig.Cos(a.XRotation), trig.Sin(a.XRotation)
	lx, ly := a.Rx*trig.Cos(angle), a.Ry*trig.Sin(angle)
	x, y := a.Center.X+cosR*lx-sinR*ly, a.Center.Y+sinR*lx+cosR*ly
	if !finite(x, y) {
		panic(ErrInvalidCenterArc)
	}
	return point2d.NewPoint(x, y)
}

// EvalArc evaluates the arc at parameter t ∈ [0,1].
//
// The parametric form of an axis-aligned ellipse is:
//
//	x' = Rx * cos(θ)
//	y' = Ry * sin(θ)
//
// where θ = StartAngle + t * SweepAngle.
// Then we rotate by XRotation and translate by Center.
func EvalArc(a CenterArc, t float64) point2d.Point {
	validateCenterArc(a)
	if !finite(t) {
		panic(ErrInvalidCenterArc)
	}
	return centerPoint(a, a.StartAngle+t*a.SweepAngle)
}

// TangentArc returns the unnormalized tangent direction at parameter t ∈ [0,1].
//
// Differentiating EvalArc with respect to t:
//
//	dx/dt = SweepAngle * (-Rx * sin(θ) * cosR - Ry * cos(θ) * sinR)
//	dy/dt = SweepAngle * (-Rx * sin(θ) * sinR + Ry * cos(θ) * cosR)
func TangentArc(a CenterArc, t float64) point2d.Point {
	validateCenterArc(a)
	if !finite(t) {
		panic(ErrInvalidCenterArc)
	}
	theta := a.StartAngle + t*a.SweepAngle
	if !finite(theta) {
		panic(ErrInvalidCenterArc)
	}
	cosT, sinT := trig.Cos(theta), trig.Sin(theta)
	cosR, sinR := trig.Cos(a.XRotation), trig.Sin(a.XRotation)
	// d/dtheta of (cosR*Rx*cosT - sinR*Ry*sinT) = -cosR*Rx*sinT - sinR*Ry*cosT
	dx := a.SweepAngle * (-cosR*a.Rx*sinT - sinR*a.Ry*cosT)
	dy := a.SweepAngle * (-sinR*a.Rx*sinT + cosR*a.Ry*cosT)
	if !finite(dx, dy) {
		panic(ErrInvalidCenterArc)
	}
	return point2d.NewPoint(dx, dy)
}

// BboxArc returns the exact extrema-based axis-aligned bounds of a center arc.
// Invalid inputs or non-finite derived bounds panic with ErrInvalidCenterArc;
// a sweep exceeding one turn panics with ErrInvalidSweep.
func BboxArc(a CenterArc) point2d.Rect {
	validateCenterArc(a)
	p0 := centerPoint(a, a.StartAngle)
	minX, maxX := p0.X, p0.X
	minY, maxY := p0.Y, p0.Y
	include := func(p point2d.Point) {
		if p.X < minX {
			minX = p.X
		}
		if p.X > maxX {
			maxX = p.X
		}
		if p.Y < minY {
			minY = p.Y
		}
		if p.Y > maxY {
			maxY = p.Y
		}
	}
	include(centerPoint(a, a.StartAngle+a.SweepAngle))
	if a.SweepAngle != 0 {
		cosR, sinR := trig.Cos(a.XRotation), trig.Sin(a.XRotation)
		xAngle := trig.Atan2(-a.Ry*sinR, a.Rx*cosR)
		yAngle := trig.Atan2(a.Ry*cosR, a.Rx*sinR)
		for _, candidate := range [...]float64{xAngle, xAngle + trig.PI, yAngle, yAngle + trig.PI} {
			var distance float64
			if a.SweepAngle > 0 {
				distance = math.Mod(candidate-a.StartAngle, trig.TwoPI)
			} else {
				distance = math.Mod(a.StartAngle-candidate, trig.TwoPI)
			}
			if distance < 0 {
				distance += trig.TwoPI
			}
			if distance <= math.Abs(a.SweepAngle)+1e-14 {
				include(centerPoint(a, candidate))
			}
		}
	}
	width, height := maxX-minX, maxY-minY
	if !finite(width, height) {
		panic(ErrInvalidCenterArc)
	}
	return point2d.NewRect(minX, minY, width, height)
}

// ToCubicBeziers approximates the arc as a sequence of cubic Bezier curves.
//
// The arc is split into segments of at most π/2 (90°). Each segment is
// approximated using the well-known cubic Bezier arc approximation formula:
//
//	k = (4/3) * tan(sweep/4)
//
// where sweep is the signed sweep angle of the segment. The control points
// are placed at distance k * radius from the endpoints, in the tangent direction.
func ToCubicBeziers(a CenterArc) []bezier2d.CubicBezier {
	validateCenterArc(a)
	// Determine number of segments: ceil(|sweep| / (π/2))
	halfPi := trig.PI / 2
	nSeg := int(math.Ceil(math.Abs(a.SweepAngle) / halfPi))
	if nSeg < 1 {
		nSeg = 1
	}
	if nSeg > 4 {
		panic(ErrInvalidSweep)
	}
	segSweep := a.SweepAngle / float64(nSeg)

	cosR, sinR := trig.Cos(a.XRotation), trig.Sin(a.XRotation)

	// localPoint converts (lx, ly) in ellipse space to world space
	localToWorld := func(lx, ly float64) point2d.Point {
		rx := cosR*lx - sinR*ly
		ry := sinR*lx + cosR*ly
		x, y := a.Center.X+rx, a.Center.Y+ry
		if !finite(x, y) {
			panic(ErrInvalidCenterArc)
		}
		return point2d.NewPoint(x, y)
	}

	curves := make([]bezier2d.CubicBezier, nSeg)
	for i := 0; i < nSeg; i++ {
		t0 := a.StartAngle + float64(i)*segSweep
		t1 := t0 + segSweep
		// k = (4/3) * tan(segSweep / 4)
		k := (4.0 / 3.0) * trig.Tan(segSweep/4)

		cos0, sin0 := trig.Cos(t0), trig.Sin(t0)
		cos1, sin1 := trig.Cos(t1), trig.Sin(t1)

		// Start and end points of the segment in ellipse local space
		p0l := point2d.NewPoint(a.Rx*cos0, a.Ry*sin0)
		p3l := point2d.NewPoint(a.Rx*cos1, a.Ry*sin1)

		// Tangent at t0: (-Rx*sin0, Ry*cos0) * k
		// Tangent at t1: (-Rx*sin1, Ry*cos1) * -k (reversed)
		p1l := point2d.NewPoint(p0l.X-k*a.Rx*sin0, p0l.Y+k*a.Ry*cos0)
		p2l := point2d.NewPoint(p3l.X+k*a.Rx*sin1, p3l.Y-k*a.Ry*cos1)

		curves[i] = bezier2d.CubicBezier{
			P0: localToWorld(p0l.X, p0l.Y),
			P1: localToWorld(p1l.X, p1l.Y),
			P2: localToWorld(p2l.X, p2l.Y),
			P3: localToWorld(p3l.X, p3l.Y),
		}
	}
	return curves
}

// ============================================================================
// SvgArc
// ============================================================================

// SvgArc is an elliptical arc in SVG endpoint form (the "A" path command).
//
// Fields:
//   - From: the start point
//   - To: the end point
//   - Rx, Ry: requested semi-axes (may be scaled up if too small)
//   - XRotation: ellipse x-axis rotation in radians
//   - LargeArc: chooses the larger of the two possible arcs
//   - Sweep: true = clockwise direction
type SvgArc struct {
	From, To        point2d.Point
	Rx, Ry          float64
	XRotation       float64
	LargeArc, Sweep bool
}

// ToCenterArc converts an SvgArc to CenterArc using the W3C SVG §B.2.4 algorithm.
//
// Returns (arc, true) on success, or (zero, false) if degenerate (same endpoints
// or zero radii). The algorithm:
//  1. Rotate midpoint vector into ellipse frame
//  2. Scale radii if they are too small
//  3. Find center in rotated frame
//  4. Convert center back to original frame
//  5. Compute start angle and sweep angle
func ToCenterArc(s SvgArc) (CenterArc, bool) {
	// Degenerate: same start and end, or zero radii
	if s.From == s.To {
		return CenterArc{}, false
	}
	rx, ry := absf(s.Rx), absf(s.Ry)
	if rx < 1e-12 || ry < 1e-12 {
		return CenterArc{}, false
	}

	// Step 1: midpoint in rotated frame
	cosR, sinR := trig.Cos(s.XRotation), trig.Sin(s.XRotation)
	dx2 := (s.From.X - s.To.X) / 2
	dy2 := (s.From.Y - s.To.Y) / 2
	// x1', y1' in rotated frame
	x1p := cosR*dx2 + sinR*dy2
	y1p := -sinR*dx2 + cosR*dy2

	// Step 2: ensure radii are large enough
	// lambda = (x1'/rx)^2 + (y1'/ry)^2; if > 1 scale up
	lambda := (x1p/rx)*(x1p/rx) + (y1p/ry)*(y1p/ry)
	if lambda > 1 {
		sqrtLambda := trig.Sqrt(lambda)
		rx *= sqrtLambda
		ry *= sqrtLambda
	}

	// Step 3: compute center in rotated frame
	// num = rx^2*ry^2 - rx^2*y1'^2 - ry^2*x1'^2
	// den = rx^2*y1'^2 + ry^2*x1'^2
	rx2, ry2 := rx*rx, ry*ry
	x1p2, y1p2 := x1p*x1p, y1p*y1p
	num := rx2*ry2 - rx2*y1p2 - ry2*x1p2
	den := rx2*y1p2 + ry2*x1p2
	if den < 1e-24 {
		return CenterArc{}, false
	}
	sq := 0.0
	if num/den > 0 {
		sq = trig.Sqrt(num / den)
	}
	// Sign: if largeArc == sweep then negative, else positive
	if s.LargeArc == s.Sweep {
		sq = -sq
	}
	cxp := sq * rx * y1p / ry
	cyp := -sq * ry * x1p / rx

	// Step 4: convert center back to original frame
	mx := (s.From.X + s.To.X) / 2
	my := (s.From.Y + s.To.Y) / 2
	cx := cosR*cxp - sinR*cyp + mx
	cy := sinR*cxp + cosR*cyp + my

	// Step 5: compute angles
	// angle between two vectors u and v:
	//   u = (x1' - cx') / rx, (y1' - cy') / ry
	//   v = (-x1' - cx') / rx, (-y1' - cy') / ry
	ux := (x1p - cxp) / rx
	uy := (y1p - cyp) / ry
	vx := (-x1p - cxp) / rx
	vy := (-y1p - cyp) / ry

	startAngle := trig.Atan2(uy, ux)
	sweepAngle := angleBetween(ux, uy, vx, vy)

	// Adjust sweep direction based on the sweep flag
	if !s.Sweep && sweepAngle > 0 {
		sweepAngle -= trig.TwoPI
	} else if s.Sweep && sweepAngle < 0 {
		sweepAngle += trig.TwoPI
	}

	return CenterArc{
		Center:     point2d.NewPoint(cx, cy),
		Rx:         rx,
		Ry:         ry,
		StartAngle: startAngle,
		SweepAngle: sweepAngle,
		XRotation:  s.XRotation,
	}, true
}

// angleBetween returns the signed angle from vector (ux,uy) to (vx,vy).
//
// Formula: angle = sign(ux*vy - uy*vx) * acos(clamp(dot/(|u||v|), -1, 1))
// We compute acos via the identity: acos(c) = atan2(sqrt(1-c^2), c)
func angleBetween(ux, uy, vx, vy float64) float64 {
	dot := ux*vx + uy*vy
	magU := trig.Sqrt(ux*ux + uy*uy)
	magV := trig.Sqrt(vx*vx + vy*vy)
	if magU < 1e-12 || magV < 1e-12 {
		return 0
	}
	cosAngle := dot / (magU * magV)
	// Clamp to [-1, 1] to guard against floating-point rounding
	if cosAngle > 1 {
		cosAngle = 1
	} else if cosAngle < -1 {
		cosAngle = -1
	}
	// acos(c) = atan2(sqrt(1 - c^2), c)
	sinAngle := trig.Sqrt(1 - cosAngle*cosAngle)
	angle := trig.Atan2(sinAngle, cosAngle)
	if ux*vy-uy*vx < 0 {
		angle = -angle
	}
	return angle
}

func absf(x float64) float64 {
	if x < 0 {
		return -x
	}
	return x
}
