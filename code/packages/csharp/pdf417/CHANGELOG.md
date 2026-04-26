# Changelog

All notable changes to `CodingAdventures.PDF417` are documented here.

This project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-04-25

### Added

- **PDF417Encoder.Encode(byte[], PDF417Options?)** — Full PDF417 encoding
  pipeline from raw bytes to a `ModuleGrid` (ISO/IEC 15438:2015 compliant).

- **PDF417Encoder.Encode(string, PDF417Options?)** — Convenience overload that
  UTF-8 encodes the string and delegates to the byte[] overload.

- **Byte compaction** (v0.1.0 scope) — All inputs encoded via byte compaction
  mode (codeword 924 latch). Full 6-byte groups are converted to 5 base-900
  codewords; remaining bytes are mapped one-to-one.

- **GF(929) field arithmetic** — Precomputed log/antilog tables for O(1)
  multiplication over the prime field GF(929). Implemented inline in
  `PDF417Encoder` (no external gf929 package dependency).

- **Reed-Solomon ECC** — Standard shift-register encoder over GF(929) using
  the b=3 convention (roots α^3, α^4, ..., α^{k+2} where α=3). Supports ECC
  levels 0–8 (2–512 ECC codewords). No interleaving (unlike QR Code).

- **Auto ECC level selection** — Chooses ECC level based on data length:
  ≤40 codewords → level 2, ≤160 → level 3, ≤320 → level 4, ≤863 → level 5,
  otherwise → level 6.

- **Auto dimension selection** — Heuristic: `cols = ceil(sqrt(total/3))`,
  clamped to 1–30; `rows = ceil(total/cols)`, clamped to 3–90.

- **Row indicators** — LRI and RRI computed for every row, encoding R_info,
  C_info, and L_info across the three cluster types.

- **Cluster tables** — All three cluster tables (929 entries each) embedded as
  static `uint[]` arrays. Tables extracted from the Python pdf417 library (MIT
  License) and cross-verified for width-sum invariant (each pattern = 17 modules).

- **Start/stop patterns** — Start: `11111111010101000` (17 modules, widths
  [8,1,1,1,1,1,1,3]). Stop: `111111101000101001` (18 modules, widths
  [7,1,1,3,1,1,1,2,1]). Correct per ISO 15438.

- **PDF417Options** record — `EccLevel` (0–8), `Columns` (1–30),
  `RowHeight` (default 3).

- **Error types** — `PDF417Exception` (base), `InputTooLongException`,
  `InvalidDimensionsException`, `InvalidECCLevelException`.

- **xUnit test suite** — 50+ unit and integration tests covering:
  - GF(929) arithmetic (add, mul, exp/log tables, Fermat's theorem, inverse)
  - RS generator polynomial (known coefficients for level 0)
  - RS encoder (codeword count, range, determinism)
  - Byte compaction (latch, 6→5, remainder, round-trip)
  - Row indicators (spec example: R=10, C=3, L=2)
  - Dimension selection and auto ECC level
  - Full encode pipeline (grid dimensions, start/stop pattern, row height)
  - Cluster table invariants (929 entries, all widths sum to 17)
  - Error handling (invalid ECC level, columns, etc.)

### Implementation notes

- GF(929) arithmetic uses precomputed exp/log tables (static constructor), not
  runtime generation per encode call. Tables take ~7 KB and initialize in <1 ms.

- The 6-byte→5-codeword conversion uses `ulong` arithmetic (256^6 < 2^48 < 2^64,
  so no overflow). BigInteger is not needed for v0.1.0 (byte compaction only).

- Row indicator formula follows the TypeScript reference implementation (verified
  to produce scannable symbols), specifically using cluster indices 0/1/2 (not
  the spec's 0/3/6 naming which refers to the same clusters by row % 3 × 3).

- Cluster tables come from the Python pdf417 library (MIT License). All 2787
  entries were verified for the width-sum-equals-17 invariant in the test suite.

### v0.2.0 planned

- Text compaction (UC/LC/ML/PL sub-modes) for ASCII content
- Numeric compaction (44-digit chunks in base 900) for digit sequences
- Mixed-mode auto-detection of text/byte/numeric segments
