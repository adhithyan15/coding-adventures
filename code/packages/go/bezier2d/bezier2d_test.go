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
	pts := PolylineQuad(straight, 0.1)
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
	pts := PolylineCubic(straight, 0.1)
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
