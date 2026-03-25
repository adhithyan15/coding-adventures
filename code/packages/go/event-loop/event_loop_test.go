package eventloop

import (
	"testing"
)

// ═══════════════════════════════════════════════════════════════════════════
// Test helpers — mock sources
// ═══════════════════════════════════════════════════════════════════════════

// fixedSource emits a predefined list of events, one batch per Poll call.
// After all batches are exhausted, subsequent Poll calls return nil.
type fixedSource[E any] struct {
	batches [][]E
	idx     int
}

func newFixedSource[E any](batches ...[]E) *fixedSource[E] {
	return &fixedSource[E]{batches: batches}
}

func (s *fixedSource[E]) Poll() []E {
	if s.idx >= len(s.batches) {
		return nil
	}
	batch := s.batches[s.idx]
	s.idx++
	return batch
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

// TestRunDeliversAllEvents verifies that every event emitted by a source
// reaches every registered handler.
func TestRunDeliversAllEvents(t *testing.T) {
	loop := New[int]()

	// Source emits [1, 2, 3] on the first poll, then nothing.
	loop.AddSource(newFixedSource([]int{1, 2, 3}))

	var received []int
	loop.OnEvent(func(e int) ControlFlow {
		received = append(received, e)
		return Continue
	})

	// The loop needs a way to stop. Add a second source that fires a quit
	// signal after the first source is exhausted.
	type signal int
	// We'll piggyback the quit on the same event type by using a sentinel value.
	loop.AddSource(newFixedSource[int](nil, []int{-1})) // second poll returns sentinel
	loop.OnEvent(func(e int) ControlFlow {
		if e == -1 {
			return Exit
		}
		return Continue
	})

	loop.Run()

	// -1 is the sentinel — check only the real events.
	want := []int{1, 2, 3}
	got := received[:3]
	for i, v := range want {
		if got[i] != v {
			t.Errorf("received[%d] = %d, want %d", i, got[i], v)
		}
	}
}

// TestExitStopsLoopImmediately verifies that when a handler returns Exit,
// the loop stops and no further events are dispatched.
func TestExitStopsLoopImmediately(t *testing.T) {
	loop := New[string]()
	loop.AddSource(newFixedSource([]string{"a", "b", "stop", "c", "d"}))

	var seen []string
	loop.OnEvent(func(e string) ControlFlow {
		seen = append(seen, e)
		if e == "stop" {
			return Exit
		}
		return Continue
	})

	loop.Run()

	// "c" and "d" must NOT have been processed.
	for _, v := range seen {
		if v == "c" || v == "d" {
			t.Errorf("event %q was delivered after Exit was returned", v)
		}
	}
	if len(seen) != 3 {
		t.Errorf("expected 3 events (a, b, stop), got %d: %v", len(seen), seen)
	}
}

// TestStopFromOutsideHandler verifies that calling Stop() terminates the loop.
func TestStopFromOutsideHandler(t *testing.T) {
	loop := New[int]()
	// A source that always returns an event — the loop would never stop without Stop().
	infiniteSource := &infiniteIntSource{}
	loop.AddSource(infiniteSource)

	count := 0
	loop.OnEvent(func(e int) ControlFlow {
		count++
		if count >= 5 {
			loop.Stop()
		}
		return Continue
	})

	loop.Run()

	if count < 5 {
		t.Errorf("expected at least 5 events before stop, got %d", count)
	}
}

// infiniteIntSource is a source that always returns one event per poll.
type infiniteIntSource struct{ n int }

func (s *infiniteIntSource) Poll() []int {
	s.n++
	return []int{s.n}
}

// TestMultipleHandlersAllReceiveEvent verifies that all registered handlers
// receive the same event.
func TestMultipleHandlersAllReceiveEvent(t *testing.T) {
	loop := New[int]()
	loop.AddSource(newFixedSource([]int{42}, []int{-1}))

	h1Saw, h2Saw := 0, 0

	loop.OnEvent(func(e int) ControlFlow {
		if e == 42 {
			h1Saw = e
		}
		if e == -1 {
			return Exit
		}
		return Continue
	})
	loop.OnEvent(func(e int) ControlFlow {
		if e == 42 {
			h2Saw = e
		}
		return Continue
	})

	loop.Run()

	if h1Saw != 42 {
		t.Errorf("handler 1 did not see event 42")
	}
	if h2Saw != 42 {
		t.Errorf("handler 2 did not see event 42")
	}
}

// TestMultipleSourcesMerged verifies that events from all sources appear
// in the dispatch queue within a single iteration.
func TestMultipleSourcesMerged(t *testing.T) {
	loop := New[string]()
	loop.AddSource(newFixedSource([]string{"from-a"}))
	loop.AddSource(newFixedSource([]string{"from-b"}))
	loop.AddSource(newFixedSource[string](nil, []string{"stop"}))

	var seen []string
	loop.OnEvent(func(e string) ControlFlow {
		if e == "stop" {
			return Exit
		}
		seen = append(seen, e)
		return Continue
	})

	loop.Run()

	if len(seen) != 2 {
		t.Errorf("expected 2 events, got %d: %v", len(seen), seen)
	}
}

// TestEmptyLoopStopsOnStop verifies that a loop with no sources stops when
// Stop() is called.
func TestEmptyLoopStopsOnStop(t *testing.T) {
	loop := New[int]()
	called := false
	loop.OnEvent(func(e int) ControlFlow {
		called = true
		return Continue
	})

	// Stop immediately from a goroutine to avoid infinite idle spin in test.
	go func() {
		loop.Stop()
	}()
	loop.Run()

	if called {
		t.Error("handler should not have been called with no sources")
	}
}

// TestControlFlowValues confirms the ControlFlow constants have distinct values.
func TestControlFlowValues(t *testing.T) {
	if Continue == Exit {
		t.Error("Continue and Exit must be distinct ControlFlow values")
	}
}
