package fpga

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

// =========================================================================
// LUT Tests
// =========================================================================

func TestLUT_ANDGate(t *testing.T) {
	// Configure a 4-input LUT as a 2-input AND gate (using I0, I1)
	// Index 3 (I0=1, I1=1) is the only case where AND is 1
	andTable := make([]int, 16)
	andTable[3] = 1

	lut := NewLUT(4, andTable)

	tests := []struct {
		inputs   []int
		expected int
	}{
		{[]int{0, 0, 0, 0}, 0},
		{[]int{1, 0, 0, 0}, 0},
		{[]int{0, 1, 0, 0}, 0},
		{[]int{1, 1, 0, 0}, 1}, // AND(1,1) = 1
	}
	for _, tc := range tests {
		got := lut.Evaluate(tc.inputs)
		if got != tc.expected {
			t.Errorf("AND LUT(%v) = %d, want %d", tc.inputs, got, tc.expected)
		}
	}
}

func TestLUT_XORGate(t *testing.T) {
	// XOR: index 1 (I0=1,I1=0) and index 2 (I0=0,I1=1) are 1
	xorTable := make([]int, 16)
	xorTable[1] = 1
	xorTable[2] = 1

	lut := NewLUT(4, xorTable)

	if got := lut.Evaluate([]int{1, 0, 0, 0}); got != 1 {
		t.Errorf("XOR(1,0) = %d, want 1", got)
	}
	if got := lut.Evaluate([]int{0, 1, 0, 0}); got != 1 {
		t.Errorf("XOR(0,1) = %d, want 1", got)
	}
	if got := lut.Evaluate([]int{1, 1, 0, 0}); got != 0 {
		t.Errorf("XOR(1,1) = %d, want 0", got)
	}
	if got := lut.Evaluate([]int{0, 0, 0, 0}); got != 0 {
		t.Errorf("XOR(0,0) = %d, want 0", got)
	}
}

func TestLUT_Configure(t *testing.T) {
	lut := NewLUT(4, nil)

	// Initially all zeros
	if got := lut.Evaluate([]int{1, 1, 0, 0}); got != 0 {
		t.Errorf("unconfigured LUT = %d, want 0", got)
	}

	// Configure as AND
	andTable := make([]int, 16)
	andTable[3] = 1
	lut.Configure(andTable)

	if got := lut.Evaluate([]int{1, 1, 0, 0}); got != 1 {
		t.Errorf("configured AND LUT = %d, want 1", got)
	}
}

func TestLUT_TruthTable(t *testing.T) {
	table := make([]int, 16)
	table[0] = 1
	table[15] = 1

	lut := NewLUT(4, table)
	tt := lut.TruthTable()
	if len(tt) != 16 {
		t.Fatalf("TruthTable length = %d, want 16", len(tt))
	}
	if tt[0] != 1 || tt[15] != 1 {
		t.Errorf("TruthTable[0]=%d, [15]=%d, want 1, 1", tt[0], tt[15])
	}
	if tt[1] != 0 {
		t.Errorf("TruthTable[1]=%d, want 0", tt[1])
	}
}

func TestLUT_K(t *testing.T) {
	lut := NewLUT(3, nil)
	if lut.K() != 3 {
		t.Errorf("K() = %d, want 3", lut.K())
	}
}

func TestLUT_2Input(t *testing.T) {
	// 2-input LUT (smallest possible)
	orTable := []int{0, 1, 1, 1}
	lut := NewLUT(2, orTable)

	if got := lut.Evaluate([]int{0, 0}); got != 0 {
		t.Errorf("OR(0,0) = %d, want 0", got)
	}
	if got := lut.Evaluate([]int{1, 0}); got != 1 {
		t.Errorf("OR(1,0) = %d, want 1", got)
	}
}

func TestLUT_Invalid(t *testing.T) {
	assertPanics(t, "k=1", func() { NewLUT(1, nil) })
	assertPanics(t, "k=7", func() { NewLUT(7, nil) })

	lut := NewLUT(4, nil)
	assertPanics(t, "wrong table len", func() { lut.Configure([]int{0, 0}) })
	assertPanics(t, "bad table bit", func() {
		bad := make([]int, 16)
		bad[0] = 2
		lut.Configure(bad)
	})
	assertPanics(t, "wrong input len", func() { lut.Evaluate([]int{0, 0}) })
	assertPanics(t, "bad input bit", func() { lut.Evaluate([]int{0, 0, 2, 0}) })
}
