## 0.538.0 - 2026-10-27

### Added

- Preserve exact sign-rooted integer provenance through additive
  subexpressions up to `i64::MAX` when an enclosing integer `div` or `mod`
  reduces the value below `2^53` before widening.
- Keep overflow, unreduced wide sums, and real division of a wide integer
  intermediate fail-closed.
