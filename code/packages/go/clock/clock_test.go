// Tests for the clock package.
//
// These tests verify the fundamental clock behavior: signal toggling,
// edge detection, cycle counting, listener notification, frequency
// division, and multi-phase generation.
package clock

import (
	"math"
	"testing"
)

// ---------------------------------------------------------------------------
// Basic clock behavior
// ---------------------------------------------------------------------------

func TestClockStartsAtZero(t *testing.T) {
	// The clock signal starts low (0), like a real oscillator
	// before it begins oscillating.
	clk := New(1_000_000)
	if clk.Value != 0 {
		t.Errorf("expected value 0, got %d", clk.Value)
	}
}

func TestClockStartsAtCycleZero(t *testing.T) {
	// No cycles have elapsed before the first tick.
	clk := New(1_000_000)
	if clk.Cycle != 0 {
		t.Errorf("expected cycle 0, got %d", clk.Cycle)
	}
}

func TestClockStartsWithZeroTicks(t *testing.T) {
	// No ticks have occurred yet.
	clk := New(1_000_000)
	if clk.TotalTicks() != 0 {
		t.Errorf("expected 0 ticks, got %d", clk.TotalTicks())
	}
}

func TestClockCustomFrequency(t *testing.T) {
	clk := New(3_000_000_000)
	if clk.FrequencyHz != 3_000_000_000 {
		t.Errorf("expected frequency 3000000000, got %d", clk.FrequencyHz)
	}
}

// ---------------------------------------------------------------------------
// Tick behavior
// ---------------------------------------------------------------------------

func TestFirstTickIsRising(t *testing.T) {
	// First tick goes from 0 to 1 -- a rising edge.
	clk := New(1_000_000)
	edge := clk.Tick()
	if !edge.IsRising {
		t.Error("first tick should be rising")
	}
	if edge.IsFalling {
		t.Error("first tick should not be falling")
	}
	if edge.Value != 1 {
		t.Errorf("expected value 1, got %d", edge.Value)
	}
	if clk.Value != 1 {
		t.Errorf("expected clock value 1, got %d", clk.Value)
	}
}

func TestSecondTickIsFalling(t *testing.T) {
	// Second tick goes from 1 to 0 -- a falling edge.
	clk := New(1_000_000)
	clk.Tick() // rising
	edge := clk.Tick()
	if edge.IsRising {
		t.Error("second tick should not be rising")
	}
	if !edge.IsFalling {
		t.Error("second tick should be falling")
	}
	if edge.Value != 0 {
		t.Errorf("expected value 0, got %d", edge.Value)
	}
}

func TestAlternatesCorrectly(t *testing.T) {
	// The clock should alternate: rise, fall, rise, fall, ...
	clk := New(1_000_000)
	for i := range 10 {
		edge := clk.Tick()
		if i%2 == 0 {
			if !edge.IsRising {
				t.Errorf("tick %d should be rising", i)
			}
		} else {
			if !edge.IsFalling {
				t.Errorf("tick %d should be falling", i)
			}
		}
	}
}

func TestCycleIncrementsOnRising(t *testing.T) {
	// Cycle count goes up by 1 on each rising edge.
	clk := New(1_000_000)

	edge1 := clk.Tick() // rising
	if edge1.Cycle != 1 || clk.Cycle != 1 {
		t.Errorf("expected cycle 1 after first rising, got edge=%d clock=%d", edge1.Cycle, clk.Cycle)
	}

	edge2 := clk.Tick() // falling
	if edge2.Cycle != 1 || clk.Cycle != 1 {
		t.Errorf("expected cycle 1 after falling, got edge=%d clock=%d", edge2.Cycle, clk.Cycle)
	}

	edge3 := clk.Tick() // rising
	if edge3.Cycle != 2 || clk.Cycle != 2 {
		t.Errorf("expected cycle 2 after second rising, got edge=%d clock=%d", edge3.Cycle, clk.Cycle)
	}
}

func TestTickCountIncrementsEveryTick(t *testing.T) {
	clk := New(1_000_000)
	clk.Tick()
	if clk.TotalTicks() != 1 {
		t.Errorf("expected 1, got %d", clk.TotalTicks())
	}
	clk.Tick()
	if clk.TotalTicks() != 2 {
		t.Errorf("expected 2, got %d", clk.TotalTicks())
	}
	clk.Tick()
	if clk.TotalTicks() != 3 {
		t.Errorf("expected 3, got %d", clk.TotalTicks())
	}
}

// ---------------------------------------------------------------------------
// FullCycle
// ---------------------------------------------------------------------------

