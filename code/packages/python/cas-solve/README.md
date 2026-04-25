# cas-solve

Equation solving over Q (Phase 1: linear + quadratic closed-form).

## Quick start

```python
from cas_solve import solve_linear, solve_quadratic
from fractions import Fraction

# 2x + 3 = 0  →  x = -3/2
solve_linear(Fraction(2), Fraction(3))

# x^2 - 5x + 6 = 0  →  x ∈ {2, 3}
solve_quadratic(Fraction(1), Fraction(-5), Fraction(6))

# x^2 + 1 = 0  →  x ∈ {i, -i}  (uses %i for the imaginary unit)
solve_quadratic(Fraction(1), Fraction(0), Fraction(1))
```

## Phase 1 scope

- ``solve_linear(a, b)`` — solves ``a·x + b = 0``. Returns:
  - ``[x]`` for the unique solution when ``a ≠ 0``.
  - ``[]`` (no solutions) when ``a = 0, b ≠ 0``.
  - ``None`` (every x is a solution) when ``a = 0, b = 0``.
- ``solve_quadratic(a, b, c)`` — solves ``a·x² + b·x + c = 0``.
  Returns a list of roots:
  - Real roots when discriminant ≥ 0; expressed exactly using
    ``Sqrt(...)`` IR for non-perfect-square discriminants.
  - Complex roots ``r ± k·%i`` when the discriminant is negative.
  - Falls back to ``solve_linear`` if ``a = 0``.

Coefficients are :class:`fractions.Fraction` for exactness. Output is
IR; pass through ``cas_simplify.simplify`` to reduce.

## Deferred (Phase 2+)

- Cardano (cubic) and Ferrari (quartic) closed-form solutions.
- Durand–Kerner numeric root-finder for degree ≥ 5.
- Linear systems (``Ax = b``) via Gaussian elimination.
- Transcendental whitelist (``sin(x) = a`` → arcsin, etc.).

## Reuse story

Universal across CAS frontends — backs Maxima's ``solve``,
Mathematica's ``Solve[]`` / ``NSolve[]``, Maple's ``solve``, SymPy's
``solve``. The same primitives also power Matlab's ``solve``,
``fzero``, ``roots``.

## Dependencies

- `coding-adventures-symbolic-ir`
