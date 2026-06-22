# Changelog

All notable changes to this project will be documented in this file.

## [0.31.0] - 2026-06-21

### Added (via the shared `s-runtime`)

- **R-38 — `kronecker(X, Y)` (Kronecker product)**, reached through ordinary R
  syntax. The block-outer product: for an `m×n` `X` and a `p×q` `Y`,
  `kronecker(X, Y)` is the `(m·p)×(n·q)` matrix whose block `(i, j)` is the
  scalar `X[i, j]` times the whole of `Y`, with
  `result[(i-1)·p + k, (j-1)·q + l] = X[i, j] · Y[k, l]` (column-major).
  - `dim(kronecker(matrix(c(1,2,3,4), nrow=2), matrix(c(0,1,1,0), nrow=2)))` is
    `c(4, 4)`; `kronecker(matrix(5), matrix(c(1,2,3,4), nrow=2))` is `5·Y` (2×2);
    a 2×3 ⊗ 1×2 gives a 2×6 matrix. The result is a real matrix —
    `dim()`/`nrow()`/`ncol()` work and it composes with `%*%`.
  - **Security**: the result is quadratic in the inputs, so the row count `m·p`,
    column count `n·q`, and their product are each formed with `checked_mul` and
    bounded by the existing `MAX_SEQ_LEN` cap before allocating — an over-large
    Kronecker product raises a clean "result too large" error rather than OOMing.
    Degenerate `0×n` / `m×0` inputs give an empty result with the right zero
    dimension and never index out of bounds.
  - **Deferred to R-40**: the R `%x%` infix operator (`X %x% Y`) needs
    lexer/grammar work; this item ships the `kronecker(X, Y)` function form only.

## [0.30.0] - 2026-06-21

### Added (via the shared `s-runtime`)

- **R-35 — ordered factors & `cut()` label polish**, reached through ordinary R
  syntax. Completes the R-33 deferral of `ordered_result =` / `dig.lab =` and adds
  the ordered-factor family.
  - **`ordered(x, levels =, labels =)`** / **`factor(x, ordered = TRUE)`** build an
    ordered factor; **`as.ordered(x)`** coerces; **`is.ordered(x)`** tests for it.
    `class(ordered(c("a","b")))` is `c("ordered", "factor")`.
  - **Ordered comparison by level index**: with
    `f <- ordered(c("lo","hi","mid"), levels = c("lo","mid","hi"))`,
    `f[1] < f[2]` (lo < hi) is `TRUE` and `f[2] < f[3]` (hi < mid) is `FALSE` —
    comparison is by the level position, not the label string. All six relational
    operators are supported; an `NA` code yields `NA`; comparing ordered factors
    with different level sets is an error.
  - **`cut(..., ordered_result = TRUE)`** returns an ordered factor (its bins
    compare by interval order); **`cut(..., dig.lab = k)`** formats break labels to
    `k` significant digits (default 3), e.g.
    `levels(cut(c(1.23456, 5.6789), breaks = c(0, 3.14159, 10), dig.lab = 2))` →
    `c("(0,3.1]", "(3.1,10]")`. (`ordered_result` is an R-only spelling — the S
    lexer reads `_` as assignment.)
  - **Security**: ordered comparison works on integer codes only (out-of-range / NA
    → NA, never a panic) and rejects differing level sets; `dig.lab` is clamped to
    `1..=22` before formatting, so no extreme value can over-allocate or panic. No
    new unbounded multiplier.
  - **Deferred to R-39**: `Ops.ordered` group-generic dispatch and ordered-factor
    `sort`/`max`/`min`/`range`.

## [0.29.0] - 2026-06-21

### Added (via the shared `s-runtime`)

