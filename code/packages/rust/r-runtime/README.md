# R Runtime

Evaluates R programs — by **reusing the shared S tree-walker**.

## What it does

R is "an implementation of the S language", and `r.grammar` was written to use
the same rule names as `s.grammar`. The `s-runtime` evaluator walks a
`GrammarASTNode` tree purely by rule name, so it is language-neutral. This crate
therefore has almost no logic of its own: it parses R with `r-parser` and hands
the tree to the S `Interpreter` via its public `eval_program` entry point.

```text
R source ──▶ r_parser::try_parse_r ──▶ GrammarASTNode
                                            │
                       s_runtime::Interpreter::eval_program
                                            │
                                            ▼
                                         Outcome
```

The value model, recycling, NA semantics, S3 dispatch, factors, data frames,
matrices, named vectors, and every built-in are exactly those of `s-runtime` — R
gets them for free. The R-specific surface (the `=` / `->>` assignment operators
and the typed-`NA` constants) is handled in the shared evaluator.

Recent additions reach R unchanged through this reuse: **named vectors (R-15)** —
`c(a = 1, b = 2)` attaches names, `names(x)` / `names(x) <- value` /
`setNames(x, nm)` get and set them, `x["b"]` indexes by name, and a named vector
prints names above values instead of the `[i]` prefix. **General attributes
(R-16)** — `attr(x, which)` / `attr(x, which) <- value` (`NULL` removes),
`attributes(x)` / `attributes(x) <- list(...)`, and `structure(x, ...)` — also
reach R unchanged; the special attributes stay consistent (`attr(x, "names")` ==
`names(x)`, `attr(x, "class")` == `class(x)`, `attr(x, "dim")` == `dim(x)`).
**`do.call` + named-list polish (R-17)** — `do.call(what, args)` calls `what` (a
function, or a string naming one) with the elements of the list `args` spread as
positional and named arguments (`do.call(paste, list("a", "b", sep = "-"))` →
`"a-b"`); `modifyList(x, val)` overlays `val` onto `x` by name (replace / append
/ `NULL` removes); and `lst$name` / `lst[["name"]]` / `lst[[i]]` index a list by
name or position, with a missing name returning `NULL`.
**`switch()` + error handling (R-18)** — `switch(EXPR, ...)` is a lazy multi-way
branch: a character `EXPR` matches arm names (unnamed final arm = default; no
match and no default → `NULL`), a numeric `EXPR` selects the n-th arm by position,
and **only the chosen arm evaluates** (`switch("a", a = "ok", b = stop("x"))`
does not raise). `stop(...)` raises an error, `warning(...)` emits a warning and
returns invisibly without aborting, and `tryCatch(expr, error = fn, finally =
cleanup)` runs `expr`, routes any error to `error` (with a condition object whose
`conditionMessage(e)` / `e$message` give the message), and always runs `finally`.
An **empty arm** falls through to the next non-empty arm (R-19):
`switch("a", a = , b = "hit")` → `"hit"`, chaining across several empties
(`a = , b = , c = "z"` → `"z"`). This needed a shared-grammar change
(`arg = NAME EQ [expr] | expr`) so `a = ,` parses; an empty arg in an ordinary
call is an eval-time error.

**Functional helpers (R-20)** — the rest of R's functional toolkit, on top of the
R-10 family. `Find(f, x)` returns the first element where `f` is `TRUE`
(`NULL` if none); `Position(f, x)` returns its 1-based index. `Negate(f)` is a
new function computing `!f(...)` (`Negate(is.na)(NA)` → `FALSE`).
`Reduce(f, x, accumulate = TRUE)` returns the running folds
(`Reduce(\(a, b) a + b, 1:4, accumulate = TRUE)` → `c(1, 3, 6, 10)`; an init
seeds the first element). `Recall(...)` re-invokes the enclosing function for
anonymous recursion (`(\(n) if (n <= 1) 1 else n * Recall(n - 1))(5)` → `120`).
All take the function by name, so they compose with the pipe
(`1:5 |> Find(f = \(x) x > 2)`).

