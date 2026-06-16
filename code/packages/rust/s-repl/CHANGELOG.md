# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-06-15

### Added

- Initial release of the historical Bell Labs S REPL crate.
- `SRepl` interactive session driver: `feed()` one line at a time, with
  `prompt()` returning `> ` (fresh) or `+ ` (continuing).
- Statement continuation across unbalanced `(`/`[`/`{` and open string
  literals.
- Auto-print of visible top-level results in S's `[i]`-prefixed vector layout;
  assignments and loops are invisible.
- `print()` output surfaced ahead of the auto-printed result.
- Recoverable error reporting (`Error: ...`); the session continues afterward.
- Quit words `q()`, `quit()`, `:quit`, plus EOF.
- The `s` command-line binary wrapping `SRepl` over stdin/stdout.
