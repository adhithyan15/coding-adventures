# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.1.0

- Initial Perl RESP2 package with typed values, binary-safe encoding,
  incremental decoding, inline commands, concatenated frames, and a streaming
  decoder.
