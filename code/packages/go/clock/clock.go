// Package clock simulates the system clock that drives all sequential logic.
//
// # Clock -- the heartbeat of every digital circuit
//
// Every sequential circuit in a computer -- flip-flops, registers, counters,
// CPU pipeline stages, GPU cores -- is driven by a clock signal. The clock
// is a square wave that alternates between 0 and 1:
//
//	+--+  +--+  +--+  +--+
//	|  |  |  |  |  |  |  |
//	---+  +--+  +--+  +--+  +--
//
// On each rising edge (0->1), flip-flops capture their inputs. This is
// what makes synchronous digital logic work -- everything happens in
// lockstep, driven by the clock.
//
// In real hardware:
//   - CPU clock: 3-5 GHz (3-5 billion cycles per second)
//   - GPU clock: 1-2 GHz
//   - Memory clock: 4-8 GHz (DDR5)
//   - The clock frequency is the single most important performance number
//
// # Why does the clock matter?
//
// Without a clock, digital circuits would be chaotic. Imagine a chain of
// logic gates where each gate has a slightly different propagation delay.
// Without synchronization, signals would arrive at different times and
// produce garbage. The clock solves this by saying: "Everyone, capture
// your inputs NOW." This is called synchronous design.
//
// The clock period must be long enough for the slowest signal path to
// settle. This slowest path is called the "critical path," and it
// determines the maximum clock frequency.
//
// # Half-cycles and edges
//
// A single clock cycle has two halves:
//
//	Tick 0: value goes 0 -> 1 (RISING EDGE)   <- most circuits trigger here
//	Tick 1: value goes 1 -> 0 (FALLING EDGE)   <- some DDR circuits use this too
//
// "DDR" (Double Data Rate) memory uses BOTH edges, which is why DDR5-6400
// actually runs at 3200 MHz but transfers data on both rising and falling
// edges, achieving 6400 MT/s (megatransfers per second).
package clock

import (
	"fmt"
)

// ---------------------------------------------------------------------------
// ClockEdge -- a record of one transition
// ---------------------------------------------------------------------------

// ClockEdge records a single clock transition.
//
// Every time the clock ticks, it produces an edge. An edge captures:
//   - Which cycle we are in (cycles count from 1)
//   - The current signal level (0 or 1)
//   - Whether this was a rising edge (0->1) or falling edge (1->0)
//
// Think of it like a timestamp in a logic analyzer trace.
type ClockEdge struct {
	Cycle     int  // Which cycle this edge belongs to (starts at 1)
	Value     int  // Current level after the transition (0 or 1)
	IsRising  bool // True if this was a 0->1 transition
	IsFalling bool // True if this was a 1->0 transition
}

// Listener is a function that receives clock edges.
// Components register listeners to react to clock transitions.
// In real hardware, this is just electrical connectivity -- the
// clock wire is physically connected to every component.
type Listener func(edge ClockEdge)

// ---------------------------------------------------------------------------
// Clock -- the main square-wave generator
// ---------------------------------------------------------------------------

// Clock is a system clock generator.
//
// The clock maintains a cycle count and alternates between low (0) and
// high (1) on each tick. Components connect to the clock and react to
// edges (transitions).
//
// A complete cycle is: low -> high -> low (two ticks).
//
// Example usage:
//
//	clk := clock.New(1_000_000)  // 1 MHz
//	edge := clk.Tick()           // rising edge, cycle 1
//	edge = clk.Tick()            // falling edge, cycle 1
//	edge = clk.Tick()            // rising edge, cycle 2
//
// The observer pattern (listeners) allows components to react to clock
// edges without polling. This mirrors how real hardware works.
type Clock struct {
	FrequencyHz int        // Clock frequency in Hz
	Cycle       int        // Current cycle count (starts at 0, increments on rising edges)
	Value       int        // Current signal level (0 or 1)
	totalTicks  int        // Total half-cycles elapsed
	listeners   []Listener // Registered edge listeners
}

// New creates a new Clock with the given frequency in Hz.
//
// The clock starts at value 0 (low), cycle 0, with no ticks elapsed.
// This is the state of a real oscillator before it starts oscillating.
func New(frequencyHz int) *Clock {
	result, _ := StartNew[*Clock]("clock.New", nil,
		func(op *Operation[*Clock], rf *ResultFactory[*Clock]) *OperationResult[*Clock] {
			op.AddProperty("frequencyHz", frequencyHz)
			return rf.Generate(true, false, &Clock{
				FrequencyHz: frequencyHz,
				Cycle:       0,
				Value:       0,
				totalTicks:  0,
				listeners:   nil,
			})
		}).GetResult()
	return result
}

