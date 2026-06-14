# Changelog — aztec-code

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This package adheres to [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-05-06

### Added

- **`encodeAztecCode :: String -> Either AztecError [[Bool]]`** — top-level
  convenience function that UTF-8 encodes a `String` and returns a boolean
  matrix (`True` = dark module) in the smallest fitting Aztec symbol.

- **`encodeWithOptions :: [Int] -> AztecOptions -> Either AztecError ModuleGrid`**
  — encodes arbitrary byte sequences with configurable ECC percentage, returning
  a `ModuleGrid` (from `barcode-2d`) for downstream rendering.

- **`encodeAndLayout :: [Int] -> AztecOptions -> Barcode2DLayoutConfig -> Either AztecError PaintScene`**
  — full pipeline from bytes to a `PaintScene` ready for the PaintVM renderer.

- **`defaultOptions :: AztecOptions`** — 23 % ECC, auto compact/full selection.

- **Compact Aztec** (layers 1–4, 15×15 → 27×27) and **Full Aztec** (layers
  1–32, 19×19 → 143×143) symbol sizes, selected automatically by
  `selectSymbol`.

- **Binary-Shift byte-mode encoding** — all input bytes are encoded as
  codeword 31 (escape to Binary-Shift from Upper mode) followed by two 4-bit
  nibbles per byte, producing 13-bit blocks.  Multi-mode optimisation is
  deferred to v0.2.0.

- **Reed-Solomon ECC over GF(256)/0x12D** (same primitive polynomial as Data
  Matrix) for data codewords.  The generator polynomial is built lazily and
  shared across calls as a CAF.

- **Reed-Solomon ECC over GF(16)/0x13** (x⁴ + x + 1) for the 28-bit (compact)
  or 40-bit (full) mode message.

- **Bit stuffing** — after every run of 4 identical bits a complement bit is
  inserted, as required by ISO/IEC 24778:2008 §7.3.

- **Bullseye finder pattern** — concentric alternating dark/light rings centred
  on the symbol, with the innermost ring (d = 0) always dark.

- **Orientation marks** — the four dark corner modules of the mode-message ring
  that encode symbol orientation for a decoder.

- **Mode message placement** — 28 or 40 stuffed bits written clockwise around
  the outermost bullseye ring.

- **Reference grid** (full symbols only) — alternating dark/light modules on
  every 16th row and column from the centre, as specified by the standard.

- **Clockwise layer spiral data placement** — data bits are written in
  alternating inward/outward passes around each layer, following the standard
  clockwise spiral.

- **58 unit tests** covering: version string, GF(16) and GF(256) RS encoding,
  bit stuffing, mode message construction, symbol size selection, grid
  dimensions, bullseye ring colours, orientation marks, determinism, error
  handling (`InputTooLong`), full-Aztec symbols, binary data, `encodeAndLayout`
  integration, ECC knobs, cross-language test vectors, and capacity ordering.

### Limitations (v0.1.0)

- Byte-mode only — multi-mode (Digit / Upper / Lower / Mixed / Punct)
  compaction is v0.2.0.
- GF(32) codewords (5-bit mode) are not yet implemented; all data uses 8-bit
  GF(256) codewords.
- Default ECC is 23 %; configurable via `azMinEccPercent` but ECC boost
  (minimum 23 % per spec Annex A) is not yet validated for edge cases.
