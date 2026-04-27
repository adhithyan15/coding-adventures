# cas-factor (Rust)

Univariate integer polynomial factoring over ℤ.
Rust port of the Python `cas-factor` package.

## Phase 1: linear factors via the rational-root test

Phase 1 uses the **Rational Root Theorem**: any integer root `r` of
`a_0 + a_1·x + … + a_n·x^n` must divide `a_0`.  We enumerate all
`±divisors(a_0)` and test each.

```rust
use cas_factor::factor_integer_polynomial;

// x^2 - 1 = (x - 1)(x + 1)
let (content, factors) = factor_integer_polynomial(&[-1, 0, 1]);
assert_eq!(content, 1);
// factors = [([-1, 1], 1), ([1, 1], 1)]
//   [-1, 1] means  -1 + x  =  x - 1
//   multiplicity 1

// 2x^2 + 4x + 2 = 2*(x + 1)^2
let (c, f) = factor_integer_polynomial(&[2, 4, 2]);
assert_eq!(c, 2);
assert_eq!(f, vec![(vec![1, 1], 2)]);
// [1, 1] = 1 + x = x + 1, multiplicity 2

// x^2 + 1 — irreducible over Q (Phase 2 handles these)
let (c, f) = factor_integer_polynomial(&[1, 0, 1]);
assert_eq!(c, 1);
assert_eq!(f, vec![(vec![1, 0, 1], 1)]);
```

## Polynomial representation

Polynomials are `Vec<i64>` with the **constant term first**:

```text
index:  0   1   2
value: a_0 a_1 a_2   represents  a_0 + a_1·x + a_2·x^2
```

## Exported helpers

| Function | Description |
|----------|-------------|
| `normalize(p)` | Strip trailing zeros |
| `degree(p)` | Degree of p, -1 for zero polynomial |
| `content(p)` | GCD of all coefficients |
| `primitive_part(p)` | Divide by content |
| `evaluate(p, x)` | Evaluate at integer x |
| `divide_linear(p, root)` | Synthetic division by (x - root) |
| `divisors(n)` | All positive divisors of |n| |
| `find_integer_roots(p)` | Integer roots via rational-root test |
| `extract_linear_factors(p)` | All linear factors with multiplicities |

## Stack position

```
(no symbolic-ir dependency)
cas-factor  ←  macsyma-runtime (IR handler layer)
```
