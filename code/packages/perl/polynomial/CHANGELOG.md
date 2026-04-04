# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-03

### Added

- Initial implementation of `CodingAdventures::Polynomial`
- Functions: `normalize`, `degree`, `zero`, `one`, `add`, `subtract`,
  `multiply`, `divmod_poly`, `divide`, `modulo`, `evaluate`, `gcd_poly`
- Polynomial long division using the classical algorithm
- Euclidean GCD for polynomials with monic normalization
- Horner's method for efficient polynomial evaluation
- Knuth-style literate comments throughout
- Test suite with 40+ tests using Test2::V0
