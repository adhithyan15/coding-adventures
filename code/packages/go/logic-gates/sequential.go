package logicgates

import "fmt"

// =========================================================================
// Sequential Logic — Circuits That Remember
// =========================================================================
//
// # Combinational vs Sequential Logic
//
// Everything in gates.go is "combinational" logic: the output depends
// ONLY on the current inputs. There is no memory, no history, no state.
//
// Sequential logic is different: the output depends on current inputs
// AND previous state. Sequential circuits can REMEMBER. This is the
// fundamental difference between a calculator (combinational) and a
// computer (sequential).
//
// The key insight: by feeding a gate's output back to its own input,
// we create a circuit that can hold a value indefinitely. This
// feedback loop is the basis of all digital memory.
//
// # The memory hierarchy
//
// From simplest to most complex:
//
//   1. SR Latch     — remembers one bit (set/reset interface)
//   2. D Latch      — remembers one bit (data interface, transparent)
//   3. D Flip-Flop  — remembers one bit (data interface, edge-triggered)
//   4. Register     — remembers N bits (parallel flip-flops)
//   5. Shift Register — moves bits left/right on each clock
//   6. Counter      — counts up on each clock pulse
//
// Each builds on the previous, adding capabilities:
//
//   SR Latch → D Latch (simpler interface)
//   D Latch  → D Flip-Flop (clock discipline)
//   D Flip-Flop → Register (multiple bits)
//   Register → Shift Register (serial I/O)
//   Register → Counter (self-incrementing)
//
// # Why this matters for GPUs
//
// A modern GPU has billions of flip-flops organized into registers,
// register files, and caches. Each shader core has its own register
// file holding intermediate computation values. Understanding how
// a single flip-flop works is the foundation for understanding how
// a GPU manages thousands of threads simultaneously.

// =========================================================================
// Types
// =========================================================================

// FlipFlopState holds the internal state of a master-slave D flip-flop.
//
// A master-slave flip-flop is actually TWO latches chained together:
//
//	         ┌─────────┐     ┌─────────┐
//	Data ──▶│  Master  │───▶│  Slave   │──▶ Q
//	         │  Latch   │    │  Latch   │──▶ Q̄
//	Clock ──▶│ (enabled │    │ (enabled │
//	         │  when    │    │  when    │
//	         │  clk=1)  │    │  clk=0)  │
//	         └─────────┘     └─────────┘
//
// The master captures data when clock is HIGH.
// The slave captures the master's output when clock is LOW.
// This two-phase operation prevents data from "racing through"
// both latches in a single clock cycle.
type FlipFlopState struct {
	MasterQ    int // Master latch output
	MasterQBar int // Master latch complementary output
	SlaveQ     int // Slave latch output (this is the flip-flop's Q)
	SlaveQBar  int // Slave latch complementary output (this is Q̄)
}

// CounterState holds the internal state of a binary counter.
//
// A counter is a register that increments its own value on each
// clock pulse. It wraps around when all bits are 1 (like an
// odometer rolling over from 999 to 000).
//
//	Width=4 counter sequence:
//	0000 → 0001 → 0010 → 0011 → 0100 → ... → 1111 → 0000
type CounterState struct {
	Bits  []int // Current count as individual bits (LSB first)
	Width int   // Number of bits in the counter
}

// =========================================================================
// SR Latch — The Simplest Memory Element
// =========================================================================
//
// An SR (Set-Reset) latch is the most basic form of digital memory.
// It is built from two NOR gates whose outputs feed back into each
// other's inputs, creating a stable feedback loop.
//
// Circuit diagram:
//
//	Set ────┐
//	        │    ┌─────┐
//	        ├───▶│ NOR │───┬──▶ Q
//	   ┌────┘    └─────┘   │
//	   │                   │
//	   │    ┌──────────────┘
//	   │    │
//	   │    ▼
//	   │    ┌─────┐
//	   └───▶│ NOR │───┬──▶ Q̄
//	        └─────┘   │
//	        ▲         │
//	        │         │
//	Reset ──┘    ┌────┘
//	             │
//	             ▼ (feeds back to top NOR)
//
// The cross-coupling creates bistability: the circuit has two stable
// states and will hold whichever one it's in until actively changed.
//
// Truth table:
//
//	S | R | Q    | Q̄   | Action
//	--|---|------|------|--------
//	0 | 0 | hold | hold | Remember (no change)
//	1 | 0 |  1   |  0   | Set (Q becomes 1)
//	0 | 1 |  0   |  1   | Reset (Q becomes 0)
//	1 | 1 |  0   |  0   | Invalid! (Q and Q̄ should be complements)
//
// The S=1, R=1 state is "invalid" because it forces both outputs
// to 0, violating the invariant that Q̄ = NOT(Q). When both inputs
// return to 0 simultaneously, the final state is unpredictable
// (a "race condition"). Real designs avoid this input combination.

