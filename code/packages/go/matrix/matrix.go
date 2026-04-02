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

import "fmt"

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
