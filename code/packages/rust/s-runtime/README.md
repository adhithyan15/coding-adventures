# S Runtime

A tree-walking evaluator for the historical
[S programming language](https://en.wikipedia.org/wiki/S_(programming_language))
(Bell Labs, 1976) — the ancestor of R.

## What it does

Evaluates S programs. It parses source with `coding-adventures-s-parser`, then
walks the resulting parse tree with a recursive `Interpreter`, computing
`SValue`s. Numeric and statistical work is delegated to the shipped substrate
(`r-vector`, `numeric-tower`, `statistics-core`) so the math has a single
authoritative home — this crate is a tree-walk over that substrate, not a
re-implementation.

## How it fits in the stack

```text
s-lexer → s-parser → GrammarASTNode → s-runtime (this crate) → s-repl
                                          |
                          r-vector / numeric-tower / statistics-core
                                          |
                          array-runtime → matrix-ir/-cpu/-runtime   (matrix %*%)
```

The matrix product `%*%` lowers onto the **shared f64 matrix-execution
substrate** (`array-runtime` → `matrix-ir` → `matrix-cpu`/`matrix-runtime`) — the
same cost-based CPU/GPU planner MATLAB uses — at full `f64` precision, instead of
a hand-written loop. See [What "S-flavored" means here](#what-s-flavored-means-here).

## What "S-flavored" means here

- **Everything is a vector** — `3` is a numeric vector of length one.
- **Recycling**, **NA propagation**, and the `logical < double < character`
  **coercion** lattice.
- The historical **`_` assignment** (`x _ 5` is `x <- 5`).
- **Lexical scoping** — closures capture their defining environment.
- **v2** adds `%op%` infix operators, a broad builtin library, **S3 method
  dispatch** (a generic `print`), **factors**, and **data frames** (`$`,
  `[[ ]]`, 2-D indexing).
- **Index sub-assignment** (R-14): the left side of `<-` may be a subscript,
  e.g. `v[i] <- x`, `v[-i] <- x`, `m[i, j] <- x`, `m[, j] <- x` — copy-on-modify,
  RHS recycled, so a prior `b <- a` copy is never aliased.
- **Named vectors / the `names` attribute** (R-15): `c(a = 1, b = 2)` attaches
  names (nested named pieces combine R-style), `names(x)` gets them and
  `names(x) <- value` / `setNames(x, nm)` set them (NA-pad short, `NULL` clears),
  `x["b"]` indexes by name, and a named vector prints names above values. Names
  are a transparent `SValue::Named` wrapper — they ride through indexing and drop
  through arithmetic, exactly as in R.
- **General attributes** (R-16): `attr(x, which)` / `attr(x, which) <- value`
  (assigning `NULL` removes), `attributes(x)` / `attributes(x) <- list(...)`, and
  `structure(x, class = "myc", foo = "bar")`. General (non-special) attributes
  live in a transparent `SValue::Attributed` wrapper; the *special* attributes
  route to their dedicated representations, so `attr(x, "names")` agrees with
  `names(x)`, `attr(x, "class")` with `class(x)`, and `attr(x, "dim")` with the
  matrix `dim` by construction. The general map is bounded and malformed input
  fails closed.
- **`do.call`, `modifyList`, named-list access** (R-17): `do.call(what, args)`
  calls `what` (a function value, or a string naming one) with the elements of
  the list `args` spread as positional and named arguments — reusing
  `Interpreter::call_value`, so `do.call(paste, list("a", "b", sep = "-"))` is
  `"a-b"`. `modifyList(x, val)` overlays `val` onto `x` by name (replace / append
  / `NULL` removes). `lst$name` / `lst[["name"]]` / `lst[[i]]` index a list by
  name or position (missing name → `NULL`), seeing through the transparent
  `Classed`/`Attributed`/`Named` wrappers. Argument/result sizes are bounded and
  malformed input (non-list, non-callable, unnamed `val` element) fails closed.
- **`switch()` + error handling** (R-18): `switch(EXPR, ...)` and
  `tryCatch(expr, error = ..., finally = ...)` are **special forms** — the call
  dispatcher intercepts them before argument evaluation, so only the selected arm
  / protected expression / chosen handler runs (`switch("a", a = "ok", b =
  stop("x"))` does not raise). `switch` matches a character `EXPR` against arm
  names (unnamed final arm = default; no match and no default → invisible `NULL`)
  or selects an arm by numeric position (out of range → `NULL`). `stop(...)`
  raises a catchable `SError::User`; `warning(...)` records and prints a warning
  (bounded by `MAX_WARNINGS`) without aborting; `tryCatch` routes any catchable
  error to its `error` handler with a minimal condition object (`list(message,
  call)` classed `c("simpleError", "error", "condition")`, so `conditionMessage`
  / `e$message` work) and always runs `finally`. Loop-control signals
  (`break`/`next`) are not caught. An **empty arm** falls through to the next
  non-empty arm (R-19): `switch("a", a = , b = "hit")` → `"hit"`, chaining across
  several empties (`a = , b = , c = "z"` → `"z"`); a matched empty arm with
  nothing non-empty after it yields `NULL`. (R-19 extended the shared S/R grammar
  to `arg = NAME EQ [expr] | expr` so `a = ,` parses; the empty value is only
  meaningful in `switch` — an empty arg in an ordinary call is an eval-time
  error.)
- **Functional helpers** (R-20): the rest of R's functional toolkit, on top of
  the R-10 family. `Find(f, x)` returns the first element where `f(element)` is
  `TRUE` (invisible `NULL` if none, short-circuiting); `Position(f, x)` returns
  its 1-based index. `Negate(f)` returns a new callable computing `!f(...)` — a
  small `SValue::Negated` wrapper the call dispatcher recognizes, invoking `f`
  through the normal depth-bounded path and logically negating the result
  (`Negate(is.na)(NA)` → `FALSE`). `Reduce(f, x, ..., accumulate = TRUE)` returns
  the running folds (`Reduce(\(a, b) a + b, 1:4, accumulate = TRUE)` →
  `c(1, 3, 6, 10)`; an init seeds the first element); the no-`accumulate`
  behaviour is unchanged. `Recall(...)` re-invokes the enclosing function for
  anonymous recursion, reading a per-interpreter call stack pushed by
  `call_closure` (RAII-popped). Recursion is bounded by `MAX_EVAL_DEPTH` and the
  accumulator by `MAX_SEQ_LEN`; non-callable predicates and out-of-closure
  `Recall` fail closed.
- **Environments & scoping** (R-21): `<<-` super-assignment (`env::super_assign`)
  walks the *enclosing* scope chain to rebind the nearest existing binding, else
  creates it in the global environment — the engine behind the counter-closure
  idiom `function() { n <- 0; function() { n <<- n + 1; n } }`. `local({ ... })`
  evaluates a block in a fresh child scope and returns its value without leaking
  locals (`local({ x <- 5; x * 2 })` → `10`). `assign`/`get`/`exists`/`rm` are
  lazy special forms (like `switch`/`tryCatch`) operating by name against the
  current scope. The chain walk is iterative over a finite acyclic scope list, so
  it always terminates; non-name super-assign targets fail closed.
- **First-class environments** (R-22): the `SValue::Environment` variant boxes a
  shared scope handle, so a scope is a value that can be passed, stored, and
  **mutated by reference**. `new.env()` makes a fresh env (parent = caller's
  scope); `environment()` returns the current env; `ls(e)` lists its names sorted;
  and `assign`/`get`/`exists`/`rm`/`local` honour an `envir = e` argument that
  operates on the passed environment (replacing R-21's rejection). Mutating an env
  through one alias is visible through every other — `e <- new.env(); f <-
  function(env) assign("x", 1, envir = env); f(e); get("x", envir = e)` → `1`. The
  cycle risk an env-holding-env would pose is broken by making `Scope::parent` a
  `Weak`: an env value owns the only strong `Rc` to its scope, parents are
  referenced but never owned, so no strong-`Rc` cycle is constructible from
  source. An environment prints as the stable placeholder `<environment>`, never a
  heap address.
- **Closure environments & frame reflection** (R-23): `environment(f)` is the env
  a closure captured at definition (non-closure → `NULL`); `environment(f) <- e`
  re-homes a closure via the replacement-function lvalue path; `environmentName(e)`
  is `"R_GlobalEnv"` / `"R_EmptyEnv"` / `""` by `Rc` identity; `globalenv()` /
  `emptyenv()` / `baseenv()` return the well-known envs (`baseenv()` aliases
  global); `parent.frame(n = 1)` is the caller's env, recorded on the R-20 call
  stack and **clamped** to global past the bottom rather than panicking;
  `is.environment(x)` is the type predicate.
- **R5 reference classes** (R-24, `src/refclass.rs`): `setRefClass("Name",
  fields = …, methods = …)` builds a **generator** (an environment carrying the
  class name, field names, and method closures); `generator$new(field = …)` builds
  an **instance** (an environment holding the fields). `obj$field` reads a field,
  `obj$field <- v` writes it **in place by reference**, and `obj$method(args)`
  rebuilds a fresh instance-bound closure on access so `field <<- value` and
  `.self$field <- v` mutate the live instance. `b <- a` *aliases* the same instance
  (reference semantics — the deliberate exception to copy-on-modify). The
  instance⇄method `Rc` cycle is broken by construction (instance-bound closures are
  rebuilt lazily and never stored; the stored method closures close over the
  *generator*); the lone `.self` self-reference is the documented,
  `MAX_ENVIRONMENTS`-bounded R-22 value-binding cycle.
- **R5 inheritance, `$copy()`, and introspection** (R-25, `src/refclass.rs`):
  `setRefClass("Sub", contains = "Base", …)` gives **single inheritance** — a
  subclass generator links to its parent (`.refParent`, a child→parent DAG edge),
  and an instance gets the union of base ∪ sub fields and methods (a sub method
  overrides a same-named base method; an inherited base method is callable on a Sub;
  a sub method reads/writes base fields). `obj$copy()` is a **deep**, independent
  value-copy (vs `b <- a`, which aliases). `is(obj, "Base")` / `inherits(obj,
  "Base")` / `class(obj)` walk the class chain `c("Sub", "Base", …, "envRefClass",
  "environment")`; `generator$fields()` / `generator$methods()` return the sorted
  effective names. A cyclic `contains =` is rejected; all chain walks are bounded by
  `MAX_CHAIN_DEPTH`.
- **R5 `callSuper()`, active bindings, and multiple inheritance** (R-26,
  `src/refclass.rs`): `callSuper()` (a lazy special form) invokes the parent's
  same-named method — each instance-bound method is materialised inside a
  **super-context** scope recording `.refSuperGens`/`.refMethodName`, so the call
  resolves the same name starting one level up and chains to the root (past-the-root
  is a clean `NULL`). A **function-valued field** is an **active binding**: `obj$ab`
  calls a nullary getter, `obj$ab <- v` calls a setter (`v` bound), distinguished by
  the new `missing(x)` special form; the binding is re-homed per instance (so a
  `$copy()` is independent) and runs through the depth-bounded call path (a
  self-referential getter errors at `MAX_EVAL_DEPTH`, never borrow-panics). **Multiple
  inheritance** `contains = c("A", "B")` stores a `.refParents` list; a left-to-right
  DFS `linearization` (de-duping diamonds; C3 not implemented) drives all
  effective-set/class-chain walks with most-derived-first precedence, and the cycle
  check runs over every parent so the multi-parent graph stays a DAG.
- **Output-formatting builtins** (R-27): `format`, `formatC`, `prettyNum`,
  `toString`, and the (already vectorized) `sprintf` — pure, deterministic,
  locale-free string formatters. `format(x, nsmall=, width=, justify=,
  big.mark=)` formats a **numeric** vector to a *common* width (right-justified
  to the widest, so columns line up) or a **character** vector with `justify`;
  `formatC(x, format=, digits=, width=, flag=)` is the C-style wrapper over the
  `sprintf` `render_conversion` engine (`"d"`/`"f"`/`"e"`/`"g"`/`"s"`/`"x"`,
  flags `"-"`/`"0"`/`"+"`); `prettyNum` inserts a thousands separator; `toString`
  collapses a vector to one string. The `MAX_FIELD = 1 MiB` field-width cap is
  shared across all of them, so a crafted `fmt` or a huge `width=`/`nsmall=`/
  `digits=` cannot trigger a giant allocation. (Exotic `formatC` corners —
  `format = "g"` rounding, `" "`/`"#"` flags, `scientific=` — deferred to a later
  formatting pass.)
- **Apply-family & grouping builtins** (R-28): `outer`, `tapply`, `split`, and
  `tabulate` — the grouping/table functions that pair the R-10 functional toolkit
  with R-11 matrices, R-6 lists, and factors. `outer(X, Y, FUN = "*")` builds the
  `length(X) × length(Y)` column-major matrix of `FUN(X[i], Y[j])` (`FUN` is
  `"*"`/`"+"` on a fast numeric path, or any callable per `(i, j)` pair);
  `tapply(X, INDEX, FUN)` groups `X` by `INDEX` and returns a named vector;
  `split(x, f)` returns a named list partitioning `x` by level; `tabulate(bin,
  nbins = max(bin))` counts `1..nbins`. `outer` guards `nrow*ncol` with
  `checked_mul` against `MAX_SEQ_LEN` *before* allocating and `tabulate` clamps
  `nbins`, so neither is a DoS vector. (`%o%` infix, matrix dimnames, multi-way
  `tapply`, and `simplify = FALSE` are deferred to a later pass.)
- **Vector set operations & ordering** (R-29): `union`, `intersect`, `setdiff`,
  `is.element`, `duplicated`, and `rank` — vectors treated as multisets, all
  numeric- and character-aware (they key on the same coerced-character form as
  `unique`/`%in%`). `union(x, y)` is the distinct elements of `c(x, y)` in
  first-occurrence order (`union(c(1,2), c(2,3))` → `c(1,2,3)`); `intersect`/
  `setdiff` keep the elements in/not-in `y`, deduplicated and in `x`'s order;
  `is.element(el, set)` is the function spelling of `el %in% set`; `duplicated(x)`
  flags repeats of earlier elements (`duplicated(c(1,1,2,3,3))` →
  `c(F,T,F,F,T)`); and `rank(x)` gives sample ranks with **average** ties
  (`rank(c(1,1,2))` → `c(1.5, 1.5, 3)`). Outputs are bounded by the inputs (each
  already `MAX_SEQ_LEN`-bounded) and `rank` is `O(n log n)`, so none is a DoS
  vector.
- **Ordering refinements** (R-30): extensions of the R-29/R-13 ordering builtins.
  **Multi-key `order(x, y, ...)`** sorts the index permutation lexicographically by
  the first key, breaking ties by the next, with remaining ties kept in original
  order (stable); keys are coerced independently (numeric: `NA` last; character:
  lexicographic) and may be mixed, and all must share the first key's length
  (`order(c(2,1,2), c(1,2,1))` → `c(2, 1, 3)`). **`rank(x, ties.method=)`** adds
  `"min"`, `"max"`, and `"first"` to the default `"average"` (`rank(c(1,1,2))` →
  `c(1.5,1.5,3)` / `c(1,1,3)` / `c(2,2,3)` / `c(1,2,3)`). **`duplicated(x,
  fromLast=TRUE)`** scans right-to-left so the *last* occurrence is the keeper
  (`duplicated(c(1,2,1), fromLast=TRUE)` → `c(T,F,F)`). **`anyDuplicated(x)`**
  returns the 1-based index of the first duplicated element, or `0` if none
  (`anyDuplicated(c(1,2,1))` → `3`). Outputs stay bounded by the (capped) inputs;
  the per-key length check guards `order` against out-of-bounds indexing.
- **Set-op & ordering refinements** (R-31): extensions of the R-29/R-30 dedup &
  ranking builtins. **`incomparables=`** on `duplicated`, `anyDuplicated`, and
  `unique` — default `FALSE` means "no incomparables"; a vector lists values that
  are **never equal to anything**, so they are never flagged/removed as duplicates
  (`duplicated(c(1,1,2,2), incomparables=1)` → `c(F,F,F,T)`; `unique(c(1,1,2,2),
  incomparables=1)` → `c(1,1,2)`; `anyDuplicated(c(1,2,1), incomparables=1)` → `0`).
  **`unique(x, fromLast=TRUE)`** keeps the *last* occurrence in input order,
  mirroring `duplicated(fromLast=)`. **`rank(x, ties.method="random")`** breaks ties
  with a Fisher–Yates shuffle over the `set.seed`-seeded session RNG, so it is
  reproducible under `set.seed`. Numeric and character vectors; bounded RNG draws;
  malformed named args error gracefully. (`incomparables=`/`fromLast=` on the binary
  set ops `union`/`intersect`/`setdiff` are deferred to R-32.)
- **Binning & cross-product utilities** (R-32): the numeric-binning family, a
  pivot away from the non-faithful R-31 deferral (base R's
  `union`/`intersect`/`setdiff` don't take `incomparables=`/`fromLast=`).
  **`findInterval(x, vec)`** returns, for each `x`, the 1-based index of the last
  break in the non-decreasing `vec` not exceeding it (`0` below the first,
  `length(vec)` at/above the last; `NA` propagates): `findInterval(c(0.5,1.5,2.5),
  c(1,2,3))` → `c(0,1,2)`, `findInterval(5, c(1,2,3))` → `3`. **`cut(x, breaks)`**
  bins `x` into the right-closed `(lo,hi]` intervals of the sorted `breaks` and
  returns a real **factor** with auto-generated `"(lo,hi]"` levels, so
  `levels()`/`as.integer()`/`as.character()`/`nlevels()` all work on the result
  (`cut(c(1,5,10), breaks=c(0,3,6,11))` → levels `"(0,3]","(3,6]","(6,11]"` with
  values `(0,3]`,`(3,6]`,`(6,11]`); values outside all breaks → `NA`. Built on
  `findInterval`; allocations bounded by the already-capped input/breaks lengths;
  missing operands error gracefully.
- **`cut()` option completeness** (R-33): the four options deferred from R-32, all
  layered onto the same interval scan. `labels=FALSE` returns the **integer bin
  codes** as a plain numeric vector (not a factor); a character `labels` vector
  becomes the levels and must match `length(breaks)-1` (else an error); absent /
  `TRUE` keeps the auto labels. `right=FALSE` gives left-closed `[lo,hi)` intervals
  (`"[lo,hi)"` labels), and the default `right=TRUE` scan now honours the `(lo,hi]`
  boundary convention exactly. `include.lowest=TRUE` folds the extreme break (lowest
  for `right=TRUE`, highest for `right=FALSE`) into the adjacent interval. A
  single-number `breaks` is the **equal-width** form: `N` bins over the range of `x`,
  extended by `dx/1000` each side (`dx=max-min`; degenerate `dx=0` → `abs(min)`, then
  `1`). `N` is capped at `MAX_SEQ_LEN` before any allocation and the breaks use
  finite/checked arithmetic, so a huge `N` or a degenerate range is rejected/handled
  rather than over-allocating or dividing by zero. (`dig.lab=` and `ordered_result=`
  land in R-35, below.)
- **String utilities** (R-34): an independent string-utility family that reuses the
  existing string machinery (`as_character`, the `Option<String>`-as-`NA`
  convention, `SValue::Character`/`SValue::Logical`) and operates on Unicode
  `char`s throughout — never raw byte indices — so multibyte UTF-8 input is always
  safe. **`startsWith(x, prefix)`** / **`endsWith(x, suffix)`** are logical, recycled
  over *both* args (`NA` → `NA`); `startsWith(c("apple","banana"),"a")` →
  `c(TRUE,FALSE)`. **`trimws(x, which="both")`** strips leading/trailing whitespace
  (`[ \t\r\n]`), `which ∈ {both,left,right}` (else an error). **`chartr(old,new,x)`**
  translates characters (`old`/`new` equal `nchar`, else an error);
  `chartr("é","e","café")` → `"cafe"`. **`strtoi(x, base=10L)`** parses integers in
  bases 2..36 (`strtol`-style: leading whitespace + sign, `0x` prefix for base 16,
  full-consume, `NA` for garbage / out-of-range digit / base outside 2..36), with
  checked `i64` accumulation (overflow → `NA`, never a panic).
- **String-utility completeness** (R-37): `strtoi(x, base=0L)` auto-detects each
  string's radix from its prefix, C `strtol`-style — `0x`/`0X` → hex, a leading `0`
  + digit → octal, a lone `"0"` → zero, else decimal (`strtoi("0x1F", 0L)` → `31`;
  `strtoi("010", 0L)` → `8`; `strtoi("08", 0L)` → `NA`). `trimws(x, whitespace=)`
  gains a keyword-only `whitespace=` argument, interpreted as a **regex** (default
  `"[ \t\r\n]"`, base-R faithful) via the same RE2 engine `grepl`/`gsub` use,
  anchored to the trimmed edge (`trimws("xxhixx", whitespace="x")` → `"hi"`). RE2's
  linear-time matching rules out ReDoS; slicing is on `char`-boundary offsets.
- **Ordered factors & `cut()` label polish** (R-35): an *ordered* factor is a factor
  whose levels carry a meaningful order — `SValue::Factor` gains an `ordered: bool`
  field, so `class()` reports `c("ordered", "factor")` when set (and the `Levels:`
  line prints with `<` separators). **`ordered(x, levels=, labels=)`** /
  **`factor(x, ordered=TRUE)`** build one; **`as.ordered(x)`** coerces;
  **`is.ordered(x)`** tests for it. The relational operators
  (`<`, `<=`, `>`, `>=`, `==`, `!=`) between two ordered factors compare **by level
  index** (the 1-based code), so with `levels=c("lo","mid","hi")` the element `"hi"`
  is `>` the element `"lo"`; an `NA` code → `NA`, and differing level sets is a clean
  error. `cut(..., ordered_result=TRUE)` makes the binned factor ordered, and
  `cut(..., dig.lab=k)` formats break labels to `k` significant digits (default 3,
  clamped to `1..=22` for safety — no extreme value can over-allocate or panic).
  (Ordered-factor `sort`/`max`/`min`/`range` and `Ops.ordered` dispatch are deferred
  to R-39.)
- **Matrix cross products** (R-36): **`crossprod(x, y)`** = `t(x) %*% y` and
  **`crossprod(x)`** = `t(x) %*% x` (the Gram matrix `X'X`); **`tcrossprod(x, y)`** =
  `x %*% t(y)` and **`tcrossprod(x)`** = `x %*% t(x)` (`XX'`). The second argument
  defaults to the first. Defined purely in terms of the existing R-11 `t()` and
  `%*%` — the implementation calls the public `t()` builtin and the evaluator's
  `matrix_multiply`, so it inherits that handler's `MAX_SEQ_LEN` allocation guard
  and `"non-conformable arguments"` error (no new linear algebra, no unchecked
  `nrow*ncol` multiply). `crossprod(matrix(c(1,2,3,4), nrow=2))` →
  `[[5,11],[11,25]]`; `tcrossprod` of the same → `[[10,14],[14,20]]`; non-square
  `matrix(1:6, nrow=2)` gives a 3×3 `crossprod` and 2×2 `tcrossprod`.
- **Cholesky factorization** (R-40): **`chol(x)`** — the Cholesky factor of a
  real symmetric positive-definite `n×n` matrix. Returns the **upper-triangular**
  `R` with **`t(R) %*% R == x`** (R's convention — the upper factor, `R'R = X`).
  Uses the Cholesky–Banachiewicz recurrence and reads only the **upper triangle**
  of `x` (like R's default). Reuses the existing `square_matrix` reader (shared
  with `det`/`solve`) for non-matrix / non-square / over-cap rejection and the
  `SValue::Matrix` constructor; column-major throughout. The diagonal pivot is
  checked `> 0` **before** the `sqrt`, so a non-positive-definite matrix is a
  clean *"…not positive definite"* error (no `NaN`, no panic); `NA` in the upper
  triangle errors too. `chol(matrix(c(4,2,2,3), nrow=2))` is `[[2,1],[0,√2]]`
  with `t(R) %*% R` reconstructing the input; `chol(diag(3))` is the identity.
  `pivot=TRUE`, `chol2inv()`, and complex matrices are deferred to R-41.
- **Triangular solves** (R-41): **`backsolve(r, x)`** / **`forwardsolve(l, x)`** —
  solve an upper- (resp. lower-) triangular system `r %*% y = x` by back- (resp.
  forward-) substitution. The right-hand side `x` is a length-`n` vector (→ a
  vector) or an `n × m` matrix (→ one solved column per RHS), the same contract as
  `solve`. `backsolve(matrix(c(2,0,1,3), nrow=2), c(5,9))` is `c(1, 3)` and
  `r %*% y` reconstructs `c(5,9)`. Reuses the `square_matrix` reader; a zero on the
  diagonal is a clean *singular*-matrix error (no `NaN`/panic). **R-42** adds the
  base-R options: **`k=`** (use the leading `k×k` block + first `k` rows of `x`),
  **`upper.tri=`** (which triangle to read — so `backsolve(L, x, upper.tri=FALSE)`
  equals `forwardsolve(L, x)`), and **`transpose=TRUE`** (solve `t(R) %*% y = x`).
- **Matrix norms** (R-43): **`norm(x, type = "O")`** — collapses a numeric matrix
  to a single non-negative "size". The one-letter, **case-insensitive** `type`
  picks the norm: **`"O"`/`"1"`** one-norm (max absolute **column** sum, R's
  default), **`"I"`** infinity-norm (max absolute **row** sum), **`"F"`/`"E"`**
  Frobenius/Euclidean (`sqrt(Σ x[i,j]²)`), **`"M"`** max-modulus (max `|x[i,j]|`).
  A bare numeric vector becomes an `n×1` column, so `norm(c(3,4), "F")` is `5`;
  `type` may be positional or named. Reuses the shared `matrix_parts` reader
  (column-major, **rectangular** — not `square_matrix`) and `as_double` for the
  vector promotion. Any `NA` entry → `NA`; an unknown `type` is a clean error (no
  panic); the Frobenius sum-of-squares accumulates in `f64` (no overflow).
  `norm(matrix(c(1,2,3,4), nrow=2))` is `7`, `…, "I")` is `6`, `…, "F")` is
  `sqrt(30) ≈ 5.477`. `type = "2"` (the spectral norm, needs an SVD) is deferred
  to R-48 with a clear error.
- **Kronecker product** (R-38): **`kronecker(X, Y)`** — the `(m·p)×(n·q)`
  block-outer product of an `m×n` `X` and a `p×q` `Y`, where block `(i, j)` is
  `X[i,j] · Y` and `result[(i-1)·p+k, (j-1)·q+l] = X[i,j] · Y[k,l]`
  (column-major). Reuses the existing matrix accessor and `SValue::Matrix`
  constructor; a bare vector promotes to an `n×1` column. The output is
  *quadratic* in the inputs, so the result row count `m·p`, column count `n·q`,
  and their product are each formed with `checked_mul` and bounded by the same
  `MAX_SEQ_LEN` cap (an over-large product errors instead of OOMing; `0×n`/`m×0`
  inputs give an empty result, no OOB). `kronecker(matrix(c(1,2,3,4), nrow=2),
  matrix(c(0,1,1,0), nrow=2))` is a 4×4 block matrix; `kronecker(matrix(5), Y)`
  is `5·Y`. The R `%x%` infix alias is deferred to R-40 (grammar work).
- **Base R Date support** (R-44): a **`Date`** is a numeric vector of **days since
  the Unix epoch 1970-01-01** carrying class `"Date"` — modelled with the existing
  transparent `SValue::Classed { inner: Double, class: ["Date"] }` wrapper, so
  **no new value variant** and every coercion / `arithmetic` call sees through to
  the day count. **`as.Date(x, format=)`** parses a character vector (default
  `"%Y-%m-%d"`, or `"%Y/%m/%d"` etc. via `format=`, fields `%Y`/`%m`/`%d`) or wraps
  a numeric as days-since-epoch; malformed/out-of-range strings → `NA`, never a
  panic. **`format.Date(d, format=)`** and the **`format()`** generic render with
  `%Y`/`%m`/`%d`/`%j`. **`Sys.Date()`** is today (wall clock; tested for structure
  only). **`difftime(d1, d2)`** and **`d1 - d2`** give the difference in **days**.
  **`weekdays(d)`** names the day (anchored on 1970-01-01 = Thursday,
  `(days+4).rem_euclid(7)` so pre-epoch counts never panic). **`as.numeric`** /
  **`as.double`** added as base coercions (a Date → days-since-epoch; a factor →
  codes). The calendar uses Howard Hinnant's dependency-free
  `days_from_civil`/`civil_from_days` (leap years and negative dates handled).
  Parse safety: bounded `i64` digit accumulation rejects absurd years before
  overflow; impossible days rejected via a civil round-trip.
- **Date/time completeness** (R-45): extends the R-44 Date builtins in place
  (same kernel, no new dependency). **`format`/`format.Date`** gain `%B` (full
  month name), `%b` (abbreviated month), `%A` (full weekday), `%a` (abbreviated
  weekday), and `%e` (space-padded day) —
  `format(as.Date("2021-01-15"), "%B %d, %Y")` → `"January 15, 2021"`.
  **`as.Date`** parses `%B`/`%b` (case-insensitive), `%A`/`%a`, and `%e`, and
  accepts the format as its 2nd positional argument —
  `as.Date("15 Jan 2021", "%d %b %Y")` → `2021-01-15`; a bad name → `NA`.
  **`seq(from, to, by)` / `seq.Date`** generate a Date sequence with `by` a
  number of days or `"day"`/`"week"`/`"month"`/`"year"` (optionally
  `"2 weeks"`); month/year steps clamp the day to month length, and
  `length.out=` is an alternative to `to`. Output length is `MAX_SEQ_LEN`-capped
  with checked arithmetic before allocating. **`months(d)`** → full month name;
  **`quarters(d)`** → `"Q1"`..`"Q4"`. Deferred to R-46: `POSIXct`/`POSIXlt` &
  timezones, `%H`/`%M`/`%S`/`%p`, `%U`/`%W`, locale names, compound `by=`.
- The **`d`/`p`/`q`/`r` distribution family** (R-8) over `statistics-core`:
  density/CDF/quantile/sampling for the normal, uniform, and exponential
  distributions, plus `set.seed` for a reproducible per-session RNG.
- **Matrices on the shared substrate (MXF-4)** — the matrix product `%*%` routes
  through [`array_runtime::execute`]`(MatMul, …)` at `DType::F64`, so R's matmul
  gets cost-based CPU/GPU dispatch at full double precision, with results
  **bit-identical** to the previous loop (R's column-major `Matrix` matches
  `array_runtime::Array`'s layout, and the `f64` kernel folds the contraction in
  the same order). NA-bearing products fall back to the loop to preserve R's
  exact NA bit pattern. `t()`, `rowSums`/`colSums`, `diag`, `solve`, and `det`
  stay on their own implementations — no clean substrate primitive exists yet.

## Quick start

```rust
use coding_adventures_s_runtime::{eval_s, format_value};

let value = eval_s("x <- c(1, 2, 3)\nmean(x)\n").unwrap();
assert_eq!(format_value(&value), vec!["[1] 2".to_string()]);
```

For a persistent session, construct an `Interpreter` and call
`Interpreter::eval_str` repeatedly — bindings persist between calls, and a
visible top-level result is auto-printed through the S3 `print` generic.

## Testing

```sh
cargo test -p coding-adventures-s-runtime
```

See [`code/specs/S00-s-language.md`](../../../specs/S00-s-language.md) for the
full specification (including the §V2 additions).