// SRLatch simulates one evaluation step of an SR latch built from
// two cross-coupled NOR gates.
//
// Parameters:
//   - set: the Set input (1 to store a 1)
//   - reset: the Reset input (1 to store a 0)
//   - q: current Q output (previous state)
//   - qBar: current Q̄ output (previous state)
//
// Returns the new (q, qBar) after one gate evaluation.
//
// Note: In real hardware, the latch settles through analog feedback.
// In our simulation, we compute one discrete step. For Set or Reset
// operations, one step is sufficient to reach the new stable state.
// For the hold state (S=0, R=0), the outputs remain unchanged.
func SRLatch(set, reset, q, qBar int) (int, int) {
	type srResult struct{ q, qBar int }
	result, _ := StartNew[srResult]("logic-gates.SRLatch", srResult{},
		func(op *Operation[srResult], rf *ResultFactory[srResult]) *OperationResult[srResult] {
			op.AddProperty("set", set)
			op.AddProperty("reset", reset)
			op.AddProperty("q", q)
			op.AddProperty("qBar", qBar)
			validateBit(set, "set")
			validateBit(reset, "reset")
			validateBit(q, "q")
			validateBit(qBar, "qBar")

			// The SR latch equations (NOR-based):
			//   Q    = NOR(Reset, Q̄)
			//   Q̄    = NOR(Set,   Q)
			//
			// In real hardware, these two NOR gates evaluate simultaneously
			// through analog feedback until the circuit reaches a stable
			// state. We simulate this by iterating: compute both gates
			// using current values, then repeat until nothing changes.
			//
			// Convergence is guaranteed for valid inputs (S=0/R=0, S=1/R=0,
			// S=0/R=1) because each iteration moves toward the stable point.
			// For S=1/R=1 (invalid), both outputs converge to 0.
			// Maximum iterations is bounded at 10 as a safety net, though
			// in practice it converges in 2-3 iterations.
			currentQ := q
			currentQBar := qBar

			for i := 0; i < 10; i++ {
				newQ := NOR(reset, currentQBar)
				newQBar := NOR(set, newQ)

				if newQ == currentQ && newQBar == currentQBar {
					break // Stable state reached
				}
				currentQ = newQ
				currentQBar = newQBar
			}

			return rf.Generate(true, false, srResult{q: currentQ, qBar: currentQBar})
		}).PanicOnUnexpected().GetResult()
	return result.q, result.qBar
}

// =========================================================================
// D Latch — Taming the SR Latch
// =========================================================================
//
// The SR latch has two problems:
//   1. The S=1, R=1 input is invalid
//   2. Two separate inputs (S and R) for one bit of data is clumsy
//
// The D (Data) latch solves both by adding a front-end that generates
// S and R from a single data input D and an enable signal:
//
//	           ┌─────┐
//	Data ──┬──▶│ AND │──▶ Set ──────┐
//	       │   └─────┘              │
//	       │       ▲                ▼
//	       │       │           ┌─────────┐
//	Enable ├───────┤           │ SR      │──▶ Q
//	       │       │           │ Latch   │──▶ Q̄
//	       │   ┌───┘           └─────────┘
//	       │   │                    ▲
//	       ▼   │                    │
//	    ┌─────┐│                    │
//	    │ NOT ││   ┌─────┐          │
//	    └──┬──┘└──▶│ AND │──▶ Reset ┘
//	       └──────▶│     │
//	               └─────┘
//
// When Enable = 1 (latch is "transparent"):
//   - If D=1: Set=1, Reset=0 → Q becomes 1
//   - If D=0: Set=0, Reset=1 → Q becomes 0
//   - Q follows D (the latch is transparent)
//
// When Enable = 0 (latch is "opaque"):
//   - Set=0, Reset=0 → Q holds its current value
//   - Changes to D are ignored
//
// The D latch NEVER produces S=1, R=1, eliminating the invalid state.

