package cpusimulator

import "fmt"

// RegisterFile represents the CPU's fast, small storage.
//
// === What are registers? ===
//
// Registers are the fastest storage in a computer. They sit inside the CPU
// itself. A typical CPU has between 8 and 32 registers, each holding one
// "word" of data (e.g., 32 bits).
type RegisterFile struct {
	NumRegisters int
	BitWidth     int
	values       []uint32
	maxValue     uint32
}

// NewRegisterFile initializes a new set of registers.
func NewRegisterFile(numRegisters, bitWidth int) *RegisterFile {
	return &RegisterFile{
		NumRegisters: numRegisters,
		BitWidth:     bitWidth,
		values:       make([]uint32, numRegisters),
		maxValue:     (1 << bitWidth) - 1,
	}
}

// Read returns the value stored in the specified register index.
func (r *RegisterFile) Read(index int) uint32 {
	if index < 0 || index >= r.NumRegisters {
		panic(fmt.Sprintf("Register index %d out of range (0-%d)", index, r.NumRegisters-1))
	}
	return r.values[index]
}

// Write stores a value into the specified register index.
// The value is masked by the BitWidth of the register file.
func (r *RegisterFile) Write(index int, value uint32) {
	if index < 0 || index >= r.NumRegisters {
		panic(fmt.Sprintf("Register index %d out of range (0-%d)", index, r.NumRegisters-1))
	}
	r.values[index] = value & r.maxValue
}

// Dump returns all register values as a map for inspection.
func (r *RegisterFile) Dump() map[string]uint32 {
	result := make(map[string]uint32, r.NumRegisters)
	for i, v := range r.values {
		result[fmt.Sprintf("R%d", i)] = v
	}
	return result
}
