# Changelog

All notable changes to `CodingAdventures::MicroQR` are documented here.

## [0.1.0] — 2026-04-24

### Added

- Initial release: ISO/IEC 18004:2015 Annex E compliant Micro QR Code encoder.
- Supports all 8 valid (version, ECC) symbol configurations:
  - M1/Detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q
- Symbol sizes: M1=11×11, M2=13×13, M3=15×15, M4=17×17.
- Three encoding modes: numeric (0-9), alphanumeric (45-char set), byte (raw bytes).
- Reed-Solomon ECC using GF(256)/0x11D with b=0 convention, single block per symbol.
- Pre-computed 32-entry FORMAT_TABLE (symbol_indicator × mask_pattern), XOR-masked with 0x4445.
- Four mask patterns (0: (r+c)%2==0; 1: r%2==0; 2: c%3==0; 3: (r+c)%3==0).
- ISO 18004 four-rule penalty scoring for mask selection.
- M1 half-codeword support (2.5 data codewords = 20 bits; last byte uses upper nibble only).
- Public API: `encode()`, `encode_at()`, `layout_grid()`.
- Exported constants: `M1`, `M2`, `M3`, `M4`, `DETECTION`, `ECC_L`, `ECC_M`, `ECC_Q`.
- Integration with `CodingAdventures::Barcode2D` for rendering via `layout_grid()`.
- Integration with `CodingAdventures::GF256` for GF(256) field arithmetic.
- 58-test suite covering dimensions, structural modules, ECC constraints,
  capacity boundaries, error handling, determinism, and cross-language corpus.
