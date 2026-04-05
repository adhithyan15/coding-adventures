// Package matrix provides basic 2D matrix operations for linear algebra.
//
// # What is a Matrix?
//
// A matrix is a rectangular array of numbers arranged in rows and columns.
// Matrices are the fundamental data structure for linear algebra, and they
// appear everywhere in computing: graphics transformations, machine learning,
// physics simulations, and signal processing.
//
// # Operations
//
// Every public function is wrapped in an Operation, giving each call
// automatic timing, structured logging, and panic recovery.
package matrix

import (
	"fmt"
	"math"
)

type Matrix struct {
	Data [][]float64
	Rows int
	Cols int
}

func Zeros(rows, cols int) *Matrix {
	result, _ := StartNew[*Matrix]("matrix.Zeros", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			op.AddProperty("rows", rows)
			op.AddProperty("cols", cols)
			m := make([][]float64, rows)
			for i := range m {
				m[i] = make([]float64, cols)
			}
			return rf.Generate(true, false, &Matrix{Data: m, Rows: rows, Cols: cols})
		}).GetResult()
	return result
}

func New2D(data [][]float64) *Matrix {
	result, _ := StartNew[*Matrix]("matrix.New2D", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			rows := len(data)
			cols := 0
			if rows > 0 {
				cols = len(data[0])
			}
			return rf.Generate(true, false, &Matrix{Data: data, Rows: rows, Cols: cols})
		}).GetResult()
	return result
}

func New1D(data []float64) *Matrix {
	result, _ := StartNew[*Matrix]("matrix.New1D", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			m := make([][]float64, 1)
			m[0] = make([]float64, len(data))
			copy(m[0], data)
			return rf.Generate(true, false, &Matrix{Data: m, Rows: 1, Cols: len(data)})
		}).GetResult()
	return result
}

func NewScalar(val float64) *Matrix {
	result, _ := StartNew[*Matrix]("matrix.NewScalar", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			op.AddProperty("val", val)
			return rf.Generate(true, false, &Matrix{Data: [][]float64{{val}}, Rows: 1, Cols: 1})
		}).GetResult()
	return result
}

func (m *Matrix) Add(other *Matrix) (*Matrix, error) {
	return StartNew[*Matrix]("matrix.Add", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			if m.Rows != other.Rows || m.Cols != other.Cols {
				return rf.Fail(nil, fmt.Errorf("matrix addition dimensions must match exactly"))
			}
			C := Zeros(m.Rows, m.Cols)
			for i := 0; i < m.Rows; i++ {
				for j := 0; j < m.Cols; j++ {
					C.Data[i][j] = m.Data[i][j] + other.Data[i][j]
				}
			}
			return rf.Generate(true, false, C)
		}).GetResult()
}

func (m *Matrix) AddScalar(scalar float64) *Matrix {
	result, _ := StartNew[*Matrix]("matrix.AddScalar", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			op.AddProperty("scalar", scalar)
			C := Zeros(m.Rows, m.Cols)
			for i := 0; i < m.Rows; i++ {
				for j := 0; j < m.Cols; j++ {
					C.Data[i][j] = m.Data[i][j] + scalar
				}
			}
			return rf.Generate(true, false, C)
		}).GetResult()
	return result
}

func (m *Matrix) Subtract(other *Matrix) (*Matrix, error) {
	return StartNew[*Matrix]("matrix.Subtract", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			if m.Rows != other.Rows || m.Cols != other.Cols {
				return rf.Fail(nil, fmt.Errorf("matrix subtraction dimensions must match exactly"))
			}
			C := Zeros(m.Rows, m.Cols)
			for i := 0; i < m.Rows; i++ {
				for j := 0; j < m.Cols; j++ {
					C.Data[i][j] = m.Data[i][j] - other.Data[i][j]
				}
			}
			return rf.Generate(true, false, C)
		}).GetResult()
}

