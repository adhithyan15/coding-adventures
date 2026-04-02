// Package eventloop provides a pluggable, generic event loop — the heartbeat
// of any interactive application.
//
// # What is an event loop?
//
// An event loop is the outermost structure of any interactive program. It runs
// forever (until told to stop), repeatedly asking "did anything happen?" and
// dispatching whatever happened to registered handlers.
//
//	while running:
//	    collect events from all sources
//	    for each event:
//	        dispatch to handlers
//	        if any handler says "exit" → stop
//
// # Why make it generic?
//
// A naïve loop hardcodes what events look like (KeyPress, MouseMove…). That
// makes the loop untestable and inflexible. A generic loop works at any layer:
// a game loop, a GUI, a network server, and a test harness all share the same
// shape. The event type E is defined by the caller.
//
// # Usage
//
//	type AppEvent int
//	const (Tick AppEvent = iota; Quit)
//
//	loop := eventloop.New[AppEvent]()
//	loop.AddSource(myTimerSource)
//	loop.OnEvent(func(e AppEvent) eventloop.ControlFlow {
//	    if e == Quit {
//	        return eventloop.Exit
//	    }
//	    fmt.Println("tick!")
//	    return eventloop.Continue
//	})
//	loop.Run()
//
// # Operations
//
// Every public method is wrapped in an Operation, giving each call
// automatic timing, structured logging, and panic recovery. This
// package declares zero OS capabilities, so no op.File / op.Net
// namespace fields are available inside callbacks.
package eventloop

import "runtime"

// ═══════════════════════════════════════════════════════════════════════════
// ControlFlow
// ═══════════════════════════════════════════════════════════════════════════

// ControlFlow signals whether the event loop should keep running or stop.
//
// Using a named type instead of bool makes call sites self-documenting:
//
//	return eventloop.Exit      // clear intent
//	return true                // ambiguous — true means what, exactly?
type ControlFlow int

const (
	// Continue tells the loop to keep running after this event.
	Continue ControlFlow = iota
	// Exit tells the loop to stop immediately after this event is handled.
	Exit
)

// ═══════════════════════════════════════════════════════════════════════════
// EventSource
// ═══════════════════════════════════════════════════════════════════════════

// EventSource is anything that can produce events for the loop to dispatch.
//
// The critical contract: Poll MUST return immediately. Return an empty slice
// if nothing is ready. Never block inside Poll — blocking is the loop's job,
// not the source's.
//
// This pull-based design keeps the loop in control of scheduling. Sources
// that receive events from other threads (e.g., a network listener) should
// buffer into a channel and expose a Poll that drains it.
//
//	type TimerSource struct{ fired bool }
//	func (t *TimerSource) Poll() []AppEvent {
//	    if !t.fired && time.Now().After(deadline) {
//	        t.fired = true
//	        return []AppEvent{Tick}
//	    }
//	    return nil
//	}
type EventSource[E any] interface {
	Poll() []E
}

// ═══════════════════════════════════════════════════════════════════════════
// EventLoop
// ═══════════════════════════════════════════════════════════════════════════

// EventLoop drives an interactive application. It repeatedly polls all
// registered sources, dispatches each event to all registered handlers, and
// yields the CPU when the queue is empty.
//
// Single-threaded by design. All sources and handlers run on the calling
// goroutine. Multi-threaded event injection is handled by wrapping a channel
// in an EventSource whose Poll drains it.
type EventLoop[E any] struct {
	sources  []EventSource[E]
	handlers []func(E) ControlFlow
	stopped  bool
}

// New creates an empty EventLoop with no sources and no handlers.
func New[E any]() *EventLoop[E] {
	return &EventLoop[E]{}
}

// AddSource registers an event source. Sources are polled in registration order.
func (l *EventLoop[E]) AddSource(s EventSource[E]) {
	_, _ = StartNew[struct{}]("event-loop.AddSource", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			l.sources = append(l.sources, s)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// OnEvent registers a handler function. Handlers receive each event in
// registration order. If any handler returns Exit, the loop terminates
// immediately — subsequent handlers for that event are not called.
func (l *EventLoop[E]) OnEvent(h func(E) ControlFlow) {
	_, _ = StartNew[struct{}]("event-loop.OnEvent", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			l.handlers = append(l.handlers, h)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Stop signals the loop to exit on the next iteration. Safe to call from
// outside a handler (e.g., from a goroutine or a deferred function).
func (l *EventLoop[E]) Stop() {
	_, _ = StartNew[struct{}]("event-loop.Stop", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			l.stopped = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Run starts the event loop. It blocks on the calling goroutine until a
// handler returns Exit or Stop is called.
//
// Each iteration performs three phases:
//
//  1. Collect — poll every source and append results to a local queue.
//  2. Dispatch — deliver each queued event to every handler in order.
//     Stop immediately if any handler returns Exit.
//  3. Idle — if no events were collected, call runtime.Gosched() to yield
//     the goroutine scheduler. This prevents the loop from busy-spinning at
//     100% CPU when the application is idle between events.
func (l *EventLoop[E]) Run() {
	_, _ = StartNew[struct{}]("event-loop.Run", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			l.stopped = false
			for !l.stopped {
				// ── Phase 1: Collect ──────────────────────────────────────────────
				//
				// Poll every source. Append whatever each returns to the queue.
				// Sources return empty slices when nothing is ready — that is normal.
				var queue []E
				for _, src := range l.sources {
					queue = append(queue, src.Poll()...)
				}

				// ── Phase 2: Dispatch ─────────────────────────────────────────────
				//
				// Deliver each event to all handlers in registration order.
				// The moment any handler returns Exit we stop the entire loop —
				// not just that event.
				for _, event := range queue {
					for _, h := range l.handlers {
						if h(event) == Exit {
							return rf.Generate(true, false, struct{}{})
						}
					}
				}

				// ── Phase 3: Idle ─────────────────────────────────────────────────
				//
				// If nothing happened this iteration, yield the goroutine scheduler.
				// Without this, an idle loop would spin at 100% CPU waiting for the
				// next event. runtime.Gosched() says "I have nothing to do right now;
				// let other goroutines run."
				if len(queue) == 0 {
					runtime.Gosched()
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}
