package backends

import (
	"math"
	"testing"

	blas "github.com/adhithyan15/coding-adventures/code/packages/go/blas-library"
)

// =========================================================================
// Helpers
// =========================================================================

func approx(a, b, tol float32) bool {
	return float32(math.Abs(float64(a-b))) < tol
}

func approxSlice(a, b []float32, tol float32) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if !approx(a[i], b[i], tol) {
			return false
		}
	}
	return true
}

func newCPU() *CpuBlas { return &CpuBlas{} }

// =========================================================================
// LEVEL 1: Vector-Vector Operations
// =========================================================================

func TestCpuName(t *testing.T) {
	cpu := newCPU()
	if cpu.Name() != "cpu" {
		t.Errorf("expected 'cpu', got %q", cpu.Name())
	}
}

func TestCpuDeviceName(t *testing.T) {
	cpu := newCPU()
	if cpu.DeviceName() != "CPU (pure Go)" {
		t.Errorf("expected 'CPU (pure Go)', got %q", cpu.DeviceName())
	}
}

func TestSaxpy_Basic(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2, 3})
	y := blas.NewVector([]float32{4, 5, 6})
	r, err := cpu.Saxpy(2.0, x, y)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := []float32{6, 9, 12} // 2*1+4, 2*2+5, 2*3+6
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSaxpy_AlphaZero(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2, 3})
	y := blas.NewVector([]float32{4, 5, 6})
	r, _ := cpu.Saxpy(0.0, x, y)
	if !approxSlice(r.Data, y.Data, 1e-6) {
		t.Errorf("alpha=0 should return y, got %v", r.Data)
	}
}

func TestSaxpy_AlphaOne(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2, 3})
	y := blas.NewVector([]float32{4, 5, 6})
	r, _ := cpu.Saxpy(1.0, x, y)
	expected := []float32{5, 7, 9}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSaxpy_Negative(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2, 3})
	y := blas.NewVector([]float32{4, 5, 6})
	r, _ := cpu.Saxpy(-1.0, x, y)
	expected := []float32{3, 3, 3}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSaxpy_DimensionMismatch(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2})
	y := blas.NewVector([]float32{4, 5, 6})
	_, err := cpu.Saxpy(1.0, x, y)
	if err == nil {
		t.Fatal("expected dimension mismatch error")
	}
}

func TestSdot_Basic(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2, 3})
	y := blas.NewVector([]float32{4, 5, 6})
	r, err := cpu.Sdot(x, y)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := float32(32) // 1*4 + 2*5 + 3*6
	if !approx(r, expected, 1e-6) {
		t.Errorf("expected %f, got %f", expected, r)
	}
}

func TestSdot_Orthogonal(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 0})
	y := blas.NewVector([]float32{0, 1})
	r, _ := cpu.Sdot(x, y)
	if !approx(r, 0, 1e-6) {
		t.Errorf("orthogonal vectors should have dot=0, got %f", r)
	}
}

func TestSdot_DimensionMismatch(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2})
	y := blas.NewVector([]float32{4, 5, 6})
	_, err := cpu.Sdot(x, y)
	if err == nil {
		t.Fatal("expected dimension mismatch error")
	}
}

func TestSnrm2_Basic(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{3, 4})
	r := cpu.Snrm2(x)
	if !approx(r, 5.0, 1e-5) {
		t.Errorf("expected 5.0, got %f", r)
	}
}

func TestSnrm2_Unit(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 0, 0})
	r := cpu.Snrm2(x)
	if !approx(r, 1.0, 1e-6) {
		t.Errorf("unit vector norm should be 1.0, got %f", r)
	}
}

func TestSnrm2_Zero(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{0, 0, 0})
	r := cpu.Snrm2(x)
	if !approx(r, 0, 1e-6) {
		t.Errorf("zero vector norm should be 0, got %f", r)
	}
}

func TestSscal_Basic(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2, 3})
	r := cpu.Sscal(2.0, x)
	expected := []float32{2, 4, 6}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSscal_Zero(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2, 3})
	r := cpu.Sscal(0.0, x)
	expected := []float32{0, 0, 0}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected zeros, got %v", r.Data)
	}
}

