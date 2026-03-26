//! # event-loop
//!
//! A pluggable, generic event loop — the heartbeat of any interactive application.
//!
//! ## What is an event loop?
//!
//! An event loop is the outermost structure of any interactive program. It runs
//! forever (until told to stop), repeatedly asking "did anything happen?" and
//! dispatching whatever happened to registered handlers.
//!
//! ```text
//! while running:
//!     collect events from all sources
//!     for each event:
//!         dispatch to handlers
//!         if any handler says "exit" → stop
//! ```
//!
//! ## Why make it generic?
//!
//! A naïve loop hardcodes what events look like (`KeyPress`, `MouseMove`…).
//! That makes the loop untestable and inflexible. A generic loop is testable
//! in isolation: inject a mock source that emits exactly the events you want.
//! The event type `E` is defined by the caller.
//!
//! ## Quick start
//!
//! ```rust
//! use event_loop::{EventLoop, EventSource, ControlFlow};
//!
//! #[derive(Debug, PartialEq)]
//! enum AppEvent { Tick, Quit }
//!
//! struct TickSource { count: usize }
//! impl EventSource<AppEvent> for TickSource {
//!     fn poll(&mut self) -> Vec<AppEvent> {
//!         if self.count < 3 {
//!             self.count += 1;
//!             vec![AppEvent::Tick]
//!         } else {
//!             vec![AppEvent::Quit]
//!         }
//!     }
//! }
//!
//! let mut loop_ = EventLoop::new();
//! loop_.add_source(TickSource { count: 0 });
//! loop_.on_event(|e: &AppEvent| {
//!     match e {
//!         AppEvent::Quit => ControlFlow::Exit,
//!         AppEvent::Tick => ControlFlow::Continue,
//!     }
//! });
//! loop_.run();
//! ```

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

// ════════════════════════════════════════════════════════════════════════════
// ControlFlow
// ════════════════════════════════════════════════════════════════════════════

/// Signals whether the event loop should continue running or stop.
///
/// Using an enum instead of `bool` makes call sites self-documenting:
///
/// ```rust
/// # use event_loop::ControlFlow;
/// let cf = ControlFlow::Exit;   // intent is clear
/// // vs.
/// // return true;               // true means… stop? continue?
/// ```
///
/// The enum also leaves room for future variants (`Pause`, `ScheduleNext`,
/// etc.) without breaking existing handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    /// Keep looping — there is more work to do.
    Continue,
    /// Stop the loop immediately after this event.
    /// Subsequent handlers for the same event are not called.
    Exit,
}

// ════════════════════════════════════════════════════════════════════════════
// EventSource
// ════════════════════════════════════════════════════════════════════════════

/// Anything that can produce events for the loop to dispatch.
///
/// The critical contract: **`poll` must return immediately**. Return an empty
/// `Vec` if nothing is ready. Never block — blocking is the loop's job.
///
/// This pull-based design keeps the loop in control of scheduling. Sources
/// that receive events from other threads should buffer into a `Mutex<VecDeque>`
/// and expose a `poll` that drains it.
///
/// # Example
///
/// ```rust
/// use event_loop::EventSource;
///
/// struct CountdownSource { remaining: usize }
///
/// impl EventSource<u32> for CountdownSource {
///     fn poll(&mut self) -> Vec<u32> {
///         if self.remaining > 0 {
///             self.remaining -= 1;
///             vec![self.remaining as u32]
///         } else {
///             vec![]
///         }
///     }
/// }
/// ```
pub trait EventSource<E> {
    /// Return all events currently available. Must not block.
    fn poll(&mut self) -> Vec<E>;
}

// ════════════════════════════════════════════════════════════════════════════
// StopHandle
// ════════════════════════════════════════════════════════════════════════════

/// A handle for stopping the event loop from outside a handler.
///
/// Obtain via [`EventLoop::stop_handle`]. Clone freely — all clones share the
/// same underlying flag.
///
/// This is useful when you need to stop the loop from another thread or from
/// a timer callback that does not have access to the loop directly.
#[derive(Clone)]
pub struct StopHandle(Arc<AtomicBool>);

