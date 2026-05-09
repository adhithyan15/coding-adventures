# cas-multivariate (Rust)

Multivariate polynomial operations and Groebner bases over exact rationals.
Rust port of the Python `cas-multivariate` package.

## What it covers

- Sparse multivariate polynomials over Q
- Monomial orders: lex, grlex, grevlex
- Multivariate reduction and S-polynomials
- Buchberger Groebner basis computation with safety caps
- Ideal solving through lex Groebner bases and back-substitution

```rust
use cas_multivariate::{buchberger, ideal_solve, MPoly, Rational};

let f1 = MPoly::new(
    [((1, 0).into(), Rational::one()), ((0, 1).into(), Rational::one()), ((0, 0).into(), Rational::from_int(-1))],
    2,
);
let f2 = MPoly::new(
    [((1, 0).into(), Rational::one()), ((0, 1).into(), Rational::from_int(-1))],
    2,
);

let basis = buchberger(&[f1.clone(), f2.clone()], "lex").unwrap();
assert_eq!(basis.len(), 2);
assert_eq!(ideal_solve(&[f1, f2]).unwrap(), vec![vec![Rational::new(1, 2), Rational::new(1, 2)]]);
```
