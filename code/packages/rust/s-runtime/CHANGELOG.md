# Changelog

All notable changes to this project will be documented in this file.

## [0.31.0] - 2026-06-21

### Added

- **Ordered factors & `cut()` label polish (R-35)** — available to both S and R
  through the shared tree-walker.
  - **Representation**: `SValue::Factor` gains an `ordered: bool` field. `class()`
    reports `c("ordered", "factor")` when set (plain `"factor"` otherwise), and an
    ordered factor prints its `Levels:` line with `<` separators
    (`Levels: lo < mid < hi`). All prior factor constructions default
    `ordered = false`, so unordered factors are bit-for-bit unchanged.
  - **`ordered(x, levels =, labels =)`** and **`factor(x, ordered = TRUE)`** build
    an ordered factor (reusing the refactored `build_factor` helper).
    **`as.ordered(x)`** coerces (a factor flips its flag; any other vector is
    factor-encoded first). **`is.ordered(x)`** is `TRUE` iff `x` is an ordered
    factor — never errors.
  - **Ordered-factor comparison**: `<`, `<=`, `>`, `>=`, `==`, `!=` between two
    ordered factors compare **by level index** (the 1-based `code`), not by label
    string, NA-propagating. Two ordered factors with **different level sets** is a
    clean error (`"level sets of factors are different"`). Order operators on an
    *unordered* factor error (`"'<' not meaningful for factors"`); `==`/`!=` fall
    back to label comparison.
  - **`cut(..., ordered_result = TRUE)`** makes the binned factor an ordered factor
    (intervals are naturally ordered low→high). **`cut(..., dig.lab = k)`** formats
    auto-generated break labels to `k` significant digits (default **3**), e.g.
    `cut(..., breaks = c(0, 3.14159, 10), dig.lab = 2)` → levels `"(0,3.1]"`,
    `"(3.1,10]"`.
  - **Security**: ordered comparison reads the integer `codes` only — an
    out-of-range or `NA` code maps to `NA`, never panicking — and rejects differing
    level sets before any compare. `dig.lab` is **clamped to `1..=22`** before
    formatting (a malformed / non-positive value falls back to the default 3), so
    no caller-controlled value can drive an unbounded format width or a panic. No
    new unbounded multiplier; the level vector is still bounded by the
    `MAX_SEQ_LEN`-bounded break count.
  - **Deferred to R-39**: the S3 `Ops.ordered` group-generic *dispatch* surface and
    order statistics on ordered factors (`sort`/`max`/`min`/`range` by level order).

## [0.30.0] - 2026-06-21

### Added

- **Matrix cross products (R-36)** — `crossprod` and `tcrossprod`, available to
  both S and R through the shared tree-walker. An independent matrix-algebra item
  (not part of the binning/set-op chains), defined **entirely in terms of the
  existing R-11 `t()` transpose and `%*%` matrix product** — no new linear algebra,
  no new value type.
  - **`crossprod(x, y)`** = `t(x) %*% y`; **`crossprod(x)`** (one argument) =
    `t(x) %*% x` (the unscaled Gram matrix `X'X`). The second argument defaults to
    the first.
  - **`tcrossprod(x, y)`** = `x %*% t(y)`; **`tcrossprod(x)`** (one argument) =
    `x %*% t(x)` (`XX'`). The "t" prefix transposes the *second* operand.
  - Worked column-major example: `A = matrix(c(1,2,3,4), nrow=2)` gives
    `crossprod(A)` = `[[5,11],[11,25]]` and `tcrossprod(A)` = `[[10,14],[14,20]]`.
    Non-square `B = matrix(1:6, nrow=2)` (2×3) gives `crossprod(B)` 3×3 and
    `tcrossprod(B)` 2×2.
  - **Reuse / security**: the implementation calls the public `t()` builtin (`b_t`,
    via a `transpose_value` helper) and the evaluator's `matrix_multiply` (the `%*%`
    handler, newly exposed `pub(crate)`). It therefore inherits that handler's
    already-reviewed `MAX_SEQ_LEN` allocation guard on the `nrow*ncol` result (no
    unchecked multiply → OOM), the `"non-conformable arguments"` error raised before
    any indexing, the column-major `array_runtime` fast path, and NA propagation.
    The new surface is just the two argument-shuffling wrappers; a bare vector flows
    through `%*%`'s existing vector promotion.

## [0.29.0] - 2026-06-21

### Added

