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
