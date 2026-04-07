package repl

// SilentWaiting is a no-op implementation of the Waiting interface.
//
// # Why have a no-op waiting implementation?
//
// The Waiting interface exists so REPLs can show a spinner or progress
// indicator while a long-running evaluation is in flight. But not every
// REPL needs (or can use) a spinner:
//
//   - Tests should never write to the terminal mid-evaluation.
//   - Batch/pipeline REPLs (reading from a file) have no terminal to animate.
//   - Simple interactive REPLs where eval is always fast.
//
// SilentWaiting satisfies the Waiting contract with the minimum possible
// code: all methods do nothing, and TickMs() returns a sensible default so
// the ticker does not spin the CPU at maximum frequency.
//
// # The 100 ms tick interval
//
// A 100 ms (10 Hz) interval is a good default:
//   - Fast enough that a spinner would look smooth to a human eye.
//   - Slow enough that the goroutine overhead is negligible (each select
//     iteration costs a few hundred nanoseconds; at 10 Hz, that is less
//     than 0.001 % of CPU time).
//
// Even though SilentWaiting's Tick does nothing, callers that subclass or
// replace it benefit from the reasonable default.
type SilentWaiting struct{}

// Start is called when evaluation begins. Returns nil (no state needed).
func (w SilentWaiting) Start() interface{} {
	// No animation to initialise. Return nil as the opaque state token.
	return nil
}

// Tick is called every TickMs milliseconds while evaluation is in flight.
// SilentWaiting does nothing and passes the state through unchanged.
func (w SilentWaiting) Tick(state interface{}) interface{} {
	// No animation to advance. Pass the state through unchanged.
	return state
}

// TickMs returns 100, giving a 10 Hz poll rate.
//
// This interval balances responsiveness (the REPL reacts within 100 ms of
// eval completing) against CPU overhead (the select wakes up only 10 times
// per second while waiting).
func (w SilentWaiting) TickMs() int {
	return 100
}

// Stop is called when evaluation completes. SilentWaiting does nothing.
func (w SilentWaiting) Stop(state interface{}) {
	// No animation to tear down.
}
