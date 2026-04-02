package intel4004gatelevel

// Register file — 16 x 4-bit registers built from D flip-flops.
//
// # How registers work in hardware
//
// A register is a group of D flip-flops that share a clock signal. Each
// flip-flop stores one bit. A 4-bit register has 4 flip-flops. The Intel
// 4004 has 16 such registers (R0-R15), for a total of 64 flip-flops just
// for the register file.
//
// In this simulation, each register call goes through:
//
//	data bits -> D flip-flop x 4 -> output bits
//
// The flip-flops are edge-triggered: they capture new data on the rising
// edge of the clock. Between edges, the stored value is stable.
//
// # Register pairs
//
// The 4004 organizes its 16 registers into 8 pairs:
//
//	P0 = R0:R1, P1 = R2:R3, ..., P7 = R14:R15
//
// A register pair holds an 8-bit value (high nibble in even register,
// low nibble in odd register). Pairs are used for FIM, SRC, FIN, JIN.
//
// # Accumulator
//
// The accumulator is a separate 4-bit register, not part of the R0-R15
// file. It has its own dedicated flip-flops and is connected directly to
// the ALU's output bus.

import (
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// RegisterFile is a 16 x 4-bit register file built from D flip-flops.
//
// Each of the 16 registers is a group of 4 D flip-flops from the
// logic_gates sequential module. Reading and writing go through
// actual flip-flop state transitions.
type RegisterFile struct {
	states [][]logicgates.FlipFlopState
}

// NewRegisterFile creates a new register file with all registers set to 0.
func NewRegisterFile() *RegisterFile {
	result, _ := StartNew[*RegisterFile]("intel4004-gatelevel.NewRegisterFile", nil,
		func(op *Operation[*RegisterFile], rf *ResultFactory[*RegisterFile]) *OperationResult[*RegisterFile] {
			regFile := &RegisterFile{
				states: make([][]logicgates.FlipFlopState, 16),
			}
			for i := 0; i < 16; i++ {
				// Initialize state by clocking zeros through
				_, state := logicgates.Register([]int{0, 0, 0, 0}, 0, nil)
				_, state = logicgates.Register([]int{0, 0, 0, 0}, 1, state)
				regFile.states[i] = state
			}
			return rf.Generate(true, false, regFile)
		}).GetResult()
	return result
}

// Read reads a register value. Returns 4-bit integer (0-15).
//
// In real hardware, this would route through a 16-to-1 multiplexer
// built from gates. We simulate the flip-flop read directly.
func (regFile *RegisterFile) Read(index int) int {
	result, _ := StartNew[int]("intel4004-gatelevel.RegisterFile.Read", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("index", index)
			// Read current output from flip-flops (clock=0, no write)
			output, _ := logicgates.Register([]int{0, 0, 0, 0}, 0, regFile.states[index])
			return rf.Generate(true, false, BitsToInt(output))
		}).GetResult()
	return result
}

// Write writes a 4-bit value to a register.
//
// In real hardware: decoder selects the register, data bus presents
// the value, clock edge latches it into the flip-flops.
func (regFile *RegisterFile) Write(index int, value int) {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.RegisterFile.Write", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("index", index)
			op.AddProperty("value", value)
			bits := IntToBits(value&0xF, 4)
			// Clock low (setup)
			_, state := logicgates.Register(bits, 0, regFile.states[index])
			// Clock high (capture on rising edge)
			_, state = logicgates.Register(bits, 1, state)
			regFile.states[index] = state
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ReadPair reads an 8-bit value from a register pair.
//
// Pair 0 = R0:R1 (R0=high nibble, R1=low nibble).
func (regFile *RegisterFile) ReadPair(pairIndex int) int {
	result, _ := StartNew[int]("intel4004-gatelevel.RegisterFile.ReadPair", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("pairIndex", pairIndex)
			high := regFile.Read(pairIndex * 2)
			low := regFile.Read(pairIndex*2 + 1)
			return rf.Generate(true, false, (high<<4)|low)
		}).GetResult()
	return result
}

