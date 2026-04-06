# parrot (Rust)

The world's simplest REPL: it repeats everything you say.

## What it does

Parrot is a demonstration program for the
[coding-adventures REPL framework](../../../packages/rust/repl/). It wires
three components together:

| Component | Role |
|-----------|------|
| `repl::EchoLanguage` | Evaluates input by echoing it back unchanged |
| `ParrotPrompt` | Provides parrot-themed prompts with the 🦜 emoji |
| `repl::SilentWaiting` | Shows nothing while the evaluator "runs" |

The framework handles the async eval loop (each evaluation runs on a dedicated
OS thread), panic recovery, and I/O injection. Parrot supplies only the
personality.

## How to run

```
cargo run
```

Or build first:

```
cargo build --release
./target/release/parrot
```

## Example session

```
🦜 Parrot REPL
I repeat everything you say! Type :quit to exit.

🦜 > hello
hello
🦜 > the quick brown fox
the quick brown fox
🦜 > :quit
Goodbye! 🦜
```

## How to test

```
cargo test
```

Or with output visible:

```
cargo test -- --nocapture
```

## Architecture

```
stdin ──► input_fn ──► run_with_io ──► EchoLanguage::eval (worker thread)
               │            │
               │            ▼
               │       SilentWaiting (polls result channel at 100ms)
               │            │
               │            ▼
               │       output_fn ──► stdout
               │
               └── prints "🦜 > " prompt before each read
```

The prompt is printed inside `input_fn` (before the `stdin.lock().lines()`
read) rather than by the runner, because the Rust framework's `run_with_io`
is generic over any input closure — it does not know whether the input is
interactive or piped. Printing the prompt in the closure keeps it adjacent
to the read, which is the correct place.

## Files

| File | Description |
|------|-------------|
| `src/main.rs` | Entry point — wires components and runs the REPL |
| `src/prompt.rs` | `ParrotPrompt` struct — parrot-themed prompts |
| `src/lib.rs` | Library target — re-exports `prompt` module for tests |
| `tests/parrot_test.rs` | 17 integration tests with injected I/O |
| `Cargo.toml` | Crate manifest with `[workspace]` isolation |
| `BUILD` | Build and test script for the build tool |

## Relationship to the repl crate

This program depends on the `repl` crate at
`code/packages/rust/repl/`. The `Cargo.toml` uses a `path` dependency
to point at the local copy — no network access is needed.

The `[workspace]` declaration in `Cargo.toml` prevents Cargo from
including this program in the `code/packages/rust/` workspace. Each
program in this repo is a standalone workspace.

## Module design

The Rust binary crate has both a `[lib]` target (`src/lib.rs`) and a
`[[bin]]` target (`src/main.rs`). The library re-exports `ParrotPrompt`
so integration tests in `tests/` can import it with `use parrot::prompt::ParrotPrompt`.
Without the library target, `tests/` would not be able to access internal
modules of the binary.
