# S REPL

An interactive Read-Eval-Print loop — and the `s` command-line binary — for the
historical [S programming language](https://en.wikipedia.org/wiki/S_(programming_language))
(Bell Labs, 1976), the ancestor of R.

## What it does

Wraps a persistent `coding-adventures-s-runtime` interpreter with the behaviors
an interactive session needs:

- **Continuation** — an incomplete statement (unbalanced `(`/`[`/`{` or an open
  string) keeps reading with a `+ ` prompt until it is whole.
- **Auto-print** — a *visible* top-level result is printed in S's `[i]`-prefixed
  vector layout. Assignments and loops are invisible.
- **`print()` output** — anything a program prints is shown before the
  auto-printed result.

## Running it

```sh
cargo run -p coding-adventures-s-repl --bin s
```

```text
S — historical Bell Labs S (v1). Type q() to quit.
> x <- c(1, 2, 3)
> mean(x)
[1] 2
> x * 10 + c(1, 2)
[1] 11 22 31
> y _ c(5, 7)        # the historical underscore assignment
> sum(y)
[1] 12
> q()
```

## Library use

```rust
use coding_adventures_s_repl::{SRepl, ReplResponse};

let mut repl = SRepl::new();
assert_eq!(repl.feed("x <- c(1, 2, 3)"), ReplResponse::Output(String::new()));
assert_eq!(repl.feed("mean(x)"), ReplResponse::Output("[1] 2\n".to_string()));
```

## Why it is hand-rolled rather than built on the `repl` crate

The generic `repl` crate evaluates on a background thread and so requires the
language backend to be `Send + Sync`. The S interpreter is deliberately
single-threaded — its environments are `Rc<RefCell<…>>`, which S closures share
and mutate — so it cannot meet that bound. An S session is inherently sequential
anyway (each line mutates the global environment), so a direct single-threaded
driver is the right fit.

## Testing

```sh
cargo test -p coding-adventures-s-repl
```

See [`code/specs/S00-s-language.md`](../../../specs/S00-s-language.md) for the
full specification.
