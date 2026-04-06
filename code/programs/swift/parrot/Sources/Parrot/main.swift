import CodingAdventuresRepl

// main.swift — Parrot: the world's simplest REPL.
//
// A parrot repeats everything you say. That is the entire feature set.
//
// This program is:
//   1. A demonstration of the CodingAdventuresRepl framework with real I/O.
//   2. A proof that you can build a complete interactive program in 10 lines
//      by composing four small building blocks.
//   3. A gentle introduction to REPL programs for new learners.
//
// Building blocks used:
//
//   EchoLanguage  — the evaluator. Input → output, unchanged. ":quit" → exit.
//   ParrotPrompt  — the prompt. Shows the 🦜 banner and per-line prompt.
//   SilentWaiting — the busy indicator. EchoLanguage is instant, so no
//                   spinner is needed. SilentWaiting satisfies the protocol
//                   requirement with no visible behaviour.
//   runWithIO     — the loop. Wires the three plugins together and drives
//                   stdin/stdout.
//
// How to run:
//
//   swift run
//
// Example session:
//
//   🦜 Parrot REPL
//   I repeat everything you say! Type :quit to exit.
//
//   🦜 >  squawk
//   squawk
//   🦜 >  Polly wants a cracker
//   Polly wants a cracker
//   🦜 >  :quit
//   Goodbye!

// Wire everything together and hand off to the framework.
// `readLine()` handles line buffering and Ctrl-D (EOF) correctly on all
// platforms. `print($0, terminator: "")` avoids adding an extra newline
// on top of any newline already embedded in the output string (e.g. the
// banner uses its own `\n` characters).
runWithIO(
    language: EchoLanguage(),
    prompt: ParrotPrompt(),
    waiting: SilentWaiting(),
    inputFn: { readLine() },
    outputFn: { print($0, terminator: "") }
)
