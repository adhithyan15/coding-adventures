# Changelog

All notable changes to the `reed-solomon` Haskell package will be documented here.

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## [0.1.0.0] - 2026-04-24

### Added

- Initial implementation of Reed-Solomon error-correcting codes (MA02).
- `RSError` type with `TooManyErrors` and `InvalidInput` variants.
- `buildGenerator`: constructs the RS generator polynomial over GF(256).
- `encode`: systematic RS encoding (message bytes + check bytes).
- `decode`: full decoding pipeline (syndromes → Berlekamp-Massey → Chien
  search → Forney algorithm → correction).
- `syndromes`: syndrome computation (big-endian polynomial evaluation).
- `errorLocator`: exposed Berlekamp-Massey error locator computation.
- Full Hspec test suite: round-trip property, error correction up to
  capacity, TooManyErrors beyond capacity, invalid-input guards.
