# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.

### Fixed

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

### Added

- Execute all six normative `classical-ciphers-v1` Atbash objects in generated,
  dependency-free native tests pinned to the corpus digest and case roster.

## 0.1.0 - 2026-07-18

- Add case-preserving Atbash encryption and decryption for ASCII letters.
- Preserve punctuation, digits, whitespace, and non-ASCII characters.
- Add tests for reference examples, complete alphabets, and involution.
