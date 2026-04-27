# Changelog — kotlin/pdf417

All notable changes to this package are documented here.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-26

### Added

- **`PDF417.kt`** — ISO/IEC 15438:2015-compliant PDF417 stacked linear barcode encoder.
  - `encode(bytes: ByteArray, options: PDF417Options): ModuleGrid` — raw-byte encoder.
  - `encodeString(text: String, options: PDF417Options): ModuleGrid` — UTF-8 convenience wrapper.
  - `PDF417Options` data class with optional `eccLevel`, `columns`, and `rowHeight` fields.
  - `PDF417Error` sealed class hierarchy: `InputTooLong`, `InvalidDimensions`, `InvalidECCLevel`.
  - Public constants: `VERSION`, `GF929_PRIME`, `GF929_ALPHA`, `GF929_ORDER`, `LATCH_BYTE`,
    `PADDING_CW`, `MIN_ROWS`, `MAX_ROWS`, `MIN_COLS`, `MAX_COLS`.
  - `autoEccLevel(dataCount: Int): Int` — auto-select ECC level by data size.
  - `computeLRI(r, rows, cols, eccLevel): Int` — Left Row Indicator computation.
  - `computeRRI(r, rows, cols, eccLevel): Int` — Right Row Indicator computation.
  - `Internal` object exposing GF tables, byte compaction, RS encode, and generator
    build functions for testing.

- **`ClusterTables.kt`** (pre-existing) — three cluster tables `K0`, `K1`, `K2` (929 packed Int
  entries each) and `CLUSTER_TABLES` array accessor.

- **`PDF417Test.kt`** — comprehensive JUnit 5 test suite with:
  - VERSION constant check
  - Error hierarchy (sealed class, RuntimeException base)
  - Basic encoding: hello string, UTF-8 convenience wrapper, empty input, single byte
  - Determinism: same input → identical ModuleGrid
  - Symbol grows with data
  - Grid shape invariants (rows, cols, boolean modules, SQUARE shape)
  - Module width formula (`69 + 17 × cols`) verification
  - ECC levels 0–8 all produce valid symbols
  - Invalid ECC level (< 0 or > 8) throws `InvalidECCLevel`
  - Higher ECC → same or larger symbol area
  - Explicit column counts with width formula verification
  - Invalid columns (0 or 31) throw `InvalidDimensions`
  - All 9 public constants verified
  - GF(929) arithmetic: commutativity, identity, zero absorption, Fermat's theorem, inverse
  - Byte compaction: latch prefix, 6→5 compression, empty/remainder byte cases, range check
  - RS ECC: correct output count per level, range check, generator degree and monic property
  - `autoEccLevel` threshold boundaries
  - Row indicator cluster routing (LRI/RRI for rows 0–3)
  - Cluster table integrity: 929 entries per table, nibble sum = 17 per entry
  - `rowHeight` option: pixel rows scale correctly, width unchanged
  - 600-byte stress test

- **`build.gradle.kts`** — Kotlin JVM 2.1.20, JVM target 21, JUnit 5.11.4, composite build
  deps on `paint-instructions` and `barcode-2d`.

- **`settings.gradle.kts`** — composite build includes for `paint-instructions` and `barcode-2d`.

- **`BUILD`** — single-line build command with `kotlin-pdf417` build lock.

- **`required_capabilities.json`** — pure computation, no I/O capabilities required.

- **`README.md`** — package documentation with API reference, algorithm description,
  ECC level guide, symbol dimension formulae, and GF(929) primer.

### v0.1.0 scope note

This release implements **byte compaction only** (codeword 924 latch). Text
compaction (codewords 900/901) and numeric compaction (codeword 902) are
planned for v0.2.0.

[0.1.0]: https://github.com/adhithyan15/coding-adventures/tree/main/code/packages/kotlin/pdf417
