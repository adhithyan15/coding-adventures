package fpga

// =========================================================================
// Slice — The Building Block of a Configurable Logic Block (CLB)
// =========================================================================
//
// A slice is one "lane" inside a CLB. It combines:
//   - 2 LUTs (A and B) for combinational logic
//   - 2 D flip-flops for registered (sequential) outputs
//   - 2 output MUXes that choose between combinational or registered output
//   - Carry chain logic for fast arithmetic
//
// The output MUX is critical: it lets the same slice be used for both
// combinational circuits (bypass the flip-flop) and sequential circuits
// (register the LUT output on the clock edge).
//
// Slice Architecture:
//
//	inputs_a ──→ [LUT A] ──→ ┌─────────┐
//	                          │ MUX_A   │──→ output_a
//	               ┌─→ [FF A]→│(sel=ff_a)│
//	               │          └─────────┘
//	               │
//	inputs_b ──→ [LUT B] ──→ ┌─────────┐
//	                          │ MUX_B   │──→ output_b
//	               ┌─→ [FF B]→│(sel=ff_b)│
//	               │          └─────────┘
//	               │
//	carry_in ──→ [CARRY] ────────────────→ carry_out
//
//	clock ──────→ [FF A] [FF B]
//
// Carry Chain:
//
//	For arithmetic operations, the carry chain connects adjacent slices
//	to propagate carry bits without going through the general routing
//	fabric. Our carry chain computes:
//	  carry_out = (LUT_A_out AND LUT_B_out) OR (carry_in AND (LUT_A_out XOR LUT_B_out))
//
//	This is the standard full-adder carry equation.

import (
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// SliceOutput holds the output from a single slice evaluation.
type SliceOutput struct {
	OutputA  int // LUT A result (combinational or registered)
	OutputB  int // LUT B result (combinational or registered)
	CarryOut int // Carry chain output (0 if carry disabled)
}

// Slice is one slice of a CLB: 2 LUTs + 2 flip-flops + output MUXes + carry chain.
type Slice struct {
	lutA *LUT
	lutB *LUT
	k    int

	// Flip-flop state (master-slave D flip-flop)
	ffAState *logicgates.FlipFlopState
	ffBState *logicgates.FlipFlopState

	// Configuration
	ffAEnabled   bool
	ffBEnabled   bool
	carryEnabled bool
}

// NewSlice creates a new slice with the given number of LUT inputs.
func NewSlice(lutInputs int) *Slice {
	return &Slice{
		lutA:     NewLUT(lutInputs, nil),
		lutB:     NewLUT(lutInputs, nil),
		k:        lutInputs,
		ffAState: &logicgates.FlipFlopState{MasterQ: 0, MasterQBar: 1, SlaveQ: 0, SlaveQBar: 1},
		ffBState: &logicgates.FlipFlopState{MasterQ: 0, MasterQBar: 1, SlaveQ: 0, SlaveQBar: 1},
	}
}

// Configure sets up the slice's LUTs, flip-flops, and carry chain.
func (s *Slice) Configure(
	lutATable, lutBTable []int,
	ffAEnabled, ffBEnabled, carryEnabled bool,
) {
	s.lutA.Configure(lutATable)
	s.lutB.Configure(lutBTable)
	s.ffAEnabled = ffAEnabled
	s.ffBEnabled = ffBEnabled
	s.carryEnabled = carryEnabled

	// Reset flip-flop state on reconfiguration
	s.ffAState = &logicgates.FlipFlopState{MasterQ: 0, MasterQBar: 1, SlaveQ: 0, SlaveQBar: 1}
	s.ffBState = &logicgates.FlipFlopState{MasterQ: 0, MasterQBar: 1, SlaveQ: 0, SlaveQBar: 1}
}

// Evaluate evaluates the slice for one half-cycle.
//
// Parameters:
//   - inputsA: input bits for LUT A (length k)
//   - inputsB: input bits for LUT B (length k)
//   - clock: clock signal (0 or 1)
//   - carryIn: carry input from previous slice (default 0)
//
// Returns a SliceOutput with output_a, output_b, and carry_out.
func (s *Slice) Evaluate(inputsA, inputsB []int, clock, carryIn int) SliceOutput {
	// Evaluate LUTs (combinational — always computed)
	lutAOut := s.lutA.Evaluate(inputsA)
	lutBOut := s.lutB.Evaluate(inputsB)

	// Flip-flop A: route through if enabled
	var outputA int
	if s.ffAEnabled {
		qA, _, newState := logicgates.DFlipFlop(lutAOut, clock, s.ffAState)
		s.ffAState = newState
		// MUX: select registered output (ff enabled → sel=1 → pick d1=qA)
		outputA = logicgates.Mux2(lutAOut, qA, 1)
	} else {
		outputA = lutAOut
	}

	// Flip-flop B: route through if enabled
	var outputB int
	if s.ffBEnabled {
		qB, _, newState := logicgates.DFlipFlop(lutBOut, clock, s.ffBState)
		s.ffBState = newState
		outputB = logicgates.Mux2(lutBOut, qB, 1)
	} else {
		outputB = lutBOut
	}

	// Carry chain: standard full-adder carry equation
	//   carry_out = (A AND B) OR (carry_in AND (A XOR B))
	var carryOut int
	if s.carryEnabled {
		carryOut = logicgates.OR(
			logicgates.AND(lutAOut, lutBOut),
			logicgates.AND(carryIn, logicgates.XOR(lutAOut, lutBOut)),
		)
	}

	return SliceOutput{
		OutputA:  outputA,
		OutputB:  outputB,
		CarryOut: carryOut,
	}
}

// LutA returns LUT A (for inspection).
func (s *Slice) LutA() *LUT { return s.lutA }

// LutB returns LUT B (for inspection).
func (s *Slice) LutB() *LUT { return s.lutB }

// K returns the number of LUT inputs.
func (s *Slice) K() int { return s.k }
