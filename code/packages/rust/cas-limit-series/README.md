# cas-limit-series (Rust)

Limit (direct substitution) and polynomial Taylor expansion over symbolic IR.

Rust port of the Python `cas-limit-series` package.

## Operations

### `limit_direct(expr, var, point) -> IRNode`

Computes `lim_{var → point} expr` by direct substitution. Does **not** simplify the result — pass through `cas_simplify::simplify` afterwards.

Returns an unevaluated `Limit(expr, var, point)` node only if the substituted result is a literal `Div(0, 0)` (basic indeterminate form detection).

### `taylor_polynomial(expr, var, point, order) -> Result<IRNode, PolynomialError>`

Truncated Taylor expansion of a polynomial expression:

```text
Σ_{k=0..order}  (1/k!) · p^(k)(point) · (var − point)^k
```

Only polynomial inputs are accepted (`Add`, `Sub`, `Neg`, `Mul`, `Pow` with non-negative integer exponents, numeric literals, the expansion variable). Transcendental functions raise `PolynomialError`.

## Usage

```rust
use cas_limit_series::{limit_direct, taylor_polynomial};
use symbolic_ir::{apply, int, sym, ADD, POW, MUL};

// Limit
let x = sym("x");
let expr = apply(sym(MUL), vec![int(2), x.clone()]);
let lim = limit_direct(expr, &x, int(3));
// lim == Mul(2, 3)  (un-simplified; pass through simplify)

// Taylor series
let poly = apply(sym(POW), vec![x.clone(), int(2)]);
let taylor = taylor_polynomial(&poly, &x, &int(0), 2).unwrap();
// taylor == Pow(x, 2)  (x^2 expanded around 0 to order 2)
```

## Head constants

| Constant | Value |
|----------|-------|
| `LIMIT` | `"Limit"` |
| `TAYLOR` | `"Taylor"` |
| `SERIES` | `"Series"` |
| `BIG_O` | `"BigO"` |

## Stack position

```
symbolic-ir  ←  cas-substitution  ←  cas-limit-series
```
