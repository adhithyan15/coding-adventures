# Changelog

All notable changes to this project will be documented in this file.

## [0.4.0] - 2026-06-16

### Added (R-9)

- The **native pipe `|>`** (`x |> f()` is `f(x)`, left-associative chains) and
  the **backslash lambda `\(x) x + 1`** (shorthand for `function(x) x + 1`),
  reaching R via the new r-lexer tokens, r-parser rules, and the shared
  evaluator's `eval_pipe`. See `s-runtime` 0.5.0.

## [0.3.0] - 2026-06-16

### Added (via the shared `s-runtime`)

- **R-8b — discrete distribution families**: `dbinom`/`pbinom`/`qbinom`/`rbinom`
  and `dpois`/`ppois`/`qpois`/`rpois`, with DoS-bounded count loops. R picks
  these up unchanged from the shared evaluator. See `s-runtime` 0.4.0.

## [0.2.0] - 2026-06-16

### Added (via the shared `s-runtime`)

- **R-8 — the `d`/`p`/`q`/`r` distribution family**: `dnorm`/`pnorm`/`qnorm`/
  `rnorm`, `dunif`/`punif`/`qunif`/`runif`, `dexp`/`pexp`/`qexp`/`rexp`, and
  `set.seed`. R picks these up unchanged from the shared evaluator. See
  `s-runtime` 0.3.0 for the full notes.

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
