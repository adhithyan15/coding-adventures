# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-08-21

### Changed

- Delegate encoding and decoding to the repository IC18 `image-codec-png`
  implementation instead of Go's standard-library PNG codec.
- Preserve all paint-facing aliases and panic/error behavior while propagating
  stable typed PNG errors and bounded-resource validation.
- Strengthen BUILD front doors with race, vet, coverage, and trimpath build
  checks, plus a real `barcode-1d` downstream validation path.

### Compatibility

- Decoding now intentionally follows the portable RGBA8 non-interlaced IC18
  profile. Palette, grayscale, 16-bit, Adam7, and APNG inputs that the standard
  library may accept are rejected with stable portable errors.

## [0.1.0] - 2026-04-13

### Added

- Initial PNG codec for converting `PixelContainer` values to and from PNG bytes.
- Convenience aliases `Encode` and `Decode` alongside `EncodePNG` and `DecodePNG`.
