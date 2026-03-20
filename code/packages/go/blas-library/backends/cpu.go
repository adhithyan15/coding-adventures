// Package backends implements BLAS backend implementations -- one CPU reference
// and six GPU-accelerated backends.
//
// # CPU Backend
//
// The CPU backend (CpuBlas) is the pure Go reference implementation. Every
// BLAS operation is implemented with explicit Go loops. No CGo, no assembly,
// no tricks -- just for loops and arithmetic. This makes every operation
// completely transparent.
//
// Every other backend's correctness is measured against CpuBlas. If
// CudaBlas.Sgemm() and CpuBlas.Sgemm() disagree on the result, the bug
// is in CudaBlas.
package backends

import (
	"fmt"
	"math"

	blas "github.com/adhithyan15/coding-adventures/code/packages/go/blas-library"
)

// =========================================================================
// Helper: access a matrix element respecting transpose
// =========================================================================

// getElement accesses a matrix element, respecting the transpose flag.
//
// Instead of physically transposing a matrix (allocating new memory and
// rearranging elements), we just swap the row/col indices:
//
//	NoTrans: A[row][col] = data[row * cols + col]
//	Trans:   A[row][col] = data[col * cols + row]
//	         (swap row and col, keep the original cols stride)
//
// This is how real BLAS libraries handle transpose -- the data stays
// in place, only the access pattern changes.
func getElement(m blas.Matrix, row, col int, trans blas.Transpose) float32 {
	if trans == blas.Trans {
		// Transposed: logical (row, col) maps to physical (col, row)
		return m.Data[col*m.Cols+row]
	}
	// Not transposed: direct access
	return m.Data[row*m.Cols+col]
}

// effectiveShape returns the effective (rows, cols) after applying transpose.
//
// A 2x3 matrix transposed becomes 3x2:
//
//	NoTrans: (2, 3) -> (2, 3)
//	Trans:   (2, 3) -> (3, 2)
func effectiveShape(m blas.Matrix, trans blas.Transpose) (int, int) {
	if trans == blas.Trans {
		return m.Cols, m.Rows
	}
	return m.Rows, m.Cols
}

// =========================================================================
// CpuBlas -- the reference implementation
// =========================================================================

// CpuBlas is the pure Go BLAS reference implementation.
//
// This struct implements both BlasBackend and MlBlasBackend interfaces using
// nothing but Go loops and the math standard library.
//
// Every other backend's correctness is measured against this one. If
// CudaBlas.Sgemm() and CpuBlas.Sgemm() disagree on the result, the bug
// is in CudaBlas, not CpuBlas.
//
// Usage:
//
//	cpu := &CpuBlas{}
//	result, _ := cpu.Saxpy(2.0, x, y)
//	result, _ := cpu.Sgemm(NoTrans, NoTrans, 1.0, A, B, 0.0, C)
type CpuBlas struct{}

// Name returns the backend identifier.
func (c *CpuBlas) Name() string { return "cpu" }

// DeviceName returns a human-readable device name.
func (c *CpuBlas) DeviceName() string { return "CPU (pure Go)" }

// =================================================================
// LEVEL 1: VECTOR-VECTOR OPERATIONS -- O(n)
// =================================================================

// Saxpy computes result = alpha * x + y.
//
// S = Single precision, A = Alpha, X = vector X, P = Plus, Y = vector Y.
//
// This is the simplest BLAS operation and our running example since
// Layer 11 (logic gates). Each element:
//
//	result[i] = alpha * x[i] + y[i]
//
// Time complexity: O(n) -- one pass through both vectors.
func (c *CpuBlas) Saxpy(alpha float32, x, y blas.Vector) (blas.Vector, error) {
	if x.Size != y.Size {
		return blas.Vector{}, fmt.Errorf(
			"SAXPY dimension mismatch: x.Size=%d != y.Size=%d", x.Size, y.Size,
		)
	}
	result := make([]float32, x.Size)
	for i := 0; i < x.Size; i++ {
		result[i] = alpha*x.Data[i] + y.Data[i]
	}
	return blas.NewVector(result), nil
}

