// EchoLanguage.swift — The simplest possible Language implementation.
//
// An echo language is the "hello world" of REPL plugins. It does exactly one
// thing: repeat your input back at you. This has two valuable uses:
//
//   1. Testing — lets you verify the framework's read/print/loop machinery
//      without implementing a real evaluator.
//
//   2. Demo / onboarding — a working REPL with zero language logic, so new
//      developers can see the framework in action immediately.
//
// Behaviour:
//
//   Input         Output          Notes
//   ──────────    ──────────────  ─────────────────────────────
//   "hello"       .ok("hello")    Echo back unchanged
//   "  hi  "      .ok("  hi  ")   Spaces preserved (no trimming)
//   ""            .ok("")         Empty string is valid input
//   ":quit"       .quit           Magic exit command
//
// The ":quit" convention is borrowed from IRB (Ruby's REPL). Other common
// alternatives are "exit", "quit", "/exit". This framework uses ":quit"
// as the default because the colon prefix makes accidental quits less likely.

/// A `Language` that mirrors input back unchanged.
///
/// `:quit` ends the session. All other input is returned as `.ok(input)`.
///
/// This is the canonical test double for the REPL framework — it verifies
/// the loop mechanics without needing a real evaluator.
public struct EchoLanguage: Language {
    public init() {}

    /// Return `.quit` for `":quit"`, otherwise echo the input as `.ok`.
    ///
    /// - Parameter input: The line of input to evaluate.
    /// - Returns: `.quit` if input is exactly `":quit"`, otherwise `.ok(input)`.
    public func eval(_ input: String) -> EvalResult {
        // The single exit sentinel. Using a colon prefix keeps it from
        // conflicting with real data ("quit" by itself could be valid input in
        // some languages).
        if input == ":quit" { return .quit }

        // For all other input, echo it back. The associated value is Optional
        // because some real languages produce no output (e.g. assignments).
        // EchoLanguage always has output, so we always wrap in .some.
        return .ok(input)
    }
}
