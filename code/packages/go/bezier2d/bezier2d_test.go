package bezier2d

import (
	"math"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/point2d"
)

const eps = 1e-9

func approxEq(a, b float64) bool { return math.Abs(a-b) < eps }
func ptEq(a, b point2d.Point) bool {
	return approxEq(a.X, b.X) && approxEq(a.Y, b.Y)
}

var q = QuadraticBezier{
	point2d.NewPoint(0, 0), point2d.NewPoint(1, 2), point2d.NewPoint(2, 0),
}

func TestQuadEndpoints(t *testing.T) {
	if !ptEq(EvalQuad(q, 0), q.P0) {
		t.Error("EvalQuad at t=0 should be P0")
	}
	if !ptEq(EvalQuad(q, 1), q.P2) {
		t.Error("EvalQuad at t=1 should be P2")
	}
}

func TestQuadMidpoint(t *testing.T) {
	m := EvalQuad(q, 0.5)
	if !approxEq(m.X, 1) || !approxEq(m.Y, 1) {
		t.Errorf("QuadMidpoint: got %v", m)
	}
}

func TestQuadDerivativeEndpoints(t *testing.T) {
	d0 := DerivQuad(q, 0)
	d1 := DerivQuad(q, 1)
	if !ptEq(d0, point2d.NewPoint(2, 4)) || !ptEq(d1, point2d.NewPoint(2, -4)) {
		t.Fatalf("unexpected quadratic derivatives: start=%v end=%v", d0, d1)
	}
}

func TestQuadSplitEndpoints(t *testing.T) {
	left, right := SplitQuad(q, 0.5)
	m := EvalQuad(q, 0.5)
	if !ptEq(left.P2, m) || !ptEq(right.P0, m) {
		t.Error("Split midpoints don't match")
	}
}

func TestQuadPolylineStraight(t *testing.T) {
	straight := QuadraticBezier{
		point2d.NewPoint(0, 0), point2d.NewPoint(1, 0), point2d.NewPoint(2, 0),
	}
	pts, err := PolylineQuad(straight, 0.1)
	if err != nil {
		t.Fatalf("PolylineQuad returned an unexpected error: %v", err)
	}
	if len(pts) != 2 {
		t.Errorf("Straight line should give 2 points, got %d", len(pts))
	}
}

func TestQuadBboxContainsEndpoints(t *testing.T) {
	bb := BboxQuad(q)
	if bb.X > 0 || bb.X+bb.Width < 2 {
		t.Errorf("BboxQuad doesn't contain endpoints: %v", bb)
	}
}

func TestQuadBboxIncludesInteriorExtrema(t *testing.T) {
	loop := QuadraticBezier{
		point2d.NewPoint(0, 0), point2d.NewPoint(4, 6), point2d.NewPoint(0, 0),
	}
	bb := BboxQuad(loop)
	if !approxEq(bb.Width, 2) || !approxEq(bb.Height, 3) {
		t.Fatalf("unexpected interior-extrema bounds: %v", bb)
	}
}

func TestQuadElevateEquivalent(t *testing.T) {
	c := ElevateQuad(q)
	for _, t_ := range []float64{0, 0.25, 0.5, 0.75, 1} {
		qp := EvalQuad(q, t_)
		cp := EvalCubic(c, t_)
		if !approxEq(qp.X, cp.X) || !approxEq(qp.Y, cp.Y) {
			t.Errorf("ElevateQuad mismatch at t=%v: %v vs %v", t_, qp, cp)
		}
	}
}

var c = CubicBezier{
	point2d.NewPoint(0, 0), point2d.NewPoint(1, 2),
	point2d.NewPoint(3, 2), point2d.NewPoint(4, 0),
}

func TestCubicEndpoints(t *testing.T) {
	if !ptEq(EvalCubic(c, 0), c.P0) {
		t.Error("EvalCubic at t=0 should be P0")
	}
	if !ptEq(EvalCubic(c, 1), c.P3) {
		t.Error("EvalCubic at t=1 should be P3")
	}
}

func TestCubicSymmetricMidpoint(t *testing.T) {
	m := EvalCubic(c, 0.5)
	if !approxEq(m.X, 2) {
		t.Errorf("Symmetric midpoint X should be 2, got %v", m.X)
	}
}

func TestCubicDerivativeEndpoints(t *testing.T) {
	d0 := DerivCubic(c, 0)
	d1 := DerivCubic(c, 1)
	if !ptEq(d0, point2d.NewPoint(3, 6)) || !ptEq(d1, point2d.NewPoint(3, -6)) {
		t.Fatalf("unexpected cubic derivatives: start=%v end=%v", d0, d1)
	}
}

func TestCubicSplitEndpoints(t *testing.T) {
	left, right := SplitCubic(c, 0.5)
	m := EvalCubic(c, 0.5)
	if !ptEq(left.P3, m) || !ptEq(right.P0, m) {
		t.Error("Split midpoints don't match")
	}
}

func TestCubicPolylineStraight(t *testing.T) {
	straight := CubicBezier{
		point2d.NewPoint(0, 0), point2d.NewPoint(1, 0),
		point2d.NewPoint(2, 0), point2d.NewPoint(3, 0),
	}
	pts, err := PolylineCubic(straight, 0.1)
	if err != nil {
		t.Fatalf("PolylineCubic returned an unexpected error: %v", err)
	}
	if len(pts) != 2 {
		t.Errorf("Straight line should give 2 points, got %d", len(pts))
	}
}

func TestCubicBboxContainsSamples(t *testing.T) {
	bb := BboxCubic(c)
	for i := 0; i <= 20; i++ {
		p := EvalCubic(c, float64(i)/20)
		if p.X < bb.X-1e-6 || p.X > bb.X+bb.Width+1e-6 {
			t.Errorf("X out of bbox at i=%d: %v", i, p)
		}
	}
}

func TestCubicBboxIncludesBothCoordinateExtrema(t *testing.T) {
	loop := CubicBezier{
		point2d.NewPoint(2, 0), point2d.NewPoint(-4, 4),
		point2d.NewPoint(4, -4), point2d.NewPoint(-2, 0),
	}
	bb := BboxCubic(loop)
	for i := 0; i <= 100; i++ {
		point := EvalCubic(loop, float64(i)/100)
		if point.X < bb.X-eps || point.X > bb.X+bb.Width+eps || point.Y < bb.Y-eps || point.Y > bb.Y+bb.Height+eps {
			t.Fatalf("sample %d escaped bounds %v: %v", i, bb, point)
		}
	}
}

func TestExtremaDegenerateAndNoRealRoots(t *testing.T) {
	if roots := extrema(0, 1, 3, 6); len(roots) != 0 {
		t.Fatalf("expected an out-of-range linear root, got %v", roots)
	}
	if roots := extrema(0, 1, 1.1, 2.1); len(roots) != 0 {
		t.Fatalf("expected no real roots, got %v", roots)
	}
}

func TestScaledSegmentDistanceDegenerateAndOverflow(t *testing.T) {
	origin := point2d.NewPoint(0, 0)
	if distance, ok := pointToSegmentDistance(origin, origin, origin); !ok || distance != 0 {
		t.Fatalf("degenerate zero segment: distance=%v ok=%v", distance, ok)
	}
	if _, ok := pointToSegmentDistance(point2d.NewPoint(math.MaxFloat64, math.MaxFloat64), origin, origin); ok {
		t.Fatal("overflowing derived distance should fail closed")
	}
}
