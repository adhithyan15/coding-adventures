package logicgates

import (
	"testing"
)

// =========================================================================
// SR Latch Tests
// =========================================================================

func TestSRLatch_Set(t *testing.T) {
	// Setting the latch: S=1, R=0
	// Starting from Q=0, Q̄=1 (reset state)
	// After set: Q=1, Q̄=0
	q, qBar := SRLatch(1, 0, 0, 1)
	if q != 1 || qBar != 0 {
		t.Errorf("SRLatch Set: got Q=%d, Q̄=%d, want Q=1, Q̄=0", q, qBar)
	}
}

func TestSRLatch_Reset(t *testing.T) {
	// Resetting the latch: S=0, R=1
	// Starting from Q=1, Q̄=0 (set state)
	// After reset: Q=0, Q̄=1
	q, qBar := SRLatch(0, 1, 1, 0)
	if q != 0 || qBar != 1 {
		t.Errorf("SRLatch Reset: got Q=%d, Q̄=%d, want Q=0, Q̄=1", q, qBar)
	}
}

func TestSRLatch_Hold_AfterSet(t *testing.T) {
	// Hold state: S=0, R=0
	// The latch should remember its current state.
	// Starting from Q=1, Q̄=0 (was previously set)
	q, qBar := SRLatch(0, 0, 1, 0)
	if q != 1 || qBar != 0 {
		t.Errorf("SRLatch Hold(after set): got Q=%d, Q̄=%d, want Q=1, Q̄=0", q, qBar)
	}
}

func TestSRLatch_Hold_AfterReset(t *testing.T) {
	// Hold state starting from Q=0, Q̄=1 (was previously reset)
	q, qBar := SRLatch(0, 0, 0, 1)
	if q != 0 || qBar != 1 {
		t.Errorf("SRLatch Hold(after reset): got Q=%d, Q̄=%d, want Q=0, Q̄=1", q, qBar)
	}
}

func TestSRLatch_Invalid(t *testing.T) {
	// Invalid state: S=1, R=1
	// Both NOR gates output 0 (Q=0, Q̄=0).
	// This violates the invariant Q̄ = NOT(Q), which is why
	// this input combination is called "invalid" or "forbidden."
	q, qBar := SRLatch(1, 1, 0, 1)
	if q != 0 || qBar != 0 {
		t.Errorf("SRLatch Invalid: got Q=%d, Q̄=%d, want Q=0, Q̄=0", q, qBar)
	}
}

func TestSRLatch_InvalidInput(t *testing.T) {
	assertPanics(t, "SRLatch invalid set", func() { SRLatch(2, 0, 0, 1) })
	assertPanics(t, "SRLatch invalid reset", func() { SRLatch(0, 2, 0, 1) })
	assertPanics(t, "SRLatch invalid q", func() { SRLatch(0, 0, 2, 1) })
	assertPanics(t, "SRLatch invalid qBar", func() { SRLatch(0, 0, 0, 2) })
}

// =========================================================================
// D Latch Tests
// =========================================================================

func TestDLatch_Transparent_StoreOne(t *testing.T) {
	// When enable=1 and data=1, the latch becomes transparent
	// and stores a 1: Q=1, Q̄=0
	q, qBar := DLatch(1, 1, 0, 1)
	if q != 1 || qBar != 0 {
		t.Errorf("DLatch Transparent(D=1): got Q=%d, Q̄=%d, want Q=1, Q̄=0", q, qBar)
	}
}

func TestDLatch_Transparent_StoreZero(t *testing.T) {
	// When enable=1 and data=0, Q should become 0
	q, qBar := DLatch(0, 1, 1, 0)
	if q != 0 || qBar != 1 {
		t.Errorf("DLatch Transparent(D=0): got Q=%d, Q̄=%d, want Q=0, Q̄=1", q, qBar)
	}
}

func TestDLatch_Hold_WhenDisabled(t *testing.T) {
	// When enable=0, the latch holds its current value
	// regardless of what data does.

	// Hold a 1
	q, qBar := DLatch(0, 0, 1, 0)
	if q != 1 || qBar != 0 {
		t.Errorf("DLatch Hold(Q=1, D=0): got Q=%d, Q̄=%d, want Q=1, Q̄=0", q, qBar)
	}

	// Hold a 0
	q, qBar = DLatch(1, 0, 0, 1)
	if q != 0 || qBar != 1 {
		t.Errorf("DLatch Hold(Q=0, D=1): got Q=%d, Q̄=%d, want Q=0, Q̄=1", q, qBar)
	}
}

