# Changelog

All notable changes to this project will be documented in this file.

## [0.12.0] - 2026-06-17

### Added

- **General attributes — `attr()`, `attributes()`, `structure()` (R-16)** — R's
  open key→value metadata map. A new transparent wrapper
  `SValue::Attributed { attrs, inner }` stores the *general* (non-special)
  attributes as an insertion-ordered association list beside a boxed inner value.
  Like `Named`/`Classed` it is *see-through*: `length`, `type_name`, `class_of`,
  the coercions, `arithmetic`, `compare`, `truthy`, `index`, `assign_index`, and
  printing all delegate to `inner`; only the attribute builtins observe the map.
  - The three **special** attributes keep their dedicated representations and are
    *never* duplicated into the general map: `names` → `SValue::Named` (R-15),
    `class` → `SValue::Classed` (S v2), `dim` → `SValue::Matrix` (R-11). This
    makes the consistency invariant structural — `attr(x, "names")` reads the same
    field as `names(x)`, `attr(x, "class")` agrees with `class(x)`/`class_of`, and
    `attr(x, "dim")` agrees with the matrix `dim`.
  - `attr(x, which)` gets one attribute (or `NULL`); `attr(x, which) <- value`
    sets/replaces it (`NULL` removes), wired through R-15's replacement-function
    lvalue path, now **generalized** to thread extra call arguments through
    (`attr(x, "foo") <- v` desugars to ``x <- `attr<-`(x, "foo", value = v)``).
  - `attributes(x)` returns all attributes as a named list (special ones first in
    R's canonical `names`/`dim`/general order, `class` last) or `NULL`;
    `attributes(x) <- list(...)` replaces the whole set; `NULL` clears it.
  - `structure(x, ...)` (previously `class`-only) now routes *every* named
    argument through the same per-name logic, so `dim`, `names`, and arbitrary
    attributes all attach in one call. `.Names`/`.Dim` aliases are honoured.
  - `dim`/`nrow`/`ncol`/`levels` peel the new transparent wrappers (and `names`
    peels class/general but stops at the names wrapper) so a classed or
    generally-attributed matrix/factor still reports its shape/levels.

### Safety

- The general attribute map is bounded by `MAX_ATTRIBUTES` (4096) — `attr<-`,
  `attributes<-`, and `structure` refuse runaway growth from crafted input. A
  `"dim"` set validates the reshape with checked multiplication against
  `MAX_SEQ_LEN` before allocating. No `unwrap`/panic is reachable from malformed
  `attributes(x) <- …` input: a non-list `value`, an unnamed element, a too-long
  `names`, a non-integer `dim` component, or a non-conforming `dim` product all
  return a clean `SError`.

## [0.11.0] - 2026-06-17

### Added

- **Named vectors / the `names` attribute (R-15)** — a new transparent wrapper
  `SValue::Named { names, values }` carries a parallel `Vec<Option<String>>` of
  element names beside a boxed atomic value (`Double`/`Logical`/`Character`). Like
  `SValue::Classed` it is *see-through*: `length`, `type_name`, `class_of`, the
  coercions, `arithmetic`, `compare`, and `truthy` all delegate to the inner
  value, so names drop exactly where R drops them and survive where R keeps them.
  - `c(a = 1, b = 2)` attaches argument tags; nested named pieces combine R-style
    (`c(x = c(a = 1), 2)` → names `"x.a"`, `""`; `c(p = c(1, 2))` → `"p1"`, `"p2"`).
    `combine` builds names only when some argument is tagged or already named.
  - `names(x)` returns the character names (or `NULL`); the `names<-` replacement
    function sets them with R's NA-padding recycling (too-short pads with `NA`,
    too-long is an error, `NULL` clears), and `setNames(x, nm)` is the functional
    form. A general **replacement-function lvalue path** in the evaluator desugars
    `f(x) <- v` to ``x <- `f<-`(x, v)`` (reusable for future `levels<-`/`dim<-`).
  - **Character indexing** `x["b"]` / `x[c("a","c")]` selects by name (unmatched →
    `NA`); a `resolve_picks_named` helper extends `resolve_picks` with the
    by-name path. Positional / negative / logical indexing still work and now
    **carry the selected names along**. `assign_index` keeps names through
    `v[i] <- val` and resolves a character subscript by name.
  - **Printing** lays a named vector out R-style — right-aligned names above
    right-aligned values, each column the wider of the two — instead of the `[i]`
    prefix; an unset name prints `<NA>`.

