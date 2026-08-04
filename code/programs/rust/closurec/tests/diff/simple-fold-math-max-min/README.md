# simple-fold-math-max-min

End-to-end fixture for folding static `Math.max(…)` / `Math.min(…)`
(ECMAScript §21.3.2.24 / .25) on numeric-literal arguments to a numeric literal
at `--compilation_level SIMPLE`.

When every argument is a numeric literal, the largest / smallest is known at
compile time and the call collapses to that literal.

## What it checks

| input expression    | SIMPLE output    | why                                   |
|---------------------|------------------|---------------------------------------|
| `Math.max(1, 2, 3)` | `3`              | largest argument                      |
| `Math.min(1, 2, 3)` | `1`              | smallest argument                     |
| `Math.max(-5, -1)`  | `-1`             | negatives handled                     |
| `Math.max(7)`       | `7`              | single argument                       |
| `Math.max(1, x)`    | `Math.max(1,x)`  | declined — `x` is a non-literal       |
| `Math.max()`        | `Math.max()`     | declined — would be `-Infinity`       |
| `m.max(1, 2)`       | `m.max(1,2)`     | declined — not the bare global `Math` |

Expected SIMPLE stdout:

```text
var a=3,b=1,c=-1,d=7,e=Math.max(1,x),f=Math.max(),g=m.max(1,2);report(a,b,c,d,e,f,g);
```

Each result flows into `report(...)` so it stays referenced past
remove-unused-vars and the fold is observable.

## Soundness

The fold applies only when there is at least one argument and **all** arguments
are numeric literals — so there is no `ToNumber` side effect and the result is a
definite finite number (`Infinity` and `NaN` are global identifiers, never
numeric literals, so a non-literal argument is declined). Signed zero follows
the spec exactly (`Math.max` prefers `+0`, `Math.min` prefers `-0`). The empty
call and a non-global receiver are declined. Declining is always safe.

## How to run

```bash
cd code/programs/rust/closurec
cargo run -- --compilation_level SIMPLE \
  --js tests/diff/simple-fold-math-max-min/input/a.js
```

The integration test `tests/diff_simple_fold_math_max_min.rs` asserts the
byte-exact stdout, the per-binding folds, and that the typed SIMPLE pipeline ran
(not the WHITESPACE_ONLY fallback, under which all seven calls would survive).