**Environments & scoping (R-21)** — R's environment-model core subset.
`local({ x <- 5; x * 2 })` → `10` and `x` does not leak (the block runs in a
fresh child scope). `<<-` super-assignment rebinds the nearest *enclosing*
binding — `f <- function() { y <<- 99 }; f(); y` → `99` (created globally when no
enclosing binding exists) — and the counter idiom `make_counter <- function() {
n <- 0; function() { n <<- n + 1; n } }` advances across calls; the R-only `->>`
form is the mirror image. `assign("q", 7); get("q")` → `7`; `exists("zzz")` →
`FALSE`; `rm("d")` removes a binding.

**First-class environments (R-22)** — an environment is now a value (the shared
`SValue::Environment`). `e <- new.env()` makes a fresh env; `assign("x", 5, envir
= e)` / `get("x", envir = e)` round-trip through it; `ls(e)` lists its names
sorted; and an environment is **mutable by reference** — `f <- function(env)
assign("x", 42, envir = env); f(e); get("x", envir = e)` → `42`. `environment()`
returns the current env; an env prints as the stable placeholder `<environment>`
with class `"environment"`. The parent link is a `Weak` so an env-holding-env
cannot leak (see `s-runtime`).

**Closure environments & frame reflection (R-23)** — `environment(f)` is the env
a closure captured at definition (a top-level closure captures the global env, so
`environmentName(environment(f))` → `"R_GlobalEnv"`; a non-closure → `NULL`).
`environment(f) <- e` re-homes a closure (its free variables then resolve from
`e`). `environmentName(e)` is `"R_GlobalEnv"` / `"R_EmptyEnv"` / `""`;
`globalenv()` / `emptyenv()` / `baseenv()` return the well-known environments
(`baseenv()` aliases global). `parent.frame(n = 1)` is the **caller's** env —
`g <- function() get("x", envir = parent.frame()); f <- function() { x <- 42;
g() }; f()` → `42` — clamping to the global env past the bottom of the stack
rather than panicking. `is.environment(x)` is the type predicate.

**R5 reference classes (R-24)** — `setRefClass("Acc", fields = list(total =
"numeric"), methods = list(add = function(x) { total <<- total + x }, get =
function() total))` builds a **generator**; `Acc$new(total = 0)` builds an
**instance** (an environment holding the fields). A method's body sees the fields
directly and updates them with `<<-`, so `a$add(5); a$add(3); a$total` → `8`.
`a$total <- 100` writes a field **by reference**. The headline R5 behaviour is
**reference semantics**: `b <- a; b$add(1); a$total` reflects b's change (the two
names share one instance, unlike R's copy-on-modify), while two separate `$new`
calls are independent. `.self` is bound in the instance, so a method can reach a
sibling as `.self$other()`. Field type strings are recorded but not enforced in
this subset.

**R5 inheritance, `$copy()`, and introspection (R-25)** — `Sub <-
setRefClass("Sub", contains = "Base", fields = list(y = "numeric"), methods =
list(sum = function() x + y))` declares a **subclass**. A `Sub` instance has the
union of base and sub fields, inherited base methods are callable on it
(`s$getx()`), a sub method reads/writes base fields, and a sub method overrides a
same-named base method. `b <- a$copy()` is a **deep, independent** copy —
`b$x <- 9` leaves `a$x` unchanged — in contrast to `d <- a; d$x <- 7`, which
aliases. `is(s, "Base")`, `inherits(s, "Base")`, and `class(s)` walk the class
chain `c("Sub", "Base", "envRefClass", "environment")`; `Sub$fields()` and
`Sub$methods()` return the sorted field/method names including inherited ones. A
cyclic or unknown `contains =` is a clean error.

**R5 `callSuper()`, active bindings, and multiple inheritance (R-26)** — completes
the R5 system. `callSuper()` inside an overriding method invokes the parent's
same-named method (`Sub <- setRefClass("Sub", contains = "Base", methods =
list(describe = function() paste(callSuper(), "sub")))` → `Sub$new()$describe()`
is `"base sub"`); it chains across levels, forwards args, runs against the
instance, and returns `NULL` past the root. A **function-valued field** is an
**active binding** — a getter/setter: `fahrenheit = function(v) { if (missing(v))
celsius*9/5+32 else celsius <<- (v-32)*5/9 }` makes `t$fahrenheit` compute and
`t$fahrenheit <- 32` assign `t$celsius`; active bindings are inherited and
`$copy()`-independent. **Multiple inheritance** `contains = c("A", "B")` unions A's
and B's fields/methods (left-to-right precedence), with `is`/`inherits`/`class`
seeing every base, diamonds de-duplicated, and mutual-inheritance cycles rejected.

