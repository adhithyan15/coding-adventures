# Changelog

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
