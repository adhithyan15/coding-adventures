# Changelog

## 0.1.0 — 2026-04-25

Initial release — Phase 1 foundation.

- Sentinel heads: ``LIMIT``, ``TAYLOR``, ``SERIES``, ``BIG_O``.
- ``limit_direct(expr, var, point)`` — direct substitution; falls
  back to unevaluated ``Limit(...)``.
- ``taylor_polynomial(p, var, point, order)`` — Taylor expansion for
  polynomial expressions (Add, Mul, Pow, Neg of literals and a
  single ``var``). Pure-Python; no dependency on symbolic-vm.
- Type-checked, ruff- and mypy-clean.

Deferred to follow-ups: L'Hôpital, transcendental Taylor, asymptotic
series.
