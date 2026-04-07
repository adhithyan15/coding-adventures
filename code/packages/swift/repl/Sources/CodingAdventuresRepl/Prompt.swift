// Prompt.swift — The prompt-string provider protocol.
//
// A REPL needs two kinds of prompt text:
//
//   1. Global (banner) — printed once at startup. Tells the user what they
//      are talking to and how to exit. Example: "Ruby 3.2.0 — type :quit to exit"
//
//   2. Line prompt — printed before each input line. Signals "I am ready for
//      input." Example: ">> " or "irb(main):001> "
//
// Separating prompt logic from the loop lets you theme a REPL without touching
// any other code. Want emoji? Timestamps? ANSI colours? Implement Prompt.
//
// Example interaction (DefaultPrompt):
//
//   REPL — type :quit to exit          ← globalPrompt(), printed once
//   > hello                            ← linePrompt() = "> ", user types "hello"
//   hello                              ← EchoLanguage output
//   > :quit                            ← linePrompt() again
//   Goodbye!

/// Supplies the prompt strings that the REPL displays to the user.
///
/// - `globalPrompt()` — returned once at startup (the banner).
/// - `linePrompt()`   — returned before each input line.
///
/// Both methods return a `String`. The runner calls `outputFn` with the
/// returned string directly, so include any trailing spaces or newlines
/// you want as part of the string itself.
public protocol Prompt {
    /// A one-time banner printed when the REPL starts.
    ///
    /// Return an empty string `""` to suppress the banner entirely.
    func globalPrompt() -> String

    /// The per-line prompt, printed immediately before reading each input line.
    ///
    /// Typically ends with a space so the user's cursor appears after it,
    /// e.g. `"> "`.
    func linePrompt() -> String
}
