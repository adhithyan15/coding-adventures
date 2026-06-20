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
- **R-15 — `names()` and named-vector access** *(this PR)*. R's *names attribute*
  on atomic vectors, in the shared `s-runtime`. A new transparent wrapper
  `SValue::Named { names, values }` carries a parallel `Vec<Option<String>>` of
  element names beside a boxed atomic value (`Double`/`Logical`/`Character`), the
  same "see-through" wrapper pattern as `SValue::Classed` (`length`, `type_name`,
  coercions, arithmetic, comparison, and `class` all delegate to the inner value;
  most operations therefore ignore names, and where R *drops* names — arithmetic,
  comparison, `c()` of partly-unnamed pieces — so do we).
  - **Construction.** `c(a = 1, b = 2, c = 3)` attaches the argument names. `c()`
    builds a names vector iff any contributing argument is named *or* already
    carries names; nested named vectors combine R-style — `c(x = c(a = 1), 2)`
    yields names `c("x.a", "")` (a named element of a named piece is `outer.inner`;
    an unnamed slot is the empty string). A `c()` with no names anywhere stays a
    plain unnamed vector.
  - **`names(x)`** returns the character vector of names (an unset name → `NA`),
    or `NULL` when `x` has none. **`names(x) <- value`** is the replacement form:
    it coerces `value` to character and, R-style, **recycles by NA-padding** — a
    too-short names vector pads the tail with `NA`, a too-long one is an error;
    `names(x) <- NULL` drops the wrapper entirely. The evaluator gains a general
    **replacement-function** lvalue path so `f(x) <- v` desugars to
    `x <- \`f<-\`(x, v)` for the registered replacements (`names<-`; the
    machinery is reusable for future `levels<-`, `dim<-`, …). `setNames(x, nm)`
    is the functional form.
  - **Character indexing.** `x["b"]` and `x[c("a", "c")]` select by name; an
    unmatched name yields an `NA` element (and an `NA` name). Positional,
    negative, and logical indexing are unchanged, and a named vector indexed
    positionally **carries the selected names along** (`v[c(1, 3)]` keeps those
    two names), matching R.
  - **Printing.** A named vector prints R-style — a row of right-aligned names
    above the row of values, each column as wide as the wider of the two — instead
    of the `[i]` index prefix. Unnamed slots print as `<NA>`.
  - **Safety.** Names are attacker-controllable in length only through the same
    vector-length channels already capped at `MAX_SEQ_LEN`; the names vector is
    always kept exactly as long as the values (truncated/`NA`-padded on every
    constructor), so no name lookup can index out of bounds, and the
    character-index path reuses the bounds-checked `resolve_picks`. No new
    unbounded allocation or integer-overflow surface is introduced.

