package intel8008gatelevel

// Register file — 7 × 8-bit registers built from D flip-flops.
//
// # The 8008's register file
//
// The Intel 8008 has 7 general-purpose 8-bit registers: A (accumulator),
// B, C, D, E, H, and L. Together these form 7 × 8 = 56 D flip-flops,
// compared to the 4004's 16 × 4 = 64 flip-flops (the 8008's register file
// actually has fewer flip-flops despite being an "upgrade" — the 4004 wasted
// transistors on register encoding flexibility).
//
// Each register write traverses:
//
//	value → IntToBits(value, 8) → Register(bits, clockEdge, state, width=8)
//	  → 8 × D_flip_flop(bit, clockEdge)
//	    → 8 × SR_latch(D, NOT(D))
//	      → 8 × 2 NOR gates = 16 NOR gates per register
//
// Total: 7 registers × 16 NOR gates = 112 NOR gates = 448 transistors
// (compare to the 4004's 16 × 8 = 128 NOR gates = 512 transistors)
//
// # Register encoding (3-bit SSS/DDD field)
//
// The 8008 uses a 3-bit field to identify registers in opcodes:
//
//	0 = B    1 = C    2 = D    3 = E
//	4 = H    5 = L    6 = M (pseudo-register, memory access)    7 = A
//
// Register index 6 is NOT a real register — it's the M pseudo-register
// that routes reads/writes to memory at the address (H & 0x3F) << 8 | L.
// Reading or writing reg 6 via this file is a programming error.
//
// # Flag register
//
// The 4 CPU flags (Carry, Zero, Sign, Parity) are stored in a 4-bit register
// of D flip-flops, separate from the data register file.

import (
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// RegisterFile is a 7 × 8-bit register file built from D flip-flops.
//
// Indices 0-5 are B, C, D, E, H, L. Index 7 is A (accumulator).
// Index 6 is the M pseudo-register and is NOT stored here.
type RegisterFile struct {
	// 8 slots allocated (indices 0-7), index 6 is unused (M pseudo-register)
	// Each slot is a list of 8 FlipFlopState values (one per bit)
	states [8][]logicgates.FlipFlopState
}

// NewRegisterFile creates a new register file with all registers set to 0.
//
// Each register is initialized by clocking a zero byte through 8 flip-flops.
func NewRegisterFile() *RegisterFile {
	result, _ := StartNew[*RegisterFile]("intel8008-gatelevel.NewRegisterFile", nil,
		func(op *Operation[*RegisterFile], rf *ResultFactory[*RegisterFile]) *OperationResult[*RegisterFile] {
			regFile := &RegisterFile{}
			zeros := make([]int, 8)
			for i := 0; i < 8; i++ {
				if i == 6 {
					continue // Index 6 is M pseudo-register, skip
				}
				// Initialize: clock low (setup) then clock high (latch)
				_, state := logicgates.Register(zeros, 0, nil)
				_, state = logicgates.Register(zeros, 1, state)
				regFile.states[i] = state
			}
			return rf.Generate(true, false, regFile)
		}).GetResult()
	return result
}

// Read reads an 8-bit register value.
//
// index must be 0-7 (excluding 6 = M pseudo-register).
// In real hardware, this routes through a 3-to-8 decoder and an 8-to-1 mux.
// We simulate the flip-flop read directly.
func (regFile *RegisterFile) Read(index int) int {
	result, _ := StartNew[int]("intel8008-gatelevel.RegisterFile.Read", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("index", index)
			if index == 6 {
				panic("intel8008-gatelevel: register index 6 is M pseudo-register — use memory, not RegisterFile.Read")
			}
			// Read current flip-flop outputs (clock=0 = read mode, no write)
			zeros := make([]int, 8)
			output, _ := logicgates.Register(zeros, 0, regFile.states[index])
			return rf.Generate(true, false, BitsToInt(output))
		}).PanicOnUnexpected().GetResult()
	return result
}

// Write writes an 8-bit value to a register.
//
// In real hardware: the 3-bit register address is decoded by a 3-to-8
// decoder (8 AND/NOT gates), which enables the write path for exactly one
// register. The 8-bit data then clocks through 8 D flip-flops.
func (regFile *RegisterFile) Write(index int, value int) {
	_, _ = StartNew[struct{}]("intel8008-gatelevel.RegisterFile.Write", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("index", index)
			op.AddProperty("value", value)
			if index == 6 {
				panic("intel8008-gatelevel: register index 6 is M pseudo-register — use memory, not RegisterFile.Write")
			}
			bits := IntToBits(value&0xFF, 8)
			// Two-phase clocking: setup (clock=0) then capture (clock=1)
			_, state := logicgates.Register(bits, 0, regFile.states[index])
			_, state = logicgates.Register(bits, 1, state)
			regFile.states[index] = state
			return rf.Generate(true, false, struct{}{})
		}).PanicOnUnexpected().GetResult()
}

