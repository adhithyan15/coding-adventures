// DefaultPrompt.swift — The built-in Prompt implementation.
//
// This prompt is intentionally minimal: a one-line banner and a simple "> "
// line prompt. It's enough to make a usable REPL without any dependencies.
//
// Typical session appearance:
//
//   REPL — type :quit to exit        ← globalPrompt() (banner, shown once)
//   > hello                          ← linePrompt() = "> ", user types "hello"
//   hello
//   > :quit                          ← linePrompt() again
//   Goodbye!
//
// If you want custom branding (e.g. a language name, version number, or emoji),
// create your own struct conforming to Prompt. See ParrotPrompt in the
// code/programs/swift/parrot package for an example.

/// The built-in `Prompt` implementation.
///
/// - `globalPrompt()` — `"REPL — type :quit to exit\n"` (with trailing newline)
/// - `linePrompt()`   — `"> "` (no trailing newline; user types on same line)
public struct DefaultPrompt: Prompt {
    public init() {}

    /// A one-time startup banner.
    ///
    /// Includes a trailing newline so the first line prompt appears on a new
    /// line after the banner.
    public func globalPrompt() -> String {
        "REPL — type :quit to exit\n"
    }

    /// The per-line prompt shown before each user input.
    ///
    /// The trailing space separates the prompt character from the user's cursor.
    public func linePrompt() -> String { "> " }
}
