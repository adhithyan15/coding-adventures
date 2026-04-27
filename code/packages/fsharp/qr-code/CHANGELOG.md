# Changelog — CodingAdventures.QRCode (F#)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-24

### Added

- Initial release of the F# QR Code encoder.
- Full ISO/IEC 18004:2015 compliant encoder supporting versions 1–40.
- Encoding modes: Numeric, Alphanumeric, Byte (auto-selected for minimum size).
- Error correction levels: L (~7%), M (~15%), Q (~25%), H (~30%).
- Reed-Solomon ECC using GF(256) with b=0 convention (poly 0x11D), implemented
  via the `CodingAdventures.Gf256` package.
- Format information: 15-bit BCH word (generator 0x537) with ISO masking
  sequence 0x5412; placed MSB-first (f14→f9) across row 8 cols 0–5.
- Version information for symbols v7+: 18-bit BCH word (generator 0x1F25).
- All 8 mask patterns with 4-rule ISO penalty evaluation.
- Output is a `ModuleGrid` from `CodingAdventures.Barcode2D` — compatible with
  `Barcode2D.layout` for pixel-level rendering.
- 37 unit tests covering: grid size, finder patterns, format info BCH validity,
  dark module placement, timing strips, all four ECC levels, all three encoding
  modes, edge cases (empty string, boundary inputs, determinism).
