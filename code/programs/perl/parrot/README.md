# Parrot REPL (Perl)

A demonstration program that uses the `CodingAdventures::Repl` Perl framework
to build the world's simplest REPL: one that echoes back whatever you type.

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

This program sits at the top of the Perl REPL framework package hierarchy:

```
code/programs/perl/parrot/        ← this program
    uses ↓
code/packages/perl/repl/          ← pluggable REPL framework
    └── lib/CodingAdventures/Repl/Loop.pm          — run() engine
    └── lib/CodingAdventures/Repl/EchoLanguage.pm  — echo language plug-in
    └── lib/CodingAdventures/Repl/SilentWaiting.pm — no-op waiting plug-in
```

The parrot provides its own `Parrot::Prompt` class, showing how to swap out
the built-in `DefaultPrompt` with a custom one.

## Usage

```bash
perl parrot.pl
```

## Running tests

```bash
cpanm --with-test --installdeps --quiet .
prove -l -v t/
```

## Architecture

Three plug-in objects are wired together via named arguments to
`CodingAdventures::Repl::Loop::run()`:

| Argument | Class | Role |
|---|---|---|
| `language` | `EchoLanguage` | Echoes input; returns `'quit'` on `:quit` |
| `prompt` | `Parrot::Prompt` | Parrot-themed banner and line prompt |
| `waiting` | `SilentWaiting` | No-op (eval is instantaneous) |

I/O is injected via `input_fn` and `output_fn` coderefs, which makes the same
code path testable without a terminal.

## File layout

```
parrot/
  parrot.pl           — main script
  lib/Parrot/
    Prompt.pm         — custom Prompt implementation
  t/
    test_parrot.t     — 16 Test2::V0 tests
  Makefile.PL         — build configuration
  cpanfile            — dependency declaration
  BUILD               — CI build command
  BUILD_windows       — skip marker for Windows CI
  README.md           — this file
  CHANGELOG.md        — version history
```
