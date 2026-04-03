# Changelog — coding_adventures_polynomial (Ruby)

## [0.1.0] — 2026-04-03

### Added

- Initial implementation of polynomial arithmetic over real numbers.
- All operations implemented as module methods on `Polynomial`.
- `normalize`, `degree`, `zero`, `one`, `add`, `subtract`, `multiply`.
- `divmod_poly` (named to avoid conflict with Ruby's Numeric#divmod).
- `divide`, `mod`, `evaluate` (Horner's method), `gcd`.
- Minitest test suite covering all functions and edge cases.
