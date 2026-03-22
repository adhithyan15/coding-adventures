package fpga

// =========================================================================
// I/O Block — Bidirectional Pad Connecting FPGA Internals to the Outside
// =========================================================================
//
// I/O blocks sit at the perimeter of the FPGA and provide the interface
// between the internal logic fabric and the external pins of the chip.
//
// Each I/O block can be configured in three modes:
//   - Input: External signal enters the FPGA (pad → internal)
//   - Output: Internal signal exits the FPGA (internal → pad)
//   - Tristate: Output is high-impedance (disconnected) when not enabled
//
// I/O Block Architecture:
//
//	External Pin (pad)
//	     │
//	     ▼
//	┌──────────────────┐
//	│    I/O Block      │
//	│                   │
//	│  ┌─────────────┐  │
//	│  │ Tri-State   │  │ ── output enable controls direction
//	│  │ Buffer      │  │
//	│  └──────┬──────┘  │
//	│         │         │
//	└──────────────────┘
//	     │
//	     ▼
//	To/From Internal Fabric

import (
	"fmt"

	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// IOMode represents the operating mode of an I/O block.
type IOMode int

const (
	// IOInput: pad drives internal signal (external → fabric).
	IOInput IOMode = iota
	// IOOutput: fabric drives pad (fabric → external).
	IOOutput
	// IOTristate: output is high-impedance (pad is disconnected).
	IOTristate
)

// String returns the string representation of the IOMode.
func (m IOMode) String() string {
	switch m {
	case IOInput:
		return "input"
	case IOOutput:
		return "output"
	case IOTristate:
		return "tristate"
	default:
		return fmt.Sprintf("IOMode(%d)", int(m))
	}
}

// IOBlock is a bidirectional I/O pad for the FPGA perimeter.
//
// Each I/O block connects one external pin to the internal fabric.
// The mode determines the direction of data flow.
type IOBlock struct {
	name          string
	mode          IOMode
	padValue      int // Signal on the external pad
	internalValue int // Signal on the fabric side
}

// NewIOBlock creates a new I/O block with the given name and mode.
//
// Panics if name is empty.
func NewIOBlock(name string, mode IOMode) *IOBlock {
	if name == "" {
		panic("fpga: IOBlock name must be a non-empty string")
	}

	return &IOBlock{
		name: name,
		mode: mode,
	}
}

// Configure changes the I/O block's operating mode.
func (io *IOBlock) Configure(mode IOMode) {
	io.mode = mode
}

// DrivePad drives the external pad with a signal (used in INPUT mode).
//
// Panics if value is not 0 or 1.
func (io *IOBlock) DrivePad(value int) {
	if value != 0 && value != 1 {
		panic(fmt.Sprintf("fpga: IOBlock value must be 0 or 1, got %d", value))
	}
	io.padValue = value
}

// DriveInternal drives the internal (fabric) side with a signal
// (used in OUTPUT mode).
//
// Panics if value is not 0 or 1.
func (io *IOBlock) DriveInternal(value int) {
	if value != 0 && value != 1 {
		panic(fmt.Sprintf("fpga: IOBlock value must be 0 or 1, got %d", value))
	}
	io.internalValue = value
}

// ReadInternal reads the signal visible to the internal fabric.
//
// In INPUT mode, returns the pad value (external → fabric).
// In OUTPUT/TRISTATE mode, returns the internally driven value.
func (io *IOBlock) ReadInternal() int {
	if io.mode == IOInput {
		return io.padValue
	}
	return io.internalValue
}

// ReadPad reads the signal visible on the external pad.
//
// In INPUT mode, returns the pad value.
// In OUTPUT mode, returns the internally driven value.
// In TRISTATE mode, returns nil (high impedance).
func (io *IOBlock) ReadPad() *int {
	switch io.mode {
	case IOInput:
		result := io.padValue
		return &result
	case IOTristate:
		return logicgates.TriState(io.internalValue, 0) // enable=0 → nil
	default: // IOOutput
		return logicgates.TriState(io.internalValue, 1) // enable=1 → &value
	}
}

// Name returns the I/O block identifier.
func (io *IOBlock) Name() string { return io.name }

// Mode returns the current operating mode.
func (io *IOBlock) Mode() IOMode { return io.mode }
