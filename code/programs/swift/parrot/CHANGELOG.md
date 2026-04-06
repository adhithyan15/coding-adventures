# Changelog

All notable changes to the Parrot Swift program will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-06

### Added

- `ParrotPrompt` struct — implements `Prompt` from `CodingAdventuresRepl`.
  - `globalPrompt()` returns a two-line parrot-themed banner with 🦜 emoji
    and a trailing blank line.
  - `linePrompt()` returns `"🦜 > "` with a trailing space so the user's
    cursor appears clearly after the prompt.
- `main.swift` — wires `EchoLanguage`, `ParrotPrompt`, and `SilentWaiting`
  into `runWithIO` with real stdin/stdout I/O (`readLine()` / `print`).
- 15 `XCTest` test cases covering:
  - Banner contains "Parrot"
  - User input is echoed back
  - `:quit` produces "Goodbye!" and ends the session
  - Multiple sequential echoes
  - Sync mode correct output
  - Line prompt contains 🦜 emoji
  - Global prompt mentions `:quit`
  - EOF (nil input) exits gracefully without crash
  - Line prompt printed before each input
  - Banner printed exactly once
  - `ParrotPrompt` returns non-empty strings
  - No output produced after `:quit`
  - Async and sync modes produce equivalent observable output
  - Empty string echoed back
- `Package.swift` using swift-tools-version 5.9, with a local path dependency
  on `../../../packages/swift/repl`.
- `BUILD` and `BUILD_windows` files for the monorepo build tool.
- Literate-programming-style comments throughout explaining all design
  decisions.

### Design Notes

- `ParrotPrompt` is `public` because the test target uses `@testable import Parrot`
  and tests reference `ParrotPrompt` directly. Public visibility ensures the
  symbol is accessible from the test target even without `@testable`.
- `main.swift` uses `print($0, terminator: "")` rather than `print($0)`
  because the prompt and banner strings already include their own newlines.
  Adding another newline would produce blank lines between prompts.
- `SilentWaiting` is used rather than a custom waiting plugin because
  `EchoLanguage` returns instantly. A spinner that flashes for < 1 ms would
  be more noise than signal.
