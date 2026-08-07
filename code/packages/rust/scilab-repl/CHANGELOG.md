# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-07-20

### Added

- Initial release — the interactive REPL and the **`scilab` binary** (item
  MA-10d), a sibling of `matlab-repl`/`maple-repl` over the `scilab-runtime`
  interpreter.
- `ScilabRepl` with a persistent workspace, `-->`/`> ` prompts, and
  `quit`/`exit`/EOF handling; `run()` drives a full stdio session.
- **Line continuation** across open brackets and an unterminated
  `if`/`select`/`while`/`for` block (until `end`) or `function` (until
  `endfunction`) — simpler than `matlab-repl`'s own tracker, since Scilab's
  `$` last-index token is never a context-sensitive bare word the way
  MATLAB's `end` is. `//` line comments and `"`-strings are skipped; `/* ...
  */` block comments are tracked across multiple physical lines.
- `read_bounded_line`, carried forward from `maple-repl`: bounds a single
  physical line to 64 KiB before `ScilabRepl::feed`'s own input-size guard
  ever runs.
- Errors are surfaced (`Error: …`) without ending the session.
