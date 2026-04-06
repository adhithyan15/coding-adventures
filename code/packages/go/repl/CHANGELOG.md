# Changelog

All notable changes to the repl package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added

- `Language` interface — pluggable evaluator with `Eval(string) Result`.
- `Result` struct — three-state outcome (`"ok"`, `"error"`, `"quit"`) with
  `Output` string and `HasOutput` boolean for explicit empty-output control.
- `Prompt` interface — `GlobalPrompt()` and `LinePrompt()` for configurable
  prompt strings.
- `Waiting` interface — `Start / Tick / Stop` lifecycle for spinner animations,
  with a `TickMs()` interval control.
- `InputFn` and `OutputFn` type aliases for fully injectable I/O.
- `Run()` — convenience entry point using `os.Stdin` / `os.Stdout`.
- `RunWithIO()` — fully injectable loop; accepts custom `InputFn` and
  `OutputFn` for testing and pipeline use.
- Async evaluation via goroutines and a buffered channel; the main loop ticks
  the `Waiting` animation while eval is in flight.
- Panic recovery in the eval goroutine — panics are caught and surfaced as
  `"error"` Results, keeping the REPL alive.
- `EchoLanguage` — built-in `Language` that echoes input; `:quit` triggers exit.
- `DefaultPrompt` — built-in `Prompt` returning `"> "` and `"... "`.
- `SilentWaiting` — built-in no-op `Waiting` with 100 ms tick interval.
- Comprehensive test suite (8 test cases) covering echo, quit, EOF, panic
  recovery, silent-ok output suppression, and the DefaultPrompt / SilentWaiting
  built-ins.
- Literate-programming documentation throughout all source files.
- `required_capabilities.json` declaring `time:read` for `time.NewTicker`.
- `gen_capabilities.go` generated capability cage.