func TestDLatch_Transparent_FollowsData(t *testing.T) {
	// When enabled, output should track data changes.
	// Start with Q=0, set D=1 → Q=1
	q, qBar := DLatch(1, 1, 0, 1)
	if q != 1 {
		t.Fatalf("DLatch: expected Q=1 after D=1, enable=1")
	}

	// Now set D=0 → Q should become 0 (transparent)
	q, qBar = DLatch(0, 1, q, qBar)
	if q != 0 || qBar != 1 {
		t.Errorf("DLatch Follow(D=0): got Q=%d, Q̄=%d, want Q=0, Q̄=1", q, qBar)
	}
}

func TestDLatch_InvalidInput(t *testing.T) {
	assertPanics(t, "DLatch invalid data", func() { DLatch(2, 1, 0, 1) })
	assertPanics(t, "DLatch invalid enable", func() { DLatch(0, 2, 0, 1) })
	assertPanics(t, "DLatch invalid q", func() { DLatch(0, 1, 2, 1) })
	assertPanics(t, "DLatch invalid qBar", func() { DLatch(0, 1, 0, 2) })
}

// =========================================================================
// D Flip-Flop Tests
// =========================================================================

func TestDFlipFlop_EdgeCapture(t *testing.T) {
	// The flip-flop should capture data on the falling edge of clock.
	// Sequence:
	//   1. Clock HIGH with Data=1 → master captures, slave holds
	//   2. Clock LOW → slave outputs master's value

	state := &FlipFlopState{0, 1, 0, 1} // initial: Q=0

	// Phase 1: clock goes HIGH, data=1
	// Master captures data=1, but slave still holds old value
	q, _, state := DFlipFlop(1, 1, state)
	if q != 0 {
		t.Errorf("DFF Phase1 (clk=1): got Q=%d, want Q=0 (slave still holds old)", q)
	}

	// Phase 2: clock goes LOW
	// Slave captures master's value, Q becomes 1
	q, qBar, state := DFlipFlop(1, 0, state)
	if q != 1 || qBar != 0 {
		t.Errorf("DFF Phase2 (clk=0): got Q=%d, Q̄=%d, want Q=1, Q̄=0", q, qBar)
	}
}

func TestDFlipFlop_StoreZero(t *testing.T) {
	// Start with Q=1, store a 0
	state := &FlipFlopState{1, 0, 1, 0} // Q=1

	// Clock HIGH with Data=0 → master captures 0
	_, _, state = DFlipFlop(0, 1, state)

	// Clock LOW → slave outputs 0
	q, qBar, _ := DFlipFlop(0, 0, state)
	if q != 0 || qBar != 1 {
		t.Errorf("DFF StoreZero: got Q=%d, Q̄=%d, want Q=0, Q̄=1", q, qBar)
	}
}

func TestDFlipFlop_NilState(t *testing.T) {
	// Passing nil state should initialize to Q=0
	q, _, state := DFlipFlop(1, 1, nil)
	if state == nil {
		t.Fatal("DFF nil state: returned nil state")
	}
	// With nil init, slave Q should be 0 (clock=1 means slave is opaque)
	if q != 0 {
		t.Errorf("DFF nil state (clk=1): got Q=%d, want Q=0", q)
	}

	// Now clock LOW to get the value through
	q, _, _ = DFlipFlop(1, 0, state)
	if q != 1 {
		t.Errorf("DFF nil state (clk=0): got Q=%d, want Q=1", q)
	}
}

func TestDFlipFlop_DataChangeDuringClockHigh(t *testing.T) {
	// If data changes while clock is HIGH, the master tracks it.
	// Only the value present when clock goes LOW gets captured.
	state := &FlipFlopState{0, 1, 0, 1}

	// Clock HIGH, data=1 → master captures 1
	_, _, state = DFlipFlop(1, 1, state)

	// Clock still HIGH, data changes to 0 → master updates to 0
	_, _, state = DFlipFlop(0, 1, state)

	// Clock LOW → slave captures master's current value (0)
	q, _, _ := DFlipFlop(0, 0, state)
	if q != 0 {
		t.Errorf("DFF data change during HIGH: got Q=%d, want Q=0", q)
	}
}

