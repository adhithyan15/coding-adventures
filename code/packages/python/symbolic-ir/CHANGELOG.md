# Changelog

## 0.2.0 — 2026-04-20

- Added `ATAN = IRSymbol("Atan")` to the elementary-functions group in
  `nodes.py` and exported it from `__init__.py`. Required by Phase 2e
  of the symbolic integration roadmap (arctan antiderivatives for
  irreducible quadratic denominators). See `arctan-integral.md`.

## 0.1.0 — 2026-04-19

Initial release.

- Six immutable node types: `IRSymbol`, `IRInteger`, `IRRational`,
  `IRFloat`, `IRString`, `IRApply`.
- `IRRational` normalization (gcd reduction, sign in numerator,
  division-by-zero validation).
- Standard head symbols (`ADD`, `MUL`, `POW`, `D`, `Integrate`, etc.)
  as module-level singletons.
- Full test suite covering construction, equality, hashing,
  immutability, and nested-tree round trips.
