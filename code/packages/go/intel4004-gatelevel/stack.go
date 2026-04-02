package intel4004gatelevel

// Hardware call stack — 3 levels of 12-bit return addresses.
//
// # The 4004's stack
//
// The Intel 4004 has a 3-level hardware call stack. This is NOT a
// software stack in RAM — it's three physical 12-bit registers plus
// a 2-bit circular pointer, all built from D flip-flops.
//
// Why only 3 levels? The 4004 was designed for calculators, which had
// simple call structures. Three levels of subroutine nesting was enough
// for the Busicom 141-PF calculator's firmware.
//
// # Silent overflow
//
// When you push a 4th address, the stack wraps silently — the oldest
// return address is overwritten. There is no stack overflow exception.
// This matches the real hardware behavior. The 4004's designers saved
// transistors by not including overflow detection.

import (
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// HardwareStack is a 3-level x 12-bit hardware call stack.
//
// Built from 3 x 12 = 36 D flip-flops for storage, plus a 2-bit
// pointer that wraps modulo 3.
type HardwareStack struct {
	levels  [][]logicgates.FlipFlopState
	pointer int // 0, 1, or 2
}

// NewHardwareStack creates a new stack with 3 empty slots and pointer at 0.
func NewHardwareStack() *HardwareStack {
	result, _ := StartNew[*HardwareStack]("intel4004-gatelevel.NewHardwareStack", nil,
		func(op *Operation[*HardwareStack], rf *ResultFactory[*HardwareStack]) *OperationResult[*HardwareStack] {
			hs := &HardwareStack{
				levels:  make([][]logicgates.FlipFlopState, 3),
				pointer: 0,
			}
			zeros := make([]int, 12)
			for i := 0; i < 3; i++ {
				_, state := logicgates.Register(zeros, 0, nil)
				_, state = logicgates.Register(zeros, 1, state)
				hs.levels[i] = state
			}
			return rf.Generate(true, false, hs)
		}).GetResult()
	return result
}

// Push pushes a return address. Wraps silently on overflow.
//
// In real hardware: the pointer selects which of the 3 registers
// to write, then the pointer increments mod 3.
func (hs *HardwareStack) Push(address int) {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.HardwareStack.Push", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			bits := IntToBits(address&0xFFF, 12)
			_, state := logicgates.Register(bits, 0, hs.levels[hs.pointer])
			_, state = logicgates.Register(bits, 1, state)
			hs.levels[hs.pointer] = state
			hs.pointer = (hs.pointer + 1) % 3
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Pop pops and returns the top address.
//
// Decrements pointer mod 3, then reads that register.
func (hs *HardwareStack) Pop() int {
	result, _ := StartNew[int]("intel4004-gatelevel.HardwareStack.Pop", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			hs.pointer = (hs.pointer + 2) % 3 // equivalent to (pointer - 1) % 3
			zeros := make([]int, 12)
			output, _ := logicgates.Register(zeros, 0, hs.levels[hs.pointer])
			return rf.Generate(true, false, BitsToInt(output))
		}).GetResult()
	return result
}

// Reset resets all stack levels to 0 and pointer to 0.
func (hs *HardwareStack) Reset() {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.HardwareStack.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			zeros := make([]int, 12)
			for i := 0; i < 3; i++ {
				_, state := logicgates.Register(zeros, 0, nil)
				_, state = logicgates.Register(zeros, 1, state)
				hs.levels[i] = state
			}
			hs.pointer = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Depth returns the current pointer position (not true depth, since we wrap).
func (hs *HardwareStack) Depth() int {
	result, _ := StartNew[int]("intel4004-gatelevel.HardwareStack.Depth", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, hs.pointer)
		}).GetResult()
	return result
}

// GateCount returns 3 x 12-bit registers (216 gates) + pointer logic (~10 gates).
func (hs *HardwareStack) GateCount() int {
	result, _ := StartNew[int]("intel4004-gatelevel.HardwareStack.GateCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, 226)
		}).GetResult()
	return result
}
