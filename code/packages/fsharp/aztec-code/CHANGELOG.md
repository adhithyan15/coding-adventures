# Changelog — CodingAdventures.AztecCode (F#)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-26

### Added

- Initial release of the F# Aztec Code encoder.
- ISO/IEC 24778:2008 compliant byte-mode encoder supporting both compact
  (1–4 layer) and full (1–32 layer) symbol variants.
- Reed-Solomon error correction over GF(256) with primitive polynomial
  `0x12D` (the same polynomial used by Data Matrix ECC200).
- GF(16) Reed-Solomon for the mode message word (primitive polynomial
  `0x13` = `x^4 + x + 1`).
- Default minimum 23% error correction with caller-tunable
  `MinEccPercent` (range 10–90).
- Auto-selection of the smallest compact / full symbol that fits the
  payload at the requested ECC level.
- Bit stuffing (insert complement after 4 consecutive identical bits)
  applied to the data + ECC stream.
- Bullseye finder pattern, orientation marks, and reference grid (full
  symbols only) drawn before the clockwise data spiral.
- Output is a `ModuleGrid` from `CodingAdventures.Barcode2D` — compatible
  with `Barcode2D.layout` for pixel-level rendering.
- Comprehensive unit tests covering: capacity tables, compact/full
  selection, bullseye geometry, orientation mark placement,
  Reed-Solomon round trips, bit stuffing edge cases, ECC option
  validation, byte-array encoding, and determinism.

### Limitations (deferred to v0.2.0)

- Single-mode (byte / Binary-Shift) encoding only — no per-character
  Digit / Upper / Lower / Mixed / Punct optimisation.
- 8-bit codewords only — GF(16) and GF(32) data codewords (used by the
  smallest compact symbols in the standard) are not implemented.
- No `forceCompact` option — the encoder always tries compact first then
  falls back to full.
