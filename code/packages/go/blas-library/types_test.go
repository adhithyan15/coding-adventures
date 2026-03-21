package blaslibrary

import (
	"testing"
)

// =========================================================================
// Vector tests
// =========================================================================

func TestNewVector(t *testing.T) {
	v := NewVector([]float32{1, 2, 3})
	if v.Size != 3 {
		t.Errorf("expected size 3, got %d", v.Size)
	}
	if v.Data[0] != 1.0 || v.Data[1] != 2.0 || v.Data[2] != 3.0 {
		t.Errorf("data mismatch: got %v", v.Data)
	}
}

func TestNewVectorEmpty(t *testing.T) {
	v := NewVector([]float32{})
	if v.Size != 0 {
		t.Errorf("expected size 0, got %d", v.Size)
	}
}

func TestNewVectorWithSize_Ok(t *testing.T) {
	v, err := NewVectorWithSize([]float32{1, 2, 3}, 3)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if v.Size != 3 {
		t.Errorf("expected size 3, got %d", v.Size)
	}
}

func TestNewVectorWithSize_Mismatch(t *testing.T) {
	_, err := NewVectorWithSize([]float32{1, 2, 3}, 5)
	if err == nil {
		t.Fatal("expected error for size mismatch")
	}
}

func TestMustVector_Ok(t *testing.T) {
	v := MustVector([]float32{1, 2}, 2)
	if v.Size != 2 {
		t.Errorf("expected size 2, got %d", v.Size)
	}
}

func TestMustVector_Panic(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic for size mismatch")
		}
	}()
	MustVector([]float32{1, 2}, 5)
}

// =========================================================================
// Matrix tests
// =========================================================================

func TestNewMatrix_Ok(t *testing.T) {
	m, err := NewMatrix([]float32{1, 2, 3, 4, 5, 6}, 2, 3)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if m.Rows != 2 || m.Cols != 3 {
		t.Errorf("expected 2x3, got %dx%d", m.Rows, m.Cols)
	}
	if m.Order != RowMajor {
		t.Errorf("expected RowMajor, got %d", m.Order)
	}
}

func TestNewMatrix_SizeMismatch(t *testing.T) {
	_, err := NewMatrix([]float32{1, 2, 3}, 2, 3)
	if err == nil {
		t.Fatal("expected error for dimension mismatch")
	}
}

func TestMustMatrix_Ok(t *testing.T) {
	m := MustMatrix([]float32{1, 2, 3, 4}, 2, 2)
	if m.Rows != 2 || m.Cols != 2 {
		t.Errorf("expected 2x2, got %dx%d", m.Rows, m.Cols)
	}
}

func TestMustMatrix_Panic(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic for dimension mismatch")
		}
	}()
	MustMatrix([]float32{1, 2, 3}, 2, 2)
}

func TestZeros(t *testing.T) {
	m := Zeros(3, 4)
	if m.Rows != 3 || m.Cols != 4 {
		t.Errorf("expected 3x4, got %dx%d", m.Rows, m.Cols)
	}
	if len(m.Data) != 12 {
		t.Errorf("expected 12 elements, got %d", len(m.Data))
	}
	for i, v := range m.Data {
		if v != 0 {
			t.Errorf("expected 0 at index %d, got %f", i, v)
		}
	}
	if m.Order != RowMajor {
		t.Errorf("expected RowMajor, got %d", m.Order)
	}
}

// =========================================================================
// Enum tests
// =========================================================================

func TestStorageOrderValues(t *testing.T) {
	if RowMajor != 0 {
		t.Errorf("RowMajor should be 0, got %d", RowMajor)
	}
	if ColumnMajor != 1 {
		t.Errorf("ColumnMajor should be 1, got %d", ColumnMajor)
	}
}

func TestTransposeValues(t *testing.T) {
	if NoTrans != 0 {
		t.Errorf("NoTrans should be 0, got %d", NoTrans)
	}
	if Trans != 1 {
		t.Errorf("Trans should be 1, got %d", Trans)
	}
}

func TestSideValues(t *testing.T) {
	if Left != 0 {
		t.Errorf("Left should be 0, got %d", Left)
	}
	if Right != 1 {
		t.Errorf("Right should be 1, got %d", Right)
	}
}

func TestNewVectorSingleElement(t *testing.T) {
	v := NewVector([]float32{42.0})
	if v.Size != 1 || v.Data[0] != 42.0 {
		t.Errorf("single element vector failed: %v", v)
	}
}

func TestNewMatrixSingleElement(t *testing.T) {
	m, err := NewMatrix([]float32{7.0}, 1, 1)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if m.Rows != 1 || m.Cols != 1 || m.Data[0] != 7.0 {
		t.Errorf("1x1 matrix failed: %v", m)
	}
}

func TestNewMatrixAccessPattern(t *testing.T) {
	// Verify row-major access: A[i][j] = Data[i * Cols + j]
	m := MustMatrix([]float32{1, 2, 3, 4, 5, 6}, 2, 3)
	// A[0][0] = 1, A[0][1] = 2, A[0][2] = 3
	// A[1][0] = 4, A[1][1] = 5, A[1][2] = 6
	if m.Data[0*3+0] != 1 || m.Data[0*3+1] != 2 || m.Data[0*3+2] != 3 {
		t.Errorf("row 0 access pattern wrong")
	}
	if m.Data[1*3+0] != 4 || m.Data[1*3+1] != 5 || m.Data[1*3+2] != 6 {
		t.Errorf("row 1 access pattern wrong")
	}
}
