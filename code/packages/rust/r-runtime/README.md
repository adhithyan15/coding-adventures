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
prints names above values instead of the `[i]` prefix.

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