func TestSasum_Basic(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, -2, 3, -4})
	r := cpu.Sasum(x)
	if !approx(r, 10.0, 1e-6) {
		t.Errorf("expected 10.0, got %f", r)
	}
}

func TestSasum_AllPositive(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2, 3})
	r := cpu.Sasum(x)
	if !approx(r, 6.0, 1e-6) {
		t.Errorf("expected 6.0, got %f", r)
	}
}

func TestIsamax_Basic(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, -5, 3})
	r := cpu.Isamax(x)
	if r != 1 {
		t.Errorf("expected index 1 (|{-5}|=5 is max), got %d", r)
	}
}

func TestIsamax_AllSame(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{2, 2, 2})
	r := cpu.Isamax(x)
	if r != 0 {
		t.Errorf("expected index 0 (first max), got %d", r)
	}
}

func TestIsamax_Empty(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{})
	r := cpu.Isamax(x)
	if r != 0 {
		t.Errorf("expected 0 for empty vector, got %d", r)
	}
}

func TestScopy_Basic(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2, 3})
	r := cpu.Scopy(x)
	if !approxSlice(r.Data, x.Data, 1e-6) {
		t.Errorf("copy should match original")
	}
	// Verify deep copy
	r.Data[0] = 999
	if x.Data[0] == 999 {
		t.Error("modifying copy should not affect original")
	}
}

func TestSswap_Basic(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2, 3})
	y := blas.NewVector([]float32{4, 5, 6})
	newX, newY, err := cpu.Sswap(x, y)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !approxSlice(newX.Data, []float32{4, 5, 6}, 1e-6) {
		t.Errorf("newX should have y's data, got %v", newX.Data)
	}
	if !approxSlice(newY.Data, []float32{1, 2, 3}, 1e-6) {
		t.Errorf("newY should have x's data, got %v", newY.Data)
	}
}

func TestSswap_DimensionMismatch(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2})
	y := blas.NewVector([]float32{4, 5, 6})
	_, _, err := cpu.Sswap(x, y)
	if err == nil {
		t.Fatal("expected dimension mismatch error")
	}
}

// =========================================================================
// LEVEL 2: Matrix-Vector Operations
// =========================================================================

