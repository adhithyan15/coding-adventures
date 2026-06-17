# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-06-16

### Added

- Initial release of the R REPL crate and the `R` binary — item R-3 of the R
  frontend, completing a working R REPL.
- `RRepl` interactive session driver (`feed`, `prompt`, `is_incomplete`,
  `is_continuing`) and the generic `run<R: BufRead, W: Write>` driver, mirroring
  `s-repl`. Statement continuation across unbalanced `(`/`[`/`{` and open
  strings; auto-print of visible results (surfaced from the runtime's shared S3
  `print` generic); recoverable error reporting; quit words and EOF.
- The `R` command-line binary wrapping `RRepl` over stdin/stdout.
- 8 tests including scripted `run()` sessions and the `=`-assignment case.