### Safety

- The names vector is always kept exactly as long as the values (every
  constructor truncates / `NA`-pads via `with_names`), so a name lookup can never
  index out of bounds; the by-name index path reuses the bounded `resolve_picks`.
  No new unbounded allocation or integer-overflow surface — name lengths ride the
  same vector-length channels already capped at `MAX_SEQ_LEN`.

## [0.10.0] - 2026-06-16

### Added

- **Index sub-assignment (R-14)** — the left side of an assignment may now be a
  subscript expression, not just a bare name. `eval_indexed_assignment` descends
  to the postfix `[ … ]`, requires a bare-name base, looks up its current value,
  resolves the subscripts, writes the (recycled) RHS into the selected cells, and
  rebinds the modified value to the base name. Supported: 1-D `v[i] <- val`,
  `v[-i] <- val`, `v[logical] <- val` (full `resolve_picks` styles), and 2-D
  `m[i, j] <- v`, `m[i, ] <- v`, `m[, j] <- v`, `m[rows, cols] <- v`. New free
  functions `assign_index` / `assign_index2d` (with `assign_positions` /
  `write_recycled` helpers) in `value.rs`.

### Safety

- Sub-assignment is **copy-on-modify**: the base value is cloned, mutated, and
  re-`define`d, so a prior copy (`b <- a; a[1] <- 9`) is never aliased and the
  rebind cannot corrupt any other binding. An out-of-range or `NA` index in an
  assignment is a hard error (no silent auto-grow); an empty replacement
  (`v[i] <- c()`) is an error; assigning into an undefined base is an error.
  Write counts are bounded by the (`MAX_SEQ_LEN`-capped) selection length.

## [0.9.0] - 2026-06-16

### Added

- **2-D matrix indexing (R-13)** — the `[` subscript operator extended to two
  dimensions. An `index_suffix` grammar change makes each comma-separated
  subscript optional, so `m[i, j]`, `m[i, ]` (whole row), `m[, j]` (whole
  column), and `m[rows, cols]` (sub-matrix) all work on `SValue::Matrix`
  (1-based, column-major). A 2-D result follows R's `drop = TRUE` (a single
  row/column collapses to a vector). `m[i]` indexes the flat column-major
  vector. The empty-subscript grammar also enables `df[, j]` / `df[i, ]`.

### Changed

- **Index resolution now supports all three R styles** (`resolve_picks`):
  **positive** (1-based; `0` drops, out-of-range/`NA` → `NA`), **negative**
  (`-k` excludes; cannot mix with positive), and **logical** (a mask recycled to
  the dimension). This fixes 1-D vector indexing — `v[-2]` and
  `v[c(TRUE, FALSE)]` now behave correctly (logical indices were previously
  mis-coerced to numbers). `dataframe::index2d` now takes optional (`None` =
  whole-dimension) subscripts.

### Security

- Out-of-range matrix subscripts are a hard error; the logical-recycle span and
  the 2-D result size are capped at `MAX_SEQ_LEN`, so negative/logical index
  expansion cannot be turned into unbounded allocation.

## [0.8.0] - 2026-06-16

### Added

- **Matrix linear algebra (R-12)** — builtins operating on the R-11
  `SValue::Matrix`: `diag()` (R's extract-diagonal / build-diagonal /
  make-identity overload, with `nrow`/`ncol`); the margin reductions
  `rowSums`/`colSums`/`rowMeans`/`colMeans` (with an `na.rm` option; an all-`NA`
  mean is `NaN`); `cbind()`/`rbind()` (bind vectors and matrices by column/row,
  recycling vectors, erroring on a mismatched matrix dimension, `NULL` for the
  empty call); and `solve()`/`det()` (matrix inverse, linear solve `a %*% x = b`,
  and determinant) via Gaussian elimination with partial pivoting — no LU
  primitive exists in the substrate, so it is implemented directly. A singular
  matrix is a clean error (`det` returns `0`); `NA` makes `det` return `NA` and
  `solve` an error; the `solve`/`det` order is capped at `MAX_SOLVE_DIM` (1000)
  so the `O(n³)` work cannot become a denial-of-service, and all construction is
  bounded by `MAX_SEQ_LEN`.

## [0.7.0] - 2026-06-16

### Added