func TestSgemv_NoTrans(t *testing.T) {
	cpu := newCPU()
	// A = [[1, 2], [3, 4]], x = [1, 1], y = [0, 0]
	a := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	x := blas.NewVector([]float32{1, 1})
	y := blas.NewVector([]float32{0, 0})
	r, err := cpu.Sgemv(blas.NoTrans, 1.0, a, x, 0.0, y)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := []float32{3, 7} // [1+2, 3+4]
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSgemv_Trans(t *testing.T) {
	cpu := newCPU()
	// A = [[1, 2], [3, 4]], transposed effectively [[1, 3], [2, 4]]
	a := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	x := blas.NewVector([]float32{1, 1})
	y := blas.NewVector([]float32{0, 0})
	r, err := cpu.Sgemv(blas.Trans, 1.0, a, x, 0.0, y)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := []float32{4, 6} // [1+3, 2+4]
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSgemv_AlphaBeta(t *testing.T) {
	cpu := newCPU()
	a := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	x := blas.NewVector([]float32{1, 1})
	y := blas.NewVector([]float32{10, 20})
	r, _ := cpu.Sgemv(blas.NoTrans, 2.0, a, x, 3.0, y)
	// alpha * A*x = 2*[3, 7] = [6, 14]
	// beta * y = 3*[10, 20] = [30, 60]
	// result = [36, 74]
	expected := []float32{36, 74}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSgemv_DimMismatch(t *testing.T) {
	cpu := newCPU()
	a := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	x := blas.NewVector([]float32{1, 2, 3})
	y := blas.NewVector([]float32{0, 0})
	_, err := cpu.Sgemv(blas.NoTrans, 1.0, a, x, 0.0, y)
	if err == nil {
		t.Fatal("expected dimension mismatch error")
	}
}

func TestSgemv_YDimMismatch(t *testing.T) {
	cpu := newCPU()
	a := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	x := blas.NewVector([]float32{1, 1})
	y := blas.NewVector([]float32{0, 0, 0})
	_, err := cpu.Sgemv(blas.NoTrans, 1.0, a, x, 0.0, y)
	if err == nil {
		t.Fatal("expected y dimension mismatch error")
	}
}

func TestSger_Basic(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2})
	y := blas.NewVector([]float32{3, 4, 5})
	a := blas.Zeros(2, 3)
	r, err := cpu.Sger(1.0, x, y, a)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := []float32{3, 4, 5, 6, 8, 10}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSger_WithAlpha(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2})
	y := blas.NewVector([]float32{3, 4})
	a := blas.MustMatrix([]float32{10, 20, 30, 40}, 2, 2)
	r, _ := cpu.Sger(2.0, x, y, a)
	// 2*[1,2]^T * [3,4] + [[10,20],[30,40]]
	// = 2*[[3,4],[6,8]] + [[10,20],[30,40]]
	// = [[16,28],[42,56]]
	expected := []float32{16, 28, 42, 56}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSger_DimMismatch(t *testing.T) {
	cpu := newCPU()
	x := blas.NewVector([]float32{1, 2, 3})
	y := blas.NewVector([]float32{3, 4})
	a := blas.Zeros(2, 2)
	_, err := cpu.Sger(1.0, x, y, a)
	if err == nil {
		t.Fatal("expected dimension mismatch error")
	}
}

// =========================================================================
// LEVEL 3: Matrix-Matrix Operations
// =========================================================================

func TestSgemm_Basic(t *testing.T) {
	cpu := newCPU()
	// A = [[1, 2], [3, 4]], B = [[5, 6], [7, 8]]
	a := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	b := blas.MustMatrix([]float32{5, 6, 7, 8}, 2, 2)
	c := blas.Zeros(2, 2)
	r, err := cpu.Sgemm(blas.NoTrans, blas.NoTrans, 1.0, a, b, 0.0, c)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := []float32{19, 22, 43, 50}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSgemm_TransA(t *testing.T) {
	cpu := newCPU()
	a := blas.MustMatrix([]float32{1, 3, 2, 4}, 2, 2) // A^T = [[1,2],[3,4]]
	b := blas.MustMatrix([]float32{5, 6, 7, 8}, 2, 2)
	c := blas.Zeros(2, 2)
	r, _ := cpu.Sgemm(blas.Trans, blas.NoTrans, 1.0, a, b, 0.0, c)
	expected := []float32{19, 22, 43, 50}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSgemm_TransB(t *testing.T) {
	cpu := newCPU()
	a := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	b := blas.MustMatrix([]float32{5, 7, 6, 8}, 2, 2) // B^T = [[5,6],[7,8]]
	c := blas.Zeros(2, 2)
	r, _ := cpu.Sgemm(blas.NoTrans, blas.Trans, 1.0, a, b, 0.0, c)
	expected := []float32{19, 22, 43, 50}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSgemm_AlphaBeta(t *testing.T) {
	cpu := newCPU()
	a := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2) // identity
	b := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	c := blas.MustMatrix([]float32{10, 10, 10, 10}, 2, 2)
	r, _ := cpu.Sgemm(blas.NoTrans, blas.NoTrans, 2.0, a, b, 3.0, c)
	// 2*I*B + 3*C = 2*B + 3*C = [[2+30, 4+30],[6+30, 8+30]] = [[32,34],[36,38]]
	expected := []float32{32, 34, 36, 38}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSgemm_NonSquare(t *testing.T) {
	cpu := newCPU()
	// A (2x3) * B (3x2) = C (2x2)
	a := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6}, 2, 3)
	b := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6}, 3, 2)
	c := blas.Zeros(2, 2)
	r, err := cpu.Sgemm(blas.NoTrans, blas.NoTrans, 1.0, a, b, 0.0, c)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := []float32{22, 28, 49, 64}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSgemm_DimMismatch(t *testing.T) {
	cpu := newCPU()
	a := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	b := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6}, 3, 2)
	c := blas.Zeros(2, 2)
	_, err := cpu.Sgemm(blas.NoTrans, blas.NoTrans, 1.0, a, b, 0.0, c)
	if err == nil {
		t.Fatal("expected dimension mismatch error")
	}
}

