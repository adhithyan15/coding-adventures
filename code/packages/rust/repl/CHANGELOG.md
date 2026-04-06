# Changelog — repl (Rust)

All notable changes to this crate will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-06

### Added

- `EvalResult` enum with three variants: `Ok(Option<String>)`, `Error(String)`, `Quit`.
- `Language` trait — pluggable evaluation backend (`Send + Sync`).
- `Prompt` trait — customisable prompt strings (`global_prompt`, `line_prompt`).
- `Waiting` trait — tick-based animation with type-erased state (`Box<dyn Any + Send>`).
- `runner::run_with_io` — the core REPL loop with fully injected I/O (closures for
  input and output); eval runs on a background thread, panics are caught with
  `catch_unwind`, and the waiting animation is driven by polling `mpsc::recv_timeout`.
- `runner::run` — convenience wrapper over `run_with_io` using `stdin` / `stdout`.
- `EchoLanguage` — built-in `Language` that echoes input and recognises `:quit`.
- `DefaultPrompt` — built-in `Prompt` returning `"> "` and `"... "`.
- `SilentWaiting` — built-in no-op `Waiting` with 100 ms tick interval; suitable for
  tests and non-interactive sessions.
- Six integration tests covering: single-line echo, immediate quit, EOF termination,
  multi-line echo, error formatting (`"Error: ..."` prefix), and `Ok(None)` silence.
- Literate doc comments on every public type, trait, and function.
- `README.md` with architecture overview, trait table, and usage examples.
- `BUILD` file for the monorepo build system.
- No external dependencies — standard library only.
