# cas-simplify (Rust)

Algebraic simplification of symbolic IR trees.
Rust port of the Python `cas-simplify` package.

## Pipeline

```
canonical  →  numeric_fold  →  identity_rules  →  (repeat to fixed point)
```

Each pass runs bottom-up until no pass changes the expression:

| Pass | What it does |
|------|-------------|
| `canonical` | Flatten nested `Add`/`Mul`, sort commutative args, drop singleton `Add(x)` → `x`, collapse empty `Add()` → `0` / `Mul()` → `1` |
| `numeric_fold` | Collapse adjacent numeric literals: `Add(2, 3, x)` → `Add(5, x)` |
| `identity_rules` | Pattern-matching rewrites: `x+0→x`, `x*1→x`, `x^0→1`, … |

## Usage

```rust
use symbolic_ir::{apply, int, sym, ADD, MUL, POW};
use cas_simplify::simplify;

// Add(x, 0) → x
let expr = apply(sym(ADD), vec![sym("x"), int(0)]);
assert_eq!(simplify(expr, 50), sym("x"));

// Mul(2, 3) → 6
let expr2 = apply(sym(MUL), vec![int(2), int(3)]);
assert_eq!(simplify(expr2, 50), int(6));

// Pow(x, 0) → 1
let expr3 = apply(sym(POW), vec![sym("x"), int(0)]);
assert_eq!(simplify(expr3, 50), int(1));
```

## Individual passes

All three passes are also exported for direct use:

```rust
use cas_simplify::{canonical, numeric_fold};

// Structural normalization only
let sorted = canonical(apply(sym(ADD), vec![sym("c"), sym("a"), sym("b")]));
// → Add(a, b, c)

// Constant folding only
let folded = numeric_fold(apply(sym(ADD), vec![int(2), int(3), sym("x")]));
// → Add(5, x)
```

## Identity rules included

| Rule | Identity |
|------|----------|
| `Add(x, 0) → x` | Additive identity |
| `Mul(x, 1) → x` | Multiplicative identity |
| `Mul(x, 0) → 0` | Zero product |
| `Pow(x, 0) → 1` | Zeroth power |
| `Pow(x, 1) → x` | First power |
| `Pow(1, x) → 1` | One to any power |
| `Sub(x, x) → 0` | Self-cancellation |
| `Div(x, x) → 1` | Self-cancellation |
| `Log(Exp(x)) → x` | Log/Exp inverse |
| `Exp(Log(x)) → x` | Exp/Log inverse |
| `Sin(0) → 0` | Trig at zero |
| `Cos(0) → 1` | Trig at zero |

## Polynomial expansion (`expand`)

```rust
use symbolic_ir::{apply, int, sym, ADD, POW};
use cas_simplify::expand;

// (x + 1)^2 -> 1 + x + x + x*x  (mathematically x^2 + 2x + 1 — see below)
let x_plus_1 = apply(sym(ADD), vec![sym("x"), int(1)]);
let expr = apply(sym(POW), vec![x_plus_1, int(2)]);
assert_eq!(format!("{}", expand(expr)), "Add(1, x, x, Mul(x, x))");
```

`expand` distributes `Mul` over `Add`/`Sub` and expands bounded non-negative
integer `Pow`s via square-and-multiply (`O(log n)` multiplications, not
`O(n)`), guarded against the doubly-exponential term-count blowup repeated
squaring can hit on a multi-term base.

**Honestly scoped**: `expand` does **not** collect like terms — the two `x`
terms above stay separate rather than combining into `2*x`, and `x*x` is not
folded into `x^2`. The result is always mathematically correct; it is not
always the most compact form. See the module docs (`src/expand.rs`) for the
full story, including why this differs from the "clean" example in the
Python reference's docstring (that example demonstrates a different,
single-variable-only fast path this port does not include).

## Stack position

```
symbolic-ir  ←  cas-pattern-matching  ←  cas-simplify
```
