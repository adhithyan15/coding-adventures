# cas-solve — Equation Solving

> **Status**: New spec. Implements the `Solve` head: find values of
> variables that satisfy one or more equations.
> Parent: `symbolic-computation.md`. Depends on `cas-factor` and
> `cas-substitution`.

## Why this package exists

`solve` is a top-three operation in any CAS. The implementation
lives in its own package because the algorithms span linear algebra
(Gaussian elimination), polynomial root-finding (closed-form for
degrees ≤ 4, numeric beyond), and equation-system manipulation
(elimination via resultants).

## Reuse story

Universal across CAS frontends. The same `Solve` head powers Maxima's
`solve`, Mathematica's `Solve[]`/`NSolve[]`/`FindRoot[]`, Maple's
`solve`/`fsolve`, SymPy's `solve`. Matlab's `solve` (Symbolic Math
Toolbox) is the same operation; even Matlab's numerical root-finders
(`fzero`, `roots`) sit on top of the same polynomial root-finding
primitive.

## Scope

In:

- `Solve(equation, var)` — solve a single equation in one variable.
- `Solve(equation_list, var_list)` — system of equations.
- Linear systems via Gaussian elimination on a coefficient matrix.
- Univariate polynomial equations via factoring + closed-form for
  degrees 1–4.
- Numeric polynomial root-finding (Aberth or Durand–Kerner) for
  degree ≥ 5.
- Trivial transcendental cases via inverse functions:
  `sin(x) = a → x = arcsin(a) + 2πn`. (Limited; full transcendental
  solving is a research problem.)

Out:

- General nonlinear systems — the literature is huge; future work.
- ODE/PDE solving — separate `cas-ode` package later.
- Inequalities — separate package.

## Public interface

```python
from cas_solve import (
    solve_linear,
    solve_polynomial,
    solve_linear_system,
    register_handlers,
)

# Solve(2*x + 3 = 7, x) -> 2
# Solve(x^2 - 5*x + 6 = 0, x) -> List(2, 3)
# Solve([x + y = 3, x - y = 1], [x, y]) -> List(Rule(x, 2), Rule(y, 1))
```

## Heads added

| Head        | Arity | Meaning                              |
|-------------|-------|--------------------------------------|
| `Solve`     | 2     | Symbolic solve.                      |
| `NSolve`    | 2     | Numerical solve (floats out).        |
| `Roots`     | 1–2   | Roots of a univariate polynomial.    |

## Algorithm

**Linear** (`a*x + b = 0`): `x = -b/a`.

**Quadratic** (`a*x^2 + b*x + c = 0`): the quadratic formula.

**Cubic and quartic**: Cardano (cubic) and Ferrari (quartic) formulae.
Both are messy but well-documented; this is the natural ceiling for
closed-form symbolic solutions.

**Degree ≥ 5**: Abel–Ruffini says no closed form in radicals exists in
general. Fall back to numeric root-finding (Durand–Kerner — simple
parallel iteration to all roots simultaneously).

**Linear systems**: Gaussian elimination with partial pivoting on a
matrix of `Fraction` coefficients (exact arithmetic).

**Transcendental**: handle a small whitelist (`sin/cos/tan/log/exp`
of a linear argument) via inverse functions. Anything else → return
the equation unevaluated.

## Test strategy

- `Solve(2*x + 3 = 7, x) = 2`.
- `Solve(x^2 - 5*x + 6 = 0, x) = {2, 3}`.
- `Solve(x^2 + 1 = 0, x) = {i, -i}` (using `IRSymbol("%i")`).
- `Solve(x^3 - 6*x^2 + 11*x - 6 = 0, x) = {1, 2, 3}`.
- `Solve(x^5 + x + 1 = 0, x)` — numeric (`NSolve`).
- `Solve([2*x + y = 5, x - y = 1], [x, y]) = {x = 2, y = 1}`.
- Coverage: ≥85%.

## Package layout

```
code/packages/python/cas-solve/
  src/cas_solve/
    __init__.py
    linear.py
    quadratic.py
    cubic.py
    quartic.py
    durand_kerner.py
    linear_system.py
    transcendental.py
    solve.py             # the orchestrator
    py.typed
  tests/
    test_linear.py
    test_quadratic.py
    test_cubic.py
    test_quartic.py
    test_durand_kerner.py
    test_linear_system.py
```

Dependencies: `coding-adventures-symbolic-ir`,
`coding-adventures-polynomial`,
`coding-adventures-polynomial-bridge`,
`coding-adventures-cas-factor`,
`coding-adventures-cas-substitution`,
`coding-adventures-cas-simplify`.
