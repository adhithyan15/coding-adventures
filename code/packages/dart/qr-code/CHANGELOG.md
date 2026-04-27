# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning 2.0.0](https://semver.org/).

---

## [0.1.0] — 2026-04-24

### Added

- Initial release of the QR Code encoder for Dart.
- Full ISO/IEC 18004:2015 compliant encoding pipeline:
  - **Mode selection**: numeric, alphanumeric, and byte (UTF-8) modes.
  - **Version selection**: automatically chooses the minimum version (1–40) that
    fits the input at the chosen ECC level.
  - **Bit stream assembly**: mode indicator, character count, encoded data,
    terminator, byte-boundary padding, 0xEC/0x11 fill padding.
  - **Reed-Solomon ECC**: GF(256) with primitive polynomial 0x11D, b=0 convention
    (roots α^0, α^1, …). Generator polynomial built on demand using `gf256` package.
  - **Block interleaving**: data codewords interleaved round-robin, then ECC
    codewords, per ISO 18004 §8.6.
  - **Grid construction**: finder patterns × 3, separators, timing strips,
    alignment patterns (all 40 versions), dark module at (4V+9, 8).
  - **Data placement**: two-column zigzag scan from bottom-right, skipping
    reserved modules and timing column 6.
  - **Mask evaluation**: all 8 mask patterns tried; lowest 4-rule penalty score
    selected.
  - **Format information**: 15-bit BCH-protected word with MSB-first bit ordering
    (lessons.md fix applied — correct bit ordering for Copy 1 row 8).
  - **Version information**: 18-bit BCH-protected block for versions 7–40.
- Public API:
  - `encode(String, EccLevel)` → `ModuleGrid`
  - `encodeAndLayout(String, EccLevel, {Barcode2DLayoutConfig?})` → `PaintScene`
  - `EccLevel` enum: `l`, `m`, `q`, `h`
  - `InputTooLongError` and `QRLayoutError` error classes
- 47 unit tests with 94.3% line coverage.
- Literate (Knuth-style) source code with inline explanations, diagrams, and
  algorithm walkthroughs throughout.
- Depends on `coding_adventures_gf256` (GF(256) arithmetic) and
  `coding_adventures_barcode_2d` (ModuleGrid type, layout pipeline).

### Implementation notes

- The format information bit ordering follows the fix documented in `lessons.md`
  (2026-04-23): Copy 1 row 8 uses MSB-first order (f14→f9 for cols 0–5), not
  LSB-first. This is the critical correctness detail that determines whether
  real QR scanners can read the symbol.
- UTF-8 encoding: byte mode uses manual UTF-8 encoding of Dart's Unicode code
  points rather than platform-specific encoding APIs, ensuring consistent output
  across all platforms.
- Kanji mode and ECI are not implemented in v0.1.0. Use byte mode for any
  non-alphanumeric text.