// Sdot computes the dot product: result = sum(x[i] * y[i]).
//
// The dot product measures how "aligned" two vectors are:
//   - Parallel vectors: large positive dot product
//   - Perpendicular vectors: dot product = 0
//   - Anti-parallel: large negative dot product
//
// It's also the building block of matrix multiply (GEMM is just a grid
// of dot products).
//
// Time complexity: O(n)
func (c *CpuBlas) Sdot(x, y blas.Vector) (float32, error) {
	if x.Size != y.Size {
		return 0, fmt.Errorf(
			"DOT dimension mismatch: x.Size=%d != y.Size=%d", x.Size, y.Size,
		)
	}
	var sum float32
	for i := 0; i < x.Size; i++ {
		sum += x.Data[i] * y.Data[i]
	}
	return sum, nil
}

// Snrm2 computes the Euclidean norm: result = sqrt(sum(x[i]^2)).
//
// The "length" of a vector in Euclidean space. Used for:
//   - Normalizing vectors (dividing by the norm to get unit vectors)
//   - Convergence checks (is the gradient small enough?)
//   - Regularization (keeping weights small)
//
// Time complexity: O(n)
func (c *CpuBlas) Snrm2(x blas.Vector) float32 {
	var sum float32
	for _, xi := range x.Data {
		sum += xi * xi
	}
	return float32(math.Sqrt(float64(sum)))
}

// Sscal computes result = alpha * x (multiply every element by alpha).
//
// Time complexity: O(n)
func (c *CpuBlas) Sscal(alpha float32, x blas.Vector) blas.Vector {
	result := make([]float32, x.Size)
	for i, xi := range x.Data {
		result[i] = alpha * xi
	}
	return blas.NewVector(result)
}

// Sasum computes the absolute sum (L1 norm): result = sum(|x[i]|).
//
// Also called the Manhattan distance or taxicab norm. Used in L1
// regularization (LASSO) which encourages sparsity.
//
// Time complexity: O(n)
func (c *CpuBlas) Sasum(x blas.Vector) float32 {
	var sum float32
	for _, xi := range x.Data {
		if xi < 0 {
			sum -= xi
		} else {
			sum += xi
		}
	}
	return sum
}

// Isamax returns the 0-based index of the element with the largest
// absolute value. Used in partial pivoting for LU decomposition to
// improve numerical stability.
//
// Time complexity: O(n)
func (c *CpuBlas) Isamax(x blas.Vector) int {
	if x.Size == 0 {
		return 0
	}
	maxIdx := 0
	maxVal := float32(math.Abs(float64(x.Data[0])))
	for i := 1; i < x.Size; i++ {
		val := float32(math.Abs(float64(x.Data[i])))
		if val > maxVal {
			maxVal = val
			maxIdx = i
		}
	}
	return maxIdx
}

// Scopy creates a deep copy of a vector. Modifying the result does not
// affect the original.
//
// Time complexity: O(n)
func (c *CpuBlas) Scopy(x blas.Vector) blas.Vector {
	result := make([]float32, x.Size)
	copy(result, x.Data)
	return blas.NewVector(result)
}

// Sswap exchanges the contents of x and y. Returns (new_x, new_y)
// where new_x has y's data and new_y has x's data.
//
// Time complexity: O(n)
func (c *CpuBlas) Sswap(x, y blas.Vector) (blas.Vector, blas.Vector, error) {
	if x.Size != y.Size {
		return blas.Vector{}, blas.Vector{}, fmt.Errorf(
			"SWAP dimension mismatch: x.Size=%d != y.Size=%d", x.Size, y.Size,
		)
	}
	newX := make([]float32, y.Size)
	newY := make([]float32, x.Size)
	copy(newX, y.Data)
	copy(newY, x.Data)
	return blas.NewVector(newX), blas.NewVector(newY), nil
}

// =================================================================
// LEVEL 2: MATRIX-VECTOR OPERATIONS -- O(n^2)
// =================================================================

