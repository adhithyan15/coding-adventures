# Changelog

All notable changes to this project will be documented in this file.

## [0.13.0] - 2026-06-19

### Added (via the shared `s-runtime`)

- **R-18 — `switch()` + error handling**: the value-returning multi-way branch
  and condition-based error handling reach R unchanged through the shared
  evaluator. **`switch(EXPR, ...)`** is lazy (only the chosen arm evaluates):
  character `EXPR` matches arm names (unnamed final arm = default; no match and
  no default → `NULL`); numeric `EXPR` selects the n-th arm by position (out of
  range → `NULL`) — so `switch("a", a = "ok", b = stop("x"))` does not raise.
  **`stop(...)`** raises an error (concatenated message); **`warning(...)`** emits
  a warning and returns invisibly without aborting; **`tryCatch(expr, error = fn,
  finally = cleanup)`** runs `expr`, routes any error to `error` (called with a
  minimal condition object so `conditionMessage(e)` and `e$message` give the
  message), and always runs `finally`. *(Empty-arm fall-through is deferred to
  R-19 — it needs a grammar production for empty args.)* See `s-runtime` 0.14.0.

## [0.12.0] - 2026-06-17

### Added (via the shared `s-runtime`)

- **R-17 — `do.call`, named-list access polish, `modifyList`**:
  `do.call(what, args)` builds and evaluates a call to `what` (a function value,
  or a string naming one) with the elements of the list `args` spread as
  arguments — unnamed positional, named by name — so
  `do.call(paste, list("a", "b", sep = "-"))` is `"a-b"`. `modifyList(x, val)`
  overlays `val`'s elements onto `x` by name (replace / append / `NULL` removes).
  The R-6 named-list access operators are pinned: `lst$name` / `lst[["name"]]` by
  name, `lst[[i]]` by position, a missing name → `NULL` (not an error), and they
  see through classed / attribute-carrying list wrappers. Both builtins are
  bounded against crafted oversize inputs and return clean errors (never panics)
  on a non-list, non-callable, or unnamed-element argument. See `s-runtime`
  0.13.0.

## [0.11.0] - 2026-06-17

### Added (via the shared `s-runtime`)

- **R-16 — general attributes**: `attr(x, which)` gets a named attribute (`NULL`
  if absent); `attr(x, which) <- value` sets/replaces it (`NULL` removes);
  `attributes(x)` gets all attributes as a named list (`NULL` if none);
  `attributes(x) <- list(...)` replaces them (`NULL` clears); `structure(x, ...)`
  returns `x` with the `...` named args attached as attributes. The special
  attributes stay consistent with their dedicated wrappers — `attr(x, "names")`
  == `names(x)` (R-15), `attr(x, "class")` == `class(x)` (S3), `attr(x, "dim")`
  == `dim(x)` (R-11) — even after layering a class on a matrix, and setting `dim`
  via `attr<-` reshapes a vector into a matrix. `attr<-`/`attributes<-` slot into
  R-15's replacement-function lvalue path (now generalized to thread the `which`
  argument through). See `s-runtime` 0.12.0.

## [0.10.0] - 2026-06-17

### Added (via the shared `s-runtime`)

- **R-15 — `names()` and named-vector access**: `c(a = 1, b = 2)` attaches names
  (nested named pieces combine R-style — `c(x = c(a = 1), 2)` → `"x.a"`, `""`);
  `names(x)` gets them (`NULL` if unset); `names(x) <- value` sets them with R's
  NA-padding recycling (`NULL` clears); `setNames(x, nm)` is the functional form;
  `x["b"]` / `x[c("a","c")]` index by name (unmatched → `NA`). Positional /
  negative / logical indexing still work and carry names along; sub-assignment
  (`x[2] <- 9`, `x["a"] <- 5`) keeps names; arithmetic drops them, as in R. Named
  vectors print names above values in aligned columns instead of the `[i]` prefix
  — a user-visible change in the R REPL. See `s-runtime` 0.11.0.

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
