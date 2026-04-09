// SilentWaiting.swift — A no-op Waiting implementation.
//
// SilentWaiting satisfies the Waiting protocol without doing anything visible.
// It is the "null object" pattern applied to the Waiting protocol:
// you can pass it anywhere a Waiting is needed without needing to guard for nil.
//
// When is this useful?
//
//   1. Testing — you don't want spinner output polluting captured output in tests.
//   2. Scripted use — when the REPL is driven programmatically, animation is
//      noise.
//   3. Piped output — terminal animation codes look wrong in log files.
//   4. Fast evaluators — if eval always returns in <50 ms, showing a spinner
//      would flash annoyingly.
//
// The State type is Int (a tick counter) purely to satisfy the associated-type
// requirement. SilentWaiting never reads the counter — it simply returns
// state + 1 on each tick so callers can observe that tick() was called.
//
// Tick interval:
//   tickMs() returns 100. Even though SilentWaiting does nothing, the runner
//   still polls at 100 ms intervals in async mode. This keeps CPU usage low
//   while not making tests noticeably slow.

/// A `Waiting` implementation that produces no visible output.
///
/// All callbacks are no-ops. The state is an `Int` tick counter that
/// increments by 1 on each `tick` call, which lets tests assert that ticks
/// are happening without needing to inspect visual output.
public struct SilentWaiting: Waiting {
    /// State type: a simple integer tick counter.
    public typealias State = Int

    public init() {}

    /// Return 0 — the initial tick count.
    public func start() -> Int { 0 }

    /// Increment the tick counter by 1 and return it.
    ///
    /// - Parameter state: The current tick count.
    /// - Returns: `state + 1`
    public func tick(_ state: Int) -> Int { state + 1 }

    /// Poll every 100 milliseconds.
    ///
    /// 100 ms is a reasonable default: fast enough to feel responsive but
    /// slow enough that polling overhead is negligible.
    public func tickMs() -> Int { 100 }

    /// Do nothing when eval finishes.
    ///
    /// - Parameter state: The final tick count (unused).
    public func stop(_ state: Int) {}
}