func (m *Matrix) Scale(scalar float64) *Matrix {
	result, _ := StartNew[*Matrix]("matrix.Scale", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			op.AddProperty("scalar", scalar)
			C := Zeros(m.Rows, m.Cols)
			for i := 0; i < m.Rows; i++ {
				for j := 0; j < m.Cols; j++ {
					C.Data[i][j] = m.Data[i][j] * scalar
				}
			}
			return rf.Generate(true, false, C)
		}).GetResult()
	return result
}

func (m *Matrix) Transpose() *Matrix {
	result, _ := StartNew[*Matrix]("matrix.Transpose", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			if m.Rows == 0 {
				return rf.Generate(true, false, Zeros(0, 0))
			}
			C := Zeros(m.Cols, m.Rows)
			for i := 0; i < m.Rows; i++ {
				for j := 0; j < m.Cols; j++ {
					C.Data[j][i] = m.Data[i][j]
				}
			}
			return rf.Generate(true, false, C)
		}).GetResult()
	return result
}

func (m *Matrix) Dot(other *Matrix) (*Matrix, error) {
	return StartNew[*Matrix]("matrix.Dot", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			if m.Cols != other.Rows {
				return rf.Fail(nil, fmt.Errorf("dot product inner dimensions mismatch: %d cols vs %d rows", m.Cols, other.Rows))
			}
			C := Zeros(m.Rows, other.Cols)
			for i := 0; i < m.Rows; i++ {
				for j := 0; j < other.Cols; j++ {
					for k := 0; k < m.Cols; k++ {
						C.Data[i][j] += m.Data[i][k] * other.Data[k][j]
					}
				}
			}
			return rf.Generate(true, false, C)
		}).GetResult()
}

// ─────────────────────────────────────────────────────────────────────────────
// Element Access
//
// Get reads a single cell. Set returns a *new* matrix with one cell changed
// (the original is never mutated, following our immutable-by-default rule).
// ─────────────────────────────────────────────────────────────────────────────

// Get returns the element at (row, col). Returns an error if the indices
// are out of bounds, mirroring how Go handles boundary violations explicitly.
func (m *Matrix) Get(row, col int) (float64, error) {
	return StartNew[float64]("matrix.Get", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if row < 0 || row >= m.Rows || col < 0 || col >= m.Cols {
				return rf.Fail(0, fmt.Errorf("index (%d, %d) out of bounds for %dx%d matrix", row, col, m.Rows, m.Cols))
			}
			return rf.Generate(true, false, m.Data[row][col])
		}).GetResult()
}

// Set returns a new matrix with the element at (row, col) replaced by value.
// The original matrix is unchanged — this is the immutable-by-default pattern.
func (m *Matrix) Set(row, col int, value float64) (*Matrix, error) {
	return StartNew[*Matrix]("matrix.Set", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			if row < 0 || row >= m.Rows || col < 0 || col >= m.Cols {
				return rf.Fail(nil, fmt.Errorf("index (%d, %d) out of bounds for %dx%d matrix", row, col, m.Rows, m.Cols))
			}
			C := Zeros(m.Rows, m.Cols)
			for i := 0; i < m.Rows; i++ {
				copy(C.Data[i], m.Data[i])
			}
			C.Data[row][col] = value
			return rf.Generate(true, false, C)
		}).GetResult()
}

// ─────────────────────────────────────────────────────────────────────────────
// Reductions
//
// Reductions collapse a matrix (or parts of it) into a single number or a
// smaller matrix. They answer aggregate questions: "What is the total?",
// "Which element is largest?", "What is the average?"
// ─────────────────────────────────────────────────────────────────────────────

// Sum returns the sum of every element in the matrix.
// For [[1,2],[3,4]] the answer is 10.
func (m *Matrix) Sum() float64 {
	result, _ := StartNew[float64]("matrix.Sum", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			total := 0.0
			for i := 0; i < m.Rows; i++ {
				for j := 0; j < m.Cols; j++ {
					total += m.Data[i][j]
				}
			}
			return rf.Generate(true, false, total)
		}).GetResult()
	return result
}