// DLatch simulates one evaluation step of a D latch.
//
// Parameters:
//   - data: the data input (what value to store)
//   - enable: when 1, the latch is transparent (output follows input)
//   - q: current Q output (previous state)
//   - qBar: current Q̄ output (previous state)
//
// Returns the new (q, qBar) after one evaluation.
func DLatch(data, enable, q, qBar int) (int, int) {
	type dlResult struct{ q, qBar int }
	result, _ := StartNew[dlResult]("logic-gates.DLatch", dlResult{},
		func(op *Operation[dlResult], rf *ResultFactory[dlResult]) *OperationResult[dlResult] {
			op.AddProperty("data", data)
			op.AddProperty("enable", enable)
			op.AddProperty("q", q)
			op.AddProperty("qBar", qBar)
			validateBit(data, "data")
			validateBit(enable, "enable")
			validateBit(q, "q")
			validateBit(qBar, "qBar")

			// Generate Set and Reset from Data and Enable:
			//   Set   = Data AND Enable
			//   Reset = NOT(Data) AND Enable
			//
			// When Enable=0: both Set and Reset are 0 → hold state.
			// When Enable=1: exactly one of Set/Reset is 1 → valid.
			set := AND(data, enable)
			reset := AND(NOT(data), enable)

			newQ, newQBar := SRLatch(set, reset, q, qBar)
			return rf.Generate(true, false, dlResult{q: newQ, qBar: newQBar})
		}).PanicOnUnexpected().GetResult()
	return result.q, result.qBar
}

// =========================================================================
// D Flip-Flop — Edge-Triggered Memory
// =========================================================================
//
// The D latch has a subtle problem: while Enable is HIGH, the output
// continuously follows the input. If the input changes multiple times
// while Enable is HIGH, all those changes propagate through. This
// "transparency" makes it hard to build reliable pipelines.
//
// The D flip-flop solves this with the master-slave technique:
// two D latches in series, with opposite enable signals.
//
//	         ┌──────────────┐    ┌──────────────┐
//	Data ───▶│ Master Latch │───▶│ Slave Latch  │───▶ Q
//	         │ Enable=Clock │    │ Enable=¬Clock │───▶ Q̄
//	Clock ──▶│              │    │              │
//	         └──────────────┘    └──────────────┘
//
// Behavior:
//   - Clock = 1: Master is transparent (captures Data), Slave is opaque
//   - Clock = 0: Master is opaque, Slave is transparent (passes to Q)
//
// The key insight: data can enter the master while clock is HIGH, but
// it cannot reach the slave until clock goes LOW. This means the output
// changes only once per clock cycle, at the falling edge. This creates
// a disciplined, synchronized system where all flip-flops update together.
//
// (Note: Some flip-flop designs trigger on the rising edge instead.
// Our master-slave design triggers on the falling edge of the clock,
// which is the classic textbook implementation.)

// DFlipFlop simulates a master-slave D flip-flop over one clock phase.
//
// Parameters:
//   - data: the data input
//   - clock: the clock signal (0 or 1)
//   - state: the current internal state (pass nil for initial state)
//
// Returns (q, qBar, newState):
//   - q: the current output of the flip-flop
//   - qBar: the complementary output
//   - newState: updated internal state for the next call
//
// Usage pattern — call once per clock phase:
//
//	state := &FlipFlopState{0, 1, 0, 1}  // initial: Q=0
//	_, _, state = DFlipFlop(1, 1, state)  // clock HIGH: master captures 1
//	q, _, state = DFlipFlop(1, 0, state)  // clock LOW: slave outputs 1
//	// q is now 1
func DFlipFlop(data, clock int, state *FlipFlopState) (int, int, *FlipFlopState) {
	type dffResult struct {
		q, qBar  int
		newState *FlipFlopState
	}
	result, _ := StartNew[dffResult]("logic-gates.DFlipFlop", dffResult{},
		func(op *Operation[dffResult], rf *ResultFactory[dffResult]) *OperationResult[dffResult] {
			op.AddProperty("data", data)
			op.AddProperty("clock", clock)
			validateBit(data, "data")
			validateBit(clock, "clock")

			if state != nil {
				validateFlipFlopState(state, "state")
			}

			if state == nil {
				state = &FlipFlopState{
					MasterQ:    0,
					MasterQBar: 1,
					SlaveQ:     0,
					SlaveQBar:  1,
				}
			}

			// Master latch: enabled when clock = 1
			masterQ, masterQBar := DLatch(data, clock, state.MasterQ, state.MasterQBar)

			// Slave latch: enabled when clock = 0 (NOT clock)
			slaveQ, slaveQBar := DLatch(masterQ, NOT(clock), state.SlaveQ, state.SlaveQBar)

			newState := &FlipFlopState{
				MasterQ:    masterQ,
				MasterQBar: masterQBar,
				SlaveQ:     slaveQ,
				SlaveQBar:  slaveQBar,
			}

			return rf.Generate(true, false, dffResult{q: slaveQ, qBar: slaveQBar, newState: newState})
		}).PanicOnUnexpected().GetResult()
	return result.q, result.qBar, result.newState
}

