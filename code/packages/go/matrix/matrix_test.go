package matrix

import (
	"reflect"
	"testing"
)

func TestZeros(t *testing.T) {
	z := Zeros(2, 3)
	if z.Rows != 2 || z.Cols != 3 || z.Data[1][2] != 0.0 {
		t.Errorf("Zeros failed")
	}
}

func TestAddSubtract(t *testing.T) {
	A := New2D([][]float64{{1, 2}, {3, 4}})
	B := New2D([][]float64{{5, 6}, {7, 8}})
	C, _ := A.Add(B)
	if !reflect.DeepEqual(C.Data, [][]float64{{6, 8}, {10, 12}}) {
		t.Errorf("Add failed")
	}
	D, _ := B.Subtract(A)
	if !reflect.DeepEqual(D.Data, [][]float64{{4, 4}, {4, 4}}) {
		t.Errorf("Subtract failed")
	}
	
	E := A.AddScalar(2.0)
	if !reflect.DeepEqual(E.Data, [][]float64{{3, 4}, {5, 6}}) {
		t.Errorf("AddScalar failed")
	}

	_, err := A.Add(New2D([][]float64{{1}}))
	if err == nil {
		t.Errorf("Expected mismatch error")
	}
}

func TestScale(t *testing.T) {
	A := New2D([][]float64{{1, 2}, {3, 4}})
	C := A.Scale(2.0)
	if !reflect.DeepEqual(C.Data, [][]float64{{2, 4}, {6, 8}}) {
		t.Errorf("Scale failed")
	}
}

func TestTranspose(t *testing.T) {
	A := New2D([][]float64{{1, 2, 3}, {4, 5, 6}})
	C := A.Transpose()
	expected := [][]float64{{1, 4}, {2, 5}, {3, 6}}
	if !reflect.DeepEqual(C.Data, expected) {
		t.Errorf("Transpose failed")
	}
}

func TestDot(t *testing.T) {
	A := New2D([][]float64{{1, 2}, {3, 4}})
	B := New2D([][]float64{{5, 6}, {7, 8}})
	C, _ := A.Dot(B)
	if !reflect.DeepEqual(C.Data, [][]float64{{19, 22}, {43, 50}}) {
		t.Errorf("Dot failed")
	}

	D := New1D([]float64{1, 2, 3})
	E := New2D([][]float64{{4}, {5}, {6}})
	F, _ := D.Dot(E)
	if !reflect.DeepEqual(F.Data, [][]float64{{32}}) {
		t.Errorf("Dot 1x3 and 3x1 failed")
	}
}
