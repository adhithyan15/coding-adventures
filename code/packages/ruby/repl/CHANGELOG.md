# Changelog

All notable changes to the `coding_adventures_repl` gem will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-06

### Added

- **Language interface** (`Repl::Language`) — module documenting the `eval(input)` contract:
  returns `[:ok, String|nil]`, `[:error, String]`, or `:quit`.
- **Prompt interface** (`Repl::Prompt`) — module documenting `global_prompt` and `line_prompt`.
- **Waiting interface** (`Repl::Waiting`) — module documenting the `start/tick/tick_ms/stop`
  state-machine for async-eval feedback animations.
- **Loop class** (`Repl::Loop`) — the core Read-Eval-Print cycle with:
  - Async eval via `Thread.new { language.eval(input) }`
  - Poll-based waiting: `thread.join(tick_ms / 1000.0)` driving the Waiting interface
  - Exception safety: `begin/rescue` inside the eval thread converts unhandled exceptions to
    `[:error, message]` results so the session survives backend bugs
  - Full I/O injection via `input_fn` and `output_fn` Procs
  - Handles nil from `input_fn` as EOF (clean termination)
  - Handles `[:ok, nil]` by printing nothing (silent success for side-effect expressions)
- **EchoLanguage** (`Repl::EchoLanguage`) — built-in Language that echoes input; returns `:quit`
  on `":quit"`.
- **DefaultPrompt** (`Repl::DefaultPrompt`) — built-in Prompt returning `"> "` and `"... "`.
- **SilentWaiting** (`Repl::SilentWaiting`) — built-in no-op Waiting with 100ms tick interval.
- **Convenience API** (`Repl.run` and `Repl.run_with_io`) — thin wrappers over `Loop.new` with
  sensible defaults.
- Comprehensive test suite with 14 test cases covering all interfaces and edge cases.
- Knuth-style literate documentation throughout all source files.
