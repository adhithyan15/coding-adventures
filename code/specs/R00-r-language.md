# R00 — The R Language

## Status

Active spec / roadmap for the **R** language frontend. R is "an implementation
of the S language" (Ihaka & Gentleman, 1993), so this frontend is built as a
sibling of the historical-S stack (`s-lexer`/`s-parser`/`s-runtime`/`s-repl`,
specced in [`S00-s-language.md`](S00-s-language.md)) and **reuses it heavily**.
This document defines what we support, the S↔R differences that matter, and the
item-by-item rollout.

## §1 Why R, on top of S

The S00 work already built a vector-first interpreter — recycling, NA
propagation, the coercion lattice, closures with lexical scope, S3 dispatch,
factors, and data frames — driven by a grammar-defined frontend over the shared
`r-vector` / `statistics-core` substrate. R shares all of that semantics and
~98% of S's grammar. So the R frontend is mostly: a token/parser grammar that
captures R's lexical and syntactic differences, plus thin crates that reuse the
S evaluator. We get an R REPL for a fraction of the cost of S because the hard
part (the evaluator and value model) already exists.

This also realizes the plan recorded in
[`ST00-r-stats-roadmap.md`](ST00-r-stats-roadmap.md), which always intended R to
sit on the generic language pipeline; S00 §9 noted "a future R frontend can
reuse most of `s-lexer`/`s-parser` with adjusted token/keyword sets."

## §2 The differences from S that matter

| Area | S | R |
|------|---|---|
| `_` | the **assignment** operator; never inside a name | an ordinary **name character** (`data_frame` is one name) — since R 1.9 (2004) |
| `=` | binds named call arguments only | also a top-level **assignment** operator |
| Right super-assign | — | `->>` |
| Typed `NA` | — | `NA_integer_`, `NA_real_`, `NA_character_` |
| Integer / complex / hex literals | — | `1L`, `1i`, `0x1F` *(later item)* |
| Pipe / lambda | — | `|>`, `\(x)` *(out of scope for now)* |

Everything else in S00 (vector semantics, operators incl. the v2 `%op%` infixes,
`$`/`[[`, control flow, function values, S3, factors, data frames) carries over
unchanged.

## §3 Rollout (one item = one PR)

- **R-1 — R00 spec + `r-lexer`** ✅ *merged*. `code/grammars/r.tokens` (the S
  token grammar with `_` moved into `NAME`, no `UNDERSCORE`, plus `->>` and the
  `NA_*` constants) and the `r-lexer` crate (a sibling of `s-lexer`, reusing the
  identical bracket-interior newline hook).
- **R-2 — `r-parser`** ✅ *merged*. `code/grammars/r.grammar`, mirroring
  `s.grammar`'s rule names exactly (so the shared `s-runtime` evaluator can
  consume the tree unchanged) and adding `=` and `->>` as assignment operators
  plus the typed-`NA` atoms. The `r-parser` crate is a sibling of `s-parser`.
- **R-3 — `r-runtime` + `r-repl` + the `R` binary** ✅ *merged*. A working R
  REPL. `s-runtime`'s `Interpreter::eval_str` was factored into a public,
  parser-agnostic `eval_program(&GrammarASTNode)`; `r-runtime` parses with
  `r-parser` and evaluates with the shared `s-runtime` (so R reuses the entire
  value model, S3 dispatch, factors, data frames, and built-ins). `=` / `->>`
  assignment and the typed-`NA` constants are handled in the shared evaluator.
  `r-repl` + the `R` binary mirror `s-repl`.
- **R-4 — typed numeric literals** ✅ *merged*. `r.tokens` gains `HEX_LIT`
  (`0x1F`, `0x1FL`), `INT_LIT` (`10L`, `1e3L`), and `COMPLEX_LIT` (`1i`), listed
  before `NUMBER`; the shared `eval_primary` maps `L`/`0x` to double (this subset
  has no distinct integer type) and reports `1i` as unsupported (no complex type
  yet) rather than producing a wrong value.
