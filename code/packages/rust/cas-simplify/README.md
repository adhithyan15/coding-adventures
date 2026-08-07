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

// (x + 1)^2 -> 1 + 2*x + x^2
let x_plus_1 = apply(sym(ADD), vec![sym("x"), int(1)]);
let expr = apply(sym(POW), vec![x_plus_1, int(2)]);
assert_eq!(format!("{}", expand(expr)), "Add(1, Mul(2, x), Pow(x, 2))");
```

`expand` distributes `Mul` over `Add`/`Sub`, expands bounded non-negative
integer `Pow`s via square-and-multiply (`O(log n)` multiplications, not
`O(n)`, guarded against the doubly-exponential term-count blowup repeated
squaring can hit on a multi-term base), and **collects like terms**
(`collect_terms`): repeated monomials combine and their coefficients sum
(`x + x` → `2*x`), and repeated multiplication folds into a power (`x*x` →
`x^2`). See the module docs (`src/collect_terms.rs`) for the full four-step
algorithm, and `src/expand.rs` for why this differs from the "clean" example
in the Python reference's docstring (that example demonstrates a different,
single-variable-only fast path this port does not include, reaching the
same collected form via polynomial arithmetic rather than monomial
grouping).

## Stack position

```
symbolic-ir  ←  cas-pattern-matching  ←  cas-simplify
```
