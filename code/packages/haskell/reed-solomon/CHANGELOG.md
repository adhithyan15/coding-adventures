# Changelog

All notable changes to the `reed-solomon` Haskell package will be documented here.

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