- **R-5 — string built-ins** ✅ *merged*. `nchar`, `toupper`, `tolower`,
  `substr` (1-based inclusive, char-boundary safe), and a minimal `sprintf`
  (`%d`/`%i`/`%s`/`%f`/`%e`/`%g`/`%%`, with width/`.precision`/`-`/`0`,
  vectorized; width/precision capped against a crafted-format DoS). Added to the
  shared `s-runtime` built-ins (S benefits too).
- **R-6 — a generic list type** *(this PR)*. `SValue::List` (ordered,
  optionally-named, heterogeneous; class `"list"`) added to the shared value
  model, with `[[i]]`/`[["name"]]`/`$name` extraction, `[i]` sub-list slicing,
  and R-style `$name`/`[[i]]` block printing. Built-ins `list(...)`,
  `lapply(x, f)` (returns a list), and `strsplit(x, split)` (fixed-string split
  → a list of character vectors). `nth_element` now iterates list elements, so
  `sapply`/`lapply` work over lists.
- **R-7 — regular expressions** *(this PR)*. `grepl(pattern, x)` → logical,
  `grep(pattern, x, value=)` → indices or matches, `gsub`/`sub(pattern, repl, x)`
  → replace all / first. Built on the `regex` crate (linear-time, no
  catastrophic backtracking), with a `fixed = TRUE` literal-match option and R
  back-reference translation (`\\1` → the crate's `${1}`). Invalid patterns
  return a clean error. *Deferred:* `regmatches`/`gregexpr`, `table`.
- **R-8 — the `d/p/q/r` distribution family** *(this PR)*. The four-prefix
  probability functions wired to `statistics-core`: density `d*`, cumulative
  `p*` (CDF), quantile `q*` (inverse CDF), and random sampling `r*`, for the
  closed-form continuous families **normal** (`dnorm`/`pnorm`/`qnorm`/`rnorm`,
  defaults `mean = 0`, `sd = 1`), **uniform** (`dunif`/…/`runif`, `min = 0`,
  `max = 1`), and **exponential** (`dexp`/…/`rexp`, `rate = 1`), plus
  `set.seed(n)`. `d*`/`p*`/`q*` are vectorized over their first argument with
  NA-propagation; parameters are read by name or position. `r*` draws from a
  per-session RNG (R-compatible MT19937) held on the `Interpreter`, so
  `set.seed(s); rnorm(n)` is reproducible; the sample count is capped at
  `MAX_SEQ_LEN` so `rnorm(1e18)` errors instead of aborting. *Deferred (R-8b):*
  the discrete families (`dbinom`/`dpois`/…), whose CDF/sampling loop over a
  user-supplied count and need their own bounds.
- **R-8b — the discrete distribution families** *(this PR)*. **Binomial**
  (`dbinom`/`pbinom`/`qbinom`/`rbinom`, parameters `size`, `prob`) and **Poisson**
  (`dpois`/`ppois`/`qpois`/`rpois`, parameter `lambda`), same `d`/`p`/`q`/`r`
  shape and reproducible RNG as R-8. The discrete CDFs and inverse-CDF samplers
  loop over an integer count (`pbinom` sums O(`size`) terms, `ppois` sums O(`x`)
  terms, `rbinom` is O(n·`size`)), so two guards bound every loop: a per-element
  cap (`size` and the `ppois` quantile ≤ `MAX_DISCRETE_SUPPORT` ≈ 1M) and a
  total-iteration budget (`MAX_DISCRETE_WORK` ≈ 134M) over `len·driver` /
  `n·per-sample`. `rbinom(1e6, 1e6, …)` and `ppois(1e18, …)` are clean errors,
  never hangs.
- **R-9 — modern R 4.1+ syntax** *(this PR)*. The **native pipe** `|>`
  (`x |> f(a)` is `f(x, a)`) and the **backslash lambda** `\(x) x + 1` (shorthand
  for `function(x) x + 1`). New `PIPE_OP`/`BACKSLASH` tokens in `r.tokens`; a
  `pipe` rule at the special-operator precedence level and a `\(…)` alternative
  added to `func_def` in `r.grammar`. The lambda *reuses the existing `func_def`
  evaluation unchanged* (same `param_list`/body children); the pipe is desugared
  in the shared evaluator (`eval_pipe`) — it inserts the left value as the first
  argument of the right-hand call, left-associatively, so `x |> f() |> g()` is
  `g(f(x))`. A bare `x |> f` (RHS not a call) is an error, as in R.
- **R-10 — higher-order functionals** *(this PR)*. The functional-programming
  toolkit that pairs with the R-9 `\(x)` lambdas, in the shared `s-runtime`:
  `Map(f, …)` (zip several sequences element-wise → a list, recycling to the
  longest), `mapply(f, …)` (same, simplified to a vector), `Reduce(f, x[, init])`
  (left fold), `Filter(f, x)` (keep elements where `f` is true), and
  `vapply(x, f, template)` (`sapply` with a result-shape check). Like
  `sapply`/`lapply` they invoke the function through `Interpreter::call_value`.
  The function is taken by name (`f =`/`FUN =`) or as the first callable
  positional, and the data are the other positionals — so they compose with the
  pipe: `1:5 |> Filter(f = \(x) x %% 2 == 0) |> Reduce(f = \(a, b) a + b)`.
- **R-11 — the matrix type** *(this PR)*. A new `SValue::Matrix { data, nrow,
  ncol }` (numeric, **column-major** — R/Fortran order, `length` is `nrow*ncol`,
  implicit class `"matrix"`). `matrix(data, nrow =, ncol =, byrow = FALSE)`
  builds one (recycling, deriving the missing dimension); `t()` transposes;
  `dim()`/`nrow()`/`ncol()` report the shape; `%*%` is the matrix product (a new
  arm in the evaluator's `%op%` infix dispatch — a bare vector becomes a row on
  the left, a column on the right, so `v %*% w` is the dot product); and
  `apply(X, MARGIN, FUN, …)` maps `FUN` over rows (`MARGIN = 1`) or columns
  (`MARGIN = 2`), simplifying to a vector or matrix. NA propagates through the
  product; the result-size and all loops are capped at `MAX_SEQ_LEN`. Matrices
  coerce to their flat vector (`c(m)`, `sum(m)`) and print with R's `[,j]`/`[i,]`
  console layout.
- **R-12 — matrix linear algebra** *(this PR)*. The builtins that turn the R-11
  matrix type into a usable linear-algebra object, all in the shared `s-runtime`:
  - `diag(x)` — the R triple-meaning overload: `diag(M)` extracts a matrix's
    diagonal as a vector; `diag(v)` for a length-`> 1` vector builds the square
    matrix with `v` on the diagonal; `diag(n)` for a single number builds the
    `n × n` identity. The vector/identity forms also accept `nrow`/`ncol`.
  - `rowSums`/`colSums`/`rowMeans`/`colMeans(x)` — the margin reductions, with an
    `na.rm = FALSE` option (NA in a margin propagates unless removed; an all-`NA`
    mean is `NaN`). Each returns a plain vector.
  - `cbind(…)` / `rbind(…)` — bind vectors and matrices by column / by row.
    Vectors are recycled to the common row (resp. column) length; a matrix whose
    rows (resp. columns) don't match is an error. The empty call is `NULL`.
  - `solve(a)` / `solve(a, b)` — the matrix inverse, and the solution `x` of
    `a %*% x = b` (`b` a vector or matrix). `det(a)` — the determinant. Both use
    **Gaussian elimination with partial pivoting** (no LU primitive exists in the
    substrate, so it is implemented directly); a singular `a` is a clean error
    (`det` of a singular matrix is `0`), an `NA` in `a` makes `det` return `NA`
    and `solve` an error, and the dimension is capped (`MAX_SOLVE_DIM`) so the
    `O(n³)` work cannot be turned into a denial-of-service. All construction is
    bounded by `MAX_SEQ_LEN`.
- **R-13 — 2-D matrix indexing** *(this PR)*. The `[` subscript operator extended
  to two dimensions and to R's full index styles, in the shared `s-runtime`:
  - A **grammar change** makes each comma-separated subscript *optional*, so an
    empty subscript selects a whole dimension: `m[i, j]` (one element),
    `m[i, ]` (a whole row), `m[, j]` (a whole column), `m[rows, cols]`
    (a sub-matrix). `index_suffix` becomes
    `LBRACKET [ subscript ] { COMMA [ subscript ] } RBRACKET` in both `s.grammar`
    and `r.grammar` (regenerated); the evaluator reads one slot per
    comma-separated position, `None` meaning "all of that dimension".
  - Each subscript resolves through a shared `resolve_picks` that now supports
    all three R index styles: **positive** (1-based; `0` drops, out-of-range/`NA`
    → `NA`), **negative** (`-k` *excludes* position `k`; cannot be mixed with
    positive), and **logical** (a mask recycled to the dimension; `TRUE` selects).
    This also fixes 1-D vector indexing — `v[-2]` and `v[c(TRUE, FALSE)]` now
    behave correctly (logical indices were previously mis-coerced to numbers).
  - `m[i]` indexes the **flat column-major** vector (dropping matrix structure,
    as R does). A 2-D result follows R's default `drop = TRUE`: a single row or
    column collapses to a vector, otherwise the result is a matrix. Out-of-range
    matrix subscripts are a hard error; the result size and the logical-recycle
    span are capped at `MAX_SEQ_LEN`. The empty-subscript grammar also enables
    `df[, j]` / `df[i, ]` on data frames.
  - *Done in R-14:* **sub-assignment** `m[i, j] <- v` (below).
- **R-14 — index sub-assignment** *(this PR)*. `m[i, j] <- v` and the 1-D
  `v[i] <- val`, in the shared `s-runtime`. The evaluator previously only
  assigned to **bare names**; R-14 adds the lvalue-target machinery so the left
  side of `<-` (or `=` / `->`) may be a **subscript expression**:
  - When the assignment target is not a bare name, the evaluator descends to the
    postfix `[ … ]`, requires a bare-name base, **looks up the current value of
    that base**, resolves the subscripts (reusing R-13's `resolve_picks` for the
    1-D case and the 2-D dimension resolver for `m[rows, cols]`), writes the RHS
    into the selected cells, and **rebinds the modified value to the base name**.
  - The write is **copy-then-rebind**: the base value is cloned, the clone is
    mutated, and `define` replaces the binding. Because `SValue` bindings are
    by-value, an earlier copy (`b <- a; a[1] <- 9`) is *not* aliased — `b` keeps
    its old contents, matching R's copy-on-modify semantics.
  - The RHS is **recycled** R-style to fill the selected cells (a length-1 RHS
    broadcasts; a length-*k* RHS repeats). 1-D selection accepts the full
    positive / negative / logical index styles; 2-D accepts `m[i, ]`, `m[, j]`,
    and `m[rows, cols]`. The matrix keeps its `dim` after assignment.
  - **Safety:** the selected positions are bounds-checked against the target
    length (an out-of-range or `NA` index in an *assignment* is a hard error, not
    a silent grow — vector auto-extension is deferred); an **empty replacement**
    (`v[i] <- c()`) is an error; assigning into an **undefined base** is an error.
    No write can touch another binding, so the rebind cannot corrupt unrelated
    variables. The number of writes is bounded by the (capped) selection length.

## §4 Reuse strategy

- **Lexer/parser:** the grammar-tools framework, exactly as S uses it. `r.tokens`
  / `r.grammar` compile to committed `_grammar.rs` in `r-lexer` / `r-parser`.
- **Runtime:** the `s-runtime` evaluator and `SValue` model are language-neutral
  — they walk a `GrammarASTNode` by rule name. By keeping `r.grammar`'s rule
  names identical to `s.grammar`'s, `r-runtime` can evaluate R programs through
  the same `Interpreter`. (R-3 adds the small public entry point for this.)
- **REPL:** `r-repl` mirrors `s-repl`'s single-threaded driver.

## §5 Out of scope (for now)

Pipes (`|>`) and backslash lambdas (`\(x)`); environments/`<<-` semantics beyond
the S subset; S4/R5/R6 OO; namespaces and `library()`; the C interface; graphics.
These layer on later, following ST00.

## §6 References

Internal: [`S00-s-language.md`](S00-s-language.md),
[`ST00-r-stats-roadmap.md`](ST00-r-stats-roadmap.md), `grammar-tools`,
`r-vector` / `statistics-core`.

External:

- R. Ihaka & R. Gentleman, *R: A Language for Data Analysis and Graphics*
  (J. Computational and Graphical Statistics, 1996).
- R Core Team, *The R Language Definition*.