// Tick advances one half-cycle and returns the edge that occurred.
//
// The clock alternates like a toggle switch:
//   - If currently 0, goes to 1 (rising edge, new cycle starts)
//   - If currently 1, goes to 0 (falling edge, cycle ends)
//
// After toggling, all registered listeners are notified with the
// edge record. This is how connected components "see" the clock.
func (c *Clock) Tick() ClockEdge {
	result, _ := StartNew[ClockEdge]("clock.Tick", ClockEdge{},
		func(_ *Operation[ClockEdge], rf *ResultFactory[ClockEdge]) *OperationResult[ClockEdge] {
			oldValue := c.Value
			c.Value = 1 - c.Value
			c.totalTicks++
			isRising := oldValue == 0 && c.Value == 1
			isFalling := oldValue == 1 && c.Value == 0
			if isRising {
				c.Cycle++
			}
			edge := ClockEdge{
				Cycle:     c.Cycle,
				Value:     c.Value,
				IsRising:  isRising,
				IsFalling: isFalling,
			}
			for _, listener := range c.listeners {
				listener(edge)
			}
			return rf.Generate(true, false, edge)
		}).GetResult()
	return result
}

// FullCycle executes one complete cycle (rising + falling edge).
//
// A full cycle is two ticks:
//  1. Rising edge (0 -> 1): the "active" half
//  2. Falling edge (1 -> 0): the "idle" half
func (c *Clock) FullCycle() (ClockEdge, ClockEdge) {
	type fullCycleResult struct {
		rising  ClockEdge
		falling ClockEdge
	}
	res, _ := StartNew[fullCycleResult]("clock.FullCycle", fullCycleResult{},
		func(_ *Operation[fullCycleResult], rf *ResultFactory[fullCycleResult]) *OperationResult[fullCycleResult] {
			rising := c.Tick()
			falling := c.Tick()
			return rf.Generate(true, false, fullCycleResult{rising: rising, falling: falling})
		}).GetResult()
	return res.rising, res.falling
}

// Run executes N complete cycles and returns all edges.
//
// Since each cycle has two edges (rising + falling), running N cycles
// produces 2N edges total.
func (c *Clock) Run(cycles int) []ClockEdge {
	result, _ := StartNew[[]ClockEdge]("clock.Run", nil,
		func(op *Operation[[]ClockEdge], rf *ResultFactory[[]ClockEdge]) *OperationResult[[]ClockEdge] {
			op.AddProperty("cycles", cycles)
			edges := make([]ClockEdge, 0, cycles*2)
			for range cycles {
				r, f := c.FullCycle()
				edges = append(edges, r, f)
			}
			return rf.Generate(true, false, edges)
		}).GetResult()
	return result
}

