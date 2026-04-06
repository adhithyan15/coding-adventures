# Changelog

All notable changes to this program will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added
- `main.lua` — Parrot REPL program demonstrating the `coding_adventures.repl`
  framework with a custom `ParrotPrompt` and parrot-themed prompts
- `tests/test_parrot.lua` — 13 busted tests covering `ParrotPrompt` unit
  behaviour and full REPL loop integration with injected I/O
- `BUILD` — runs the busted test suite
- `BUILD_windows` — skips gracefully on Windows CI
- `README.md` — usage, architecture, and how the program fits in the stack
