package blaslibrary

// =========================================================================
// BlasBackend -- the core interface every backend must implement
// =========================================================================

// BlasBackend is the BLAS backend interface -- the contract every backend
// must fulfill. Whether you're running on an NVIDIA GPU, an Apple M4, or
// a Raspberry Pi CPU, if you implement this interface, you're a valid BLAS
// backend.
//
// All operations return NEW Matrix/Vector objects. They do not mutate inputs.
// This is cleaner for testing and avoids aliasing bugs. Real BLAS mutates
// in-place for performance, but we optimize for clarity.
//
// # BLAS Levels
//
//	Level 1: Vector-Vector operations -- O(n)
//	  Saxpy, Sdot, Snrm2, Sscal, Sasum, Isamax, Scopy, Sswap
//
//	Level 2: Matrix-Vector operations -- O(n^2)
//	  Sgemv, Sger
//
//	Level 3: Matrix-Matrix operations -- O(n^3)
//	  Sgemm, Ssymm, SgemmBatched
type BlasBackend interface {
	// Name returns the backend identifier: "cpu", "cuda", "metal", etc.
	Name() string

	// DeviceName returns a human-readable device name:
	// "NVIDIA H100", "Apple M4", "CPU", etc.
	DeviceName() string

	// ==========================================================
	// LEVEL 1: VECTOR-VECTOR OPERATIONS -- O(n)
	// ==========================================================

	// Saxpy computes y = alpha * x + y (Single-precision Alpha X Plus Y).
	//
	// The most famous BLAS operation. Each element:
	//   result[i] = alpha * x[i] + y[i]
	//
	// Requires: x.Size == y.Size
	// Returns: new Vector of same size, or error on dimension mismatch.
	Saxpy(alpha float32, x, y Vector) (Vector, error)

	// Sdot computes the dot product: result = sum(x[i] * y[i]).
	//
	// Measures how "aligned" two vectors are:
	//   Parallel vectors:      large positive dot product
	//   Perpendicular vectors: dot product = 0
	//   Anti-parallel:         large negative dot product
	//
	// Requires: x.Size == y.Size
	Sdot(x, y Vector) (float32, error)

	// Snrm2 computes the Euclidean norm: result = sqrt(sum(x[i]^2)).
	//
	// The "length" of a vector in Euclidean space. Used for normalizing
	// vectors, convergence checks, and regularization.
	Snrm2(x Vector) float32

	// Sscal computes result = alpha * x (scale every element).
	Sscal(alpha float32, x Vector) Vector

	// Sasum computes the absolute sum (L1 norm): result = sum(|x[i]|).
	//
	// Also called the Manhattan distance. Used in L1 regularization (LASSO).
	Sasum(x Vector) float32

	// Isamax returns the 0-based index of the element with the largest
	// absolute value. Used in partial pivoting for LU decomposition.
	Isamax(x Vector) int

	// Scopy creates a deep copy of a vector.
	Scopy(x Vector) Vector

	// Sswap exchanges the contents of x and y.
	// Returns (new_x with y's data, new_y with x's data).
	// Requires: x.Size == y.Size
	Sswap(x, y Vector) (Vector, Vector, error)

	// ==========================================================
	// LEVEL 2: MATRIX-VECTOR OPERATIONS -- O(n^2)
	// ==========================================================

	// Sgemv computes y = alpha * op(A) * x + beta * y.
	//
	// If trans == Trans, uses A^T instead of A.
	// Effective dimensions after transpose:
	//   NoTrans: A is (M x N), x must be size N, y must be size M
	//   Trans:   A is (M x N), x must be size M, y must be size N
	Sgemv(trans Transpose, alpha float32, a Matrix, x Vector, beta float32, y Vector) (Vector, error)

	// Sger computes the rank-1 update: A = alpha * x * y^T + A.
	//
	// Every element: result[i][j] = alpha * x[i] * y[j] + A[i][j]
	// Requires: A.Rows == x.Size, A.Cols == y.Size
	Sger(alpha float32, x, y Vector, a Matrix) (Matrix, error)

	// ==========================================================
	// LEVEL 3: MATRIX-MATRIX OPERATIONS -- O(n^3)
	// ==========================================================

	// Sgemm computes C = alpha * op(A) * op(B) + beta * C.
	//
	// where op(X) = X if trans == NoTrans, op(X) = X^T if trans == Trans.
	//
	// Dimensions after transpose:
	//   op(A) is (M x K), op(B) is (K x N), C is (M x N)
	//
	// This is the most important function in all of computing. 70-90% of
	// ML training time is spent here.
	Sgemm(transA, transB Transpose, alpha float32, a, b Matrix, beta float32, c Matrix) (Matrix, error)

	// Ssymm computes C = alpha * A * B + beta * C where A is symmetric.
	//
	// If side == Left:  C = alpha * A * B + beta * C
	// If side == Right: C = alpha * B * A + beta * C
	// A must be square and symmetric.
	Ssymm(side Side, alpha float32, a, b Matrix, beta float32, c Matrix) (Matrix, error)

	// SgemmBatched computes multiple independent GEMMs.
	//
	//   Cs[i] = alpha * op(As[i]) * op(Bs[i]) + beta * Cs[i]
	//
	// Requires: len(aList) == len(bList) == len(cList)
	SgemmBatched(transA, transB Transpose, alpha float32, aList, bList []Matrix, beta float32, cList []Matrix) ([]Matrix, error)
}

