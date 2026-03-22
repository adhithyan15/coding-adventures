// Package fpga implements a simplified but structurally accurate FPGA
// (Field-Programmable Gate Array) model: LUTs, Slices, CLBs, switch
// matrices, I/O blocks, bitstream configuration, and the top-level fabric.
//
// # What is an FPGA?
//
// An FPGA is a chip containing a grid of programmable logic blocks connected
// by a configurable routing fabric. By loading a bitstream (configuration
// data), the same physical chip can become any digital circuit — a CPU, a
// signal processor, a network switch, or anything that fits within its
// resources.
//
// # Package organization
//
//   - lut.go: Look-Up Table — the atom of programmable logic
//   - slice.go: Slice — 2 LUTs + 2 flip-flops + carry chain
//   - clb.go: Configurable Logic Block — 2 slices
//   - switch_matrix.go: Programmable routing crossbar
//   - io_block.go: Bidirectional I/O pad
//   - bitstream.go: Configuration data structures
//   - fabric.go: Top-level FPGA model
package fpga

import (
	"fmt"

	blockram "github.com/adhithyan15/coding-adventures/code/packages/go/block-ram"
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// =========================================================================
// Look-Up Table (LUT) — The Atom of Programmable Logic
// =========================================================================
//
// A Look-Up Table is the fundamental building block of every FPGA. The key
// insight behind programmable logic is deceptively simple:
//
//	A truth table IS a program.
//
// Any boolean function of K inputs can be described by a truth table with
// 2^K entries. A K-input LUT stores that truth table in SRAM and uses a
// MUX tree to select the correct output for any combination of inputs.
//
// This means a single LUT can implement ANY boolean function of K variables:
// AND, OR, XOR, majority vote, parity — anything. To "reprogram" the LUT,
// you just load a different truth table into the SRAM.
//
// How it works — a 4-input LUT (K=4) has:
//   - 16 SRAM cells (2^4 = 16 truth table entries)
//   - A 16-to-1 MUX tree (built from 2:1 MUXes)
//   - 4 input signals that act as MUX select lines
//
// The truth table index is computed as:
//
//	index = I0 + 2*I1 + 4*I2 + 8*I3  (binary number with I0 as LSB)
//
// Then the MUX tree selects SRAM[index] as the output.

// LUT is a K-input Look-Up Table — the atom of programmable logic.
//
// A LUT stores a truth table in SRAM cells and uses a MUX tree to
// select the output based on input signals. It can implement ANY
// boolean function of K variables.
type LUT struct {
	k    int
	size int
	sram []*blockram.SRAMCell
}

// NewLUT creates a K-input LUT. K must be between 2 and 6.
//
// If truthTable is non-nil, it is used to program the LUT immediately.
// Otherwise all entries default to 0.
func NewLUT(k int, truthTable []int) *LUT {
	if k < 2 || k > 6 {
		panic(fmt.Sprintf("fpga: LUT k must be between 2 and 6, got %d", k))
	}

	size := 1 << k
	sram := make([]*blockram.SRAMCell, size)
	for i := 0; i < size; i++ {
		sram[i] = blockram.NewSRAMCell()
	}

	lut := &LUT{k: k, size: size, sram: sram}
	if truthTable != nil {
		lut.Configure(truthTable)
	}
	return lut
}

// Configure loads a new truth table (reprograms the LUT).
//
// The truthTable must have exactly 2^k entries, each 0 or 1.
func (l *LUT) Configure(truthTable []int) {
	if len(truthTable) != l.size {
		panic(fmt.Sprintf("fpga: LUT truthTable length %d does not match 2^k = %d", len(truthTable), l.size))
	}

	for i, bit := range truthTable {
		if bit != 0 && bit != 1 {
			panic(fmt.Sprintf("fpga: truthTable[%d] must be 0 or 1, got %d", i, bit))
		}
	}

	for i, bit := range truthTable {
		l.sram[i].Write(1, bit)
	}
}

// Evaluate computes the LUT output for the given inputs.
//
// Uses a MUX tree (via MuxN) to select the correct truth table entry
// based on the input signals.
//
// The inputs slice must have exactly k elements, each 0 or 1.
// inputs[0] = I0 (LSB of truth table index).
func (l *LUT) Evaluate(inputs []int) int {
	if len(inputs) != l.k {
		panic(fmt.Sprintf("fpga: LUT inputs length %d does not match k = %d", len(inputs), l.k))
	}

	for i, bit := range inputs {
		if bit != 0 && bit != 1 {
			panic(fmt.Sprintf("fpga: inputs[%d] must be 0 or 1, got %d", i, bit))
		}
	}

	// Read all SRAM cells to form the MUX data inputs
	data := make([]int, l.size)
	for i, cell := range l.sram {
		val := cell.Read(1) // word_line=1 always returns non-nil
		data[i] = *val
	}

	// Use MUX tree to select the output
	return logicgates.MuxN(data, inputs)
}

// K returns the number of inputs.
func (l *LUT) K() int { return l.k }

// TruthTable returns a copy of the current truth table.
func (l *LUT) TruthTable() []int {
	result := make([]int, l.size)
	for i, cell := range l.sram {
		val := cell.Read(1)
		result[i] = *val
	}
	return result
}