- **`cut()` option completeness (R-33)** — the four options deferred from R-32,
  available to both S and R through the shared tree-walker. All extend the R-32
  `cut` handler in place (same interval scan, same `SValue::Factor` builder); no
  new value type, no new cap.
  - **`labels =`** — `labels = FALSE` returns the **integer bin codes** as a plain
    numeric vector (no factor allocation). A character `labels` vector is used
    verbatim as the levels and must have length `length(breaks) - 1` (else a clean
    `BadArgs` error). Absent / `labels = TRUE` keeps the auto-generated interval
    labels. `strip_wrappers` peels `Classed`/`Named`/`Attributed` so we can tell
    `FALSE`/`TRUE` from a character vector.
  - **`right = FALSE`** — left-closed `[lo, hi)` intervals (the index becomes
    `#{breaks <= x}` rather than `#{breaks < x}`); auto-labels become `"[lo,hi)"`
    via the new `cut_interval_label` helper. The default `right = TRUE` scan now
    also honours the `(lo,hi]` boundary convention exactly (an `x` equal to an
    interior break lands in the lower interval).
  - **`include.lowest = TRUE`** — folds the single extreme boundary value (the
    lowest break for `right = TRUE`, the highest for `right = FALSE`) into the
    adjacent interval so it bins instead of going `NA`.
  - **integer `breaks`** — a single number `N` requests `N` equal-width bins over
    the range of `x`, extended by `dx/1000` each side (`dx = max - min`; degenerate
    `dx == 0` falls back to `abs(min)`, then `1`) — see `equal_width_breaks`.
  - The new `cut_code` helper centralises the per-value interval logic shared by the
    factor and `labels = FALSE` paths.
  - **Security**: `N` is capped at `MAX_SEQ_LEN` **before** any vector is built;
    the equal-width breaks use finite/checked arithmetic (no divide-by-zero on a
    degenerate range); the `labels` length check returns `Err`, never panics. No new
    user-controlled multiplier.
  - **Deferred to R-34**: `dig.lab=` and `ordered_result=`.

## [0.28.0] - 2026-06-21

### Added

- **Binning & cross-product utilities (R-32)** — the numeric-binning family,
  available to both S and R through the shared tree-walker. A pivot away from the
  thin R-31 deferral of `incomparables=`/`fromLast=` on the binary set ops
  (`union`/`intersect`/`setdiff`): base R does not accept those arguments there, so
  implementing them would be non-faithful. All build on existing machinery
  (`as_double`, `na_real`/`is_na_real`, `first_positional`, and the existing
  `SValue::Factor { codes, levels }` constructor) — no new value type, no new cap.
  - **`findInterval(x, vec)`** — for each element of the non-decreasing breakpoint
    vector `vec`, the 1-based index of the last break not exceeding `x`: `0` below
    the first, `length(vec)` at/above the last; `NA`/non-finite `x` → `NA`.
    `findInterval(c(0.5, 1.5, 2.5), c(1, 2, 3))` → `c(0, 1, 2)`;
    `findInterval(5, c(1, 2, 3))` → `3`. A linear scan over the (short, sorted)
    breaks; a non-finite/NA/out-of-order break stops the count, so it never indexes
    out of bounds.
  - **`cut(x, breaks)`** — bins `x` into the `k-1` right-closed `(lo,hi]` intervals
    of the sorted `breaks`, returning a real **`Factor`** whose `levels` are the
    auto-generated `"(lo,hi]"` labels. Built directly on `findInterval`: the
    interval index is the 1-based level code when it lies in `1..=k-1`; boundary
    indices `0`/`k` and `NA` `x` map to a `<NA>` code. So `levels()`,
    `as.integer()`, `as.character()`, and `nlevels()` are all factor-aware on the
    result. `cut(c(1, 5, 10), breaks = c(0, 3, 6, 11))` → a factor with levels
    `c("(0,3]", "(3,6]", "(6,11]")` and values `(0,3]`, `(3,6]`, `(6,11]`;
    `cut(c(-1, 20), breaks = c(0, 3, 6, 11))` → both `NA`. `saturating_sub` guards
    the interval count when fewer than two breaks are supplied (→ all `NA`).
  - **Security**: no new user-controlled multiplier. `cut` allocates one code per
    input element (length already `MAX_SEQ_LEN`-bounded) and `k-1` level strings
    (bounded by the capped `breaks` length); `findInterval` is `O(len(x)·len(vec))`
    with both lengths capped; the existing `tabulate` `nbins`-vs-`MAX_SEQ_LEN`
    checked guard is unchanged. The `vec=`/`breaks=` readers (shared `second_arg`
    helper) return a clean `BadArgs` error when absent; no path indexes out of
    bounds.
  - **Deferred to R-33**: `cut`'s `labels=` (custom labels), `right=FALSE`
    (left-closed `[lo,hi)`), `include.lowest=`, and integer `breaks` (equal-width
    bin count).

## [0.27.0] - 2026-06-21

### Added

