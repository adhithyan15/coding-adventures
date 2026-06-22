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
    children. Therefore: (a) **no cycle through the parent chain is constructible**
    — every parent edge is `Weak`, so the parent relation stays a finite acyclic
    list, and the chain-walk operations (`lookup`/`exists`/`super_assign`) always
    terminate; (b) a child environment captured as a value keeps itself alive
    through that strong binding, and its `Weak` parent upgrades as long as
    *something else* still owns the parent (the global env always does,
    transitively). A `Weak` parent that cannot be upgraded is treated as "no
    parent" — the chain walk stops, as at the root.
  - **The residual cycle: value bindings (a bounded, documented limitation).** The
    `Weak` parent breaks only cycles *through the parent edge*. A cycle can still
    form *through a value binding*, because an environment value is a **strong**
    `Rc`: `assign("self", e, envir = e)` stores a strong `Rc`-to-`e` inside `e`
    itself (and a mutual `a`/`b` pair does the same), an `Rc` cycle that — absent a
    tracing GC, which R has and we do not — cannot be reclaimed once unreachable.
    R-22 does **not** claim to collect it; it **bounds** the damage with a
    per-session `MAX_ENVIRONMENTS` cap (in `eval.rs`) on the number of
    environments `new.env()`/`environment()` may reify, so a crafted loop building
    cyclic environments hits a clean error instead of exhausting memory. This and
    the `Weak` parent are documented inline in `env.rs`/`value.rs` and are the
    focus of the R-22 security review. (Divergence from the first draft of this
    spec, which over-claimed that *no* strong-`Rc` cycle was constructible; the
    value-binding cycle and its bounded mitigation are the corrected model.)
  - **`new.env()`** — create a fresh environment whose parent is the **caller's**
    current environment, and return it as a first-class value. Two calls produce
    two independent environments. A lazy special form (it needs the current
    `env`).
  - **`environment()`** — the **current** environment as a value. The
    `environment(f)` form (a closure's captured environment) lands in **R-23**
    (it reaches into the `Closure { env, .. }` payload).
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
    lists the current environment. `environmentName` lands in **R-23**.
  - **Printing.** An environment prints as `<environment>` — a **stable
    placeholder**, deliberately *not* the real heap address R shows
    (`<environment: 0x55...>`), so test output is deterministic across runs and
    platforms. Its implicit class is `"environment"` and `type_name` is
    `"environment"`.

- **R-23 — closure environments & call-frame reflection** *(this PR)*. Builds
  directly on the R-22 `SValue::Environment` value, the `MAX_ENVIRONMENTS` cap,
  and the R-20 call stack. Everything lands in the shared `s-runtime`; no grammar
  change (every name lexes as an ordinary identifier).
  - **Two well-known environments held by the interpreter.** R-22 kept one
    long-lived strong handle: `Interpreter::global`. R-23 adds a second, the
    **empty** environment — a single root scope with **no parent and no bindings**
    that the interpreter owns for the whole session (a strong `Rc`, exactly like
    `global`). It is the terminus R uses for `emptyenv()` and the value that
    `environmentName` recognises as `"R_EmptyEnv"`. Holding it on the interpreter
    means `emptyenv()`/`globalenv()` hand back the *same* `Rc` every call, so
    `identical`-style reference equality (pointer equality on the `Rc`) is stable.
  - **`environment(f)`** — the environment a closure **captured** at definition.
    A `Closure` already stores its defining scope in `Closure { env, .. }`; R-23
    exposes that `env` as an `SValue::Environment`. For a closure defined at top
    level this is the global env, so `identical(environment(f), globalenv())` is
    `TRUE`. A non-closure argument (a builtin, a number, …) returns `NULL`,
    matching R (`environment(sum)` is `NULL`). Reifying the captured env counts
    against `MAX_ENVIRONMENTS` like any other reification — it can equally
    participate in a value-binding cycle.
  - **`environment(f) <- e`** — set a closure's captured environment, reusing the
    R-15/R-16 replacement-function lvalue path (`f(x) <- v` ≡
    ``x <- `f<-`(x, v)``). A new `environment<-` builtin takes the closure and the
    replacement environment `value = e` and returns a **fresh** `Closure` with its
    `env` field swapped (closures are immutable values; we rebind the variable, as
    R does). The first argument must be a closure and `value` must be an
    environment, else a clean `BadArgs`/`TypeError` (never a panic). Because the
    new closure now closes over `e`, free variables in its body resolve from `e`'s
    chain.
  - **`environmentName(e)`** — `"R_GlobalEnv"` if `e` **is** the global env (`Rc`
    pointer equality against `Interpreter::global`), `"R_EmptyEnv"` if it is the
    empty env, `""` otherwise. A non-environment argument is a clean error.
  - **`globalenv()` / `emptyenv()` / `baseenv()`** — the well-known environments
    as values. `globalenv()` and `baseenv()` both return the session global env
    (this runtime installs its builtins directly into the global frame — there is
    no separate base namespace, so `baseenv()` aliases the global env, documented
    as a deliberate simplification); `emptyenv()` returns the interpreter's empty
    env. All three are lazy special forms (they need the interpreter, not the
    current `env`); none allocate, so none counts against `MAX_ENVIRONMENTS`.
  - **`parent.frame(n = 1)`** — the environment of the **caller** `n` frames up
    the call stack. R-20's call stack recorded the *closure* being run (for
    `Recall`); R-23 records the **caller's environment** alongside it, so a frame
    is now `(closure, caller_env)`. `call_closure` pushes the env in which the
    call expression was evaluated; `parent.frame()` reads the top caller env,
    `parent.frame(n)` the n-th from the top. A binding the caller made is visible
    through it: `g <- function() get("x", envir = parent.frame()); f <- function()
    { x <- 42; g() }; f()` → `42`. **Clamping (not panicking) past the bottom.**
    `n` larger than the live frame depth, or `parent.frame()` at top level, falls
    back to the **global** environment — R returns `R_GlobalEnv` there — so the
    walk can never index out of bounds. `n` must be a positive whole number; a
    non-positive or non-finite `n` is a clean `BadArgs` error.
  - **`is.environment(x)`** — `TRUE` iff `x` is an environment value, else
    `FALSE` (the predicate the tests assert `environment(f)` through). A trivial
    one-element logical, vectorised over… nothing (it inspects the single value's
    type), matching R's scalar predicate.
  - **Rc-cycle safety (unchanged model, two new exposure points).** R-23 hands out
    two *new* kinds of environment value — a closure's captured env and a caller
    frame's env — but neither widens the ownership model R-22 established. The
    caller-env stored on the call stack is dropped when the frame is popped (the
    RAII `CallFrameGuard`), so it never outlives the call; `parent.frame()` clones
    the `Rc` out for the duration of one expression. A closure's captured env was
    already reachable (the closure owned a strong `Rc` to it); exposing it as a
    value adds another strong `Rc`, the same situation as `environment()`, and is
    bounded by the same `MAX_ENVIRONMENTS` cap. The `Weak` parent link still
    prevents any parent-chain cycle. `parent.frame(n)` clamps rather than indexes,
    so a crafted deep `n` cannot panic.

