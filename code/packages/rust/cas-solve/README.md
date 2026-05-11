# cas-solve (Rust)

Closed-form equation solving over ℚ (Phase 1: linear, quadratic, and cubic).
Rust port of the Python `cas-solve` package.

## Usage

```rust
use cas_solve::{solve_cubic, solve_linear, solve_quadratic, SolveResult};
use cas_solve::frac::Frac;
use symbolic_ir::{int, rat};

// 2x + 3 = 0  →  x = -3/2
let r = solve_linear(Frac::from_int(2), Frac::from_int(3));
assert_eq!(r, SolveResult::Solutions(vec![rat(-3, 2)]));

// x^2 - 5x + 6 = 0  →  {2, 3}
let r2 = solve_quadratic(
    Frac::from_int(1), Frac::from_int(-5), Frac::from_int(6),
);
assert_eq!(r2, SolveResult::Solutions(vec![int(2), int(3)]));

// x^3 - 6x^2 + 11x - 6 = 0  →  {1, 2, 3}
let r3 = solve_cubic(
    Frac::from_int(1), Frac::from_int(-6), Frac::from_int(11), Frac::from_int(-6),
);
assert_eq!(r3, SolveResult::Solutions(vec![int(1), int(2), int(3)]));
```

## SolveResult

```rust
pub enum SolveResult {
    Solutions(Vec<IRNode>),  // empty = no solution or unevaluated fallback
    All,                     // 0 = 0: every x satisfies
}
```

## Discriminant cases for quadratics

| Discriminant | Result |
|--------------|--------|
| Perfect-square rational | Rational roots (exact) |
| Positive, not a perfect square | `Div(Add/Sub(-b, Sqrt(disc)), 2a)` |
| Zero | Single repeated rational root |
| Negative | Complex roots `r ± k·%i` (Maxima `%i` imaginary unit) |

## Cubic behavior

`solve_cubic(a, b, c, d)` first delegates to `solve_quadratic` when `a = 0`.
Otherwise it finds exact rational roots, deflates to a quadratic, and
deduplicates repeated roots. If no rational root exists, it uses Cardano's
formula with symbolic `Cbrt`, `Sqrt`, and `%i` IR nodes for the one-real /
two-complex branch. The casus irreducibilis branch returns an empty solution
list, matching the Python reference's unevaluated fallback behavior.

## Stack position

```
symbolic-ir  ←  cas-solve
```
