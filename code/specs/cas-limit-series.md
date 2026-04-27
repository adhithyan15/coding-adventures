# cas-limit-series — Limits and Taylor Series

> **Status**: New spec. Implements the `Limit` and `Taylor`/`Series`
> heads. Parent: `symbolic-computation.md`. Depends on `cas-substitution`,
> `cas-simplify`.

## Why this package exists

Limits and Taylor series are fundamental analysis operations. They are
related (Taylor coefficient `n` is `lim_{x→a} D^n[f]/n!`) but distinct
enough to share a package and not a head.

## Reuse story

Identical operation across CAS frontends. Same algorithms power
`limit()`, `Limit[]`, `taylor()`, `Series[]`, `series()`, `Taylor[]`.

## Scope

In:

- `Limit(expr, var, point)` and `Limit(expr, var, point, direction)`
  for one-sided limits.
- Algebraic limits via direct substitution + simplification.
- Indeterminate forms `0/0`, `∞/∞` resolved via L'Hôpital's rule
  (recursive, with a depth cap).
- Common limits at infinity for rational functions and standard
  transcendentals (`sin(x)/x → 1` as `x → 0`, `(1+1/n)^n → e` as
  `n → ∞`, etc.).
- `Taylor(expr, var, point, order)` — produce the truncated Taylor
  polynomial of degree `order`.
- `Series(expr, var, point, order)` — Taylor with a Big-O remainder
  term `IRApply(Big_O, ...)` attached.

Out:

- Asymptotic series (Laurent, Puiseux) — future package.
- Multivariate limits — future.

## Public interface

```python
from cas_limit_series import limit, taylor, series, register_handlers

limit(IRApply(SIN, (x,)) / x, x, 0)   # → 1
taylor(IRApply(EXP, (x,)), x, 0, 4)   # → 1 + x + x^2/2 + x^3/6 + x^4/24
```

## Heads added

| Head     | Arity | Meaning                                     |
|----------|-------|---------------------------------------------|
| `Limit`  | 3–4   | `Limit(expr, var, point[, direction])`.     |
| `Taylor` | 4     | `Taylor(expr, var, point, order)`.          |
| `Series` | 4     | `Taylor` + `Big_O` remainder.               |
| `Big_O`  | 1+    | Asymptotic order term.                      |

## Algorithm

**Limit**:

1. Try `Subst(point, var, expr)` and simplify. If a finite value
   results, return it.
2. If `0/0` or `∞/∞`, apply L'Hôpital: differentiate numerator and
   denominator and recurse. Bound recursion at 5 levels.
3. Pattern-match common standard limits.
4. Otherwise, return `Limit(...)` unevaluated.

**Taylor**:

1. For `n` in `0..order`:
   - Compute `D^n(expr)` via repeated derivative.
   - Substitute `var = point`.
   - Multiply by `(var - point)^n / n!`.
2. Sum the resulting polynomial.

## Test strategy

- `Limit(sin(x)/x, x, 0) = 1`.
- `Limit((1 - cos(x))/x^2, x, 0) = 1/2`.
- `Limit(x^2/exp(x), x, ∞) = 0`.
- `Limit((x+1)/(x-1), x, ∞) = 1`.
- `Taylor(exp(x), x, 0, 4) = 1 + x + x^2/2 + x^3/6 + x^4/24`.
- `Taylor(sin(x), x, 0, 5) = x - x^3/6 + x^5/120`.
- Coverage: ≥85%.

## Package layout

```
code/packages/python/cas-limit-series/
  src/cas_limit_series/
    __init__.py
    limit.py
    lhopital.py
    standard_limits.py
    taylor.py
    big_o.py
    py.typed
  tests/
    test_limit.py
    test_lhopital.py
    test_taylor.py
```

Dependencies: `coding-adventures-symbolic-ir`,
`coding-adventures-symbolic-vm`,
`coding-adventures-cas-substitution`,
`coding-adventures-cas-simplify`.