// Sgemv computes y = alpha * op(A) * x + beta * y.
//
// op(A) is the matrix A, optionally transposed:
//
//	NoTrans: op(A) = A    (M x N)
//	Trans:   op(A) = A^T  (N x M)
//
// After applying the transpose:
//
//	op(A) has shape (m x n)
//	x must have size n
//	y must have size m
//	result has size m
//
// Each element of the result:
//
//	result[i] = alpha * sum(op(A)[i][k] * x[k], k=0..n-1) + beta * y[i]
//
// Time complexity: O(M * N)
func (c *CpuBlas) Sgemv(
	trans blas.Transpose,
	alpha float32,
	a blas.Matrix,
	x blas.Vector,
	beta float32,
	y blas.Vector,
) (blas.Vector, error) {
	m, n := effectiveShape(a, trans)

	if x.Size != n {
		return blas.Vector{}, fmt.Errorf(
			"GEMV dimension mismatch: op(A) is %dx%d but x.Size=%d", m, n, x.Size,
		)
	}
	if y.Size != m {
		return blas.Vector{}, fmt.Errorf(
			"GEMV dimension mismatch: op(A) is %dx%d but y.Size=%d", m, n, y.Size,
		)
	}

	result := make([]float32, m)
	for i := 0; i < m; i++ {
		var s float32
		for k := 0; k < n; k++ {
			s += getElement(a, i, k, trans) * x.Data[k]
		}
		result[i] = alpha*s + beta*y.Data[i]
	}
	return blas.NewVector(result), nil
}

// Sger computes the rank-1 update: A = alpha * x * y^T + A.
//
// The outer product of two vectors creates a matrix:
//
//	x = [a, b]     y = [c, d, e]
//
//	x * y^T = [ a*c  a*d  a*e ]
//	          [ b*c  b*d  b*e ]
//
// Then we scale by alpha and add to the existing matrix A.
// Each element: result[i][j] = alpha * x[i] * y[j] + A[i][j]
//
// Time complexity: O(M * N)
func (c *CpuBlas) Sger(alpha float32, x, y blas.Vector, a blas.Matrix) (blas.Matrix, error) {
	if a.Rows != x.Size {
		return blas.Matrix{}, fmt.Errorf(
			"GER dimension mismatch: A.Rows=%d != x.Size=%d", a.Rows, x.Size,
		)
	}
	if a.Cols != y.Size {
		return blas.Matrix{}, fmt.Errorf(
			"GER dimension mismatch: A.Cols=%d != y.Size=%d", a.Cols, y.Size,
		)
	}

	result := make([]float32, len(a.Data))
	copy(result, a.Data)
	for i := 0; i < a.Rows; i++ {
		for j := 0; j < a.Cols; j++ {
			result[i*a.Cols+j] += alpha * x.Data[i] * y.Data[j]
		}
	}
	return blas.Matrix{Data: result, Rows: a.Rows, Cols: a.Cols, Order: a.Order}, nil
}

// =================================================================
// LEVEL 3: MATRIX-MATRIX OPERATIONS -- O(n^3)
// =================================================================