func TestSgemm_CDimMismatch(t *testing.T) {
	cpu := newCPU()
	a := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	b := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	c := blas.Zeros(3, 3) // Wrong shape
	_, err := cpu.Sgemm(blas.NoTrans, blas.NoTrans, 1.0, a, b, 0.0, c)
	if err == nil {
		t.Fatal("expected C dimension mismatch error")
	}
}

func TestSsymm_Left(t *testing.T) {
	cpu := newCPU()
	// A = [[1, 2], [2, 3]] (symmetric), B = [[1, 0], [0, 1]]
	a := blas.MustMatrix([]float32{1, 2, 2, 3}, 2, 2)
	b := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	c := blas.Zeros(2, 2)
	r, err := cpu.Ssymm(blas.Left, 1.0, a, b, 0.0, c)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// A * I = A
	expected := []float32{1, 2, 2, 3}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSsymm_Right(t *testing.T) {
	cpu := newCPU()
	a := blas.MustMatrix([]float32{1, 2, 2, 3}, 2, 2)
	b := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	c := blas.Zeros(2, 2)
	r, _ := cpu.Ssymm(blas.Right, 1.0, a, b, 0.0, c)
	expected := []float32{1, 2, 2, 3}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestSsymm_NotSquare(t *testing.T) {
	cpu := newCPU()
	a := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6}, 2, 3)
	b := blas.Zeros(2, 2)
	c := blas.Zeros(2, 2)
	_, err := cpu.Ssymm(blas.Left, 1.0, a, b, 0.0, c)
	if err == nil {
		t.Fatal("expected error for non-square A")
	}
}

func TestSgemmBatched_Basic(t *testing.T) {
	cpu := newCPU()
	a := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	b := blas.MustMatrix([]float32{2, 3, 4, 5}, 2, 2)
	c := blas.Zeros(2, 2)
	results, err := cpu.SgemmBatched(blas.NoTrans, blas.NoTrans, 1.0,
		[]blas.Matrix{a}, []blas.Matrix{b}, 0.0, []blas.Matrix{c})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(results))
	}
	expected := []float32{2, 3, 4, 5}
	if !approxSlice(results[0].Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, results[0].Data)
	}
}

func TestSgemmBatched_Multiple(t *testing.T) {
	cpu := newCPU()
	id := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	b1 := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	b2 := blas.MustMatrix([]float32{5, 6, 7, 8}, 2, 2)
	z := blas.Zeros(2, 2)
	results, err := cpu.SgemmBatched(blas.NoTrans, blas.NoTrans, 1.0,
		[]blas.Matrix{id, id}, []blas.Matrix{b1, b2}, 0.0, []blas.Matrix{z, z})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(results) != 2 {
		t.Fatalf("expected 2 results, got %d", len(results))
	}
}

func TestSgemmBatched_SizeMismatch(t *testing.T) {
	cpu := newCPU()
	a := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	b := blas.MustMatrix([]float32{2, 3, 4, 5}, 2, 2)
	_, err := cpu.SgemmBatched(blas.NoTrans, blas.NoTrans, 1.0,
		[]blas.Matrix{a, a}, []blas.Matrix{b}, 0.0, []blas.Matrix{blas.Zeros(2, 2)})
	if err == nil {
		t.Fatal("expected batch size mismatch error")
	}
}

// =========================================================================
// ML EXTENSIONS: Activation Functions
// =========================================================================