impl StopHandle {
    /// Signal the associated event loop to stop on its next iteration.
    pub fn stop(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

// ════════════════════════════════════════════════════════════════════════════
// EventLoop
// ════════════════════════════════════════════════════════════════════════════

/// A pluggable, generic event loop.
///
/// `EventLoop<E>` is generic over the event type `E`. You define what events
/// look like; the loop handles collection and dispatch.
///
/// # Single-threaded design
///
/// All sources and handlers run on the thread that calls `run()`. Multi-thread
/// event injection is handled by wrapping a `Mutex<VecDeque<E>>` in a source
/// whose `poll` drains it — the loop never needs to know.
pub struct EventLoop<E> {
    sources: Vec<Box<dyn EventSource<E>>>,
    handlers: Vec<Box<dyn FnMut(&E) -> ControlFlow>>,
    stopped: Arc<AtomicBool>,
}

impl<E> EventLoop<E> {
    /// Create a new, empty event loop.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            handlers: Vec::new(),
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Register an event source. Sources are polled in registration order.
    pub fn add_source<S: EventSource<E> + 'static>(&mut self, source: S) {
        self.sources.push(Box::new(source));
    }

    /// Register an event handler.
    ///
    /// Handlers receive each event in registration order. If any handler
    /// returns [`ControlFlow::Exit`], the loop stops immediately — subsequent
    /// handlers for the same event are not called.
    pub fn on_event<F: FnMut(&E) -> ControlFlow + 'static>(&mut self, handler: F) {
        self.handlers.push(Box::new(handler));
    }

    /// Return a [`StopHandle`] that can stop this loop from outside a handler.
    pub fn stop_handle(&self) -> StopHandle {
        StopHandle(Arc::clone(&self.stopped))
    }

    /// Signal the loop to exit on the next iteration.
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
    }

    /// Start the event loop. Blocks until a handler returns `Exit` or `stop()` is called.
    ///
    /// Each iteration performs three phases:
    ///
    /// 1. **Collect** — poll every source; append results to a local queue.
    /// 2. **Dispatch** — deliver each queued event to every handler in order.
    ///    Stop immediately if any handler returns `Exit`.
    /// 3. **Idle** — if the queue was empty, call [`std::thread::yield_now()`]
    ///    to give the OS a chance to schedule other threads. Without this, an
    ///    idle loop would spin at 100 % CPU waiting for the next event.
    pub fn run(&mut self) {
        self.stopped.store(false, Ordering::Relaxed);

        loop {
            // Check the stop flag at the top of each iteration so that a call
            // to stop() takes effect even when no events are arriving.
            if self.stopped.load(Ordering::Relaxed) {
                return;
            }

            // ── Phase 1: Collect ─────────────────────────────────────────
            let mut queue: Vec<E> = Vec::new();
            for source in &mut self.sources {
                queue.extend(source.poll());
            }

            // ── Phase 2: Dispatch ────────────────────────────────────────
            let mut should_exit = false;
            'dispatch: for event in &queue {
                for handler in &mut self.handlers {
                    if handler(event) == ControlFlow::Exit {
                        should_exit = true;
                        break 'dispatch;
                    }
                }
            }
            if should_exit {
                return;
            }

            // ── Phase 3: Idle ────────────────────────────────────────────
            if queue.is_empty() {
                std::thread::yield_now();
            }
        }
    }
}

impl<E> Default for EventLoop<E> {
    fn default() -> Self {
        Self::new()
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Tests
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test helpers ────────────────────────────────────────────────────────

    /// A source that emits a fixed sequence of event batches, then nothing.
    struct FixedSource<E: Clone> {
        batches: Vec<Vec<E>>,
        index: usize,
    }

    impl<E: Clone> FixedSource<E> {
        fn new(batches: Vec<Vec<E>>) -> Self {
            Self { batches, index: 0 }
        }
    }

    impl<E: Clone> EventSource<E> for FixedSource<E> {
        fn poll(&mut self) -> Vec<E> {
            if self.index >= self.batches.len() {
                return vec![];
            }
            let batch = self.batches[self.index].clone();
            self.index += 1;
            batch
        }
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    /// All events emitted by a source must reach registered handlers.
    #[test]
    fn delivers_all_events() {
        let mut loop_ = EventLoop::new();
        // First batch: real events. Second batch: sentinel to stop the loop.
        loop_.add_source(FixedSource::new(vec![
            vec![1i32, 2, 3],
            vec![-1], // sentinel: Exit
        ]));

        let mut received: Vec<i32> = Vec::new();
        loop_.on_event(move |&e| {
            if e == -1 {
                return ControlFlow::Exit;
            }
            received.push(e);
            ControlFlow::Continue
        });

        loop_.run();

        // Can't inspect `received` after the closure captures it, so we test
        // via a shared vec instead.
    }

    /// Variant of delivers_all_events using shared state so we can assert.
    #[test]
    fn delivers_all_events_shared() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let mut loop_ = EventLoop::<i32>::new();
        loop_.add_source(FixedSource::new(vec![vec![1, 2, 3], vec![-1]]));

        let received = Rc::new(RefCell::new(Vec::new()));
        let received_clone = Rc::clone(&received);

        loop_.on_event(move |&e| {
            if e == -1 {
                return ControlFlow::Exit;
            }
            received_clone.borrow_mut().push(e);
            ControlFlow::Continue
        });

        loop_.run();

        assert_eq!(*received.borrow(), vec![1, 2, 3]);
    }

    /// When a handler returns Exit, subsequent events must not be dispatched.
    #[test]
    fn exit_stops_dispatch_immediately() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let mut loop_ = EventLoop::new();
        loop_.add_source(FixedSource::new(vec![vec!["a", "stop", "b", "c"]]));

        let seen = Rc::new(RefCell::new(Vec::new()));
        let seen_clone = Rc::clone(&seen);

        loop_.on_event(move |&e| {
            seen_clone.borrow_mut().push(e);
            if e == "stop" {
                ControlFlow::Exit
            } else {
                ControlFlow::Continue
            }
        });

        loop_.run();

        let s = seen.borrow();
        assert_eq!(*s, vec!["a", "stop"], "events after 'stop' should not be seen");
    }

