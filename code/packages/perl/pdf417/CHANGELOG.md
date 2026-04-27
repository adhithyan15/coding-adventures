# Changelog — CodingAdventures::PDF417

All notable changes to this Perl package are documented here.

## [0.1.0] — 2026-04-25

### Added

- Initial release of the PDF417 encoder for Perl.
- Implements **byte compaction only** (codeword 924 latch, 6-bytes → 5-codeword
  base-900 groups, 1-byte remainder direct mapping).
- **GF(929) arithmetic** — prime field over the integers mod 929; log/antilog
  tables for O(1) multiplication; α = 3 as the primitive root (ISO/IEC
  15438:2015 Annex A.4).
- **Reed-Solomon ECC over GF(929)** — b=3 convention (roots at α^3…α^{k+2});
  LFSR shift-register encoder; nine ECC levels 0–8 (2–512 ECC codewords).
- **Auto ECC level selection** based on data codeword count:
  ≤40 → level 2, ≤160 → level 3, ≤320 → level 4, ≤863 → level 5, else → level 6.
- **Automatic dimension selection** — `choose_dimensions(total)` picks c =
  ceil(sqrt(total/3)) data columns and r = ceil(total/c) rows, both clamped to
  the ISO-specified ranges (1–30 columns, 3–90 rows).
- **Three codeword cluster tables** (clusters 0/3/6 → indices 0/1/2) — 929 × 3
  = 2787 packed 32-bit patterns extracted from the Python pdf417 library (MIT
  licence).  Each pattern encodes 4 bars + 4 spaces summing to 17 modules.
- **Row indicator codewords** (LRI + RRI per row) encoding R_info, C_info, and
  L_info across the three clusters as specified in ISO Table 2.
- **Start/stop patterns** — 17-module start (11111111010101000) and 18-module
  stop (111111101000101001) patterns added to every row.
- **ModuleGrid output** — returns a `CodingAdventures::Barcode2D` ModuleGrid
  hashref (rows × cols boolean grid).
- **Public API**:
  - `encode(\@bytes, \%opts)` — encode a byte array.
  - `encode_str($string, \%opts)` — encode a Perl string (UTF-8 bytes).
  - `byte_compact(\@bytes)` — lower-level byte compaction.
  - `auto_ecc_level($n)`, `choose_dimensions($n)` — dimension helpers.
  - `compute_lri($r, $R, $C, $L)`, `compute_rri($r, $R, $C, $L)` — row indicators.
  - `rs_encode(\@data, $level)` — Reed-Solomon encoding.
  - `gf929_mul($a, $b)`, `gf929_add($a, $b)` — GF(929) arithmetic.
- **Options**: `ecc_level` (0–8), `columns` (1–30), `row_height` (1–10).
- Test suite (`t/pdf417.t`) with 25 subtests covering GF arithmetic, RS encoding,
  byte compaction, row indicators, full encode roundtrips, error handling,
  cluster table sanity, and module structure.

### Limitations (planned for v0.2.0)

- Text compaction (sub-modes UC/LC/ML/PL) — not yet implemented.
- Numeric compaction (44-digit base-900 chunks) — not yet implemented.
- Mixed-mode auto-detection — not yet implemented; all input uses byte mode.
- Macro PDF417 (codewords 925–928) — not implemented.
