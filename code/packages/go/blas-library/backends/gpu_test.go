package backends

import (
	"math"
	"testing"

	blas "github.com/adhithyan15/coding-adventures/code/packages/go/blas-library"
)

// =========================================================================
// GPU Backend Test Suite
// =========================================================================
//
// This file tests all seven BLAS backends through a shared test suite. Each
// backend must produce results identical to the CPU reference (within
// floating-point tolerance). We use subtests so that each backend is tested
// independently.
//
// The pattern:
//
//	for _, tc := range backends {
//	    t.Run(tc.name, func(t *testing.T) {
//	        // test body
//	    })
//	}

// backendCase holds a named BLAS backend for table-driven tests.
type backendCase struct {
	name    string
	backend blas.MlBlasBackend
}

// allBackends returns all seven backends for testing.
// Each backend is created fresh to avoid shared state.
func allBackends(t *testing.T) []backendCase {
	t.Helper()
	cases := []backendCase{
		{"cpu", &CpuBlas{}},
	}

	cuda, err := NewCudaBlas()
	if err != nil {
		t.Fatalf("failed to create CUDA backend: %v", err)
	}
	cases = append(cases, backendCase{"cuda", cuda})

	metal, err := NewMetalBlas()
	if err != nil {
		t.Fatalf("failed to create Metal backend: %v", err)
	}
	cases = append(cases, backendCase{"metal", metal})

	vk, err := NewVulkanBlas()
	if err != nil {
		t.Fatalf("failed to create Vulkan backend: %v", err)
	}
	cases = append(cases, backendCase{"vulkan", vk})

	cl, err := NewOpenClBlas()
	if err != nil {
		t.Fatalf("failed to create OpenCL backend: %v", err)
	}
	cases = append(cases, backendCase{"opencl", cl})

	wg, err := NewWebGpuBlas()
	if err != nil {
		t.Fatalf("failed to create WebGPU backend: %v", err)
	}
	cases = append(cases, backendCase{"webgpu", wg})

	gl, err := NewOpenGlBlas()
	if err != nil {
		t.Fatalf("failed to create OpenGL backend: %v", err)
	}
	cases = append(cases, backendCase{"opengl", gl})

	return cases
}

// =========================================================================
// Backend identity tests
// =========================================================================

func TestAllBackends_Name(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			if tc.backend.Name() != tc.name {
				t.Errorf("expected name %q, got %q", tc.name, tc.backend.Name())
			}
		})
	}
}

func TestAllBackends_DeviceName(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			name := tc.backend.DeviceName()
			if name == "" {
				t.Error("device name should not be empty")
			}
		})
	}
}

// =========================================================================
// LEVEL 1: All backends
// =========================================================================

func TestAllBackends_Saxpy(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.NewVector([]float32{1, 2, 3})
			y := blas.NewVector([]float32{4, 5, 6})
			r, err := tc.backend.Saxpy(2.0, x, y)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			expected := []float32{6, 9, 12}
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

func TestAllBackends_Saxpy_DimMismatch(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.NewVector([]float32{1, 2})
			y := blas.NewVector([]float32{4, 5, 6})
			_, err := tc.backend.Saxpy(1.0, x, y)
			if err == nil {
				t.Fatal("expected error for dimension mismatch")
			}
		})
	}
}

func TestAllBackends_Sdot(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.NewVector([]float32{1, 2, 3})
			y := blas.NewVector([]float32{4, 5, 6})
			r, err := tc.backend.Sdot(x, y)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if !approx(r, 32, 1e-4) {
				t.Errorf("expected 32, got %f", r)
			}
		})
	}
}

func TestAllBackends_Sdot_DimMismatch(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.NewVector([]float32{1})
			y := blas.NewVector([]float32{4, 5})
			_, err := tc.backend.Sdot(x, y)
			if err == nil {
				t.Fatal("expected error")
			}
		})
	}
}

func TestAllBackends_Snrm2(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.NewVector([]float32{3, 4})
			r := tc.backend.Snrm2(x)
			if !approx(r, 5.0, 1e-4) {
				t.Errorf("expected 5.0, got %f", r)
			}
		})
	}
}