// =========================================================================
// MlBlasBackend -- optional ML extensions beyond classic BLAS
// =========================================================================

// MlBlasBackend extends BlasBackend with ML operations: activation functions,
// normalization, convolution, and attention.
//
// Classic BLAS handles linear algebra. ML needs additional operations.
// These CAN be built from BLAS primitives (attention = two GEMMs + softmax),
// but dedicated implementations are much faster.
//
// This interface is OPTIONAL. A backend that only implements BlasBackend is
// still a valid BLAS backend.
type MlBlasBackend interface {
	BlasBackend

	// Relu computes ReLU: result[i] = max(0, x[i]).
	Relu(x Matrix) Matrix

	// Gelu computes GELU: result[i] = x[i] * Phi(x[i]) where Phi is
	// the CDF of N(0,1). Used in GPT, BERT, and modern Transformers.
	Gelu(x Matrix) Matrix

	// Sigmoid computes sigmoid: result[i] = 1 / (1 + exp(-x[i])).
	Sigmoid(x Matrix) Matrix

	// TanhActivation computes tanh: result[i] = tanh(x[i]).
	TanhActivation(x Matrix) Matrix

	// Softmax computes numerically stable softmax along an axis.
	// axis=-1 means "along the last dimension" (columns for 2D).
	Softmax(x Matrix, axis int) Matrix

	// LayerNorm computes Layer Normalization (Ba et al., 2016).
	// gamma and beta are learnable parameters (one per feature).
	LayerNorm(x Matrix, gamma, beta Vector, eps float32) (Matrix, error)

	// BatchNorm computes Batch Normalization (Ioffe & Szegedy, 2015).
	// Uses running statistics in inference mode, batch statistics in training.
	BatchNorm(x Matrix, gamma, beta, runningMean, runningVar Vector, eps float32, training bool) (Matrix, error)

	// Conv2d computes 2D convolution via im2col + GEMM.
	// Simplified single-channel convolution for demonstration.
	Conv2d(input, weight Matrix, bias *Vector, stride, padding int) (Matrix, error)

	// Attention computes Scaled Dot-Product Attention (Vaswani et al., 2017).
	//   Attention(Q, K, V) = softmax(Q * K^T / sqrt(d_k)) * V
	Attention(q, k, v Matrix, mask *Matrix, scale *float32) (Matrix, error)
}
