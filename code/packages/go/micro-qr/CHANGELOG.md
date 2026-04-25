# Changelog — micro-qr (Go)

All notable changes to this package are documented in this file.

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of the Micro QR Code encoder for Go.
- `Encode(input string, version *MicroQRVersion, ecc *MicroQREccLevel)` — encodes
  any string to a `barcode2d.ModuleGrid`, auto-selecting the smallest symbol (M1..M4)
  and mode (numeric, alphanumeric, byte) unless overridden by the caller.
- `EncodeAt(input string, version MicroQRVersion, ecc MicroQREccLevel)` — convenience
  wrapper for callers that know exactly which symbol version and ECC level they need.
- `Layout(grid barcode2d.ModuleGrid, config *barcode2d.Barcode2DLayoutConfig)` —
  converts a `ModuleGrid` to a `PaintScene` via `barcode-2d`'s `Layout` function,
  defaulting to a 2-module quiet zone (the Micro QR minimum).
- Full symbol configuration table for all 8 valid (version, ECC) combinations:
  M1/Detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q.
- Compile-time RS generator polynomials for GF(256)/0x11D with b=0 convention:
  counts 2, 5, 6, 8, 10, 14.
- Pre-computed format information table (all 32 15-bit format words, XOR-masked
  with 0x4445 per ISO/IEC 18004:2015 Annex E).
- Literate-programming-style comments throughout: every function, every constant,
  and every algorithm step is annotated with plain-language explanations, ASCII
  diagrams, and cross-references to the standard.
- Test suite with 95.9% statement coverage:
  - Symbol dimension checks (11×11 … 17×17)
  - Auto-version and auto-mode selection
  - Structural module tests (finder, separator, timing)
  - Determinism tests
  - ECC-level constraint tests (valid and invalid combinations)
  - Capacity boundary tests (at-limit and over-limit)
  - Format information sanity checks
  - Grid completeness checks
  - Cross-language corpus tests matching the spec
  - White-box unit tests for `bitWriter`, `maskCondition`, `selectMode`