- **R-36 — matrix cross products**: `crossprod` and `tcrossprod`, reached through
  ordinary R syntax. An independent matrix-algebra item, defined **entirely in
  terms of the existing R-11 `t()` transpose and `%*%` matrix product** — no new
  linear algebra, no new value type, no grammar change.
  - **`crossprod(x, y)`** = `t(x) %*% y`; **`crossprod(x)`** = `t(x) %*% x`
    (the Gram matrix `X'X`). **`tcrossprod(x, y)`** = `x %*% t(y)`;
    **`tcrossprod(x)`** = `x %*% t(x)` (`XX'`). The second argument defaults to the
    first.
  - `crossprod(matrix(c(1,2,3,4), nrow=2))` → `[[5,11],[11,25]]`;
    `tcrossprod(...)` of the same → `[[10,14],[14,20]]`. Non-square
    `B = matrix(1:6, nrow=2)`: `dim(crossprod(B))` is `c(3,3)`,
    `dim(tcrossprod(B))` is `c(2,2)`. A non-conformable pair (e.g.
    `crossprod(A, matrix(1:6, nrow=3))`) raises the same `"non-conformable
    arguments"` error `%*%` raises.
  - **Security**: no new user-controlled multiplier — the impl reuses the `%*%`
    handler's existing `MAX_SEQ_LEN` allocation guard and conformability check, so
    there is no unchecked `nrow*ncol` multiply and no out-of-bounds path; the new
    code is just two argument-shuffling wrappers.

## [0.28.0] - 2026-06-21

### Added (via the shared `s-runtime`)