- **R-24 — R5 reference classes (`setRefClass`)** *(this PR)*. The payoff of the
  R-21/22/23 environment work: an R5 object **is** an environment holding its
  fields, and a method **is** a closure whose enclosing environment is the
  instance environment — so a method body sees fields directly and writes them
  back with `<<-`. Everything lands in the shared `s-runtime`; no grammar change
  (`$` already exists from the data-frame work).
  - **`setRefClass("Name", fields = list(x = "numeric", …), methods = list(m = function(…) …))`**
    → a **generator** object. `setRefClass` is a **lazy special form** (like
    `switch`/`new.env`): it must capture the `methods = list(…)` *function
    definitions unevaluated* and the *current environment* (the generator's
    enclosing scope), neither of which an eager builtin receives. The generator
    is represented as an `SValue::Environment` — a fresh scope carrying three
    private bindings: `.refClassName` (the class name, a character), `.refFields`
    (the field names, a character vector, in declaration order), and `.refMethods`
    (a `list` of the method **closures** as written, each closing over the
    generator's defining scope). The field *type* strings (`"numeric"`,
    `"character"`) are recorded but **not enforced** in this subset — R5 type
    checking is deferred — so a field may hold any value. Reifying the generator
    env counts against `MAX_ENVIRONMENTS`.
  - **`generator$new(x = …, y = …)`** → an **instance**. `new` creates a *fresh*
    child environment of the generator's defining scope and binds: every declared
    field (to the matching `new(field = …)` argument, or `NULL` when omitted),
    `.self` (an `SValue::Environment` pointing at the instance itself), and
    `.refMethods` (carried from the generator so methods can be rebuilt on
    access). The instance also counts against `MAX_ENVIRONMENTS`.
  - **`obj$field`** reads a field — an ordinary `env::lookup` in the instance
    scope (the `$` read path, extended for `SValue::Environment`, returns the
    bound value, or `NULL` for an unset field, matching R5).
  - **`obj$field <- v`** writes a field **by reference**. The `$<-` lvalue path is
    extended for an `SValue::Environment` target: it `env::define`s the name in the
    *instance scope in place*, mutating the live environment rather than rebinding a
    copy. This is what gives R5 its **reference semantics** — see below.
  - **`obj$method(args)`** calls a method. The `$` read path, seeing a name that is
    *not* a field but *is* present in `.refMethods`, **reconstructs** a fresh
    `Closure` whose `env` is the **instance** environment (not the generator's),
    then the trailing `call_suffix` applies it. Because the method closes over the
    instance scope, its body sees the fields as free variables and updates them
    with `field <<- value` (super-assignment walks to the enclosing instance frame
    and rebinds in place — R-21's `<<-`), or via `.self$field <- v`. `.self` is
    bound in the instance, so a method may call a sibling method as
    `.self$other(…)`.
  - **Reference semantics (the headline).** `b <- a` copies the *environment
    handle* (a strong `Rc` to the same scope), **not** the scope's contents — so
    `a` and `b` are two references to one instance. `b$add(1)` mutates the shared
    scope and `a$total` reflects it. This is the deliberate exception to R's
    otherwise copy-on-modify value semantics, and it falls straight out of reusing
    R-22's first-class, mutate-by-reference `SValue::Environment`: the instance *is*
    a live scope, and assigning it to a new name aliases rather than clones it. Two
    instances built by two separate `$new` calls are *independent* (distinct
    scopes), so their fields do not interfere.
  - **Rc-cycle safety (the instance⇄method trap, broken by construction).** The
    obvious encoding — bind each method *as a closure* directly in the instance's
    `vars`, with that closure's `env` being the instance — forms a **strong
    reference cycle**: `instance.vars["m"]` is a `Closure` whose `env` is a strong
    `Rc` back to `instance`. `Rc` never reclaims a cycle, so every such instance
    would leak its whole scope. **We break it by never storing the
    instance-bound closures.** The instance holds only the *field bindings*, `.self`,
    and `.refMethods` — and the closures inside `.refMethods` close over the
    **generator's** scope, *not* the instance's, so they create no edge back to the
    instance. The instance-bound method closure is materialised **lazily, per
    access**, in `obj$method`, lives only for the duration of that one call, and is
    dropped immediately after — it is never stored anywhere reachable from the
    instance, so the instance→method→instance edge never exists at rest. The one
    remaining strong self-reference is `.self` (an `Environment` to the instance
    stored *inside* the instance) — exactly the *documented, pre-existing* R-22
    value-binding self-cycle, bounded (not collected) by the `MAX_ENVIRONMENTS`
    session cap; we inherit that boundary verbatim rather than widening it.
  - **Borrow-panic safety.** A method that mutates a field mid-call goes through
    `env::define`/`env::lookup`, each of which takes and releases the instance
    scope's `RefCell` borrow *per operation* (the iterative-walk discipline R-22
    established), so a method reading one field and writing another never holds two
    borrows of the same scope at once. Malformed `setRefClass`/`$new` arguments — a
    non-character class name, a `fields`/`methods` that is not a list, a method
    entry that is not a function, an unknown `new()` argument — are all clean
    `BadArgs`/`TypeError` results, never panics.
  - **Deferred to R-25.** Inheritance (`contains = "Base"`), `$copy()` (a deep
    value-copy breaking the reference sharing), and the `$methods()`/`$fields()`
    introspection accessors land in **R-25** (below). Active bindings remain
    deferred to **R-26**. R-24 ships fields + methods + `$new` + `$field`
    read/write + method calls with `<<-`/`.self$field <-` field update, solidly
    and with full reference-semantics tests.

- **R-25 — R5 inheritance, `$copy()`, and introspection** *(this PR)*. Builds
  directly on R-24's generator/instance model in `refclass.rs`; no grammar change.
  - **`setRefClass("Sub", contains = "Base", fields = …, methods = …)`** —
    **single inheritance**. The `contains =` argument names the parent class; it
    is given as the parent's **generator** value (the result of an earlier
    `setRefClass`), or a length-1 character giving the parent class *name* (resolved
    against the current environment). The subclass generator carries a fourth
    private binding, `.refParent` (an `SValue::Environment` to the parent generator),
    so the class chain can be walked at instantiation and introspection time. The
    **effective field set** is the union *base ∪ sub* in **base-first** declaration
    order (an inherited base field precedes a new sub field); the **effective method
    set** is *base ∪ sub* with a **sub method overriding** a same-named base method.
    An inherited base method is callable on a `Sub` instance, and a `Sub` method may
    read and write base fields (they live in the one flat instance frame, so `<<-`
    reaches them identically).
  - **`obj$copy()`** — a **deep** value-copy. Unlike `b <- a` (which aliases the
    same scope — R-24's headline reference semantics), `b <- a$copy()` returns a
    **new, independent** instance: a fresh child scope of the same generator, with
    each *field* binding copied across by value, and fresh `.self`/`.refMethods`
    markers. Mutating `b` afterward does **not** affect `a`. The copy charges one
    fresh environment against `MAX_ENVIRONMENTS` (it is a new reified scope, exactly
    like `$new`). Field values are cloned via the normal `SValue` clone; a field
    that *itself* holds another instance is copied as a **handle** (a shallow alias
    of that nested instance) rather than recursively deep-copied — matching R5's
    `copy(shallow = TRUE)` default and sidestepping any unbounded copy recursion.
  - **Introspection.** `generator$fields()` returns the **sorted** character vector
    of all field names (including inherited ones); `generator$methods()` returns the
    **sorted** character vector of all method names (including inherited ones).
    `is(obj, "Base")` and `inherits(obj, "Base")` return `TRUE` when `obj`'s class
    is `"Base"` *or descends from it* through the `contains =` chain — the **class
    chain** of an R5 instance is `c("Sub", "Base", …, "envRefClass", "environment")`,
    computed by walking `.refParent` from the instance's generator up to the root.
  - **Rc-cycle safety.** Inheritance adds exactly one new edge — the subclass
    generator's `.refParent` strong `Rc` to the parent generator — which is a
    **DAG** edge (child → parent), never a cycle: a `contains =` that would close a
    loop (`A contains B contains A`) is **rejected** at `setRefClass` time by walking
    the prospective parent chain and refusing if the new class name already appears
    in it (or if the chain exceeds a depth bound). `$copy()` introduces no cycle: it
    builds a sibling instance exactly as `$new` does, and copies fields by value
    (nested instances aliased, not recursed), so the copy is bounded by the field
    count and charged against `MAX_ENVIRONMENTS`. The instance⇄method discipline is
    inherited verbatim from R-24 (methods are rebuilt lazily, never stored on the
    instance).
  - **Borrow-panic safety.** Inheritance and `$copy()` go through the same
    per-operation `env::define`/`env::lookup` borrows R-24 established. Malformed
    inputs — a `contains =` naming a non-generator / undefined class, a cyclic
    `contains =`, `$copy()` called on a generator rather than an instance — are clean
    `BadArgs`/`TypeError` results, never panics.
  - **Deferred to R-26.** Multiple inheritance (`contains = c("A", "B")`),
    `callSuper()`, and active bindings remain out of scope. R-25 ships
    single-`contains=` inheritance + `$copy()` + `is`/`inherits` over the R5 class
    chain + `$fields()`/`$methods()`, solidly. A clean partial beats a sprawling one.

- **R-26 — R5 `callSuper()`, active bindings, and multiple inheritance** *(this PR)*.
  Completes the R5 reference-class system on top of R-24/R-25's generator/instance
  model in `refclass.rs`; no grammar change.
  - **`callSuper(...)`** — inside an overriding method, invoke the **same-named**
    method from the parent class. The challenge is method-identity: when `Sub$describe`
    overrides `Base$describe`, a `callSuper()` inside the running `describe` must reach
    `Base`'s `describe`, *not* re-enter `Sub`'s (infinite recursion). We solve it by
    making `rebuild_method` — which already materialises a fresh instance-bound closure
    on each `obj$method` access — record **which class defined the method version it is
    about to run** and wrap the closure's captured environment in a thin *super-context*
    scope binding two private markers: `.refSuperGen` (the defining class's **parent**
    generator, where same-name resolution should *restart*) and `.refMethodName` (the
    method's name). The method body runs in a child of that super-context scope, so a
    `callSuper()` call inside it finds both markers by ordinary lexical lookup. `callSuper`
    itself is a **lazy special form** (it must read the markers from the *calling*
    environment, and forward the call args): it resolves the same method name starting at
    `.refSuperGen`, re-homes that closure onto the instance with a *fresh* super-context
    pointing one level further up (so a chain `C→B→A` of `callSuper()` walks all the way
    to the root), and applies it to the forwarded args. **Past-the-root safety:** a
    `callSuper()` in a method that has no parent definition of that name is a clean,
    no-recursion `NULL` (matching R5, which silently returns `NULL` when there is no
    super method) — never a panic and never an unbounded walk.
  - **Active bindings** — a field whose declared *type* is a `function(v)` becomes an
    **active binding**: reading `obj$ab` **calls** the function as a getter (with no
    argument, so `missing(v)` is `TRUE`), and `obj$ab <- val` **calls** it as a setter
    with `v = val`. In `setRefClass(…, fields = list(celsius = "numeric", fahrenheit =
    function(v) …))` the function-valued field *is* the active binding; a string-valued
    field stays an ordinary data field. The active-binding function is stored on the
    instance frame under the field name (re-homed onto the instance so it reads sibling
    fields and writes them with `<<-`). The `$` read path detects a callable field value
    and invokes it nullary; the `$<-` path detects it and invokes it with the new value.
    `missing(v)` is added as a special form that reports whether the named parameter was
    actually supplied at the call site — `TRUE` in the getter (no arg), `FALSE` in the
    setter (`v` supplied) — which is exactly how a single `function(v)` serves both
    directions. **Re-entrancy / borrow safety:** the getter/setter is invoked through
    the ordinary depth-bounded call path (`MAX_EVAL_DEPTH`), so a getter that reads its
    *own* binding (`obj$ab` inside the `ab` getter) recurses through the call path and
    hits the depth cap with a clean error rather than a borrow panic or a hang; field
    reads/writes inside the body use the existing per-operation `env::define`/`lookup`
    borrows (each takes-and-releases the scope `RefCell` within the call), so a setter
    that mutates a sibling field never holds two borrows of the same scope at once.
  - **Multiple inheritance** — `contains = c("A", "B")`. A subclass generator now
    carries a **list** of parent generators (`.refParents`) rather than a single
    `.refParent`. The **linearization** is a simple **left-to-right depth-first**
    pre-order walk: the class itself, then A and all of A's ancestors, then B and all
    of B's ancestors (skipping any already seen). C3 is *not* implemented — the simple
    DFS order is documented and sufficient for the union semantics. The **effective
    field set** is the union over the linearization in first-seen order (so A's fields
    precede B's, both precede the class's own only where re-declared — own declarations
    extend the set); the **effective method set** is the union with **left-to-right,
    most-derived-first precedence** (the class's own methods override A's, which override
    B's, which override their ancestors'). `is`/`inherits` see every class in the
    linearization. **Cycle / Rc safety:** the `.refParents` edges are all DAG edges
    (child → parent); a `contains =` that names a class already in *any* prospective
    parent's chain (the `A ↔ B` mutual-inheritance case, or self-inheritance) is
    **rejected** at `setRefClass` time by the same name-in-ancestry check, now run over
    *each* listed parent — so the multi-parent graph stays a DAG and every chain walk is
    bounded by `MAX_CHAIN_DEPTH`. Diamond inheritance (`C` contains `A` and `B`, both
    containing a common base `Z`) is fine: the DFS de-dups `Z`, so it appears once.
  - **Scope outcome.** All three shipped solidly in this PR; nothing deferred to R-27.
    The two highest-value pieces (`callSuper()` + active bindings) and multiple
    inheritance all reuse the existing chain-walk + lazy-method-rebuild machinery, so
    the change stayed contained.

- **R-27 — output-formatting functions** *(this PR)*. A **pivot off the R5/OOP lane**
  (R-24…R-26) into a fresh data/utility area: the string-formatting builtins R users
  reach for when turning numbers and vectors into human-readable text. Everything is a
  pure builtin in the shared `s-runtime` (R inherits via the tree-walker); **no grammar
  change**. The watchword is **determinism**: no clock, no locale — the default
  thousands separator is `","` and the decimal point is `"."`, fixed regardless of host
  locale, and that choice is documented so the output never surprises a CI run in a
  different locale. Five functions, all vectorized:
  - **`format(x, nsmall=, width=, justify=, big.mark=)`** — the general-purpose
    formatter. For a **numeric** vector: a supplied `nsmall` is the **decimal count** —
    it pads short values *and* rounds long ones to exactly that many places (so
    `format(3.14159, nsmall = 2)` is `"3.14"` and `format(3, nsmall = 2)` is `"3.00"`),
    while `nsmall = 0` (the default) uses R's default rendering untouched. (Real R's
    `nsmall` is a *minimum* layered on top of significant-digit rounding; this subset
    uses the simpler, fully deterministic decimal-count reading and defers the
    `scientific=`/sig-digit corner to R-28.) `big.mark` inserts a thousands separator
    into the integer part; then —
    crucially — **a numeric vector formats to a *common* width**: R right-pads every
    element to the width of the widest, so `format(c(1, 10, 100))` is
    `c("  1", " 10", "100")`. For a **character** vector, `justify` (`"left"` /
    `"right"` / `"centre"`, default `"left"`) controls padding within the field. `width`
    is the *minimum* field width; the effective width is `max(width, widest element)`.
    Returns a character vector the same length as `x`.
  - **`formatC(x, format=, digits=, width=, flag=)`** — the C-style formatter, a thin
    R-level wrapper over the same `printf` engine that powers `sprintf`. `format` is one
    of `"d"` (integer), `"f"` (fixed), `"e"` (scientific), `"g"` (shortest), `"s"`
    (string), or `"x"` (hex, integers only); `digits` is the precision; `width` the
    minimum field width; `flag` a string of `printf` flags (`"-"` left-justify, `"0"`
    zero-pad, `"+"` force a leading sign). `formatC(3.14159, format = "f", digits = 2)`
    is `"3.14"`; `formatC(42, width = 6, flag = "0")` is `"000042"`. Vectorized over `x`.
  - **`prettyNum(x, big.mark = ",")`** — insert a thousands separator into each number's
    integer part. `prettyNum(1234567, big.mark = ",")` is `"1,234,567"`. Negative
    numbers and decimal fractions are handled (the separator goes only in the integer
    part, after any sign).
  - **`toString(x, sep = ", ")`** — collapse a whole vector into a **single** string,
    joining the (character-coerced) elements with `sep`. `toString(1:3)` is `"1, 2, 3"`;
    `toString(c("a", "b"), sep = "; ")` is `"a; b"`. Length-1 result always.
  - **Vectorized `sprintf(fmt, ...)`** — R-5 shipped a scalar-recycling `sprintf`; R-27
    confirms and tests the **full vectorized recycling** contract: every `%`-conversion
    pulls its argument from the matching positional, all arguments are recycled to the
    longest length, and the result has that length. `sprintf("%d-%s", 1:2, c("a","b"))`
    is `c("1-a", "2-b")`; `sprintf("%05.2f", 3.1)` is `"03.10"`. (The recycling was
    already present in the R-5 implementation; R-27 adds the regression coverage and a
    shared `printf` core that `formatC` reuses.)
  - **Width-DoS cap (security).** A user `fmt` and the `width`/`nsmall`/`digits`
    arguments are **data**, and a crafted spec like `%999999999d` or
    `formatC(x, width = 1e9)` must not trigger a giant allocation. Every field width and
    precision is capped at **`MAX_FIELD = 1 << 20`** (1 MiB) — the same cap the R-5
    `sprintf` already enforced — and `format`/`formatC`/`prettyNum` clamp their
    `width`/`nsmall`/`digits` to that bound (oversize is a clean `BadArgs` error, never a
    panic or OOM). Because the per-field cap alone does **not** bound a *long vector ×
    wide field* (e.g. `format(rep(0, 1e7), width = 1e6)` would be ≈ 10 TB), the
    vectorized formatters additionally reject any request whose **product** — common
    width × element count (or `big.mark` length × count for `prettyNum`) — exceeds
    **`MAX_TOTAL_OUTPUT = 256 MiB`**, computed with `saturating_mul` so the check itself
    cannot overflow. All field-width arithmetic uses `saturating_*`. There is no `%n`-style
    conversion (the engine only supports the value-rendering conversions above), so there
    is no write-what-where primitive. A `%`-spec with no matching argument renders from a
    `0`/`""` default rather than panicking; a wrong-typed argument coerces (numbers via
    `as.double`, strings via `as.character`) rather than erroring.
  - **Scope outcome / deferred to R-28.** `format`, `prettyNum`, `toString`, and the
    vectorized-`sprintf` regression ship **solidly**. `formatC` ships the common
    `format`/`digits`/`width`/`flag` combinations (`"d"`/`"f"`/`"e"`/`"g"`/`"s"`/`"x"`).
    Deferred to **R-28**: the exotic `formatC` corners — `format = "g"`'s exact
    significant-digit/`%g` rounding edge cases, the `" "` and `"#"` flags, `mode=`/
    `big.mark=` on `formatC`, and scientific-notation control on `format()` (`scientific=`)
    — kept out to keep this PR a clean partial rather than a sprawling one.

- **R-28 — apply-family & grouping** *(this PR)*. A second pivot into the data/utility
  area: the **grouping and table** builtins that pair R's functional toolkit (R-10's
  `sapply`/`Reduce`/`Map`) with its matrix (R-11) and factor (R's F4) machinery.
  Everything is a pure builtin in the shared `s-runtime` (R inherits via the
  tree-walker); **no grammar change**. Four functions:
  - **`outer(X, Y, FUN = "*")`** — the *outer product* generalised to any binary
    operation. Builds the `length(X) × length(Y)` matrix whose `(i, j)` entry is
    `FUN(X[i], Y[j])`, stored **column-major** (reusing the R-11 `SValue::Matrix`
    representation), so `outer(1:3, 1:2)` is the 3×2 matrix `[[1,2,3],[2,4,6]]`
    (column-major data `c(1,2,3,2,4,6)`). `FUN` defaults to `"*"` and may be the
    string `"*"` or `"+"` (taken fast, element-by-element) **or an arbitrary
    function** — a closure, a builtin, or an R-9 lambda — invoked through
    `interp.call_value` once per `(i, j)` pair: `outer(1:2, 1:2, \(a, b) a*10 + b)`
    is `[[11,12],[21,22]]` (column-major `c(11, 21, 12, 22)`). (The `%o%` infix
    alias is **deferred to R-29** — it needs a grammar rule; the named `outer(...)`
    form covers the same ground.)
  - **`tapply(X, INDEX, FUN)`** — *table apply*: split `X` into groups by `INDEX`
    (a factor, or any vector coerced to its character labels), apply `FUN` to each
    group, and return a **named** vector — names are the **sorted unique levels**,
    in lockstep with the per-group results. `tapply(c(1,2,3,4), c("a","b","a","b"),
    sum)` is the named vector `c(a = 4, b = 6)`. Reuses the R-10 `split_fun` /
    `call_value` plumbing and the factor-level machinery (F4).
  - **`split(x, f)`** — partition `x` by the factor (or coerced vector) `f`, returning
    a **named list** (one element per level, in sorted-unique-level order, names = the
    levels): `split(1:4, c("a","b","a","b"))` is `list(a = c(1, 3), b = c(2, 4))`.
    Reuses the R-6 `SValue::List` construction.
  - **`tabulate(bin, nbins = max(bin))`** — count how many times each of `1..nbins`
    appears in the integer vector `bin`, returning an integer vector of length
    `nbins`. Values `< 1` or `> nbins` (and `NA`) are silently ignored, matching R:
    `tabulate(c(1,2,2,3,3,3))` is `c(1, 2, 3)`; `tabulate(c(2,3,5), nbins = 5)` is
    `c(0, 1, 1, 0, 1)`.
  - **Output-size caps (security).** `X`, `Y`, `bin`, `nbins` are all **data** and a
    crafted call must never trigger a giant allocation or a panic. `outer` is
    `O(length(X) · length(Y))`: the element count is computed with **`checked_mul`**
    and rejected (clean `Index` error) when it overflows or exceeds **`MAX_SEQ_LEN`**
    *before* any `Vec` is allocated. `tabulate` caps `nbins` at `MAX_SEQ_LEN` (a
    crafted `nbins = 1e18` or a `bin` containing a huge value must not allocate
    terabytes); a non-finite or negative `nbins` clamps to `0`. `tapply`/`split`
    allocate at most one slot per input element (already `MAX_SEQ_LEN`-bounded) plus
    one group per distinct level (≤ element count), so no extra cap is needed. A
    non-callable `FUN` (to `outer`/`tapply`) is a clean `NotCallable` error; an
    `INDEX`/`f` whose length differs from `X`/`x` recycles/truncates against the data
    length rather than panicking; empty groups and an empty input yield empty results,
    never an index panic.
  - **Scope outcome / deferred to R-29.** `outer` (with `"*"`, `"+"`, and an
    arbitrary function), `tapply`, `split`, and `tabulate` ship **solidly**. Deferred
    to **R-29**: the `%o%` infix alias for `outer` (needs a grammar rule); matrix
    **dimnames** carried by `outer`/`tapply` (the results are unnamed in the matrix
    dimension, named only where R-15's vector/list `names` already reach);
    `tapply` over a *multi-way* `INDEX` (a list of factors → an N-dimensional array);
    and `simplify = FALSE` (always-list `tapply`).

- **R-29 — vector set operations & ordering** *(this PR)*. A pivot into the
  **set-theory and ranking** corner of R's base toolkit: the small, pure builtins
  R users reach for when treating vectors as multisets (`union`/`intersect`/`setdiff`),
  testing membership (`is.element`), spotting repeats (`duplicated`), or assigning
  sample ranks (`rank`). All ship as pure builtins in the shared `s-runtime` (R
  inherits via the tree-walker); **no grammar change** (none is an operator). They
  reuse the existing vector machinery wholesale — `as_character` (the order-preserving
  string key used by `unique`/`%in%`), `value::index` (the 1-based gather that
  preserves element type), `value::membership` (the `%in%` engine), `combine` (for
  `c(x, y)`), and `first_positional`. Six functions:
  - **`union(x, y)`** — the distinct elements of `c(x, y)`, in **first-occurrence
    order** (R's order-preserving union, *not* a sorted set). Implemented as
    `unique(c(x, y))`: concatenate with `combine`, then keep the first sighting of
    each `as_character` key. `union(c(1,2), c(2,3))` is `c(1, 2, 3)`. Works on numeric
    and character vectors (the comparison key is the coerced character form, exactly
    as `unique`/`%in%` already do).
  - **`intersect(x, y)`** — the elements present in **both** `x` and `y`, in `x`'s
    order, **deduplicated** (each distinct value appears once). A value of `x` is kept
    iff its key is in `y`'s key-set *and* it has not already been kept.
    `intersect(c(1,2,3), c(2,3,4))` is `c(2, 3)`.
  - **`setdiff(x, y)`** — the elements of `x` **not** in `y`, deduplicated,
    order-preserving by `x`. `setdiff(c(1,2,3,4), c(2,4))` is `c(1, 3)`.
  - **`is.element(el, set)`** — exactly `el %in% set` (a vectorized logical, one entry
    per element of `el`). `is.element(2, c(1,2,3))` is `TRUE`. A thin alias over
    `value::membership` so the two stay bug-for-bug identical.
  - **`duplicated(x)`** — a logical vector, `TRUE` where an element equals one that
    appeared **earlier** (first occurrence is always `FALSE`).
    `duplicated(c(1,1,2,3,3))` is `c(FALSE, TRUE, FALSE, FALSE, TRUE)`. Uses the same
    `as_character`-key + "seen" set as `unique`, but emits the flag instead of gathering.
  - **`rank(x)`** — **sample ranks** with **average tie handling** (R's default
    `ties.method = "average"`). Each element's rank is its 1-based position in
    ascending order; tied values share the **mean** of the positions they span.
    `rank(c(3,1,2))` is `c(3, 1, 2)`; `rank(c(1,1,2))` is `c(1.5, 1.5, 3)` (the two
    1s occupy positions 1 and 2, averaging to 1.5). Numeric ranks numerically;
    character ranks lexicographically. The output is always numeric (averages are
    fractional), matching R.
  - **Output-size caps (security).** Every input is **data**, so a crafted call must
    not allocate unboundedly or panic. The set ops produce **at most** `length(x) +
    length(y)` (union) or `length(x)` (intersect/setdiff) elements — each operand is
    already `MAX_SEQ_LEN`-bounded, and `union` routes through `combine` (whose result
    is itself a vector subject to the same limit), so no result can exceed what the
    inputs already permit; no separate cap is introduced. `duplicated`/`is.element`
    emit exactly one logical per input element. `rank` allocates one `f64` per element
    and sorts the index permutation in `O(n log n)`; there is no quadratic blowup and
    no user-controlled multiplier. Empty inputs yield empty results; `NA` is treated
    as an ordinary (matchable) key via `as_character`'s `None`, never an index panic.
  - **Scope outcome / deferred to R-30.** `union`, `intersect`, `setdiff`,
    `is.element`, `duplicated`, and `rank` (average ties) ship **solidly**. Deferred
    to **R-30**: `rank`'s other `ties.method` options (`"first"`, `"min"`, `"max"`,
    `"random"`); the `incomparables =`/`fromLast =` arguments of the set ops and
    `duplicated`; `anyDuplicated`; and **multi-key `order(x, y, ...)`** (the single-key
    `order` from R-13 is unchanged). The `%o%` infix alias for `outer` carried over
    from R-28's deferral list also remains open for a later grammar pass.

- **R-30 — ordering refinements** *(this PR)*. The follow-up to R-29 that fills
  in the *ties, direction, and multi-key* corners of the ordering builtins. All
  changes are **extensions of the existing R-29/R-13 handlers** in the shared
  `s-runtime` (no new value type, no grammar change); R inherits them through the
  tree-walker. They reuse the same machinery: `as_character` (the order-and-equality
  key shared by `unique`/`%in%`/the set ops), `value::index` (the type-preserving
  1-based gather), `first_positional`, `arg.value.truthy()` (the `na.rm =` boolean
  reader pattern), and `arg.value.as_character()` (the string keyword reader the
  `factor(levels =, labels =)` builtin uses).
  - **Multi-key `order(x, y, ...)`** — sort the index permutation by the **first**
    key, breaking ties by the **second**, and so on, lexicographically; remaining
    ties keep their **original order** (a *stable* sort). The R-13 single-key form is
    the `...`-arity-1 special case, unchanged. Each key is coerced to its comparison
    form independently (numeric keys compare numerically with `NA` last, mirroring
    `na.last = TRUE`; character keys compare lexicographically), so a numeric key and
    a character key can be mixed across positions. Every key must have the **same
    length** as the first; a length mismatch is a graceful error, never a panic.
    `order(c(2,1,2), c(1,2,1))` is `c(2, 1, 3)` (the lone `1` first, then the two `2`s
    tied on both keys, kept in original order: index 1 before index 3).
  - **`rank(x, ties.method = ...)`** — the R-29 `rank` gains the **four exact** tie
    rules (the `"random"` jitter rule stays deferred, as it needs an RNG seed
    contract): `"average"` (the default, unchanged — tied values share the **mean**
    of the positions they span); `"min"` (every tie takes the **lowest** position in
    its run); `"max"` (the **highest**); `"first"` (positions assigned in **original
    order** within the run, so ties get distinct consecutive ranks). The keyword is
    read with `arg.value.as_character()`; an unrecognised method is a graceful error.
    `rank(c(1,1,2))` is `c(1.5,1.5,3)` (average), `c(1,1,3)` (min), `c(2,2,3)` (max),
    `c(1,2,3)` (first). Min/max/first are **integer-valued** but still returned as
    numeric (R's `rank` always yields a double), matching the existing return type.
  - **`duplicated(x, fromLast = TRUE)`** — the R-29 `duplicated` gains the
    direction flag. By default the **first** occurrence of each value is `FALSE` and
    later repeats are `TRUE`; with `fromLast = TRUE` the scan runs **right-to-left**,
    so the **last** occurrence is the keeper (`FALSE`) and the **earlier** repeats are
    `TRUE`. `duplicated(c(1,2,1))` is `c(FALSE, FALSE, TRUE)`;
    `duplicated(c(1,2,1), fromLast = TRUE)` is `c(TRUE, FALSE, FALSE)`. The flag is
    read with `arg.value.truthy()`.
  - **`anyDuplicated(x)`** — a **scalar integer**: the 1-based index of the **first
    element that is itself a duplicate** (i.e. the first position whose value was seen
    earlier), or `0` when `x` has no duplicates. Defined to agree exactly with
    `which(duplicated(x))[1]` (and `0L` when that is empty). `anyDuplicated(c(1,2,1))`
    is `3`; `anyDuplicated(c(1,2,3))` is `0`. Works on numeric and character vectors
    via the shared character key.
  - **Output-size caps (security).** No new user-controlled multiplier is
    introduced. `order` allocates one `usize` per element of the first key and sorts
    `O(n log n)`; the per-key length check rejects mismatched keys before any indexing.
    `rank` (all methods) allocates one `f64` per element. `duplicated` (both
    directions) and `anyDuplicated` emit/scan exactly one entry per input element.
    Every operand is already `MAX_SEQ_LEN`-bounded; outputs are bounded by the
    (already-capped) input lengths, so no separate cap is added. `NA` remains an
    ordinary matchable key (`as_character`'s `None`); empty inputs yield empty/`0`
    results; no path can index out of bounds.
  - **Scope outcome / deferred to R-31.** Multi-key `order`, `rank`'s
    `average`/`min`/`max`/`first` tie methods, `duplicated(fromLast =)`, and
    `anyDuplicated` ship **solidly**. **Deferred to R-31:** `incomparables =` on the
    set ops / `duplicated` / `anyDuplicated` (it needs `NA`-comparison plumbing — a
    way to mark certain values as "never equal to anything", which the current
    `as_character`-key path does not model); `rank`'s `"random"` tie method (needs the
    RNG-seed contract); and the `fromLast =` argument on the set ops. The `%o%` infix
    alias for `outer` remains open for a later grammar pass.

- **R-31 — set-op & ordering refinements** *(this PR)*. The follow-up to R-30 that
  lands the two deferrals which DO have clean, unambiguous semantics on the existing
  `as_character`-key path, plus the RNG-backed tie method. All three are **extensions
  of the existing R-29/R-30 handlers** in the shared `s-runtime`; no new value type
  and no new cap are introduced.
  - **`incomparables =` on `duplicated`, `anyDuplicated`, and `unique`.** The default
    is `incomparables = FALSE`, meaning "there are no incomparable values" — identical
    to the prior behaviour. A **vector** value lists the elements to treat as
    *incomparable*: a value listed there is **never considered equal to anything**
    (not even another copy of itself), so it is **never flagged as a duplicate** and
    **never removed** as one. Mechanically we coerce the `incomparables` vector to the
    same character key the builtins already use (`as_character`) and build a small
    `HashSet` of "incomparable keys"; during the dup scan, any element whose key is in
    that set short-circuits to "not a duplicate" and is **never inserted into the
    `seen` set** (so it cannot suppress a later genuine duplicate either). Worked
    examples (numeric and character keys both supported):
    `duplicated(c(1,1,2,2), incomparables = 1)` is `c(FALSE, FALSE, FALSE, TRUE)` (the
    1s are never dups; the second 2 still is); `unique(c(1,1,2,2), incomparables = 1)`
    is `c(1, 1, 2)` (both 1s kept, 2 deduped); `anyDuplicated(c(1,2,1), incomparables = 1)`
    is `0` (the only repeat is an incomparable), while `anyDuplicated(c(1,2,2),
    incomparables = 1)` is `3`.
  - **`unique(x, fromLast = TRUE)`.** R-30 added `fromLast =` to `duplicated`; R-31
    extends `unique` symmetrically. With `fromLast = TRUE` the dedup keeps the **last**
    occurrence of each distinct value (scanning right-to-left), versus the default
    first occurrence. The kept positions are gathered in **ascending index order** so
    the surviving elements stay in input order (R's behaviour). `incomparables =` and
    `fromLast =` compose: an incomparable value is kept at every one of its positions
    regardless of direction.
  - **`rank(x, ties.method = "random")`.** A run of `m` tied values is assigned the
    consecutive ranks `lo, lo+1, …, hi` (as in `"first"`), but the **assignment of
    those ranks to the tied positions is a uniform random permutation** drawn from the
    **session RNG** — the same generator seeded by `set.seed()` that the R-8
    distribution family (`runif`/`rnorm`/…) uses (`Interpreter::sample_with`). The
    permutation is a Fisher–Yates shuffle driven by `RngState::next_u32`, so the result
    is **fully reproducible** under `set.seed`: `set.seed(s); rank(x, ties.method =
    "random")` gives the same vector every run. The two 3s in `rank(c(3,1,3),
    ties.method = "random")` receive ranks `{2, 3}` in a seed-determined order while the
    lone 1 always gets rank 1. `"average"` remains the default; `"min"/"max"/"first"`
    are unchanged from R-30.
  - **Output-size caps (security).** No new user-controlled multiplier. `incomparables`
    adds one `HashSet` whose size is bounded by the (already-`MAX_SEQ_LEN`-capped)
    `incomparables` vector; membership tests are `O(1)`. `"random"` does a single
    `O(m)` Fisher–Yates pass per tie run with **bounded** RNG draws (one `next_u32`
    per swap, at most `n` total), so RNG use cannot trigger unbounded work. Named-arg
    readers reject malformed values gracefully (`Err`, never panic): `ties.method` is
    read as a character scalar (an unknown method is a `BadArgs` error), `fromLast =`
    via `truthy()` (a non-logical or `NA` is an error), and `incomparables =` is
    coerced through `as_character` (any vector is acceptable; no path indexes out of
    bounds).
  - **Scope outcome / deferred to R-32.** `incomparables =` on `duplicated` /
    `anyDuplicated` / `unique`, `unique(fromLast =)`, and `rank(ties.method =
    "random")` ship **solidly**. **Deferred to R-32:** `incomparables =` on the binary
    set ops (`union`/`intersect`/`setdiff`) — R's semantics there interact with which
    operand the incomparable belongs to and are ambiguous enough to warrant their own
    pass — and the `fromLast =` argument on those same binary set ops. The `%o%` infix
    alias for `outer` remains open for a later grammar pass.

- **R-32 — binning & cross-product utilities** *(this PR)*. A **pivot** away from
  the R-31 deferral of `incomparables=`/`fromLast=` on the binary set ops
  (`union`/`intersect`/`setdiff`): on inspection base R's `union`/`intersect`/`setdiff`
  do **not** accept those arguments at all (only the `{set,union,...}` generics and
  `duplicated`/`unique` do), so wiring them onto the binary set ops would be
  **non-faithful**. R-32 instead lands a coherent, faithful adjacent unit — the
  numeric-binning family — all in the shared `s-runtime` (R reuses them verbatim
  through the shared tree-walker). They build on the existing factor value
  (`SValue::Factor { codes, levels }`, the R-13 factor type) and the existing
  `MAX_SEQ_LEN` cap; no new value type is introduced.
  - **`findInterval(x, vec)`** — the primitive the others build on. `vec` must be
    **non-decreasing** (a sorted vector of breakpoints). For each element of `x` it
    returns the largest index `i` (1-based) such that `vec[i] <= x`, i.e. the count of
    breakpoints that do not exceed `x`: `0` when `x < vec[1]`, `length(vec)` when
    `x >= vec[length(vec)]`. Implemented as a linear scan (`vec` is assumed short and
    sorted); `NA`/non-finite `x` propagate to `NA`. Worked examples:
    `findInterval(c(0.5, 1.5, 2.5), c(1, 2, 3))` is `c(0, 1, 2)`;
    `findInterval(5, c(1, 2, 3))` is `3`.
  - **`tabulate(bin, nbins)`** — unchanged from R-28 (already shipped); listed here as
    part of the binning family. Counts integer codes `1..nbins`; codes `<= 0` or
    `> nbins` are ignored; `nbins` defaults to `max(bin)` and is capped at
    `MAX_SEQ_LEN`. `tabulate(c(1,2,2,3,5), nbins = 5)` is `c(1, 2, 1, 0, 1)`.
  - **`cut(x, breaks)`** — bin a numeric vector `x` into the intervals delimited by
    the **sorted** breakpoint vector `breaks`, returning a **`factor`** (a real
    `SValue::Factor`, so `levels()`, `as.integer()`, `as.character()`, and `table()`
    all work on the result). With `k = length(breaks)` breakpoints there are `k - 1`
    intervals; the default intervals are **right-closed** `(lo, hi]`, and the
    auto-generated level labels are exactly `"(lo,hi]"` formatted from the numeric
    breakpoints. An element that falls in no interval — `x <= breaks[1]` or
    `x > breaks[k]`, or `NA`/non-finite — maps to a `NA` factor code (printed `<NA>`),
    not to a level. `cut` is implemented **on top of `findInterval`**: the interval
    index is `findInterval(x, breaks)`, which is the 1-based level code when it lies in
    `1..k-1` and `NA` (out of range) otherwise. Worked example:
    `cut(c(1, 5, 10), breaks = c(0, 3, 6, 11))` is a factor with levels
    `c("(0,3]", "(3,6]", "(6,11]")` and values `(0,3]`, `(3,6]`, `(6,11]`;
    `cut(c(-1, 20), breaks = c(0, 3, 6, 11))` is `NA, NA` (both outside the breaks).
  - **Output-size caps (security).** No new unbounded multiplier. `cut` allocates one
    output code per input element (length already `MAX_SEQ_LEN`-bounded) and `k - 1`
    level strings (bounded by the `breaks` length, itself capped). `findInterval` is
    `O(len(x) * len(vec))` with both lengths capped. `tabulate` keeps its existing
    `nbins`-vs-`MAX_SEQ_LEN` checked guard. Named-arg readers reject malformed values
    gracefully (`Err`, never panic): `breaks` is read through `as_double` (NA/empty is
    a clean error or empty result), and the deferred options below are simply ignored
    when absent.
  - **Scope outcome / deferred to R-33.** `findInterval`, `tabulate`, and `cut`
    (default right-closed `(lo,hi]` intervals with auto-generated labels) ship
    **solidly** in R-32.

- **R-33 — `cut()` option completeness** *(this PR)*. Extends the R-32 `cut`
  handler in place (same `findInterval`-backed kernel, same factor builder) with
  the four deferred options. None of them change the default behaviour; they are
  pure refinements layered onto the existing interval scan.
  - **`labels =`** — three forms:
    - **absent / `labels = TRUE`** — the auto-generated interval strings (the
      R-32 default), `"(lo,hi]"` (or `"[lo,hi)"` when `right = FALSE`).
    - **a character vector** — used verbatim as the factor levels. Its length
      **must equal the number of intervals** (`length(breaks) - 1`); otherwise
      `cut` raises an error (`"lengths of 'breaks' and 'labels' differ"`).
      `cut(c(1,5,10), breaks=c(0,3,6,11), labels=c("lo","mid","hi"))` → a factor
      with levels `c("lo","mid","hi")`.
    - **`labels = FALSE`** — return the **integer bin codes** (a plain numeric
      vector, *not* a factor). Out-of-range / `NA` values become `NA`.
      `cut(c(1,2,3), breaks=c(0,3,6), labels=FALSE)` → `c(1,1,2)`.
  - **`right = FALSE`** — left-closed intervals `[lo, hi)` rather than the
    default right-closed `(lo, hi]`. The bin scan switches from "largest break
    `<= x`" to "number of breaks `< x`", and the auto-labels become `"[lo,hi)"`.
    `cut(c(1,3), breaks=c(0,3,6), right=FALSE)` → `1 ∈ "[0,3)"`, `3 ∈ "[3,6)"`.
  - **`include.lowest = TRUE`** — include the extreme boundary value in the
    closest interval. With `right = TRUE` (default) the **lowest** break is folded
    into the first interval (so `x == breaks[1]` lands in interval 1 instead of
    `NA`); with `right = FALSE` the **highest** break is folded into the last
    interval (so `x == breaks[k]` lands in interval `k-1`). Default `FALSE`.
    `cut(c(0,1,2), breaks=c(0,1,2), include.lowest=TRUE)` → `0` lands in the first
    interval rather than `NA`.
  - **integer `breaks` (a single number `N`)** — divide the **range of `x`** into
    `N` equal-width intervals. R's `cut.default` extends the range by 0.1 % on each
    side so the extreme data points sit strictly inside the outer bins; we
    replicate that exactly:
    ```text
      rx = range(x, na.rm = TRUE)         # = (min, max) over finite x
      dx = rx.max - rx.min
      if dx == 0:                          # degenerate (all x equal)
          dx = abs(rx.min)                 # R: abs(rx[1L]); if still 0, dx = 1
          if dx == 0: dx = 1
      lo = rx.min - dx/1000
      hi = rx.max + dx/1000
      breaks = N+1 equally spaced points from lo to hi   # N equal-width bins
    ```
    `N` is bounded by `MAX_SEQ_LEN` (a huge `N` → huge levels vector is rejected,
    not allocated) and the spacing is computed with checked/finite arithmetic so a
    degenerate range never divides by zero. `cut(0:10, breaks=5)` → a factor with
    5 levels spanning the slightly-extended `0..10` range; every value gets a
    non-`NA` bin.
  - **Security.** `N` for integer breaks is capped at `MAX_SEQ_LEN` before any
    allocation; the equal-width break vector is built with finite/checked
    arithmetic (degenerate all-equal `x` is handled without divide-by-zero);
    `labels` length validation returns a clean `Err` (never panics); `labels =
    FALSE` returns a numeric vector with no factor allocation. No new
    user-controlled multiplier beyond the existing `MAX_SEQ_LEN`-bounded input and
    break lengths.
  - **Scope outcome / deferred to R-34.** `labels =` (incl. `FALSE`),
    `right = FALSE`, `include.lowest =`, and integer `breaks` ship **solidly**.
    **Deferred to R-34:** `dig.lab =` (significant-digit control of auto-label
    formatting) and `ordered_result =` (an ordered factor result). The `%o%` infix
    alias for `outer` remains open for a later grammar pass.

- **R-34 — string utilities** *(this PR)*. An **independent string-utility family**
  (not part of the in-flight cut/set-ops chain): five base-R string builtins that
  reuse the existing string machinery shipped in R-5/R-7/R-27 — the `as_character`
  coercion, the `Option<String>` NA convention (`None` = `NA`), and the
  `SValue::Character`/`SValue::Logical(Vec<Option<bool>>)` constructors. Everything
  operates on **Unicode `char`s**, never raw byte indices, so multibyte UTF-8 input
  can never split a code point or panic.
  - **`startsWith(x, prefix)`** — a **logical** vector, `TRUE` where `x[i]` begins
    with `prefix[i]`. Vectorized and **recycled** over *both* arguments to the
    longer length (R recycles `x` and `prefix` independently). `NA` in either
    position yields `NA`. `startsWith(c("apple","banana"), "a")` → `c(TRUE, FALSE)`;
    `startsWith(c("ab","cd","ae"), "a")` → `c(TRUE, FALSE, TRUE)`;
    `startsWith(NA, "a")` → `NA`.
  - **`endsWith(x, suffix)`** — the trailing-edge analogue, same recycling and NA
    rules. `endsWith(c("file.txt","file.csv"), ".txt")` → `c(TRUE, FALSE)`.
  - **`trimws(x, which = "both")`** — strip leading and/or trailing whitespace from
    each element. `which ∈ {"both","left","right"}` (read as the second positional
    or the `which =` named arg); any other value is a clean error. Whitespace is
    R's default class `[ \t\r\n]`. `trimws("  hi  ")` → `"hi"`;
    `trimws("  hi  ", "left")` → `"hi  "`; `trimws("  hi  ", "right")` → `"  hi"`;
    `trimws(NA)` → `NA`. Char-based, UTF-8 safe.
  - **`chartr(old, new, x)`** — translate characters: each char of `x` that appears
    at position *i* of `old` becomes the char at position *i* of `new`. `old` and
    `new` are **length-one** character scalars and must have **equal `nchar`**, else
    an error (R: *"'old' and 'new' must be of the same length"*). Translation is by
    Unicode `char`, so multibyte `old`/`new`/`x` work and never panic.
    `chartr("abc", "xyz", "cab")` → `"zxy"`. Vectorized over `x`; `NA` element → `NA`.
  - **`strtoi(x, base = 10L)`** — parse each string as an integer in the given
    `base` (an integer in **2..36**, read as the second positional or `base =`),
    returning a **double** vector (this subset has no distinct integer type), with
    `NA` for anything unparseable. Semantics follow C `strtol` as R does:
    - Leading ASCII whitespace is skipped; an optional `+`/`-` sign is honored.
    - For **base 16**, an optional `0x`/`0X` prefix is accepted (e.g.
      `strtoi("0xFF", 16L)` → `255`); for other bases the digits are taken literally.
    - The **whole remaining string must be consumed** — trailing non-digit garbage
      (including trailing whitespace) yields `NA`. An empty string yields `NA`.
    - A digit outside the base's range makes the element `NA`
      (`strtoi(c("7","8"), 8L)` → `c(7, NA)`; `strtoi("z", 16L)` → `NA`).
    - A `base` outside **2..36** makes **every** element `NA` (matching base R's
      `strtol`-driven behavior — it does *not* error).
    Examples: `strtoi("FF", 16L)` → `255`; `strtoi("10", 2L)` → `2`;
    `strtoi(c("7","8"), 8L)` → `c(7, NA)`.
  - **Safety.** No raw byte indexing anywhere — `startsWith`/`endsWith` use
    `str::starts_with`/`ends_with` (code-point safe), `trimws`/`chartr` iterate
    `char`s. `strtoi` parses with checked `i64` accumulation bounded by base ≤ 36 and
    a fixed length cap, so a long all-digits string cannot overflow or hang
    (overflow → `NA`, never a panic). Recycling length is the max of the (bounded)
    input lengths; an empty input recycles to length 0.
  - **Scope outcome / deferred to R-36.** `startsWith`, `endsWith`, `trimws`,
    `chartr`, and `strtoi` (explicit bases **2..36**) ship **solidly**. **Deferred to
    R-36:** `strtoi`'s `base = 0L` auto-detection (C `strtol`'s convention where a
    `0x` prefix selects base 16 and a leading `0` selects base 8) — a self-contained
    extension of the same parser. `trimws`'s custom `whitespace =` regex argument
    (R ≥ 3.6) is likewise deferred; the default class covers the common case.

## §4 Reuse strategy

- **Lexer/parser:** the grammar-tools framework, exactly as S uses it. `r.tokens`
  / `r.grammar` compile to committed `_grammar.rs` in `r-lexer` / `r-parser`.
- **Runtime:** the `s-runtime` evaluator and `SValue` model are language-neutral
  — they walk a `GrammarASTNode` by rule name. By keeping `r.grammar`'s rule
  names identical to `s.grammar`'s, `r-runtime` can evaluate R programs through
  the same `Interpreter`. (R-3 adds the small public entry point for this.)
- **REPL:** `r-repl` mirrors `s-repl`'s single-threaded driver.

## §5 Out of scope (for now)

Pipes (`|>`) and backslash lambdas (`\(x)`). The `SValue::Environment` value,
`new.env()`, `environment()`, `ls(envir=)`, and the `envir = e` argument of
`assign`/`get`/`exists`/`rm` land in **R-22**; the `environment(f)` form,
`environment(f) <-`, `environmentName`, `globalenv()`/`emptyenv()`/`baseenv()`,
`parent.frame()`, and `is.environment()` land in **R-23**. Still out of scope:
`sys.call`/`sys.function`/`match.call` and the rest of the call-introspection
family; S4 and R6 OO (R5 reference classes land in **R-24**; single-`contains=`
inheritance, `$copy()`, `is`/`inherits` over the class chain, and
`$methods()`/`$fields()` introspection land in **R-25**, with `callSuper()`,
active bindings, and multiple inheritance (`contains = c("A", "B")`) completing
the R5 system in **R-26**); the output-formatting family (`format`, `formatC`,
`prettyNum`, `toString`, vectorized `sprintf`) lands in **R-27**, with the exotic
`formatC` corners (`format = "g"` rounding edges, the `" "`/`"#"` flags,
`scientific=`) deferred to a later formatting pass; the apply-family & grouping
builtins (`outer`, `tapply`, `split`, `tabulate`) land in **R-28**, with the `%o%`
infix alias, matrix dimnames, multi-way `tapply`, and `simplify = FALSE` deferred to
a later pass; the vector set-operation & ranking builtins (`union`, `intersect`,
`setdiff`, `is.element`, `duplicated`, `rank`) land in **R-29**; the ordering
refinements (multi-key `order(x, y, ...)`, `rank`'s `ties.method` =
`average`/`min`/`max`/`first`, `duplicated(fromLast=)`, `anyDuplicated`) land in
**R-30**; the set-op & ordering refinements (`incomparables=` on
`duplicated`/`anyDuplicated`/`unique`, `unique(fromLast=)`, and `rank`'s `"random"`
tie method) land in **R-31**; the binning & cross-product utilities (`findInterval`,
`cut` returning a factor) land in **R-32** — a pivot away from `incomparables=`/`fromLast=`
on the binary set ops (`union`/`intersect`/`setdiff`), which base R does not accept there,
making them non-faithful; `cut`'s `labels=`/`right=FALSE`/`include.lowest=` and integer
`breaks` are deferred to **R-33**; the independent string-utility family
(`startsWith`, `endsWith`, `trimws`, `chartr`, `strtoi` over explicit bases 2..36)
lands in **R-34**, with `strtoi`'s `base = 0L` auto-detection and `trimws`'s custom
`whitespace=` argument deferred to **R-36**; namespaces and `library()`
(so `baseenv()` aliases the
global env for now); the C interface; graphics. These layer on later, following
ST00.

## §6 References

Internal: [`S00-s-language.md`](S00-s-language.md),
[`ST00-r-stats-roadmap.md`](ST00-r-stats-roadmap.md), `grammar-tools`,
`r-vector` / `statistics-core`.

External:

- R. Ihaka & R. Gentleman, *R: A Language for Data Analysis and Graphics*
  (J. Computational and Graphical Statistics, 1996).
- R Core Team, *The R Language Definition*.