func TestDFlipFlop_InvalidInput(t *testing.T) {
	state := &FlipFlopState{0, 1, 0, 1}
	assertPanics(t, "DFF invalid data", func() { DFlipFlop(2, 0, state) })
	assertPanics(t, "DFF invalid clock", func() { DFlipFlop(0, 2, state) })
}

func TestDFlipFlop_HoldDuringClockLow(t *testing.T) {
	// When clock stays LOW, slave is transparent but master is opaque.
	// If data changes during clock LOW, it should NOT affect the output
	// because the master is not capturing.
	state := &FlipFlopState{0, 1, 0, 1}

	// First, store a 1 normally
	_, _, state = DFlipFlop(1, 1, state) // master captures 1
	_, _, state = DFlipFlop(1, 0, state) // slave outputs 1

	// Now clock stays LOW, data changes to 0 — should NOT affect output
	q, _, _ := DFlipFlop(0, 0, state)
	if q != 1 {
		t.Errorf("DFF hold during LOW: got Q=%d, want Q=1", q)
	}
}

// =========================================================================
// Register Tests
// =========================================================================

func TestRegister_StoreAndRetrieve(t *testing.T) {
	// Store the 4-bit value 1010 (binary) in a register.
	data := []int{1, 0, 1, 0}

	// Phase 1: clock HIGH — master latches capture data
	_, state := Register(data, 1, nil)

	// Phase 2: clock LOW — slave latches output data
	outputs, _ := Register(data, 0, state)

	for i, want := range data {
		if outputs[i] != want {
			t.Errorf("Register[%d] = %d, want %d", i, outputs[i], want)
		}
	}
}

func TestRegister_HoldValue(t *testing.T) {
	// After storing 1010, feeding different data with no clock edge
	// should not change the output.
	data := []int{1, 0, 1, 0}

	// Store it
	_, state := Register(data, 1, nil)
	outputs, state := Register(data, 0, state)

	// Feed different data (0101) but keep clock LOW — register holds
	newData := []int{0, 1, 0, 1}
	outputs, _ = Register(newData, 0, state)

	for i, want := range data {
		if outputs[i] != want {
			t.Errorf("Register Hold[%d] = %d, want %d (original data)", i, outputs[i], want)
		}
	}
}

func TestRegister_Overwrite(t *testing.T) {
	// Store 1010 then overwrite with 0101
	data1 := []int{1, 0, 1, 0}
	data2 := []int{0, 1, 0, 1}

	// Store data1
	_, state := Register(data1, 1, nil)
	_, state = Register(data1, 0, state)

	// Store data2
	_, state = Register(data2, 1, state)
	outputs, _ := Register(data2, 0, state)

	for i, want := range data2 {
		if outputs[i] != want {
			t.Errorf("Register Overwrite[%d] = %d, want %d", i, outputs[i], want)
		}
	}
}

func TestRegister_InvalidInput(t *testing.T) {
	assertPanics(t, "Register empty data", func() { Register([]int{}, 1, nil) })
	assertPanics(t, "Register invalid clock", func() { Register([]int{0, 1}, 2, nil) })
	assertPanics(t, "Register invalid bit", func() { Register([]int{0, 2}, 1, nil) })
	assertPanics(t, "Register state mismatch", func() {
		state := make([]FlipFlopState, 3)
		Register([]int{0, 1}, 1, state)
	})
}

// =========================================================================
// Shift Register Tests
// =========================================================================

func makeShiftState(n int) []FlipFlopState {
	state := make([]FlipFlopState, n)
	for i := range state {
		state[i] = FlipFlopState{0, 1, 0, 1}
	}
	return state
}

// clockCycle performs a full clock cycle (HIGH then LOW) for the shift register.
func shiftCycle(serialIn int, state []FlipFlopState, direction string) ([]int, int, []FlipFlopState) {
	_, _, state = ShiftRegister(serialIn, 1, state, direction)
	return ShiftRegister(serialIn, 0, state, direction)
}