- **R-16 — general attributes** *(this PR)*. R's *general attribute system* — the
  open key→value metadata map every object carries — in the shared `s-runtime`.
  R-15 modelled the one special attribute it needed (`names`) as a transparent
  `SValue::Named` wrapper; R-16 generalizes that to **arbitrary** attributes while
  keeping the three *special* attributes — `names`, `class`, `dim` — routed to
  their existing dedicated representations so they can never disagree with the
  wrappers built for them in R-11/R-15/S v2.
  - **Representation.** A new transparent wrapper
    `SValue::Attributed { attrs: Vec<(String, SValue)>, inner }` stores the
    *general* (non-special) attributes as an insertion-ordered association list
    beside a boxed inner value — the same "see-through" pattern as `Named` and
    `Classed` (`length`, `type_name`, the coercions, arithmetic, comparison,
    `class`, indexing, and printing all delegate to `inner`; only the attribute
    builtins observe the map). The three special attributes are **never** stored
    in this map: `names` lives in `SValue::Named`, `class` in `SValue::Classed`,
    and `dim` is the `nrow`/`ncol` of `SValue::Matrix` (and the implicit
    `c(nrow, ncol)` of a data frame). This makes the consistency invariant
    structural rather than a runtime check — `attr(x, "names")` *is*
    `names(x)` because both read the very same `Named.names` field, and likewise
    for `class`/`dim`.
  - **`attr(x, which)`** returns the named attribute or `NULL` if absent. The
    special names are synthesized from the wrappers (`"names"` → the names vector,
    `"class"` → the explicit class if one was set, `"dim"` → `c(nrow, ncol)`);
    everything else is looked up in the general map. **`attr(x, which) <- value`**
    is the replacement form, wired through R-15's general replacement-function
    lvalue path (`attr(x, "foo") <- v` desugars to
    `x <- \`attr<-\`(x, "foo", v)`). Assigning `NULL` **removes** the attribute
    (clears the `Named`/`Classed`/`Matrix`-derived wrapper for a special name, or
    drops the entry — and the whole `Attributed` wrapper when its last entry goes
    — for a general one). Setting `"names"`/`"class"` routes to the same
    `with_names`/`Classed` machinery `names<-`/`structure` use; setting `"dim"`
    reshapes a length-`nrow*ncol` vector into a matrix (the product must equal the
    element count, as in R).
  - **`attributes(x)`** returns *all* attributes as a named `list` (special ones
    first, in R's canonical `names`, `dim`, then-general order, with `class`
    last), or `NULL` when the value has none. **`attributes(x) <- list(...)`**
    replaces the whole set: each named element is applied as the corresponding
    attribute (special names routed, the rest stored generally); `NULL` clears
    every attribute. An unnamed element, or a `value` that is not a list (other
    than `NULL`), is an error.
  - **`structure(x, ...)`** returns `x` with each `...` named argument attached as
    an attribute (`structure(1:3, class = "myc", foo = "bar")`). R-16 extends the
    v2 `structure` (which handled only `class`) to route every named argument
    through the same per-name logic as `attr<-`, so `dim`, `names`, and arbitrary
    attributes all attach correctly in one call. The `.Names`/`.Dim` aliases R
    accepts for `names`/`dim` inside `structure` are honoured.
  - **Safety.** The general attribute map is bounded: a per-object cap
    (`MAX_ATTRIBUTES`) refuses runaway `attr<-`/`attributes<-`/`structure` growth
    (a crafted `attributes(x) <- list` with millions of entries), and a `"dim"`
    set validates the reshape length with checked multiplication against
    `MAX_SEQ_LEN` before allocating, reusing the existing matrix bound. No
    `unwrap`/panic is reachable from malformed `attributes(x) <- …` input — a
    non-list `value`, an unnamed element, a too-long `names`, or a non-conforming
    `dim` all return a clean `SError`.