// Sgemm computes C = alpha * op(A) * op(B) + beta * C.
//
// This is the most important function in all of computing. NVIDIA employs
// entire teams to optimize it. 70-90% of ML training time is spent here.
//
//	C = alpha * op(A) * op(B) + beta * C
//
// where:
//
//	op(A) has shape (M x K)
//	op(B) has shape (K x N)
//	C     has shape (M x N)
//
// The triple nested loop:
//
//	for i in range(M):          // row of C
//	    for j in range(N):      // col of C
//	        sum = 0.0
//	        for k in range(K):  // shared dimension
//	            sum += op(A)[i][k] * op(B)[k][j]
//	        C[i][j] = alpha * sum + beta * C[i][j]
//
// Time complexity: O(M * N * K)
func (c *CpuBlas) Sgemm(
	transA, transB blas.Transpose,
	alpha float32,
	a, b blas.Matrix,
	beta float32,
	cMat blas.Matrix,
) (blas.Matrix, error) {
	// Determine effective shapes after transpose
	m, kA := effectiveShape(a, transA)
	kB, n := effectiveShape(b, transB)

	// The inner dimensions must match
	if kA != kB {
		return blas.Matrix{}, fmt.Errorf(
			"GEMM dimension mismatch: op(A) is %dx%d, op(B) is %dx%d. Inner dimensions %d != %d",
			m, kA, kB, n, kA, kB,
		)
	}
	k := kA

	// C must have shape (M x N)
	if cMat.Rows != m || cMat.Cols != n {
		return blas.Matrix{}, fmt.Errorf(
			"GEMM dimension mismatch: result should be %dx%d but C is %dx%d",
			m, n, cMat.Rows, cMat.Cols,
		)
	}

	// The triple nested loop -- the heart of linear algebra
	result := make([]float32, m*n)
	for i := 0; i < m; i++ {
		for j := 0; j < n; j++ {
			var s float32
			for kk := 0; kk < k; kk++ {
				s += getElement(a, i, kk, transA) * getElement(b, kk, j, transB)
			}
			result[i*n+j] = alpha*s + beta*cMat.Data[i*cMat.Cols+j]
		}
	}
	return blas.Matrix{Data: result, Rows: m, Cols: n, Order: cMat.Order}, nil
}

// Ssymm computes the symmetric matrix multiply.
//
// Like GEMM, but exploits the fact that A is symmetric (A = A^T).
//
//	Left:  C = alpha * A * B + beta * C
//	Right: C = alpha * B * A + beta * C
//
// A must be square (Rows == Cols).
func (c *CpuBlas) Ssymm(
	side blas.Side,
	alpha float32,
	a, b blas.Matrix,
	beta float32,
	cMat blas.Matrix,
) (blas.Matrix, error) {
	if a.Rows != a.Cols {
		return blas.Matrix{}, fmt.Errorf(
			"SSYMM: A must be square but is %dx%d", a.Rows, a.Cols,
		)
	}

	var m, n int
	if side == blas.Left {
		m = a.Rows
		n = b.Cols
		if b.Rows != m {
			return blas.Matrix{}, fmt.Errorf(
				"SSYMM LEFT: A is %dx%d but B.Rows=%d", m, m, b.Rows,
			)
		}
	} else {
		m = b.Rows
		n = a.Rows
		if b.Cols != n {
			return blas.Matrix{}, fmt.Errorf(
				"SSYMM RIGHT: A is %dx%d but B.Cols=%d", n, n, b.Cols,
			)
		}
	}

	if cMat.Rows != m || cMat.Cols != n {
		return blas.Matrix{}, fmt.Errorf(
			"SSYMM: C should be %dx%d but is %dx%d", m, n, cMat.Rows, cMat.Cols,
		)
	}

	// Use Sgemm with NoTrans for both -- A is symmetric so A = A^T
	if side == blas.Left {
		return c.Sgemm(blas.NoTrans, blas.NoTrans, alpha, a, b, beta, cMat)
	}
	return c.Sgemm(blas.NoTrans, blas.NoTrans, alpha, b, a, beta, cMat)
}

// SgemmBatched computes multiple independent GEMMs.
//
// Used for multi-head attention (each head is a separate GEMM), batched
// inference (each sample is a separate GEMM), and more.
//
// On a GPU, all GEMMs can run in parallel. On CPU, we just loop.
func (c *CpuBlas) SgemmBatched(
	transA, transB blas.Transpose,
	alpha float32,
	aList, bList []blas.Matrix,
	beta float32,
	cList []blas.Matrix,
) ([]blas.Matrix, error) {
	if len(aList) != len(bList) || len(bList) != len(cList) {
		return nil, fmt.Errorf(
			"batched GEMM: batch sizes don't match: A=%d, B=%d, C=%d",
			len(aList), len(bList), len(cList),
		)
	}
	results := make([]blas.Matrix, len(aList))
	for i := range aList {
		r, err := c.Sgemm(transA, transB, alpha, aList[i], bList[i], beta, cList[i])
		if err != nil {
			return nil, fmt.Errorf("batched GEMM[%d]: %w", i, err)
		}
		results[i] = r
	}
	return results, nil
}