    /// stop() called from outside a handler terminates the loop.
    #[test]
    fn stop_handle_terminates_loop() {
        use std::cell::Cell;
        use std::rc::Rc;

        let mut loop_ = EventLoop::<i32>::new();

        // Infinite source.
        struct Counter(i32);
        impl EventSource<i32> for Counter {
            fn poll(&mut self) -> Vec<i32> {
                self.0 += 1;
                vec![self.0]
            }
        }
        loop_.add_source(Counter(0));

        let count = Rc::new(Cell::new(0u32));
        let count_clone = Rc::clone(&count);
        let handle = loop_.stop_handle();

        loop_.on_event(move |_| {
            let n = count_clone.get() + 1;
            count_clone.set(n);
            if n >= 5 {
                handle.stop();
            }
            ControlFlow::Continue
        });

        loop_.run();

        assert!(count.get() >= 5);
    }

    /// Multiple handlers all receive the same event.
    #[test]
    fn multiple_handlers_all_see_event() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let mut loop_ = EventLoop::<i32>::new();
        loop_.add_source(FixedSource::new(vec![vec![99], vec![-1]]));

        let h1 = Rc::new(RefCell::new(0i32));
        let h2 = Rc::new(RefCell::new(0i32));
        let h1c = Rc::clone(&h1);
        let h2c = Rc::clone(&h2);

        loop_.on_event(move |&e| {
            if e == 99 {
                *h1c.borrow_mut() = e;
            }
            if e == -1 {
                return ControlFlow::Exit;
            }
            ControlFlow::Continue
        });
        loop_.on_event(move |&e| {
            if e == 99 {
                *h2c.borrow_mut() = e;
            }
            ControlFlow::Continue
        });

        loop_.run();

        assert_eq!(*h1.borrow(), 99);
        assert_eq!(*h2.borrow(), 99);
    }

    /// Events from multiple sources are all collected and dispatched.
    #[test]
    fn multiple_sources_merged() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let mut loop_ = EventLoop::new();
        loop_.add_source(FixedSource::new(vec![vec!["alpha"]]));
        loop_.add_source(FixedSource::new(vec![vec!["beta"]]));
        loop_.add_source(FixedSource::new(vec![vec![], vec!["stop"]]));

        let seen = Rc::new(RefCell::new(Vec::new()));
        let seen_clone = Rc::clone(&seen);

        loop_.on_event(move |&e| {
            if e == "stop" {
                return ControlFlow::Exit;
            }
            seen_clone.borrow_mut().push(e);
            ControlFlow::Continue
        });

        loop_.run();

        let s = seen.borrow();
        assert_eq!(s.len(), 2);
        assert!(s.contains(&"alpha"));
        assert!(s.contains(&"beta"));
    }

    /// Default impl is the same as new().
    #[test]
    fn default_creates_empty_loop() {
        let _loop: EventLoop<i32> = EventLoop::default();
    }

    /// ControlFlow variants are distinct.
    #[test]
    fn control_flow_variants_distinct() {
        assert_ne!(ControlFlow::Continue, ControlFlow::Exit);
    }
}
