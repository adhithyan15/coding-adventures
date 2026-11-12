# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## [0.01] - 2026-04-05

### Added

- Initial implementation of SHA-256 (FIPS 180-4)
- One-shot API: `sha256()` returns 32-byte raw digest, `sha256_hex()` returns 64-char hex string
- Streaming API: `new()`, `update()`, `digest()`, `hex_digest()`, `copy()`
- Literate programming style with extensive inline documentation
- Full test suite with NIST test vectors, boundary conditions, and streaming tests