func TestShiftRegister_LeftShift(t *testing.T) {
	// Shift 1s in from the left (bit 0) side.
	// After each cycle, bits should move right (toward MSB index).
	//
	// Initial:   [0, 0, 0, 0]
	// Shift 1:   [1, 0, 0, 0]  serialOut=0
	// Shift 1:   [1, 1, 0, 0]  serialOut=0
	// Shift 1:   [1, 1, 1, 0]  serialOut=0
	// Shift 1:   [1, 1, 1, 1]  serialOut=0
	// Shift 0:   [0, 1, 1, 1]  serialOut=1

	state := makeShiftState(4)

	for i := 0; i < 4; i++ {
		var outputs []int
		var serialOut int
		outputs, serialOut, state = shiftCycle(1, state, "left")
		_ = outputs
		if i < 3 && serialOut != 0 {
			t.Errorf("Left shift step %d: serialOut=%d, want 0", i, serialOut)
		}
	}

	// Now shift in a 0 — the 1 at position 3 should come out
	outputs, serialOut, _ := shiftCycle(0, state, "left")
	if serialOut != 1 {
		t.Errorf("Left shift final: serialOut=%d, want 1", serialOut)
	}
	if outputs[0] != 0 {
		t.Errorf("Left shift final: bit[0]=%d, want 0", outputs[0])
	}
}

func TestShiftRegister_RightShift(t *testing.T) {
	// Shift 1s in from the right (MSB) side.
	// Initial:   [0, 0, 0, 0]
	// Shift 1:   [0, 0, 0, 1]  serialOut=0
	// Shift 1:   [0, 0, 1, 1]  serialOut=0
	// ...
	// Shift 0 after filling: serialOut=1

	state := makeShiftState(4)

	for i := 0; i < 4; i++ {
		_, _, state = shiftCycle(1, state, "right")
	}

	// All bits should be 1 now, shift in 0
	outputs, serialOut, _ := shiftCycle(0, state, "right")
	if serialOut != 1 {
		t.Errorf("Right shift: serialOut=%d, want 1", serialOut)
	}
	if outputs[3] != 0 {
		t.Errorf("Right shift: bit[3]=%d, want 0", outputs[3])
	}
}

func TestShiftRegister_InvalidInput(t *testing.T) {
	state := makeShiftState(4)
	assertPanics(t, "ShiftRegister invalid serialIn", func() { ShiftRegister(2, 1, state, "left") })
	assertPanics(t, "ShiftRegister invalid clock", func() { ShiftRegister(0, 2, state, "left") })
	assertPanics(t, "ShiftRegister invalid direction", func() { ShiftRegister(0, 1, state, "up") })
	assertPanics(t, "ShiftRegister nil state", func() { ShiftRegister(0, 1, nil, "left") })
	assertPanics(t, "ShiftRegister empty state", func() { ShiftRegister(0, 1, []FlipFlopState{}, "left") })
}

func TestShiftRegister_SingleBit(t *testing.T) {
	// Edge case: 1-bit shift register
	state := makeShiftState(1)

	outputs, serialOut, state := shiftCycle(1, state, "left")
	if outputs[0] != 1 {
		t.Errorf("1-bit shift: got %d, want 1", outputs[0])
	}
	if serialOut != 0 {
		t.Errorf("1-bit shift: serialOut=%d, want 0", serialOut)
	}

	outputs, serialOut, _ = shiftCycle(0, state, "left")
	if outputs[0] != 0 {
		t.Errorf("1-bit shift second: got %d, want 0", outputs[0])
	}
	if serialOut != 1 {
		t.Errorf("1-bit shift second: serialOut=%d, want 1", serialOut)
	}
}

// =========================================================================
// Counter Tests
// =========================================================================

