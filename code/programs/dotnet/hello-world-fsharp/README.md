# hello-world-fsharp

The first F# program — the starting point for the computing-stack journey in
the .NET ecosystem, from a functional-programming perspective.

## What it does

Prints `Hello, World!` to the console using F# on .NET 9.

## Usage

```bash
dotnet run --disable-build-servers
```

## How it fits in the stack

```
Source code (Program.fs)
→ F# compiler (fsc)   (F# → CIL bytecode)
→ CLR JIT compiler    (CIL → native machine code)
→ CPU execution       (native instructions → syscall → terminal output)
```

F# and C# compile to the same CIL bytecode and run on the same CLR. The
difference is in the source language: F# is functional-first, with immutability
as the default, pattern matching as the primary control-flow tool, and a
Hindley-Milner type system with full inference.

## Relation to other functional languages in this repo

| Language | VM       | Evaluation | Primary paradigm        |
|----------|----------|------------|-------------------------|
| F#       | CLR      | Eager      | Functional-first, OO    |
| Haskell  | GHC RTS  | Lazy       | Pure functional         |
| Elixir   | BEAM     | Eager      | Functional, actor model |

All three model programs as transformations of immutable data, avoiding shared
mutable state — the root cause of most concurrency bugs.

## Why `--disable-build-servers`

The `dotnet` CLI starts a long-lived MSBuild server in the background. In CI
this causes port conflicts when the build runs more than once on the same
runner. `--disable-build-servers` disables the server for this invocation,
keeping CI builds reliable and side-effect-free.
