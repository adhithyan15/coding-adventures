package affine2d

import (
	"math"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/point2d"
	"github.com/adhithyan15/coding-adventures/code/packages/go/trig"
)

const eps = 1e-9

func approxEq(a, b float64) bool { return math.Abs(a-b) < eps }
func ptEq(a, b point2d.Point) bool {
	return approxEq(a.X, b.X) && approxEq(a.Y, b.Y)
}

func TestIdentityLeavePointUnchanged(t *testing.T) {
	p := point2d.NewPoint(3, 4)
	q := ApplyToPoint(Identity(), p)
	if !ptEq(q, p) {
		t.Errorf("Identity should leave point unchanged: got %v", q)
	}
}

func TestTranslate(t *testing.T) {
	q := ApplyToPoint(Translate(5, -3), point2d.NewPoint(1, 2))
	if !approxEq(q.X, 6) || !approxEq(q.Y, -1) {
		t.Errorf("Translate: got %v", q)
	}
}

func TestTranslateDoesNotMoveVector(t *testing.T) {
	v := point2d.NewPoint(1, 1)
	w := ApplyToVector(Translate(100, 200), v)
	if !ptEq(w, v) {
		t.Errorf("Translate should not move vectors: got %v", w)
	}
}

func TestRotate90(t *testing.T) {
	q := ApplyToPoint(Rotate(trig.PI/2), point2d.NewPoint(1, 0))
	if !approxEq(q.X, 0) || !approxEq(q.Y, 1) {
		t.Errorf("Rotate90: got %v", q)
	}
}

func TestRotate360IsIdentity(t *testing.T) {
	if !IsIdentity(Rotate(2 * trig.PI)) {
		t.Error("Rotate 360 should be identity")
	}
}

func TestScale(t *testing.T) {
	q := ApplyToPoint(Scale(2, 3), point2d.NewPoint(1, 1))
	if !approxEq(q.X, 2) || !approxEq(q.Y, 3) {
		t.Errorf("Scale: got %v", q)
	}
}

func TestDeterminantIdentity(t *testing.T) {
	if !approxEq(Determinant(Identity()), 1) {
		t.Error("Determinant of identity should be 1")
	}
}

func TestInvert(t *testing.T) {
	m := Translate(3, -7)
	inv, ok := Invert(m)
	if !ok {
		t.Fatal("Should be invertible")
	}
	composed := Multiply(m, inv)
	if !IsIdentity(composed) {
		t.Errorf("m * inv should be identity, got %v", composed)
	}
}

func TestInvertSingular(t *testing.T) {
	_, ok := Invert(Affine2D{})
	if ok {
		t.Error("Singular matrix should not be invertible")
	}
}

func TestIsTranslationOnly(t *testing.T) {
	if !IsTranslationOnly(Identity()) {
		t.Error("Identity should be translation-only")
	}
	if !IsTranslationOnly(Translate(5, 3)) {
		t.Error("Translate should be translation-only")
	}
	if IsTranslationOnly(Rotate(0.1)) {
		t.Error("Rotate should not be translation-only")
	}
}

func TestToArray(t *testing.T) {
	m := Affine2D{A: 1, B: 2, C: 3, D: 4, E: 5, F: 6}
	arr := ToArray(m)
	expected := [6]float64{1, 2, 3, 4, 5, 6}
	if arr != expected {
		t.Errorf("ToArray: got %v", arr)
	}
}

func TestMultiplyTwoRotations(t *testing.T) {
	r90 := Rotate(trig.PI / 2)
	r180 := Rotate(trig.PI)
	composed := Multiply(r90, r90)
	// Should equal 180-degree rotation.
	arr1 := ToArray(composed)
	arr2 := ToArray(r180)
	for i := range arr1 {
		if !approxEq(arr1[i], arr2[i]) {
			t.Errorf("Two 90° rotations != 180°: %v vs %v", arr1, arr2)
			break
		}
	}
}
