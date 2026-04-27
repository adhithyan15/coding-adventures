# cas-solve (Rust)

Closed-form equation solving over ℚ (Phase 1: linear and quadratic).
Rust port of the Python `cas-solve` package.

## Usage

```rust
use cas_solve::{solve_linear, solve_quadratic, SolveResult};
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
```

## SolveResult

```rust
pub enum SolveResult {
    Solutions(Vec<IRNode>),  // empty = no solution
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

## Stack position

```
symbolic-ir  ←  cas-solve
```
