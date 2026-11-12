# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- `BUILD_windows` now runs `cabal test all` unconditionally. It used to run Cabal only if
  `%ProgramFiles%\cabal\bin\cabal.exe` existed, which it never does on the CI runner, so the
  tests were silently skipped on Windows.

### Fixed

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## [0.1.0] — 2026-04-20

### Added

- Initial pure-Haskell Argon2id implementation following RFC 9106.
- `argon2id` / `argon2idHex` high-level API.
- `argon2Version` constant (`0x13`).
- Hybrid `fillSegment` that picks data-INDEPENDENT addressing during
  `r == 0 && sl < 2` and data-DEPENDENT addressing everywhere else,
  matching the Argon2id specification.
- Shared G-mixer, permutation P, compression G, H' variable-length
  hash, `index_alpha`, and input validation identical to the sibling
  `argon2d` / `argon2i` packages.
- Hspec test suite matching the RFC 9106 §5.3 canonical vector plus
  validation, determinism, input-binding, tag-length variants, and a
  cross-variant distinctness check.
