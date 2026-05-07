# Changelog — CodingAdventures.MicroQR

All notable changes to this package follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-05-06

### Added

- Initial implementation of ISO/IEC 18004:2015 Annex E Micro QR Code encoder.
- `MicroQR.Encode(string, MicroQRVersion?, MicroQREccLevel?)` — main entry point,
  returns a `ModuleGrid` (from `CodingAdventures.Barcode2D`).
- Auto-selection of smallest fitting symbol (M1→M4-Q order).
- Encoding modes: numeric, alphanumeric (45-char set), byte (UTF-8).
- All 8 valid (version, ECC) symbol configurations from Annex E.
- Reed-Solomon ECC over GF(256)/0x11D with b=0 convention, single block.
  Generator polynomials for ECC counts: 2, 5, 6, 8, 10, 14.
- 7×7 finder pattern at top-left corner, L-shaped separator, timing at row 0/col 0.
- Two-column zigzag data placement from bottom-right.
- 4 mask patterns with full 4-rule penalty evaluation (rules 1–4).
- Pre-computed 32-entry format information table (8 symbols × 4 masks), XOR masked
  with 0x4445 (not 0x5412 as in regular QR).
- 15-bit format information written in single L-shaped strip (row 8 + col 8).
- M1 special handling: 20-bit data capacity (2 full bytes + 4-bit half-codeword).
- Comprehensive error types: `InputTooLongException`, `UnsupportedModeException`,
  `InvalidCharacterException`, `EccNotAvailableException`.
- 70+ unit tests covering RS encoder, format table, mode selection, config
  selection, data codewords, penalty scoring, integration, and error paths.
- Literate programming style throughout — all constants, algorithms, and data
  structures fully commented with analogies, diagrams, and examples.

### Dependencies

- `CodingAdventures.Barcode2D` (project reference)
- `CodingAdventures.Gf256` (project reference)
