# Parrot (Swift)

A minimal interactive REPL that repeats everything you type — because it's a
parrot.

## What Does It Do?

Whatever you type, Parrot repeats back. That's it.

```
🦜 Parrot REPL
I repeat everything you say! Type :quit to exit.

🦜 >  squawk
squawk
🦜 >  Polly wants a cracker
Polly wants a cracker
🦜 >  :quit
Goodbye!
```

## Why Does This Exist?

Parrot is the Swift reference implementation of the Parrot REPL program — a
pattern repeated across every language in the coding-adventures monorepo
(Python, Ruby, Go, Rust, Elixir, TypeScript, Lua, ...). Each language's Parrot
uses that language's REPL framework package, demonstrating:

1. How to build a runnable program on top of the framework.
2. How to wire together `Language`, `Prompt`, and `Waiting` plugins.
3. How to write I/O-injected tests for a REPL program.

## How to Run

```bash
swift run
```

## How to Test

```bash
swift test --enable-code-coverage
```

## Architecture

```
main.swift
  └─ runWithIO(
        language: EchoLanguage(),   ← from CodingAdventuresRepl
        prompt:   ParrotPrompt(),   ← defined here
        waiting:  SilentWaiting(),  ← from CodingAdventuresRepl
        inputFn:  readLine,
        outputFn: print(terminator:"")
     )
```

Two custom types, one entry point, zero logic.

## ParrotPrompt

```swift
public struct ParrotPrompt: Prompt {
    public func globalPrompt() -> String {
        """
        🦜 Parrot REPL
        I repeat everything you say! Type :quit to exit.

        """
    }
    public func linePrompt() -> String { "🦜 > " }
}
```

Replace `EchoLanguage` with your own `Language` implementation and swap
`ParrotPrompt` for your own `Prompt` to turn this into a real interactive
tool.

## Dependencies

- [`CodingAdventuresRepl`](../../../packages/swift/repl) — the Swift REPL
  framework (local path dependency, no internet required).

## Where It Fits

```
Logic Gates → Arithmetic → CPU → ARM/RISC-V → Assembler
  → Lexer → Parser → Compiler → VM → REPL → [Parrot]
```

Parrot sits at the very top of the stack — it's the simplest possible program
that uses the REPL framework end-to-end.
