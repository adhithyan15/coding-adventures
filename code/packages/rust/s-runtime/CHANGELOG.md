# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-06-15

### Added

- **Infix `%op%` operators**: built-in `%%` (modulo), `%/%` (floor division),
  `%in%` (membership), `%o%` (outer product), and user-defined `%name%`.
- **Builtin library**: vectorized math (`abs sqrt exp log log10 floor ceiling
  round sin cos tan`); utilities (`rev sort order rep unique which any all is.na
  cumsum cumprod paste paste0`); the apply-family `sapply`.
- **S3 method dispatch**: `class`, `structure`, `inherits`, `unclass`, `cat`,
  and a generic `print` that dispatches to `print.<class>` (used by the REPL's
  auto-print). A `Classed` value is transparent to arithmetic/coercion.
- **Factors**: `factor`, `levels`, `nlevels`, `as.character`, `as.integer`.
- **Data frames**: `data.frame`, `$` / `[[ ]]` / 2-D `df[i, j]` access,
  `nrow`, `ncol`, `names`, `colnames`, `dim`, `head`, and table printing.

### Changed

- Built-ins now receive an `Interpreter` handle (`fn(&Interpreter, &[Arg])`),
  enabling `sapply` and S3 dispatch to call back into user functions.
- **Operator-precedence fix**: `:` now binds tighter than `+ - * /` (matching
  R), so `1:3+1` is `c(2, 3, 4)`. A new `%op%` precedence level sits between
  `* /` and `:`.

## [0.1.0] - 2026-06-15

### Added

- Initial release of the historical Bell Labs S tree-walking evaluator.
- `Interpreter` / `eval_s` / `Outcome`; the `SValue` model (double, logical,
  character, NULL, closures, built-ins).
- Everything-is-a-vector semantics: recycling, NA propagation, the
  `logical < double < character` coercion lattice.
- `<- / _ / ->` assignment, `c()`, the `:` sequence operator, positive-integer
  indexing, lexical-scope closures with named/default arguments, `if`/`for`/
  `while`/`repeat` as expressions, and result visibility.
- Built-ins `c`, `length`, `print`, `seq`, and the statistics reductions
  (`mean sum sd var median min max prod`) over `statistics-core`.
- Resource guards: bounded `:`/`seq()` allocation and a recursion-depth limit.
