# Changelog — PDF417 (Swift)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-26

### Added

- `encode(_:options:)` — main public API. Accepts a `String`, encodes it as
  UTF-8 bytes through byte compaction, and returns a `ModuleGrid` ready for
  rendering via `Barcode2D.layout()`.

- `encode(bytes:options:)` — overload for raw `[UInt8]` input. Use this for
  binary data that is not valid UTF-8.

- `encodeAndLayout(_:options:config:)` — convenience wrapper that runs
  `encode()` then passes the resulting `ModuleGrid` through
  `Barcode2D.layout()`. Defaults `quietZoneModules` to 2 (PDF417 minimum,
  half of QR Code's 4-module quiet zone).

- `PDF417Options` struct with optional `eccLevel` (0–8), optional `columns`
  (1–30), and `rowHeight` (default 3).

- `PDF417Error` enum with cases:
  - `.inputTooLong(String)` — data does not fit in any valid symbol.
  - `.invalidDimensions(String)` — `columns` outside 1–30.
  - `.invalidECCLevel(String)` — `eccLevel` outside 0–8.
  - `.layoutError(String)` — error from `Barcode2D.layout()`.

- Full encoding pipeline:
  - `pdf417ByteCompact(bytes:)` — byte compaction with codeword-924 latch,
    6-byte → 5-codeword base-900 packing, and 1-codeword-per-trailing-byte.
  - `pdf417BuildGenerator(eccLevel:)` — RS generator polynomial in GF(929),
    b=3 convention.
  - `pdf417RSEncode(data:eccLevel:)` — LFSR polynomial division producing
    `2^(eccLevel+1)` ECC codewords.
  - `pdf417AutoEccLevel(dataCount:)` — recommended ECC level given codeword
    count; thresholds at 40 / 160 / 320 / 863.
  - `pdf417ChooseDimensions(total:)` — square-ish dimension heuristic
    `c = ⌈√(total / 3)⌉`, `r = ⌈total / c⌉`, clamped to spec ranges.
  - `pdf417ComputeLRI(r:rows:cols:eccLevel:)` and
    `pdf417ComputeRRI(r:rows:cols:eccLevel:)` — Left/Right Row Indicator
    codewords per ISO/IEC 15438 (cluster-aware formula).
  - `pdf417ExpandPattern(packed:into:)` — UInt32 packed bar/space widths
    → 17 boolean modules.
  - `pdf417ExpandWidths(widths:into:)` — `[Int]` bar/space width array
    → boolean modules (used for start/stop patterns).
  - `pdf417Rasterize(...)` — assemble the final `ModuleGrid`.

- `PDF417_CLUSTER_TABLES` — three cluster tables of 929 entries each,
  bit-identical to the TypeScript reference and the Java/Python ports.

- `PDF417_START_PATTERN` and `PDF417_STOP_PATTERN` — fixed start/stop
  bar/space width arrays. Decode to canonical 17-/18-module bit strings.

- GF(929) arithmetic primitives:
  - `PDF417_GF_EXP` and `PDF417_GF_LOG` log/antilog tables built at
    initialization time.
  - `pdf417GFAdd(_:_:)` and `pdf417GFMul(_:_:)` constant-time helpers using
    log/antilog lookup.

- Comprehensive test suite (61 tests across 12 suites):
  - Cluster table structure (3 clusters × 929 entries).
  - Every cluster entry expands to exactly 17 modules.
  - Start/stop patterns match canonical bit strings.
  - GF(929) table values: `α^0=1`, `α^1=3`, `α^2=9`, `α^3=27`, `α^928=1`
    (Fermat).
  - GF(929) arithmetic: `gfAdd` boundary cases, `gfMul` multiplicative
    inverses (3 × 310 ≡ 1).
  - Byte compaction: empty input → `[924]`, single byte, 6-byte group,
    7-byte group + 1, 12-byte two groups.
  - Row indicators verified against the cluster-aware formula.
  - Dimension heuristic invariants (≥ 3 rows, ≥ 1 col, capacity ≥ total).
  - Auto-ECC thresholds at 40, 160, 320, 863 codewords.
  - Module-width formula `cols = 69 + 17 × c` for c in {1, 3, 5, 10, 30}.
  - Start/stop pattern appears in every row of every encoded symbol.
  - All-256 byte values, repeated 0xFF, empty input, "HELLO WORLD".
  - Determinism (identical input → identical grid).
  - Higher ECC level produces ≥ symbol.
  - rowHeight scales pixel-grid height linearly.
  - Error throws for invalid ECC level (9, -1), invalid columns (0, 31),
    and oversized input forced into 1 column.
  - `encodeAndLayout` returns a `PaintScene` with positive dimensions and
    > 1 instructions.

### Notes

This Swift port matches the TypeScript reference implementation in
`code/packages/typescript/pdf417/`. Cluster tables and pattern arrays are
bit-identical; row-indicator and ECC algorithms produce identical output
for the same input.

The package depends on:
- `Barcode2D` for `ModuleGrid` and the `layout()` function.
- `PaintInstructions` for the `PaintScene` returned by `encodeAndLayout`.

No filesystem, network, process, or environment access is required (see
`required_capabilities.json`).