- **Set-op & ordering refinements (R-31)** — the two R-30 deferrals that have
  clean, unambiguous semantics on the existing `as_character`-key path, plus the
  RNG-backed tie method. All extensions of the R-29/R-30 dedup & ranking handlers
  (no new value type, no grammar change), available to both S and R.
  - **`incomparables =` on `duplicated`, `anyDuplicated`, and `unique`** — the
    default `FALSE` means "no incomparables" (prior behaviour); a **vector** lists
    the elements to treat as *incomparable*. An incomparable value is **never equal
    to anything**, so it is never flagged as a duplicate and never removed as one,
    and it is never recorded in the `seen` set (so it cannot suppress a later
    genuine duplicate either). Modelled on the same `as_character` key the set-op
    family already uses, via a shared `incomparables_keys` reader, so numeric and
    character incomparables both work. `duplicated(c(1,1,2,2), incomparables = 1)` →
    `c(FALSE, FALSE, FALSE, TRUE)`; `unique(c(1,1,2,2), incomparables = 1)` →
    `c(1, 1, 2)`; `anyDuplicated(c(1,2,1), incomparables = 1)` → `0`, while
    `anyDuplicated(c(1,2,2), incomparables = 1)` → `3`.
  - **`unique(x, fromLast = TRUE)`** — keeps the **last** occurrence of each
    distinct value (right-to-left scan), gathered in ascending index order so the
    survivors stay in input order. Mirrors R-30's `duplicated(fromLast =)`, and
    composes with `incomparables =`. `unique(c(1,2,1), fromLast = TRUE)` →
    `c(2, 1)`.
  - **`rank(x, ties.method = "random")`** — scores a tie run as the consecutive
    ranks `lo..=hi` (like `"first"`) but assigns them to the tied positions in a
    **uniform random order** drawn from the session RNG (the `set.seed`-seeded
    generator shared with the R-8 distribution family, via
    `Interpreter::sample_with` + `RngState::next_u32`). A Fisher–Yates shuffle makes
    the result **fully reproducible** under `set.seed`. `"average"` remains the
    default; `"min"/"max"/"first"` are unchanged. Numeric and character vectors.

### Security

- **Bounded work, graceful parsing.** `incomparables =` adds one `HashSet` whose
  size is bounded by the already-`MAX_SEQ_LEN`-capped `incomparables` vector;
  membership tests are `O(1)`. `"random"` does one `O(m)` Fisher–Yates pass per
  tie run with at most one `next_u32` per swap (≤ `n` draws total), so RNG use
  cannot trigger unbounded work. Named-arg readers reject malformed values
  gracefully (`Err`, never panic): `ties.method` as a character scalar (unknown →
  `BadArgs`), `fromLast =` via `truthy()` (non-logical / `NA` → error), and
  `incomparables =` coerced through the total `as_character`; no path indexes out
  of bounds.

### Deferred to R-32

- `incomparables =` / `fromLast =` on the binary set ops
  (`union` / `intersect` / `setdiff`) — their R semantics are ambiguous enough to
  warrant a separate pass.

## [0.26.0] - 2026-06-21

### Added

- **Ordering refinements (R-30)** — extensions of the R-29/R-13 ordering builtins,
  available to both S and R via the shared tree-walker; no new value type, no
  grammar change. They reuse the existing `as_character` key, `value::index`,
  `truthy()` (the `na.rm =` boolean reader), and `as_character()` string-keyword
  reads (the `factor(levels =, labels =)` pattern).
  - **Multi-key `order(x, y, ...)`** — sort the index permutation lexicographically
    by the first key, breaking ties by the second, then the next, …; indices still
    tied after every key keep their **original order** (a stable sort). Each key is
    coerced independently — numeric keys compare numerically with `NA` sorting last
    (`na.last = TRUE`), character keys lexicographically — so numeric and character
    keys may be mixed across positions. All keys must share the first key's length
    (a mismatch is a graceful error, never an out-of-bounds index). The single-key
    R-13 form is the arity-1 special case, unchanged.
    `order(c(2,1,2), c(1,2,1))` → `c(2, 1, 3)`.
  - **`rank(x, ties.method = ...)`** — adds `"min"` (every tie takes the lowest
    position in its run), `"max"` (the highest), and `"first"` (consecutive ranks
    in original order, so ties get distinct ranks) alongside the default
    `"average"` (unchanged). An unrecognised method is a graceful error. The result
    stays numeric. `rank(c(1,1,2))` → `c(1.5,1.5,3)` / `c(1,1,3)` / `c(2,2,3)` /
    `c(1,2,3)` for average / min / max / first.
  - **`duplicated(x, fromLast = TRUE)`** — runs the duplicate scan right-to-left,
    so the **last** occurrence of each value is the keeper (`FALSE`) and earlier
    repeats are flagged. Default (`fromLast = FALSE`) unchanged.
    `duplicated(c(1,2,1), fromLast = TRUE)` → `c(TRUE, FALSE, FALSE)`.
  - **`anyDuplicated(x)`** — the 1-based index of the first duplicated element
    (the first position whose value appeared earlier), or `0` when there are none;
    agrees with `which(duplicated(x))[1]`. `anyDuplicated(c(1,2,1))` → `3`;
    `anyDuplicated(c(1,2,3))` → `0`. Numeric and character vectors.

### Security