**Output formatting (R-27)** — a pivot off the R5/OOP lane into a fresh
data/utility area, with deterministic, locale-free output (fixed `","` thousands
separator and `"."` decimal point). `format(x, nsmall=, width=, justify=,
big.mark=)` formats numbers and character vectors — a numeric vector pads to a
*common* width (`format(c(1, 10, 100))` → `c("  1", " 10", "100")`),
`format(3.14159, nsmall = 2)` → `"3.14"`, `format(42, width = 5)` → `"   42"`,
character `justify` is `"left"`/`"right"`/`"centre"`, and `big.mark` groups
thousands. `formatC(x, format=, digits=, width=, flag=)` is the C-style
formatter (`format` `"d"`/`"f"`/`"e"`/`"g"`/`"s"`/`"x"`, `flag` `"-"`/`"0"`/`"+"`):
`formatC(3.14159, format = "f", digits = 2)` → `"3.14"`, `formatC(42, width = 6,
flag = "0")` → `"000042"`. `prettyNum(1234567, big.mark = ",")` → `"1,234,567"`,
`toString(1:3)` → `"1, 2, 3"`, and `sprintf` recycles over vector arguments
(`sprintf("%d-%s", 1:2, c("a","b"))` → `c("1-a", "2-b")`). Every field width and
precision is capped at 1 MiB so a crafted `fmt` or a huge `width=` cannot trigger
a giant allocation. (Exotic `formatC` corners — `format = "g"` rounding, the
`" "`/`"#"` flags, `scientific=` — are deferred to a later formatting pass.)