- **R-17 — `do.call` + named-list access polish** *(this PR)*. The reflective
  call primitive `do.call`, a small list-overlay helper `modifyList`, and a
  hardening pass over the R-6 named-list access operators — all in the shared
  `s-runtime` (R gets them through the same evaluator).
  - **`do.call(what, args)`** constructs and evaluates a call to `what` with the
    arguments supplied as the elements of the `list` `args`. `what` is either a
    callable value (closure or built-in) *or a length-one character string naming
    one*, in which case the name is resolved in the interpreter's global
    environment (an unknown or non-callable name is a clean error). Each element
    of `args` becomes one call argument: an **unnamed** element is positional and
    a **named** element is passed by name, preserving order — so
    `do.call(paste, list("a", "b", sep = "-"))` yields `"a-b"`, exactly as if
    `paste("a", "b", sep = "-")` had been written. The call reuses the
    interpreter's existing `call_value`/apply machinery (the same path
    `lapply`/`Reduce`/`Map` use), so default arguments, named/positional matching,
    and visibility all behave identically to a direct call. A `args` that is not
    a list (or `NULL`, treated as the empty argument list) is an error rather than
    a panic. The number of spread arguments is bounded by `MAX_DOCALL_ARGS` so a
    crafted multi-million-element `args` cannot build an unbounded call frame.
  - **Named-list access polish.** R-6 already routes `lst$name`,
    `lst[["name"]]`, and `lst[[i]]` through `dataframe::column_by_name` /
    `dataframe::extract`; R-17 pins the full contract with tests and closes any
    gaps: `lst$name` and `lst[["name"]]` return the element bound to that name;
    `lst[[i]]` returns the `i`-th element (1-based); a **missing** name
    (`lst$absent`, `lst[["absent"]]`) returns `NULL` (not an error), matching R;
    and these see through the `Classed`/`Attributed`/`Named` transparent wrappers
    so an attribute-carrying or classed list still indexes by name. (`$`/`[[name]]`
    on a *data frame* keep their stricter "undefined column" error — only the
    `list` type returns `NULL` for an absent name, as in R.)
  - **`modifyList(x, val)`** returns the list `x` with the elements of the list
    `val` overlaid **by name**: a name present in `val` and in `x` replaces that
    element; a name present only in `val` is appended; and a `val` element whose
    value is `NULL` **removes** that name from `x` (R's documented deletion
    semantics). Element order follows `x` (replacements in place, removals
    dropped) with `val`'s new names appended in `val`'s order. Both arguments must
    be lists (an unnamed `val` element, or a non-list argument, is an error); it
    composes with the access operators above. The result size is bounded by the
    same `MAX_DOCALL_ARGS`-class limit as `do.call`'s argument count so a crafted
    `val` cannot blow the list up without bound.
- **R-18 — `switch()` + error handling** *(this PR)*. R's value-returning
  multi-way branch and its condition-based error handling, all in the shared
  `s-runtime` (R inherits them through the same evaluator). The defining property
  is **lazy evaluation**: `switch` and `tryCatch` are **special forms** — the
  call dispatcher intercepts them and evaluates only the selected arm / protected
  expression / chosen handler, never all arms eagerly. See S00 §V2.9 for the full
  semantics. In brief:
  - **`switch(EXPR, ...)`** — character `EXPR` matches arm *names* (an **unnamed
    final arm** is the default; no match and no default → invisible `NULL`);
    numeric `EXPR` selects the n-th arm by position (out of range → `NULL`). Only
    the chosen arm evaluates, so `switch("a", a = stop("x"), b = "ok")` does not
    raise. An **empty arm** falls through to the next non-empty arm:
    `switch("a", a = , b = "hit")` → `"hit"` (and chains: `a = , b = , c = "z"`
    → `"z"`). *(R-19 added this.* It extends the shared S/R grammar's `arg` rule
    to `arg = NAME EQ [expr] | expr` so a named argument may omit its value, then
    regenerates the compiled `_grammar.rs` for both `s-parser` and `r-parser`.
    `eval_switch` already implemented the fall-through; the grammar change makes
    `a = ,` parse instead of erroring. An empty value parses everywhere but is
    only meaningful in `switch`; an empty arg in an ordinary call is an eval-time
    error, matching R. See S00 §V2.9.)*
  - **`stop(...)`** raises an error (concatenated message → `SError::User`);
    **`warning(...)`** emits a warning and returns invisibly without aborting;
    **`tryCatch(expr, error = fn, finally = cleanup)`** runs `expr`, routing any
    error to `error` (called with a minimal condition object —
    `list(message, call)` classed `c("simpleError","error","condition")`, so
    `conditionMessage(e)` and `e$message` give the message), and always running
    `finally`. **`conditionMessage(e)`** reads the condition's message. Full R
    condition machinery (custom condition classes, calling handlers, restarts) is
    out of scope.