// =================================================================
// ML EXTENSIONS: Activation Functions
// =================================================================

// Relu computes the ReLU activation: max(0, x).
//
// The most common activation function in deep learning:
//
//	relu(x) = max(0, x)
//
// Truth table for a single element:
//
//	x < 0  -> 0.0    (negative inputs are zeroed)
//	x >= 0 -> x      (positive inputs pass through)
//
// ReLU is popular because:
//  1. It's extremely fast to compute (just a comparison)
//  2. It doesn't saturate for positive values (no vanishing gradient)
//  3. It produces sparse activations (many zeros)
func (c *CpuBlas) Relu(x blas.Matrix) blas.Matrix {
	result := make([]float32, len(x.Data))
	for i, v := range x.Data {
		if v > 0 {
			result[i] = v
		}
	}
	return blas.Matrix{Data: result, Rows: x.Rows, Cols: x.Cols, Order: x.Order}
}

// Gelu computes the GELU activation: x * Phi(x) where Phi is the CDF of N(0,1).
//
// Used in GPT, BERT, and modern Transformers. Unlike ReLU which has a hard
// cutoff at 0, GELU smoothly transitions:
//
//	gelu(x) = x * 0.5 * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))
//
// This approximation (from Hendrycks & Gimpel, 2016) is what PyTorch and
// TensorFlow use.
func (c *CpuBlas) Gelu(x blas.Matrix) blas.Matrix {
	sqrt2OverPi := float32(math.Sqrt(2.0 / math.Pi))
	result := make([]float32, len(x.Data))
	for i, v := range x.Data {
		inner := sqrt2OverPi * (v + 0.044715*v*v*v)
		result[i] = 0.5 * v * (1.0 + float32(math.Tanh(float64(inner))))
	}
	return blas.Matrix{Data: result, Rows: x.Rows, Cols: x.Cols, Order: x.Order}
}

// Sigmoid computes the sigmoid activation: 1 / (1 + exp(-x)).
//
// Maps any real number to the range (0, 1):
//
//	sigmoid(-inf) -> 0
//	sigmoid(0)    -> 0.5
//	sigmoid(+inf) -> 1
//
// Numerically stable implementation: for large negative x, exp(-x) overflows.
// We use: if x >= 0, compute as 1/(1+exp(-x)); if x < 0, compute as
// exp(x)/(1+exp(x)).
func (c *CpuBlas) Sigmoid(x blas.Matrix) blas.Matrix {
	result := make([]float32, len(x.Data))
	for i, v := range x.Data {
		if v >= 0 {
			result[i] = float32(1.0 / (1.0 + math.Exp(float64(-v))))
		} else {
			ev := math.Exp(float64(v))
			result[i] = float32(ev / (1.0 + ev))
		}
	}
	return blas.Matrix{Data: result, Rows: x.Rows, Cols: x.Cols, Order: x.Order}
}

// TanhActivation computes tanh(x).
//
// Maps any real number to (-1, 1). Used in RNNs and as an activation
// function. Related to sigmoid: tanh(x) = 2*sigmoid(2x) - 1.
func (c *CpuBlas) TanhActivation(x blas.Matrix) blas.Matrix {
	result := make([]float32, len(x.Data))
	for i, v := range x.Data {
		result[i] = float32(math.Tanh(float64(v)))
	}
	return blas.Matrix{Data: result, Rows: x.Rows, Cols: x.Cols, Order: x.Order}
}

// =================================================================
// ML EXTENSIONS: Softmax
// =================================================================

