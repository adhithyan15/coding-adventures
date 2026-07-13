# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Initial release — the interactive REPL and the **`j` binary** (item
  MA-6d), a sibling of `apl-repl`/`matlab-repl`/`s-repl`/`r-repl` over the
  `j-runtime` interpreter.
- `JRepl` with a persistent workspace, `>> `/`... ` prompts, and `quit`/
  `exit`/EOF handling; `run()` drives a full stdio session.
- **Line continuation across an open `(`** — the only grouping construct in
  this language cut (no block keywords, no string type). Continuation lines
  are joined with a space rather than a real newline, since `j.tokens` does
  not drop newlines inside `(...)` in this first cut — mirrors `apl-repl`'s
  own identical scanner design.
- `NB.` line comments need zero REPL-level handling — they're stripped
  entirely at the lexer's skip-pattern level, exactly like APL's `⍝`.
- Errors are surfaced (`Error: …`) without ending the session.
- `MAX_CONTINUATION_BUFFER` (64 KiB) caps the pending-continuation buffer
  while a `(` is still unbalanced, discarding it with a clean error rather
  than growing without bound — ported forward from `apl-repl`'s own
  already-security-reviewed guard (same rationale: low severity under this
  crate's stdio-only threat model, kept anyway for defense in depth).
