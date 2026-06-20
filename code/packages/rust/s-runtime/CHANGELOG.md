# Changelog

All notable changes to this project will be documented in this file.

## [0.16.0] - 2026-06-20

### Added

- **Functional helpers (R-20)** — the remaining members of R's
  functional-programming toolkit, built on the R-10 family (`Map`/`Reduce`/
  `Filter`/`mapply`/`vapply`) and, like it, plain builtins in the shared runtime
  (no grammar change). They take the function by `f =`/`FUN =` or the first
  callable positional (the R-10 `split_fun` helper), so they compose with the
  pipe (`1:5 |> Find(f = \(x) x > 2)`).
  - **`Find(f, x)`** — the first element of `x` where `f(element)` is `TRUE`, or
    an invisible `NULL` if none. Short-circuits on the first hit.
  - **`Position(f, x)`** — the 1-based index of the first matching element
    (`Find` returns the value, `Position` the index); `NULL` if none.
  - **`Negate(f)`** — a new callable computing `!f(...)`. Implemented as a small
    dedicated value, `SValue::Negated(Box<SValue>)`, recognized by the call
    dispatcher (`apply`/`call_value`): it invokes the wrapped `f` through the
    normal depth-bounded path and logically negates the verdict (new `logical_not`
    helper — `NA`-preserving, like `!`). `Negate(is.na)(NA)` → `FALSE`. The
    wrapper is a function for `class`/`type_name`/`length`/`is_callable`. A
    non-callable `f` is a clean `NotCallable` error.
  - **`Reduce(f, x, ..., accumulate = TRUE)`** — extends the R-10 left fold to
    return the vector/list of *running* folds; with an `init`, the init is the
    first accumulated element (`Reduce(\(a, b) a + b, 1:3, 10, accumulate=TRUE)`
    → `c(10, 11, 13, 16)`). Built with the shared `combine`/`c()` engine; the
    no-`accumulate` behaviour is unchanged. Bounded by `MAX_SEQ_LEN` (the
    accumulator can be no longer than `x`, itself capped).
  - **`Recall(...)`** — anonymous recursion: re-invokes the enclosing function.
    The interpreter now keeps a small call stack of the currently-executing
    closures (pushed/popped by `call_closure` via an RAII `CallFrameGuard`, so it
    is exception-safe); `Recall` reads the top. Outside any function it is an
    error. Recursion is bounded by `MAX_EVAL_DEPTH`, so runaway anonymous
    recursion fails cleanly instead of overflowing the native stack.

## [0.15.0] - 2026-06-19

### Added

- **Empty-arm `switch()` fall-through (R-19)** — `switch("a", a = , b = "hit")`
  now returns `"hit"`. R-18 already implemented the fall-through in `eval_switch`
  (an empty arm has `arm_body() == None`, so the loop over `arms[pos..]` skips to
  the next non-empty value), but it was inert because the shared S/R grammar's
  `arg = NAME EQ expr` rejected an empty value. R-19 extends the grammar to
  `arg = NAME EQ [expr] | expr` (see `s-parser`/`r-parser` 0.3.0) and regenerates
  both compiled `_grammar.rs`, so `a = ,` finally parses. The fall-through chains
  across several empty arms (`a = , b = , c = "z"` → `"z"`); a matched empty arm
  with nothing non-empty after it yields invisible `NULL`.

### Note

- The empty named-argument value parses everywhere an arg list appears, but is
  only **meaningful** in `switch`. In an ordinary call (`c(x = )`) it surfaces as
  an eval-time parse-style error via `eval_arg`'s `only_node` (no panic),
  matching R's "argument is missing" behaviour.

## [0.14.0] - 2026-06-19

### Added

