# Changelog

All notable changes to this project will be documented in this file.

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
