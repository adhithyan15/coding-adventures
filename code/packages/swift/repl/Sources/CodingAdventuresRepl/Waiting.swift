// Waiting.swift — The "busy indicator" plugin protocol.
//
// When a REPL evaluation takes more than a fraction of a second, users need
// feedback that the system is still working — not frozen. This protocol
// models that feedback as a tick-driven state machine:
//
//   State machine:
//
//     ┌──────┐  start()  ┌──────────┐  tick()  ┌──────────┐
//     │ idle │ ─────────► │ running  │ ─────────► │ running  │ ─ …
//     └──────┘           └──────────┘           └──────────┘
//                                                     │
//                                               stop(state)
//                                                     │
//                                                     ▼
//                                               ┌──────────┐
//                                               │   idle   │
//                                               └──────────┘
//
// The associated type `State` lets each implementation carry whatever data it
// needs between ticks without allocating a class. A spinner needs a frame
// index; a progress bar needs elapsed time; `SilentWaiting` uses `Int` but
// never reads it.
//
// tick() timing:
//   The runner calls tick() every tickMs() milliseconds (approximately).
//   Exact timing depends on DispatchGroup.wait(timeout:) precision. Don't
//   rely on sub-millisecond accuracy; this is for human-visible animation.
//
// Example spinner (hypothetical):
//
//   struct SpinnerWaiting: Waiting {
//       typealias State = Int
//       let frames = ["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"]
//       func start() -> Int { print("", terminator: ""); return 0 }
//       func tick(_ s: Int) -> Int {
//           print("\r\(frames[s % frames.count]) ", terminator: "")
//           return s + 1
//       }
//       func tickMs() -> Int { 80 }
//       func stop(_ s: Int) { print("\r  \r", terminator: "") }
//   }

/// A tick-driven animation plugin shown while the evaluator is running.
///
/// Implement this protocol to provide visual feedback (spinner, dots, elapsed
/// timer, etc.) during slow evaluations.
///
/// - `State`: The type that carries animation state between ticks. Use `Int`
///   for a simple frame counter; use a more complex type if you need elapsed
///   time, a mutable buffer, etc.
public protocol Waiting {
    /// The type of state passed between `start`, `tick`, and `stop`.
    associatedtype State

    /// Called once immediately before eval starts.
    ///
    /// - Returns: The initial animation state.
    func start() -> State

    /// Called every `tickMs()` milliseconds while eval is running.
    ///
    /// - Parameter state: The current animation state.
    /// - Returns: The next animation state (passed back on the next tick).
    func tick(_ state: State) -> State

    /// How many milliseconds to wait between `tick` calls.
    ///
    /// Smaller values = smoother animation but more CPU polling.
    /// 100 ms is a good default for simple indicators.
    func tickMs() -> Int

    /// Called once after eval finishes.
    ///
    /// Use this to clear the animation and restore the terminal to a clean
    /// state before the result is printed.
    ///
    /// - Parameter state: The final animation state.
    func stop(_ state: State)
}