// SumRows returns an (rows x 1) column vector where each entry is the sum
// of the corresponding row. Imagine collapsing each row into a single number.
func (m *Matrix) SumRows() *Matrix {
	result, _ := StartNew[*Matrix]("matrix.SumRows", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			C := Zeros(m.Rows, 1)
			for i := 0; i < m.Rows; i++ {
				s := 0.0
				for j := 0; j < m.Cols; j++ {
					s += m.Data[i][j]
				}
				C.Data[i][0] = s
			}
			return rf.Generate(true, false, C)
		}).GetResult()
	return result
}

// SumCols returns a (1 x cols) row vector where each entry is the sum
// of the corresponding column. Imagine collapsing each column downward.
func (m *Matrix) SumCols() *Matrix {
	result, _ := StartNew[*Matrix]("matrix.SumCols", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			C := Zeros(1, m.Cols)
			for j := 0; j < m.Cols; j++ {
				s := 0.0
				for i := 0; i < m.Rows; i++ {
					s += m.Data[i][j]
				}
				C.Data[0][j] = s
			}
			return rf.Generate(true, false, C)
		}).GetResult()
	return result
}

// Mean returns the arithmetic mean of all elements (sum / count).
func (m *Matrix) Mean() (float64, error) {
	return StartNew[float64]("matrix.Mean", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			n := m.Rows * m.Cols
			if n == 0 {
				return rf.Fail(0, fmt.Errorf("cannot compute mean of an empty matrix"))
			}
			return rf.Generate(true, false, m.Sum()/float64(n))
		}).GetResult()
}

// Min returns the smallest element in the matrix.
func (m *Matrix) Min() (float64, error) {
	return StartNew[float64]("matrix.Min", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if m.Rows == 0 || m.Cols == 0 {
				return rf.Fail(0, fmt.Errorf("cannot compute min of an empty matrix"))
			}
			best := m.Data[0][0]
			for i := 0; i < m.Rows; i++ {
				for j := 0; j < m.Cols; j++ {
					if m.Data[i][j] < best {
						best = m.Data[i][j]
					}
				}
			}
			return rf.Generate(true, false, best)
		}).GetResult()
}

// Max returns the largest element in the matrix.
func (m *Matrix) Max() (float64, error) {
	return StartNew[float64]("matrix.Max", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if m.Rows == 0 || m.Cols == 0 {
				return rf.Fail(0, fmt.Errorf("cannot compute max of an empty matrix"))
			}
			best := m.Data[0][0]
			for i := 0; i < m.Rows; i++ {
				for j := 0; j < m.Cols; j++ {
					if m.Data[i][j] > best {
						best = m.Data[i][j]
					}
				}
			}
			return rf.Generate(true, false, best)
		}).GetResult()
}

// Argmin returns the (row, col) position of the smallest element.
// First occurrence wins when there are ties.
func (m *Matrix) Argmin() (int, int, error) {
	type pos struct {
		r, c int
	}
	p, err := StartNew[pos]("matrix.Argmin", pos{},
		func(op *Operation[pos], rf *ResultFactory[pos]) *OperationResult[pos] {
			if m.Rows == 0 || m.Cols == 0 {
				return rf.Fail(pos{}, fmt.Errorf("cannot compute argmin of an empty matrix"))
			}
			bestVal := m.Data[0][0]
			bestR, bestC := 0, 0
			for i := 0; i < m.Rows; i++ {
				for j := 0; j < m.Cols; j++ {
					if m.Data[i][j] < bestVal {
						bestVal = m.Data[i][j]
						bestR, bestC = i, j
					}
				}
			}
			return rf.Generate(true, false, pos{bestR, bestC})
		}).GetResult()
	return p.r, p.c, err
}

// Argmax returns the (row, col) position of the largest element.
// First occurrence wins when there are ties.
func (m *Matrix) Argmax() (int, int, error) {
	type pos struct {
		r, c int
	}
	p, err := StartNew[pos]("matrix.Argmax", pos{},
		func(op *Operation[pos], rf *ResultFactory[pos]) *OperationResult[pos] {
			if m.Rows == 0 || m.Cols == 0 {
				return rf.Fail(pos{}, fmt.Errorf("cannot compute argmax of an empty matrix"))
			}
			bestVal := m.Data[0][0]
			bestR, bestC := 0, 0
			for i := 0; i < m.Rows; i++ {
				for j := 0; j < m.Cols; j++ {
					if m.Data[i][j] > bestVal {
						bestVal = m.Data[i][j]
						bestR, bestC = i, j
					}
				}
			}
			return rf.Generate(true, false, pos{bestR, bestC})
		}).GetResult()
	return p.r, p.c, err
}

