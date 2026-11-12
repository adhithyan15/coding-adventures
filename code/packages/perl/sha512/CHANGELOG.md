# Changelog

All notable changes to the SHA-512 Perl package will be documented in this file.

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## [0.02] - Unreleased

- Consume the bounded HF04 corpus and encode the full 128-bit length field with checked arbitrary-precision accounting.

## [0.01] - 2026-04-05

### Added
- Initial implementation of SHA-512 (FIPS 180-4)
- `hex($message)` function returning 128-character lowercase hex string
- `digest($message)` function returning 64-element byte arrayref
- Full FIPS 180-4 test vectors
- OO and functional interfaces
- Literate-style inline documentation explaining the algorithm