// Softmax computes numerically stable softmax along an axis.
//
// Converts a vector of real numbers into a probability distribution:
//
//	softmax(x)[i] = exp(x[i]) / sum(exp(x[j]))
//
// The NAIVE implementation overflows for large x because exp(710) is
// infinity in float64. The STABLE version subtracts the max first:
//
//	softmax(x)[i] = exp(x[i] - max(x)) / sum(exp(x[j] - max(x)))
//
// This works because softmax is invariant to constant shifts:
//
//	softmax(x + c) = softmax(x) for any constant c
//
// axis=-1 means "along the last dimension" (columns for 2D). For a 2D
// matrix, this means each ROW becomes a probability distribution that
// sums to 1.0.
func (c *CpuBlas) Softmax(x blas.Matrix, axis int) blas.Matrix {
	// Normalize axis
	if axis == -1 {
		axis = 1 // last axis for 2D matrix = columns
	}

	result := make([]float32, len(x.Data))

	if axis == 1 {
		// Softmax along each row
		for i := 0; i < x.Rows; i++ {
			rowStart := i * x.Cols
			// Find max for numerical stability
			maxVal := x.Data[rowStart]
			for j := 1; j < x.Cols; j++ {
				if x.Data[rowStart+j] > maxVal {
					maxVal = x.Data[rowStart+j]
				}
			}
			// Compute exp and sum
			var total float32
			for j := 0; j < x.Cols; j++ {
				result[rowStart+j] = float32(math.Exp(float64(x.Data[rowStart+j] - maxVal)))
				total += result[rowStart+j]
			}
			// Normalize
			for j := 0; j < x.Cols; j++ {
				result[rowStart+j] /= total
			}
		}
	} else {
		// axis == 0: softmax along each column
		copy(result, x.Data)
		for j := 0; j < x.Cols; j++ {
			// Find max for this column
			maxVal := x.Data[j]
			for i := 1; i < x.Rows; i++ {
				if x.Data[i*x.Cols+j] > maxVal {
					maxVal = x.Data[i*x.Cols+j]
				}
			}
			// Compute exp and sum
			var total float32
			exps := make([]float32, x.Rows)
			for i := 0; i < x.Rows; i++ {
				exps[i] = float32(math.Exp(float64(x.Data[i*x.Cols+j] - maxVal)))
				total += exps[i]
			}
			// Normalize
			for i := 0; i < x.Rows; i++ {
				result[i*x.Cols+j] = exps[i] / total
			}
		}
	}

	return blas.Matrix{Data: result, Rows: x.Rows, Cols: x.Cols, Order: x.Order}
}

// =================================================================
// ML EXTENSIONS: Normalization
// =================================================================

// LayerNorm computes Layer Normalization (Ba et al., 2016).
//
// For each row (sample) in the matrix:
//
//  1. Compute mean: mu = sum(x) / n
//  2. Compute variance: var = sum((x - mu)^2) / n
//  3. Normalize: x_hat = (x - mu) / sqrt(var + eps)
//  4. Scale and shift: result = gamma * x_hat + beta
//
// gamma and beta are learnable parameters (one per feature).
//
// Used in: Transformers, GPT, BERT (before every attention/FFN block).
func (c *CpuBlas) LayerNorm(
	x blas.Matrix,
	gamma, beta blas.Vector,
	eps float32,
) (blas.Matrix, error) {
	if gamma.Size != x.Cols {
		return blas.Matrix{}, fmt.Errorf(
			"LayerNorm: gamma.Size=%d != x.Cols=%d", gamma.Size, x.Cols,
		)
	}
	if beta.Size != x.Cols {
		return blas.Matrix{}, fmt.Errorf(
			"LayerNorm: beta.Size=%d != x.Cols=%d", beta.Size, x.Cols,
		)
	}

	result := make([]float32, x.Rows*x.Cols)
	n := x.Cols

	for i := 0; i < x.Rows; i++ {
		rowStart := i * n

		// Step 1: mean
		var mean float32
		for j := 0; j < n; j++ {
			mean += x.Data[rowStart+j]
		}
		mean /= float32(n)

		// Step 2: variance
		var variance float32
		for j := 0; j < n; j++ {
			diff := x.Data[rowStart+j] - mean
			variance += diff * diff
		}
		variance /= float32(n)

		// Step 3 & 4: normalize, scale, shift
		invStd := float32(1.0 / math.Sqrt(float64(variance+eps)))
		for j := 0; j < n; j++ {
			xHat := (x.Data[rowStart+j] - mean) * invStd
			result[rowStart+j] = gamma.Data[j]*xHat + beta.Data[j]
		}
	}

	return blas.Matrix{Data: result, Rows: x.Rows, Cols: x.Cols, Order: x.Order}, nil
}

