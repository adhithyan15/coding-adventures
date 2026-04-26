# Changelog — pdf417 (Java)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.1.0] — 2026-04-26

### Added

- `PDF417.java` — complete PDF417 encoder implementing ISO/IEC 15438:2015:
  - **Byte compaction** (v0.1.0 mode): codeword 924 latch + 6-bytes-to-5-codewords
    base-900 encoding + direct 1-byte-per-codeword remainder encoding.
  - **GF(929) field arithmetic**: log/antilog tables built at class load time
    from α = 3 (primitive root mod 929). O(1) multiplication via table lookups.
  - **Reed-Solomon ECC**: b=3 convention generator polynomial
    g(x) = (x−α³)(x−α⁴)···(x−α^{k+2}). Single RS block, no interleaving.
    Levels 0–8 (k = 2^(L+1) ECC codewords, 2–512).
  - **Auto ECC level**: selects minimum recommended level based on data count
    (≤40 → L2, ≤160 → L3, ≤320 → L4, ≤863 → L5, else → L6).
  - **Dimension selection**: heuristic c = ceil(sqrt(total/3)), clamped 1–30;
    then r = ceil(total/c), clamped 3–90.
  - **Row indicator computation**: LRI and RRI per row encode R_info, C_info,
    L_info using cluster-dependent formulas. Matches the Python pdf417 library
    convention (verified scannable symbols).
  - **Rasterization**: start pattern (17 modules) + LRI + data columns + RRI
    + stop pattern (18 modules) per logical row, repeated `rowHeight` times.
    Uses a mutable `boolean[][]` buffer during construction and converts to
    the immutable `ModuleGrid` at the end for O(n) overall complexity.
  - `PDF417Options`: `eccLevel`, `columns`, `rowHeight` fields.
  - `encode(byte[])`, `encode(byte[], PDF417Options)`,
    `encode(String)`, `encode(String, PDF417Options)` overloads.
  - `PDF417Exception`, `InputTooLongException`, `InvalidDimensionsException`,
    `InvalidECCLevelException` error hierarchy.

- `ClusterTables.java` — three cluster tables (0, 1, 2), each with 929 entries.
  Each entry is a packed `int` with 8 bar/space widths in 4-bit nibbles
  (bits 31..28 = b1, 27..24 = s1, ..., 3..0 = s4). Sum of widths = 17 for
  every entry. Tables extracted from the Python pdf417 library (MIT License).

- `PDF417Test.java` — 53 JUnit 5 tests covering:
  - GF(929) arithmetic (add, mul, log/antilog round-trip, Fermat's theorem,
    primitive root coverage, commutativity, inverses)
  - RS ECC (generator degree, output length, determinism, differentiation)
  - Byte compaction (single byte, high byte, 6→5 groups, 7 bytes, 12 bytes,
    empty input, reversibility)
  - Auto ECC level selection (all threshold boundaries)
  - Dimension selection (covers total, bounds respected)
  - Row indicators (spec test vector for 10×3 ECC2 symbol, range checks)
  - Integration (ModuleGrid returned, width formula, row height multiplier,
    start/stop pattern in every row, hello world, digits, all 256 bytes,
    determinism, all ECC levels, all column counts)
  - Error handling (invalid ECC level, invalid columns)
  - Cluster table integrity (size, pattern width sums = 17)

- `build.gradle.kts` — Gradle 8 build with `java-library` plugin,
  composite builds for `paint-instructions` and `barcode-2d`,
  Java 21 source/target compatibility, JUnit 5.11.4.
  `layout.buildDirectory = file("gradle-build")` avoids clash with `BUILD` file
  on case-insensitive macOS/Windows filesystems (see lessons.md).

- `settings.gradle.kts` — `rootProject.name = "pdf417"` with `includeBuild`
  directives for `paint-instructions` and `barcode-2d`.

- `BUILD` — mono-repo build script with `.build-locks/java-pdf417.lock`
  for serialized JVM builds.

- `README.md` — package overview, usage examples, key concepts (GF(929),
  cluster tables, row indicators), ECC level table, package structure.

### Design decisions

- **Byte compaction only**: v0.1.0 uses only byte mode (codeword 924).
  Text and numeric compaction are deferred to v0.2.0. The output is valid
  and scannable for any input; it is just not maximally dense for pure
  ASCII or digit inputs.
- **Mutable buffer in rasterize()**: `boolean[][]` is used during rasterization
  for O(1) pixel writes. Converted to the immutable `ModuleGrid` once at the
  end. Using `Barcode2D.setModule()` per pixel would be O(pixels²) due to
  immutability.
- **No gf929 sub-package**: GF(929) arithmetic is implemented inline in
  `PDF417.java` rather than extracted to a separate sibling package. This
  keeps the Java package self-contained and avoids a new multi-package Gradle
  composite-build chain. A future refactor can extract gf929 if other Java
  packages need it (e.g., MicroPDF417).
- **RRI formula**: uses the Python pdf417 library's RRI convention rather than
  the spec text's column assignment. The Python library produces verified
  scannable symbols confirmed by ZXing and standard barcode readers.
