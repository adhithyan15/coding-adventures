# hello-world-csharp

The first C# program — the starting point for the computing-stack journey in
the .NET ecosystem.

## What it does

Prints `Hello, World!` to the console using C# top-level statements on .NET 9.

## Usage

```bash
dotnet run --disable-build-servers
```

## How it fits in the stack

This is the same program as `go/hello-world`, `python/hello-world`, and every
other language's entry point — the goal is to trace "Hello, World!" all the way
down through every layer:

```
Source code (Program.cs)
→ Roslyn compiler     (C# → CIL bytecode)
→ CLR JIT compiler    (CIL → native machine code)
→ CPU execution       (native instructions → syscall → terminal output)
```

The .NET CLR is its own full execution stack, analogous to what we build by
hand in coding-adventures: lexer → parser → compiler → VM → CPU simulator.

## Language

C# with top-level statements (C# 9+). No `class Program` or `static void Main`
boilerplate — the compiler synthesises the entry point automatically.

## Why `--disable-build-servers`

The `dotnet` CLI starts a long-lived MSBuild server in the background. In CI
this causes port conflicts when the build runs more than once on the same
runner. `--disable-build-servers` disables the server for this invocation,
keeping CI builds reliable and side-effect-free.