// BatchNorm computes Batch Normalization (Ioffe & Szegedy, 2015).
//
// Unlike layer norm (which normalizes each sample), batch norm normalizes
// each FEATURE across all samples in the batch:
//
// Training mode:
//
//	mean_j = sum(x[i][j] for i in batch) / batch_size
//	var_j  = sum((x[i][j] - mean_j)^2 for i in batch) / batch_size
//	x_hat[i][j] = (x[i][j] - mean_j) / sqrt(var_j + eps)
//	result[i][j] = gamma[j] * x_hat[i][j] + beta[j]
//
// Inference mode: Uses runningMean and runningVar instead of batch statistics.
//
// Used in: CNNs, ResNets, most non-Transformer architectures.
func (c *CpuBlas) BatchNorm(
	x blas.Matrix,
	gamma, beta, runningMean, runningVar blas.Vector,
	eps float32,
	training bool,
) (blas.Matrix, error) {
	if gamma.Size != x.Cols {
		return blas.Matrix{}, fmt.Errorf(
			"BatchNorm: gamma.Size=%d != x.Cols=%d", gamma.Size, x.Cols,
		)
	}
	if beta.Size != x.Cols {
		return blas.Matrix{}, fmt.Errorf(
			"BatchNorm: beta.Size=%d != x.Cols=%d", beta.Size, x.Cols,
		)
	}

	result := make([]float32, x.Rows*x.Cols)
	batchSize := x.Rows
	nFeatures := x.Cols

	if training {
		// Compute batch statistics
		for j := 0; j < nFeatures; j++ {
			// Column mean
			var mean float32
			for i := 0; i < batchSize; i++ {
				mean += x.Data[i*nFeatures+j]
			}
			mean /= float32(batchSize)

			// Column variance
			var variance float32
			for i := 0; i < batchSize; i++ {
				diff := x.Data[i*nFeatures+j] - mean
				variance += diff * diff
			}
			variance /= float32(batchSize)

			invStd := float32(1.0 / math.Sqrt(float64(variance+eps)))
			for i := 0; i < batchSize; i++ {
				xHat := (x.Data[i*nFeatures+j] - mean) * invStd
				result[i*nFeatures+j] = gamma.Data[j]*xHat + beta.Data[j]
			}
		}
	} else {
		// Use running statistics
		for j := 0; j < nFeatures; j++ {
			mean := runningMean.Data[j]
			variance := runningVar.Data[j]
			invStd := float32(1.0 / math.Sqrt(float64(variance+eps)))
			for i := 0; i < batchSize; i++ {
				xHat := (x.Data[i*nFeatures+j] - mean) * invStd
				result[i*nFeatures+j] = gamma.Data[j]*xHat + beta.Data[j]
			}
		}
	}

	return blas.Matrix{Data: result, Rows: x.Rows, Cols: x.Cols, Order: x.Order}, nil
}

// =================================================================
// ML EXTENSIONS: Convolution
// =================================================================

