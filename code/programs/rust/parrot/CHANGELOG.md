# Changelog — parrot (Rust)

All notable changes to this program are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [1.0.0] — 2026-04-06

### Added

- Initial release of the Parrot REPL program in Rust.
- `src/prompt.rs` — `ParrotPrompt` struct implementing `repl::Prompt` with
  parrot-themed prompts and the 🦜 emoji.
- `src/lib.rs` — library target re-exporting `prompt::ParrotPrompt` so
  integration tests in `tests/` can access it.
- `src/main.rs` — binary entry point wiring `repl::EchoLanguage`,
  `ParrotPrompt`, and `repl::SilentWaiting` via `repl::runner::run_with_io`.
  Prints a welcome banner on startup and a goodbye message on exit.
- `tests/parrot_test.rs` — 17 integration tests covering:
  - Basic echo
  - Quit termination
  - Multiple inputs echoed in order
  - Sync and async mode parity
  - GlobalPrompt and LinePrompt content (emoji, cursor, trailing space)
  - EOF graceful exit
  - Empty string echo
  - Input with internal spaces preserved verbatim
  - Output ordering in sync mode
  - Only :quit produces no echoed output
- `Cargo.toml` — standalone `[workspace]` declaration preventing inclusion
  in the `code/packages/rust/` workspace; `[lib]` and `[[bin]]` targets.
- `BUILD` — build and test script for the build tool.
