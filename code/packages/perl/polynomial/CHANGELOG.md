# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- `BUILD_windows` added, running the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries). The Windows build previously fell back to the POSIX-only `BUILD`.

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
