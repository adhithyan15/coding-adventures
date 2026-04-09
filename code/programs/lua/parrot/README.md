# Parrot REPL (Lua)

A demonstration program that uses the `coding_adventures.repl` Lua framework to
build the world's simplest REPL: one that echoes back whatever you type.

## What it does

The Parrot REPL reads lines from standard input and prints each one back.
Type `:quit` to exit, or press Ctrl-D (Unix) / Ctrl-Z (Windows) to signal EOF.

```
Parrot REPL - I repeat everything you say! (:quit to exit)
🦜 > hello there
hello there
Parrot REPL - I repeat everything you say! (:quit to exit)
🦜 > squawk!
squawk!
Parrot REPL - I repeat everything you say! (:quit to exit)
🦜 > :quit
```

## How it fits in the stack

This program sits at the top of the REPL framework package hierarchy:

```
code/programs/lua/parrot/   ← this program
    uses ↓
code/packages/lua/repl/     ← pluggable REPL framework
    └── loop.lua            — run_with_io() engine
    └── echo_language.lua   — EchoLanguage plug-in
    └── silent_waiting.lua  — SilentWaiting plug-in
```

The parrot provides its own `ParrotPrompt` object, demonstrating how to swap
out the built-in `DefaultPrompt` with a custom one.

## Usage

```bash
lua main.lua
```

## Running tests

```bash
cd tests && busted . --verbose --pattern=test_
```

## Architecture

Three plug-in objects are wired together:

| Plug-in | Class | Role |
|---|---|---|
| Language | `EchoLanguage` | Echoes input; returns `{tag="quit"}` on `:quit` |
| Prompt | `ParrotPrompt` | Parrot-themed banner and line prompt |
| Waiting | `SilentWaiting` | No-op (eval is instantaneous) |

I/O is injected via function parameters to `Loop.run_with_io()`, which makes
the same code path testable without a terminal.
