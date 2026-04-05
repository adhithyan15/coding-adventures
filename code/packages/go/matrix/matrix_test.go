package matrix

import (
	"math"
	"reflect"
	"testing"
)

// ======================================================================
// Existing base tests
// ======================================================================

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

// ======================================================================
// Element access tests
// ======================================================================

func TestGet(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	v, err := M.Get(0, 0)
	if err != nil || v != 1.0 {
		t.Errorf("Get(0,0) = %v, err=%v; want 1.0", v, err)
	}
	v, err = M.Get(1, 1)
	if err != nil || v != 4.0 {
		t.Errorf("Get(1,1) = %v, err=%v; want 4.0", v, err)
	}
}

func TestGetOutOfBounds(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	_, err := M.Get(2, 0)
	if err == nil {
		t.Errorf("Expected out-of-bounds error")
	}
	_, err = M.Get(0, 2)
	if err == nil {
		t.Errorf("Expected out-of-bounds error for col")
	}
}

func TestSet(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	N, err := M.Set(0, 0, 99)
	if err != nil {
		t.Fatalf("Set failed: %v", err)
	}
	// Original unchanged
	v, _ := M.Get(0, 0)
	if v != 1.0 {
		t.Errorf("Original mutated: got %v, want 1.0", v)
	}
	v, _ = N.Get(0, 0)
	if v != 99.0 {
		t.Errorf("Set value wrong: got %v, want 99.0", v)
	}
}

func TestSetOutOfBounds(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	_, err := M.Set(2, 0, 5)
	if err == nil {
		t.Errorf("Expected out-of-bounds error")
	}
}

// ======================================================================
// Reduction tests
// ======================================================================

func TestSum(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	if M.Sum() != 10.0 {
		t.Errorf("Sum = %v; want 10.0", M.Sum())
	}
}

func TestSumRows(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	result := M.SumRows()
	if !reflect.DeepEqual(result.Data, [][]float64{{3}, {7}}) {
		t.Errorf("SumRows = %v; want [[3],[7]]", result.Data)
	}
}

func TestSumCols(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	result := M.SumCols()
	if !reflect.DeepEqual(result.Data, [][]float64{{4, 6}}) {
		t.Errorf("SumCols = %v; want [[4,6]]", result.Data)
	}
}

func TestMean(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	mean, err := M.Mean()
	if err != nil || mean != 2.5 {
		t.Errorf("Mean = %v, err=%v; want 2.5", mean, err)
	}
}

func TestMinMax(t *testing.T) {
	M := New2D([][]float64{{3, 1}, {4, 2}})
	minV, _ := M.Min()
	maxV, _ := M.Max()
	if minV != 1.0 {
		t.Errorf("Min = %v; want 1.0", minV)
	}
	if maxV != 4.0 {
		t.Errorf("Max = %v; want 4.0", maxV)
	}
}

func TestMinMaxNegative(t *testing.T) {
	M := New2D([][]float64{{-5, 3}, {0, -1}})
	minV, _ := M.Min()
	maxV, _ := M.Max()
	if minV != -5.0 {
		t.Errorf("Min = %v; want -5.0", minV)
	}
	if maxV != 3.0 {
		t.Errorf("Max = %v; want 3.0", maxV)
	}
}

func TestArgmin(t *testing.T) {
	M := New2D([][]float64{{3, 1}, {4, 2}})
	r, c, err := M.Argmin()
	if err != nil || r != 0 || c != 1 {
		t.Errorf("Argmin = (%d,%d), err=%v; want (0,1)", r, c, err)
	}
}

func TestArgmax(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	r, c, err := M.Argmax()
	if err != nil || r != 1 || c != 1 {
		t.Errorf("Argmax = (%d,%d), err=%v; want (1,1)", r, c, err)
	}
}

func TestArgmaxFirstOccurrence(t *testing.T) {
	M := New2D([][]float64{{4, 2}, {3, 4}})
	r, c, _ := M.Argmax()
	if r != 0 || c != 0 {
		t.Errorf("Argmax first occurrence = (%d,%d); want (0,0)", r, c)
	}
}

// ======================================================================
// Element-wise math tests
// ======================================================================

func TestMap(t *testing.T) {
	M := New2D([][]float64{{1, 4}, {9, 16}})
	result := M.Map(math.Sqrt)
	if !reflect.DeepEqual(result.Data, [][]float64{{1, 2}, {3, 4}}) {
		t.Errorf("Map(sqrt) = %v; want [[1,2],[3,4]]", result.Data)
	}
}

