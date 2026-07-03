# Changelog

All notable changes to `wolfram-repl` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/) and this project uses
[Semantic Versioning](https://semver.org/).

## [0.1.1] — 2026-06-25

### Added (W-21 — pattern operator sugar)

- An end-to-end test confirming the W-21 pattern operators (`a | b`,
  `patt /; test`, `patt ? fn`, `expr //. rules`) parse and evaluate through the
  REPL's real lex → parse → lower → eval `feed` path. No driver change was
  needed — the operator support comes entirely from the upstream lexer/parser
  (0.4.0) and runtime (0.17.0) lowering; this is a regression guard.

## [0.1.0] — 2026-06-17

Initial release — the **W-5** REPL of the Wolfram-language lane (MA04 §7.3).

### Added

- `WolframRepl` — a persistent interactive driver over `wolfram-runtime`'s
  `WolframSession`, with `In[n]:= ` / `... ` prompts and Mathematica-style
  newline-terminated line continuation: it keeps reading physical lines while a
  `[ ]`/`{ }`/`( )` is open or a `"…"`/`(* *)` is unterminated. The accumulation
  buffer is size-capped so input that never balances cannot grow memory without
  bound.
- `ReplResponse` (`Output`/`NeedMore`/`Quit`) and `feed(line)` for testing the
  driver without real I/O; `run(reader, writer)` drives a full session over any
  `BufRead`/`Write`.
- `Quit`/`Quit[]`/`Exit`/`Exit[]` (and lowercase `quit`/`exit`) and Ctrl-D end the
  session; surface errors print and the session continues.
- The `wolfram` binary and its historical alias `math`, both driving `run` over
  stdin/stdout.
