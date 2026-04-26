# Changelog

## 0.1.0 — 2026-04-25

Initial release.

- Sentinel heads: ``LENGTH``, ``FIRST``, ``REST``, ``LAST``,
  ``APPEND``, ``REVERSE``, ``RANGE``, ``MAP``, ``APPLY``, ``SELECT``,
  ``SORT``, ``PART``, ``FLATTEN``, ``JOIN``.
- Pure-Python implementations of every operation that work on raw
  ``IRApply(LIST, ...)`` values without needing backend integration.
- 1-based indexing throughout (MACSYMA / Mathematica convention).
- Type-checked, ruff- and mypy-clean.
