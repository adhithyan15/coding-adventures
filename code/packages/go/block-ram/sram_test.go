package blockram

import (
	"testing"
)

// =========================================================================
// Test Helpers
// =========================================================================

func assertPanics(t *testing.T, name string, fn func()) {
	t.Helper()
	defer func() {
		r := recover()
		if r == nil {
			t.Errorf("%s: expected panic, but did not panic", name)
		}
	}()
	fn()
}

func assertSliceEqual(t *testing.T, name string, got, want []int) {
	t.Helper()
	if len(got) != len(want) {
		t.Errorf("%s: length mismatch: got %d, want %d", name, len(got), len(want))
		return
	}
	for i := range got {
		if got[i] != want[i] {
			t.Errorf("%s[%d] = %d, want %d", name, i, got[i], want[i])
		}
	}
}

// =========================================================================
// SRAMCell Tests
// =========================================================================

func TestSRAMCell_InitialValue(t *testing.T) {
	cell := NewSRAMCell()
	if cell.Value() != 0 {
		t.Errorf("NewSRAMCell().Value() = %d, want 0", cell.Value())
	}
}

func TestSRAMCell_WriteAndRead(t *testing.T) {
	cell := NewSRAMCell()

	// Write 1 when word line is active
	cell.Write(1, 1)
	if cell.Value() != 1 {
		t.Errorf("After Write(1,1): Value() = %d, want 1", cell.Value())
	}

	// Read when word line is active
	val := cell.Read(1)
	if val == nil || *val != 1 {
		t.Errorf("Read(1) = %v, want &1", val)
	}
}

func TestSRAMCell_ReadNotSelected(t *testing.T) {
	cell := NewSRAMCell()
	cell.Write(1, 1)

	// Read when word line is inactive → nil
	val := cell.Read(0)
	if val != nil {
		t.Errorf("Read(0) = %v, want nil", val)
	}
}

func TestSRAMCell_WriteNotSelected(t *testing.T) {
	cell := NewSRAMCell()
	cell.Write(1, 1) // Store 1

	// Write with word line inactive → no change
	cell.Write(0, 0)
	if cell.Value() != 1 {
		t.Errorf("After Write(0,0): Value() = %d, want 1 (unchanged)", cell.Value())
	}
}

func TestSRAMCell_Overwrite(t *testing.T) {
	cell := NewSRAMCell()
	cell.Write(1, 1)
	cell.Write(1, 0)
	if cell.Value() != 0 {
		t.Errorf("After overwrite: Value() = %d, want 0", cell.Value())
	}
}

func TestSRAMCell_InvalidInput(t *testing.T) {
	cell := NewSRAMCell()
	assertPanics(t, "Read(2)", func() { cell.Read(2) })
	assertPanics(t, "Write(2,0)", func() { cell.Write(2, 0) })
	assertPanics(t, "Write(1,2)", func() { cell.Write(1, 2) })
}

// =========================================================================
// SRAMArray Tests
// =========================================================================

func TestSRAMArray_InitialValues(t *testing.T) {
	arr := NewSRAMArray(4, 8)
	rows, cols := arr.Shape()
	if rows != 4 || cols != 8 {
		t.Errorf("Shape() = (%d, %d), want (4, 8)", rows, cols)
	}

	// All cells should be 0
	for r := 0; r < 4; r++ {
		data := arr.Read(r)
		for c, v := range data {
			if v != 0 {
				t.Errorf("arr.Read(%d)[%d] = %d, want 0", r, c, v)
			}
		}
	}
}

func TestSRAMArray_WriteAndRead(t *testing.T) {
	arr := NewSRAMArray(4, 8)

	data := []int{1, 0, 1, 0, 0, 1, 0, 1}
	arr.Write(0, data)
	assertSliceEqual(t, "Read(0)", arr.Read(0), data)

	// Other rows should still be 0
	assertSliceEqual(t, "Read(1)", arr.Read(1), []int{0, 0, 0, 0, 0, 0, 0, 0})
}

func TestSRAMArray_Overwrite(t *testing.T) {
	arr := NewSRAMArray(2, 4)

	arr.Write(0, []int{1, 1, 1, 1})
	arr.Write(0, []int{0, 0, 0, 0})
	assertSliceEqual(t, "Read(0) after overwrite", arr.Read(0), []int{0, 0, 0, 0})
}

func TestSRAMArray_MultipleRows(t *testing.T) {
	arr := NewSRAMArray(3, 2)
	arr.Write(0, []int{1, 0})
	arr.Write(1, []int{0, 1})
	arr.Write(2, []int{1, 1})

	assertSliceEqual(t, "Read(0)", arr.Read(0), []int{1, 0})
	assertSliceEqual(t, "Read(1)", arr.Read(1), []int{0, 1})
	assertSliceEqual(t, "Read(2)", arr.Read(2), []int{1, 1})
}

func TestSRAMArray_Invalid(t *testing.T) {
	assertPanics(t, "NewSRAMArray(0,1)", func() { NewSRAMArray(0, 1) })
	assertPanics(t, "NewSRAMArray(1,0)", func() { NewSRAMArray(1, 0) })

	arr := NewSRAMArray(2, 4)
	assertPanics(t, "Read(-1)", func() { arr.Read(-1) })
	assertPanics(t, "Read(2)", func() { arr.Read(2) })
	assertPanics(t, "Write(-1, data)", func() { arr.Write(-1, []int{0, 0, 0, 0}) })
	assertPanics(t, "Write(2, data)", func() { arr.Write(2, []int{0, 0, 0, 0}) })
	assertPanics(t, "Write wrong length", func() { arr.Write(0, []int{0, 0}) })
	assertPanics(t, "Write bad bit", func() { arr.Write(0, []int{0, 0, 2, 0}) })
}