// WritePair writes an 8-bit value to a register pair.
func (regFile *RegisterFile) WritePair(pairIndex int, value int) {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.RegisterFile.WritePair", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("pairIndex", pairIndex)
			op.AddProperty("value", value)
			regFile.Write(pairIndex*2, (value>>4)&0xF)
			regFile.Write(pairIndex*2+1, value&0xF)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Reset resets all registers to 0 by clocking in zeros.
func (regFile *RegisterFile) Reset() {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.RegisterFile.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for i := 0; i < 16; i++ {
				regFile.Write(i, 0)
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// GateCount returns the gate count for the register file.
//
// 16 registers x 4 bits x ~6 gates per D flip-flop = 384 gates.
// Plus 4-to-16 decoder for write select: ~32 gates.
// Plus 16-to-1 mux for read select: ~64 gates.
// Total: ~480 gates.
func (regFile *RegisterFile) GateCount() int {
	result, _ := StartNew[int]("intel4004-gatelevel.RegisterFile.GateCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, 480)
		}).GetResult()
	return result
}

// Accumulator is a 4-bit accumulator register built from D flip-flops.
//
// The accumulator is the 4004's main working register. Almost every
// arithmetic and logic operation reads from or writes to it.
type Accumulator struct {
	state []logicgates.FlipFlopState
}

// NewAccumulator creates a new accumulator initialized to 0.
func NewAccumulator() *Accumulator {
	result, _ := StartNew[*Accumulator]("intel4004-gatelevel.NewAccumulator", nil,
		func(op *Operation[*Accumulator], rf *ResultFactory[*Accumulator]) *OperationResult[*Accumulator] {
			_, state := logicgates.Register([]int{0, 0, 0, 0}, 0, nil)
			_, state = logicgates.Register([]int{0, 0, 0, 0}, 1, state)
			return rf.Generate(true, false, &Accumulator{state: state})
		}).GetResult()
	return result
}

// Read reads the accumulator value (0-15).
func (a *Accumulator) Read() int {
	result, _ := StartNew[int]("intel4004-gatelevel.Accumulator.Read", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			output, _ := logicgates.Register([]int{0, 0, 0, 0}, 0, a.state)
			return rf.Generate(true, false, BitsToInt(output))
		}).GetResult()
	return result
}

// Write writes a 4-bit value to the accumulator.
func (a *Accumulator) Write(value int) {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.Accumulator.Write", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("value", value)
			bits := IntToBits(value&0xF, 4)
			_, state := logicgates.Register(bits, 0, a.state)
			_, state = logicgates.Register(bits, 1, state)
			a.state = state
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Reset resets to 0.
func (a *Accumulator) Reset() {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.Accumulator.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			a.Write(0)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// GateCount returns 4 D flip-flops x ~6 gates = 24 gates.
func (a *Accumulator) GateCount() int {
	result, _ := StartNew[int]("intel4004-gatelevel.Accumulator.GateCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, 24)
		}).GetResult()
	return result
}

// CarryFlag is a 1-bit carry/borrow flag built from a D flip-flop.
//
// The carry flag is set by arithmetic operations and read by
// conditional jumps and multi-digit BCD arithmetic.
type CarryFlag struct {
	state []logicgates.FlipFlopState
}

// NewCarryFlag creates a new carry flag initialized to false.
func NewCarryFlag() *CarryFlag {
	result, _ := StartNew[*CarryFlag]("intel4004-gatelevel.NewCarryFlag", nil,
		func(op *Operation[*CarryFlag], rf *ResultFactory[*CarryFlag]) *OperationResult[*CarryFlag] {
			_, state := logicgates.Register([]int{0}, 0, nil)
			_, state = logicgates.Register([]int{0}, 1, state)
			return rf.Generate(true, false, &CarryFlag{state: state})
		}).GetResult()
	return result
}

// Read reads carry flag as a boolean.
func (c *CarryFlag) Read() bool {
	result, _ := StartNew[bool]("intel4004-gatelevel.CarryFlag.Read", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			output, _ := logicgates.Register([]int{0}, 0, c.state)
			return rf.Generate(true, false, output[0] == 1)
		}).GetResult()
	return result
}

// Write writes carry flag.
func (c *CarryFlag) Write(value bool) {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.CarryFlag.Write", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("value", value)
			bit := 0
			if value {
				bit = 1
			}
			_, state := logicgates.Register([]int{bit}, 0, c.state)
			_, state = logicgates.Register([]int{bit}, 1, state)
			c.state = state
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Reset resets to false.
func (c *CarryFlag) Reset() {
	_, _ = StartNew[struct{}]("intel4004-gatelevel.CarryFlag.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			c.Write(false)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// GateCount returns 1 D flip-flop x ~6 gates = 6 gates.
func (c *CarryFlag) GateCount() int {
	result, _ := StartNew[int]("intel4004-gatelevel.CarryFlag.GateCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, 6)
		}).GetResult()
	return result
}
