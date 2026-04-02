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
func validateBit(value int, name string) {
	if value != 0 && value != 1 {
		panic(fmt.Sprintf("blockram: %s must be 0 or 1, got %d", name, value))
	}
}

// =========================================================================
// SRAMCell — Single-Bit Storage
// =========================================================================

// SRAMCell is a single-bit storage element modeled at the gate level.
type SRAMCell struct {
	value int
}

// NewSRAMCell creates an SRAM cell initialized to 0.
func NewSRAMCell() *SRAMCell {
	result, _ := StartNew[*SRAMCell]("block-ram.NewSRAMCell", nil,
		func(op *Operation[*SRAMCell], rf *ResultFactory[*SRAMCell]) *OperationResult[*SRAMCell] {
			return rf.Generate(true, false, &SRAMCell{value: 0})
		}).GetResult()
	return result
}

// Read returns the stored bit if the cell is selected (word_line=1).
// When word_line=0, returns nil. When word_line=1, returns a pointer to the stored value.
func (c *SRAMCell) Read(wordLine int) *int {
	result, _ := StartNew[*int]("block-ram.SRAMCell.Read", nil,
		func(op *Operation[*int], rf *ResultFactory[*int]) *OperationResult[*int] {
			op.AddProperty("wordLine", wordLine)
			validateBit(wordLine, "wordLine")
			if wordLine == 0 {
				return rf.Generate(true, false, nil)
			}
			v := c.value
			return rf.Generate(true, false, &v)
		}).PanicOnUnexpected().GetResult()
	return result
}

// Write stores a bit in the cell if selected (word_line=1).
func (c *SRAMCell) Write(wordLine, bitLine int) {
	_, _ = StartNew[struct{}]("block-ram.SRAMCell.Write", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("wordLine", wordLine)
			op.AddProperty("bitLine", bitLine)
			validateBit(wordLine, "wordLine")
			validateBit(bitLine, "bitLine")
			if wordLine == 1 {
				c.value = bitLine
			}
			return rf.Generate(true, false, struct{}{})
		}).PanicOnUnexpected().GetResult()
}

// Value returns the current stored value (for inspection/debugging).
func (c *SRAMCell) Value() int {
	result, _ := StartNew[int]("block-ram.SRAMCell.Value", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, c.value)
		}).GetResult()
	return result
}

// =========================================================================
// SRAMArray — 2D Grid of SRAM Cells
// =========================================================================

// SRAMArray is a 2D grid of SRAM cells with row/column addressing.
type SRAMArray struct {
	rows  int
	cols  int
	cells [][]*SRAMCell
}

// shapePair holds the return value of Shape() to pass through StartNew.
type shapePair struct {
	rows int
	cols int
}

// NewSRAMArray creates an SRAM array initialized to all zeros.
// Panics if rows or cols < 1.
func NewSRAMArray(rows, cols int) *SRAMArray {
	result, _ := StartNew[*SRAMArray]("block-ram.NewSRAMArray", nil,
		func(op *Operation[*SRAMArray], rf *ResultFactory[*SRAMArray]) *OperationResult[*SRAMArray] {
			op.AddProperty("rows", rows)
			op.AddProperty("cols", cols)
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
					cells[r][c] = &SRAMCell{value: 0}
				}
			}
			return rf.Generate(true, false, &SRAMArray{rows: rows, cols: cols, cells: cells})
		}).PanicOnUnexpected().GetResult()
	return result
}

// Read reads all columns of a row.
// Panics if row is out of range.
func (a *SRAMArray) Read(row int) []int {
	result, _ := StartNew[[]int]("block-ram.SRAMArray.Read", nil,
		func(op *Operation[[]int], rf *ResultFactory[[]int]) *OperationResult[[]int] {
			op.AddProperty("row", row)
			a.validateRow(row)
			out := make([]int, a.cols)
			for c, cell := range a.cells[row] {
				val := cell.Read(1)
				out[c] = *val
			}
			return rf.Generate(true, false, out)
		}).PanicOnUnexpected().GetResult()
	return result
}

// Write writes data to a row.
// Panics if row is out of range or data length does not match cols.
func (a *SRAMArray) Write(row int, data []int) {
	_, _ = StartNew[struct{}]("block-ram.SRAMArray.Write", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("row", row)
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
			return rf.Generate(true, false, struct{}{})
		}).PanicOnUnexpected().GetResult()
}

// Shape returns the array dimensions as (rows, cols).
func (a *SRAMArray) Shape() (int, int) {
	result, _ := StartNew[shapePair]("block-ram.SRAMArray.Shape", shapePair{},
		func(op *Operation[shapePair], rf *ResultFactory[shapePair]) *OperationResult[shapePair] {
			return rf.Generate(true, false, shapePair{rows: a.rows, cols: a.cols})
		}).GetResult()
	return result.rows, result.cols
}

// validateRow checks that row index is in range.
func (a *SRAMArray) validateRow(row int) {
	if row < 0 || row >= a.rows {
		panic(fmt.Sprintf("blockram: SRAMArray row %d out of range [0, %d]", row, a.rows-1))
	}
}