- **`switch()` + error handling (R-18)** — the value-returning multi-way branch
  and condition-based error handling, in the shared runtime so R and S get them.
  - **`switch(EXPR, ...)`** is a **special form**, intercepted in `eval_postfix`
    before argument evaluation so it sees the *unevaluated* arm expressions and
    evaluates only the chosen one. A character `EXPR` matches arm names (an
    unnamed final arm is the default; no match and no default → invisible `NULL`);
    a numeric `EXPR` selects the n-th arm by position (out of range → `NULL`).
    Because only the selected arm runs, `switch("a", a = "ok", b = stop("x"))`
    does not raise. *(Empty-arm fall-through `switch("a", a = , b = "hit")` is
    implemented in `eval_switch` but deferred to R-19 — the shared S/R grammar's
    `arg = NAME EQ expr` has no empty-value production, so `a = ,` is a parse
    error today.)*
  - **`stop(...)`** raises a new typed error variant `SError::User` whose message
    is the concatenation of its arguments (catchable by `tryCatch`).
  - **`warning(...)`** records and prints a warning (bounded by `MAX_WARNINGS`)
    and returns invisibly without aborting.
  - **`tryCatch(expr, error = handler, finally = cleanup)`** is a **special form**
    (lazy): it evaluates `expr`, routes any catchable error to the `error` handler
    (called with a minimal condition object `list(message, call)` classed
    `c("simpleError", "error", "condition")`, returning the handler's value), and
    always runs `finally`. Loop-control signals (`break`/`next`) are **not** caught
    (`SError::is_catchable`).
  - **`conditionMessage(e)`** / `e$message` recover the condition's message.

## [0.13.0] - 2026-06-17

### Added

- **`do.call`, `modifyList`, named-list access polish (R-17)** — reflective call
  + list-overlay builtins, both in the shared runtime so R and S get them.
  - **`do.call(what, args)`** builds and evaluates a call to `what` with the
    elements of the list `args` spread as arguments — unnamed elements positional,
    named elements passed by name, in order. `what` is a callable value, or a
    length-one character string naming one (resolved in the global environment).
    Reuses `Interpreter::call_value` (the same path `lapply`/`Reduce`/`Map` use),
    so default arguments, named/positional matching, recycling, and visibility are
    identical to a direct call — `do.call(paste, list("a", "b", sep = "-"))` is
    `paste("a", "b", sep = "-")` → `"a-b"`.
  - **`modifyList(x, val)`** returns `x` with `val`'s elements overlaid by name:
    a name in both is replaced in place, a name only in `val` is appended, and a
    `val` element whose value is `NULL` removes that name (R's deletion
    semantics). Order follows `x` (removals dropped), then `val`'s new names.
  - **Named-list access polish.** A new wrapper-transparent `as_list` helper lets
    `$` / `[[name]]` / `[[i]]` (and both new builtins) see through
    `Classed`/`Attributed`/`Named` list wrappers, so a classed or
    attribute-carrying list still indexes by name. The R-6 contract is pinned with
    tests: `lst$name` / `lst[["name"]]` by name, `lst[[i]]` by position, a missing
    name → `NULL` (not an error). Data frames keep their stricter
    "undefined column" error — only the `list` type returns `NULL`.

### Safety

- `do.call`'s spread argument count and `modifyList`'s result size are both
  bounded by `MAX_DOCALL_ARGS` (100 000) against a crafted multi-million-element
  `args`/`val`. A non-list `args`/`val` (with `NULL` treated as the empty list for
  `do.call`), a non-callable `what`, an unknown function name, and an unnamed
  `val` element all return a clean `SError` — no `unwrap`/panic is reachable from
  malformed input.

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

### Changed

- **`%*%` adopts the shared f64 matrix-execution substrate (MXF-4)** — R's
  matrix product, formerly a hand-written `f64` triple loop in `eval.rs`, now
  routes through [`array_runtime::execute`]`(Kernel::MatMul, …)` at
  `DType::F64`. R's matmul therefore flows through the same cost-based CPU/GPU
  planner as MATLAB's `A * B`, at full double precision (MXF-3's bit-exact
  8-byte `f64` path) — completing the MX12 `f64`-substrate rollout. R's `Matrix`
  is already column-major `[nrow, ncol]`, identical to `array_runtime::Array`'s
  layout, and `matrix-cpu`'s `matmul_f64` folds the contraction left-to-right
  from `0.0` just as the old loop did, so results are **bit-identical**. Adds
  `array-runtime` as a dependency (it pulls `matrix-ir`/`matrix-cpu`/
  `matrix-runtime`/`executor-protocol`/`compute-ir` transitively).

### Safety

- The names vector is always kept exactly as long as the values (every
  constructor truncates / `NA`-pads via `with_names`), so a name lookup can never
  index out of bounds; the by-name index path reuses the bounded `resolve_picks`.
  No new unbounded allocation or integer-overflow surface — name lengths ride the
  same vector-length channels already capped at `MAX_SEQ_LEN`.
- **NA correctness boundary (MXF-4).** R's NA is a *specific* NaN bit pattern;
  IEEE arithmetic on a NaN yields an implementation-defined payload, so an NA
  pushed through the substrate's floating multiply/add would not reliably return
  as R's NA. When either operand contains an NA (or the inner dimension is `0`),
  `matrix_multiply` keeps the original loop, which emits `na_real()` exactly as
  before. The conformability check and the `MAX_SEQ_LEN` result-size cap are
  preserved ahead of dispatch; any substrate error falls through to the bounded
  loop. `t()`, `rowSums`/`colSums`/`apply`, `diag`, `solve`, and `det` are left
  on their existing implementations — none has a matching `array-runtime`
  primitive (transpose/axis-reductions are not lowered for execution;
  `solve`/`det` are LU algorithms the substrate doesn't model), and `solve`
  keeps its O(n³) size guard.

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
