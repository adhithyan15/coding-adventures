# Changelog — @coding-adventures/pdf417

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-24

### Added

- Initial release of the PDF417 stacked linear barcode encoder.
- **Byte compaction** (codeword 924 latch): the only compaction mode in v0.1.0.
  - 6-byte groups are encoded as 5 base-900 codewords (BigInt for 48-bit precision).
  - Remaining 1–5 bytes are encoded as direct (value-passthrough) codewords.
- **GF(929) arithmetic** implemented inline using log/antilog lookup tables.
  - Generator element α = 3 (primitive root mod 929).
  - Tables built at module load time (IIFE, ~0.1 ms, ~3.7 KB).
- **Reed-Solomon ECC** with the b=3 convention (roots α³ … α^{k+2}).
  - Single-block encoding (no interleaving, simpler than QR Code).
  - Levels 0–8 supported, producing 2–512 ECC codewords.
  - Auto-selection heuristic: level 2 for ≤40 data codewords, up to level 6.
- **Dimension selection**: `c = ⌈√(total/3)⌉`, `r = ⌈total/c⌉`, clamped
  to 3–90 rows × 1–30 columns.
- **Row indicators** (LRI + RRI): encode R_info, C_info, L_info across three
  cluster cycles per the Python `pdf417` library (verified scannable reference).
- **Cluster table lookup**: 929-entry × 3-cluster tables extracted from the
  Python `pdf417` library (MIT License); stored as packed u32 width tuples.
- **Start pattern** `[8,1,1,1,1,1,1,3]` → `11111111010101000` (17 modules).
- **Stop pattern** `[7,1,1,3,1,1,1,2,1]` → `111111101000101001` (18 modules).
- **`ModuleGrid`** output via `@coding-adventures/barcode-2d`.
- **`encode()`** — produces a `ModuleGrid` from raw bytes or a `number[]`.
- **`encodeAndLayout()`** — convenience wrapper that runs the layout pipeline
  to produce a `PaintScene` ready for a render backend.
- **`PDF417Options`** interface: optional `eccLevel`, `columns`, `rowHeight`.
- **`PDF417Error`** base class with subclasses `InputTooLongError`,
  `InvalidDimensionsError`, `InvalidECCLevelError`.
- 61 unit tests covering: GF arithmetic, byte compaction, row indicators,
  start/stop patterns, module width formula, row height scaling, ECC level
  bounds, integration (hello world, all-256-bytes, empty input, number[] input),
  error cases, determinism, row repetition, and encodeAndLayout.
- Test coverage: 97.65% statements, 95.12% branches, 100% functions.

### Notes on spec divergence

The spec (`code/specs/pdf417.md`) states that Cluster 0 RRI encodes L_info.
The Python `pdf417` library (the verified-scannable reference) gives Cluster 0
RRI = C_info instead. This implementation follows the Python library. The spec
has been updated to match.
