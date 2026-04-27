# Changelog

## 0.1.0 — 2026-04-25

Initial release — Phase C foundation.

- ``canonical(expr)`` — flatten, sort, drop singletons, fold empty
  containers.
- ``simplify(expr)`` — fixed-point loop: canonical + numeric-fold +
  identity-rule application.
- Identity rule database (Add/Mul/Pow/Sub/Div + a starter set of
  elementary-function identities like ``Log(Exp(x)) → x``).
- ``Simplify`` and ``Canonical`` IR head sentinels for backends to
  install handlers against.
- Type-checked, ruff- and mypy-clean.

Deferred: ``Expand``, ``Collect``, ``Together``, ``Apart``,
``cas-trig-simplify``.
