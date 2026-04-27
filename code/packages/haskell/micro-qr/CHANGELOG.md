# Changelog — micro-qr

## 0.1.0 — 2026-04-27

### Added

- `CodingAdventures.MicroQR` module implementing ISO/IEC 18004:2015 Annex E Micro QR Code encoder
- All 8 valid (version, ECC) symbol configurations: M1/Detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q
- `MicroQRVersion` data type: M1, M2, M3, M4
- `MicroQREccLevel` data type: Detection, L, M, Q
- Numeric mode encoding: groups of 3 digits → 10 bits, pairs → 7 bits, singles → 4 bits
- Alphanumeric mode encoding: 45-character set, pairs → 11 bits, singles → 6 bits
- Byte mode encoding: raw UTF-8 bytes, 8 bits each
- Auto-mode selection: numeric > alphanumeric > byte (most compact first)
- M1 special codeword packing: 20-bit capacity (3 codewords where last is 4-bit nibble)
- Terminator zero-bits with capacity-aware truncation
- Alternating 0xEC/0x11 padding codewords
- GF(256)/0x11D Reed-Solomon encoder with b=0 convention (roots α⁰…α^{n-1})
- Pre-computed RS generator polynomials for all 6 ECC codeword counts: {2, 5, 6, 8, 10, 14}
- Pre-computed 32-entry format information table (8 symbol indicators × 4 mask patterns)
  with XOR mask 0x4445 as specified by Micro QR standard
- Single 7×7 finder pattern placement at top-left corner
- L-shaped separator placement (row 7 and col 7, light modules)
- Timing pattern placement along row 0 and col 0 (starting at position 8)
- Format information reservation (row 8 cols 1–8, col 8 rows 1–7)
- Two-column zigzag data placement from bottom-right corner
- 4 mask patterns with full ISO penalty scoring (Rules 1–4)
- Best mask selection by minimum penalty (ties broken by lower index)
- 15-bit format information write with single-copy placement
- cwsToBits helper with M1 half-codeword support
- UTF-8 encoding for byte mode
- `encode`, `encodeAt`, `encodeAndLayout` public API functions
- `MicroQRError` type with `InputTooLong`, `ECCNotAvailable`, `UnsupportedMode`, `InvalidConfiguration` variants
- Integration with `barcode-2d`'s `ModuleGrid`, `emptyGrid`, `setModule`, `layout`
- Comprehensive hspec test suite with 40+ test cases covering:
  - All four symbol sizes (M1–M4)
  - Finder pattern and timing pattern structural invariants
  - Determinism, auto-selection, error cases, encodeAt
  - All ECC levels for M4
  - Module count sanity checks
  - encodeAndLayout round-trip
- Literate Haskell code with Knuth-style inline comments and algorithmic explanations
