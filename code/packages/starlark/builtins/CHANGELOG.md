# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-22

### Added

- **`cmd.star`**: Structured command builders for OS-aware BUILD rules.
  - `cmd(program, args)` — universal command dict builder.
  - `cmd_windows(program, args)` — Windows-only command (returns `None` on other platforms).
  - `cmd_linux(program, args)` — Linux-only command.
  - `cmd_macos(program, args)` — macOS-only command.
  - `cmd_unix(program, args)` — runs on any Unix platform (not Windows).
  - `filter_commands(cmds)` — strips `None` entries from command lists.
- Reads `_ctx["os"]` at module load time (injected by the build tool via `WithGlobals()`).
- Captures platform value once at module level for use in all functions.
