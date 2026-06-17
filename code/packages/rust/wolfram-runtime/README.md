# wolfram-runtime

The **W-4** runtime of the Wolfram-language lane: it takes the parsed
M-expression AST from
[`wolfram-parser`](../wolfram-parser), **lowers** it to the shared
[`symbolic-ir`](../symbolic-ir) term representation, and **evaluates** it with
[`symbolic-vm`](../symbolic-vm) — reusing the same symbolic substrate that
Macsyma/Maxima drive rather than writing a bespoke evaluator.

See the spec: [`code/specs/MA04-wolfram-language.md`](../../../specs/MA04-wolfram-language.md) §7.

## What it does

```text
  Wolfram source
       │  wolfram-parser::parse            (W-3)
  GrammarASTNode  (additive, power, postfix, list, …)
       │  this crate: lower
  symbolic_ir::IRNode  (Add, Mul, Pow, List, Rule, …)
       │  ├─ ReplaceAll? → cas-pattern-matching::rewrite
       │  symbolic_vm::VM over SymbolicBackend
  symbolic_ir::IRNode  (evaluated)
       │  this crate: print
  Wolfram surface string  (infix, f[…], {…})
```

"Everything is `head[args]`" (Wolfram's defining idea) makes this a *lowering*,
not a translation: `2 + 3` is `Plus[2, 3]` is `Add(2, 3)`, which the
`SymbolicBackend` folds to `5`. The whole rewrite engine — numeric folding,
algebraic identities, the elementary-function handlers, user-defined functions —
is the *same* handler table Macsyma uses.

### The head-name bridge

The one subtlety: Wolfram's **surface** head names are not the IR's **canonical**
head names. The VM is keyed on `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`; Wolfram speaks
`Plus`/`Subtract`/`Times`/`Divide`/`Power`/`Minus`. The lowering bridges them in
both directions of entry — the infix operators *and* an explicit head-application
like `Plus[1, 2, 3]` map to the same IR head — so `1 + 2` and `Plus[1, 2]`
evaluate identically.

| Surface | IR head | | Surface | IR head |
|---------|---------|-|---------|---------|
| `+` `Plus` | `Add` | | `==` `Equal` | `Equal` |
| `-` `Subtract` | `Sub` | | `<` `Less` | `Less` |
| `*` `Times` | `Mul` | | `&&` `And` | `And` |
| `/` `Divide` | `Div` | | `\|\|` `Or` | `Or` |
| `^` `Power` | `Pow` | | `!` `Not` | `Not` |
| unary `-` | `Neg` | | `{…}` `List` | `List` |
| `=` `Set` | `Assign` | | `:=` `SetDelayed` | `Define` |

`Sin`/`Cos`/`Exp`/`Log`/`Sqrt`/… are already IR head names and pass through; an
unknown `f[…]` also passes through unevaluated (Mathematica semantics). Patterns
(`_`, `x_`, `_h`, `x_h`) and rules (`->`, `:>`) lower to the
[`cas-pattern-matching`](../cas-pattern-matching) node shapes, and `expr /. rules`
is run through that crate's `rewrite`.

## Usage

```rust
use coding_adventures_wolfram_runtime::{eval, WolframSession};

// One-shot:
assert_eq!(eval("1 + 2*3\n").unwrap(), "Out[1]= 7\n");
assert_eq!(eval("Power[2, 10]\n").unwrap(), "Out[1]= 1024\n");
assert_eq!(eval("{1 + 1, 2*3}\n").unwrap(), "Out[1]= {2, 6}\n");
assert_eq!(eval("x /. x -> 5\n").unwrap(), "Out[1]= 5\n");

// Stateful (bindings and definitions persist):
let mut s = WolframSession::new();
s.feed("square[x_] := x^2;\n").unwrap();   // `;` suppresses display
assert_eq!(s.feed("square[5]\n").unwrap(), "Out[2]= 25\n");
```

A `;` at the end of a line suppresses that result's display (the notebook
convention) but the statement still runs and still advances the `Out[n]` counter.

## Robustness

`feed` is the trust boundary for the whole reused stack, so — mirroring
`maxima-runtime` — it guards against crafted input: an input-size cap
(`MAX_INPUT_LEN`), a per-statement token cap (`MAX_STATEMENT_TOKENS`, measured on
the real lexer token stream) that bounds parse-tree depth so deep nesting cannot
overflow the stack, and a bounded worker thread with `catch_unwind` plus
session-rebuild so a panic becomes a clean `Err` rather than a crash.

## Where it fits

- **W-1** spec + grammar, **W-2** `wolfram-lexer`, **W-3** `wolfram-parser`
  (all merged) — the frontend.
- **W-4** (this crate) — lowering + evaluation over the shared symbolic engine.
- **W-5** [`wolfram-repl`](../wolfram-repl) — the interactive `wolfram`/`math`
  binary on top of this crate.
- **W-6** — the full `cas-*` function surface under Wolfram names
  (`Simplify`, `Expand`, `Factor`, `Solve`, …), a later item.

## Testing

```sh
cargo test -p coding-adventures-wolfram-runtime
```
