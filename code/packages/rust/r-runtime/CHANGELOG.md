# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-06-16

### Added

- Initial release of the R runtime — item R-3 of the R frontend.
- `RInterpreter` and `eval_r()`: parse R with `r-parser` and evaluate via the
  shared `s-runtime` tree-walker, reusing the entire S evaluator (value model,
  recycling, NA semantics, S3 dispatch, factors, data frames, all built-ins).
- Re-exports `SValue`, `SError` (also as `RError`), `SResult`, `Outcome`, and
  `format_value` from `s-runtime`.
- R-specific surface handled in the shared evaluator: `=` / `->>` assignment and
  the typed-`NA` constants (`NA_integer_`, `NA_real_`, `NA_character_`).
- 13 tests: the canonical R session (`data_frame <- c(1,2,3); mean(data_frame)`),
  recycling, `=`/`->>` assignment, the inherited precedence fix, NA handling,
  closures + `sapply`, infix operators, data frames + `$`, persistence, errors.

### Changed (in the shared `s-runtime`)

- Factored `Interpreter::eval_str` into a public, parser-agnostic
  `eval_program(&GrammarASTNode)` so a different front end (R) can feed its own
  parse tree. `eval_str` is unchanged for S callers.