func TestRelu_Basic(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{-2, -1, 0, 1, 2, 3}, 2, 3)
	r := cpu.Relu(x)
	expected := []float32{0, 0, 0, 1, 2, 3}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestRelu_AllNegative(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{-5, -3, -1, -0.5}, 2, 2)
	r := cpu.Relu(x)
	for _, v := range r.Data {
		if v != 0 {
			t.Errorf("all outputs should be 0, got %f", v)
		}
	}
}

func TestRelu_AllPositive(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	r := cpu.Relu(x)
	if !approxSlice(r.Data, x.Data, 1e-6) {
		t.Errorf("positive inputs should pass through")
	}
}

func TestGelu_ZeroInput(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{0}, 1, 1)
	r := cpu.Gelu(x)
	if !approx(r.Data[0], 0, 1e-4) {
		t.Errorf("GELU(0) should be ~0, got %f", r.Data[0])
	}
}

func TestGelu_PositiveInput(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{1.0}, 1, 1)
	r := cpu.Gelu(x)
	// GELU(1.0) ~ 0.8412
	if !approx(r.Data[0], 0.8412, 0.01) {
		t.Errorf("GELU(1.0) should be ~0.8412, got %f", r.Data[0])
	}
}

func TestSigmoid_Zero(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{0}, 1, 1)
	r := cpu.Sigmoid(x)
	if !approx(r.Data[0], 0.5, 1e-6) {
		t.Errorf("sigmoid(0) should be 0.5, got %f", r.Data[0])
	}
}

func TestSigmoid_Positive(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{100}, 1, 1)
	r := cpu.Sigmoid(x)
	if !approx(r.Data[0], 1.0, 1e-4) {
		t.Errorf("sigmoid(100) should be ~1.0, got %f", r.Data[0])
	}
}

func TestSigmoid_Negative(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{-100}, 1, 1)
	r := cpu.Sigmoid(x)
	if !approx(r.Data[0], 0.0, 1e-4) {
		t.Errorf("sigmoid(-100) should be ~0.0, got %f", r.Data[0])
	}
}

func TestTanhActivation_Zero(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{0}, 1, 1)
	r := cpu.TanhActivation(x)
	if !approx(r.Data[0], 0, 1e-6) {
		t.Errorf("tanh(0) should be 0, got %f", r.Data[0])
	}
}

func TestTanhActivation_LargePositive(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{100}, 1, 1)
	r := cpu.TanhActivation(x)
	if !approx(r.Data[0], 1.0, 1e-4) {
		t.Errorf("tanh(100) should be ~1.0, got %f", r.Data[0])
	}
}

// =========================================================================
// ML EXTENSIONS: Softmax
// =========================================================================

func TestSoftmax_Row(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{1, 2, 3, 1, 2, 3}, 2, 3)
	r := cpu.Softmax(x, -1)
	// Each row should sum to 1
	for i := 0; i < 2; i++ {
		var sum float32
		for j := 0; j < 3; j++ {
			sum += r.Data[i*3+j]
		}
		if !approx(sum, 1.0, 1e-5) {
			t.Errorf("row %d sum should be 1.0, got %f", i, sum)
		}
	}
}

func TestSoftmax_Column(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	r := cpu.Softmax(x, 0)
	// Each column should sum to 1
	for j := 0; j < 2; j++ {
		var sum float32
		for i := 0; i < 2; i++ {
			sum += r.Data[i*2+j]
		}
		if !approx(sum, 1.0, 1e-5) {
			t.Errorf("col %d sum should be 1.0, got %f", j, sum)
		}
	}
}

func TestSoftmax_AllSame(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{1, 1, 1}, 1, 3)
	r := cpu.Softmax(x, -1)
	for _, v := range r.Data {
		if !approx(v, 1.0/3.0, 1e-5) {
			t.Errorf("uniform input should give 1/3, got %f", v)
		}
	}
}

// =========================================================================
// ML EXTENSIONS: Normalization
// =========================================================================

