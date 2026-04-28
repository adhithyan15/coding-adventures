# Changelog — coding_adventures_data_matrix

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-26

### Added

- **`encode(String data, {DataMatrixOptions options})`** — encodes a string to a
  `ModuleGrid` following ISO/IEC 16022:2006 ECC200. Auto-selects the smallest
  fitting symbol when no size is specified.

- **`layoutGrid(ModuleGrid grid, {Barcode2DLayoutConfig? config})`** — converts a
  `ModuleGrid` to a `PaintScene` using the barcode-2d layout engine with a
  1-module quiet zone (Data Matrix minimum).

- **`encodeAndLayout(String data, {...})`** — convenience wrapper that encodes
  and lays out in one step.

- **`gridToString(ModuleGrid grid)`** — debug utility rendering a grid as
  multi-line '0'/'1' string for snapshot comparison and cross-language corpus
  verification.

- **`SymbolShape` enum** — `square` (default), `rectangle`, `any` shape
  preference for auto-selection.

- **`DataMatrixOptions`** — options class accepting an optional `size` (forced
  symbol dimension) and `shape` preference.

- **`DataMatrixError`** — abstract base class for all encoder errors, implementing
  `Exception`.

- **`InputTooLongError`** — thrown when input encodes to more codewords than any
  fitting symbol can hold.

- **`InvalidSizeError`** — thrown when a caller-specified `size` does not match
  any ECC200 symbol dimension.

- **`dataMatrixVersion`** constant (`'0.1.0'`).

- **`gf256Prime`** constant (`0x12D` = 301) — the GF(256) primitive polynomial
  for Data Matrix ECC200.

- **`minSize`** / **`maxSize`** constants (`10` and `144`).

### Algorithm details

- **GF(256)/0x12D field** — builds exp/log tables inline (separate from the
  `coding_adventures_gf256` package which uses QR Code's 0x11D polynomial).

- **Reed-Solomon encoding** — LFSR shift-register method, b=1 convention
  (roots α¹…α^n), over GF(256)/0x12D.

- **ASCII codeword encoding** — digit-pair compression (two adjacent digit bytes
  → single codeword 130+pair), single-byte encoding (+1 shift), and extended-ASCII
  UPPER_SHIFT (235) support.

- **Scrambled pad codewords** — EOM byte (129) followed by position-dependent
  scrambled bytes per ISO/IEC 16022:2006 §5.2.3.

- **24 square symbol sizes** from 10×10 (3 data CW) to 144×144 (1558 data CW).

- **6 rectangular symbol sizes** from 8×18 to 16×48.

- **Multi-block Reed-Solomon interleaving** — data and ECC codewords are
  interleaved round-robin across blocks to distribute burst errors.

- **Grid initialisation** — L-shaped finder (left column + bottom row all dark),
  timing border (top row + right column alternating dark/light), and 2-module
  alignment borders for multi-region symbols. Written in the correct order so
  the L-finder bottom row always wins at intersections.

- **Utah placement algorithm** — diagonal zigzag traversal of the logical data
  matrix with four corner patterns (ISO/IEC 16022:2006 Annex F) and ISO fill
  rule for residual modules ((r+c) mod 2 == 1).

- **Logical-to-physical coordinate mapping** — maps Utah logical coordinates to
  physical symbol coordinates, accounting for the 1-module outer border and
  2-module alignment borders.

- **No masking** — Data Matrix ECC200 has no masking step; the diagonal
  traversal naturally distributes bits without the clustering that requires
  masking in QR Code.

### Test coverage

- 18 test groups with 60+ individual test cases covering:
  - Package constants
  - Error type hierarchy
  - Basic encoding output shape
  - Grid dimension consistency
  - All-boolean module values
  - Determinism
  - Monotonic symbol size growth
  - `InputTooLongError` for oversized inputs
  - Empty string encoding
  - L-finder structural invariants
  - Timing border structural invariants
  - Forced size encoding
  - `InputTooLongError` for forced size
  - `InvalidSizeError` for invalid dimensions
  - `gridToString` utility
  - `layoutGrid` and `encodeAndLayout` helpers
  - Cross-language corpus size expectations
  - `SymbolShape` option
