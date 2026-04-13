package logicgates

import (
	"testing"
)

// =========================================================================
// Test Helpers
// =========================================================================

// gateTest defines a single row in a truth table test.
type gateTest struct {
	a, b     int // inputs
	expected int // expected output
}

// unaryGateTest defines a single row in a unary truth table test.
type unaryGateTest struct {
	a        int
	expected int
}

// runGateTests runs a truth table test for a binary gate function.
func runGateTests(t *testing.T, name string, fn func(int, int) int, tests []gateTest) {
	t.Helper()
	for _, tc := range tests {
		got := fn(tc.a, tc.b)
		if got != tc.expected {
			t.Errorf("%s(%d, %d) = %d, want %d", name, tc.a, tc.b, got, tc.expected)
		}
	}
}

// assertPanics verifies that a function panics with a message containing substr.
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
// AND Gate Tests
// =========================================================================

func TestAND(t *testing.T) {
	// Full truth table for AND:
	//   0 AND 0 = 0
	//   0 AND 1 = 0
	//   1 AND 0 = 0
	//   1 AND 1 = 1
	runGateTests(t, "AND", AND, []gateTest{
		{0, 0, 0},
		{0, 1, 0},
		{1, 0, 0},
		{1, 1, 1},
	})
}

// =========================================================================
// OR Gate Tests
// =========================================================================

func TestOR(t *testing.T) {
	runGateTests(t, "OR", OR, []gateTest{
		{0, 0, 0},
		{0, 1, 1},
		{1, 0, 1},
		{1, 1, 1},
	})
}

// =========================================================================
// NOT Gate Tests
// =========================================================================

func TestNOT(t *testing.T) {
	tests := []unaryGateTest{
		{0, 1},
		{1, 0},
	}
	for _, tc := range tests {
		got := NOT(tc.a)
		if got != tc.expected {
			t.Errorf("NOT(%d) = %d, want %d", tc.a, got, tc.expected)
		}
	}
}

// =========================================================================
// XOR Gate Tests
// =========================================================================

func TestXOR(t *testing.T) {
	runGateTests(t, "XOR", XOR, []gateTest{
		{0, 0, 0},
		{0, 1, 1},
		{1, 0, 1},
		{1, 1, 0},
	})
}

// =========================================================================
// NAND Gate Tests
// =========================================================================

func TestNAND(t *testing.T) {
	runGateTests(t, "NAND", NAND, []gateTest{
		{0, 0, 1},
		{0, 1, 1},
		{1, 0, 1},
		{1, 1, 0},
	})
}

// =========================================================================
// NOR Gate Tests
// =========================================================================

func TestNOR(t *testing.T) {
	runGateTests(t, "NOR", NOR, []gateTest{
		{0, 0, 1},
		{0, 1, 0},
		{1, 0, 0},
		{1, 1, 0},
	})
}

// =========================================================================
// XNOR Gate Tests
// =========================================================================

func TestXNOR(t *testing.T) {
	runGateTests(t, "XNOR", XNOR, []gateTest{
		{0, 0, 1},
		{0, 1, 0},
		{1, 0, 0},
		{1, 1, 1},
	})
}

// =========================================================================
// NAND-Derived Gate Tests — Proving Functional Completeness
// =========================================================================
//
// Each NAND-derived function must produce identical output to its
// direct counterpart for ALL input combinations. This is the proof
// that NAND is functionally complete.

func TestNAND_NOT(t *testing.T) {
	for _, a := range []int{0, 1} {
		got := NAND_NOT(a)
		want := NOT(a)
		if got != want {
			t.Errorf("NAND_NOT(%d) = %d, want %d (same as NOT)", a, got, want)
		}
	}
}

func TestNAND_AND(t *testing.T) {
	// Verify NAND_AND matches AND for all input combinations
	for _, a := range []int{0, 1} {
		for _, b := range []int{0, 1} {
			got := NAND_AND(a, b)
			want := AND(a, b)
			if got != want {
				t.Errorf("NAND_AND(%d, %d) = %d, want %d (same as AND)", a, b, got, want)
			}
		}
	}
}