// ─────────────────────────────────────────────────────────────────────────────
// Element-wise Math
//
// These methods apply a function to every element independently. The shape
// stays the same; only the values change. Think of each cell being transformed
// in isolation, like applying a filter to every pixel in an image.
// ─────────────────────────────────────────────────────────────────────────────

// Map applies fn to every element and returns a new matrix.
// This is the most general element-wise operation — Sqrt, Abs, and Pow
// are all built on top of it.
func (m *Matrix) Map(fn func(float64) float64) *Matrix {
	result, _ := StartNew[*Matrix]("matrix.Map", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			C := Zeros(m.Rows, m.Cols)
			for i := 0; i < m.Rows; i++ {
				for j := 0; j < m.Cols; j++ {
					C.Data[i][j] = fn(m.Data[i][j])
				}
			}
			return rf.Generate(true, false, C)
		}).GetResult()
	return result
}

// Sqrt returns a new matrix with the square root of each element.
func (m *Matrix) Sqrt() *Matrix {
	return m.Map(math.Sqrt)
}

// Abs returns a new matrix with the absolute value of each element.
func (m *Matrix) Abs() *Matrix {
	return m.Map(math.Abs)
}

// Pow returns a new matrix with each element raised to exp.
func (m *Matrix) Pow(exp float64) *Matrix {
	return m.Map(func(x float64) float64 { return math.Pow(x, exp) })
}

// ─────────────────────────────────────────────────────────────────────────────
// Shape Operations
//
// Shape operations rearrange elements without altering their values.
// Flatten, Reshape, Row, Col, and Slice all produce new matrices.
// ─────────────────────────────────────────────────────────────────────────────

// Flatten returns a 1 x n row vector containing all elements in row-major
// order (left-to-right, top-to-bottom).
func (m *Matrix) Flatten() *Matrix {
	result, _ := StartNew[*Matrix]("matrix.Flatten", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			n := m.Rows * m.Cols
			flat := make([]float64, 0, n)
			for i := 0; i < m.Rows; i++ {
				flat = append(flat, m.Data[i]...)
			}
			return rf.Generate(true, false, New1D(flat))
		}).GetResult()
	return result
}

// Reshape returns a new matrix with the given dimensions. The total number
// of elements must stay the same — you cannot create or destroy values.
func (m *Matrix) Reshape(rows, cols int) (*Matrix, error) {
	return StartNew[*Matrix]("matrix.Reshape", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			total := m.Rows * m.Cols
			if rows*cols != total {
				return rf.Fail(nil, fmt.Errorf("cannot reshape %dx%d (%d elements) into %dx%d (%d elements)",
					m.Rows, m.Cols, total, rows, cols, rows*cols))
			}
			flat := m.Flatten()
			C := Zeros(rows, cols)
			for i := 0; i < rows; i++ {
				for j := 0; j < cols; j++ {
					C.Data[i][j] = flat.Data[0][i*cols+j]
				}
			}
			return rf.Generate(true, false, C)
		}).GetResult()
}

// Row extracts row i as a 1 x cols matrix.
func (m *Matrix) Row(i int) (*Matrix, error) {
	return StartNew[*Matrix]("matrix.Row", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			if i < 0 || i >= m.Rows {
				return rf.Fail(nil, fmt.Errorf("row %d out of bounds for %d-row matrix", i, m.Rows))
			}
			row := make([]float64, m.Cols)
			copy(row, m.Data[i])
			return rf.Generate(true, false, New1D(row))
		}).GetResult()
}

