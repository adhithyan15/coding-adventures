# Changelog

All notable changes to `@coding-adventures/repl` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-06

### Added

- **`EvalResult` type** — tagged union with three variants: `ok` (with nullable
  string output), `error` (with message), and `quit`.
- **`Language` interface** — single async `eval(input)` method; implementors
  return `EvalResult` Promises.
- **`Prompt` interface** — `globalPrompt()` and `linePrompt()` for primary and
  continuation prompt strings.
- **`Waiting` interface** — state-machine animation driver: `start()`, `tick()`,
  `tickMs()`, `stop()`.
- **`InputFn` and `OutputFn` types** — plain function types for injecting I/O,
  enabling full testability without touching `process.stdin`/`stdout`.
- **`EchoLanguage`** — built-in Language that echoes every input back unchanged;
  recognises `:quit` as the exit command.
- **`DefaultPrompt`** — built-in Prompt returning `"> "` (global) and `"... "`
  (line continuation).
- **`SilentWaiting`** — built-in no-op Waiting implementation; safe for tests,
  CI, and non-interactive environments.
- **`runWithIo()`** — fully injectable async REPL loop; drives the
  Language/Prompt/Waiting interfaces with explicit InputFn and OutputFn.
  Uses `setInterval` + `clearInterval` to interleave animation ticks with the
  async eval Promise.
- **`run()`** — convenience entry point wired to `process.stdin`/`stdout` via
  Node.js `readline`; uses `DefaultPrompt` and `SilentWaiting` by default.
- **Vitest test suite** — 6 integration tests covering: echo, quit, multi-turn,
  null output, error formatting, and exception safety; plus unit tests for all
  three built-in implementations.