**Apply-family & grouping (R-28)** — the grouping and table builtins that pair
R's functional toolkit (R-10) with matrices (R-11) and factors. `outer(X, Y,
FUN = "*")` builds the `length(X) × length(Y)` column-major matrix of
`FUN(X[i], Y[j])`: `outer(1:3, 1:2)` → the 3×2 product matrix, and `FUN` may be
`"*"`/`"+"` or an arbitrary function (`outer(1:2, 1:2, \(a, b) a*10 + b)` →
`c(11, 21, 12, 22)`). `tapply(X, INDEX, FUN)` groups `X` by `INDEX` and applies
`FUN` per group, returning a named vector (`tapply(c(1,2,3,4),
c("a","b","a","b"), sum)` → `c(a = 4, b = 6)`). `split(x, f)` partitions into a
named list (`split(1:4, c("a","b","a","b"))` → `list(a = c(1,3), b = c(2,4))`),
and `tabulate(bin, nbins = max(bin))` counts occurrences of `1..nbins`
(`tabulate(c(2,3,5), nbins = 5)` → `c(0,1,1,0,1)`). `outer` guards its element
count with `checked_mul` against the sequence cap *before* allocating and
`tabulate` clamps `nbins`, so neither can be turned into an OOM. (The `%o%`
infix alias, matrix dimnames, multi-way `tapply`, and `simplify = FALSE` are
deferred to a later pass.)

**Vector set operations & ordering (R-29)** — the set-theory and ranking corner
of R's base toolkit, all numeric- and character-aware. `union(x, y)` is the
distinct elements of `c(x, y)` in first-occurrence order (`union(c(1,2),
c(2,3))` → `c(1, 2, 3)`); `intersect(c(1,2,3), c(2,3,4))` → `c(2, 3)` and
`setdiff(c(1,2,3,4), c(2,4))` → `c(1, 3)` keep the elements in / not-in `y`,
deduplicated and in `x`'s order. `is.element(2, c(1,2,3))` → `TRUE` is the
function spelling of `el %in% set`. `duplicated(c(1,1,2,3,3))` →
`c(FALSE, TRUE, FALSE, FALSE, TRUE)` flags repeats of earlier elements. `rank(x)`
gives sample ranks with **average** tie handling (R's default): `rank(c(3,1,2))`
→ `c(3, 1, 2)` and `rank(c(1,1,2))` → `c(1.5, 1.5, 3)`. Outputs are bounded by
the inputs (each already sequence-capped) and `rank` is `O(n log n)`, so none is
a DoS vector.

**Ordering refinements (R-30)** — extensions of the R-29/R-13 builtins. Multi-key
`order(x, y, ...)` sorts lexicographically by the first key, breaking ties by the
next, remaining ties kept in original order (`order(c(2,1,2), c(1,2,1))` →
`c(2, 1, 3)`; keys may mix numeric and character, and all must share the first
key's length). `rank(x, ties.method=)` adds `"min"`, `"max"`, and `"first"` to the
default `"average"` (`rank(c(1,1,2))` → `c(1.5,1.5,3)` / `c(1,1,3)` / `c(2,2,3)` /
`c(1,2,3)`). `duplicated(x, fromLast=TRUE)` keeps the *last* occurrence
(`duplicated(c(1,2,1), fromLast=TRUE)` → `c(TRUE, FALSE, FALSE)`).
`anyDuplicated(x)` returns the 1-based index of the first duplicated element, or
`0` if none (`anyDuplicated(c(1,2,1))` → `3`).

**Set-op & ordering refinements (R-31)** — extensions of the R-29/R-30 dedup &
ranking builtins. `incomparables=` on `duplicated`, `anyDuplicated`, and `unique`:
the default `FALSE` means "no incomparables"; a vector lists values that are
**never equal to anything**, so they are never flagged/removed as duplicates
(`duplicated(c(1,1,2,2), incomparables=1)` → `c(F,F,F,T)`; `unique(c(1,1,2,2),
incomparables=1)` → `c(1,1,2)`; `anyDuplicated(c(1,2,1), incomparables=1)` → `0`).
`unique(x, fromLast=TRUE)` keeps the *last* occurrence in input order, mirroring
`duplicated(fromLast=)`. `rank(x, ties.method="random")` breaks ties with a
Fisher–Yates shuffle over the `set.seed`-seeded session RNG, so it is reproducible
under `set.seed`. Numeric and character vectors; bounded RNG draws; malformed named
args error gracefully. (`incomparables=`/`fromLast=` on the binary set ops
`union`/`intersect`/`setdiff` are deferred to R-32.)

**Binning & cross-product utilities (R-32)** — the numeric-binning family,
reached through ordinary R syntax. A pivot away from the R-31 deferral of
`incomparables=`/`fromLast=` on the binary set ops, which base R does not accept
there (so implementing them would be non-faithful). `findInterval(x, vec)` returns,
for each `x`, the 1-based index of the last break in the non-decreasing `vec` not
exceeding it — `0` below the first, `length(vec)` at/above the last, `NA` propagates
(`findInterval(c(0.5,1.5,2.5), c(1,2,3))` → `c(0,1,2)`; `findInterval(5, c(1,2,3))`
→ `3`). `cut(x, breaks)` bins `x` into the right-closed `(lo,hi]` intervals of the
sorted `breaks` and returns a real **factor**: `class(cut(...))` is `"factor"`,
`levels()` are the `"(lo,hi]"` labels, and `as.character()`/`as.integer()`/
`nlevels()` all see through to it (`cut(c(1,5,10), breaks=c(0,3,6,11))` → a factor
with levels `"(0,3]","(3,6]","(6,11]"` and values `(0,3]`,`(3,6]`,`(6,11]`); values
outside all breaks → `NA`. Built on `findInterval`; allocations bounded by the
already-capped input/breaks lengths; missing operands error gracefully.

**R-33 — `cut()` option completeness.** `cut` now takes the four options that were
deferred from R-32. `labels=FALSE` returns the **integer bin codes** as a plain
numeric vector (not a factor) — `cut(c(1,2,5), breaks=c(0,3,6), labels=FALSE)` →
`c(1,1,2)`; a character `labels` vector becomes the factor levels and must match
`length(breaks)-1` (else an error) — `cut(c(1,5,10), breaks=c(0,3,6,11),
labels=c("lo","mid","hi"))`. `right=FALSE` switches to left-closed `[lo,hi)`
intervals (labels `"[lo,hi)"`) — `cut(c(1,3), breaks=c(0,3,6), right=FALSE)` →
`[0,3)`,`[3,6)`. `include.lowest=TRUE` folds the extreme break (the lowest for
`right=TRUE`, the highest for `right=FALSE`) into the adjacent interval so it bins
instead of going `NA`. A single-number `breaks` is the **equal-width** form: `N`
bins over the range of `x`, extended by `dx/1000` on each side (`dx=max-min`;
degenerate `dx=0` → `abs(min)`, then `1`) — `cut(0:10, breaks=5)` → 5 levels, every
value binned. Security: `N` is capped at `MAX_SEQ_LEN` before any allocation, the
breaks use finite/checked arithmetic (no divide-by-zero on a degenerate range), and
the `labels` length check never panics. `dig.lab=` and `ordered_result=` land in
R-35, below.

**R-35 — ordered factors & `cut()` label polish.** An *ordered* factor is a factor
whose levels carry a meaningful order. `ordered(x, levels=, labels=)` (and the
synonym `factor(x, ordered=TRUE)`) build one, `as.ordered(x)` coerces, and
`is.ordered(x)` tests for it; `class(ordered(c("a","b")))` is
`c("ordered", "factor")`. The relational operators (`<`, `<=`, `>`, `>=`, `==`,
`!=`) between ordered factors compare **by level index**, not label string: with
`f <- ordered(c("lo","hi","mid"), levels=c("lo","mid","hi"))`, `f[1] < f[2]`
(lo < hi) is `TRUE` while `f[2] < f[3]` (hi < mid) is `FALSE`. An `NA` code yields
`NA`, and comparing ordered factors with different level sets is an error.
`cut(..., ordered_result=TRUE)` returns an ordered factor (bins compare by interval
order), and `cut(..., dig.lab=k)` formats break labels to `k` significant digits
(default 3), e.g. `levels(cut(c(1.23456, 5.6789), breaks=c(0, 3.14159, 10),
dig.lab=2))` → `c("(0,3.1]", "(3.1,10]")`. Security: ordered comparison reads the
integer codes only (out-of-range / NA → NA, never a panic), and `dig.lab` is clamped
to `1..=22` before formatting. (`sort`/`max`/`min`/`range` on ordered factors and
`Ops.ordered` dispatch are deferred to R-39.)

**String utilities (R-34)** — an independent string-utility family reached through
ordinary R syntax (not part of the cut/set-ops chain). All five reuse the existing
string machinery and operate on Unicode `char`s, never raw byte indices, so
multibyte UTF-8 input is always safe. `startsWith(x, prefix)` / `endsWith(x, suffix)`
are logical, recycled over *both* args with `NA` → `NA`
(`startsWith(c("apple","banana"), "a")` → `c(TRUE, FALSE)`). `trimws(x, which="both")`
strips leading/trailing whitespace (`[ \t\r\n]`); `which ∈ {both,left,right}`, any
other value an error. `chartr(old, new, x)` translates characters, requiring
`old`/`new` of equal `nchar` (`chartr("é","e","café")` → `"cafe"`). `strtoi(x,
base=10L)` parses integers in bases 2..36 the way C `strtol` does — leading
whitespace and a sign, a `0x` prefix for base 16, the whole string consumed, and
`NA` for an empty string, garbage, an out-of-range digit, or a base outside 2..36
(`strtoi("FF", 16L)` → `255`; `strtoi(c("7","8"), 8L)` → `c(7, NA)`). Parsing uses
checked `i64` arithmetic, so overflow yields `NA` rather than a panic.

**R-37 — string-utility completeness** finishes the family. `strtoi(x, base=0L)`
auto-detects each string's radix from its prefix, C `strtol`-style — `0x`/`0X` →
hex, a leading `0` + digit → octal, a lone `"0"` → zero, else decimal
(`strtoi("0x1F", 0L)` → `31`; `strtoi("010", 0L)` → `8`; `strtoi("12", 0L)` → `12`;
`strtoi("08", 0L)` → `NA`). `trimws(x, whitespace=)` adds a keyword-only
`whitespace=` argument, interpreted as a **regex** (default `"[ \t\r\n]"`, faithful
to base R ≥ 3.6) via the same RE2 engine `grepl`/`gsub` use, anchored to the trimmed
edge (`trimws("xxhixx", whitespace="x")` → `"hi"`). RE2's linear-time matching rules
out ReDoS, and slicing is on `char`-boundary offsets (UTF-8 safe).

**R-36 — matrix cross products** add `crossprod` and `tcrossprod`, again through the
shared `s-runtime`. An independent matrix-algebra item, defined entirely in terms of
the existing R-11 `t()` transpose and `%*%` matrix product (no new linear algebra).
`crossprod(x, y)` = `t(x) %*% y` and `crossprod(x)` = `t(x) %*% x` (the Gram matrix
`X'X`); `tcrossprod(x, y)` = `x %*% t(y)` and `tcrossprod(x)` = `x %*% t(x)` (`XX'`).
The second argument defaults to the first. `crossprod(matrix(c(1,2,3,4), nrow=2))`
→ `[[5,11],[11,25]]`; `tcrossprod(...)` of the same → `[[10,14],[14,20]]`; for
`B = matrix(1:6, nrow=2)`, `dim(crossprod(B))` is `c(3,3)` and `dim(tcrossprod(B))`
is `c(2,2)`. A non-conformable pair raises the same `"non-conformable arguments"`
error `%*%` raises. Because the impl reuses the `%*%` handler, it inherits its
`MAX_SEQ_LEN` allocation guard and conformability check — no new unbounded multiplier.

**R-38 — `kronecker()` (Kronecker product)** adds the block-outer product, again
through the shared `s-runtime`. For an `m×n` `X` and a `p×q` `Y`, `kronecker(X, Y)`
is the `(m·p)×(n·q)` matrix whose block `(i, j)` is `X[i,j] · Y`, i.e.
`result[(i-1)·p+k, (j-1)·q+l] = X[i,j] · Y[k,l]` (column-major). It reuses the
existing matrix accessor and `SValue::Matrix` constructor (no new value type); a
bare vector promotes to an `n×1` column.
`dim(kronecker(matrix(c(1,2,3,4), nrow=2), matrix(c(0,1,1,0), nrow=2)))` is
`c(4,4)`; `kronecker(matrix(5), matrix(c(1,2,3,4), nrow=2))` is `5·Y` (2×2); a
2×3 ⊗ 1×2 gives a 2×6 matrix. The result is a real matrix — `dim`/`nrow`/`ncol`
work and it composes with `%*%`. Because the result is *quadratic* in the inputs,
the row count `m·p`, column count `n·q`, and their product are each formed with
`checked_mul` and bounded by the `MAX_SEQ_LEN` cap before allocating — an
over-large product errors rather than OOMing, and `0×n`/`m×0` inputs give an empty
result with no out-of-bounds access. The R `%x%` infix alias (`X %x% Y`) needs
lexer/grammar work and is deferred to **R-40**; this ships the function form only.

**R-40 — `chol()` (Cholesky factorization)** adds the Cholesky factor of a real
symmetric positive-definite matrix, again through the shared `s-runtime`. For an
`n×n` SPD `x`, `chol(x)` returns the **upper-triangular** `R` with
`t(R) %*% R == x` (R's convention — the upper factor, `R'R = X`), via the
Cholesky–Banachiewicz recurrence, reading only the **upper triangle** of `x`
(like R's default). It reuses the existing `square_matrix` reader (shared with
`det`/`solve`) for the non-matrix / non-square / over-cap rejection and the
`SValue::Matrix` constructor (no new value type), column-major throughout.
`chol(matrix(c(4,2,2,3), nrow=2))` is `[[2,1],[0,√2]]` and `t(R) %*% R`
reconstructs `x`; `chol(diag(3))` is the identity. The diagonal pivot is checked
`> 0` **before** the `sqrt`, so a non-positive-definite matrix
(`chol(matrix(c(1,2,2,1), nrow=2))`, eigenvalues `3, -1`) is a clean
*"…not positive definite"* error — never `NaN`, never a panic; a non-square
matrix errors before any indexing. `pivot=TRUE` (pivoted Cholesky), `chol2inv()`,
and complex (Hermitian) matrices are deferred to **R-41**.

## Usage

```rust
use coding_adventures_r_runtime::{eval_r, format_value};

let v = eval_r("data_frame <- c(1, 2, 3)\nmean(data_frame)\n").unwrap();
assert_eq!(format_value(&v), vec!["[1] 2".to_string()]);
```

For a persistent session (the REPL), construct an `RInterpreter` and call
`eval_str` repeatedly — bindings persist.

## Testing

```sh
cargo test -p coding-adventures-r-runtime
```

See [`code/specs/R00-r-language.md`](../../../specs/R00-r-language.md).
