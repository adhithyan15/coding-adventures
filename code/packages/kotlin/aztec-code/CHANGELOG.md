# Changelog — aztec-code (Kotlin)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning 2.0](https://semver.org/).

---

## [0.1.0] — 2026-05-06

### Added

- **`AztecCode.kt`** — full Aztec Code encoder, ISO/IEC 24778:2008.
  - Compact Aztec: layers 1–4, symbols 15×15–27×27, formula `size = 11 + 4*layers`.
  - Full Aztec: layers 1–32, symbols 19×19–143×143, formula `size = 15 + 4*layers`.
  - Automatic symbol selection: tries compact 1–4 first, then full 1–32; selects
    smallest symbol that fits the data at the requested ECC level.
  - Byte-mode encoding via Binary-Shift from Upper mode (v0.1.0):
    - Emits 5-bit escape (0b11111) then 5-bit or 11-bit length prefix.
    - Each byte follows as 8 bits MSB-first.
  - GF(256)/0x12D Reed-Solomon ECC for data codewords (same polynomial as
    Data Matrix ECC200; differs from QR Code's 0x11D). Implemented inline
    with log/antilog tables for O(1) multiply.
  - GF(16)/0x13 Reed-Solomon ECC for mode message nibbles. Compact mode uses
    a (7,2) code (5 check nibbles); full mode uses a (10,4) code (6 check nibbles).
  - Bit stuffing: insert complement bit after every run of 4 identical bits,
    applied to the combined data+ECC bit stream.
  - Bullseye finder pattern: concentric dark/light rings at Chebyshev distances
    0–5 (compact) or 0–7 (full); d≤1 are both DARK (inner 3×3 core).
  - Reference grid for full symbols: alternating dark/light lines at ±16 module
    intervals from the center row and column.
  - Orientation marks: four dark corners of the mode message ring (breaks the
    rotational symmetry of the bullseye).
  - Mode message placement: 28 bits (compact) or 40 bits (full) in the ring
    immediately outside the bullseye, non-corner positions only, clockwise from
    top-left+1.
  - Data layer spiral: clockwise from innermost layer outward, 2 modules wide
    per layer, outer row/column first then inner.
  - All-zero last codeword avoidance: if the last data codeword before RS would
    be 0x00, replace with 0xFF.
  - Returns a fully immutable `ModuleGrid` (both inner row lists and outer list
    are unmodifiable).

- **`AztecOptions`** data class:
  - `minEccPercent`: ECC level (default 23, range 10–90).

- **`AztecError`** sealed class hierarchy (subtypes of `Exception`):
  - `InputTooLong` — data exceeds 32-layer full symbol capacity.

- **`VERSION`** top-level constant = `"0.1.0"`.

- **`AztecCodeTest.kt`** — JUnit 5 test suite (87 assertions):
  - VERSION constant value.
  - AztecError sealed class hierarchy: each subtype is both an `AztecError`
    and an `Exception`.
  - GF(16) arithmetic: identity, commutativity, period-15 order, generator degree.
  - GF(16) RS: output length, nibble range, determinism.
  - GF(256)/0x12D arithmetic: identity, commutativity, period-255 order.
  - GF(256) RS: output length, determinism, different-data → different-ECC.
  - Bit stuffing: no stuffing for alternating bits; 4 identical → 1 stuff;
    8 identical → 2 stuffs; precise result verification for mixed runs.
  - Binary-Shift encoding: escape prefix, length field, correct bit encoding.
  - Symbol size selection: compact for short input, full for large input,
    InputTooLong for oversized input, dataCwCount+eccCwCount=total.
  - Mode message: 28-bit compact, 40-bit full, only 0/1 bits, deterministic,
    different layers/codeword-counts produce different messages.
  - Geometry helpers: compact size = 11+4*layers, full size = 15+4*layers.
  - Smoke tests: non-null grid, square shape, correct module dimensions,
    determinism, ByteArray overload, options-default parity.
  - Grid size by input: "A" → 15×15, "Hello World" → 19×19.
  - Bullseye pattern: d=0/1 dark, d=2 light, d=3 dark, d=4 light, d=5 dark.
  - Orientation marks: all four corners of mode ring are dark.
  - Error handling: InputTooLong, empty string encodes successfully.
  - ModuleGrid immutability: outer list and inner row lists throw on mutation.
  - Cross-language corpus: "A"→15×15, "Hello World"→19×19, URL→23×23,
    digits→23×23.
  - Padding: exact size, bit preservation, zero-fill, truncation.

- **`build.gradle.kts`** — Kotlin JVM 2.1.20, Java 21, JUnit Jupiter 5.11.4.
  Gradle build directory redirected to `gradle-build/` to avoid collision with
  the `BUILD` file on case-insensitive filesystems.

- **`settings.gradle.kts`** — composite build includes `paint-instructions`
  and `barcode-2d` (leaf to root order).

- **`BUILD`** — build-tool command with file-system lock.

- **`README.md`** — installation, usage examples, API reference, encoding
  pipeline, symbol structure diagram, RS parameters, dependency list.

- **`CHANGELOG.md`** — this file.

### Implementation notes

- GF(256)/0x12D is implemented inline (not via the repo's `reed-solomon` package)
  because that package uses GF(256)/0x11D (QR Code polynomial). Aztec Code uses
  the Data Matrix polynomial 0x12D. The two are incompatible.

- GF(256) tables are built at class load time inside a Kotlin `object` initializer
  (`Gf256Tables`), which runs once when the object is first accessed.

- The `SymbolSpec` data class is `internal` to allow direct unit testing of
  `selectSymbol()` without exposing it as public API.

- `gf16Mul`, `gf16RsEncode`, `gf256Mul`, `gf256RsEncode`, `stuffBits`,
  `encodeBytesAsBits`, `selectSymbol`, `encodeModeMessage`, `symbolSize`,
  `bullseyeRadius`, `drawBullseye`, `drawReferenceGrid`,
  `drawOrientationAndModeMessage`, `placeDataBits`, and `padToBytes` are all
  `internal` to allow direct testing from the test module.

- `encode(String)` and `encode(ByteArray)` are public top-level functions.
  The `String` overload is UTF-8 encoded first.