func TestNAND_OR(t *testing.T) {
	for _, a := range []int{0, 1} {
		for _, b := range []int{0, 1} {
			got := NAND_OR(a, b)
			want := OR(a, b)
			if got != want {
				t.Errorf("NAND_OR(%d, %d) = %d, want %d (same as OR)", a, b, got, want)
			}
		}
	}
}

func TestNAND_XOR(t *testing.T) {
	for _, a := range []int{0, 1} {
		for _, b := range []int{0, 1} {
			got := NAND_XOR(a, b)
			want := XOR(a, b)
			if got != want {
				t.Errorf("NAND_XOR(%d, %d) = %d, want %d (same as XOR)", a, b, got, want)
			}
		}
	}
}

// =========================================================================
// Multi-Input Gate Tests
// =========================================================================

func TestANDn(t *testing.T) {
	tests := []struct {
		name     string
		inputs   []int
		expected int
	}{
		// 2 inputs — same as basic AND
		{"all zeros", []int{0, 0}, 0},
		{"all ones", []int{1, 1}, 1},
		{"mixed", []int{1, 0}, 0},

		// 3 inputs
		{"three ones", []int{1, 1, 1}, 1},
		{"three with zero", []int{1, 1, 0}, 0},
		{"three all zero", []int{0, 0, 0}, 0},

		// 4 inputs — wide AND gate
		{"four ones", []int{1, 1, 1, 1}, 1},
		{"four with one zero", []int{1, 1, 0, 1}, 0},

		// 5 inputs
		{"five ones", []int{1, 1, 1, 1, 1}, 1},
		{"five with zero at start", []int{0, 1, 1, 1, 1}, 0},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := ANDn(tc.inputs...)
			if got != tc.expected {
				t.Errorf("ANDn(%v) = %d, want %d", tc.inputs, got, tc.expected)
			}
		})
	}
}

func TestORn(t *testing.T) {
	tests := []struct {
		name     string
		inputs   []int
		expected int
	}{
		{"all zeros", []int{0, 0}, 0},
		{"all ones", []int{1, 1}, 1},
		{"mixed", []int{0, 1}, 1},

		{"three zeros", []int{0, 0, 0}, 0},
		{"three with one", []int{0, 0, 1}, 1},

		{"four zeros", []int{0, 0, 0, 0}, 0},
		{"four with one", []int{0, 0, 1, 0}, 1},

		{"five zeros", []int{0, 0, 0, 0, 0}, 0},
		{"five with one at end", []int{0, 0, 0, 0, 1}, 1},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := ORn(tc.inputs...)
			if got != tc.expected {
				t.Errorf("ORn(%v) = %d, want %d", tc.inputs, got, tc.expected)
			}
		})
	}
}

// =========================================================================
// Input Validation Tests — Ensuring Invalid Inputs Are Rejected
// =========================================================================
//
// In real hardware, feeding invalid voltages into a gate causes
// undefined behavior. Our software gates must panic on invalid input
// to catch programming errors early.

func TestAND_InvalidInput(t *testing.T) {
	assertPanics(t, "AND(2,0)", func() { AND(2, 0) })
	assertPanics(t, "AND(0,2)", func() { AND(0, 2) })
	assertPanics(t, "AND(-1,0)", func() { AND(-1, 0) })
	assertPanics(t, "AND(0,-1)", func() { AND(0, -1) })
}

func TestOR_InvalidInput(t *testing.T) {
	assertPanics(t, "OR(2,0)", func() { OR(2, 0) })
	assertPanics(t, "OR(0,3)", func() { OR(0, 3) })
}

func TestNOT_InvalidInput(t *testing.T) {
	assertPanics(t, "NOT(2)", func() { NOT(2) })
	assertPanics(t, "NOT(-1)", func() { NOT(-1) })
}

func TestXOR_InvalidInput(t *testing.T) {
	assertPanics(t, "XOR(2,0)", func() { XOR(2, 0) })
	assertPanics(t, "XOR(0,5)", func() { XOR(0, 5) })
}

func TestNAND_InvalidInput(t *testing.T) {
	assertPanics(t, "NAND(2,0)", func() { NAND(2, 0) })
	assertPanics(t, "NAND(0,2)", func() { NAND(0, 2) })
}

func TestNOR_InvalidInput(t *testing.T) {
	assertPanics(t, "NOR(2,0)", func() { NOR(2, 0) })
	assertPanics(t, "NOR(0,2)", func() { NOR(0, 2) })
}