- **R-33 — `cut()` option completeness**: the four options deferred from R-32,
  reached through ordinary R syntax. All extend the R-32 `cut` handler in place
  (same `findInterval`-backed scan, same factor builder); no new value type, no
  grammar change.
  - **`labels =`** — `labels = FALSE` returns the **integer bin codes** as a plain
    numeric vector (NOT a factor): `cut(c(1, 2, 5), breaks = c(0, 3, 6), labels =
    FALSE)` → `c(1, 1, 2)`, `class(...)` is `"numeric"`. A character `labels`
    vector is used verbatim as the factor levels and **must** have length
    `length(breaks) - 1`, else a clean error (`lengths of 'breaks' and 'labels'
    differ`): `cut(c(1, 5, 10), breaks = c(0, 3, 6, 11), labels = c("lo", "mid",
    "hi"))` → a factor with levels `c("lo", "mid", "hi")`. Absent / `labels = TRUE`
    keeps the auto-generated interval labels.
  - **`right = FALSE`** — left-closed `[lo, hi)` intervals instead of the default
    right-closed `(lo, hi]`; auto-labels become `"[lo,hi)"`. `cut(c(1, 3), breaks =
    c(0, 3, 6), right = FALSE)` → `1 ∈ [0,3)`, `3 ∈ [3,6)`. (The interval scan also
    now honours the boundary convention exactly: under the default `right = TRUE`,
    `x` equal to an interior break lands in the *lower* `(lo,hi]` interval.)
  - **`include.lowest = TRUE`** — fold the extreme break into the adjacent interval:
    the lowest break (`right = TRUE`) or the highest (`right = FALSE`), so that
    boundary value bins instead of going `NA`. `cut(c(0, 1, 2), breaks = c(0, 1, 2),
    include.lowest = TRUE)` → `0` lands in the first interval.
  - **integer `breaks`** — a single number `N` requests `N` equal-width bins over
    the range of `x`, with the range extended by `dx/1000` on each side
    (`dx = max - min`; a degenerate all-equal `x` falls back to `abs(min)`, then
    `1`). `cut(0:10, breaks = 5)` → a factor with 5 levels covering the extended
    `0..10` range; every value gets a non-`NA` bin. (Note: `cut(x, breaks = c(5))`
    is now this equal-width form, matching base R, rather than "fewer than two
    breaks → all `NA`".)
  - **Security**: `N` is capped at `MAX_SEQ_LEN` **before** any break/level vector
    is built (a huge `N` errors, not allocates); the equal-width breaks use
    finite/checked arithmetic so a degenerate range never divides by zero; the
    `labels` length check returns a clean `Err` (never panics); `labels = FALSE`
    allocates no factor. No new user-controlled multiplier.
  - **Deferred to R-34**: `dig.lab=` (auto-label significant digits) and
    `ordered_result=` (an ordered factor result).

## [0.27.0] - 2026-06-21

### Added (via the shared `s-runtime`)

- **R-32 — binning & cross-product utilities**: the numeric-binning family,
  reached through ordinary R syntax. A pivot away from the thin R-31 deferral of
  `incomparables=`/`fromLast=` on the binary set ops (`union`/`intersect`/`setdiff`)
  — base R does not accept those arguments there, so implementing them would be
  non-faithful. All build on the existing factor value and `MAX_SEQ_LEN` cap; no new
  value type, no grammar change.
  - **`findInterval(x, vec)`** — 1-based index of the last break in the
    non-decreasing `vec` not exceeding each `x`; `0` below the first,
    `length(vec)` at/above the last; `NA`/non-finite `x` → `NA`.
    `findInterval(c(0.5, 1.5, 2.5), c(1, 2, 3))` → `c(0, 1, 2)`;
    `findInterval(5, c(1, 2, 3))` → `3`.
  - **`cut(x, breaks)`** — bins `x` into the right-closed `(lo,hi]` intervals of
    the sorted `breaks`, returning a real **factor**. `class(cut(...))` is
    `"factor"`, `levels()` are the `"(lo,hi]"` labels, and `as.character()` /
    `as.integer()` / `nlevels()` all see through to it. Values outside all breaks
    (or `NA`) become `NA` factor codes. `cut(c(1, 5, 10), breaks = c(0, 3, 6, 11))`
    → a factor with levels `c("(0,3]", "(3,6]", "(6,11]")` and values `(0,3]`,
    `(3,6]`, `(6,11]`; `cut(c(-1, 20), breaks = c(0, 3, 6, 11))` → both `NA`.
  - **Security**: no new user-controlled multiplier; `cut` allocations are bounded
    by the already-capped input/breaks lengths; `findInterval` is `O(len(x)·len(vec))`
    with both capped; `tabulate`'s `nbins` cap is unchanged; the `vec=`/`breaks=`
    readers reject a missing operand with a clean error (never panic); no path
    indexes out of bounds.
  - **Deferred to R-33**: `cut`'s `labels=`, `right=FALSE`, `include.lowest=`, and
    integer `breaks`.

## [0.26.0] - 2026-06-21

### Added (via the shared `s-runtime`)

- **R-31 — set-op & ordering refinements**: the R-30 deferrals with clean
  semantics on the shared `as_character`-key path, plus the RNG-backed tie method.
  All extensions of the R-29/R-30 dedup & ranking handlers (no grammar change),
  reached through ordinary R syntax, numeric- and character-aware.
  - **`incomparables =` on `duplicated` / `anyDuplicated` / `unique`** — default
    `FALSE` means "no incomparables"; a vector lists elements that are **never
    equal to anything**, so they are never flagged/removed as duplicates (and never
    suppress a later real one). `duplicated(c(1,1,2,2), incomparables = 1)` →
    `c(FALSE, FALSE, FALSE, TRUE)`; `unique(c(1,1,2,2), incomparables = 1)` →
    `c(1, 1, 2)`; `anyDuplicated(c(1,2,1), incomparables = 1)` → `0`;
    `anyDuplicated(c(1,2,2), incomparables = 1)` → `3`.
  - **`unique(x, fromLast = TRUE)`** — keeps the **last** occurrence of each value,
    in input order. Mirrors R-30's `duplicated(fromLast =)`; composes with
    `incomparables =`. `unique(c(1,2,1), fromLast = TRUE)` → `c(2, 1)`.
  - **`rank(x, ties.method = "random")`** — tied positions get consecutive ranks in
    a uniform random order from the `set.seed`-seeded session RNG (a Fisher–Yates
    shuffle), so `set.seed(s); rank(x, ties.method = "random")` is reproducible.
    `"average"` stays the default; `"min"/"max"/"first"` unchanged.
  - **Security**: no new user-controlled multiplier; `incomparables` adds one
    bounded `HashSet`; `"random"` draws ≤ `n` `u32`s total; named-arg readers
    reject malformed `ties.method` / `fromLast =` / `incomparables =` gracefully
    (`Err`, never panic); no path indexes out of bounds.
  - 4 new R-syntax tests (numeric + character; seed-reproducibility for `"random"`).
  - **Deferred to R-32**: `incomparables =` / `fromLast =` on the binary set ops
    (`union` / `intersect` / `setdiff`) — ambiguous R semantics, own pass.

## [0.25.0] - 2026-06-21

### Added (via the shared `s-runtime`)

- **R-30 — ordering refinements**: the follow-up to R-29, filling in the ties,
  direction, and multi-key corners of the ordering builtins. All extensions of
  the existing R-29/R-13 handlers in the shared `s-runtime` (no grammar change),
  reached through ordinary R syntax, numeric- and character-aware.
  - **Multi-key `order(x, y, ...)`** — sorts the index permutation lexicographically
    by the first key, breaking ties by the next, with remaining ties kept in
    original order (stable). Keys coerced independently (numeric: `NA` last;
    character: lexicographic), so they may be mixed; all keys must share the first
    key's length (mismatch → graceful error). The R-13 single-key form is unchanged.
    `order(c(2,1,2), c(1,2,1))` → `c(2, 1, 3)`.
  - **`rank(x, ties.method = ...)`** — `"min"`, `"max"`, and `"first"` join the
    default `"average"`. `rank(c(1,1,2))` → `c(1.5,1.5,3)` / `c(1,1,3)` / `c(2,2,3)`
    / `c(1,2,3)`. Result stays numeric; unknown method → graceful error.
  - **`duplicated(x, fromLast = TRUE)`** — right-to-left scan keeps the **last**
    occurrence; earlier repeats are flagged. Default unchanged.
    `duplicated(c(1,2,1), fromLast = TRUE)` → `c(TRUE, FALSE, FALSE)`.
  - **`anyDuplicated(x)`** — the 1-based index of the first duplicated element, or
    `0` if none. `anyDuplicated(c(1,2,1))` → `3`; `anyDuplicated(c(1,2,3))` → `0`.
  - **Security**: no new user-controlled multiplier; outputs bounded by the
    already-`MAX_SEQ_LEN`-capped inputs; the per-key length check guards `order`
    against out-of-bounds indexing; `NA` stays an ordinary matchable key; empty
    inputs yield empty/`0`.
  - 5 new R-syntax tests (numeric + character).
  - **Deferred to R-31**: `incomparables=` on the set ops / `duplicated` /
    `anyDuplicated` (needs `NA`-comparison plumbing), the `fromLast=` set-op
    argument, and `rank`'s `"random"` tie method.

## [0.24.0] - 2026-06-21

### Added (via the shared `s-runtime`)

- **R-29 — vector set operations & ordering**: the set-theory and ranking corner
  of R's base toolkit. All pure builtins in the shared `s-runtime` (no grammar
  change), reached through ordinary R syntax, numeric- and character-aware.
  - **`union(x, y)`** — `union(c(1,2), c(2,3))` → `c(1, 2, 3)` (distinct elements
    of `c(x, y)`, first-occurrence order).
  - **`intersect(x, y)`** — `intersect(c(1,2,3), c(2,3,4))` → `c(2, 3)` (elements
    in both, in `x`'s order, deduplicated).
  - **`setdiff(x, y)`** — `setdiff(c(1,2,3,4), c(2,4))` → `c(1, 3)` (elements of
    `x` not in `y`, deduplicated).
  - **`is.element(el, set)`** — `is.element(2, c(1,2,3))` → `TRUE` (the function
    spelling of `el %in% set`, vectorized over `el`).
  - **`duplicated(x)`** — `duplicated(c(1,1,2,3,3))` →
    `c(FALSE, TRUE, FALSE, FALSE, TRUE)` (TRUE where an element repeats an earlier
    one).
  - **`rank(x)`** — sample ranks with **average** tie handling (R's default):
    `rank(c(3,1,2))` → `c(3, 1, 2)`; `rank(c(1,1,2))` → `c(1.5, 1.5, 3)`. Numeric
    ranks numerically, character lexicographically; result is always numeric.
  - **Security**: outputs are bounded by the inputs (union ≤ `|x|+|y|`, others ≤
    `|x|`, each already `MAX_SEQ_LEN`-bounded), so no fresh cap is needed;
    `rank` is `O(n log n)` with one `f64` per element and no user multiplier;
    empty inputs yield empty results, `NA` is an ordinary matchable key.
  - 5 new R-syntax tests (numeric + character).
  - **Deferred to R-30**: `rank`'s other `ties.method` options, the
    `incomparables=`/`fromLast=` set-op arguments, `anyDuplicated`, and multi-key
    `order`.

## [0.23.0] - 2026-06-21

### Added (via the shared `s-runtime`)

- **R-28 — apply-family & grouping**: a second pivot into the data/utility area,
  the grouping and table builtins that pair R's functional toolkit (R-10) with
  matrices (R-11) and factors. All pure builtins in the shared `s-runtime` (no
  grammar change), reached through ordinary R syntax.
  - **`outer(X, Y, FUN = "*")`** — `outer(1:3, 1:2)` → the 3×2 column-major
    product matrix `c(1,2,3,2,4,6)`; `outer(1:2, 1:2, "+")` → the sums;
    `outer(1:2, 1:2, \(a, b) a*10 + b)` → `c(11,21,12,22)` (an arbitrary
    function, called per `(i, j)` pair).
  - **`tapply(X, INDEX, FUN)`** — `tapply(c(1,2,3,4), c("a","b","a","b"), sum)`
    → the named vector `c(a = 4, b = 6)`; a factor `INDEX` works too.
  - **`split(x, f)`** — `split(1:4, c("a","b","a","b"))` →
    `list(a = c(1,3), b = c(2,4))` (a named list, one element per level).
  - **`tabulate(bin, nbins = max(bin))`** — `tabulate(c(1,2,2,3,3,3))` →
    `c(1,2,3)`; `tabulate(c(2,3,5), nbins = 5)` → `c(0,1,1,0,1)`.
  - **`names()`** now reports a named list's element names (so
    `names(split(...))` returns the levels rather than `NULL`).
  - **Security**: `outer` guards `nrow*ncol` with `checked_mul` against
    `MAX_SEQ_LEN` *before* allocating (a crafted `outer(1:1e6, 1:1e6)` is a clean
    error, not an OOM); `tabulate` clamps `nbins` to `[0, MAX_SEQ_LEN]`. A
    non-callable `FUN`, a length-mismatched `INDEX`, and empty inputs are all
    clean errors / empty results, never panics.
  - 8 new R-syntax tests.
  - **Deferred to R-29**: the `%o%` infix alias (needs a grammar rule), matrix
    dimnames, multi-way `tapply`, and `simplify = FALSE`.

## [0.22.0] - 2026-06-20

### Added (via the shared `s-runtime`)

- **R-27 — output-formatting functions**: a pivot off the R5/OOP lane into a
  fresh data/utility area. All pure builtins in the shared `s-runtime` (no
  grammar change), reached through ordinary R syntax. Deterministic, locale-free
  output (fixed `","` thousands separator and `"."` decimal point).
  - **`format`** — `format(3.14159, nsmall = 2)` → `"3.14"`;
    `format(42, width = 5)` → `"   42"`; `format(c(1, 10, 100))` →
    `c("  1", " 10", "100")` (a numeric vector pads to a common width);
    `format(c("a","bb"), justify = "right")` → `c(" a", "bb")`;
    `format(1234567, big.mark = ",")` → `"1,234,567"`.
  - **`formatC`** — `formatC(3.14159, format = "f", digits = 2)` → `"3.14"`;
    `formatC(42, width = 6, flag = "0")` → `"000042"`; `formatC(255, format =
    "x")` → `"ff"`; vectorized over `x`.
  - **`prettyNum`** — `prettyNum(1234567, big.mark = ",")` → `"1,234,567"`.
  - **`toString`** — `toString(1:3)` → `"1, 2, 3"`;
    `toString(c("a","b"), sep = "; ")` → `"a; b"`.
  - **`sprintf`** (vectorized) — `sprintf("%d-%s", 1:2, c("a","b"))` →
    `c("1-a", "2-b")`; `sprintf("%05.2f", 3.1)` → `"03.10"`. Already vectorized
    since R-5; R-27 adds the regression coverage.
  - **Security**: all field widths/precisions are capped at 1 MiB
    (`MAX_FIELD`), so a crafted `fmt` or a huge `width=`/`nsmall=`/`digits=`
    cannot trigger a giant allocation; the vectorized formatters additionally
    enforce a 256 MiB total-output budget (width × vector length) so a long
    vector formatted to a wide field is a clean error, not an OOM.
  - 13 new R-syntax tests.
  - **Deferred to R-28**: exotic `formatC` corners (`format = "g"` rounding,
    the `" "`/`"#"` flags, `scientific=`).

## [0.21.0] - 2026-06-20

### Added (via the shared `s-runtime`)

- **R-26 — R5 `callSuper()`, active bindings, and multiple inheritance**:
  completes the R5 system through the shared evaluator (all logic in
  `s-runtime/src/refclass.rs`; no grammar change). In R syntax:
  - **`callSuper()`** — an overriding method re-uses its parent's same-named method:
    `Sub <- setRefClass("Sub", contains = "Base", methods = list(describe =
    function() paste(callSuper(), "sub")))` → `Sub$new()$describe()` is
    `"base sub"`. Chains across levels (`C→B→A`), forwards args, runs against the
    instance (can read/write fields), and returns `NULL` past the root.
  - **Active bindings** — a function-valued field is a getter/setter:
    `Temp <- setRefClass("Temp", fields = list(celsius = "numeric", fahrenheit =
    function(v) { if (missing(v)) celsius*9/5+32 else celsius <<- (v-32)*5/9 }))`
    → `t$fahrenheit` computes `212`; `t$fahrenheit <- 32` sets `t$celsius` to `0`.
    Inherited and `$copy()`-independent.
  - **Multiple inheritance** — `contains = c("A", "B")`: a `C` unions A's and B's
    fields and methods (`o$fa()`, `o$fb()`), with left-to-right precedence;
    `is`/`inherits`/`class` see every base; diamonds de-dup the shared ancestor;
    mutual-inheritance cycles are rejected.
  - 16 new R-syntax tests.

## [0.20.0] - 2026-06-20

### Added (via the shared `s-runtime`)

- **R-25 — R5 inheritance, `$copy()`, and `is`/`inherits` introspection**:
  reaches R unchanged through the shared evaluator (all logic lives in
  `s-runtime/src/refclass.rs`; no grammar change). In R syntax:
  - **`Sub <- setRefClass("Sub", contains = "Base", fields = …, methods = …)`** —
    single inheritance. A `Sub` instance has the union of base and sub fields;
    inherited base methods are callable on it; a sub method may read/write base
    fields; a sub method overrides a same-named base method. `contains =` takes the
    parent generator value or a class-name string. Cyclic / unknown `contains =`
    is a clean error.
  - **`b <- a$copy()`** — a deep, independent copy: `b$x <- 9` leaves `a$x`
    untouched, whereas `d <- a; d$x <- 7` aliases (`a$x` becomes 7).
  - **`is(s, "Base")` / `inherits(s, "Base")` / `class(s)`** — walk the R5 class
    chain `c("Sub", "Base", "envRefClass", "environment")`.
  - **`Sub$fields()` / `Sub$methods()`** — sorted field / method names, including
    inherited ones.
  - 20 new R-syntax tests plus an R-24 regression test.
  - Defers multiple inheritance + active bindings to **R-26**.

## [0.19.0] - 2026-06-20

### Added (via the shared `s-runtime`)

- **R-24 — R5 reference classes (`setRefClass`)**: reaches R unchanged through
  the shared evaluator (everything lives in `s-runtime` — `$` already existed, so
  no grammar change). In R syntax:
  - **`setRefClass("Acc", fields = list(total = "numeric"), methods = list(add = function(x) { total <<- total + x }, get = function() total))`**
    → a **generator**; **`Acc$new(total = 0)`** → an **instance** (an environment
    holding the fields).
  - **`a$add(5); a$add(3); a$total`** → `8` and **`a$get()`** → `8`: a method's
    body sees the fields directly and writes them back with `<<-`.
  - **`a$total <- 100; a$total`** → `100`: field write **by reference**.
  - **Reference semantics**: `b <- a; b$add(1); a$total` reflects b's change (the
    two names share one instance — unlike R's copy-on-modify). Two `$new`
    instances are independent.
  - **`.self`** is bound in the instance, so a method can call a sibling as
    `.self$other()` and write a field as `.self$field <- v`.
  - Field type strings are **not enforced** in this subset; a class may have no
    fields and/or no methods. Malformed `setRefClass`/`$new` arguments are clean
    errors, never panics.
  - See `s-runtime` 0.20.0 for the instance⇄method Rc-cycle handling (broken by
    construction — method closures close over the *generator*, instance-bound
    closures are rebuilt lazily per access and never stored) and the deferral of
    inheritance / `$copy()` / active bindings / introspection to R-25.

## [0.18.0] - 2026-06-20

### Added (via the shared `s-runtime`)

- **R-23 — closure environments & call-frame reflection**: reaches R unchanged
  through the shared evaluator (everything lives in `s-runtime` — no grammar
  change). In R syntax:
  - **`environment(f)`** — a closure's captured (defining) env. For a top-level
    closure that is the global env, so `environmentName(environment(f))` is
    `"R_GlobalEnv"` and `is.environment(environment(f))` is `TRUE`. A non-closure
    argument returns `NULL` (R's `environment(sum)`).
  - **`environment(f) <- e`** — re-home a closure: after it, a free variable in
    `f`'s body resolves from `e`'s chain (`assign("k", 99, envir = e);
    environment(f) <- e; f()` → `99`). A non-environment value is a clean error.
  - **`environmentName(e)`** — `"R_GlobalEnv"` / `"R_EmptyEnv"` / `""`.
  - **`globalenv()` / `emptyenv()` / `baseenv()`** — the well-known environments
    as values (`baseenv()` aliases the global env in this runtime — no separate
    base namespace yet).
  - **`parent.frame(n = 1)`** — the **caller's** environment: `g <- function()
    get("x", envir = parent.frame()); f <- function() { x <- 42; g() }; f()` →
    `42`. `parent.frame(2)` reaches the caller's caller. At top level, or for `n`
    past the bottom of the stack, it **clamps** to the global env (never panics);
    a non-positive `n` is a clean error.
  - **`is.environment(x)`** — `TRUE` for an environment, `FALSE` otherwise.
  - See `s-runtime` 0.19.0 for the (unchanged) Rc-cycle ownership model: the
    caller env on the call stack is dropped when its frame is popped, and the
    captured-env exposure is bounded by `MAX_ENVIRONMENTS`.

## [0.17.0] - 2026-06-20

### Added (via the shared `s-runtime`)

- **R-22 — first-class environment values**: reaches R unchanged through the
  shared evaluator (the `SValue::Environment` variant, the `Weak` parent link, and
  all new forms live in `s-runtime` — no grammar change). In R syntax:
  - **`e <- new.env(); assign("x", 5, envir = e); get("x", envir = e)`** → `5`.
    `exists("x", envir = e)` → `TRUE` (missing → `FALSE`); `rm("x", envir = e)`
    then `exists` → `FALSE`.
  - **By-reference mutation**: `f <- function(env) assign("x", 42, envir = env);
    f(e); get("x", envir = e)` → `42` — the binding `f` made in `e` is visible to
    the caller (the defining difference from R's copy-on-modify semantics).
  - **`ls(e)` / `ls(envir = e)`** lists the env's own names, **sorted**. Two
    `new.env()` calls are independent.
  - **`environment()`** returns the current environment; an environment prints as
    the stable placeholder `<environment>` and has class `"environment"`. A
    non-environment `envir =` is a clean error.
  - Deferred to **R-23**: `environment(f)` (a closure's captured environment) and
    `environmentName`. See `s-runtime` 0.18.0 for the Rc-cycle ownership model
    (the parent link is `Weak`, so an env-holding-env cannot leak).

## [0.16.0] - 2026-06-20

### Added (via the shared `s-runtime`)

- **R-21 — environments & scoping (core subset)**: R's environment model reaches
  R unchanged through the shared evaluator (the scope chain and the new forms all
  live in `s-runtime` — no grammar change; the `<<-`/`->>` tokens already lex and
  parse). In R syntax:
  - **`local({ x <- 5; x * 2 })`** → `10`, and `x` is unbound afterward (locals
    don't leak).
  - **`<<-` super-assignment** rebinds the nearest *enclosing* binding, else
    creates one globally: `f <- function() { y <<- 99 }; f(); y` → `99`. The
    counter idiom `make_counter <- function() { n <- 0; function() { n <<- n + 1;
    n } }` advances across calls. The R-only right form `->>` (`7 ->> z`) behaves
    identically.
  - **`assign("q", 7); get("q")`** → `7`; **`exists("zzz")`** → `FALSE`,
    `exists("mean")` → `TRUE`; **`rm("d")`** removes a binding from the current
    scope.
  - Deferred to **R-22** (first-class environment values): `new.env()`,
    `environment()`, and the `envir = e` argument (rejected today with a clear
    error). See `s-runtime` 0.17.0.

## [0.15.0] - 2026-06-20

### Added (via the shared `s-runtime`)

- **R-20 — functional helpers**: `Find`, `Position`, `Negate`,
  `Reduce(..., accumulate = TRUE)`, and `Recall` reach R unchanged through the
  shared evaluator (they are plain `s-runtime` builtins — no grammar change).
  In R syntax: `Find(\(x) x > 2, 1:5)` → `3`, `Position(\(x) x > 2, 1:5)` → `3`
  (`NULL` when nothing matches); `Negate(is.na)(NA)` → `FALSE` and
  `Negate(\(x) x > 0)(5)` → `FALSE`; `Reduce(\(a, b) a + b, 1:4,
  accumulate = TRUE)` → `c(1, 3, 6, 10)` (with an init,
  `Reduce(\(a, b) a + b, 1:3, 10, accumulate = TRUE)` → `c(10, 11, 13, 16)`);
  and `Recall` drives anonymous recursion —
  `(\(n) if (n <= 1) 1 else n * Recall(n - 1))(5)` → `120`. All take the function
  by name (`f =`), so they compose with the pipe (`1:5 |> Find(f = \(x) x > 2)`).
  See `s-runtime` 0.16.0.

## [0.14.0] - 2026-06-19

### Added (via the shared `s-runtime`)

- **R-19 — empty-arm `switch()` fall-through**: `switch("a", a = , b = "hit")`
  now returns `"hit"` in R syntax too. R-18 deferred this because the shared S/R
  grammar had no empty named-argument production; R-19 extends `r.grammar`'s `arg`
  rule to `arg = NAME EQ [expr] | expr` (mirroring the S change) and regenerates
  `r-parser`'s embedded `_grammar.rs`. Fall-through chains across several empty
  arms (`a = , b = , c = "z"` → `"z"`); a matched empty arm with nothing
  non-empty after it yields `NULL`. An empty arg in an ordinary call is an
  eval-time error, matching R. See `s-runtime` 0.15.0.

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
