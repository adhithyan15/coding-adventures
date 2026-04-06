# CodingAdventuresRepl (Swift)

A minimal, pluggable **REPL framework** for Swift — standard library and
Foundation only.

## What Is a REPL?

REPL stands for **Read–Eval–Print Loop** — the interactive shell model used
by Swift (`swift repl`), Python (`python -i`), Ruby (`irb`), Node.js (`node`),
Elixir (`iex`), and countless others:

```
1. Read   — print a prompt, read a line of input from the user
2. Eval   — pass that line to the language evaluator
3. Print  — display the result (or error message)
4. Loop   — go back to step 1
```

This package provides the *loop* infrastructure. The **language**, **prompt**,
and **waiting animation** are entirely pluggable via three protocols.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                     runWithIO(...)                        │
│                                                          │
│  ┌──────────┐   ┌──────────┐   ┌──────────────────────┐ │
│  │  Prompt  │   │ Language │   │      Waiting         │ │
│  │(protocol)│   │(protocol)│   │     (protocol)       │ │
│  └──────────┘   └──────────┘   └──────────────────────┘ │
│       ↑               ↑                  ↑               │
│ DefaultPrompt   EchoLanguage      SilentWaiting          │
│  (built-in)     (built-in)         (built-in)            │
│                                                          │
│  I/O injection: inputFn / outputFn closures              │
└──────────────────────────────────────────────────────────┘
```

## The Three Protocols

### `Language`

Maps user input to an `EvalResult`:

| Result            | Meaning                                        |
|-------------------|------------------------------------------------|
| `.ok(nil)`        | Success, nothing to display                    |
| `.ok("text")`     | Success with displayable output                |
| `.error("msg")`   | Failure — REPL prints `"Error: msg"`           |
| `.quit`           | End the session — REPL prints `"Goodbye!"`     |

```swift
public protocol Language {
    func eval(_ input: String) -> EvalResult
}
```

`eval` is synchronous. In `.async_mode` the runner dispatches it to a
background `DispatchQueue` thread.

### `Prompt`

Supplies the strings shown to the user:

```swift
public protocol Prompt {
    func globalPrompt() -> String   // startup banner (once)
    func linePrompt() -> String     // per-input prompt (each iteration)
}
```

### `Waiting`

Tick-driven animation while the evaluator runs (async mode only):

```swift
public protocol Waiting {
    associatedtype State
    func start() -> State
    func tick(_ state: State) -> State
    func tickMs() -> Int
    func stop(_ state: State)
}
```

The runner calls `start()` before eval, `tick(state)` every `tickMs()`
milliseconds while waiting, and `stop(state)` when eval finishes.

## Quick Start

```swift
import CodingAdventuresRepl

runWithIO(
    language: EchoLanguage(),
    prompt: DefaultPrompt(),
    waiting: SilentWaiting(),
    inputFn: { readLine() },
    outputFn: { print($0, terminator: "") }
)
```

Session:

```
REPL — type :quit to exit
> hello
hello
> :quit
Goodbye!
```

## Custom Language

```swift
import CodingAdventuresRepl

struct DoubleLanguage: Language {
    func eval(_ input: String) -> EvalResult {
        if input == ":quit" { return .quit }
        if let n = Int(input) { return .ok(String(n * 2)) }
        return .error("not an integer: \(input)")
    }
}

runWithIO(
    language: DoubleLanguage(),
    prompt: DefaultPrompt(),
    waiting: SilentWaiting(),
    inputFn: { readLine() },
    outputFn: { print($0, terminator: "") }
)
```

```
REPL — type :quit to exit
> 7
14
> hello
Error: not an integer: hello
> :quit
Goodbye!
```

## Custom Prompt

```swift
struct MyPrompt: Prompt {
    func globalPrompt() -> String { "MyLang 1.0 — :quit to exit\n" }
    func linePrompt() -> String { "myLang> " }
}
```

## Custom Waiting (spinner)

```swift
import Foundation

struct SpinnerWaiting: Waiting {
    typealias State = Int
    let frames = ["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"]

    func start() -> Int { 0 }
    func tick(_ s: Int) -> Int {
        print("\r\(frames[s % frames.count]) ", terminator: "")
        fflush(stdout)
        return s + 1
    }
    func tickMs() -> Int { 80 }
    func stop(_ s: Int) {
        print("\r  \r", terminator: "")
        fflush(stdout)
    }
}
```

## Testing (I/O Injection)

```swift
import XCTest
import CodingAdventuresRepl

func testEcho() {
    var output: [String] = []
    var inputs: [String?] = ["hello", ":quit"]
    runWithIO(
        language: EchoLanguage(),
        prompt: DefaultPrompt(),
        waiting: SilentWaiting(),
        inputFn: { inputs.isEmpty ? nil : inputs.removeFirst() },
        outputFn: { output.append($0) }
    )
    XCTAssertTrue(output.contains("hello"))
}
```

No real stdin/stdout — fully deterministic.

## Sync vs Async Mode

| Mode            | Eval dispatched to    | Waiting plugin invoked? |
|-----------------|-----------------------|-------------------------|
| `.sync`         | Calling thread        | No                      |
| `.async_mode`   | `DispatchQueue.global()` | Yes                  |

`Mode.default` is `.async_mode`.

> **Note on naming:** Swift 5.5+ reserves the keyword `async` for structured
> concurrency. The enum case is therefore spelled `async_mode`. Use
> `Mode.async_mode` (not `Mode.async`) everywhere.

## Built-in Implementations

| Type              | Protocol   | Description                               |
|-------------------|------------|-------------------------------------------|
| `EchoLanguage`    | `Language` | Mirrors input; `:quit` ends the session   |
| `DefaultPrompt`   | `Prompt`   | Simple `"> "` prompt with one-line banner |
| `SilentWaiting`   | `Waiting`  | No-op; 100 ms tick interval               |

## Where It Fits

```
Logic Gates → Arithmetic → CPU → ARM/RISC-V → Assembler
  → Lexer → Parser → Compiler → VM → [REPL]
```

The REPL framework sits at the top of the computing stack, wrapping any
evaluator in a polished interactive shell.

## Requirements

- Swift 5.9+
- macOS 10.15+ / Linux with Foundation (DispatchGroup)
- No external dependencies
