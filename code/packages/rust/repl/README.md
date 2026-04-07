# repl (Rust)

A pluggable Read-Eval-Print Loop (REPL) framework with async evaluation and injected I/O.

## What This Is

This crate provides the scaffolding for building interactive REPLs in Rust. It separates the
four concerns of a REPL into pluggable traits, so you can swap out the language, the prompt
style, and the waiting animation independently — without changing the loop logic.

No external dependencies. Standard library only.

## How It Fits in the Stack

A REPL is the user-facing shell for any interactive language runtime. This framework sits
above the evaluator (your language implementation) and below the terminal UI. It handles
the plumbing: spawning eval threads, catching panics, polling for results, and routing
output back to the caller.

## Architecture

```text
┌─────────────────────────────────────┐
│              run_with_io            │  ← entry point
│                                     │
│  input_fn()  ──► spawn thread ──►  │  Language::eval(input)
│                       │            │
│  Waiting::tick loop   │            │
│                       ▼            │
│  output_fn() ◄── EvalResult        │
└─────────────────────────────────────┘
```

### Traits

| Trait | Purpose | Built-in |
|-------|---------|---------|
| `Language` | Evaluates one line of input | `EchoLanguage` |
| `Prompt` | Provides prompt strings | `DefaultPrompt` |
| `Waiting` | Animates while waiting | `SilentWaiting` |

### `EvalResult`

```rust
pub enum EvalResult {
    Ok(Option<String>),  // success; print optional output
    Error(String),       // recoverable error; print message; continue
    Quit,                // end the session
}
```

## Usage

### Minimal (echo REPL)

```rust
use std::sync::Arc;
use repl::runner::run_with_io;
use repl::echo_language::EchoLanguage;
use repl::default_prompt::DefaultPrompt;
use repl::silent_waiting::SilentWaiting;

let inputs = vec!["hello".to_string(), ":quit".to_string()];
let mut iter = inputs.into_iter();
let mut outputs = Vec::new();

run_with_io(
    Arc::new(EchoLanguage),
    Arc::new(DefaultPrompt),
    Arc::new(SilentWaiting),
    || iter.next(),
    |s| outputs.push(s.to_string()),
);

assert_eq!(outputs, vec!["hello"]);
```

### Custom language backend

```rust
use repl::language::Language;
use repl::types::EvalResult;

struct Calculator;

impl Language for Calculator {
    fn eval(&self, input: &str) -> EvalResult {
        match input.trim() {
            ":quit" | "exit" => EvalResult::Quit,
            expr => match expr.parse::<i64>() {
                Ok(n) => EvalResult::Ok(Some(n.to_string())),
                Err(_) => EvalResult::Error(format!("not a number: {expr}")),
            },
        }
    }
}
```

### Interactive terminal

```rust
use std::sync::Arc;
use repl::runner::run;
use repl::echo_language::EchoLanguage;
use repl::default_prompt::DefaultPrompt;
use repl::silent_waiting::SilentWaiting;

run(
    Arc::new(EchoLanguage),
    Arc::new(DefaultPrompt),
    Arc::new(SilentWaiting),
);
```

## Running Tests

```bash
cargo test -p repl
```
