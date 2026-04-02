package intel4004gatelevel

// Program counter — 12-bit register with increment and load.
//
// # The 4004's program counter
//
// The program counter (PC) holds the address of the next instruction to
// fetch from ROM. It's 12 bits wide, addressing 4096 bytes of ROM.
//
// In real hardware, the PC is:
//   - A 12-bit register (12 D flip-flops)
//   - An incrementer (chain of half-adders for PC+1 or PC+2)
//   - A load input (for jump instructions)
//
// The incrementer uses half-adders chained together. To add 1:
//
//	bit0 -> half_adder(bit0, 1) -> sum0, carry
//	bit1 -> half_adder(bit1, carry) -> sum1, carry
//	...and so on for all 12 bits.
//
// This is simpler than a full adder chain because we're always adding
// a constant (1 or 2), so one input is fixed.

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/arithmetic"
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// ProgramCounter is a 12-bit program counter built from flip-flops and half-adders.
//
// Supports:
//   - Increment(): PC += 1 (for 1-byte instructions)
//   - Increment2(): PC += 2 (for 2-byte instructions)
//   - Load(addr): PC = addr (for jumps)
//   - Read(): current PC value
type ProgramCounter struct {
	state []logicgates.FlipFlopState
}

// NewProgramCounter creates a new PC initialized to 0.
func NewProgramCounter() *ProgramCounter {
	result, _ := StartNew[*ProgramCounter]("intel4004-gatelevel.NewProgramCounter", nil,
		func(op *Operation[*ProgramCounter], rf *ResultFactory[*ProgramCounter]) *OperationResult[*ProgramCounter] {
			zeros := make([]int, 12)
			_, state := logicgates.Register(zeros, 0, nil)
			_, state = logicgates.Register(zeros, 1, state)
			return rf.Generate(true, false, &ProgramCounter{state: state})
		}).GetResult()
	return result
}

// Read reads the current PC value (0-4095).
func (pc *ProgramCounter) Read() int {
	result, _ := StartNew[int]("intel4004-gatelevel.ProgramCounter.Read", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			zeros := make([]int, 12)
			output, _ := logicgates.Register(zeros, 0, pc.state)
			return rf.Generate(true, false, BitsToInt(output))
		}).GetResult()
	return result
}

// Load loads a new address into the PC (for jumps).
func (pc *ProgramCounter) Load(address int) {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.ProgramCounter.Load", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			bits := IntToBits(address&0xFFF, 12)
			_, state := logicgates.Register(bits, 0, pc.state)
			_, state = logicgates.Register(bits, 1, state)
			pc.state = state
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Increment increments PC by 1 using a chain of half-adders.
//
// This is how a real incrementer works:
//
//	carry_in = 1 (we're adding 1)
//	For each bit position:
//	    (new_bit, carry) = half_adder(old_bit, carry)
func (pc *ProgramCounter) Increment() {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.ProgramCounter.Increment", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			currentBits := IntToBits(pc.Read(), 12)
			carry := 1 // Adding 1
			newBits := make([]int, 12)
			for i, bit := range currentBits {
				sumBit, c := arithmetic.HalfAdder(bit, carry)
				newBits[i] = sumBit
				carry = c
			}
			pc.Load(BitsToInt(newBits))
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Increment2 increments PC by 2 (for 2-byte instructions).
//
// Two cascaded increments through the half-adder chain.
func (pc *ProgramCounter) Increment2() {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.ProgramCounter.Increment2", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			pc.Increment()
			pc.Increment()
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Reset resets PC to 0.
func (pc *ProgramCounter) Reset() {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.ProgramCounter.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			pc.Load(0)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// GateCount returns 12-bit register (72 gates) + 12 half-adders (24 gates) = 96.
func (pc *ProgramCounter) GateCount() int {
	result, _ := StartNew[int]("intel4004-gatelevel.ProgramCounter.GateCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, 96)
		}).GetResult()
	return result
}