// Col extracts column j as a rows x 1 matrix.
func (m *Matrix) Col(j int) (*Matrix, error) {
	return StartNew[*Matrix]("matrix.Col", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			if j < 0 || j >= m.Cols {
				return rf.Fail(nil, fmt.Errorf("column %d out of bounds for %d-column matrix", j, m.Cols))
			}
			C := Zeros(m.Rows, 1)
			for i := 0; i < m.Rows; i++ {
				C.Data[i][0] = m.Data[i][j]
			}
			return rf.Generate(true, false, C)
		}).GetResult()
}

// Slice extracts a sub-matrix for rows [r0, r1) and cols [c0, c1).
// Half-open intervals, just like Go slices.
func (m *Matrix) Slice(r0, r1, c0, c1 int) (*Matrix, error) {
	return StartNew[*Matrix]("matrix.Slice", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			if r0 < 0 || r1 > m.Rows || c0 < 0 || c1 > m.Cols {
				return rf.Fail(nil, fmt.Errorf("slice [%d:%d, %d:%d] out of bounds for %dx%d matrix",
					r0, r1, c0, c1, m.Rows, m.Cols))
			}
			if r0 >= r1 || c0 >= c1 {
				return rf.Fail(nil, fmt.Errorf("slice dimensions must be positive (r0 < r1, c0 < c1)"))
			}
			C := Zeros(r1-r0, c1-c0)
			for i := r0; i < r1; i++ {
				for j := c0; j < c1; j++ {
					C.Data[i-r0][j-c0] = m.Data[i][j]
				}
			}
			return rf.Generate(true, false, C)
		}).GetResult()
}

// ─────────────────────────────────────────────────────────────────────────────
// Equality and Comparison
// ─────────────────────────────────────────────────────────────────────────────

// Equals returns true if both matrices have the same shape and every
// corresponding element is identical.
func (m *Matrix) Equals(other *Matrix) bool {
	result, _ := StartNew[bool]("matrix.Equals", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			if m.Rows != other.Rows || m.Cols != other.Cols {
				return rf.Generate(true, false, false)
			}
			for i := 0; i < m.Rows; i++ {
				for j := 0; j < m.Cols; j++ {
					if m.Data[i][j] != other.Data[i][j] {
						return rf.Generate(true, false, false)
					}
				}
			}
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}

// Close checks whether two matrices are element-wise within tolerance.
// Useful for comparing floating-point results where tiny rounding errors
// make exact equality unreliable.
func (m *Matrix) Close(other *Matrix, tolerance float64) bool {
	result, _ := StartNew[bool]("matrix.Close", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			if m.Rows != other.Rows || m.Cols != other.Cols {
				return rf.Generate(true, false, false)
			}
			for i := 0; i < m.Rows; i++ {
				for j := 0; j < m.Cols; j++ {
					if math.Abs(m.Data[i][j]-other.Data[i][j]) > tolerance {
						return rf.Generate(true, false, false)
					}
				}
			}
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}

// ─────────────────────────────────────────────────────────────────────────────
// Factory Methods
//
// These create common matrix patterns without manually constructing data.
// ─────────────────────────────────────────────────────────────────────────────

// Identity returns an n x n identity matrix — the matrix equivalent of
// the number 1. Multiplying any matrix by the identity leaves it unchanged.
func Identity(n int) *Matrix {
	result, _ := StartNew[*Matrix]("matrix.Identity", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			C := Zeros(n, n)
			for i := 0; i < n; i++ {
				C.Data[i][i] = 1.0
			}
			return rf.Generate(true, false, C)
		}).GetResult()
	return result
}

// FromDiagonal creates a square diagonal matrix from a slice of values.
// A diagonal matrix has non-zero entries only on the main diagonal.
func FromDiagonal(values []float64) *Matrix {
	result, _ := StartNew[*Matrix]("matrix.FromDiagonal", nil,
		func(op *Operation[*Matrix], rf *ResultFactory[*Matrix]) *OperationResult[*Matrix] {
			n := len(values)
			C := Zeros(n, n)
			for i := 0; i < n; i++ {
				C.Data[i][i] = values[i]
			}
			return rf.Generate(true, false, C)
		}).GetResult()
	return result
}
