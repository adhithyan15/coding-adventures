# cas-solve (Rust)

Closed-form and numeric equation solving over ℚ: linear, quadratic, cubic,
quartic, and Durand-Kerner polynomial roots.
Rust port of the Python `cas-solve` package.

## Usage

```rust
use cas_solve::{nsolve_poly, solve_cubic, solve_linear, solve_quadratic, solve_quartic, Complex, SolveResult};
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

// x^4 - 5x^2 + 4 = 0  →  {-2, -1, 1, 2}
let r4 = solve_quartic(
    Frac::from_int(1), Frac::from_int(0), Frac::from_int(-5),
    Frac::from_int(0), Frac::from_int(4),
);
assert!(matches!(r4, SolveResult::Solutions(roots) if roots.contains(&int(-2)) && roots.contains(&int(2))));

// Numeric roots for x^2 + 1 = 0.
let numeric = nsolve_poly(
    &[Complex::new(1.0, 0.0), Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
    200,
    1e-12,
);
assert_eq!(numeric.len(), 2);
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

## Quartic behavior

`solve_quartic(a, b, c, d, e)` first delegates to `solve_cubic` when `a = 0`,
then applies rational-root deflation. If no rational root exists, it solves
biquadratic quartics through the quadratic solver and uses Ferrari
factorization for general quartics whose resolvent cubic has a usable rational
root. Other resolvent cases return an empty solution list, matching the Python
reference's unevaluated fallback behavior.

## Numeric polynomial solving

`nsolve_poly(coeffs, max_iter, tol)` accepts real or complex coefficients in
decreasing degree order and normalizes by the leading coefficient. It uses the
Durand-Kerner / Weierstrass iteration to return approximate complex roots for
polynomials of arbitrary degree. `roots_to_ir` converts nearly-real roots to
`Float` nodes and complex roots to `Add(Float(re), Mul(Float(im), %i))`.
`nsolve_fraction_poly` is the exact-rational coefficient convenience wrapper.

## Stack position

```
symbolic-ir  ←  cas-solve
```
