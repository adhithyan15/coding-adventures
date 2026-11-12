# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

### Security

- Hardening (CWE-789): _deserialise_tokens clamps the untrusted header count to the complete tokens present instead of looping over the claimed count. decode stops at an invalid (zero or too-large) offset instead of reading undef / wrapped indexes.
- Added t/02-hostile-headers.t (Test::More): hostile headers checked for bounded time and sane results, plus round trips over every byte string of length <= 2 and a seeded random sample. (The pre-existing t/*.t need Test2::V0, which is not installed in the local review environment.)

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZ77 encoding/decoding (CMP00 specification)
- Streaming API: `encode` and `decode` for token stream manipulation
- One-shot API: `compress` and `decompress` for byte string I/O
- Fixed-width 4-byte token serialisation using `pack`/`unpack` with formats `N` and `nCC`
- Token hashrefs `{offset, length, next_char}`
- Byte-by-byte copy in `decode` handles overlapping matches correctly
- 23 tests, all passing — Test2::V0
- Windows not supported (BUILD_windows prints skip message)
