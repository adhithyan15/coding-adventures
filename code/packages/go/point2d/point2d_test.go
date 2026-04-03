package point2d

import (
	"math"
	"testing"
)

const eps = 1e-9

func approxEq(a, b float64) bool { return math.Abs(a-b) < eps }
func ptEq(a, b Point) bool       { return approxEq(a.X, b.X) && approxEq(a.Y, b.Y) }

func TestOrigin(t *testing.T) {
	o := Origin()
	if o.X != 0 || o.Y != 0 {
		t.Errorf("Origin: got %v", o)
	}
}

func TestAdd(t *testing.T) {
	a := NewPoint(1, 2)
	b := NewPoint(3, 4)
	c := a.Add(b)
	if c.X != 4 || c.Y != 6 {
		t.Errorf("Add: got %v", c)
	}
}

func TestSubtract(t *testing.T) {
	a := NewPoint(5, 7)
	b := NewPoint(2, 3)
	c := a.Subtract(b)
	if c.X != 3 || c.Y != 4 {
		t.Errorf("Subtract: got %v", c)
	}
}

func TestScale(t *testing.T) {
	p := NewPoint(3, 4)
	q := p.Scale(2)
	if q.X != 6 || q.Y != 8 {
		t.Errorf("Scale: got %v", q)
	}
}

func TestNegate(t *testing.T) {
	p := NewPoint(3, -4)
	n := p.Negate()
	if n.X != -3 || n.Y != 4 {
		t.Errorf("Negate: got %v", n)
	}
}

func TestDotPerpendicular(t *testing.T) {
	x := NewPoint(1, 0)
	y := NewPoint(0, 1)
	if x.Dot(y) != 0 {
		t.Errorf("Dot perpendicular: got %v", x.Dot(y))
	}
}

func TestCrossCCW(t *testing.T) {
	x := NewPoint(1, 0)
	y := NewPoint(0, 1)
	if x.Cross(y) != 1 {
		t.Errorf("Cross CCW: got %v", x.Cross(y))
	}
}

func TestMagnitude345(t *testing.T) {
	p := NewPoint(3, 4)
	if !approxEq(p.Magnitude(), 5) {
		t.Errorf("Magnitude: got %v", p.Magnitude())
	}
}

func TestMagnitudeSquared(t *testing.T) {
	p := NewPoint(3, 4)
	if p.MagnitudeSquared() != 25 {
		t.Errorf("MagnitudeSquared: got %v", p.MagnitudeSquared())
	}
}

func TestNormalizeUnit(t *testing.T) {
	p := NewPoint(3, 4)
	n := p.Normalize()
	if !approxEq(n.X, 0.6) || !approxEq(n.Y, 0.8) {
		t.Errorf("Normalize: got %v", n)
	}
}

func TestNormalizeZero(t *testing.T) {
	n := Origin().Normalize()
	if n.X != 0 || n.Y != 0 {
		t.Errorf("Normalize zero: got %v", n)
	}
}

func TestDistance(t *testing.T) {
	a := Origin()
	b := NewPoint(3, 4)
	if !approxEq(a.Distance(b), 5) {
		t.Errorf("Distance: got %v", a.Distance(b))
	}
}

func TestLerpMidpoint(t *testing.T) {
	a := Origin()
	b := NewPoint(10, 10)
	m := a.Lerp(b, 0.5)
	if !approxEq(m.X, 5) || !approxEq(m.Y, 5) {
		t.Errorf("Lerp: got %v", m)
	}
}

func TestPerpendicular(t *testing.T) {
	p := NewPoint(1, 0)
	q := p.Perpendicular()
	if q.X != 0 || q.Y != 1 {
		t.Errorf("Perpendicular: got %v", q)
	}
}

func TestAngleRight(t *testing.T) {
	p := NewPoint(1, 0)
	if !approxEq(p.Angle(), 0) {
		t.Errorf("Angle right: got %v", p.Angle())
	}
}

func TestAngleUp(t *testing.T) {
	p := NewPoint(0, 1)
	if !approxEq(p.Angle(), math.Pi/2) {
		t.Errorf("Angle up: got %v", p.Angle())
	}
}

// ============================================================================
// Rect tests
// ============================================================================

func TestRectContainsInside(t *testing.T) {
	r := NewRect(0, 0, 10, 10)
	if !r.ContainsPoint(NewPoint(5, 5)) {
		t.Error("Should contain (5,5)")
	}
}

func TestRectContainsExclusive(t *testing.T) {
	r := NewRect(0, 0, 10, 10)
	if r.ContainsPoint(NewPoint(10, 5)) {
		t.Error("Right edge should be exclusive")
	}
}

func TestRectUnion(t *testing.T) {
	a := NewRect(0, 0, 5, 5)
	b := NewRect(10, 10, 5, 5)
	u := a.Union(b)
	if !approxEq(u.Width, 15) || !approxEq(u.Height, 15) {
		t.Errorf("Union: got %v", u)
	}
}

func TestRectIntersection(t *testing.T) {
	a := NewRect(0, 0, 10, 10)
	b := NewRect(5, 5, 10, 10)
	i, ok := a.Intersection(b)
	if !ok {
		t.Error("Expected intersection")
	}
	if !approxEq(i.X, 5) || !approxEq(i.Width, 5) {
		t.Errorf("Intersection: got %v", i)
	}
}

func TestRectNoIntersection(t *testing.T) {
	a := NewRect(0, 0, 5, 5)
	b := NewRect(10, 10, 5, 5)
	_, ok := a.Intersection(b)
	if ok {
		t.Error("Should have no intersection")
	}
}

func TestRectExpand(t *testing.T) {
	r := NewRect(1, 1, 8, 8)
	e := r.ExpandBy(1)
	if !approxEq(e.X, 0) || !approxEq(e.Width, 10) {
		t.Errorf("ExpandBy: got %v", e)
	}
}

func TestRectIsEmpty(t *testing.T) {
	if !ZeroRect().IsEmpty() {
		t.Error("Zero rect should be empty")
	}
	if NewRect(0, 0, 5, 5).IsEmpty() {
		t.Error("5x5 rect should not be empty")
	}
}

func TestRectMinMaxCenter(t *testing.T) {
	r := NewRect(2, 3, 8, 4)
	if !ptEq(r.MinPoint(), NewPoint(2, 3)) {
		t.Errorf("MinPoint: got %v", r.MinPoint())
	}
	if !ptEq(r.MaxPoint(), NewPoint(10, 7)) {
		t.Errorf("MaxPoint: got %v", r.MaxPoint())
	}
	if !ptEq(r.Center(), NewPoint(6, 5)) {
		t.Errorf("Center: got %v", r.Center())
	}
}
