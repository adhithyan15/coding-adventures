// Types.swift — Core value types for the CodingAdventuresRepl framework.
//
// These two enums form the vocabulary that every component in the framework
// speaks. By keeping them in one file, a new reader gets a complete picture
// of the data model before diving into any protocol or implementation file.

// ─────────────────────────────────────────────────────────────────────────────
// EvalResult
// ─────────────────────────────────────────────────────────────────────────────
//
// Every call to Language.eval returns one of three outcomes:
//
//   .ok(nil)      — evaluation succeeded, nothing to print
//   .ok("hello")  — evaluation succeeded, print "hello"
//   .error("oops") — evaluation failed, print "Error: oops"
//   .quit          — the user asked to end the session
//
// Truth table of the REPL's response to each result:
//
//   ┌────────────────────┬──────────────────────────────────────┐
//   │ EvalResult         │ REPL action                          │
//   ├────────────────────┼──────────────────────────────────────┤
//   │ .ok(nil)           │ Print nothing, read next line        │
//   │ .ok("text")        │ Print "text", read next line         │
//   │ .error("msg")      │ Print "Error: msg", read next line   │
//   │ .quit              │ Print "Goodbye!", return             │
//   └────────────────────┴──────────────────────────────────────┘

/// The three possible outcomes from evaluating a line of user input.
///
/// - `ok`: Evaluation succeeded. The associated `String?` is the output to
///   display; `nil` means the evaluation produced no output (e.g. an
///   assignment statement in most languages).
/// - `error`: Evaluation failed. The associated `String` is a human-readable
///   error message. The REPL prepends "Error: " when printing it.
/// - `quit`: The user (or the language plugin) asked to end the session.
///   The REPL prints "Goodbye!" and exits the loop.
public enum EvalResult: Equatable {
    case ok(String?)   // success, optional output
    case error(String) // failure with message
    case quit          // user asked to exit
}

// ─────────────────────────────────────────────────────────────────────────────
// Mode
// ─────────────────────────────────────────────────────────────────────────────
//
// The Mode enum controls HOW the runner dispatches the eval call.
//
//   .sync       — eval runs on the calling thread, blocking it until done.
//                 Simplest path; no concurrency overhead. Use when the caller
//                 already runs on a background thread or latency doesn't matter.
//
//   .async_mode — eval runs on a DispatchQueue.global() thread. The calling
//                 thread polls with DispatchGroup.wait(timeout:) so the Waiting
//                 plugin can tick (e.g. show a spinner) while eval runs.
//
// NOTE: The case is named `async_mode` — not `async` — because `async` is a
// reserved keyword in Swift 5.5+ (concurrency model). If we named it `async`
// we would need backtick escaping everywhere: `Mode.\`async\``.

/// Controls whether the runner dispatches eval synchronously or asynchronously.
///
/// The default is `.async_mode` so that the `Waiting` plugin can animate while
/// a slow evaluator runs. Use `.sync` in tests or contexts where you need
/// deterministic, single-threaded execution.
public enum Mode {
    /// Evaluate on the calling thread — simple, no concurrency.
    case sync

    /// Evaluate on a background thread via `DispatchGroup` + `DispatchQueue.global()`.
    /// The calling thread polls at `waiting.tickMs()` intervals so the Waiting
    /// plugin can animate.
    ///
    /// Named `async_mode` because `async` is a reserved Swift keyword.
    case async_mode

    /// The default mode. Currently `.async_mode` so the Waiting plugin is active.
    public static var `default`: Mode { .async_mode }
}