// RegisterListener adds a function to be called on every clock edge.
//
// In real hardware, this is like connecting a wire from the clock
// to a component's clock input pin.
func (c *Clock) RegisterListener(listener Listener) {
	_, _ = StartNew[struct{}]("clock.RegisterListener", struct{}{},
		func(_ *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			c.listeners = append(c.listeners, listener)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// UnregisterListener removes a previously registered listener by index.
//
// Since Go functions are not comparable, we identify listeners by their
// position in the listener list. Use the index returned by counting
// from registration order.
//
// Returns an error if the index is out of bounds.
func (c *Clock) UnregisterListener(index int) error {
	if index < 0 || index >= len(c.listeners) {
		return fmt.Errorf("listener index %d out of range [0, %d)", index, len(c.listeners))
	}
	c.listeners = append(c.listeners[:index], c.listeners[index+1:]...)
	return nil
}

// ListenerCount returns the number of registered listeners.
func (c *Clock) ListenerCount() int {
	return len(c.listeners)
}

// Reset restores the clock to its initial state.
//
// Sets the value back to 0, cycle count to 0, and tick count to 0.
// Listeners are preserved -- only the timing state is reset.
// This is like hitting the reset button on an oscillator.
func (c *Clock) Reset() {
	c.Cycle = 0
	c.Value = 0
	c.totalTicks = 0
}

// PeriodNs returns the clock period in nanoseconds.
//
// The period is the time for one complete cycle (rising + falling).
// For a 1 GHz clock, the period is 1 ns. For 1 MHz, it is 1000 ns.
//
// Formula: period_ns = 1e9 / frequency_hz
func (c *Clock) PeriodNs() float64 {
	return 1e9 / float64(c.FrequencyHz)
}

// TotalTicks returns the total number of half-cycles elapsed.
func (c *Clock) TotalTicks() int {
	return c.totalTicks
}

// ---------------------------------------------------------------------------
// ClockDivider -- frequency division
// ---------------------------------------------------------------------------

// ClockDivider divides a clock frequency by an integer factor.
//
// In hardware, clock dividers are used to generate slower clocks from
// a fast master clock. For example, a 1 GHz CPU clock might be divided
// by 4 to get a 250 MHz bus clock.
//
// How it works:
//   - Count rising edges from the source clock
//   - Every `divisor` rising edges, generate one full cycle on the output
//
// Real-world uses:
//   - CPU-to-bus clock ratio (e.g., CPU at 4 GHz, bus at 1 GHz)
//   - USB clock derivation from system clock
//   - Audio sample rate generation from master clock
type ClockDivider struct {
	Source  *Clock // The faster source clock
	Divisor int    // Division factor
	Output  *Clock // The slower output clock
	counter int    // Rising edge counter
}

// NewClockDivider creates a clock divider.
//
// The divisor must be >= 2. The output clock's frequency is set to
// source.FrequencyHz / divisor.
//
// The divider automatically registers itself as a listener on the
// source clock, so it starts working immediately.
func NewClockDivider(source *Clock, divisor int) (*ClockDivider, error) {
	if divisor < 2 {
		return nil, fmt.Errorf("divisor must be >= 2, got %d", divisor)
	}

	cd := &ClockDivider{
		Source:  source,
		Divisor: divisor,
		Output:  New(source.FrequencyHz / divisor),
		counter: 0,
	}

	// Register ourselves as a listener on the source clock.
	source.RegisterListener(cd.onEdge)

	return cd, nil
}

// onEdge is called on every source clock edge.
//
// We only count rising edges. When we have counted `divisor` rising
// edges, we generate one complete output cycle (rising + falling).
func (cd *ClockDivider) onEdge(edge ClockEdge) {
	if edge.IsRising {
		cd.counter++
		if cd.counter >= cd.Divisor {
			cd.counter = 0
			cd.Output.Tick() // rising
			cd.Output.Tick() // falling
		}
	}
}

// ---------------------------------------------------------------------------
// MultiPhaseClock -- non-overlapping phase generation
// ---------------------------------------------------------------------------

// MultiPhaseClock generates multiple clock phases from a single source.
//
// Used in CPU pipelines where different stages need offset clocks.
// A 4-phase clock generates 4 non-overlapping clock signals, each
// active for 1/4 of the master cycle.
//
// Timing diagram for a 4-phase clock:
//
//	Source:  _|^|_|^|_|^|_|^|_
//	Phase 0: _|^|___|___|___|_
//	Phase 1: _|___|^|___|___|_
//	Phase 2: _|___|___|^|___|_
//	Phase 3: _|___|___|___|^|_
//
// On each rising edge of the source, exactly ONE phase is active (1)
// and all others are inactive (0). The active phase rotates.
//
// Real-world uses:
//   - Classic RISC pipelines (fetch, decode, execute, writeback)
//   - DRAM refresh timing
//   - Multiplexed bus access
type MultiPhaseClock struct {
	Source      *Clock // The master clock
	Phases      int    // Number of phases
	ActivePhase int    // Index of the currently active phase
	phaseValues []int  // Current value of each phase (0 or 1)
}

// NewMultiPhaseClock creates a multi-phase clock.
//
// The number of phases must be >= 2. The multi-phase clock registers
// itself as a listener on the source clock and starts working immediately.
func NewMultiPhaseClock(source *Clock, phases int) (*MultiPhaseClock, error) {
	if phases < 2 {
		return nil, fmt.Errorf("phases must be >= 2, got %d", phases)
	}

	mpc := &MultiPhaseClock{
		Source:      source,
		Phases:      phases,
		ActivePhase: 0,
		phaseValues: make([]int, phases),
	}

	source.RegisterListener(mpc.onEdge)

	return mpc, nil
}

// GetPhase returns the current value of phase N.
//
// Returns 1 if the phase is active, 0 if inactive.
func (mpc *MultiPhaseClock) GetPhase(index int) int {
	return mpc.phaseValues[index]
}

// onEdge is called on every source clock edge.
//
// On rising edges, we rotate the active phase. Only one phase
// is high at any time -- this is the "non-overlapping" property
// that prevents pipeline hazards.
func (mpc *MultiPhaseClock) onEdge(edge ClockEdge) {
	if edge.IsRising {
		// Reset all phases to 0
		for i := range mpc.phaseValues {
			mpc.phaseValues[i] = 0
		}
		// Activate the current phase
		mpc.phaseValues[mpc.ActivePhase] = 1
		// Rotate to next phase
		mpc.ActivePhase = (mpc.ActivePhase + 1) % mpc.Phases
	}
}
