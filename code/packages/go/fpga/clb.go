package fpga

// =========================================================================
// Configurable Logic Block (CLB) — The Core Compute Tile of an FPGA
// =========================================================================
//
// A CLB is the primary logic resource in an FPGA. It contains multiple
// slices, each with LUTs, flip-flops, and carry chains. CLBs are
// connected to each other through the routing fabric.
//
// Our CLB follows the Xilinx-style architecture with 2 slices:
//
//	┌──────────────────────────────────────────────┐
//	│                     CLB                       │
//	│                                               │
//	│  ┌─────────────────────┐                      │
//	│  │       Slice 0       │                      │
//	│  │  [LUT A] [LUT B]   │                      │
//	│  │  [FF A]  [FF B]    │                      │
//	│  │  [carry chain]      │                      │
//	│  └─────────┬───────────┘                      │
//	│            │ carry                             │
//	│  ┌─────────▼───────────┐                      │
//	│  │       Slice 1       │                      │
//	│  │  [LUT A] [LUT B]   │                      │
//	│  │  [FF A]  [FF B]    │                      │
//	│  │  [carry chain]      │                      │
//	│  └─────────────────────┘                      │
//	│                                               │
//	└──────────────────────────────────────────────┘
//
// The carry chain flows from slice 0 → slice 1, enabling fast multi-bit
// arithmetic within a single CLB.

// CLBOutput holds the output from a CLB evaluation.
type CLBOutput struct {
	Slice0 SliceOutput // Output from slice 0
	Slice1 SliceOutput // Output from slice 1
}

// CLB is a Configurable Logic Block containing 2 slices.
//
// The carry chain connects slice 0's carry_out to slice 1's carry_in,
// enabling fast multi-bit arithmetic.
type CLB struct {
	slice0 *Slice
	slice1 *Slice
	k      int
}

// NewCLB creates a new CLB with the given number of LUT inputs per slice.
func NewCLB(lutInputs int) *CLB {
	return &CLB{
		slice0: NewSlice(lutInputs),
		slice1: NewSlice(lutInputs),
		k:      lutInputs,
	}
}

// Slice0 returns the first slice.
func (c *CLB) Slice0() *Slice { return c.slice0 }

// Slice1 returns the second slice.
func (c *CLB) Slice1() *Slice { return c.slice1 }

// K returns the number of LUT inputs per slice.
func (c *CLB) K() int { return c.k }

// Evaluate evaluates both slices in the CLB.
//
// The carry chain flows: carryIn → slice0 → slice1.
//
// Parameters:
//   - slice0InputsA/B: inputs for slice 0's LUTs
//   - slice1InputsA/B: inputs for slice 1's LUTs
//   - clock: clock signal (0 or 1)
//   - carryIn: external carry input (default 0)
//
// Returns a CLBOutput containing both slices' outputs.
func (c *CLB) Evaluate(
	slice0InputsA, slice0InputsB []int,
	slice1InputsA, slice1InputsB []int,
	clock, carryIn int,
) CLBOutput {
	// Evaluate slice 0 first (carry chain starts here)
	out0 := c.slice0.Evaluate(slice0InputsA, slice0InputsB, clock, carryIn)

	// Slice 1 receives carry from slice 0
	out1 := c.slice1.Evaluate(slice1InputsA, slice1InputsB, clock, out0.CarryOut)

	return CLBOutput{Slice0: out0, Slice1: out1}
}
