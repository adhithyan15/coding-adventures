# wolfram-repl

The **W-5** interactive REPL and the `wolfram` (alias `math`) binary for the
Wolfram Language. A thin console layer over
[`wolfram-runtime`](../wolfram-runtime), which does the actual lowering to
[`symbolic-ir`](../symbolic-ir) and evaluation via
[`symbolic-vm`](../symbolic-vm).

See the spec: [`code/specs/MA04-wolfram-language.md`](../../../specs/MA04-wolfram-language.md) §7.3.

## What it does

`WolframRepl` wraps a persistent `WolframSession` and adds the Mathematica console
contract:

- **`In[n]:= ` / `... ` prompts** — the input prompt shows the next input index;
  the continuation prompt shows while a statement spans physical lines.
- **Line continuation** — a Wolfram statement is terminated by a *newline* once
  brackets are balanced and no string/comment is open. The REPL keeps reading
  while a `[ ]`/`{ }`/`( )` is open or a `"…"`/`(* *)` is unterminated, then
  submits the whole buffer. (This is the one Wolfram-specific difference from
  `maxima-repl`, which terminates on `;`/`$`.) The accumulation buffer is
  size-capped so input that never balances cannot grow memory without bound.
- **Quit / EOF** — `Quit`, `Quit[]`, `Exit`, `Exit[]` (or `quit`/`exit`), or
  Ctrl-D end the session.
- **Non-fatal errors** — a surface error prints and the session continues.

## Running

```sh
cargo run -p coding-adventures-wolfram-repl --bin wolfram
# or the historical alias:
cargo run -p coding-adventures-wolfram-repl --bin math
```

```text
Wolfram Language (on the shared symbolic stack) — one statement per line, type Quit to exit.
In[1]:= 1 + 2*3
Out[1]= 7
In[2]:= square[x_] := x^2;
In[3]:= square[5]
Out[3]= 25
In[4]:= {a, b} /. {a -> 1, b -> 2}
Out[4]= {1, 2}
In[5]:= Quit
```

## Library API

The driver logic is testable without real I/O:

```rust
use coding_adventures_wolfram_repl::{WolframRepl, ReplResponse};

let mut r = WolframRepl::new();
assert_eq!(r.feed("{1,"), ReplResponse::NeedMore);   // bracket open → keep reading
assert!(matches!(r.feed("2, 3}"), ReplResponse::Output(t) if t.contains("{1, 2, 3}")));
```

`run(reader, writer)` drives a full session over any `BufRead`/`Write`.

## Testing

```sh
cargo test -p coding-adventures-wolfram-repl
```
