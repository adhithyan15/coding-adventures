# Changelog

All notable changes to the coding-adventures-repl package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added
- `Language` abstract base class — pluggable evaluator interface with four
  possible return values: `("ok", str)`, `("ok", None)`, `("error", str)`,
  `"quit"`.
- `Prompt` abstract base class — pluggable prompt-string provider with
  `global_prompt()` and `line_prompt()`.
- `Waiting` abstract base class — tick-model animation plugin with `start()`,
  `tick(state)`, `tick_ms()`, and `stop(state)`.
- `EchoLanguage` — concrete Language that mirrors input back unchanged;
  `":quit"` returns `"quit"` to end the session.
- `DefaultPrompt` — concrete Prompt returning `"> "` and `"... "`.
- `SilentWaiting` — concrete Waiting with all-no-op callbacks and 100 ms
  tick interval.
- `loop.run_with_io()` — REPL loop with injected `input_fn` / `output_fn`
  callables for testing and embedding.
- `loop.run()` — REPL loop wired to `input()` / `print()` for interactive
  terminal use.
- `Repl` namespace class exposing `Repl.run` and `Repl.run_with_io` as
  static methods.
- Async eval via `threading.Thread`; the main thread polls with
  `thread.join(timeout)` while driving waiting ticks.
- Exception safety: unhandled exceptions inside `language.eval()` are caught
  in the background thread and converted to `("error", message)` results so
  the REPL never crashes.
- `None` return from `input_fn` treated as end-of-input (equivalent to
  Ctrl-D), terminating the loop cleanly.
- Full test suite with six test scenarios (echo, quit, multiple turns, nil
  output, error, exception safety) plus unit tests for all three built-in
  implementations.
- `py.typed` marker for PEP 561 typed-package support.
- `pyproject.toml` with hatchling build backend, src layout, no runtime
  dependencies, and dev extras (pytest, pytest-cov, ruff, mypy).
- `BUILD` file for the monorepo build tool: `uv venv`, `uv pip install`,
  `pytest`.
- Literate-programming-style docstrings throughout — explanations, diagrams,
  and worked examples inline with the code.
