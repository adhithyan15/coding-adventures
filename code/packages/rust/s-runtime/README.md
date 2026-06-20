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
  it always terminates; non-name super-assign targets and the not-yet-supported
  `envir = e` argument fail closed. First-class environment *values* (`new.env`,
  `environment`) are deferred to R-22.
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
