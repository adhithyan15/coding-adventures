// Package blockram implements SRAM cells, arrays, and RAM modules — the
// memory building blocks for CPUs, caches, and FPGA Block RAM.
//
// # What is SRAM?
//
// SRAM (Static Random-Access Memory) is the fastest type of memory in a
// computer. It is used for CPU caches (L1/L2/L3), register files, and
// FPGA Block RAM. "Static" means the memory holds its value as long as
// power is supplied — unlike DRAM, which must be periodically refreshed.
//
// # The SRAM Cell — 6 Transistors Holding 1 Bit
//
// In real hardware, each SRAM cell uses 6 transistors:
//   - 2 cross-coupled inverters forming a bistable latch (stores the bit)
//   - 2 access transistors controlled by the word line (gates read/write)
//
// We model this at the gate level:
//   - Cross-coupled inverters = two NOT gates in a feedback loop
//   - Access transistors = AND gates that pass data only when word_line=1
//
// The cell has three operations:
//   - Hold (word_line=0): Access transistors block external access.
//     The inverter loop maintains the stored value indefinitely.
//   - Read (word_line=1): Access transistors open. The stored value
//     appears on the bit lines without disturbing it.
//   - Write (word_line=1 + drive bit lines): The external driver
//     overpowers the internal inverters, forcing a new value.
//
// # From Cell to Array
//
// A RAM chip is a 2D grid of SRAM cells. To access a specific cell:
//  1. A row decoder converts address bits into a one-hot word line signal
//  2. A column MUX selects which columns to read/write
//
// This file provides:
//   - SRAMCell: single-bit storage at the gate level
//   - SRAMArray: 2D grid with row/column addressing
package blockram

import "fmt"

// =========================================================================
// Input Validation
// =========================================================================

// validateBit checks that a value is a valid binary digit (0 or 1).
//
// In digital electronics, a "bit" is a signal that is either LOW (0) or
// HIGH (1). Anything else is meaningless. Real hardware enforces this
// through voltage thresholds; we enforce it with a runtime check.
//
// This is duplicated from logic-gates to avoid importing internal helpers.
func validateBit(value int, name string) {
	if value != 0 && value != 1 {
		panic(fmt.Sprintf("blockram: %s must be 0 or 1, got %d", name, value))
	}
}

// =========================================================================
// SRAMCell — Single-Bit Storage
// =========================================================================

// SRAMCell is a single-bit storage element modeled at the gate level.
//
// Internally, this is a pair of cross-coupled inverters (forming a
// bistable latch) gated by access transistors controlled by the word line.
//
// In our simulation, we model the steady-state behavior directly rather
// than simulating individual gate delays:
//   - word_line=0: cell is isolated, value is retained
//   - word_line=1, reading: value is output
//   - word_line=1, writing: new value overwrites stored value
//
// This matches the real behavior of a 6T SRAM cell while keeping the
// simulation fast enough to model arrays of thousands of cells.
type SRAMCell struct {
	value int
}

// NewSRAMCell creates an SRAM cell initialized to 0.
//
// The initial state of 0 represents the cell after power-on reset.
// In real hardware, SRAM cells power up in an indeterminate state,
// but we initialize to 0 for predictability in simulation.
func NewSRAMCell() *SRAMCell {
	return &SRAMCell{value: 0}
}

// Read returns the stored bit if the cell is selected (word_line=1).
//
// When word_line=0 (cell not selected), returns nil — the cell's access
// transistors are closed, so no output appears on the bit line.
// When word_line=1, returns a pointer to the stored value.
func (c *SRAMCell) Read(wordLine int) *int {
	validateBit(wordLine, "wordLine")

	if wordLine == 0 {
		return nil
	}

	result := c.value
	return &result
}

// Write stores a bit in the cell if selected (word_line=1).
//
// When word_line=1, the access transistors open and the external bit_line
// driver overpowers the internal inverter loop, forcing the cell to store
// the new value.
//
// When word_line=0, the access transistors are closed and the write has
// no effect — the cell retains its previous value.
func (c *SRAMCell) Write(wordLine, bitLine int) {
	validateBit(wordLine, "wordLine")
	validateBit(bitLine, "bitLine")

	if wordLine == 1 {
		c.value = bitLine
	}
}

// Value returns the current stored value (for inspection/debugging).
func (c *SRAMCell) Value() int {
	return c.value
}

// =========================================================================
// SRAMArray — 2D Grid of SRAM Cells
// =========================================================================

// SRAMArray is a 2D grid of SRAM cells with row/column addressing.
//
// An SRAM array organizes cells into rows and columns:
//   - Each row shares a word line (activated by the row decoder)
//   - Each column shares a bit line (carries data in/out)
//
// To read: activate a row's word line → all cells in that row
// output their values onto their respective bit lines.
//
// To write: activate a row's word line and drive the bit lines
// with the desired data → all cells in that row store the new values.
//
// Memory map (4x4 array example):
//
//	Row 0 (WL0): [Cell00] [Cell01] [Cell02] [Cell03]
//	Row 1 (WL1): [Cell10] [Cell11] [Cell12] [Cell13]
//	Row 2 (WL2): [Cell20] [Cell21] [Cell22] [Cell23]
//	Row 3 (WL3): [Cell30] [Cell31] [Cell32] [Cell33]
type SRAMArray struct {
	rows  int
	cols  int
	cells [][]*SRAMCell
}

// NewSRAMArray creates an SRAM array initialized to all zeros.
//
// Panics if rows or cols < 1.
func NewSRAMArray(rows, cols int) *SRAMArray {
	if rows < 1 {
		panic(fmt.Sprintf("blockram: SRAMArray rows must be >= 1, got %d", rows))
	}
	if cols < 1 {
		panic(fmt.Sprintf("blockram: SRAMArray cols must be >= 1, got %d", cols))
	}

	cells := make([][]*SRAMCell, rows)
	for r := 0; r < rows; r++ {
		cells[r] = make([]*SRAMCell, cols)
		for c := 0; c < cols; c++ {
			cells[r][c] = NewSRAMCell()
		}
	}

	return &SRAMArray{rows: rows, cols: cols, cells: cells}
}

// Read reads all columns of a row.
//
// Activates the word line for the given row, causing all cells in that
// row to output their stored values.
//
// Panics if row is out of range.
func (a *SRAMArray) Read(row int) []int {
	a.validateRow(row)

	result := make([]int, a.cols)
	for c, cell := range a.cells[row] {
		val := cell.Read(1) // word_line=1 always returns non-nil
		result[c] = *val
	}
	return result
}

// Write writes data to a row.
//
// Activates the word line for the given row and drives the bit lines
// with the given data, storing values in all cells of the row.
//
// Panics if row is out of range or data length does not match cols.
func (a *SRAMArray) Write(row int, data []int) {
	a.validateRow(row)

	if len(data) != a.cols {
		panic(fmt.Sprintf("blockram: SRAMArray Write data length %d does not match cols %d", len(data), a.cols))
	}

	for i, bit := range data {
		validateBit(bit, fmt.Sprintf("data[%d]", i))
	}

	for c, bit := range data {
		a.cells[row][c].Write(1, bit)
	}
}

// Shape returns the array dimensions as (rows, cols).
func (a *SRAMArray) Shape() (int, int) {
	return a.rows, a.cols
}

// validateRow checks that row index is in range.
func (a *SRAMArray) validateRow(row int) {
	if row < 0 || row >= a.rows {
		panic(fmt.Sprintf("blockram: SRAMArray row %d out of range [0, %d]", row, a.rows-1))
	}
}