- **Output-size caps.** No new user-controlled multiplier. `order` allocates one
  `usize` per element of the first key and sorts `O(n log n)`; the per-key length
  check rejects mismatched keys before any indexing. `rank` (all methods) and
  `duplicated` (both directions) emit exactly one entry per input element, and
  `anyDuplicated` scans once. Every operand is already `MAX_SEQ_LEN`-bounded, so
  outputs are bounded by the (capped) input lengths; `NA` remains an ordinary
  matchable key; empty inputs yield empty/`0` results; no path can index out of
  bounds.

### Deferred to R-31

- `incomparables =` on the set ops / `duplicated` / `anyDuplicated` (needs
  `NA`-comparison plumbing — a way to mark values "never equal to anything", which
  the `as_character`-key path does not model), the `fromLast =` set-op argument,
  and `rank`'s `"random"` tie method (needs an RNG-seed contract).

### Tests

- New `r30_ordering` module: 17 tests covering multi-key `order` (tie-break,
  stable fallback, character + mixed keys, length-mismatch error), `rank`
  ties.method (average/min/max/first, numeric + character, unknown-method error),
  `duplicated(fromLast=)`, and `anyDuplicated` (numeric + character + empty).

## [0.25.0] - 2026-06-21

### Added

- **Vector set operations & ordering (R-29)** — pure builtins available to both
  S and R via the shared tree-walker; no grammar change. They treat vectors as
  multisets and reuse the existing `as_character` key, `value::index`,
  `value::membership`, and `combine` machinery. Numeric and character vectors are
  handled uniformly (the comparison key is the coerced character form, exactly as
  `unique`/`%in%` already do).
  - **`union(x, y)`** — the distinct elements of `c(x, y)` in **first-occurrence
    order** (i.e. `unique(c(x, y))`, *not* a sorted set). `union(c(1,2), c(2,3))`
    → `c(1, 2, 3)`.
  - **`intersect(x, y)`** — the elements present in both, in `x`'s order,
    deduplicated. `intersect(c(1,2,3), c(2,3,4))` → `c(2, 3)`.
  - **`setdiff(x, y)`** — the elements of `x` not in `y`, deduplicated,
    order-preserving by `x`. `setdiff(c(1,2,3,4), c(2,4))` → `c(1, 3)`.
  - **`is.element(el, set)`** — the function spelling of `el %in% set` (a
    vectorized logical, one entry per element of `el`); a thin alias over
    `value::membership`. `is.element(2, c(1,2,3))` → `TRUE`.
  - **`duplicated(x)`** — a logical vector, `TRUE` where an element repeats one
    seen earlier (first occurrence is `FALSE`). `duplicated(c(1,1,2,3,3))` →
    `c(FALSE, TRUE, FALSE, FALSE, TRUE)`.
  - **`rank(x)`** — sample ranks with **average** tie handling (R's default
    `ties.method = "average"`): each element's rank is its 1-based position in
    ascending order, and a run of equal values shares the mean of the positions
    it spans. `rank(c(3,1,2))` → `c(3, 1, 2)`; `rank(c(1,1,2))` → `c(1.5, 1.5, 3)`.
    Numeric ranks numerically, character lexicographically; result is always
    numeric, `NA` sorts last.

### Security

- **Output-size caps.** Every input is data. The set ops produce at most
  `length(x) + length(y)` (union) or `length(x)` (intersect/setdiff) elements —
  each operand already `MAX_SEQ_LEN`-bounded and `union` routes through `combine`
  (itself bounded), so no result can exceed what the inputs already permit; no
  fresh cap is introduced. `duplicated`/`is.element` emit one logical per input
  element; `rank` allocates one `f64` per element and sorts in `O(n log n)` with
  no user-controlled multiplier. Empty inputs yield empty results and `NA` is an
  ordinary matchable key, never an index panic.

### Tests

- A `r29_set_ops` module: union/intersect/setdiff (numeric + character;
  first-occurrence / x-order / dedup), is.element (scalar + vectorized),
  duplicated, rank (no-ties / average-ties / character), and the empty /
  all-removed degenerate edges.

### Deferred to R-30

- `rank`'s other `ties.method` options (`"first"`/`"min"`/`"max"`/`"random"`);
  the `incomparables=`/`fromLast=` set-op arguments; `anyDuplicated`; and
  multi-key `order(x, y, …)` (single-key `order` from R-13 is unchanged).

## [0.24.0] - 2026-06-21

### Added

- **Apply-family & grouping builtins (R-28)** — pure builtins available to both
  S and R via the shared tree-walker; no grammar change. Pair the R-10
  functional toolkit with R-11 matrices, R-6 lists, and factors.
  - **`outer(X, Y, FUN = "*")`** — the outer product generalised to any binary
    operation: the `length(X) × length(Y)` column-major matrix of
    `FUN(X[i], Y[j])`. `FUN` defaults to `"*"` and may be `"*"`/`"+"` (taken on a
    fast numeric path) **or any callable** (closure / builtin / `\(a, b) …`
    lambda), invoked once per `(i, j)` pair. `outer(1:3, 1:2)` → the 3×2 product
    matrix; `outer(1:2, 1:2, \(a, b) a*10 + b)` → `[[11,12],[21,22]]`.
  - **`tapply(X, INDEX, FUN)`** — group `X` by `INDEX` (a factor or any vector
    coerced to character labels), apply `FUN` per group, return a **named**
    vector (names = sorted unique levels). `tapply(c(1,2,3,4),
    c("a","b","a","b"), sum)` → `c(a = 4, b = 6)`.
  - **`split(x, f)`** — partition `x` by `f` into a **named list** (one element
    per level). `split(1:4, c("a","b","a","b"))` → `list(a = c(1,3), b = c(2,4))`.
  - **`tabulate(bin, nbins = max(bin))`** — count occurrences of each of
    `1..nbins` in `bin`; values `< 1`, `> nbins`, and `NA` are ignored.
    `tabulate(c(2,3,5), nbins = 5)` → `c(0,1,1,0,1)`.
