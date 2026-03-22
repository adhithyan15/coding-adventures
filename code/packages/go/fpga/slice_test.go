package fpga

import (
	"testing"
)

// =========================================================================
// Slice Tests
// =========================================================================

func TestSlice_Combinational(t *testing.T) {
	s := NewSlice(4)

	// LUT A = AND, LUT B = XOR
	andTT := make([]int, 16)
	andTT[3] = 1
	xorTT := make([]int, 16)
	xorTT[1] = 1
	xorTT[2] = 1

	s.Configure(andTT, xorTT, false, false, false)

	out := s.Evaluate([]int{1, 1, 0, 0}, []int{1, 0, 0, 0}, 0, 0)

	if out.OutputA != 1 {
		t.Errorf("Slice AND(1,1) output_a = %d, want 1", out.OutputA)
	}
	if out.OutputB != 1 {
		t.Errorf("Slice XOR(1,0) output_b = %d, want 1", out.OutputB)
	}
	if out.CarryOut != 0 {
		t.Errorf("Slice carry_out = %d, want 0 (disabled)", out.CarryOut)
	}
}

func TestSlice_WithFlipFlops(t *testing.T) {
	s := NewSlice(4)

	andTT := make([]int, 16)
	andTT[3] = 1

	s.Configure(andTT, andTT, true, true, false)

	// Clock HIGH: master captures LUT output
	s.Evaluate([]int{1, 1, 0, 0}, []int{1, 1, 0, 0}, 1, 0)

	// Clock LOW: slave outputs → registered output should appear
	out := s.Evaluate([]int{1, 1, 0, 0}, []int{1, 1, 0, 0}, 0, 0)

	// With FF enabled, the output is the registered (flip-flop) value
	// After HIGH then LOW, the flip-flop should have captured the LUT output of 1
	if out.OutputA != 1 {
		t.Errorf("Slice FF A output = %d, want 1", out.OutputA)
	}
	if out.OutputB != 1 {
		t.Errorf("Slice FF B output = %d, want 1", out.OutputB)
	}
}

func TestSlice_CarryChain(t *testing.T) {
	s := NewSlice(4)

	// LUT A and B both output 1 when I0=1, I1=1
	andTT := make([]int, 16)
	andTT[3] = 1

	s.Configure(andTT, andTT, false, false, true)

	// With both LUTs outputting 1 and carry_in=0:
	// carry_out = (1 AND 1) OR (0 AND (1 XOR 1)) = 1 OR 0 = 1
	out := s.Evaluate([]int{1, 1, 0, 0}, []int{1, 1, 0, 0}, 0, 0)
	if out.CarryOut != 1 {
		t.Errorf("Carry chain output = %d, want 1", out.CarryOut)
	}

	// With LUT A=1, LUT B=0, carry_in=1:
	// carry_out = (1 AND 0) OR (1 AND (1 XOR 0)) = 0 OR 1 = 1
	out = s.Evaluate([]int{1, 1, 0, 0}, []int{0, 0, 0, 0}, 0, 1)
	if out.CarryOut != 1 {
		t.Errorf("Carry chain propagate = %d, want 1", out.CarryOut)
	}

	// With LUT A=0, LUT B=0, carry_in=1:
	// carry_out = (0 AND 0) OR (1 AND (0 XOR 0)) = 0 OR 0 = 0
	out = s.Evaluate([]int{0, 0, 0, 0}, []int{0, 0, 0, 0}, 0, 1)
	if out.CarryOut != 0 {
		t.Errorf("Carry chain block = %d, want 0", out.CarryOut)
	}
}

func TestSlice_Properties(t *testing.T) {
	s := NewSlice(4)
	if s.K() != 4 {
		t.Errorf("K() = %d, want 4", s.K())
	}
	if s.LutA() == nil {
		t.Error("LutA() is nil")
	}
	if s.LutB() == nil {
		t.Error("LutB() is nil")
	}
}

// =========================================================================
// CLB Tests
// =========================================================================

func TestCLB_Evaluate(t *testing.T) {
	clb := NewCLB(4)

	// Configure both slices with AND gates
	andTT := make([]int, 16)
	andTT[3] = 1

	clb.Slice0().Configure(andTT, andTT, false, false, false)
	clb.Slice1().Configure(andTT, andTT, false, false, false)

	out := clb.Evaluate(
		[]int{1, 1, 0, 0}, []int{1, 1, 0, 0}, // slice 0
		[]int{0, 1, 0, 0}, []int{1, 0, 0, 0}, // slice 1
		0, 0,
	)

	if out.Slice0.OutputA != 1 {
		t.Errorf("Slice0 A = %d, want 1", out.Slice0.OutputA)
	}
	if out.Slice1.OutputA != 0 {
		t.Errorf("Slice1 A = %d, want 0", out.Slice1.OutputA)
	}
}

func TestCLB_CarryChainBetweenSlices(t *testing.T) {
	clb := NewCLB(4)

	// Both slices: LUT A and LUT B are AND gates with carry enabled
	andTT := make([]int, 16)
	andTT[3] = 1

	clb.Slice0().Configure(andTT, andTT, false, false, true)
	clb.Slice1().Configure(andTT, andTT, false, false, true)

	// Slice 0: both LUTs output 1 → carry_out = 1
	// Slice 1: both LUTs output 1, carry_in = 1 (from slice 0)
	// carry_out = (1 AND 1) OR (1 AND (1 XOR 1)) = 1 OR 0 = 1
	out := clb.Evaluate(
		[]int{1, 1, 0, 0}, []int{1, 1, 0, 0},
		[]int{1, 1, 0, 0}, []int{1, 1, 0, 0},
		0, 0,
	)

	if out.Slice0.CarryOut != 1 {
		t.Errorf("Slice0 carry = %d, want 1", out.Slice0.CarryOut)
	}
	if out.Slice1.CarryOut != 1 {
		t.Errorf("Slice1 carry = %d, want 1", out.Slice1.CarryOut)
	}
}

func TestCLB_Properties(t *testing.T) {
	clb := NewCLB(4)
	if clb.K() != 4 {
		t.Errorf("K() = %d, want 4", clb.K())
	}
	if clb.Slice0() == nil {
		t.Error("Slice0() is nil")
	}
	if clb.Slice1() == nil {
		t.Error("Slice1() is nil")
	}
}
