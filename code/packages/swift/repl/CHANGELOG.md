# Changelog

All notable changes to the CodingAdventuresRepl Swift package will be
documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-06

### Added

- `EvalResult` enum — the three possible outcomes from `Language.eval`:
  `.ok(String?)` (success with optional output), `.error(String)` (failure
  with message), `.quit` (end the session). Conforms to `Equatable`.
- `Mode` enum — controls how the runner dispatches eval: `.sync` (on the
  calling thread) or `.async_mode` (background `DispatchQueue` thread).
  Named `async_mode` rather than `async` because `async` is a reserved Swift
  keyword. `Mode.default` is `.async_mode`.
- `Language` protocol — `eval(_ input: String) -> EvalResult`. Synchronous;
  the runner handles async dispatch.
- `Prompt` protocol — `globalPrompt() -> String` (startup banner) and
  `linePrompt() -> String` (per-line prompt).
- `Waiting` protocol — tick-driven animation plugin with associated type
  `State`. Methods: `start() -> State`, `tick(_ state: State) -> State`,
  `tickMs() -> Int`, `stop(_ state: State)`.
- `EchoLanguage` struct — mirrors input back unchanged; `:quit` returns
  `.quit`. The canonical test double for the framework.
- `DefaultPrompt` struct — `"REPL — type :quit to exit\n"` banner and `"> "`
  line prompt.
- `SilentWaiting` struct — no-op `Waiting` implementation; state is an `Int`
  tick counter; `tickMs()` = 100 ms.
- `runWithIO<L, P, W>` — generic free function that drives the REPL loop.
  Accepts injected `inputFn: () -> String?` and `outputFn: (String) -> Void`
  closures for deterministic testing. In `.async_mode` uses `DispatchGroup` +
  `DispatchQueue.global()` with `group.wait(timeout: .milliseconds(tickMs))`
  polling — identical in spirit to Python's `thread.join(timeout)`.
- 25 XCTest cases covering: echo, quit, empty string, whitespace, error
  display, `.ok(nil)` silence, EOF via nil, multiple inputs, sync mode, async
  mode, nil waiting plugin in async mode, long sequence (50 inputs), banner
  once, line prompt count, output function correctness, Mode.default, and
  CountingWaiting plugin invocation.
- Literate-programming-style comments throughout — diagrams, truth tables,
  rationale, and worked examples inline with every type and function.
- `Package.swift` targeting swift-tools-version 5.9 for broad compatibility.
- `BUILD` file for the monorepo build tool (xcrun-aware for macOS).
- `BUILD_windows` file for Windows CI (skips gracefully if Swift not present).

### Design Notes

- `async` is a reserved keyword in Swift 5.5+ (structured concurrency). The
  `Mode` case is therefore spelled `async_mode`. All documentation calls this
  out explicitly so future maintainers don't try to rename it.
- The `Waiting` protocol uses an associated type (`State`) rather than `Any`
  so implementations can carry strongly-typed state without heap allocation.
- `runWithIO` is a free function rather than a method so callers need not
  construct a runner object. Generic type inference handles all three plugin
  types automatically.
- Tests default to `.sync` mode for determinism; a dedicated test
  (`testAsyncMode`, `testAsyncModeInvokesWaitingPlugin`) exercises async paths.