func TestAllBackends_Sscal(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.NewVector([]float32{1, 2, 3})
			r := tc.backend.Sscal(3.0, x)
			expected := []float32{3, 6, 9}
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

func TestAllBackends_Sasum(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.NewVector([]float32{1, -2, 3, -4})
			r := tc.backend.Sasum(x)
			if !approx(r, 10.0, 1e-4) {
				t.Errorf("expected 10.0, got %f", r)
			}
		})
	}
}

func TestAllBackends_Isamax(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.NewVector([]float32{1, -5, 3})
			r := tc.backend.Isamax(x)
			if r != 1 {
				t.Errorf("expected index 1, got %d", r)
			}
		})
	}
}

func TestAllBackends_Scopy(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.NewVector([]float32{1, 2, 3})
			r := tc.backend.Scopy(x)
			if !approxSlice(r.Data, x.Data, 1e-4) {
				t.Errorf("copy should match original, got %v", r.Data)
			}
		})
	}
}

func TestAllBackends_Sswap(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.NewVector([]float32{1, 2})
			y := blas.NewVector([]float32{3, 4})
			newX, newY, err := tc.backend.Sswap(x, y)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if !approxSlice(newX.Data, []float32{3, 4}, 1e-4) {
				t.Errorf("newX should be [3,4], got %v", newX.Data)
			}
			if !approxSlice(newY.Data, []float32{1, 2}, 1e-4) {
				t.Errorf("newY should be [1,2], got %v", newY.Data)
			}
		})
	}
}

func TestAllBackends_Sswap_DimMismatch(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.NewVector([]float32{1})
			y := blas.NewVector([]float32{3, 4})
			_, _, err := tc.backend.Sswap(x, y)
			if err == nil {
				t.Fatal("expected error")
			}
		})
	}
}

// =========================================================================
// LEVEL 2: All backends
// =========================================================================

func TestAllBackends_Sgemv(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			a := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			x := blas.NewVector([]float32{1, 1})
			y := blas.NewVector([]float32{0, 0})
			r, err := tc.backend.Sgemv(blas.NoTrans, 1.0, a, x, 0.0, y)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			expected := []float32{3, 7}
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

func TestAllBackends_Sgemv_Trans(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			a := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			x := blas.NewVector([]float32{1, 1})
			y := blas.NewVector([]float32{0, 0})
			r, err := tc.backend.Sgemv(blas.Trans, 1.0, a, x, 0.0, y)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			expected := []float32{4, 6}
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

func TestAllBackends_Sger(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.NewVector([]float32{1, 2})
			y := blas.NewVector([]float32{3, 4})
			a := blas.Zeros(2, 2)
			r, err := tc.backend.Sger(1.0, x, y, a)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			expected := []float32{3, 4, 6, 8}
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

// =========================================================================
// LEVEL 3: All backends
// =========================================================================

func TestAllBackends_Sgemm(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			a := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			b := blas.MustMatrix([]float32{5, 6, 7, 8}, 2, 2)
			c := blas.Zeros(2, 2)
			r, err := tc.backend.Sgemm(blas.NoTrans, blas.NoTrans, 1.0, a, b, 0.0, c)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			expected := []float32{19, 22, 43, 50}
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

func TestAllBackends_Sgemm_Identity(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			id := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
			b := blas.MustMatrix([]float32{5, 6, 7, 8}, 2, 2)
			c := blas.Zeros(2, 2)
			r, _ := tc.backend.Sgemm(blas.NoTrans, blas.NoTrans, 1.0, id, b, 0.0, c)
			if !approxSlice(r.Data, b.Data, 1e-4) {
				t.Errorf("identity * B should be B, got %v", r.Data)
			}
		})
	}
}

func TestAllBackends_Ssymm(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			a := blas.MustMatrix([]float32{1, 2, 2, 3}, 2, 2)
			b := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
			c := blas.Zeros(2, 2)
			r, err := tc.backend.Ssymm(blas.Left, 1.0, a, b, 0.0, c)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			expected := []float32{1, 2, 2, 3}
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

func TestAllBackends_SgemmBatched(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			id := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
			b := blas.MustMatrix([]float32{2, 3, 4, 5}, 2, 2)
			c := blas.Zeros(2, 2)
			results, err := tc.backend.SgemmBatched(blas.NoTrans, blas.NoTrans, 1.0,
				[]blas.Matrix{id}, []blas.Matrix{b}, 0.0, []blas.Matrix{c})
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if len(results) != 1 {
				t.Fatalf("expected 1 result, got %d", len(results))
			}
			expected := []float32{2, 3, 4, 5}
			if !approxSlice(results[0].Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, results[0].Data)
			}
		})
	}
}