func TestCounter_CountUp(t *testing.T) {
	// A 4-bit counter should count from 0 to 15 then wrap to 0.
	//
	// Count sequence (LSB first):
	//   0: [0,0,0,0]
	//   1: [1,0,0,0]
	//   2: [0,1,0,0]
	//   3: [1,1,0,0]
	//   ...
	//  15: [1,1,1,1]
	//   0: [0,0,0,0]  (wrap!)

	state := &CounterState{Bits: []int{0, 0, 0, 0}, Width: 4}

	expected := [][]int{
		{1, 0, 0, 0}, // 1
		{0, 1, 0, 0}, // 2
		{1, 1, 0, 0}, // 3
		{0, 0, 1, 0}, // 4
		{1, 0, 1, 0}, // 5
		{0, 1, 1, 0}, // 6
		{1, 1, 1, 0}, // 7
		{0, 0, 0, 1}, // 8
	}

	for i, want := range expected {
		var outputs []int
		outputs, state = Counter(1, 0, state)
		for j := 0; j < 4; j++ {
			if outputs[j] != want[j] {
				t.Errorf("Counter step %d, bit %d: got %d, want %d", i+1, j, outputs[j], want[j])
			}
		}
	}
}

func TestCounter_Wrap(t *testing.T) {
	// A 4-bit counter at 1111 (15) should wrap to 0000 (0)
	state := &CounterState{Bits: []int{1, 1, 1, 1}, Width: 4}

	outputs, _ := Counter(1, 0, state)
	for i, v := range outputs {
		if v != 0 {
			t.Errorf("Counter wrap bit %d: got %d, want 0", i, v)
		}
	}
}

func TestCounter_Reset(t *testing.T) {
	// Reset should immediately clear the counter regardless of state
	state := &CounterState{Bits: []int{1, 0, 1, 0}, Width: 4}

	outputs, newState := Counter(1, 1, state)
	for i, v := range outputs {
		if v != 0 {
			t.Errorf("Counter reset bit %d: got %d, want 0", i, v)
		}
	}
	for i, v := range newState.Bits {
		if v != 0 {
			t.Errorf("Counter reset state bit %d: got %d, want 0", i, v)
		}
	}
}

func TestCounter_Hold(t *testing.T) {
	// When clock=0, counter should hold its value
	state := &CounterState{Bits: []int{1, 0, 1, 0}, Width: 4}

	outputs, _ := Counter(0, 0, state)
	want := []int{1, 0, 1, 0}
	for i := range outputs {
		if outputs[i] != want[i] {
			t.Errorf("Counter hold bit %d: got %d, want %d", i, outputs[i], want[i])
		}
	}
}

func TestCounter_InvalidInput(t *testing.T) {
	state := &CounterState{Bits: []int{0, 0}, Width: 2}
	assertPanics(t, "Counter invalid clock", func() { Counter(2, 0, state) })
	assertPanics(t, "Counter invalid reset", func() { Counter(0, 2, state) })
	assertPanics(t, "Counter nil state", func() { Counter(0, 0, nil) })
	assertPanics(t, "Counter zero width", func() { Counter(0, 0, &CounterState{Width: 0}) })
}

func TestCounter_SingleBit(t *testing.T) {
	// 1-bit counter toggles: 0 → 1 → 0 → 1 → ...
	state := &CounterState{Bits: []int{0}, Width: 1}

	outputs, state := Counter(1, 0, state)
	if outputs[0] != 1 {
		t.Errorf("1-bit counter step 1: got %d, want 1", outputs[0])
	}

	outputs, _ = Counter(1, 0, state)
	if outputs[0] != 0 {
		t.Errorf("1-bit counter step 2: got %d, want 0", outputs[0])
	}
}

func TestCounter_EmptyBitsInitialized(t *testing.T) {
	// Counter with Width=3 but empty Bits slice should initialize to zeros
	state := &CounterState{Width: 3}

	outputs, newState := Counter(1, 0, state)
	want := []int{1, 0, 0} // 0 + 1 = 1
	for i := range outputs {
		if outputs[i] != want[i] {
			t.Errorf("Counter init bit %d: got %d, want %d", i, outputs[i], want[i])
		}
	}
	if newState.Width != 3 {
		t.Errorf("Counter init width: got %d, want 3", newState.Width)
	}
}

func TestCounter_ResetWithClockLow(t *testing.T) {
	// Reset works regardless of clock value
	state := &CounterState{Bits: []int{1, 1, 0, 1}, Width: 4}

	outputs, _ := Counter(0, 1, state)
	for i, v := range outputs {
		if v != 0 {
			t.Errorf("Counter reset(clk=0) bit %d: got %d, want 0", i, v)
		}
	}
}
