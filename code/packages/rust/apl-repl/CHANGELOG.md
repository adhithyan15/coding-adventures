# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial release — the interactive REPL and the **`apl` binary** (item
  MA-4e), a sibling of `matlab-repl`/`s-repl`/`r-repl` over the
  `apl-runtime` interpreter.
- `AplRepl` with a persistent workspace, `>> `/`... ` prompts, and `quit`/
  `exit`/EOF handling; `run()` drives a full stdio session.
- **Line continuation across an open `(`** — the only grouping construct in
  this language cut (no block keywords, no string type). Continuation lines
  are joined with a space rather than a real newline, since `apl.tokens`
  does not drop newlines inside `(...)` in this first cut.
- Errors are surfaced (`Error: …`) without ending the session.
