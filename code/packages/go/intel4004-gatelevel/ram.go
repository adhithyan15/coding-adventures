package intel4004gatelevel

// RAM — 4 banks x 4 registers x 20 nibbles, built from flip-flops.
//
// # The 4004's RAM architecture
//
// The Intel 4004 used separate RAM chips (Intel 4002), each containing:
//   - 4 registers
//   - Each register has 16 main characters + 4 status characters
//   - Each character is a 4-bit nibble
//   - Total per chip: 4 x 20 x 4 = 320 bits
//
// The full system supports up to 4 RAM banks (4 chips), selected by the
// DCL instruction. Within a bank, the SRC instruction sets which register
// and character to access.
//
// In real hardware, each nibble is stored in 4 D flip-flops. The full
// RAM system uses 4 x 4 x 20 x 4 = 1,280 flip-flops. We simulate this
// using the Register() function from the logic_gates package.
//
// # Addressing
//
// RAM is addressed in two steps:
//  1. DCL sets the bank (0-3, from accumulator bits 0-2)
//  2. SRC sends an 8-bit address from a register pair:
//     - High nibble -> register index (0-3)
//     - Low nibble -> character index (0-15)

import (
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// RAM is the 4004 RAM: 4 banks x 4 registers x (16 main + 4 status) nibbles.
//
// Every nibble is stored in 4 D flip-flops from the sequential logic
// package. Reading and writing physically route through flip-flop
// state transitions.
type RAM struct {
	// main[bank][reg][char] = flip-flop state for one nibble
	main   [4][4][16][]logicgates.FlipFlopState
	status [4][4][4][]logicgates.FlipFlopState
	// Output ports (one per bank, written by WMP)
	output [4]int
}

// NewRAM creates a new RAM with all cells initialized to 0.
func NewRAM() *RAM {
	result, _ := StartNew[*RAM]("intel4004-gatelevel.NewRAM", nil,
		func(op *Operation[*RAM], rf *ResultFactory[*RAM]) *OperationResult[*RAM] {
			r := &RAM{}
			for bank := 0; bank < 4; bank++ {
				for reg := 0; reg < 4; reg++ {
					for ch := 0; ch < 16; ch++ {
						_, state := logicgates.Register([]int{0, 0, 0, 0}, 0, nil)
						_, state = logicgates.Register([]int{0, 0, 0, 0}, 1, state)
						r.main[bank][reg][ch] = state
					}
					for st := 0; st < 4; st++ {
						_, state := logicgates.Register([]int{0, 0, 0, 0}, 0, nil)
						_, state = logicgates.Register([]int{0, 0, 0, 0}, 1, state)
						r.status[bank][reg][st] = state
					}
				}
			}
			return rf.Generate(true, false, r)
		}).GetResult()
	return result
}

// ReadMain reads a main character (4-bit nibble) from RAM.
func (r *RAM) ReadMain(bank, reg, char int) int {
	result, _ := StartNew[int]("intel4004-gatelevel.RAM.ReadMain", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("bank", bank)
			op.AddProperty("reg", reg)
			op.AddProperty("char", char)
			state := r.main[bank&3][reg&3][char&0xF]
			output, _ := logicgates.Register([]int{0, 0, 0, 0}, 0, state)
			return rf.Generate(true, false, BitsToInt(output))
		}).GetResult()
	return result
}

// WriteMain writes a 4-bit value to a main character.
func (r *RAM) WriteMain(bank, reg, char, value int) {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.RAM.WriteMain", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("bank", bank)
			op.AddProperty("reg", reg)
			op.AddProperty("char", char)
			op.AddProperty("value", value)
			bits := IntToBits(value&0xF, 4)
			state := r.main[bank&3][reg&3][char&0xF]
			_, state = logicgates.Register(bits, 0, state)
			_, state = logicgates.Register(bits, 1, state)
			r.main[bank&3][reg&3][char&0xF] = state
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ReadStatus reads a status character (0-3) from RAM.
func (r *RAM) ReadStatus(bank, reg, index int) int {
	result, _ := StartNew[int]("intel4004-gatelevel.RAM.ReadStatus", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("bank", bank)
			op.AddProperty("reg", reg)
			op.AddProperty("index", index)
			state := r.status[bank&3][reg&3][index&3]
			output, _ := logicgates.Register([]int{0, 0, 0, 0}, 0, state)
			return rf.Generate(true, false, BitsToInt(output))
		}).GetResult()
	return result
}

// WriteStatus writes a 4-bit value to a status character.
func (r *RAM) WriteStatus(bank, reg, index, value int) {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.RAM.WriteStatus", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("bank", bank)
			op.AddProperty("reg", reg)
			op.AddProperty("index", index)
			op.AddProperty("value", value)
			bits := IntToBits(value&0xF, 4)
			state := r.status[bank&3][reg&3][index&3]
			_, state = logicgates.Register(bits, 0, state)
			_, state = logicgates.Register(bits, 1, state)
			r.status[bank&3][reg&3][index&3] = state
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ReadOutput reads a RAM output port value.
func (r *RAM) ReadOutput(bank int) int {
	result, _ := StartNew[int]("intel4004-gatelevel.RAM.ReadOutput", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("bank", bank)
			return rf.Generate(true, false, r.output[bank&3])
		}).GetResult()
	return result
}

// WriteOutput writes to a RAM output port (WMP instruction).
func (r *RAM) WriteOutput(bank, value int) {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.RAM.WriteOutput", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("bank", bank)
			op.AddProperty("value", value)
			r.output[bank&3] = value & 0xF
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Reset resets all RAM to 0.
func (r *RAM) Reset() {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.RAM.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for bank := 0; bank < 4; bank++ {
				for reg := 0; reg < 4; reg++ {
					for char := 0; char < 16; char++ {
						r.WriteMain(bank, reg, char, 0)
					}
					for stat := 0; stat < 4; stat++ {
						r.WriteStatus(bank, reg, stat, 0)
					}
				}
				r.output[bank] = 0
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// GateCount returns 4 banks x 4 regs x 20 nibbles x 4 bits x 6 gates/ff = 7680.
// Plus addressing/decoding: ~200 gates.
func (r *RAM) GateCount() int {
	result, _ := StartNew[int]("intel4004-gatelevel.RAM.GateCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, 7880)
		}).GetResult()
	return result
}