- **R-20 — functional helpers** *(this PR)*. The remaining members of R's
  functional-programming toolkit (`?funprog`), built on the R-10 family and, like
  it, living in the shared `s-runtime` (R inherits them through the same
  evaluator — no grammar change, all plain builtins). They take their function by
  name (`f =`/`FUN =`) or as the first callable positional and the data as the
  remaining positionals (the R-10 `split_fun` helper), so they compose with the
  pipe (`1:5 |> Find(f = \(x) x > 2)`):
  - **`Find(f, x)`** — the **first** element of `x` for which `f(element)` is
    `TRUE`; if none matches, an **invisible `NULL`**. `f` is invoked through
    `Interpreter::call_value`, exactly as `Filter` does, but it short-circuits on
    the first hit rather than scanning the whole vector.
  - **`Position(f, x)`** — the **1-based index** of the first matching element
    (the counterpart to `Find`, which returns the *value*); `NULL` if none match.
  - **`Negate(f)`** — returns a **new callable** computing `!f(...)`: the logical
    negation of `f`'s result. Implemented with a small dedicated value,
    `SValue::Negated(Box<SValue>)`, that wraps the function; calling it invokes
    the inner `f` (through the same `call_value`/`apply` path, so recursion is
    bounded by `MAX_EVAL_DEPTH`) and negates the verdict via the shared `negate`
    coercion (so `Negate(is.na)(NA)` → `FALSE`, `Negate(\(x) x > 0)(5)` →
    `FALSE`). `is.callable`/`Negate(f)(...)` see it as a function; it negates
    element-wise and is `NA`-preserving like `!`. The wrapped `f` must itself be
    callable (else an `NotCallable` error, never a panic).
  - **`Reduce(f, x, ..., accumulate = FALSE)`** — the R-10 left fold, now with
    R's **`accumulate`** flag. With `accumulate = TRUE` the result is the vector
    (or list) of *running* folds rather than the final one:
    `Reduce(\(a, b) a + b, 1:4, accumulate = TRUE)` → `c(1, 3, 6, 10)`; with an
    `init`, the init is the **first** accumulated element
    (`Reduce(\(a, b) a + b, 1:3, 10, accumulate = TRUE)` → `c(10, 11, 13, 16)`).
    The no-`accumulate` behaviour (with and without `init`) is unchanged. The
    accumulated result is built with the shared `combine`/`c()` engine, so it
    simplifies to a vector for atomic folds and stays a list when the folds are
    themselves lists; an empty `x` with no `init` is `NULL`. The accumulated
    length never exceeds `length(x) (+1 for init)`, itself bounded by
    `MAX_SEQ_LEN`.
  - **`Recall(...)`** — **anonymous recursion**: inside a running function body,
    `Recall(args…)` re-invokes *that same function*. The interpreter keeps a small
    **call stack** of the currently-executing closures (pushed/popped by
    `call_closure` with an RAII guard, so it is exception-safe); `Recall` reads
    the top and calls it with the supplied arguments. Outside any function it is
    an error (`"Recall called from outside a closure"`). Recursion is bounded by
    `MAX_EVAL_DEPTH` (each `Recall` goes through `call_value` → `eval_node`),
    so runaway anonymous recursion returns a clean error instead of overflowing
    the native stack. This makes the classic anonymous factorial work:
    `(\(n) if (n <= 1) 1 else n * Recall(n - 1))(5)` → `120`.

