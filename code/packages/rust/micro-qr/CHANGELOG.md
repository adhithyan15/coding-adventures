# Changelog — micro-qr

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of the Micro QR Code encoder (ISO/IEC 18004:2015 Annex E).
- Supports all four symbol versions: M1 (11×11), M2 (13×13), M3 (15×15), M4 (17×17).
- Supports all applicable encoding modes: numeric, alphanumeric, byte.
- Supports all ECC levels: Detection (M1), L and M (M2, M3, M4), Q (M4 only).
- Auto-selects the smallest symbol + ECC combination that fits the input.
- Correct RS encoder: GF(256)/0x11D, b=0 convention, single block (no interleaving).
- Correct grid structure: single 7×7 finder pattern, L-shaped separator, timing at
  row 0 and col 0, format information at row 8 / col 8 (single copy).
- 4-mask evaluation with ISO 18004 4-rule penalty scoring (rules 1–4).
- Pre-computed format information table (all 32 entries) with XOR mask 0x4445.
- M1 half-codeword handling: last data codeword contributes only 4 bits.
- Public API: `encode()`, `layout_grid()`.
- Full error hierarchy: `MicroQRError` with variants `InputTooLong`, `UnsupportedMode`,
  `ECCNotAvailable`, `InvalidCharacter`, `LayoutError`.
- 44 unit tests + 1 doctest, all passing.
- Literate programming style: all algorithms explained inline with diagrams and examples.
