//! The `Waiting` trait — animated feedback during long evaluations.
//!
//! # Why a waiting spinner?
//!
//! Some evaluations complete in microseconds; others (network calls, heavy
//! computation, infinite loops the user wants to interrupt) can take seconds
//! or forever. Without feedback, the terminal looks frozen. A spinner or
//! progress indicator reassures the user that *something* is happening.
//!
//! # The tick model
//!
//! Rather than running a separate long-lived animation thread, the runner
//! uses a *polling* design:
//!
//! ```text
//! state = waiting.start()
//! loop {
//!     wait up to tick_ms() for eval result
//!     if result arrived → waiting.stop(state); handle result; break
//!     if timeout        → state = waiting.tick(state)   // advance animation
//! }
//! ```
//!
//! This keeps the runner's threading model simple: one background eval thread
//! and one main-thread loop. The `Waiting` implementation decides what to
//! draw on each tick.
//!
//! # State threading with `Box<dyn Any>`
//!
//! The animation may need internal state (e.g., which spinner frame to show
//! next). Because the `Waiting` trait must be object-safe and generic over
//! state types, we use `Box<dyn std::any::Any + Send>` as the state carrier.
//!
//! A concrete implementation uses `Box::new(())` for stateless waiting, or
//! `Box::new(0usize)` for a frame counter, and downcasts inside `tick`.
//!
//! # Example — spinning braille dots
//!
//! ```rust
//! use std::any::Any;
//! use repl::waiting::Waiting;
//!
//! struct SpinnerWaiting;
//!
//! const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
//!
//! impl Waiting for SpinnerWaiting {
//!     fn start(&self) -> Box<dyn Any + Send> {
//!         print!("  ");
//!         Box::new(0usize)
//!     }
//!     fn tick(&self, state: Box<dyn Any + Send>) -> Box<dyn Any + Send> {
//!         let frame = *state.downcast::<usize>().unwrap();
//!         print!("\r{} ", FRAMES[frame % FRAMES.len()]);
//!         Box::new(frame + 1)
//!     }
//!     fn tick_ms(&self) -> u64 { 80 }
//!     fn stop(&self, _state: Box<dyn Any + Send>) {
//!         print!("\r  \r"); // erase spinner
//!     }
//! }
//! ```

/// Controls animated feedback shown while waiting for an evaluation result.
///
/// Implementations are called by the REPL runner in a tight polling loop:
/// [`start`](Waiting::start) once before the loop, [`tick`](Waiting::tick)
/// on every timeout, and [`stop`](Waiting::stop) once when the result arrives.
///
/// State is passed by value through the loop as `Box<dyn Any + Send>`, so
/// each tick receives the previous tick's state and returns a new one.
///
/// # Thread safety
///
/// Must be `Send + Sync` — the runner calls these methods from the main
/// thread while the evaluator runs on a worker thread.
pub trait Waiting: Send + Sync {
    /// Called once before the polling loop begins.
    ///
    /// Use this to print an initial indicator, record the start time, or
    /// initialise animation state. Return the initial state that will be
    /// passed to the first [`tick`](Waiting::tick) call.
    fn start(&self) -> Box<dyn std::any::Any + Send>;

    /// Called on every polling timeout — i.e., every [`tick_ms`](Waiting::tick_ms)
    /// milliseconds while waiting for the evaluation result.
    ///
    /// Receives ownership of the current state, may mutate it, and returns
    /// the next state. Typically used to advance animation frames.
    fn tick(&self, state: Box<dyn std::any::Any + Send>) -> Box<dyn std::any::Any + Send>;

    /// How many milliseconds to wait between ticks.
    ///
    /// The runner calls `recv_timeout(Duration::from_millis(tick_ms()))`.
    /// Smaller values → smoother animation, more CPU overhead.
    /// Larger values → coarser animation, less overhead.
    ///
    /// Typical values: 80–200 ms for spinners, 1000 ms for minimal overhead.
    fn tick_ms(&self) -> u64;

    /// Called once when the evaluation result arrives (or the channel disconnects).
    ///
    /// Use this to erase the spinner, print elapsed time, or clean up any
    /// terminal state set in [`start`](Waiting::start) or [`tick`](Waiting::tick).
    ///
    /// Receives ownership of the final state (e.g., total tick count).
    fn stop(&self, state: Box<dyn std::any::Any + Send>);
}