func TestSqrt(t *testing.T) {
	M := New2D([][]float64{{1, 4}, {9, 16}})
	if !reflect.DeepEqual(M.Sqrt().Data, [][]float64{{1, 2}, {3, 4}}) {
		t.Errorf("Sqrt failed")
	}
}

func TestAbs(t *testing.T) {
	M := New2D([][]float64{{-1, 2}, {-3, 4}})
	if !reflect.DeepEqual(M.Abs().Data, [][]float64{{1, 2}, {3, 4}}) {
		t.Errorf("Abs failed")
	}
}

func TestPow(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	result := M.Pow(2)
	if !reflect.DeepEqual(result.Data, [][]float64{{1, 4}, {9, 16}}) {
		t.Errorf("Pow(2) = %v; want [[1,4],[9,16]]", result.Data)
	}
}

func TestSqrtPowRoundtrip(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	if !M.Close(M.Sqrt().Pow(2.0), 1e-9) {
		t.Errorf("M.Close(M.Sqrt().Pow(2)) should be true")
	}
}

// ======================================================================
// Shape operation tests
// ======================================================================

func TestFlatten(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	flat := M.Flatten()
	if !reflect.DeepEqual(flat.Data, [][]float64{{1, 2, 3, 4}}) {
		t.Errorf("Flatten = %v; want [[1,2,3,4]]", flat.Data)
	}
	if flat.Rows != 1 || flat.Cols != 4 {
		t.Errorf("Flatten shape = %dx%d; want 1x4", flat.Rows, flat.Cols)
	}
}

func TestReshape(t *testing.T) {
	M := New1D([]float64{1, 2, 3, 4, 5, 6})
	result, err := M.Reshape(2, 3)
	if err != nil {
		t.Fatalf("Reshape failed: %v", err)
	}
	if !reflect.DeepEqual(result.Data, [][]float64{{1, 2, 3}, {4, 5, 6}}) {
		t.Errorf("Reshape = %v; want [[1,2,3],[4,5,6]]", result.Data)
	}
}

func TestReshapeInvalid(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	_, err := M.Reshape(3, 3)
	if err == nil {
		t.Errorf("Expected reshape error for incompatible dimensions")
	}
}

func TestFlattenReshapeRoundtrip(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	result, _ := M.Flatten().Reshape(M.Rows, M.Cols)
	if !result.Equals(M) {
		t.Errorf("Flatten().Reshape() should round-trip to original")
	}
}

func TestRow(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	r0, _ := M.Row(0)
	r1, _ := M.Row(1)
	if !reflect.DeepEqual(r0.Data, [][]float64{{1, 2}}) {
		t.Errorf("Row(0) = %v; want [[1,2]]", r0.Data)
	}
	if !reflect.DeepEqual(r1.Data, [][]float64{{3, 4}}) {
		t.Errorf("Row(1) = %v; want [[3,4]]", r1.Data)
	}
}

func TestRowOutOfBounds(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	_, err := M.Row(2)
	if err == nil {
		t.Errorf("Expected row out-of-bounds error")
	}
}

func TestCol(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	c0, _ := M.Col(0)
	c1, _ := M.Col(1)
	if !reflect.DeepEqual(c0.Data, [][]float64{{1}, {3}}) {
		t.Errorf("Col(0) = %v; want [[1],[3]]", c0.Data)
	}
	if !reflect.DeepEqual(c1.Data, [][]float64{{2}, {4}}) {
		t.Errorf("Col(1) = %v; want [[2],[4]]", c1.Data)
	}
}

func TestColOutOfBounds(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	_, err := M.Col(2)
	if err == nil {
		t.Errorf("Expected col out-of-bounds error")
	}
}

func TestSlice(t *testing.T) {
	M := New2D([][]float64{{1, 2, 3}, {4, 5, 6}, {7, 8, 9}})
	result, err := M.Slice(0, 2, 1, 3)
	if err != nil {
		t.Fatalf("Slice failed: %v", err)
	}
	if !reflect.DeepEqual(result.Data, [][]float64{{2, 3}, {5, 6}}) {
		t.Errorf("Slice = %v; want [[2,3],[5,6]]", result.Data)
	}
}

