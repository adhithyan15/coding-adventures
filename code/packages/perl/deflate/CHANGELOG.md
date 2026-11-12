# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

### Fixed

- `BUILD_windows` passes dependency library paths with `-I` instead of `set PERL5LIB=...`, which MSYS perl on CI splits on `:` and so never found.

## 0.1.0 — 2026-04-12

### Added

- Initial implementation of CMP05 DEFLATE compression and decompression.
- `compress($data)` — two-pass DEFLATE encoder.
- `decompress($data)` — CMP05 wire-format decoder.
- Length and distance code tables with extra bits.
- LSB-first bit stream packing/unpacking.
- Dependencies on `CodingAdventures::LZSS` (CMP02) and `CodingAdventures::HuffmanTree` (DT27).