func TestFullCycleReturnsRisingThenFalling(t *testing.T) {
	clk := New(1_000_000)
	rising, falling := clk.FullCycle()
	if !rising.IsRising {
		t.Error("first edge should be rising")
	}
	if !falling.IsFalling {
		t.Error("second edge should be falling")
	}
}

func TestFullCycleEndsAtZero(t *testing.T) {
	clk := New(1_000_000)
	clk.FullCycle()
	if clk.Value != 0 {
		t.Errorf("expected value 0 after full cycle, got %d", clk.Value)
	}
}

func TestFullCycleCycleCountIsOne(t *testing.T) {
	clk := New(1_000_000)
	clk.FullCycle()
	if clk.Cycle != 1 {
		t.Errorf("expected cycle 1, got %d", clk.Cycle)
	}
}

func TestFullCycleTwoTicksElapsed(t *testing.T) {
	clk := New(1_000_000)
	clk.FullCycle()
	if clk.TotalTicks() != 2 {
		t.Errorf("expected 2 ticks, got %d", clk.TotalTicks())
	}
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

func TestRunProducesCorrectEdgeCount(t *testing.T) {
	// N cycles = 2N edges.
	clk := New(1_000_000)
	edges := clk.Run(5)
	if len(edges) != 10 {
		t.Errorf("expected 10 edges, got %d", len(edges))
	}
}

func TestRunEdgesAlternate(t *testing.T) {
	clk := New(1_000_000)
	edges := clk.Run(3)
	for i, edge := range edges {
		if i%2 == 0 {
			if !edge.IsRising {
				t.Errorf("edge %d should be rising", i)
			}
		} else {
			if !edge.IsFalling {
				t.Errorf("edge %d should be falling", i)
			}
		}
	}
}

func TestRunFinalCycleCount(t *testing.T) {
	clk := New(1_000_000)
	clk.Run(7)
	if clk.Cycle != 7 {
		t.Errorf("expected cycle 7, got %d", clk.Cycle)
	}
}

func TestRunZeroCycles(t *testing.T) {
	clk := New(1_000_000)
	edges := clk.Run(0)
	if len(edges) != 0 {
		t.Errorf("expected 0 edges, got %d", len(edges))
	}
	if clk.Cycle != 0 {
		t.Errorf("expected cycle 0, got %d", clk.Cycle)
	}
}

// ---------------------------------------------------------------------------
// Listeners
// ---------------------------------------------------------------------------

func TestListenerCalledOnTick(t *testing.T) {
	clk := New(1_000_000)
	var received []ClockEdge
	clk.RegisterListener(func(edge ClockEdge) {
		received = append(received, edge)
	})
	clk.Tick()
	if len(received) != 1 {
		t.Errorf("expected 1 edge, got %d", len(received))
	}
	if !received[0].IsRising {
		t.Error("expected rising edge")
	}
}

func TestListenerSeesAllEdges(t *testing.T) {
	clk := New(1_000_000)
	var received []ClockEdge
	clk.RegisterListener(func(edge ClockEdge) {
		received = append(received, edge)
	})
	clk.Run(3)
	if len(received) != 6 {
		t.Errorf("expected 6 edges, got %d", len(received))
	}
}

func TestMultipleListeners(t *testing.T) {
	clk := New(1_000_000)
	var a, b []ClockEdge
	clk.RegisterListener(func(edge ClockEdge) { a = append(a, edge) })
	clk.RegisterListener(func(edge ClockEdge) { b = append(b, edge) })
	clk.Tick()
	if len(a) != 1 || len(b) != 1 {
		t.Errorf("expected 1 edge each, got a=%d b=%d", len(a), len(b))
	}
}

func TestUnregisterListener(t *testing.T) {
	clk := New(1_000_000)
	var received []ClockEdge
	clk.RegisterListener(func(edge ClockEdge) {
		received = append(received, edge)
	})
	clk.Tick() // 1 edge received
	err := clk.UnregisterListener(0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	clk.Tick() // should NOT be received
	if len(received) != 1 {
		t.Errorf("expected 1 edge, got %d", len(received))
	}
}

func TestUnregisterNonexistentReturnsError(t *testing.T) {
	clk := New(1_000_000)
	err := clk.UnregisterListener(0)
	if err == nil {
		t.Error("expected error for invalid index")
	}
}

func TestUnregisterNegativeIndexReturnsError(t *testing.T) {
	clk := New(1_000_000)
	err := clk.UnregisterListener(-1)
	if err == nil {
		t.Error("expected error for negative index")
	}
}

func TestListenerCount(t *testing.T) {
	clk := New(1_000_000)
	if clk.ListenerCount() != 0 {
		t.Errorf("expected 0 listeners, got %d", clk.ListenerCount())
	}
	clk.RegisterListener(func(_ ClockEdge) {})
	if clk.ListenerCount() != 1 {
		t.Errorf("expected 1 listener, got %d", clk.ListenerCount())
	}
	clk.RegisterListener(func(_ ClockEdge) {})
	if clk.ListenerCount() != 2 {
		t.Errorf("expected 2 listeners, got %d", clk.ListenerCount())
	}
}

// ---------------------------------------------------------------------------
// Reset
// ---------------------------------------------------------------------------

func TestResetValue(t *testing.T) {
	clk := New(1_000_000)
	clk.Tick()
	clk.Reset()
	if clk.Value != 0 {
		t.Errorf("expected value 0 after reset, got %d", clk.Value)
	}
}

func TestResetCycle(t *testing.T) {
	clk := New(1_000_000)
	clk.Run(5)
	clk.Reset()
	if clk.Cycle != 0 {
		t.Errorf("expected cycle 0 after reset, got %d", clk.Cycle)
	}
}

func TestResetTicks(t *testing.T) {
	clk := New(1_000_000)
	clk.Run(5)
	clk.Reset()
	if clk.TotalTicks() != 0 {
		t.Errorf("expected 0 ticks after reset, got %d", clk.TotalTicks())
	}
}

func TestResetPreservesListeners(t *testing.T) {
	clk := New(1_000_000)
	var received []ClockEdge
	clk.RegisterListener(func(edge ClockEdge) {
		received = append(received, edge)
	})
	clk.Run(3)  // 6 edges
	clk.Reset() // listeners preserved
	clk.Tick()  // 1 more edge
	if len(received) != 7 {
		t.Errorf("expected 7 edges total, got %d", len(received))
	}
}

func TestResetPreservesFrequency(t *testing.T) {
	clk := New(5_000_000)
	clk.Run(10)
	clk.Reset()
	if clk.FrequencyHz != 5_000_000 {
		t.Errorf("expected frequency 5000000, got %d", clk.FrequencyHz)
	}
}

// ---------------------------------------------------------------------------
// Period calculation
// ---------------------------------------------------------------------------

func TestPeriodNs1MHz(t *testing.T) {
	// 1 MHz = 1000 ns period.
	clk := New(1_000_000)
	if clk.PeriodNs() != 1000.0 {
		t.Errorf("expected 1000.0 ns, got %f", clk.PeriodNs())
	}
}

func TestPeriodNs1GHz(t *testing.T) {
	// 1 GHz = 1 ns period.
	clk := New(1_000_000_000)
	if clk.PeriodNs() != 1.0 {
		t.Errorf("expected 1.0 ns, got %f", clk.PeriodNs())
	}
}

func TestPeriodNs3GHz(t *testing.T) {
	// 3 GHz ~ 0.333 ns period.
	clk := New(3_000_000_000)
	expected := 1e9 / 3_000_000_000.0
	if math.Abs(clk.PeriodNs()-expected) > 1e-10 {
		t.Errorf("expected %f ns, got %f", expected, clk.PeriodNs())
	}
}

// ---------------------------------------------------------------------------
// ClockDivider
// ---------------------------------------------------------------------------

func TestDivideBy2(t *testing.T) {
	// Every 2 source cycles = 1 output cycle.
	master := New(1_000_000)
	divider, err := NewClockDivider(master, 2)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	master.Run(4)
	if divider.Output.Cycle != 2 {
		t.Errorf("expected 2 output cycles, got %d", divider.Output.Cycle)
	}
}

func TestDivideBy4(t *testing.T) {
	master := New(1_000_000_000)
	divider, err := NewClockDivider(master, 4)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	master.Run(8)
	if divider.Output.Cycle != 2 {
		t.Errorf("expected 2 output cycles, got %d", divider.Output.Cycle)
	}
}

func TestDividerOutputFrequency(t *testing.T) {
	master := New(1_000_000_000)
	divider, err := NewClockDivider(master, 4)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if divider.Output.FrequencyHz != 250_000_000 {
		t.Errorf("expected 250000000 Hz, got %d", divider.Output.FrequencyHz)
	}
}

func TestDividerDivisorTooSmall(t *testing.T) {
	master := New(1_000_000)
	_, err := NewClockDivider(master, 1)
	if err == nil {
		t.Error("expected error for divisor < 2")
	}
}

func TestDividerDivisorZero(t *testing.T) {
	master := New(1_000_000)
	_, err := NewClockDivider(master, 0)
	if err == nil {
		t.Error("expected error for divisor 0")
	}
}

func TestDividerDivisorNegative(t *testing.T) {
	master := New(1_000_000)
	_, err := NewClockDivider(master, -1)
	if err == nil {
		t.Error("expected error for negative divisor")
	}
}

func TestDividerOutputValueReturnsToZero(t *testing.T) {
	master := New(1_000_000)
	divider, err := NewClockDivider(master, 2)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	master.Run(2)
	if divider.Output.Value != 0 {
		t.Errorf("expected output value 0, got %d", divider.Output.Value)
	}
}

// ---------------------------------------------------------------------------
// MultiPhaseClock
// ---------------------------------------------------------------------------

func TestMultiPhaseInitialStateAllZero(t *testing.T) {
	master := New(1_000_000)
	mpc, err := NewMultiPhaseClock(master, 4)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	for i := range 4 {
		if mpc.GetPhase(i) != 0 {
			t.Errorf("phase %d should be 0 initially", i)
		}
	}
}

func TestMultiPhaseFirstRisingActivatesPhase0(t *testing.T) {
	master := New(1_000_000)
	mpc, err := NewMultiPhaseClock(master, 4)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	master.Tick() // rising edge
	if mpc.GetPhase(0) != 1 {
		t.Error("phase 0 should be active")
	}
	for i := 1; i < 4; i++ {
		if mpc.GetPhase(i) != 0 {
			t.Errorf("phase %d should be inactive", i)
		}
	}
}

func TestMultiPhasePhasesRotate(t *testing.T) {
	master := New(1_000_000)
	mpc, err := NewMultiPhaseClock(master, 4)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	for expectedPhase := range 4 {
		master.Tick() // rising
		for p := range 4 {
			expected := 0
			if p == expectedPhase {
				expected = 1
			}
			if mpc.GetPhase(p) != expected {
				t.Errorf("phase %d: expected %d, got %d (active phase should be %d)",
					p, expected, mpc.GetPhase(p), expectedPhase)
			}
		}
		master.Tick() // falling (no change)
	}
}

func TestMultiPhasePhasesWrapAround(t *testing.T) {
	master := New(1_000_000)
	mpc, err := NewMultiPhaseClock(master, 3)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// 3 rising edges cycle through phases 0, 1, 2
	for range 3 {
		master.FullCycle()
	}

	// 4th rising edge should activate phase 0 again
	master.Tick()
	if mpc.GetPhase(0) != 1 {
		t.Error("phase 0 should be active after wrap")
	}
	if mpc.GetPhase(1) != 0 {
		t.Error("phase 1 should be inactive after wrap")
	}
	if mpc.GetPhase(2) != 0 {
		t.Error("phase 2 should be inactive after wrap")
	}
}

func TestMultiPhaseOnlyOnePhaseActive(t *testing.T) {
	master := New(1_000_000)
	mpc, err := NewMultiPhaseClock(master, 4)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	for range 20 {
		master.Tick()
		activeCount := 0
		for i := range 4 {
			if mpc.GetPhase(i) == 1 {
				activeCount++
			}
		}
		if activeCount > 1 {
			t.Error("more than one phase active")
		}
	}
}

func TestMultiPhasePhasesTooSmall(t *testing.T) {
	master := New(1_000_000)
	_, err := NewMultiPhaseClock(master, 1)
	if err == nil {
		t.Error("expected error for phases < 2")
	}
}

func TestMultiPhasePhasesZero(t *testing.T) {
	master := New(1_000_000)
	_, err := NewMultiPhaseClock(master, 0)
	if err == nil {
		t.Error("expected error for phases 0")
	}
}

func TestMultiPhaseTwoPhase(t *testing.T) {
	master := New(1_000_000)
	mpc, err := NewMultiPhaseClock(master, 2)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	master.Tick() // rising -> phase 0 active
	if mpc.GetPhase(0) != 1 || mpc.GetPhase(1) != 0 {
		t.Error("phase 0 should be active, phase 1 inactive")
	}

	master.Tick() // falling -> no change
	master.Tick() // rising -> phase 1 active
	if mpc.GetPhase(0) != 0 || mpc.GetPhase(1) != 1 {
		t.Error("phase 0 should be inactive, phase 1 active")
	}
}

// ---------------------------------------------------------------------------
// ClockEdge struct
// ---------------------------------------------------------------------------

func TestClockEdgeFields(t *testing.T) {
	edge := ClockEdge{Cycle: 3, Value: 1, IsRising: true, IsFalling: false}
	if edge.Cycle != 3 || edge.Value != 1 || !edge.IsRising || edge.IsFalling {
		t.Error("edge fields not set correctly")
	}
}

func TestClockEdgeEquality(t *testing.T) {
	a := ClockEdge{Cycle: 1, Value: 1, IsRising: true, IsFalling: false}
	b := ClockEdge{Cycle: 1, Value: 1, IsRising: true, IsFalling: false}
	if a != b {
		t.Error("identical edges should be equal")
	}
}