// =========================================================================
// Register — N Bits of Parallel Storage
// =========================================================================
//
// A register is simply N flip-flops sharing the same clock signal.
// Each flip-flop stores one bit, so an N-bit register stores N bits.
//
//	        ┌──────┐
//	D[0] ──▶│ DFF  │──▶ Q[0]
//	        └──┬───┘
//	        ┌──┴───┐
//	D[1] ──▶│ DFF  │──▶ Q[1]
//	        └──┬───┘
//	        ┌──┴───┐
//	D[2] ──▶│ DFF  │──▶ Q[2]
//	        └──┬───┘
//	           │
//	Clock ─────┘ (shared by all flip-flops)
//
// Real-world use: CPU registers (like x86's EAX, EBX), GPU shader
// registers, pipeline stage registers, memory address registers.
// A 64-bit CPU register is literally 64 flip-flops sharing a clock.

// Register simulates an N-bit register (N parallel D flip-flops).
//
// Parameters:
//   - data: N-bit input data (each element must be 0 or 1)
//   - clock: the shared clock signal
//   - state: slice of N FlipFlopStates (pass nil for initial state)
//
// Returns (outputs, newState):
//   - outputs: the N-bit output (Q from each flip-flop)
//   - newState: updated state for the next call
func Register(data []int, clock int, state []FlipFlopState) ([]int, []FlipFlopState) {
	type regResult struct {
		outputs  []int
		newState []FlipFlopState
	}
	result, _ := StartNew[regResult]("logic-gates.Register", regResult{},
		func(op *Operation[regResult], rf *ResultFactory[regResult]) *OperationResult[regResult] {
			op.AddProperty("clock", clock)
			validateBit(clock, "clock")
			validateBits(data, "data")

			n := len(data)
			if n == 0 {
				panic("logicgates: Register requires at least 1 bit of data")
			}

			// Initialize state if nil
			if state == nil {
				state = make([]FlipFlopState, n)
				for i := range state {
					state[i] = FlipFlopState{0, 1, 0, 1}
				}
			}

			if len(state) != n {
				panic("logicgates: Register data and state length mismatch")
			}
			for i := range state {
				validateFlipFlopState(&state[i], fmt.Sprintf("state[%d]", i))
			}

			outputs := make([]int, n)
			newState := make([]FlipFlopState, n)

			// Each bit gets its own flip-flop, all sharing the same clock.
			// In hardware, these all evaluate simultaneously (parallelism!).
			for i := 0; i < n; i++ {
				s := state[i]
				q, _, ns := DFlipFlop(data[i], clock, &s)
				outputs[i] = q
				newState[i] = *ns
			}

			return rf.Generate(true, false, regResult{outputs: outputs, newState: newState})
		}).PanicOnUnexpected().GetResult()
	return result.outputs, result.newState
}

