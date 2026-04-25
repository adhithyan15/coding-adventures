# Changelog

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