func TestLayerNorm_Basic(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6}, 2, 3)
	gamma := blas.NewVector([]float32{1, 1, 1})
	beta := blas.NewVector([]float32{0, 0, 0})
	r, err := cpu.LayerNorm(x, gamma, beta, 1e-5)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Each row should have mean ~0 and std ~1
	for i := 0; i < 2; i++ {
		var mean float32
		for j := 0; j < 3; j++ {
			mean += r.Data[i*3+j]
		}
		mean /= 3.0
		if !approx(mean, 0, 1e-4) {
			t.Errorf("row %d mean should be ~0, got %f", i, mean)
		}
	}
}

func TestLayerNorm_GammaMismatch(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	gamma := blas.NewVector([]float32{1, 1, 1}) // Wrong size
	beta := blas.NewVector([]float32{0, 0})
	_, err := cpu.LayerNorm(x, gamma, beta, 1e-5)
	if err == nil {
		t.Fatal("expected gamma dimension mismatch error")
	}
}

func TestLayerNorm_BetaMismatch(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	gamma := blas.NewVector([]float32{1, 1})
	beta := blas.NewVector([]float32{0, 0, 0}) // Wrong size
	_, err := cpu.LayerNorm(x, gamma, beta, 1e-5)
	if err == nil {
		t.Fatal("expected beta dimension mismatch error")
	}
}

func TestBatchNorm_Training(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6}, 3, 2)
	gamma := blas.NewVector([]float32{1, 1})
	beta := blas.NewVector([]float32{0, 0})
	rm := blas.NewVector([]float32{0, 0})
	rv := blas.NewVector([]float32{1, 1})
	r, err := cpu.BatchNorm(x, gamma, beta, rm, rv, 1e-5, true)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Each column (feature) should have mean ~0
	for j := 0; j < 2; j++ {
		var mean float32
		for i := 0; i < 3; i++ {
			mean += r.Data[i*2+j]
		}
		mean /= 3.0
		if !approx(mean, 0, 1e-4) {
			t.Errorf("feature %d mean should be ~0, got %f", j, mean)
		}
	}
}

func TestBatchNorm_Inference(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	gamma := blas.NewVector([]float32{1, 1})
	beta := blas.NewVector([]float32{0, 0})
	rm := blas.NewVector([]float32{2, 3})
	rv := blas.NewVector([]float32{1, 1})
	r, err := cpu.BatchNorm(x, gamma, beta, rm, rv, 1e-5, false)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Should use running stats: (x - mean) / sqrt(var + eps)
	if r.Rows != 2 || r.Cols != 2 {
		t.Errorf("expected 2x2 output, got %dx%d", r.Rows, r.Cols)
	}
}

func TestBatchNorm_GammaMismatch(t *testing.T) {
	cpu := newCPU()
	x := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	gamma := blas.NewVector([]float32{1, 1, 1})
	beta := blas.NewVector([]float32{0, 0})
	rm := blas.NewVector([]float32{0, 0})
	rv := blas.NewVector([]float32{1, 1})
	_, err := cpu.BatchNorm(x, gamma, beta, rm, rv, 1e-5, true)
	if err == nil {
		t.Fatal("expected gamma dimension mismatch")
	}
}

// =========================================================================
// ML EXTENSIONS: Convolution
// =========================================================================