// =========================================================================
// Shift Register — Moving Bits Along a Chain
// =========================================================================
//
// A shift register is a chain of flip-flops where each one feeds
// into the next. On each clock cycle, every bit shifts one position
// left or right, and a new bit enters from the serial input.
//
// Left shift (data flows from LSB to MSB):
//
//	SerialIn ──▶ [DFF 0] ──▶ [DFF 1] ──▶ [DFF 2] ──▶ SerialOut
//	               Q[0]        Q[1]        Q[2]
//
// Right shift (data flows from MSB to LSB):
//
//	SerialOut ◀── [DFF 0] ◀── [DFF 1] ◀── [DFF 2] ◀── SerialIn
//	               Q[0]        Q[1]        Q[2]
//
// Real-world use:
//   - Serial-to-parallel conversion (UART receivers)
//   - Parallel-to-serial conversion (SPI transmitters)
//   - Delay lines (audio effects, video processing)
//   - Pseudo-random number generators (LFSR)
//   - GPU texture coordinate interpolation

// ShiftRegister simulates one clock cycle of an N-bit shift register.
//
// Parameters:
//   - serialIn: the bit entering the register
//   - clock: the clock signal
//   - state: slice of FlipFlopStates (pass nil for initial state)
//   - direction: "left" (toward MSB) or "right" (toward LSB)
//
// Returns (outputs, serialOut, newState):
//   - outputs: all N bits of the register
//   - serialOut: the bit that was shifted out
//   - newState: updated state for the next call
func ShiftRegister(serialIn, clock int, state []FlipFlopState, direction string) ([]int, int, []FlipFlopState) {
	type srResult struct {
		outputs   []int
		serialOut int
		newState  []FlipFlopState
	}
	result, _ := StartNew[srResult]("logic-gates.ShiftRegister", srResult{},
		func(op *Operation[srResult], rf *ResultFactory[srResult]) *OperationResult[srResult] {
			op.AddProperty("serialIn", serialIn)
			op.AddProperty("clock", clock)
			op.AddProperty("direction", direction)
			validateBit(serialIn, "serialIn")
			validateBit(clock, "clock")

			if direction != "left" && direction != "right" {
				panic("logicgates: ShiftRegister direction must be \"left\" or \"right\"")
			}

			if state == nil || len(state) == 0 {
				panic("logicgates: ShiftRegister requires non-empty state")
			}

			n := len(state)
			for i := range state {
				validateFlipFlopState(&state[i], fmt.Sprintf("state[%d]", i))
			}
			outputs := make([]int, n)
			newState := make([]FlipFlopState, n)

			// Capture current outputs before shifting (to determine serialOut)
			currentOutputs := make([]int, n)
			for i := 0; i < n; i++ {
				currentOutputs[i] = state[i].SlaveQ
			}

			if direction == "left" {
				// Left shift: bit 0 gets serialIn, bit i gets old bit i-1
				// Serial out is the MSB (last bit)
				//
				//   serialIn → [0] → [1] → [2] → serialOut
				//
				// Each flip-flop's data input is the PREVIOUS flip-flop's
				// current output (before this clock edge).
				for i := 0; i < n; i++ {
					var dataIn int
					if i == 0 {
						dataIn = serialIn
					} else {
						dataIn = currentOutputs[i-1]
					}
					s := state[i]
					q, _, ns := DFlipFlop(dataIn, clock, &s)
					outputs[i] = q
					newState[i] = *ns
				}
				return rf.Generate(true, false, srResult{outputs: outputs, serialOut: currentOutputs[n-1], newState: newState})
			}

			// Right shift: last bit gets serialIn, bit i gets old bit i+1
			// Serial out is the LSB (first bit)
			//
			//   serialOut ← [0] ← [1] ← [2] ← serialIn
			for i := n - 1; i >= 0; i-- {
				var dataIn int
				if i == n-1 {
					dataIn = serialIn
				} else {
					dataIn = currentOutputs[i+1]
				}
				s := state[i]
				q, _, ns := DFlipFlop(dataIn, clock, &s)
				outputs[i] = q
				newState[i] = *ns
			}
			return rf.Generate(true, false, srResult{outputs: outputs, serialOut: currentOutputs[0], newState: newState})
		}).PanicOnUnexpected().GetResult()
	return result.outputs, result.serialOut, result.newState
}

