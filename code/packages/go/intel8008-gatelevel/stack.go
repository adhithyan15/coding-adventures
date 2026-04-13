package intel8008gatelevel

// 8-level hardware push-down stack for the Intel 8008.
//
// # The 8008's stack architecture
//
// The 8008's stack is fundamentally different from the 4004's 3-level stack.
// The 8008 uses a push-down architecture where entry 0 is ALWAYS the current
// program counter. There is no separate PC register — the PC IS the top of
// the stack.
//
// # How push-down works
//
// When a CALL happens:
//
//	CALL target:
//	  entry[7] ← entry[6]   (rotate all entries down by one)
//	  entry[6] ← entry[5]
//	  ...
//	  entry[1] ← entry[0]   (entry[0] was current PC, now saved as return address)
//	  entry[0] ← target      (new PC is the call target)
//
// When a RETURN happens:
//
//	RET:
//	  entry[0] ← entry[1]   (pop: return address moves to current PC)
//	  entry[1] ← entry[2]   (rotate all entries up by one)
//	  ...
//	  entry[6] ← entry[7]
//	  entry[7] ← 0           (zero-fill at the bottom)
//
// For JMP (not CALL):
//
//	JMP target:
//	  entry[0] ← target      (just overwrite current PC, no stack rotation)
//
// # Why this matters
//
// The push-down architecture means the program counter is not a special register
// — it's physically implemented as the first of the 8 stack registers. This
// reduces transistor count by sharing flip-flops between the PC and stack.
//
// The 8008 can make at most 7 nested calls. The 8th call overwrites the oldest
// saved return address silently (no overflow detection). This is a hardware
// constraint, not a software limitation.
//
// # Implementation
//
// Each of the 8 stack entries is a 14-bit register (8 × 14 = 112 D flip-flops).
// A 3-bit stack depth counter tracks how many entries contain valid return addresses.

import (
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// PushDownStack is the 8008's 8-level hardware push-down stack.
//
// Entry 0 is always the current program counter.
// Entries 1-7 hold saved return addresses from nested calls.
type PushDownStack struct {
	// 8 × 14-bit registers, each stored as FlipFlopState slices
	levels [8][]logicgates.FlipFlopState
	// Number of valid return addresses (0-7)
	// depth=0: only entry[0] (current PC) is valid
	// depth=7: maximum nesting, all 8 slots occupied
	depth int
}

// NewPushDownStack creates a new push-down stack with all entries set to 0.
func NewPushDownStack() *PushDownStack {
	result, _ := StartNew[*PushDownStack]("intel8008-gatelevel.NewPushDownStack", nil,
		func(op *Operation[*PushDownStack], rf *ResultFactory[*PushDownStack]) *OperationResult[*PushDownStack] {
			s := &PushDownStack{depth: 0}
			zeros := make([]int, 14)
			for i := 0; i < 8; i++ {
				_, state := logicgates.Register(zeros, 0, nil)
				_, state = logicgates.Register(zeros, 1, state)
				s.levels[i] = state
			}
			return rf.Generate(true, false, s)
		}).GetResult()
	return result
}

// readLevel reads the 14-bit value stored in one stack level.
func (s *PushDownStack) readLevel(level int) int {
	zeros := make([]int, 14)
	output, _ := logicgates.Register(zeros, 0, s.levels[level])
	return BitsToInt(output)
}

// writeLevel writes a 14-bit value to one stack level.
func (s *PushDownStack) writeLevel(level int, address int) {
	bits := IntToBits(address&0x3FFF, 14)
	_, state := logicgates.Register(bits, 0, s.levels[level])
	_, state = logicgates.Register(bits, 1, state)
	s.levels[level] = state
}

// PC returns the current program counter (always entry 0).
func (s *PushDownStack) PC() int {
	result, _ := StartNew[int]("intel8008-gatelevel.PushDownStack.PC", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, s.readLevel(0))
		}).GetResult()
	return result
}