// =========================================================================
// ML Extensions: All backends
// =========================================================================

func TestAllBackends_Relu(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{-2, -1, 0, 1, 2, 3}, 2, 3)
			r := tc.backend.Relu(x)
			expected := []float32{0, 0, 0, 1, 2, 3}
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

func TestAllBackends_Gelu(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{0}, 1, 1)
			r := tc.backend.Gelu(x)
			if !approx(r.Data[0], 0, 1e-3) {
				t.Errorf("GELU(0) should be ~0, got %f", r.Data[0])
			}
		})
	}
}

func TestAllBackends_Sigmoid(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{0}, 1, 1)
			r := tc.backend.Sigmoid(x)
			if !approx(r.Data[0], 0.5, 1e-4) {
				t.Errorf("sigmoid(0) should be 0.5, got %f", r.Data[0])
			}
		})
	}
}

func TestAllBackends_TanhActivation(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{0}, 1, 1)
			r := tc.backend.TanhActivation(x)
			if !approx(r.Data[0], 0, 1e-4) {
				t.Errorf("tanh(0) should be 0, got %f", r.Data[0])
			}
		})
	}
}

func TestAllBackends_Softmax(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{1, 2, 3}, 1, 3)
			r := tc.backend.Softmax(x, -1)
			var sum float32
			for _, v := range r.Data {
				sum += v
			}
			if !approx(sum, 1.0, 1e-4) {
				t.Errorf("softmax should sum to 1, got %f", sum)
			}
		})
	}
}

func TestAllBackends_LayerNorm(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			gamma := blas.NewVector([]float32{1, 1})
			beta := blas.NewVector([]float32{0, 0})
			r, err := tc.backend.LayerNorm(x, gamma, beta, 1e-5)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			// Each row should have mean ~0
			for i := 0; i < 2; i++ {
				var mean float32
				for j := 0; j < 2; j++ {
					mean += r.Data[i*2+j]
				}
				mean /= 2
				if !approx(mean, 0, 1e-3) {
					t.Errorf("row %d mean should be ~0, got %f", i, mean)
				}
			}
		})
	}
}

func TestAllBackends_BatchNorm(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6}, 3, 2)
			gamma := blas.NewVector([]float32{1, 1})
			beta := blas.NewVector([]float32{0, 0})
			rm := blas.NewVector([]float32{0, 0})
			rv := blas.NewVector([]float32{1, 1})
			r, err := tc.backend.BatchNorm(x, gamma, beta, rm, rv, 1e-5, true)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if r.Rows != 3 || r.Cols != 2 {
				t.Errorf("expected 3x2, got %dx%d", r.Rows, r.Cols)
			}
		})
	}
}

func TestAllBackends_Conv2d(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			input := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			weight := blas.MustMatrix([]float32{1}, 1, 1)
			r, err := tc.backend.Conv2d(input, weight, nil, 1, 0)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			expected := []float32{1, 2, 3, 4}
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

func TestAllBackends_Attention(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			q := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
			k := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
			v := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
			r, err := tc.backend.Attention(q, k, v, nil, nil)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if r.Rows != 2 || r.Cols != 2 {
				t.Errorf("expected 2x2, got %dx%d", r.Rows, r.Cols)
			}
		})
	}
}

// =========================================================================
// Error path tests for GPU backends
// =========================================================================

func TestAllBackends_Sgemv_DimMismatch(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			a := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			x := blas.NewVector([]float32{1, 2, 3}) // Wrong size
			y := blas.NewVector([]float32{0, 0})
			_, err := tc.backend.Sgemv(blas.NoTrans, 1.0, a, x, 0.0, y)
			if err == nil {
				t.Fatal("expected dimension mismatch error")
			}
		})
	}
}

func TestAllBackends_Sger_DimMismatch(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.NewVector([]float32{1, 2, 3})
			y := blas.NewVector([]float32{4, 5})
			a := blas.Zeros(2, 2)
			_, err := tc.backend.Sger(1.0, x, y, a)
			if err == nil {
				t.Fatal("expected dimension mismatch error")
			}
		})
	}
}

