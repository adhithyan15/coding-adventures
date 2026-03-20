// Package blaslibrary implements Layer 3 of the accelerator computing stack --
// a pluggable BLAS (Basic Linear Algebra Subprograms) library with swappable
// backend implementations.
//
// # What is BLAS?
//
// BLAS (Basic Linear Algebra Subprograms) is a specification for standard
// linear algebra operations. Published in 1979, it defines vector and matrix
// operations at three levels:
//
//	Level 1 (1979): Vector-Vector -- O(n)    -- SAXPY, DOT, NRM2, SCAL...
//	Level 2 (1988): Matrix-Vector -- O(n^2)  -- GEMV, GER
//	Level 3 (1990): Matrix-Matrix -- O(n^3)  -- GEMM, SYMM, Batched GEMM
//
// Plus ML extensions: ReLU, softmax, layer norm, attention, conv2d.
//
// The key insight: separate the INTERFACE (what operations exist) from the
// IMPLEMENTATION (how they run on specific hardware). Write blas.Sgemm()
// once, run it on any GPU or CPU.
//
// # The Food Delivery App Analogy
//
// Layer 4 gave us six different restaurant menus (CUDA, Metal, OpenCL, etc.)
// that all share one kitchen (Layer 5). Layer 3 gives us a single food
// delivery app that routes your order to whichever restaurant is open. You
// just say "I want matrix multiplication" -- the library picks the backend.
package blaslibrary

import "fmt"

// =========================================================================
// Enumerations -- small types that control BLAS operation behavior
// =========================================================================

// StorageOrder describes how matrix elements are laid out in memory.
//
// A 2x3 matrix:
//
//	[ 1  2  3 ]
//	[ 4  5  6 ]
//
// Row-major (C convention):    [1, 2, 3, 4, 5, 6]
//
//	A[i][j] = data[i * cols + j]
//
// Column-major (Fortran/BLAS): [1, 4, 2, 5, 3, 6]
//
//	A[i][j] = data[j * rows + i]
//
// We default to row-major because C, Go, and most ML frameworks use
// row-major. Traditional BLAS uses column-major (Fortran heritage).
type StorageOrder int

const (
	// RowMajor stores elements row by row: [row0..., row1..., row2...]
	RowMajor StorageOrder = iota

	// ColumnMajor stores elements column by column: [col0..., col1..., col2...]
	ColumnMajor
)

// Transpose controls whether a matrix is logically transposed in GEMM/GEMV.
//
// When computing C = alpha * A * B + beta * C, you often want to use A^T or
// B^T without physically transposing the matrix. The Transpose flag tells the
// backend to "pretend" the matrix is transposed.
//
// This is a classic BLAS optimization: instead of allocating a new matrix and
// copying transposed data, you just change the access pattern. For a row-major
// matrix with shape (M, N):
//
//	NoTrans: access as (M, N), stride = N
//	Trans:   access as (N, M), stride = M
type Transpose int

const (
	// NoTrans means use the matrix as-is.
	NoTrans Transpose = iota

	// Trans means logically transpose the matrix.
	Trans
)

// Side controls which side the special matrix is on (for SYMM, TRMM).
//
// SYMM computes C = alpha * A * B + beta * C where A is symmetric.
//
//	Left:  A is on the left  -> C = alpha * (A) * B + beta * C
//	Right: A is on the right -> C = alpha * B * (A) + beta * C
type Side int

const (
	// Left means the symmetric matrix is on the left.
	Left Side = iota

	// Right means the symmetric matrix is on the right.
	Right
)

// =========================================================================
// Vector -- a 1-D array of single-precision floats
// =========================================================================

// Vector is a 1-D array of single-precision floats.
//
// This is the simplest possible vector type. It holds:
//   - Data: a flat slice of float32 values
//   - Size: how many elements
//
// It is NOT a tensor. It is NOT a GPU buffer. It lives on the host (CPU).
// Each backend copies it to the device when needed and copies results back.
// This keeps the interface dead simple.
//
// Example:
//
//	v := NewVector([]float32{1.0, 2.0, 3.0})
//	v.Data[0]  // 1.0
//	v.Size     // 3
type Vector struct {
	Data []float32
	Size int
}

// NewVector creates a new Vector from a slice of float32 values.
// The size is inferred from the length of the data slice.
func NewVector(data []float32) Vector {
	return Vector{Data: data, Size: len(data)}
}

// NewVectorWithSize creates a Vector and validates that data length matches size.
// Returns an error if there is a mismatch.
func NewVectorWithSize(data []float32, size int) (Vector, error) {
	if len(data) != size {
		return Vector{}, fmt.Errorf(
			"vector data has %d elements but size=%d", len(data), size,
		)
	}
	return Vector{Data: data, Size: size}, nil
}

// =========================================================================
// Matrix -- a 2-D array of single-precision floats (flat storage)
// =========================================================================

// Matrix is a 2-D array of single-precision floats stored as a flat slice.
//
// Stored as a flat slice in row-major order by default:
//
//	NewMatrix([]float32{1,2,3,4,5,6}, 2, 3)
//
//	represents:  [ 1  2  3 ]
//	             [ 4  5  6 ]
//
//	Data[i * Cols + j] = element at row i, column j
//
// # Why Flat Storage?
//
// GPUs need contiguous memory. A [][]float32 (nested slices) has each row
// allocated separately in memory. A flat []float32 is one contiguous block --
// when we upload it to GPU memory, it's a single memcpy.
type Matrix struct {
	Data  []float32
	Rows  int
	Cols  int
	Order StorageOrder
}

// NewMatrix creates a new Matrix from flat data with the given dimensions.
// Uses row-major order by default. Returns an error if data length does not
// match rows * cols.
func NewMatrix(data []float32, rows, cols int) (Matrix, error) {
	if len(data) != rows*cols {
		return Matrix{}, fmt.Errorf(
			"matrix data has %d elements but shape is %dx%d = %d",
			len(data), rows, cols, rows*cols,
		)
	}
	return Matrix{Data: data, Rows: rows, Cols: cols, Order: RowMajor}, nil
}

// MustMatrix creates a Matrix and panics if the dimensions don't match.
// Useful in tests where you know the data is valid.
func MustMatrix(data []float32, rows, cols int) Matrix {
	m, err := NewMatrix(data, rows, cols)
	if err != nil {
		panic(err)
	}
	return m
}

// MustVector creates a Vector and panics if size doesn't match.
// Useful in tests where you know the data is valid.
func MustVector(data []float32, size int) Vector {
	v, err := NewVectorWithSize(data, size)
	if err != nil {
		panic(err)
	}
	return v
}

// Zeros creates a zero-filled Matrix with the given dimensions.
func Zeros(rows, cols int) Matrix {
	return Matrix{
		Data:  make([]float32, rows*cols),
		Rows:  rows,
		Cols:  cols,
		Order: RowMajor,
	}
}
