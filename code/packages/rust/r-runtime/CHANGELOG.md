# Changelog

All notable changes to this project will be documented in this file.

## [0.9.0] - 2026-06-16

### Added (via the shared `s-runtime` lvalue machinery)

- **R-14 — index sub-assignment**: `m[i, j] <- v`, `m[i, ] <- v`, `m[, j] <- v`,
  `m[rows, cols] <- v`, and 1-D `v[i] <- val`, `v[-i] <- val`,
  `v[logical] <- val`. The RHS recycles R-style; the matrix keeps its `dim`.
  Sub-assignment is copy-on-modify (a prior `b <- a` copy is unaffected); an
  out-of-range/`NA` index, an empty replacement, or an undefined base are clean
  errors. This completes the R-13 deferral. See `s-runtime` 0.10.0.

## [0.8.0] - 2026-06-16

### Added (via the shared `s-runtime` + the `r.grammar` index_suffix change)

- **R-13 — 2-D matrix indexing**: `m[i, j]`, `m[i, ]`, `m[, j]`, `m[rows, cols]`
  (drop-to-vector on a single row/column), `m[i]` linear indexing, and full
  positive / negative / logical subscript support (which also fixes 1-D vector
  negative/logical indexing). Sub-assignment `m[i, j] <- v` is deferred to R-14.
  See `s-runtime` 0.9.0.

## [0.7.0] - 2026-06-16

### Added (via the shared `s-runtime`)

- **R-12 — matrix linear algebra**: `diag()`, `rowSums`/`colSums`/`rowMeans`/
  `colMeans` (with `na.rm`), `cbind()`/`rbind()`, and `solve()`/`det()`. See
  `s-runtime` 0.8.0.

## [0.6.0] - 2026-06-16

### Added (via the shared `s-runtime`)

- **R-11 — the matrix type**: `matrix()`, `%*%`, `t()`, `dim`/`nrow`/`ncol`, and
  `apply()`. See `s-runtime` 0.7.0.

## [0.5.0] - 2026-06-16

### Added (via the shared `s-runtime`)

- **R-10 — higher-order functionals**: `Map`/`mapply`/`Reduce`/`Filter`/`vapply`,
  composing with the R-9 lambdas and pipe. See `s-runtime` 0.6.0.

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
