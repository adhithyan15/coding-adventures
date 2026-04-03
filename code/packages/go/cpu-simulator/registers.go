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
	result, _ := StartNew[*RegisterFile]("cpu-simulator.NewRegisterFile", nil,
		func(op *Operation[*RegisterFile], rf *ResultFactory[*RegisterFile]) *OperationResult[*RegisterFile] {
			op.AddProperty("num_registers", numRegisters)
			op.AddProperty("bit_width", bitWidth)
			return rf.Generate(true, false, &RegisterFile{
				NumRegisters: numRegisters,
				BitWidth:     bitWidth,
				values:       make([]uint32, numRegisters),
				maxValue:     (1 << bitWidth) - 1,
			})
		}).GetResult()
	return result
}

// Read returns the value stored in the specified register index.
func (r *RegisterFile) Read(index int) uint32 {
	result, _ := StartNew[uint32]("cpu-simulator.RegisterFile.Read", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("index", index)
			if index < 0 || index >= r.NumRegisters {
				panic(fmt.Sprintf("Register index %d out of range (0-%d)", index, r.NumRegisters-1))
			}
			return rf.Generate(true, false, r.values[index])
		}).PanicOnUnexpected().GetResult()
	return result
}

// Write stores a value into the specified register index.
// The value is masked by the BitWidth of the register file.
func (r *RegisterFile) Write(index int, value uint32) {
	_, _ = StartNew[struct{}]("cpu-simulator.RegisterFile.Write", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("index", index)
			if index < 0 || index >= r.NumRegisters {
				panic(fmt.Sprintf("Register index %d out of range (0-%d)", index, r.NumRegisters-1))
			}
			r.values[index] = value & r.maxValue
			return rf.Generate(true, false, struct{}{})
		}).PanicOnUnexpected().GetResult()
}

// Dump returns all register values as a map for inspection.
func (r *RegisterFile) Dump() map[string]uint32 {
	result, _ := StartNew[map[string]uint32]("cpu-simulator.RegisterFile.Dump", nil,
		func(op *Operation[map[string]uint32], rf *ResultFactory[map[string]uint32]) *OperationResult[map[string]uint32] {
			m := make(map[string]uint32, r.NumRegisters)
			for i, v := range r.values {
				m[fmt.Sprintf("R%d", i)] = v
			}
			return rf.Generate(true, false, m)
		}).GetResult()
	return result
}
