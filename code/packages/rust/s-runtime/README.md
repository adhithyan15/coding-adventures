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
```

## What "S-flavored" means here

- **Everything is a vector** — `3` is a numeric vector of length one.
- **Recycling**, **NA propagation**, and the `logical < double < character`
  **coercion** lattice.
- The historical **`_` assignment** (`x _ 5` is `x <- 5`).
- **Lexical scoping** — closures capture their defining environment.
- **v2** adds `%op%` infix operators, a broad builtin library, **S3 method
  dispatch** (a generic `print`), **factors**, and **data frames** (`$`,
  `[[ ]]`, 2-D indexing).
- The **`d`/`p`/`q`/`r` distribution family** (R-8) over `statistics-core`:
  density/CDF/quantile/sampling for the normal, uniform, and exponential
  distributions, plus `set.seed` for a reproducible per-session RNG.

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