func TestAllBackends_Sgemm_DimMismatch(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			a := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			b := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6}, 3, 2)
			c := blas.Zeros(2, 2)
			_, err := tc.backend.Sgemm(blas.NoTrans, blas.NoTrans, 1.0, a, b, 0.0, c)
			if err == nil {
				t.Fatal("expected dimension mismatch error")
			}
		})
	}
}

func TestAllBackends_Ssymm_NotSquare(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			a := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6}, 2, 3)
			b := blas.Zeros(2, 2)
			c := blas.Zeros(2, 2)
			_, err := tc.backend.Ssymm(blas.Left, 1.0, a, b, 0.0, c)
			if err == nil {
				t.Fatal("expected error for non-square A")
			}
		})
	}
}

func TestAllBackends_LayerNorm_DimMismatch(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			gamma := blas.NewVector([]float32{1, 1, 1})
			beta := blas.NewVector([]float32{0, 0})
			_, err := tc.backend.LayerNorm(x, gamma, beta, 1e-5)
			if err == nil {
				t.Fatal("expected error for gamma dimension mismatch")
			}
		})
	}
}

func TestAllBackends_BatchNorm_DimMismatch(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			gamma := blas.NewVector([]float32{1, 1, 1})
			beta := blas.NewVector([]float32{0, 0})
			rm := blas.NewVector([]float32{0, 0})
			rv := blas.NewVector([]float32{1, 1})
			_, err := tc.backend.BatchNorm(x, gamma, beta, rm, rv, 1e-5, true)
			if err == nil {
				t.Fatal("expected error for gamma dimension mismatch")
			}
		})
	}
}

func TestAllBackends_Ssymm_Left_NonTrivial(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			// Symmetric A = [[2, 1], [1, 2]], B = [[1, 2], [3, 4]]
			a := blas.MustMatrix([]float32{2, 1, 1, 2}, 2, 2)
			b := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			c := blas.Zeros(2, 2)
			r, err := tc.backend.Ssymm(blas.Left, 1.0, a, b, 0.0, c)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			// A*B = [[2+3, 4+4], [1+6, 2+8]] = [[5, 8], [7, 10]]
			expected := []float32{5, 8, 7, 10}
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

func TestAllBackends_Ssymm_Right_NonTrivial(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			a := blas.MustMatrix([]float32{2, 1, 1, 2}, 2, 2)
			b := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			c := blas.Zeros(2, 2)
			r, err := tc.backend.Ssymm(blas.Right, 1.0, a, b, 0.0, c)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			// B*A = [[1*2+2*1, 1*1+2*2], [3*2+4*1, 3*1+4*2]] = [[4, 5], [10, 11]]
			expected := []float32{4, 5, 10, 11}
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

func TestAllBackends_SgemmBatched_Multiple(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			id := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
			b1 := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			b2 := blas.MustMatrix([]float32{5, 6, 7, 8}, 2, 2)
			z := blas.Zeros(2, 2)
			results, err := tc.backend.SgemmBatched(blas.NoTrans, blas.NoTrans, 1.0,
				[]blas.Matrix{id, id}, []blas.Matrix{b1, b2}, 0.0, []blas.Matrix{z, z})
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if len(results) != 2 {
				t.Fatalf("expected 2 results, got %d", len(results))
			}
			if !approxSlice(results[0].Data, b1.Data, 1e-4) {
				t.Errorf("batch 0: expected %v, got %v", b1.Data, results[0].Data)
			}
			if !approxSlice(results[1].Data, b2.Data, 1e-4) {
				t.Errorf("batch 1: expected %v, got %v", b2.Data, results[1].Data)
			}
		})
	}
}

func TestAllBackends_Sgemm_TransBoth(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			a := blas.MustMatrix([]float32{1, 3, 2, 4}, 2, 2) // A^T = [[1,2],[3,4]]
			b := blas.MustMatrix([]float32{5, 7, 6, 8}, 2, 2) // B^T = [[5,6],[7,8]]
			c := blas.Zeros(2, 2)
			r, _ := tc.backend.Sgemm(blas.Trans, blas.Trans, 1.0, a, b, 0.0, c)
			expected := []float32{19, 22, 43, 50}
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

func TestAllBackends_Conv2d_WithPadding(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			input := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			weight := blas.MustMatrix([]float32{1, 1, 1, 1}, 2, 2)
			r, err := tc.backend.Conv2d(input, weight, nil, 1, 1)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if r.Rows != 3 || r.Cols != 3 {
				t.Errorf("expected 3x3 output with padding=1, got %dx%d", r.Rows, r.Cols)
			}
		})
	}
}

