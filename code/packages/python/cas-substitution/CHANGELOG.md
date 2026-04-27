# Changelog

## 0.1.0 — 2026-04-25

Initial release.

- ``subst(value, var, expr)`` — replace every occurrence of a symbol
  with a value.
- ``subst_many(rules, expr)`` — sequence of ``(var, value)`` pairs.
- ``replace_all(expr, rule)`` — Mathematica-style pattern-aware
  substitution everywhere a rule matches.
- ``replace_all_many(expr, rules)`` — list of rules.
- Type-checked, ruff- and mypy-clean.
