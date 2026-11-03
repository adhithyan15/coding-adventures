## 0.552.0 - 2026-11-03

### Added

- Preserve `entier(k * sqrt(n))` in the exact integer domain for checked static
  integer coefficients and radicands, including negative coefficients'
  ceiling-root adjustment, while overflow, multiple roots, division,
  overrides, and dynamic roots fail closed.
