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
`FALSE`; `rm("d")` removes a binding. First-class environment *values*
(`new.env()`, `environment()`, the `envir = e` argument) are deferred to R-22 and
rejected today with a clear error.

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
