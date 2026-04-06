// Language.swift — The evaluator plugin protocol.
//
// In a classic REPL the "eval" step takes a string of user input and returns
// some result. This protocol captures that contract without prescribing what
// any specific language does.
//
// Examples of Language implementations:
//   EchoLanguage     — repeats input back (this package, for testing/demos)
//   ParrotLanguage   — same as echo, with a parrot theme
//   LispLanguage     — interprets a Lisp-like expression language
//   PythonBridge     — forwards to a Python subprocess
//   BytecodeVM       — executes bytecode produced by a compiler
//
// ┌──────────────────────────────────────────────────────────┐
// │  Caller thread                                           │
// │                                                          │
// │  input ──► language.eval(input) ──► EvalResult          │
// │                  ↑                                        │
// │             (may block)                                   │
// └──────────────────────────────────────────────────────────┘
//
// eval is a SYNCHRONOUS call. The runner is responsible for dispatching it
// to a background thread if async mode is chosen. This keeps the Language
// protocol simple: implementors do not need to know about DispatchGroup or
// any other concurrency primitive.

/// A pluggable evaluator for the REPL loop.
///
/// Implementors receive a line of user input and return an `EvalResult`
/// describing what happened. The call is synchronous and may block
/// arbitrarily long (e.g. waiting for a subprocess or compiling code).
///
/// Thread safety: `eval` will be called from a background `DispatchQueue`
/// thread in `.async_mode`. Implementations must be safe to call from any
/// thread, but do not need to be re-entrant (the runner never calls `eval`
/// concurrently).
public protocol Language {
    /// Evaluate one line of user input.
    ///
    /// - Parameter input: The raw line entered by the user, with trailing
    ///   newlines stripped.
    /// - Returns: An `EvalResult` describing the outcome.
    func eval(_ input: String) -> EvalResult
}
