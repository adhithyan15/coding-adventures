# Changelog — coding-adventures-pdf417

All notable changes to this package are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.1.0] — 2026-05-06

### Added

- Initial implementation of the PDF417 stacked linear barcode encoder.
- `encode(data, *, ecc_level, columns, row_height)` — core public API.
  Returns a `ModuleGrid` from `barcode-2d`.
- `compute_lri(r, rows, cols, ecc_level)` — Left Row Indicator computation.
- `compute_rri(r, rows, cols, ecc_level)` — Right Row Indicator computation.
- `grid_to_string(grid)` — debug rendering as '0'/'1' string.
- GF(929) arithmetic via precomputed exp/log tables (α = 3).
- Reed-Solomon ECC over GF(929) with b=3 convention, levels 0–8.
- Byte compaction mode (codeword 924 latch, 6-bytes-to-5-codewords).
- Auto-ECC level selection based on data codeword count.
- Auto-dimension selection heuristic (roughly square symbol).
- All three cluster tables embedded as static constants (3 × 929 entries).
- Start pattern (17 modules) and stop pattern (18 modules) per row.
- Error types: `PDF417Error`, `InputTooLongError`, `InvalidDimensionsError`,
  `InvalidECCLevelError`.
- Full type annotations throughout.
- Literate programming style with inline explanations.

### Implementation notes

- v0.1.0 implements **byte compaction only**.  All input is treated as raw
  UTF-8 bytes regardless of content.  Text and numeric compaction (which
  yield denser codeword sequences for ASCII/digit inputs) are planned for
  v0.2.0.
- Cluster tables sourced from the TypeScript reference implementation and
  verified against the ISO/IEC 15438:2015 annex B conventions.
- GF(929) uses modular integer arithmetic (prime characteristic) — not
  binary polynomial arithmetic like GF(256).  This is the key algorithmic
  difference from QR Code, Data Matrix, and Aztec Code encoders.
