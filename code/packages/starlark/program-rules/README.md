# Starlark Program Rules

Build rule definitions for executable program packages in the coding-adventures monorepo.

## What Are These Files?

Each `.star` file defines a **build rule** for programs (binaries) — code that runs directly rather than being imported by other code. The key difference from library rules is the **entry point**: a binary has a specific file you execute (like `main.py` or `src/main.rs`).

## Library vs Binary

| Aspect | Library | Binary |
|--------|---------|--------|
| Purpose | Imported by other code | Runs directly |
| Location | `code/packages/<lang>/` | `code/programs/<lang>/` |
| Entry point | No (it's a module) | Yes (`main.py`, `src/main.rs`, etc.) |
| Example | `py_library("logic-gates")` | `py_binary("build-tool")` |

## Available Rules

| File | Rule Function | Language | Entry Point Default |
|------|--------------|----------|-------------------|
| `python_binary.star` | `py_binary()` | Python | `main.py` |
| `go_binary.star` | `go_binary()` | Go | N/A (always `func main()`) |
| `ruby_binary.star` | `ruby_binary()` | Ruby | `main.rb` |
| `typescript_binary.star` | `ts_binary()` | TypeScript | `src/index.ts` |
| `rust_binary.star` | `rust_binary()` | Rust | N/A (always `src/main.rs`) |
| `elixir_binary.star` | `elixir_binary()` | Elixir | `lib/main.ex` |

Note: Go and Rust don't need entry_point parameters because their conventions are rigid — Go always uses `func main()` in `package main`, and Rust always uses `fn main()` in `src/main.rs`.

## Usage

In a program's BUILD file:

```python
load("//rules:python_binary.star", "py_binary")

py_binary(
    name = "build-tool",
    srcs = ["src/**/*.py"],
    deps = ["python/directed-graph", "python/starlark-vm"],
    entry_point = "main.py",
)
```

## Common Parameters

All binary rules share these parameters:

- **name** — Program name matching the directory name
- **srcs** — Glob patterns for change detection
- **deps** — Dependencies as `"language/package-name"` strings

Most binary rules also have:
- **entry_point** — The file to execute (except Go and Rust, which have fixed conventions)

## How This Fits in the Stack

```
BUILD files (per-program)          — "I am a Go binary called build-tool"
    |
    v
Program rules (.star)              — "Here's what go_binary means"  <-- YOU ARE HERE
    |
    v
Build tool (Go program)            — "I'll compile it, test it, validate it"
    |
    v
Language toolchains                — "I'll produce the actual executable"
(go build, cargo build, etc.)
```
