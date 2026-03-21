package matrix

import "fmt"

type Matrix struct {
	Data [][]float64
	Rows int
	Cols int
}

func Zeros(rows, cols int) *Matrix {
	m := make([][]float64, rows)
	for i := range m {
		m[i] = make([]float64, cols)
	}
	return &Matrix{Data: m, Rows: rows, Cols: cols}
}

func New2D(data [][]float64) *Matrix {
	rows := len(data)
	cols := 0
	if rows > 0 {
		cols = len(data[0])
	}
	return &Matrix{Data: data, Rows: rows, Cols: cols}
}

func New1D(data []float64) *Matrix {
	m := make([][]float64, 1)
	m[0] = make([]float64, len(data))
	copy(m[0], data)
	return &Matrix{Data: m, Rows: 1, Cols: len(data)}
}

func NewScalar(val float64) *Matrix {
	return &Matrix{Data: [][]float64{{val}}, Rows: 1, Cols: 1}
}

func (m *Matrix) Add(other *Matrix) (*Matrix, error) {
	if m.Rows != other.Rows || m.Cols != other.Cols {
		return nil, fmt.Errorf("matrix addition dimensions must match exactly")
	}
	C := Zeros(m.Rows, m.Cols)
	for i := 0; i < m.Rows; i++ {
		for j := 0; j < m.Cols; j++ {
			C.Data[i][j] = m.Data[i][j] + other.Data[i][j]
		}
	}
	return C, nil
}

func (m *Matrix) AddScalar(scalar float64) *Matrix {
	C := Zeros(m.Rows, m.Cols)
	for i := 0; i < m.Rows; i++ {
		for j := 0; j < m.Cols; j++ {
			C.Data[i][j] = m.Data[i][j] + scalar
		}
	}
	return C
}

func (m *Matrix) Subtract(other *Matrix) (*Matrix, error) {
	if m.Rows != other.Rows || m.Cols != other.Cols {
		return nil, fmt.Errorf("matrix subtraction dimensions must match exactly")
	}
	C := Zeros(m.Rows, m.Cols)
	for i := 0; i < m.Rows; i++ {
		for j := 0; j < m.Cols; j++ {
			C.Data[i][j] = m.Data[i][j] - other.Data[i][j]
		}
	}
	return C, nil
}

func (m *Matrix) Scale(scalar float64) *Matrix {
	C := Zeros(m.Rows, m.Cols)
	for i := 0; i < m.Rows; i++ {
		for j := 0; j < m.Cols; j++ {
			C.Data[i][j] = m.Data[i][j] * scalar
		}
	}
	return C
}

func (m *Matrix) Transpose() *Matrix {
	if m.Rows == 0 {
		return Zeros(0, 0)
	}
	C := Zeros(m.Cols, m.Rows)
	for i := 0; i < m.Rows; i++ {
		for j := 0; j < m.Cols; j++ {
			C.Data[j][i] = m.Data[i][j]
		}
	}
	return C
}

func (m *Matrix) Dot(other *Matrix) (*Matrix, error) {
	if m.Cols != other.Rows {
		return nil, fmt.Errorf("dot product inner dimensions mismatch: %d cols vs %d rows", m.Cols, other.Rows)
	}
	C := Zeros(m.Rows, other.Cols)
	for i := 0; i < m.Rows; i++ {
		for j := 0; j < other.Cols; j++ {
			for k := 0; k < m.Cols; k++ {
				C.Data[i][j] += m.Data[i][k] * other.Data[k][j]
			}
		}
	}
	return C, nil
}