- **`names()` on a named list** now returns its element names (an unset element
  name renders as `""`, a wholly-unnamed list as `NULL`) — correct R behaviour
  that `split()`'s named-list result relies on.

### Security

- **Output-size caps.** `outer` computes its element count `nrow*ncol` with
  `checked_mul` and rejects an overflowing or `> MAX_SEQ_LEN` product with a
  clean `Index` error **before** any allocation, so a crafted
  `outer(1:1e6, 1:1e6)` cannot OOM. `tabulate` clamps `nbins` to
  `[0, MAX_SEQ_LEN]` (non-finite/negative → `0`; over-large → clean error), so a
  crafted `nbins = 1e18` cannot allocate terabytes. A non-callable `FUN`
  (to `outer`/`tapply`) is a clean `NotCallable` error; an `INDEX`/`f` whose
  length differs from the data recycles/truncates against the data length rather
  than panicking; empty groups and empty inputs yield empty results.

### Deferred to R-29

- The `%o%` infix alias for `outer` (needs a grammar rule), matrix dimnames
  carried by `outer`/`tapply`, multi-way `tapply` (a list of factors →
  N-dimensional array), and `simplify = FALSE`.

## [0.23.0] - 2026-06-20

### Added

- **Output-formatting builtins (R-27)** — a family of pure, deterministic
  (locale-free) string formatters, available to both S and R via the shared
  tree-walker; no grammar change.
  - **`format(x, nsmall=, width=, justify=, big.mark=)`** — the general
    formatter. A **numeric** vector formats to a *common* width (every element is
    right-justified to the width of the widest, so columns line up); a
    **character** vector pads to `max(width, widest)` honouring `justify`
    (`"left"` / `"right"` / `"centre"`). A supplied `nsmall` is the decimal count
    (`format(3, nsmall = 2)` → `"3.00"`); `big.mark` inserts a thousands
    separator. Returns a character vector the length of `x`.
  - **`formatC(x, format=, digits=, width=, flag=)`** — C-style: `format` one of
    `"d"`/`"f"`/`"e"`/`"g"`/`"s"`/`"x"`, `digits` precision, `width` minimum field
    width, `flag` a string of `printf` flags (`"-"` left, `"0"` zero-pad, `"+"`
    force sign). Reuses the `sprintf` conversion engine.
  - **`prettyNum(x, big.mark = ",")`** — insert a thousands separator (sign
    preserved, never grouped).
  - **`toString(x, sep = ", ")`** — collapse a whole vector to one string.
  - **`sprintf`** stays fully vectorized (it was since R-5); R-27 adds regression
    coverage and shares the `printf` core with `formatC`.

### Security

- The `MAX_FIELD = 1 MiB` field-width/precision cap that `sprintf` enforced is
  **hoisted to module scope** and now shared by `format`/`formatC`: a crafted
  `fmt` or a huge `width=`/`nsmall=`/`digits=` is clamped (oversize literal
  `sprintf` widths are a clean error), so no caller-controlled value can trigger
  a giant allocation. Because the per-field cap does not bound a *long vector ×
  wide field*, `format`/`formatC`/`prettyNum` also enforce a
  **`MAX_TOTAL_OUTPUT = 256 MiB`** budget on the *product* (common width ×
  element count), computed with `saturating_mul`. All width arithmetic uses
  `saturating_*`. There is no `%n`-style conversion, so no write-what-where
  primitive exists.

### Deferred

- Exotic `formatC` corners — `format = "g"` exact-rounding edge cases, the `" "`
  and `"#"` flags, and `format()`'s `scientific=` — are left for **R-28**.

## [0.22.0] - 2026-06-20

### Added