// SetPC writes directly to entry 0 (used by JMP — no stack rotation).
//
// JMP does NOT save a return address. It just overwrites the current PC.
// This is different from CALL (Push) which rotates the stack down first.
func (s *PushDownStack) SetPC(address int) {
	_, _ = StartNew[struct{}]("intel8008-gatelevel.PushDownStack.SetPC", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			s.writeLevel(0, address)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Increment increments entry 0 (the PC) by n using a chain of half-adders.
//
// The 8008 has a 14-bit PC, so we chain 14 half-adders. For n=1 (1-byte
// instruction) or n=2 (2-byte) or n=3 (3-byte instruction), we add n by
// cascading n increment calls. This models how the real chip incremented
// the PC one byte at a time during fetch.
func (s *PushDownStack) Increment(n int) {
	_, _ = StartNew[struct{}]("intel8008-gatelevel.PushDownStack.Increment", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("n", n)
			current := s.readLevel(0)
			// Chain n half-adder increments to simulate PC+n
			for i := 0; i < n; i++ {
				current = incrementPC14(current)
			}
			s.writeLevel(0, current)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// incrementPC14 increments a 14-bit value by 1 using a chain of half-adders.
//
// This is how a real 14-bit incrementer works:
//
//	carry_in = 1 (we're adding 1)
//	half_adder(pc_bit0, 1)  → sum0, carry0
//	half_adder(pc_bit1, carry0) → sum1, carry1
//	...
//	half_adder(pc_bit13, carry12) → sum13, (carry13 discarded — wraps at 0x3FFF)
func incrementPC14(pc int) int {
	bits := IntToBits(pc, 14)
	carry := 1 // adding 1
	for i := 0; i < 14; i++ {
		sum := bits[i] ^ carry   // XOR gate
		newCarry := bits[i] & carry // AND gate
		bits[i] = sum
		carry = newCarry
	}
	// carry bit 14 is discarded (14-bit wrap)
	return BitsToInt(bits)
}

// Push saves the current PC as a return address and jumps to target.
//
// This is the CALL operation:
//
//	entry[7] ← entry[6]
//	entry[6] ← entry[5]
//	...
//	entry[1] ← entry[0]  (current PC saved here — caller's return address)
//	entry[0] ← target     (new PC = call target)
//
// On overflow (8th nested call), entry[7] is silently overwritten.
func (s *PushDownStack) Push(target int) {
	_, _ = StartNew[struct{}]("intel8008-gatelevel.PushDownStack.Push", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("target", target)
			// Rotate entries down: 7←6, 6←5, ..., 1←0
			for i := 7; i > 0; i-- {
				s.writeLevel(i, s.readLevel(i-1))
			}
			// Set entry 0 to call target
			s.writeLevel(0, target&0x3FFF)
			// Track depth (max 7 saved return addresses)
			if s.depth < 7 {
				s.depth++
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Pop returns the current PC to the caller's address.
//
// This is the RETURN operation:
//
//	entry[0] ← entry[1]  (return address becomes new PC)
//	entry[1] ← entry[2]
//	...
//	entry[6] ← entry[7]
//	entry[7] ← 0          (zero-fill bottom)
func (s *PushDownStack) Pop() {
	_, _ = StartNew[struct{}]("intel8008-gatelevel.PushDownStack.Pop", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			// Rotate entries up: 0←1, 1←2, ..., 6←7
			for i := 0; i < 7; i++ {
				s.writeLevel(i, s.readLevel(i+1))
			}
			// Zero-fill entry 7
			s.writeLevel(7, 0)
			// Track depth
			if s.depth > 0 {
				s.depth--
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Depth returns the number of valid return addresses on the stack (0-7).
func (s *PushDownStack) Depth() int {
	result, _ := StartNew[int]("intel8008-gatelevel.PushDownStack.Depth", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, s.depth)
		}).GetResult()
	return result
}

// ReadLevel reads any stack level for inspection (0 = current PC).
func (s *PushDownStack) ReadLevel(level int) int {
	result, _ := StartNew[int]("intel8008-gatelevel.PushDownStack.ReadLevel", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("level", level)
			return rf.Generate(true, false, s.readLevel(level))
		}).GetResult()
	return result
}

// Reset resets all stack entries to 0 and depth to 0.
func (s *PushDownStack) Reset() {
	_, _ = StartNew[struct{}]("intel8008-gatelevel.PushDownStack.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for i := 0; i < 8; i++ {
				s.writeLevel(i, 0)
			}
			s.depth = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// GateCount returns the estimated gate count for the stack.
//
// 8 levels × 14 bits × ~4 NOR gates per flip-flop = 448 NOR gates for storage.
// Plus rotation mux logic: ~56 gates.
// Plus 3-bit depth counter: ~18 gates.
// Total: ~522 gates.
func (s *PushDownStack) GateCount() int {
	result, _ := StartNew[int]("intel8008-gatelevel.PushDownStack.GateCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, 522)
		}).GetResult()
	return result
}