// HLAddress computes the 14-bit M pseudo-register address from H and L.
//
// The 8008 forms the memory address as:
//
//	address = (H & 0x3F) << 8 | L
//
// Why mask H to 6 bits? The 8008's address bus is 14 bits wide. H contributes
// the upper 6 bits (bits 8-13) and L contributes the lower 8 bits (bits 0-7).
// The upper 2 bits of H (bits 6-7) are ignored.
//
// In gate logic, this is a combination of AND gates (to mask H's upper bits),
// a shift (hardwired — no gates needed), and OR gates to combine.
func (regFile *RegisterFile) HLAddress() int {
	result, _ := StartNew[int]("intel8008-gatelevel.RegisterFile.HLAddress", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			h := regFile.Read(4) // H = register index 4
			l := regFile.Read(5) // L = register index 5

			// Mask H to 6 bits using AND gates: H & 0x3F
			hBits := IntToBits(h, 8)
			maskBits := IntToBits(0x3F, 8)
			hMasked := 0
			for i := 0; i < 8; i++ {
				hMasked |= logicgates.AND(hBits[i], maskBits[i]) << i
			}

			// Combine: address = (hMasked << 8) | l
			address := (hMasked << 8) | l
			return rf.Generate(true, false, address&0x3FFF) // ensure 14-bit
		}).GetResult()
	return result
}

// Reset resets all registers to 0.
func (regFile *RegisterFile) Reset() {
	_, _ = StartNew[struct{}]("intel8008-gatelevel.RegisterFile.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for i := 0; i < 8; i++ {
				if i == 6 {
					continue // Skip M pseudo-register
				}
				regFile.Write(i, 0)
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// GateCount returns the gate count for the register file.
//
// 7 registers × 8 bits × ~4 NOR gates per flip-flop = 224 NOR gates.
// Plus 3-to-8 decoder for write select: ~24 gates.
// Plus 8-to-1 mux for read select: ~56 gates.
// Total: ~304 gates.
func (regFile *RegisterFile) GateCount() int {
	result, _ := StartNew[int]("intel8008-gatelevel.RegisterFile.GateCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, 304)
		}).GetResult()
	return result
}

// FlagRegister stores the 4 CPU flags in a 4-bit D flip-flop register.
//
// Flags:
//
//	bit 0 = Carry
//	bit 1 = Zero
//	bit 2 = Sign
//	bit 3 = Parity (even parity → 1)
//
// This is 4 D flip-flops = ~16 NOR gates = ~64 transistors.
type FlagRegister struct {
	state []logicgates.FlipFlopState
}

// NewFlagRegister creates a flag register initialized to all zeros.
func NewFlagRegister() *FlagRegister {
	result, _ := StartNew[*FlagRegister]("intel8008-gatelevel.NewFlagRegister", nil,
		func(op *Operation[*FlagRegister], rf *ResultFactory[*FlagRegister]) *OperationResult[*FlagRegister] {
			zeros := make([]int, 4)
			_, state := logicgates.Register(zeros, 0, nil)
			_, state = logicgates.Register(zeros, 1, state)
			return rf.Generate(true, false, &FlagRegister{state: state})
		}).GetResult()
	return result
}

// ReadFlags reads all 4 flags as a packed integer.
//
// Returns bit 0=Carry, bit 1=Zero, bit 2=Sign, bit 3=Parity.
func (f *FlagRegister) ReadFlags() (carry, zero, sign, parity bool) {
	type flagVals struct {
		carry, zero, sign, parity bool
	}
	fv, _ := StartNew[flagVals]("intel8008-gatelevel.FlagRegister.ReadFlags", flagVals{},
		func(op *Operation[flagVals], rf *ResultFactory[flagVals]) *OperationResult[flagVals] {
			zeros := make([]int, 4)
			output, _ := logicgates.Register(zeros, 0, f.state)
			return rf.Generate(true, false, flagVals{
				carry:  output[0] == 1,
				zero:   output[1] == 1,
				sign:   output[2] == 1,
				parity: output[3] == 1,
			})
		}).GetResult()
	return fv.carry, fv.zero, fv.sign, fv.parity
}

// WriteFlags writes all 4 flags.
func (f *FlagRegister) WriteFlags(carry, zero, sign, parity bool) {
	_, _ = StartNew[struct{}]("intel8008-gatelevel.FlagRegister.WriteFlags", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			bits := make([]int, 4)
			if carry {
				bits[0] = 1
			}
			if zero {
				bits[1] = 1
			}
			if sign {
				bits[2] = 1
			}
			if parity {
				bits[3] = 1
			}
			_, state := logicgates.Register(bits, 0, f.state)
			_, state = logicgates.Register(bits, 1, state)
			f.state = state
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Reset resets all flags to false.
func (f *FlagRegister) Reset() {
	f.WriteFlags(false, false, false, false)
}

// GateCount returns ~16 gates (4 flip-flops × ~4 NOR gates each).
func (f *FlagRegister) GateCount() int {
	result, _ := StartNew[int]("intel8008-gatelevel.FlagRegister.GateCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, 16)
		}).GetResult()
	return result
}
