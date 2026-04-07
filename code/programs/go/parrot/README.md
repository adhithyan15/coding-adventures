# parrot (Go)

The world's simplest REPL: it repeats everything you say.

## What it does

Parrot is a demonstration program for the
[coding-adventures REPL framework](../../../packages/go/repl/). It wires
three components together:

| Component | Role |
|-----------|------|
| `repl.EchoLanguage` | Evaluates input by echoing it back unchanged |
| `ParrotPrompt` | Provides parrot-themed prompts with the 🦜 emoji |
| `repl.SilentWaiting` | Shows nothing while the evaluator "runs" |

The framework handles the read-eval-print loop, goroutine isolation, panic
recovery, and I/O injection. Parrot supplies only the personality.

## How to run

```
go run .
```

Or build first:

```
go build -o parrot .
./parrot
```

On Windows:

```
go build -o parrot.exe .
.\parrot.exe
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
go test ./... -v
```

## Architecture

```
stdin ──► inputFn ──► repl.RunWithIO ──► repl.EchoLanguage.Eval
                           │
                           ▼
                     ParrotPrompt (writes "🦜 > " before each line)
                           │
                           ▼
                     outputFn ──► stdout
```

The framework runs `Eval` in a goroutine (async mode) and polls for the
result every 100 ms via `repl.SilentWaiting`. For EchoLanguage this is
essentially instantaneous, but the same architecture scales to slow
evaluators (e.g., a network call or a compiled script).

## Files

| File | Description |
|------|-------------|
| `main.go` | Entry point — wires components and runs the REPL |
| `prompt.go` | `ParrotPrompt` struct — parrot-themed prompts |
| `main_test.go` | 17 integration tests with injected I/O |
| `go.mod` | Module declaration with local replace directive |
| `BUILD` | Build script for the build tool (Unix) |
| `BUILD_windows` | Build script for the build tool (Windows) |

## Relationship to the REPL package

This program depends on
`github.com/adhithyan15/coding-adventures/code/packages/go/repl`.
The `go.mod` file uses a `replace` directive to point at the local copy
in `code/packages/go/repl/`, so no network access is needed.

The program adds zero new framework logic — it is purely a demonstration
of what a minimal REPL program looks like when built on the framework.