- **Matrix type (R-11)** — a new `SValue::Matrix { data, nrow, ncol }` (numeric,
  column-major, implicit class `"matrix"`), the `%*%` matrix-multiply operator
  (a new arm in the `%op%` infix dispatch — a bare vector is a row on the left,
  a column on the right, so `v %*% w` is the dot product; NA propagates), and the
  builtins `matrix(data, nrow =, ncol =, byrow =)`, `t()`, and
  `apply(X, MARGIN, FUN, …)` (over rows/columns, simplifying to a vector or
  matrix). `dim()`/`nrow()`/`ncol()` extended for matrices. Matrices coerce to
  their flat vector and print with R's `[,j]`/`[i,]` console layout. The product
  size and all loops are capped at `MAX_SEQ_LEN`.

## [0.6.0] - 2026-06-16

### Added

- **Higher-order functionals (R-10)** — `Map`, `mapply`, `Reduce`, `Filter`,
  `vapply`, pairing with the R-9 `\(x)` lambdas. `Map(f, …)`/`mapply(f, …)` zip
  several sequences element-wise (recycling to the longest; Map → list, mapply →
  vector); `Reduce(f, x[, init])` left-folds; `Filter(f, x)` keeps elements where
  `f` is true (preserving list-vs-vector); `vapply(x, f, template)` is `sapply`
  with a per-element result-shape check. They invoke the function via
  `Interpreter::call_value`, taking it by name (`f =`/`FUN =`) or as the first
  callable positional, so they compose with the `|>` pipe.

## [0.5.0] - 2026-06-16

### Added

- **The native pipe `|>` (R-9)** — `eval_pipe` desugars `lhs |> f(a)` to
  `f(lhs, a)`, inserting the piped value as the first positional argument of the
  right-hand call, left-associatively (`x |> f() |> g()` is `g(f(x))`). The
  right-hand side must be a function call; a bare `x |> f` is an error. (The
  `|>`/`pipe` syntax is R-only; `s.grammar` is unchanged, so S never produces a
  `pipe` node.) The backslash lambda `\(x) …` reuses the existing `func_def`
  evaluation unchanged.

## [0.4.0] - 2026-06-16

### Added

- **Discrete distribution family (R-8b)** — the `d`/`p`/`q`/`r` functions for the
  discrete distributions, wired to `statistics-core`:
  - **Binomial**: `dbinom`, `pbinom`, `qbinom`, `rbinom` (parameters `size`,
    `prob`).
  - **Poisson**: `dpois`, `ppois`, `qpois`, `rpois` (parameter `lambda`).

  Same vectorized `d`/`p`/`q` (NA-propagating), named/positional parameters, and
  reproducible per-session RNG as the continuous families (R-8).

### Security

- The discrete CDFs and inverse-CDF samplers loop over an integer count
  (`pbinom` is O(`size`), `ppois` is O(`x`), `rbinom` is O(n·`size`)). Two guards
  bound every loop: a per-element cap (`MAX_DISCRETE_SUPPORT` ≈ 1M on `size` and
  the `ppois` quantile) and a total-iteration budget (`MAX_DISCRETE_WORK` ≈ 134M
  over `len·driver` / `n·per-sample`). A crafted `rbinom(1e6, 1e6, …)` or
  `ppois(1e18, …)` is a clean error, not an unbounded loop.

## [0.3.0] - 2026-06-16

### Added

- **Distribution family (R-8)** — the `d`/`p`/`q`/`r` probability functions
  wired to `statistics-core`, for the closed-form continuous distributions:
  - **Normal**: `dnorm`, `pnorm`, `qnorm`, `rnorm` (defaults `mean = 0`,
    `sd = 1`).
  - **Uniform**: `dunif`, `punif`, `qunif`, `runif` (`min = 0`, `max = 1`).
  - **Exponential**: `dexp`, `pexp`, `qexp`, `rexp` (`rate = 1`).
  - `set.seed(n)` to make the `r*` sampling stream reproducible.

  `d*`/`p*`/`q*` vectorize over their first argument with NA-propagation;
  distribution parameters are read by name (`sd =`) or position. `r*` draws from
  a per-session R-compatible MT19937 generator; the sample count is capped at
  `MAX_SEQ_LEN`, so `rnorm(1e18)` is a clean error rather than an OOM abort.

### Changed

- The `Interpreter` now carries a `RefCell<RngState>` (the session RNG) that
  `set.seed` reseeds and the `r*` builtins draw from. `set.seed` returns
  invisibly, alongside `print`/`cat`.

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
