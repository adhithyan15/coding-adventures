# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-06-16

### Added

- Initial release — the interactive REPL and the **`matlab` binary** (item
  MA-3d), a sibling of `s-repl`/`r-repl` over the `matlab-runtime` interpreter.
- `MatlabRepl` with a persistent workspace, `>> `/`... ` prompts, and `quit`/
  `exit`/EOF handling; `run()` drives a full stdio session.
- **Line continuation** across open brackets *and* unterminated block keywords
  (`if`/`for`/`while`/`switch`/`try`/`function`), counting `end` as a block
  closer only at bracket depth 0 (so `A(end)` does not open a block). `%` line
  comments and `"`-strings are skipped.
- Errors are surfaced (`Error: …`) without ending the session.