// =========================================================================
// Counter — A Self-Incrementing Register
// =========================================================================
//
// A binary counter combines a register with an incrementer circuit.
// On each clock pulse, it adds 1 to its current value. When it
// reaches all 1s, it wraps around to all 0s.
//
// The incrementer works by chaining XOR gates with carry propagation:
//
//	Bit 0: always toggles (XOR with carry_in=1)
//	Bit 1: toggles when bit 0 is 1 (carry from bit 0)
//	Bit 2: toggles when bits 0 AND 1 are 1 (carry from bit 1)
//	...
//
// This is a "ripple carry" incrementer — the carry ripples from
// LSB to MSB, just like carrying in decimal addition:
//
//	  0 0 1 1  (decimal 3)
//	+       1
//	---------
//	  0 1 0 0  (decimal 4, carry rippled through bits 0 and 1)
//
// In hardware, the carry chain limits the maximum clock frequency.
// Real CPUs use "carry lookahead" to speed this up, but the ripple
// carry is the simplest to understand.
//
// Real-world use: program counters (PC), timer circuits, memory
// refresh counters, GPU thread ID generators.

// Counter simulates one clock cycle of an N-bit binary counter.
//
// Parameters:
//   - clock: the clock signal
//   - reset: when 1, asynchronously resets the counter to 0
//   - state: the current counter state (pass nil for initial state)
//
// Returns (outputs, newState):
//   - outputs: the current N-bit count (LSB first)
//   - newState: updated state for the next call
func Counter(clock, reset int, state *CounterState) ([]int, *CounterState) {
	type ctrResult struct {
		outputs  []int
		newState *CounterState
	}
	result, _ := StartNew[ctrResult]("logic-gates.Counter", ctrResult{},
		func(op *Operation[ctrResult], rf *ResultFactory[ctrResult]) *OperationResult[ctrResult] {
			op.AddProperty("clock", clock)
			op.AddProperty("reset", reset)
			validateBit(clock, "clock")
			validateBit(reset, "reset")

			if state == nil {
				panic("logicgates: Counter requires non-nil state")
			}

			width := state.Width
			if width < 1 {
				panic("logicgates: Counter width must be at least 1")
			}
			if width > MaxCounterWidth {
				panic(fmt.Sprintf("logicgates: Counter width %d exceeds the maximum %d", width, MaxCounterWidth))
			}

			// Initialize bits if empty; otherwise the length must match Width
			// BEFORE any indexing, and every bit must be 0 or 1 on every path
			// (hold and reset included).
			if len(state.Bits) == 0 {
				state.Bits = make([]int, width)
			}
			if len(state.Bits) != width {
				panic(fmt.Sprintf("logicgates: Counter state.Bits length %d does not match Width %d", len(state.Bits), width))
			}
			validateBits(state.Bits, "state.Bits")

			// Asynchronous reset: immediately clear all bits
			if reset == 1 {
				newBits := make([]int, width)
				newState := &CounterState{Bits: newBits, Width: width}
				return rf.Generate(true, false, ctrResult{outputs: append([]int(nil), newBits...), newState: newState})
			}

			// On clock = 1, increment the counter
			// On clock = 0, hold the current value
			if clock == 0 {
				output := make([]int, width)
				copy(output, state.Bits)
				newState := &CounterState{Bits: output, Width: width}
				return rf.Generate(true, false, ctrResult{outputs: append([]int(nil), output...), newState: newState})
			}

			// Increment using ripple carry:
			//   new_bit[i] = XOR(old_bit[i], carry[i])
			//   carry[i+1] = AND(old_bit[i], carry[i])
			//   carry[0] = 1 (we're adding 1)
			//
			// This is exactly how binary addition works, one bit at a time.
			newBits := make([]int, width)
			carry := 1 // Start with carry = 1 (adding 1 to the counter)

			for i := 0; i < width; i++ {
				newBits[i] = XOR(state.Bits[i], carry)
				carry = AND(state.Bits[i], carry)
			}

			newState := &CounterState{Bits: newBits, Width: width}
			return rf.Generate(true, false, ctrResult{outputs: append([]int(nil), newBits...), newState: newState})
		}).PanicOnUnexpected().GetResult()
	return result.outputs, result.newState
}

// validateFlipFlopState rejects a non-bit in any FlipFlopState field up front,
// naming the field (e.g. "state.MasterQ", "state[2].SlaveQBar").
func validateFlipFlopState(s *FlipFlopState, name string) {
	validateBit(s.MasterQ, name+".MasterQ")
	validateBit(s.MasterQBar, name+".MasterQBar")
	validateBit(s.SlaveQ, name+".SlaveQ")
	validateBit(s.SlaveQBar, name+".SlaveQBar")
}