- **R-21 — environments & scoping (core subset)** *(this PR)*. R's *environment
  model* — the machinery behind lexical scoping — made writable from R source.
  R reuses the shared S scope chain (`env::Scope`, an `Rc<RefCell<…>>` frame with
  an optional parent), so this item is **grammar-free**: the `<<-`/`->>` tokens
  already lex (they were reserved in `s.tokens`/`r.tokens` from the start and the
  `->>` right form was wired through the `assignment` rule by R-3), and the new
  by-name binding operations are **lazy special forms** intercepted at the call
  site (like R-18's `switch`/`tryCatch`), since they need the *current*
  environment, which ordinary eager builtins never see. This PR ships the core
  subset; first-class environment **values** are deferred to R-22 (see below).
  - **`<<-` super-assignment** (and its right form `->>`). Where `<-` binds in
    the *current* scope, `<<-` walks the chain of **enclosing** scopes (skipping
    the current one) looking for an existing binding of the name and rebinds the
    **nearest** one it finds. If no enclosing scope binds the name, the value is
    created in the **global** environment (matching R). This is what makes the
    counter-closure idiom work: `make_counter <- function() { n <- 0; function()
    { n <<- n + 1; n } }` — the inner function mutates the `n` captured in its
    enclosing frame rather than shadowing it locally. The chain walk is bounded
    by the finite scope depth (every frame is a distinct `Rc`, and `child` only
    ever links to an existing parent, so the chain is a DAG-free finite list — no
    cycle is constructible from R source), so the walk always terminates.
  - **`local({ … })`** — evaluate a block in a **fresh child environment** of the
    current scope and return the block's value. Bindings made with `<-` inside the
    block are locals and do **not** leak: `local({ x <- 5; x * 2 })` → `10`, and
    `x` is unbound afterwards. A lazy special form: the unevaluated block argument
    is evaluated in `Scope::child(env)`. (R also accepts a second `envir`
    argument; that requires first-class environment values and is deferred.)
  - **`assign(x, value)` / `get(x)` / `exists(x)` / `rm(x)`** — by-name binding
    operations against the **current** environment. `assign("y", 10)` binds `y`;
    `get("y")` returns it (erroring if unbound, like R); `exists("y")` is
    `TRUE`/`FALSE` searching the whole chain (so `exists("mean")` is `TRUE`,
    `exists("zzz")` is `FALSE`); `rm("y")` removes a binding from the current
    frame. These are special forms because they must touch `env`; the name
    argument is a length-one string evaluated normally (so `assign(nm, v)` with a
    variable name works). The optional `envir = e` argument is **deferred** to
    R-22 (it needs a first-class environment value to point at).
  - **Deferred to R-22 (first-class environment values).** `new.env()`,
    `environment()` / `environment(f)`, and the `envir = e` argument of
    `assign`/`get`/`exists`/`rm`/`local` all require a new `SValue::Environment`
    variant that boxes an `Env` handle and survives being passed as a value.
    That is a larger change (printing, `is`-dispatch, copy semantics, and a fresh
    round of Rc-cycle / leak analysis), so per the roadmap's "clean subset beats a
    sprawling PR" guidance it is split out. The subset above already delivers the
    *behaviour* R programs reach for most (`local`, `<<-`, `exists`/`get`/`assign`
    against the current scope); R-22 adds the reified handles on top without
    changing any of it.

- **R-22 — first-class environment values** *(this PR)*. Reifies the shared S
  scope chain as a first-class value, completing the piece R-21 deferred. A new
  `SValue::Environment(Env)` variant boxes the shared `Env`
  (`Rc<RefCell<Scope>>`) handle, so a scope can be passed around, stored, and
  mutated **by reference**. This is grammar-free (the names lex as ordinary
  identifiers — `.` is a name character in both `s.tokens` and `r.tokens`, so
  `new.env` is one token) and is wired through the same lazy-special-form path
  R-18/R-21 use.
  - **Rc-cycle ownership model (the central correctness concern).** Once a scope
    can hold an `SValue::Environment` in its `vars`, an environment value can
    reference another environment — and, crucially, a child's *parent* link could
    point back to a scope that (transitively) holds the child. A naive strong
    `Rc` parent link would then form a reference cycle that `Rc` can never
    collect, leaking every binding in it. **R-22 makes the `Scope::parent` link a
    `Weak<RefCell<Scope>>`.** The interpreter keeps the global environment alive
    for the whole session (a strong `Rc` in `Interpreter::global`), and each live
    call frame is held by a strong `Rc` on the native call stack for the duration
    of the call; parents are only ever *referenced*, never *owned*, by their
    children. Therefore: (a) no cycle of strong `Rc`s is constructible from R
    source — every strong edge runs root→leaf via interpreter/call-stack
    ownership or a value binding, while every parent edge is `Weak`; (b) a child
    environment captured as a value (e.g. returned from `new.env()` and stored in
    a variable) keeps itself alive through that strong binding, and its `Weak`
    parent upgrades successfully as long as *something else* still owns the
    parent (the global env always does, transitively). A `Weak` parent that
    cannot be upgraded (its frame was dropped) is treated as "no parent" — the
    chain walk simply stops, exactly as it would at the root. This is documented
    inline in `env.rs` and is the focus of the R-22 security review.
  - **`new.env()`** — create a fresh environment whose parent is the **caller's**
    current environment, and return it as a first-class value. Two calls produce
    two independent environments. A lazy special form (it needs the current
    `env`).
  - **`environment()`** — the **current** environment as a value. The
    `environment(f)` form (a closure's captured environment) is **deferred to
    R-23** with a clear note: it requires reaching into the `Closure { env, .. }`
    payload, which is a smaller, orthogonal follow-up.
  - **`envir = e` on `assign`/`get`/`exists`/`rm`** — operate on the passed
    environment **value** rather than the current scope. R-21's runtime rejection
    is replaced by the real behaviour: `assign("x", 1, envir = e)` binds `x` in
    `e`; `get`/`exists` read it; `rm` deletes it. A non-environment `envir`
    argument is a clean `BadArgs` error (never a panic). `assign`/`rm` act on the
    target frame directly; `get`/`exists` walk *that* environment's chain
    outward, matching R.
  - **By-reference mutation.** Because an environment value shares the same
    `Rc<RefCell<Scope>>`, mutating it through one alias is visible through every
    other: `e <- new.env(); f <- function(env) assign("x", 1, envir = env); f(e);
    get("x", envir = e)` → `1`. This is the defining difference from R's
    otherwise copy-on-modify value semantics.
  - **`ls(envir = e)`** — the names bound directly in `e` (its own frame, not the
    enclosing chain), **sorted** as a character vector. `ls()` with no `envir`
    lists the current environment. `environmentName` is **deferred to R-23**.
  - **Printing.** An environment prints as `<environment>` — a **stable
    placeholder**, deliberately *not* the real heap address R shows
    (`<environment: 0x55...>`), so test output is deterministic across runs and
    platforms. Its implicit class is `"environment"` and `type_name` is
    `"environment"`.

## §4 Reuse strategy

- **Lexer/parser:** the grammar-tools framework, exactly as S uses it. `r.tokens`
  / `r.grammar` compile to committed `_grammar.rs` in `r-lexer` / `r-parser`.
- **Runtime:** the `s-runtime` evaluator and `SValue` model are language-neutral
  — they walk a `GrammarASTNode` by rule name. By keeping `r.grammar`'s rule
  names identical to `s.grammar`'s, `r-runtime` can evaluate R programs through
  the same `Interpreter`. (R-3 adds the small public entry point for this.)
- **REPL:** `r-repl` mirrors `s-repl`'s single-threaded driver.

## §5 Out of scope (for now)

Pipes (`|>`) and backslash lambdas (`\(x)`); the `environment(f)` form (a
closure's captured environment) and `environmentName` — **deferred to R-23** (the
`SValue::Environment` value, `new.env()`, `environment()`, `ls(envir=)`, and the
`envir = e` argument of `assign`/`get`/`exists`/`rm` all land in **R-22**);
S4/R5/R6 OO; namespaces and `library()`; the C interface; graphics. These layer
on later, following ST00.

## §6 References

Internal: [`S00-s-language.md`](S00-s-language.md),
[`ST00-r-stats-roadmap.md`](ST00-r-stats-roadmap.md), `grammar-tools`,
`r-vector` / `statistics-core`.

External:

- R. Ihaka & R. Gentleman, *R: A Language for Data Analysis and Graphics*
  (J. Computational and Graphical Statistics, 1996).
- R Core Team, *The R Language Definition*.
