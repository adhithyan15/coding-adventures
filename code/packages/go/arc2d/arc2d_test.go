package arc2d

import (
	"math"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/point2d"
	"github.com/adhithyan15/coding-adventures/code/packages/go/trig"
)

const eps = 1e-6

func approxEq(a, b float64) bool { return math.Abs(a-b) < eps }
func ptApproxEq(a, b point2d.Point) bool {
	return approxEq(a.X, b.X) && approxEq(a.Y, b.Y)
}

// Unit circle arc from 0 to π/2
var unitArc = CenterArc{
	Center:     point2d.NewPoint(0, 0),
	Rx:         1, Ry: 1,
	StartAngle: 0, SweepAngle: trig.PI / 2,
	XRotation: 0,
}

func TestEvalArcAtZero(t *testing.T) {
	p := EvalArc(unitArc, 0)
	if !ptApproxEq(p, point2d.NewPoint(1, 0)) {
		t.Errorf("EvalArc at t=0: got %v", p)
	}
}

func TestEvalArcAtOne(t *testing.T) {
	p := EvalArc(unitArc, 1)
	if !ptApproxEq(p, point2d.NewPoint(0, 1)) {
		t.Errorf("EvalArc at t=1: got %v, want (0,1)", p)
	}
}

func TestEvalArcMidpoint(t *testing.T) {
	p := EvalArc(unitArc, 0.5)
	expected := 1.0 / math.Sqrt2
	if !approxEq(p.X, expected) || !approxEq(p.Y, expected) {
		t.Errorf("EvalArc at t=0.5: got %v", p)
	}
}

func TestEvalArcEllipse(t *testing.T) {
	// 2:1 ellipse, start at 0, sweep full circle
	a := CenterArc{
		Center: point2d.NewPoint(0, 0), Rx: 2, Ry: 1,
		StartAngle: 0, SweepAngle: trig.PI, XRotation: 0,
	}
	p := EvalArc(a, 0)
	if !ptApproxEq(p, point2d.NewPoint(2, 0)) {
		t.Errorf("Ellipse start: got %v", p)
	}
	p = EvalArc(a, 1)
	if !ptApproxEq(p, point2d.NewPoint(-2, 0)) {
		t.Errorf("Ellipse end: got %v", p)
	}
}

func TestTangentArcDirection(t *testing.T) {
	// At t=0, tangent of unit circle quarter-arc should point in +Y direction
	tan := TangentArc(unitArc, 0)
	if tan.X >= 0 || tan.Y <= 0 {
		// Actually for CCW quarter from (1,0) to (0,1) tangent at t=0 is (0, sweep*Ry) ~ positive Y, negative X slope
		// d/dt: dx/dt = sweep*(-Rx*sin(start)) = PI/2 * (-1 * 0) = 0
		// dy/dt = sweep*(Ry*cos(start)) = PI/2 * (1 * 1) = PI/2 > 0
		if tan.X > 1e-6 || tan.Y < 0 {
			t.Errorf("Tangent at t=0 should be (0, positive): got %v", tan)
		}
	}
}

func TestBboxArcUnitCircle(t *testing.T) {
	// Full circle bbox should contain the unit circle
	fullCircle := CenterArc{
		Center: point2d.NewPoint(0, 0), Rx: 1, Ry: 1,
		StartAngle: 0, SweepAngle: 2 * trig.PI, XRotation: 0,
	}
	bb := BboxArc(fullCircle)
	if bb.X > -1+0.01 || bb.X+bb.Width < 1-0.01 {
		t.Errorf("BboxArc full circle X range: %v", bb)
	}
	if bb.Y > -1+0.01 || bb.Y+bb.Height < 1-0.01 {
		t.Errorf("BboxArc full circle Y range: %v", bb)
	}
}

func TestToCubicBeziersReproducesArc(t *testing.T) {
	// Approximate a quarter circle and check that each Bezier's endpoints
	// lie on the circle.
	curves := ToCubicBeziers(unitArc)
	if len(curves) == 0 {
		t.Fatal("ToCubicBeziers returned no curves")
	}
	// First curve's P0 should be (1, 0)
	if !ptApproxEq(curves[0].P0, point2d.NewPoint(1, 0)) {
		t.Errorf("First curve P0: got %v", curves[0].P0)
	}
	// Last curve's P3 should be (0, 1)
	last := curves[len(curves)-1]
	if !ptApproxEq(last.P3, point2d.NewPoint(0, 1)) {
		t.Errorf("Last curve P3: got %v", last.P3)
	}
}

func TestToCenterArcDegenerate(t *testing.T) {
	// Same start and end
	_, ok := ToCenterArc(SvgArc{
		From: point2d.NewPoint(1, 1), To: point2d.NewPoint(1, 1),
		Rx: 1, Ry: 1,
	})
	if ok {
		t.Error("Same endpoints should be degenerate")
	}
	// Zero radius
	_, ok = ToCenterArc(SvgArc{
		From: point2d.NewPoint(0, 0), To: point2d.NewPoint(1, 0),
		Rx: 0, Ry: 1,
	})
	if ok {
		t.Error("Zero radius should be degenerate")
	}
}

func TestToCenterArcSemicircle(t *testing.T) {
	// SVG arc from (1,0) to (-1,0) with Rx=Ry=1, no rotation, sweep=true
	// This is a semicircle with center at (0,0)
	arc, ok := ToCenterArc(SvgArc{
		From: point2d.NewPoint(1, 0), To: point2d.NewPoint(-1, 0),
		Rx: 1, Ry: 1, XRotation: 0, LargeArc: false, Sweep: true,
	})
	if !ok {
		t.Fatal("ToCenterArc failed for semicircle")
	}
	// Center should be near (0,0)
	if !ptApproxEq(arc.Center, point2d.NewPoint(0, 0)) {
		t.Errorf("Semicircle center: got %v", arc.Center)
	}
	// Radius should be 1
	if !approxEq(arc.Rx, 1) || !approxEq(arc.Ry, 1) {
		t.Errorf("Semicircle radii: rx=%v ry=%v", arc.Rx, arc.Ry)
	}
	// The arc should evaluate to endpoints
	p0 := EvalArc(arc, 0)
	p1 := EvalArc(arc, 1)
	if !ptApproxEq(p0, point2d.NewPoint(1, 0)) {
		t.Errorf("SvgArc start point: got %v", p0)
	}
	if !ptApproxEq(p1, point2d.NewPoint(-1, 0)) {
		t.Errorf("SvgArc end point: got %v", p1)
	}
}