func TestSliceSingleColumn(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	result, _ := M.Slice(0, 2, 0, 1)
	if !reflect.DeepEqual(result.Data, [][]float64{{1}, {3}}) {
		t.Errorf("Slice single col = %v; want [[1],[3]]", result.Data)
	}
}

func TestSliceOutOfBounds(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	_, err := M.Slice(0, 3, 0, 1)
	if err == nil {
		t.Errorf("Expected slice out-of-bounds error")
	}
}

// ======================================================================
// Equality tests
// ======================================================================

func TestEquals(t *testing.T) {
	A := New2D([][]float64{{1, 2}, {3, 4}})
	B := New2D([][]float64{{1, 2}, {3, 4}})
	C := New2D([][]float64{{1, 2}, {3, 5}})
	if !A.Equals(B) {
		t.Errorf("A.Equals(B) should be true")
	}
	if A.Equals(C) {
		t.Errorf("A.Equals(C) should be false")
	}
}

func TestEqualsShapeMismatch(t *testing.T) {
	A := New1D([]float64{1, 2})
	B := New2D([][]float64{{1}, {2}})
	if A.Equals(B) {
		t.Errorf("Different shapes should not be equal")
	}
}

func TestClose(t *testing.T) {
	A := New1D([]float64{1, 2})
	B := New1D([]float64{1 + 1e-10, 2 - 1e-10})
	if !A.Close(B, 1e-9) {
		t.Errorf("A.Close(B) should be true")
	}
}

func TestCloseFails(t *testing.T) {
	A := New1D([]float64{1, 2})
	B := New1D([]float64{1.1, 2})
	if A.Close(B, 1e-9) {
		t.Errorf("A.Close(B) should be false for tolerance 1e-9")
	}
}

// ======================================================================
// Factory method tests
// ======================================================================

func TestIdentity(t *testing.T) {
	I := Identity(3)
	expected := [][]float64{{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}
	if !reflect.DeepEqual(I.Data, expected) {
		t.Errorf("Identity(3) = %v; want %v", I.Data, expected)
	}
}

func TestIdentityDot(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}, {5, 6}})
	I := Identity(3)
	result, _ := I.Dot(M)
	if !result.Equals(M) {
		t.Errorf("Identity(3).Dot(M) should equal M")
	}
}

func TestFromDiagonal(t *testing.T) {
	D := FromDiagonal([]float64{2, 3})
	expected := [][]float64{{2, 0}, {0, 3}}
	if !reflect.DeepEqual(D.Data, expected) {
		t.Errorf("FromDiagonal = %v; want %v", D.Data, expected)
	}
}

func TestFromDiagonalSingle(t *testing.T) {
	D := FromDiagonal([]float64{5})
	if !reflect.DeepEqual(D.Data, [][]float64{{5}}) {
		t.Errorf("FromDiagonal([5]) = %v; want [[5]]", D.Data)
	}
}

// ======================================================================
// Parity test vectors (cross-language consistency)
// ======================================================================

func TestParitySumMean(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	if M.Sum() != 10.0 {
		t.Errorf("Parity: sum = %v; want 10.0", M.Sum())
	}
	mean, _ := M.Mean()
	if mean != 2.5 {
		t.Errorf("Parity: mean = %v; want 2.5", mean)
	}
}

func TestParitySumRowsCols(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	if !reflect.DeepEqual(M.SumRows().Data, [][]float64{{3}, {7}}) {
		t.Errorf("Parity: SumRows failed")
	}
	if !reflect.DeepEqual(M.SumCols().Data, [][]float64{{4, 6}}) {
		t.Errorf("Parity: SumCols failed")
	}
}

func TestParityIdentityDot(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}, {5, 6}})
	result, _ := Identity(3).Dot(M)
	if !result.Equals(M) {
		t.Errorf("Parity: Identity(3).Dot(M) != M")
	}
}

func TestParityFlattenReshape(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	result, _ := M.Flatten().Reshape(M.Rows, M.Cols)
	if !result.Equals(M) {
		t.Errorf("Parity: Flatten().Reshape() roundtrip failed")
	}
}

func TestParityCloseSqrtPow(t *testing.T) {
	M := New2D([][]float64{{1, 2}, {3, 4}})
	if !M.Close(M.Sqrt().Pow(2.0), 1e-9) {
		t.Errorf("Parity: M.Close(M.Sqrt().Pow(2)) should be true")
	}
}
