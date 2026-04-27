# Changelog — coding_adventures_micro_qr

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of the Micro QR Code encoder (ISO/IEC 18004:2015 Annex E).
- `encode(input, version:, ecc:)` — encode a string to a `ModuleGrid`; auto-selects
  the smallest symbol (M1..M4) and encoding mode (numeric / alphanumeric / byte).
- `encode_at(input, version, ecc)` — positional-argument convenience alias for `encode`.
- `layout(grid, config)` — converts a `ModuleGrid` to a `PaintScene` via `barcode_2d`.
  Defaults to `quiet_zone_modules: 2` (Micro QR minimum).
- `encode_and_layout(input, version:, ecc:, config:)` — convenience: encode + layout
  in one call.
- All 8 symbol configurations from the standard:
  M1/Detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q.
- Reed-Solomon ECC over GF(256)/0x11D with b=0 convention (generator polynomials
  for n ∈ {2, 5, 6, 8, 10, 14}).
- All 32 pre-computed format information words (XOR mask 0x4445).
- Numeric, alphanumeric (45-char QR set), and byte encoding modes.
- M1 half-codeword special case (20-bit data capacity).
- 4 mask patterns with penalty scoring (rules 1–4 identical to regular QR).
- `MicroQRVersion` module with constants M1, M2, M3, M4.
- `MicroQREccLevel` module with constants Detection, L, M, Q.
- Error classes: `InputTooLong`, `UnsupportedMode`, `ECCNotAvailable`,
  `InvalidCharacter`.
- Comprehensive test suite with 80+ test cases covering dimensions, auto-selection,
  structural modules, determinism, ECC constraints, capacity boundaries, format info,
  cross-language corpus, and internal helpers.
- `simplecov` integration targeting ≥90% line coverage.
