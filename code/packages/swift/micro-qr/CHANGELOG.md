# Changelog — MicroQR (Swift)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-24

### Added

- `MicroQRVersion` enum with cases `.M1`, `.M2`, `.M3`, `.M4` and a `size`
  computed property returning the symbol side length (11, 13, 15, or 17).
  Conforms to `CaseIterable`, `Comparable`, and `Sendable`.

- `MicroQREccLevel` enum with cases `.detection`, `.L`, `.M`, `.Q`.
  M1 supports `.detection` only. M2/M3 support `.L` and `.M`. M4 supports
  `.L`, `.M`, and `.Q`. Level H is not available in any Micro QR symbol.

- `MicroQRError` enum with five error cases:
  - `.inputTooLong(String)` — input exceeds maximum capacity.
  - `.eccNotAvailable(String)` — invalid (version, ECC) combination.
  - `.unsupportedMode(String)` — no encoding mode can handle the input.
  - `.invalidCharacter(String)` — character outside the mode's character set.
  - `.layoutError(String)` — error from the barcode-2d rendering layer.

- `encode(_:version:ecc:)` — main public API. Auto-selects the smallest
  symbol (M1..M4) and mode (numeric > alphanumeric > byte) that fits the
  input. Optional `version` and `ecc` parameters override the auto-selection.
  Returns a `ModuleGrid` ready for rendering.

- `encodeAt(_:version:ecc:)` — convenience wrapper over `encode()` when
  both version and ECC level are known at the call site.

- `layoutGrid(_:config:)` — converts a `ModuleGrid` to a `PaintScene` via
  the `Barcode2D.layout()` function. Defaults to `quietZoneModules = 2`
  (Micro QR minimum, half of regular QR's 4-module quiet zone).

- Full encoding pipeline:
  - `SymbolConfig` struct holding all compile-time constants for each of the
    8 valid (version, ECC) combinations.
  - `SYMBOL_CONFIGS` static table: M1/Detection, M2/L, M2/M, M3/L, M3/M,
    M4/L, M4/M, M4/Q.
  - `FORMAT_TABLE` — 32 pre-computed 15-bit format words (8 symbol indicators
    × 4 mask patterns), XOR-masked with 0x4445 per the Micro QR standard.
  - `generator(for:)` — pre-computed monic RS generator polynomials for
    ECC counts {2, 5, 6, 8, 10, 14} using GF(256)/0x11D with b=0 convention.
  - `BitWriter` class — accumulates bits MSB-first and flushes to bytes.
  - Numeric encoding: groups of 3 → 10 bits, pairs → 7 bits, singles → 4 bits.
  - Alphanumeric encoding: pairs → 11 bits (first×45+second), singles → 6 bits.
    Uses the standard 45-character set.
  - Byte encoding: raw UTF-8 bytes, one byte per 8-bit codeword.
  - `rsEncode(data:generator:)` — LFSR-based Reed-Solomon encoder over
    GF(256)/0x11D with b=0 convention. Identical to the regular QR RS encoder.
  - `buildDataCodewords(input:cfg:mode:)` — assembles mode indicator,
    character count, encoded data, terminator, byte-align padding, and
    alternating 0xEC/0x11 pad codewords. Special-cases M1's 20-bit capacity.
  - `selectConfig(input:version:ecc:)` — symbol auto-selection logic.
  - `WorkGrid` class — mutable grid tracking both module values and reserved
    (structural) positions.
  - `placeFinder(_:)` — 7×7 finder pattern at top-left (rows 0–6, cols 0–6).
  - `placeSeparator(_:)` — L-shaped separator (row 7 cols 0–7, col 7 rows 0–7).
  - `placeTiming(_:)` — timing extension along row 0 and col 0 (positions 8+).
  - `reserveFormatInfo(_:)` / `writeFormatInfo(_:fmt:)` — 15-bit format info
    at row 8 (cols 1–8) and col 8 (rows 1–7); single copy, XOR 0x4445.
  - `placeBits(g:bits:)` — two-column zigzag data placement from bottom-right.
  - `maskCondition(_:row:col:)` — 4 Micro QR mask patterns.
  - `applyMask(modules:reserved:size:maskIdx:)` — XOR flip of unreserved modules.
  - `computePenalty(modules:size:)` — 4-rule penalty scoring (same as regular QR).

- Package.swift with `swift-tools-version: 6.0`, local `.package(path:)`
  references to `../gf256`, `../Barcode2D`, and `../PaintInstructions`.

- BUILD and BUILD_windows scripts (`if command -v xcrun …` for macOS CI
  compatibility).

- 67-test suite across 17 test suites using Swift Testing (`@Test`, `@Suite`,
  `#expect`), covering:
  - Symbol dimension verification (11×11 through 17×17)
  - Auto-version selection edge cases
  - Structural module placement (finder, separator, timing)
  - Determinism (same input → identical grid across calls)
  - ECC level constraints (valid and invalid combinations)
  - Error handling (inputTooLong, eccNotAvailable, emptyString)
  - Capacity boundary tests (at-max and overflow-by-one)
  - Format information non-zero module checks
  - Grid completeness (square, correct dimensions)
  - Cross-language corpus (shared reference test vectors)
  - Module value spot checks (fixed structural positions)
  - encodeAt convenience API
  - Numeric/alphanumeric/byte mode edge cases
  - All 8 valid symbol configurations
  - required_capabilities.json (pure computation, no I/O capabilities)

### Implementation notes

- The `@testable import` attribute is used in tests to give access to
  `MicroQRError` (which is public) and `encode()` (also public).
- The `BitWriter` is a class (reference type) rather than a struct because
  the encoding functions take it as a mutable argument and Swift's value
  semantics would require `inout` throughout.
- `error.localizedDescription` is not available on `any Error` in Swift 6
  strict concurrency mode. The `layoutGrid()` function uses
  `String(describing: error)` instead.
- The `EncodingMode` enum and all internal helpers are `private` or
  file-private, keeping the public API surface minimal.
