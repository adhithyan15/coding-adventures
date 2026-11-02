# Changelog

## [Unreleased]

### Fixed

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

### Added

- Execute all six normative `classical-ciphers-v1` Atbash objects in generated,
  dependency-free native tests pinned to the corpus digest and case roster.

## 0.1.0 - 2026-07-18

- Add case-preserving Atbash encryption and decryption for ASCII letters.
- Preserve punctuation, digits, whitespace, and non-ASCII characters.
- Add tests for reference examples, complete alphabets, and involution.
