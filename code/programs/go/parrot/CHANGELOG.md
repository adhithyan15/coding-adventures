# Changelog — parrot (Go)

All notable changes to this program are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [1.0.0] — 2026-04-06

### Added

- Initial release of the Parrot REPL program.
- `ParrotPrompt` struct implementing `repl.Prompt` with parrot-themed prompts
  and the 🦜 emoji.
- `main.go` wiring `repl.EchoLanguage`, `ParrotPrompt`, and `repl.SilentWaiting`
  together via `repl.RunWithIO`.
- Welcome banner printed once at startup (separate from the per-line prompt).
- Goodbye message printed after the loop exits.
- `prompt.go` — separate file so `ParrotPrompt` is accessible from
  `package main_test` tests.
- 17 unit/integration tests covering:
  - Basic echo
  - Quit termination
  - Multiple inputs
  - Sync and async mode parity
  - Banner and prompt content
  - EOF graceful exit
  - Empty string echo
  - Output order preservation
  - Input with spaces preserved verbatim
- `BUILD` and `BUILD_windows` scripts for the build tool.
- `go.mod` with `replace` directive pointing to the local repl package.