func TestAllBackends_Attention_WithMask(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			q := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
			k := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
			v := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
			negInf := float32(-1e9)
			mask := blas.MustMatrix([]float32{0, negInf, 0, 0}, 2, 2)
			r, err := tc.backend.Attention(q, k, v, &mask, nil)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if r.Rows != 2 || r.Cols != 2 {
				t.Errorf("expected 2x2, got %dx%d", r.Rows, r.Cols)
			}
		})
	}
}

func TestAllBackends_Softmax_Column(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			r := tc.backend.Softmax(x, 0)
			for j := 0; j < 2; j++ {
				var sum float32
				for i := 0; i < 2; i++ {
					sum += r.Data[i*2+j]
				}
				if !approx(sum, 1.0, 1e-4) {
					t.Errorf("col %d should sum to 1, got %f", j, sum)
				}
			}
		})
	}
}

func TestAllBackends_BatchNorm_Inference(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			gamma := blas.NewVector([]float32{1, 1})
			beta := blas.NewVector([]float32{0, 0})
			rm := blas.NewVector([]float32{2, 3})
			rv := blas.NewVector([]float32{1, 1})
			r, err := tc.backend.BatchNorm(x, gamma, beta, rm, rv, 1e-5, false)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if r.Rows != 2 || r.Cols != 2 {
				t.Errorf("expected 2x2, got %dx%d", r.Rows, r.Cols)
			}
		})
	}
}

func TestAllBackends_Gelu_Positive(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{1.0}, 1, 1)
			r := tc.backend.Gelu(x)
			if !approx(r.Data[0], 0.8412, 0.02) {
				t.Errorf("GELU(1.0) should be ~0.8412, got %f", r.Data[0])
			}
		})
	}
}

func TestAllBackends_TanhActivation_Large(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{100, -100}, 1, 2)
			r := tc.backend.TanhActivation(x)
			if !approx(r.Data[0], 1.0, 1e-3) {
				t.Errorf("tanh(100) should be ~1, got %f", r.Data[0])
			}
			if !approx(r.Data[1], -1.0, 1e-3) {
				t.Errorf("tanh(-100) should be ~-1, got %f", r.Data[1])
			}
		})
	}
}

func TestAllBackends_Conv2d_WithBias(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			input := blas.MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
			weight := blas.MustMatrix([]float32{1}, 1, 1)
			bias := blas.NewVector([]float32{10})
			r, err := tc.backend.Conv2d(input, weight, &bias, 1, 0)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			expected := []float32{11, 12, 13, 14}
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

func TestAllBackends_Attention_WithScale(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			q := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
			k := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
			v := blas.MustMatrix([]float32{1, 0, 0, 1}, 2, 2)
			scale := float32(1.0)
			r, err := tc.backend.Attention(q, k, v, nil, &scale)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if r.Rows != 2 || r.Cols != 2 {
				t.Errorf("expected 2x2, got %dx%d", r.Rows, r.Cols)
			}
		})
	}
}

// =========================================================================
// Serialization helpers tests
// =========================================================================

func TestFloat32sToBytes_Roundtrip(t *testing.T) {
	original := []float32{1.0, -2.5, 3.14, 0, math.MaxFloat32}
	bytes := float32sToBytes(original)
	if len(bytes) != len(original)*4 {
		t.Fatalf("expected %d bytes, got %d", len(original)*4, len(bytes))
	}
	result := bytesToFloat32s(bytes, len(original))
	if !approxSlice(result, original, 1e-6) {
		t.Errorf("roundtrip failed: %v != %v", result, original)
	}
}

func TestFloat32sToBytes_Empty(t *testing.T) {
	bytes := float32sToBytes([]float32{})
	if len(bytes) != 0 {
		t.Errorf("empty input should produce empty bytes, got %d", len(bytes))
	}
	result := bytesToFloat32s(bytes, 0)
	if len(result) != 0 {
		t.Errorf("expected empty result, got %v", result)
	}
}

