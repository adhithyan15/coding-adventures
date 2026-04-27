# cas-limit-series

Limit and Taylor series operations on the symbolic IR.

## Phase 1 scope

- `LIMIT`, `TAYLOR`, `SERIES`, `BIG_O` head sentinels.
- ``limit_direct(expr, var, point)`` — direct substitution. Returns a
  finite IR result if the substitution succeeds, or ``Limit(expr, var,
  point)`` unevaluated otherwise.
- ``taylor_polynomial(p, var, point, order)`` — Taylor expansion of a
  *polynomial* expression around ``point`` to order ``order``. Uses a
  pure-Python polynomial-derivative routine; doesn't depend on
  ``symbolic-vm``.

Both operations work on raw IR and return raw IR; consumers run the
result through ``cas_simplify.simplify`` to reduce.

## Deferred

- L'Hôpital's rule for ``0/0`` and ``∞/∞`` indeterminate forms (needs
  general symbolic differentiation, which lives in ``symbolic-vm`` —
  follow-up PR will add a thin wrapper).
- Taylor expansion for transcendental functions (exp, sin, cos, ...).
  Once the differentiation hook lands, the same code works.
- Asymptotic series (Laurent, Puiseux).

## Reuse story

Universal across CAS frontends — same operation in Maxima,
Mathematica, Maple, SymPy.

## Dependencies

- `coding-adventures-symbolic-ir`
- `coding-adventures-cas-substitution`