- **R5 `callSuper()`, active bindings, and multiple inheritance (R-26)** —
  completes the R5 reference-class system on R-24/R-25's `src/refclass.rs`
  generator/instance model; grammar-free.
  - **`callSuper(...)`** — inside an overriding method, invoke the **same-named**
    parent method. `rebuild_method` now materialises each instance-bound method
    inside a thin **super-context** scope (parent = the instance) that records two
    private markers: `.refSuperGens` (the parent generators of the class that
    *defined* the running method version, where same-name resolution restarts) and
    `.refMethodName`. `callSuper` is a new lazy special form: it reads those markers
    from the current env, resolves the method starting at the super generators,
    re-homes it onto the instance with a *fresh* super-context one level further up
    (so chained `callSuper()` walks `C→B→A`), and applies it to the forwarded args.
    **Past-the-root** (no parent definition) returns `NULL` — no recursion, no panic.
  - **Active bindings** — a `fields = list(ab = function(v) …)` entry whose value is
    a closure is an **active binding**: reading `obj$ab` **calls** it as a nullary
    getter (`missing(v)` TRUE); `obj$ab <- val` **calls** it as a setter (`v` bound,
    `missing(v)` FALSE). The function is re-homed onto each instance (so it reads
    sibling fields and writes them with `<<-`); `$new`/`$copy` install it per
    instance (the copy gets its *own* re-homed binding, never the source's). New
    `missing(x)` special form reports whether a formal was supplied in the current
    frame. Getter/setter run through the depth-bounded call path, so a
    self-referential getter hits `MAX_EVAL_DEPTH` cleanly rather than borrow-panicking
    or hanging.
  - **Multiple inheritance** — `contains = c("A", "B")`. The single `.refParent`
    link becomes a `.refParents` **list**; a new `linearization` does a left-to-right
    **depth-first** pre-order walk (de-duping shared ancestors, so a diamond's base
    appears once) that all the effective-field/method/class-chain computations and
    the cycle check now consult. Method/field precedence is most-derived-first,
    left-to-right (own ⊳ A ⊳ B ⊳ ancestors). C3 is **not** implemented (documented
    simple DFS). The name-in-ancestry cycle check runs over **every** listed parent,
    so the multi-parent graph stays a DAG; all walks bounded by `MAX_CHAIN_DEPTH`.
  - **Rc-cycle / re-entrancy safety.** All new edges (`.refParents`, the
    super-context's parent-to-instance link, an active binding's instance-homed
    closure) are DAG/lazy edges; no new at-rest instance⇄method or generator cycle.
    16 new R-syntax tests (in `r-runtime`).

## [0.21.0] - 2026-06-20

### Added

- **R5 inheritance, `$copy()`, and introspection (R-25)** — builds directly on
  R-24's `src/refclass.rs` generator/instance model; grammar-free.
  - **`setRefClass("Sub", contains = "Base", fields = …, methods = …)`** — single
    inheritance. The subclass generator carries a new `.refParent` link to the
    base generator (a strict child→parent **DAG** edge). The **effective field
    set** is the union *base ∪ sub* in base-first declaration order; the
    **effective method set** is *base ∪ sub* with a **sub method overriding** a
    same-named base method. `instantiate` now binds the effective fields and
    carries the effective method list, so an inherited base method is callable on
    a `Sub` and a `Sub` method reads/writes base fields (one flat instance frame).
    `contains =` accepts the parent **generator value** or a length-1 **character**
    class name (resolved as a variable). A cyclic `contains =` (`A`⊃`B`⊃`A`, or
    self-inheritance) is **rejected** at `setRefClass` time; a non-generator /
    undefined parent is a clean error.
  - **`obj$copy()`** — a **deep** value-copy returning a NEW, independent instance
    (each effective field copied by value; a nested instance is aliased, not
    recursed — bounded). Contrast `b <- a`, which still **aliases** (R-24
    reference semantics preserved). Charged against `MAX_ENVIRONMENTS` like `$new`.
  - **`is(obj, "Base")` / `inherits(obj, "Base")` / `class(obj)`** — an R5
    instance's class vector is now its inheritance chain
    `c("Sub", "Base", …, "envRefClass", "environment")`, computed by walking
    `.refGenerator` → `.refParent`. New `is` builtin registered.
  - **`generator$fields()` / `generator$methods()`** — the **sorted** effective
    field / method names (including inherited), reached via nullary reference-method
    markers routed through `apply`/`call_value`, mirroring the `$new` marker.
  - **Rc-cycle safety.** The only new edges (subclass→parent generator,
    instance→generator) are DAG edges, never cycles; `$copy()` builds a sibling
    instance (no recursion through nested instances). The instance⇄method
    lazy-rebuild discipline is inherited verbatim. All chain walks are bounded by
    `MAX_CHAIN_DEPTH`.
  - Defers multiple inheritance and active bindings to **R-26**.

## [0.20.0] - 2026-06-20

### Added

- **R5 reference classes (R-24)** — `setRefClass`, generators, and instances,
  living in the shared runtime so both S and R inherit them. Grammar-free (`$`
  already existed). New `src/refclass.rs` module.
  - **`setRefClass("Name", fields = list(x = "numeric", …), methods = list(m = function(…) …))`**
    → a **generator** (a first-class environment carrying `.refClassName`,
    `.refFields`, and `.refMethods`). A *lazy special form*: it evaluates the
    `fields`/`methods` arguments in the current scope, so each method
    `function(...)` closes over where the class was defined. Field *type* strings
    are recorded but **not enforced** in this subset. Two `fields` shapes are
    accepted: a named `list(x = "numeric", …)` and a bare character vector
    `c("x", "y")`.
  - **`generator$new(x = …, …)`** → an **instance**: a fresh child environment
    binding each declared field (to the matching `new()` argument, or `NULL` when
    omitted), `.self` (the instance, for `.self$method()`), and `.refMethods`. An
    unknown `new()` argument is a clean error.
  - **`obj$field`** reads a field (frame-local), **`obj$field <- v`** writes it
    **in place by reference** (the `$<-` lvalue path now accepts an environment
    target), and **`obj$method(args)`** rebuilds a fresh instance-bound closure on
    access and applies it — so `field <<- value` (R-21 super-assignment) and
    `.self$field <- v` mutate the live instance.
  - **Reference (alias) semantics** — `b <- a` shares state (both reference the
    same instance scope), unlike R's copy-on-modify; two separate `$new` instances
    are independent.
  - **Rc-cycle safety (instance⇄method).** The naïve encoding (method closures
    stored in the instance, closing over the instance) is a strong, uncollectable
    cycle. Broken **by construction**: the instance stores only fields, `.self`,
    and `.refMethods` (whose closures close over the *generator*, not the
    instance); the instance-bound closure is materialised lazily per `obj$method`
    access and never stored. The lone remaining `.self` strong self-reference is
    the documented, `MAX_ENVIRONMENTS`-bounded R-22 value-binding self-cycle.
  - New **`env::lookup_local`** (frame-local read, no parent walk) so an instance
    — a child of its generator — is never misclassified as a generator.
  - Malformed `setRefClass`/`$new` arguments (non-character name, non-list
    `fields`/`methods`, non-function method, unknown field, `$<-` on a non-env)
    are all clean errors, never panics.
  - Each reified generator/instance environment counts against `MAX_ENVIRONMENTS`.
  - **Deferred to R-25:** inheritance (`contains =`), `$copy()`, active bindings,
    `$methods()`/`$fields()` introspection.

## [0.19.0] - 2026-06-20

### Added

- **Closure environments & call-frame reflection (R-23)** — builds on the R-22
  `SValue::Environment` value, the `MAX_ENVIRONMENTS` cap, and the R-20 call
  stack. Grammar-free; lives in the shared runtime so both S and R inherit it.
  - **`environment(f)`** — the environment a closure **captured** at definition,
    read straight from `Closure { env, .. }` and handed back as an
    `SValue::Environment`. A non-closure argument (a builtin, a number, …) returns
    `NULL`, matching R (`environment(sum)`). `environment()` (no argument)
    continues to return the current scope. Reifying either env counts against
    `MAX_ENVIRONMENTS`.
  - **`environment(f) <- e`** — a new `environment<-` builtin wired through the
    R-15/R-16 replacement-function lvalue path. Returns a *fresh* `Closure` with
    its `env` swapped (closures are immutable values; the variable is rebound, as
    in R), so the re-homed closure's free variables now resolve from `e`'s chain.
    A non-closure target or non-environment value is a clean error.
  - **`environmentName(e)`** — `"R_GlobalEnv"` / `"R_EmptyEnv"` / `""` decided by
    `Rc` **pointer** identity (`env::same_env`, `Rc::ptr_eq`) against the
    interpreter's long-lived `global`/`empty` handles — O(1), re-entrancy-safe
    (never borrows the `RefCell`).
  - **`globalenv()` / `emptyenv()` / `baseenv()`** — the well-known environments
    as values. `globalenv()`/`baseenv()` both return the session global env
    (no separate base namespace yet, so `baseenv()` aliases global — documented);
    `emptyenv()` returns a new long-lived **empty** root (`Scope::empty`: no
    parent, no bindings). All three hand back the *same* `Rc` each call and
    allocate nothing, so none counts against `MAX_ENVIRONMENTS`.
  - **`parent.frame(n = 1)`** — the **caller's** environment `n` frames up. The
    call stack now records a `(closure, caller_env)` `CallFrame` per call (R-20
    recorded only the closure, for `Recall`); `call_closure` pushes the env the
    call expression was evaluated in, and `parent.frame` reads it. **Clamps** to
    the global env past the bottom of the live stack (and at top level), so it
    never indexes out of bounds or panics; `n` must be a positive, finite whole
    number, else a clean `BadArgs`.
  - **`is.environment(x)`** — scalar type predicate.
  - **Ownership model unchanged.** The caller env on the call stack is dropped
    when its frame is popped (the RAII `CallFrameGuard`), so it never outlives the
    call; the captured-env exposure is the same strong-`Rc` situation as
    `environment()`, bounded by the same `MAX_ENVIRONMENTS` cap; the `Weak` parent
    link still prevents any parent-chain cycle. `env.rs` gains `Scope::empty` and
    `same_env`.

## [0.18.0] - 2026-06-20

### Added

- **First-class environment values (R-22)** — reifies the shared scope chain as a
  value, completing the piece R-21 deferred. Grammar-free (the names lex as
  ordinary dotted identifiers).
  - **`SValue::Environment(Env)`** — boxes the shared `Rc<RefCell<Scope>>` handle,
    so a scope can be passed, stored, and **mutated by reference**. Implicit class
    and `type_name` are `"environment"`; `length` is 1; it is not an atomic vector
    (the coercions reject it cleanly through their existing fallbacks, never a
    panic). Prints as the **stable** placeholder `<environment>`, deliberately not
    R's real heap address, so test output is deterministic.
  - **Rc-cycle ownership model (the central correctness concern).** `Scope::parent`
    changes from a strong `Rc` to a **`Weak<RefCell<Scope>>`**. An environment
    value owns the only strong `Rc` to its scope; the global env and each live
    call frame are held strongly by the interpreter / native call stack; parents
    are only ever *referenced*, never *owned*, by their children. So **no cycle
    through the parent chain is constructible** — the parent relation stays a
    finite acyclic list, and all chain walks (`lookup`/`exists`/`super_assign`,
    iterative — a deep chain cannot overflow the native stack) terminate. A
    `Weak` parent that fails to upgrade (dropped frame) is treated as "no parent".
  - **Residual value-binding cycle — bounded, not collected.** The `Weak` parent
    breaks only parent-edge cycles. A cycle can still form through a *value
    binding*, since an environment value is a strong `Rc`:
    `assign("self", e, envir = e)` stores a strong `Rc`-to-`e` inside `e`, which
    `Rc` cannot reclaim without a tracing GC (R has one; we do not). This is a
    documented limitation, **bounded** by a new per-session `MAX_ENVIRONMENTS`
    (2^20) cap on the number of environments `new.env()`/`environment()` may
    reify, so a crafted loop building cyclic environments fails closed at a clean
    error instead of exhausting memory. Documented inline in `env.rs` and the
    `SValue::Environment` doc-comment. (The first draft over-claimed "no strong-Rc
    cycle is constructible"; corrected here after the security review.)
  - **`new.env()`** — fresh environment whose parent is the caller's scope,
    returned as a value; two calls are independent. **`environment()`** — the
    current environment as a value. **`ls([envir = e])` / `ls(e)`** — the names
    bound directly in a frame (`env::names_in`), **sorted**.
  - **`envir = e` on `assign`/`get`/`exists`/`rm`/`local`** — operate on the passed
    environment value rather than the current scope (replaces R-21's runtime
    rejection). `assign`/`rm` act on the target frame directly; `get`/`exists` walk
    that environment's chain. A non-environment `envir` is a clean `BadArgs` error.
  - **By-reference mutation** — passing an environment to a function and binding a
    name inside it is visible to the caller; the defining difference from R's
    otherwise copy-on-modify value semantics.
  - `env.rs` gains `names_in`; the parent link is now `Weak` (with a `parent_of`
    upgrade helper).

### Deferred to R-23

- `environment(f)` (a closure's captured environment — needs to reach into the
  `Closure { env, .. }` payload) and `environmentName`. Passing an argument to
  `environment()` is a clean error pointing at R-23, not a wrong answer.

## [0.17.0] - 2026-06-20

### Added

- **Environments & scoping (R-21)** — R's environment-model core subset, living
  in the shared runtime so both S and R inherit it. Grammar-free: the `<<-`
  super-assign token already lexed and parsed.
  - **`<<-` super-assignment** — a new `env::super_assign` walks the chain of
    *enclosing* environments (skipping the current frame) and rebinds the
    **nearest** existing binding of the name; if none exists, it creates the
    binding in the **global** environment. This is what makes the counter-closure
    idiom work — `make <- function() { n <- 0; function() { n <<- n + 1; n } }`
    mutates the captured `n` rather than shadowing it. The walk is iterative (no
    native-stack recursion) over the finite, acyclic scope list, so it always
    terminates; a non-name target (`x[i] <<- v`) is a clean error.
  - **`local({ ... })`** — evaluates a block in a fresh child environment and
    returns its value; bindings made inside do not leak
    (`local({ x <- 5; x * 2 })` → `10`, `x` unbound after).
  - **`assign(x, value)` / `get(x)` / `exists(x)` / `rm(x)`** — by-name binding
    ops against the **current** environment, implemented as lazy special forms
    (intercepted at the call site like `switch`/`tryCatch`, since they need the
    live scope). The name is a length-one character (a variable holding the name
    works too); `get` of an unbound name errors, `exists` searches the whole
    chain, `rm` deletes from the current frame.
  - `env.rs` gains `exists` and `remove` helpers alongside `super_assign`.

### Deferred to R-22

- First-class environment **values**: `new.env()`, `environment()` /
  `environment(f)`, and the `envir = e` argument of
  `assign`/`get`/`exists`/`rm`/`local` (they need a new `SValue::Environment`
  variant and a fresh round of Rc-cycle / leak analysis). The `envir` argument is
  rejected today with a clear "deferred to R-22" error rather than silently
  ignored.

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