// =========================================================================
// Additional edge-case tests per backend
// =========================================================================

func TestAllBackends_Saxpy_LargeVector(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			n := 100
			xData := make([]float32, n)
			yData := make([]float32, n)
			for i := 0; i < n; i++ {
				xData[i] = float32(i)
				yData[i] = float32(i * 2)
			}
			x := blas.NewVector(xData)
			y := blas.NewVector(yData)
			r, err := tc.backend.Saxpy(1.0, x, y)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			for i := 0; i < n; i++ {
				expected := float32(i) + float32(i*2)
				if !approx(r.Data[i], expected, 1e-3) {
					t.Errorf("element %d: expected %f, got %f", i, expected, r.Data[i])
					break
				}
			}
		})
	}
}

func TestAllBackends_Sgemm_1x1(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			a := blas.MustMatrix([]float32{3}, 1, 1)
			b := blas.MustMatrix([]float32{4}, 1, 1)
			c := blas.Zeros(1, 1)
			r, _ := tc.backend.Sgemm(blas.NoTrans, blas.NoTrans, 1.0, a, b, 0.0, c)
			if !approx(r.Data[0], 12, 1e-4) {
				t.Errorf("expected 12, got %f", r.Data[0])
			}
		})
	}
}

func TestAllBackends_Relu_AllNegative(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{-5, -3, -1, -0.5}, 2, 2)
			r := tc.backend.Relu(x)
			for i, v := range r.Data {
				if v != 0 {
					t.Errorf("element %d should be 0, got %f", i, v)
				}
			}
		})
	}
}

func TestAllBackends_Softmax_Uniform(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{1, 1, 1, 1}, 1, 4)
			r := tc.backend.Softmax(x, -1)
			for _, v := range r.Data {
				if !approx(v, 0.25, 1e-4) {
					t.Errorf("uniform softmax should give 0.25, got %f", v)
				}
			}
		})
	}
}

func TestAllBackends_Sigmoid_Large(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{100, -100}, 1, 2)
			r := tc.backend.Sigmoid(x)
			if !approx(r.Data[0], 1.0, 1e-3) {
				t.Errorf("sigmoid(100) should be ~1, got %f", r.Data[0])
			}
			if !approx(r.Data[1], 0.0, 1e-3) {
				t.Errorf("sigmoid(-100) should be ~0, got %f", r.Data[1])
			}
		})
	}
}

func TestAllBackends_Sgemv_Rectangular(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			// A is 2x3, x is 3x1, y is 2x1
			a := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6}, 2, 3)
			x := blas.NewVector([]float32{1, 1, 1})
			y := blas.NewVector([]float32{0, 0})
			r, err := tc.backend.Sgemv(blas.NoTrans, 1.0, a, x, 0.0, y)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			expected := []float32{6, 15} // [1+2+3, 4+5+6]
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

func TestAllBackends_Sgemm_NonSquare(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			// A (2x3) * B (3x1) = C (2x1)
			a := blas.MustMatrix([]float32{1, 2, 3, 4, 5, 6}, 2, 3)
			b := blas.MustMatrix([]float32{1, 1, 1}, 3, 1)
			c := blas.Zeros(2, 1)
			r, err := tc.backend.Sgemm(blas.NoTrans, blas.NoTrans, 1.0, a, b, 0.0, c)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			expected := []float32{6, 15}
			if !approxSlice(r.Data, expected, 1e-4) {
				t.Errorf("expected %v, got %v", expected, r.Data)
			}
		})
	}
}

func TestAllBackends_LayerNorm_WithScaleShift(t *testing.T) {
	for _, tc := range allBackends(t) {
		t.Run(tc.name, func(t *testing.T) {
			x := blas.MustMatrix([]float32{1, 3}, 1, 2) // mean=2, var=1
			gamma := blas.NewVector([]float32{2, 2})
			beta := blas.NewVector([]float32{5, 5})
			r, err := tc.backend.LayerNorm(x, gamma, beta, 1e-5)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			// Normalized: [-1, 1], Scaled: [-2, 2], Shifted: [3, 7]
			if !approx(r.Data[0], 3.0, 0.1) || !approx(r.Data[1], 7.0, 0.1) {
				t.Errorf("expected ~[3, 7], got %v", r.Data)
			}
		})
	}
}
