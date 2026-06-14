# Changelog

## 0.1.0 — 2026-04-27

Initial release — Phase 1 (TrigSimplify, TrigExpand, TrigReduce).

- `trig_simplify(expr)` — Pythagorean identity reduction and special-value
  lookup: `sin²(x)+cos²(x)→1`, `sin(-x)→-sin(x)`, `sin(π)→0`, etc.
- `trig_expand(expr)` — expand compound angles and integer multiples:
  `sin(a+b)→sin(a)cos(b)+cos(a)sin(b)`, Chebyshev recurrence for `sin(nx)`.
- `trig_reduce(expr)` — reduce powers to multiple-angle form:
  `sin²(x)→(1-cos(2x))/2`, `cos³(x)→(3cos(x)+cos(3x))/4`.
- `special_values` lookup table for `sin`/`cos`/`tan` at rational multiples
  of π up to `2π`.
- `build_trig_handler_table()` — returns handler dict for `SymbolicBackend`.
- 80+ tests across four test files.
- Type-checked, ruff- and mypy-clean.
