# Changelog — CodingAdventures.PDF417.FSharp

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-25

### Added

- Initial implementation of the PDF417 stacked linear barcode encoder
  (ISO/IEC 15438:2015).
- **GF(929) arithmetic** — log/antilog lookup tables for O(1) multiplication
  over the prime field GF(929). Generator α = 3 (primitive root mod 929,
  per ISO/IEC 15438:2015 Annex A.4).
- **Reed-Solomon ECC** — b=3 convention (roots α^3 through α^{k+2}), generator
  polynomial built by multiplying linear factors. Single RS encoder, no
  interleaving (simpler than QR Code).
- **Byte compaction** — codeword 924 latch; 6 bytes → 5 codewords via
  48-bit base-900 arithmetic (int64); remaining 1–5 bytes encoded directly.
- **Auto ECC level selection** — levels 2–6 chosen based on data codeword count
  per the spec thresholds (≤40 → L2, ≤160 → L3, ≤320 → L4, ≤863 → L5,
  else L6).
- **Dimension selection** — heuristic `c = ceil(sqrt(total/3))` clamped 1–30;
  `r = ceil(total/c)` clamped 3–90. Optional `Columns` override.
- **Row indicator codewords** — LRI and RRI per row encode R_info, C_info,
  L_info across the three cluster types (spec Table 2).
- **Cluster tables** — three 929-entry tables (CLUSTER0, CLUSTER1, CLUSTER2)
  embedded as compile-time `uint32[]` constants, extracted from the Python
  pdf417 library (MIT License), verified against ISO/IEC 15438:2015 Annex B.
- **Start/stop patterns** — 17-module start `[8;1;1;1;1;1;1;3]` and 18-module
  stop `[7;1;1;3;1;1;1;2;1]` patterns, identical for every row.
- **Rasterization** — mutable in-place grid construction for performance,
  wrapped in an immutable `ModuleGrid` record from `barcode-2d`.
- **Public API** — `encode`, `encodeString`, `autoEccLevel`, `computeLRI`,
  `computeRRI`, `defaultOptions`.
- **Internal module** — `Internal.*` functions exported for unit testing.
- **Comprehensive tests** — 35+ tests covering GF arithmetic, RS ECC, byte
  compaction, row indicators, error handling, grid dimensions, and start/stop
  patterns. Target ≥ 90% line coverage.

### Implementation notes

- v0.1.0 implements **byte compaction only**. Text and numeric compaction are
  planned for v0.2.0.
- The RRI formula follows the Python pdf417 reference library (which produces
  verified scannable symbols) rather than literal ISO spec Table 2 text. The
  net effect is equivalent — LRI and RRI together encode all three symbol
  metadata quantities across every three consecutive rows.
- No Macro PDF417 (codewords 925–928) in this version.
