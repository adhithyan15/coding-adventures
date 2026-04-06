import CodingAdventuresRepl

// ParrotPrompt.swift — The personality layer for the Parrot REPL.
//
// A parrot's defining characteristic is repetition: you say something, it
// says it back. This prompt reflects that identity with parrot-themed text
// and the 🦜 emoji in the line prompt.
//
// The global prompt (banner) explains what the program does in one sentence
// — important because new users shouldn't need to read documentation to
// understand a simple echo program.
//
// The line prompt uses 🦜 to give the user a clear visual cue that they
// are talking to the parrot. Emoji in prompts is non-standard, but the
// parrot theme justifies it — it's whimsical and memorable.
//
// Example session:
//
//   🦜 Parrot REPL                          ← globalPrompt()
//   I repeat everything you say! Type :quit to exit.
//
//   🦜 >  hello                             ← linePrompt() + user input
//   hello                                   ← EchoLanguage output
//   🦜 >  :quit
//   Goodbye!

/// The `Prompt` implementation for the Parrot REPL.
///
/// - `globalPrompt()` — a two-line banner announcing the Parrot REPL.
/// - `linePrompt()`   — `"🦜 > "`, a parrot-emoji-prefixed cursor.
public struct ParrotPrompt: Prompt {
    public init() {}

    /// A parrot-themed startup banner.
    ///
    /// Uses a multi-line string literal ending with a blank line so the
    /// first input prompt appears visually separated from the banner.
    public func globalPrompt() -> String {
        """
        🦜 Parrot REPL
        I repeat everything you say! Type :quit to exit.

        """
    }

    /// The per-line prompt — a parrot emoji followed by `" > "`.
    ///
    /// The trailing space keeps the user's cursor visually separated from
    /// the `>` character.
    public func linePrompt() -> String { "🦜 > " }
}
