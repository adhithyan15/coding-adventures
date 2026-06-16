# R REPL

An interactive Read-Eval-Print loop — and the `R` command-line binary — for the
R language, reusing the shared S evaluator.

## What it does

Wraps a persistent `coding-adventures-r-runtime` `RInterpreter` (which itself
reuses the `s-runtime` tree-walker) with the interactive behaviors a console
needs: statement continuation across unbalanced `(`/`[`/`{` and open strings,
auto-print of visible results in the `[i]`-prefixed vector layout, and surfacing
of `print()` output. It is the direct sibling of `s-repl`'s `SRepl`.

## Running it

```sh
cargo run -p coding-adventures-r-repl --bin R
```

```text
R (reusing the S evaluator) — type q() to quit.
> data_frame <- c(1, 2, 3)
> mean(data_frame)
[1] 2
> x = 5          # `=` assignment (R, not S)
> x + 1
[1] 6
> q()
```

## Library use

```rust
use coding_adventures_r_repl::{RRepl, ReplResponse};

let mut repl = RRepl::new();
repl.feed("data_frame <- c(1, 2, 3)");
assert_eq!(repl.feed("mean(data_frame)"), ReplResponse::Output("[1] 2\n".to_string()));
```

Hand-rolled rather than built on the generic `repl` crate for the same reason as
`s-repl`: the interpreter is single-threaded (`Rc<RefCell<…>>`) and cannot meet
that crate's `Send + Sync` bound.

## Testing

```sh
cargo test -p coding-adventures-r-repl
```

See [`code/specs/R00-r-language.md`](../../../specs/R00-r-language.md).
