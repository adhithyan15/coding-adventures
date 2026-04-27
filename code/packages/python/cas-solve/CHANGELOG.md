# Changelog

## 0.4.0 — 2026-04-27

Phase 2 — Cubic and quartic closed-form solvers.

- ``solve_cubic(a, b, c, d)`` — closed form for monic cubics:
  - Rational-root theorem pass (rational roots → deflation to quadratic).
  - Cardano's formula for ``D > 0`` (one real + two complex roots).
  - Repeated-root case (``D = 0``).
  - Casus irreducibilis (``D < 0``, three real irrational roots) returns
    ``[]`` — cannot be expressed in real radicals.
  - Uses ``Cbrt`` IR head for symbolic cube roots.
- ``solve_quartic(a, b, c, d, e)`` — closed form for quartics via Ferrari:
  - Delegates to ``solve_cubic`` when leading coefficient is zero.
  - Rational-root theorem pass first.
  - Biquadratic special case (no odd-degree terms).
  - Full Ferrari method via resolvent cubic (requires rational resolvent
    root; returns ``[]`` otherwise).
- 21 new cubic tests + 13 new quartic tests.
- Type-checked, ruff- and mypy-clean.

## 0.1.0 — 2026-04-25

Initial release — Phase 1.

- ``solve_linear(a, b)`` — closed form for ``a·x + b = 0``.
- ``solve_quadratic(a, b, c)`` — closed form for ``a·x² + b·x + c = 0``,
  including complex roots via ``%i`` and exact ``Sqrt`` for irrational
  discriminants.
- ``SOLVE``, ``NSOLVE``, ``ROOTS`` head sentinels.
- Type-checked, ruff- and mypy-clean.

Deferred to Phase 2: cubic/quartic closed forms, Durand–Kerner numeric,
linear systems via Gaussian elimination.