func TestConv2d_NoStridePadding(t *testing.T) {
	cpu := newCPU()
	input := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6, 7, 8, 9}, 3, 3)
	weight := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	r, err := cpu.Conv2d(input, weight, nil, 1, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if r.Rows != 2 || r.Cols != 2 {
		t.Errorf("expected 2x2 output, got %dx%d", r.Rows, r.Cols)
	}
	// kernel [[1,0],[0,1]] picks diagonal elements
	expected := []float32{1 + 5, 2 + 6, 4 + 8, 5 + 9}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestConv2d_WithPadding(t *testing.T) {
	cpu := newCPU()
	input := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	weight := blas.MustMatrix([]float32{1, 1, 1, 1}, 2, 2)
	r, err := cpu.Conv2d(input, weight, nil, 1, 1)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// With padding=1, output is 3x3
	if r.Rows != 3 || r.Cols != 3 {
		t.Errorf("expected 3x3 output, got %dx%d", r.Rows, r.Cols)
	}
}

func TestConv2d_WithBias(t *testing.T) {
	cpu := newCPU()
	input := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	weight := blas.MustMatrix([]float32{1}, 1, 1)
	bias := blas.NewVector([]float32{10})
	r, err := cpu.Conv2d(input, weight, &bias, 1, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := []float32{11, 12, 13, 14}
	if !approxSlice(r.Data, expected, 1e-6) {
		t.Errorf("expected %v, got %v", expected, r.Data)
	}
}

func TestConv2d_Stride2(t *testing.T) {
	cpu := newCPU()
	input := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16}, 4, 4)
	weight := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	r, err := cpu.Conv2d(input, weight, nil, 2, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if r.Rows != 2 || r.Cols != 2 {
		t.Errorf("expected 2x2 output, got %dx%d", r.Rows, r.Cols)
	}
}

// =========================================================================
// ML EXTENSIONS: Attention
// =========================================================================

func TestAttention_Basic(t *testing.T) {
	cpu := newCPU()
	// Q, K, V: 2x2 matrices
	q := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	k := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	v := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	r, err := cpu.Attention(q, k, v, nil, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if r.Rows != 2 || r.Cols != 2 {
		t.Errorf("expected 2x2 output, got %dx%d", r.Rows, r.Cols)
	}
}

func TestAttention_WithScale(t *testing.T) {
	cpu := newCPU()
	q := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	k := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	v := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	scale := float32(1.0)
	r, err := cpu.Attention(q, k, v, nil, &scale)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if r.Rows != 2 || r.Cols != 2 {
		t.Errorf("expected 2x2, got %dx%d", r.Rows, r.Cols)
	}
}

func TestAttention_WithMask(t *testing.T) {
	cpu := newCPU()
	q := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	k := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	v := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
	// Causal mask: -inf for future positions
	negInf := float32(-1e9)
	mask := blas.MustMatrix([]float32{0, negInf, 0, 0}, 2, 2)
	r, err := cpu.Attention(q, k, v, &mask, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if r.Rows != 2 || r.Cols != 2 {
		t.Errorf("expected 2x2, got %dx%d", r.Rows, r.Cols)
	}
}

// =========================================================================
// Helper function tests
// =========================================================================

func TestGetElement_NoTrans(t *testing.T) {
	m := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	if getElement(m, 0, 0, blas.NoTrans) != 1 {
		t.Errorf("A[0][0] should be 1")
	}
	if getElement(m, 0, 1, blas.NoTrans) != 2 {
		t.Errorf("A[0][1] should be 2")
	}
	if getElement(m, 1, 0, blas.NoTrans) != 3 {
		t.Errorf("A[1][0] should be 3")
	}
	if getElement(m, 1, 1, blas.NoTrans) != 4 {
		t.Errorf("A[1][1] should be 4")
	}
}

func TestGetElement_Trans(t *testing.T) {
	m := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	// Transposed: [[1, 3], [2, 4]]
	if getElement(m, 0, 0, blas.Trans) != 1 {
		t.Errorf("A^T[0][0] should be 1")
	}
	if getElement(m, 0, 1, blas.Trans) != 3 {
		t.Errorf("A^T[0][1] should be 3")
	}
	if getElement(m, 1, 0, blas.Trans) != 2 {
		t.Errorf("A^T[1][0] should be 2")
	}
	if getElement(m, 1, 1, blas.Trans) != 4 {
		t.Errorf("A^T[1][1] should be 4")
	}
}

func TestEffectiveShape_NoTrans(t *testing.T) {
	m := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6}, 2, 3)
	r, c := effectiveShape(m, blas.NoTrans)
	if r != 2 || c != 3 {
		t.Errorf("expected (2, 3), got (%d, %d)", r, c)
	}
}

func TestEffectiveShape_Trans(t *testing.T) {
	m := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6}, 2, 3)
	r, c := effectiveShape(m, blas.Trans)
	if r != 3 || c != 2 {
		t.Errorf("expected (3, 2), got (%d, %d)", r, c)
	}
}
