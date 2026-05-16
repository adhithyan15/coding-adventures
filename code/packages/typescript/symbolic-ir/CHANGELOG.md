# Changelog

## [0.2.0] - 2026-05-16

### Added

- Added first-class reciprocal hyperbolic head symbols `COTH`, `SECH`, and
  `CSCH` for `Coth`, `Sech`, and `Csch`.
- Added eight Phase 27 named ODE solution function head symbols: `LEGENDRE_P`,
  `LEGENDRE_Q`, `BESSEL_J`, `BESSEL_Y`, `HERMITE_H`, `HERMITE_H2`,
  `CHEBYSHEV_T`, and `CHEBYSHEV_U`, covering the four classical
  variable-coefficient ODE families (Legendre, Bessel, Hermite, Chebyshev).

## [0.1.0] - 2026-05-08

### Added

- Initial pure TypeScript symbolic IR.
- Six immutable node forms: symbol, integer, rational, float, string, apply.
- Exact `bigint` rational normalization.
- Structural equality, structural keys, display strings, and standard CAS head symbols.