func TestXNOR_InvalidInput(t *testing.T) {
	assertPanics(t, "XNOR(2,0)", func() { XNOR(2, 0) })
	assertPanics(t, "XNOR(0,2)", func() { XNOR(0, 2) })
}

func TestNAND_NOT_InvalidInput(t *testing.T) {
	assertPanics(t, "NAND_NOT(2)", func() { NAND_NOT(2) })
}

func TestNAND_AND_InvalidInput(t *testing.T) {
	assertPanics(t, "NAND_AND(2,0)", func() { NAND_AND(2, 0) })
	assertPanics(t, "NAND_AND(0,2)", func() { NAND_AND(0, 2) })
}

func TestNAND_OR_InvalidInput(t *testing.T) {
	assertPanics(t, "NAND_OR(2,0)", func() { NAND_OR(2, 0) })
	assertPanics(t, "NAND_OR(0,2)", func() { NAND_OR(0, 2) })
}

func TestNAND_XOR_InvalidInput(t *testing.T) {
	assertPanics(t, "NAND_XOR(2,0)", func() { NAND_XOR(2, 0) })
	assertPanics(t, "NAND_XOR(0,2)", func() { NAND_XOR(0, 2) })
}

func TestANDn_InvalidInput(t *testing.T) {
	// Too few inputs
	assertPanics(t, "ANDn(single)", func() { ANDn(1) })
	assertPanics(t, "ANDn(empty)", func() { ANDn() })

	// Invalid bit values
	assertPanics(t, "ANDn(1,2)", func() { ANDn(1, 2) })
	assertPanics(t, "ANDn(3,0)", func() { ANDn(3, 0) })
}

func TestORn_InvalidInput(t *testing.T) {
	assertPanics(t, "ORn(single)", func() { ORn(0) })
	assertPanics(t, "ORn(empty)", func() { ORn() })
	assertPanics(t, "ORn(0,2)", func() { ORn(0, 2) })
	assertPanics(t, "ORn(5,0)", func() { ORn(5, 0) })
}

// =========================================================================
// XORn Gate Tests
// =========================================================================

func TestXORn(t *testing.T) {
	// Two-input: should match XOR truth table
	if XORn(0, 0) != 0 {
		t.Error("XORn(0,0) should be 0")
	}
	if XORn(0, 1) != 1 {
		t.Error("XORn(0,1) should be 1")
	}
	if XORn(1, 0) != 1 {
		t.Error("XORn(1,0) should be 1")
	}
	if XORn(1, 1) != 0 {
		t.Error("XORn(1,1) should be 0")
	}

	// Three inputs: XOR(XOR(a,b),c)
	if XORn(1, 0, 0) != 1 {
		t.Error("XORn(1,0,0) should be 1 (one 1-bit → odd)")
	}
	if XORn(1, 1, 0) != 0 {
		t.Error("XORn(1,1,0) should be 0 (two 1-bits → even)")
	}
	if XORn(1, 1, 1) != 1 {
		t.Error("XORn(1,1,1) should be 1 (three 1-bits → odd)")
	}

	// Eight inputs — parity of a full byte
	// 0b00000011 = 3 (two 1-bits → even parity → XORn = 0)
	if XORn(1, 1, 0, 0, 0, 0, 0, 0) != 0 {
		t.Error("XORn(0b00000011) should be 0 (even parity)")
	}
	// 0b00000111 = 7 (three 1-bits → odd parity → XORn = 1)
	if XORn(1, 1, 1, 0, 0, 0, 0, 0) != 1 {
		t.Error("XORn(0b00000111) should be 1 (odd parity)")
	}
	// All ones (8 ones → even → XORn = 0)
	if XORn(1, 1, 1, 1, 1, 1, 1, 1) != 0 {
		t.Error("XORn(all 1s) should be 0 (even parity)")
	}
}

func TestXORn_InvalidInput(t *testing.T) {
	assertPanics(t, "XORn(single)", func() { XORn(0) })
	assertPanics(t, "XORn(empty)", func() { XORn() })
	assertPanics(t, "XORn(0,2)", func() { XORn(0, 2) })
	assertPanics(t, "XORn(5,0)", func() { XORn(5, 0) })
}