// Conv2d computes a simplified 2D convolution.
//
// We treat input as a 2D spatial feature map (height x width) and weight as
// a 2D filter (kH x kW). This is a simplified single-channel convolution
// for demonstration.
//
// Steps:
//  1. Apply padding if needed
//  2. Extract all patches
//  3. Compute dot product of weight with each patch
//
// The output has shape:
//
//	outH = (height + 2*padding - kH) / stride + 1
//	outW = (width + 2*padding - kW) / stride + 1
func (c *CpuBlas) Conv2d(
	input, weight blas.Matrix,
	bias *blas.Vector,
	stride, padding int,
) (blas.Matrix, error) {
	hIn := input.Rows
	wIn := input.Cols
	kH := weight.Rows
	kW := weight.Cols

	// Output dimensions
	outH := (hIn + 2*padding - kH) / stride + 1
	outW := (wIn + 2*padding - kW) / stride + 1

	if outH <= 0 || outW <= 0 {
		return blas.Matrix{}, fmt.Errorf(
			"Conv2d: output dimensions are non-positive: %dx%d", outH, outW,
		)
	}

	// Create padded input if needed
	var padded []float32
	var paddedW int
	if padding > 0 {
		paddedH := hIn + 2*padding
		paddedW = wIn + 2*padding
		padded = make([]float32, paddedH*paddedW)
		for i := 0; i < hIn; i++ {
			for j := 0; j < wIn; j++ {
				padded[(i+padding)*paddedW+(j+padding)] = input.Data[i*wIn+j]
			}
		}
	} else {
		paddedW = wIn
		padded = make([]float32, len(input.Data))
		copy(padded, input.Data)
	}

	// Compute convolution
	result := make([]float32, outH*outW)
	for oh := 0; oh < outH; oh++ {
		for ow := 0; ow < outW; ow++ {
			var s float32
			for kh := 0; kh < kH; kh++ {
				for kw := 0; kw < kW; kw++ {
					ih := oh*stride + kh
					iw := ow*stride + kw
					s += padded[ih*paddedW+iw] * weight.Data[kh*kW+kw]
				}
			}
			if bias != nil && bias.Size > 0 {
				s += bias.Data[0]
			}
			result[oh*outW+ow] = s
		}
	}

	return blas.Matrix{Data: result, Rows: outH, Cols: outW, Order: blas.RowMajor}, nil
}

// =================================================================
// ML EXTENSIONS: Attention
// =================================================================

// Attention computes Scaled Dot-Product Attention (Vaswani et al., 2017).
//
//	Attention(Q, K, V) = softmax(Q * K^T / sqrt(d_k)) * V
//
// Steps:
//  1. scores = Q * K^T                   (Sgemm, Level 3)
//  2. scores = scores / scale             (element-wise)
//  3. if mask: scores = scores + mask     (element-wise)
//  4. weights = softmax(scores, axis=-1)  (ML extension)
//  5. output = weights * V               (Sgemm, Level 3)
//
// Q shape: (seqLen x dK)
// K shape: (seqLen x dK)
// V shape: (seqLen x dV)
// Returns: (seqLen x dV)
//
// This is the function that enables GPT, BERT, and every Transformer
// model to attend to different parts of the input.
func (c *CpuBlas) Attention(
	q, k, v blas.Matrix,
	mask *blas.Matrix,
	scale *float32,
) (blas.Matrix, error) {
	dK := q.Cols
	var scaleVal float32
	if scale != nil {
		scaleVal = *scale
	} else {
		scaleVal = float32(math.Sqrt(float64(dK)))
	}

	// Step 1: scores = Q * K^T using Sgemm
	seqLen := q.Rows
	scoresC := blas.Zeros(seqLen, k.Rows)
	scores, err := c.Sgemm(blas.NoTrans, blas.Trans, 1.0, q, k, 0.0, scoresC)
	if err != nil {
		return blas.Matrix{}, fmt.Errorf("attention scores: %w", err)
	}

	// Step 2: scale
	scaledData := make([]float32, len(scores.Data))
	for i, v := range scores.Data {
		scaledData[i] = v / scaleVal
	}

	// Step 3: apply mask (additive, typically -inf for masked positions)
	if mask != nil {
		for i := range scaledData {
			scaledData[i] += mask.Data[i]
		}
	}

	scoresMatrix := blas.Matrix{Data: scaledData, Rows: scores.Rows, Cols: scores.Cols}

	// Step 4: softmax along the last dimension (each row)
	weights := c.Softmax(scoresMatrix, -1)

	// Step 5: output = weights * V using Sgemm
	outputC := blas.Zeros(weights.Rows, v.Cols)
	return c.Sgemm(blas.NoTrans, blas.NoTrans, 1.0, weights, v, 0.0, outputC)
}

// Compile-time check that CpuBlas implements both interfaces.
var _ blas.BlasBackend = (*CpuBlas)(nil)
var _ blas.MlBlasBackend = (*CpuBlas)(nil)
