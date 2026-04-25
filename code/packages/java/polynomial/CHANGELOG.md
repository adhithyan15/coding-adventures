# Changelog — polynomial (Java)

## 0.1.0 — 2026-04-24

### Added

- `FieldOps.java` — interface abstracting coefficient field operations.
  Two built-in implementations: `INTEGER_OPS` (ordinary integer arithmetic)
  and `GF256_OPS` (GF(2^8) via the local gf256 package).
- `Polynomial.java` — static utility class with all polynomial operations:
  `normalize`, `degree`, `add`, `sub`, `mul`, `divmod`, `divide`, `mod`,
  `evaluate` (Horner's method), `gcd` (Euclidean algorithm).
- Full JUnit Jupiter test suite (`PolynomialTest.java`) covering:
  - All spec MA00 worked examples (add, sub, mul, divmod, evaluate, gcd)
  - GF(256) specific tests: XOR addition, RS generator polynomial construction,
    root evaluation, exact division
  - Edge cases: zero polynomial, empty inputs, division theorem verification
- `BUILD` and `BUILD_windows` scripts for the monorepo build tool.
- Composite build (`includeBuild("../gf256")`) so the gf256 dependency resolves
  locally without requiring a Maven publish step.
