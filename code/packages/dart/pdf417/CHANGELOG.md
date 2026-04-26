# Changelog — coding_adventures_pdf417

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-25

### Added

**Initial implementation of the PDF417 encoder (ISO/IEC 15438:2015).**

#### Core encoding

- `encode(List<int> bytes, {Pdf417Options options})` — encode raw bytes to a
  `ModuleGrid`. Supports the full byte compaction pipeline (codeword 924 latch,
  6-bytes→5-codewords base-900 conversion, direct 1-byte remainder mapping).
- `encodeString(String s, {Pdf417Options options})` — convenience wrapper that
  UTF-8 encodes the string before calling `encode`.
- `encodeAndLayout(List<int> bytes, {Pdf417Options, Barcode2DLayoutConfig})` —
  convenience wrapper that calls `encode` then `layout` from `barcode-2d`.

#### GF(929) field arithmetic

- Lazy-initialized log/antilog tables for GF(929) (the integers mod 929).
- `_gfMul` using log/antilog tables for O(1) multiplication.
- `_gfAdd` for modular addition in GF(929).

#### Reed-Solomon ECC

- `_buildGenerator(eccLevel)` — constructs the RS generator polynomial over
  GF(929) using the **b=3 convention**: roots α^3, α^4, …, α^(k+2) where
  k = 2^(eccLevel+1).
- `_rsEncode(data, eccLevel)` — LFSR shift-register RS encoding. No
  interleaving (unlike QR Code), a single pass over all data.
- 9 ECC levels (0–8): 2, 4, 8, 16, 32, 64, 128, 256, 512 ECC codewords.

#### Symbol layout

- `_autoEccLevel` — selects minimum recommended ECC level based on data
  codeword count (thresholds: 40/160/320/863).
- `_chooseDimensions` — picks c (columns) and r (rows) for a roughly square
  symbol using the heuristic `c = ceil(sqrt(total/3))`.
- `computeLri` / `computeRri` — row indicator codeword computation for all
  three clusters, following the Python pdf417 library's verified formula.
- `_rasterize` — converts the flat codeword sequence to a `ModuleGrid` using
  cluster tables, row indicators, and fixed start/stop patterns.

#### Cluster tables

- Embedded `kClusterTables` constant with 3 × 929 = 2787 packed 32-bit entries.
  Extracted from the Python pdf417 library (MIT License), verified against
  ISO/IEC 15438:2015 Annex B.
- `kStartPattern` — 8-element bar/space width array: `[8,1,1,1,1,1,1,3]` (17 modules).
- `kStopPattern` — 9-element bar/space width array: `[7,1,1,3,1,1,1,2,1]` (18 modules).

#### Error types

- `Pdf417Error` — base class for all encoder errors.
- `InputTooLongError` — input exceeds symbol capacity (90×30 = 2700 slots).
- `InvalidDimensionsError` — `columns` out of range 1–30.
- `InvalidEccLevelError` — `eccLevel` out of range 0–8.

#### Options

- `Pdf417Options` — immutable options record:
  - `eccLevel` (int?, default: auto) — ECC level 0–8.
  - `columns` (int?, default: auto) — data columns 1–30.
  - `rowHeight` (int, default: 3) — module-rows per logical PDF417 row.

### Not yet implemented (v0.2.0)

- Text compaction mode (codeword 900 latch, UC/LC/ML/PL sub-modes).
- Numeric compaction mode (codeword 902 latch, 44-digit chunks).
- Mixed-mode auto-detection (segment runs of text/digits/binary).
- Macro PDF417 (codewords 925–928, multi-symbol sequences).

### Test coverage

64 unit and integration tests covering:
- GF(929) arithmetic properties
- RS ECC codeword counts for all 9 levels
- Byte compaction (full groups, remainders, edge cases)
- ECC auto-level selection
- Dimension chooser and custom overrides
- Row indicator LRI/RRI for all three clusters
- Start and stop pattern correctness in every row
- Row height repetition
- Symbol dimension formulae
- encodeString / encodeAndLayout convenience functions
- Error handling (all error types, all bounds violations)
- Determinism
- Cross-language corpus dimension verification
