# Changelog — pdf417 (Rust)

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-24

### Added

- Initial release of the PDF417 stacked linear barcode encoder.
- **Byte compaction** (codeword 924 latch): the only compaction mode in v0.1.0.
  - 6-byte groups are encoded as 5 base-900 codewords (48-bit big-endian integer).
  - Remaining 1–5 bytes are encoded as direct (value-passthrough) codewords.
- **GF(929) arithmetic** implemented inline using log/antilog lookup tables.
  - Generator element α = 3 (primitive root mod 929).
  - `OnceLock`-based lazy initialization; safe to call from multiple threads.
- **Reed-Solomon ECC** with the b=3 convention (roots α³ … α^{k+2}).
  - Single-block encoding (no interleaving, simpler than QR Code).
  - Levels 0–8 supported, producing 2–512 ECC codewords.
  - Auto-selection heuristic: level 2 for ≤40 data codewords, up to level 6.
- **Dimension selection**: `c = ⌈√(total/3)⌉`, `r = ⌈total/c⌉`, clamped to 3–90 rows × 1–30 cols.
- **Row indicators** (LRI + RRI): encode R_info, C_info, L_info across three
  cluster cycles per the Python `pdf417` library (verified scannable reference).
- **Cluster table lookup**: 929-entry × 3-cluster tables pre-computed from the
  Python `pdf417` library (MIT License); stored as packed `u32` width tuples.
- **Start pattern** `[8,1,1,1,1,1,1,3]` → `11111111010101000` (17 modules).
- **Stop pattern** `[7,1,1,3,1,1,1,2,1]` → `111111101000101001` (18 modules).
- **`ModuleGrid`** output via `barcode-2d` crate (boolean 2D grid, `true` = dark).
- **`encode()`** — produces a `ModuleGrid` from raw bytes.
- **`encode_and_layout()`** — convenience wrapper that runs the layout pipeline
  to produce a `PaintScene` ready for the PaintVM.
- **`PDF417Options`** struct: optional `ecc_level`, `columns`, `row_height`.
- **`PDF417Error`** enum: `InputTooLong`, `InvalidDimensions`, `InvalidECCLevel`.
- 33 unit tests covering: GF arithmetic, byte compaction, row indicators,
  start/stop patterns, module width formula, row height scaling, ECC level
  bounds, integration (hello world, all-256-bytes, empty input), error cases,
  determinism, and cross-row structural correctness.

### Notes on spec divergence

The spec (`code/specs/pdf417.md`) states that Cluster 0 RRI encodes L_info.
The Python `pdf417` library (the verified-scannable reference) gives Cluster 0
RRI = C_info instead. This implementation follows the Python library. The spec
has been updated to match.
