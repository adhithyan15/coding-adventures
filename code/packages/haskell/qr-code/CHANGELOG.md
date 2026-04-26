# Changelog — qr-code (Haskell)

## 0.1.0.0 — 2026-04-24

Initial release.

### Added

- `EccLevel` type: `L`, `M`, `Q`, `H` error correction levels
- `QRCodeError` type: `InputTooLong`, `LayoutError`
- `encode :: String -> EccLevel -> Either QRCodeError ModuleGrid`
  - Encodes any UTF-8 string into a QR Code module grid
  - Auto-selects minimum version (1–40) that fits the input
  - Supports Numeric, Alphanumeric, and Byte encoding modes
- `encodeAndLayout :: String -> EccLevel -> Barcode2DLayoutConfig -> Either QRCodeError PaintScene`
  - Convenience wrapper combining `encode` and `barcode-2d`'s `layout`
- `renderSvg :: String -> EccLevel -> Barcode2DLayoutConfig -> Either QRCodeError String`
  - Placeholder for SVG output (requires paint-vm-svg backend)

### Reed-Solomon

- GF(256) with primitive polynomial 0x11D (same as gf256 package)
- b=0 generator polynomial convention: `g(x) = ∏(x + αⁱ)` for i in 0..n-1
- Precomputed generator polynomials via `buildGenerator`
- LFSR-based polynomial division via `rsEncode`

### Module placement

- Three 7×7 finder patterns at the three corners
- 1-module-wide separator borders around each finder
- Alternating timing strips on row 6 and column 6
- Alignment patterns for versions 2–40 (ISO Annex E positions)
- Two-column zigzag data placement (bottom-right to top-left)
- All 8 mask patterns evaluated; lowest ISO penalty score selected
- 15-bit BCH format information (two copies, XOR mask 0x5412)
- 18-bit BCH version information for versions 7–40

### Testing

- 79 unit and integration tests covering:
  - ECC indicator mapping, symbol size formula
  - RS generator polynomial: length, monic coefficient
  - Data mode selection (Numeric/Alphanumeric/Byte)
  - Bit stream padding, block structure, interleaving
  - Format and version information BCH validity
  - Finder patterns, timing strips, dark module placement
  - All four ECC levels, determinism, error case
  - Canonical 5-input test corpus from the spec
