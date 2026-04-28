# Changelog — DataMatrix (Swift)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Version numbers follow [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-04-27

### Added

- **`DataMatrix.swift`** — full ISO/IEC 16022:2006 ECC200 encoder:
  - `encode(_:options:)` — encode a string to a `ModuleGrid` using auto-selected
    or explicitly forced symbol size and shape.
  - `encode(_:squareSize:)` — convenience overload for explicit square sizes.
  - `encode(_:rows:cols:)` — encode to a specific rectangular symbol size.
  - `gridToString(_:)` — debug utility: render a `ModuleGrid` as '0'/'1' text.
  - `dataMatrixVersion` constant (`"0.1.0"`).
  - `gf256Prime` constant (`0x12D = 301`) — the Data Matrix primitive polynomial.

- **`DataMatrixOptions`** struct:
  - `size: Int?` — nil for auto-select, or explicit square side length.
  - `shape: SymbolShape` — `.square` (default), `.rectangle`, or `.any`.

- **`SymbolShape`** enum: `.square`, `.rectangle`, `.any`.

- **`DataMatrixError`** enum:
  - `.inputTooLong(String)` — input exceeds 144×144 capacity (1558 codewords).
  - `.invalidSize(String)` — requested size is not a valid ECC200 symbol.

- Full GF(256)/0x12D field arithmetic (exp/log tables distinct from QR's 0x11D).
- RS generator polynomials for all 17 distinct ECC block sizes used by ECC200,
  precomputed at module-load time into an immutable dictionary (Swift 6 concurrency safe).
- Reed-Solomon LFSR shift-register encoding (GF(256)/0x12D, b=1 convention).
- ASCII mode encoding with digit-pair compaction (ISO §5.2.4).
- Scrambled EOM pad sequence (ISO §5.2.3).
- Symbol size table: all 24 square + 6 rectangular sizes from ISO Table 7.
- Multi-block data interleaving (round-robin data then ECC).
- Grid initializer: L-shaped finder, timing borders, alignment borders for
  multi-region symbols (region_rows × region_cols > 1).
- Utah diagonal codeword placement algorithm (ISO Annex F):
  - Four boundary wrap rules.
  - Four corner placement patterns (Corner1–Corner4).
  - ISO right-and-bottom fill rule for unvisited residual modules.
- Logical → physical coordinate mapping for multi-region symbols.

- **`DataMatrixTests.swift`** — 72 tests across 16 suites using Swift Testing:
  - Package constants, auto-size selection, grid structure, determinism.
  - L-finder pattern (bottom row + left column all dark).
  - Timing border pattern (alternating top row + right column, with corner rules).
  - Empty input, error handling, explicit size overrides.
  - Rectangular symbols, digit-pair compaction, multi-region symbols.
  - Cross-language corpus tests for interoperability.

- **`README.md`** — full documentation with API reference, symbol size tables,
  encoding pipeline diagram, and usage examples.

### Technical notes

- **Swift 6 concurrency**: generator polynomials are precomputed into an immutable
  `let` constant at module load time, avoiding the `nonisolated global mutable state`
  error that a lazy `var` cache would trigger under strict concurrency.
- **GF(256)/0x12D vs 0x11D**: Data Matrix uses a different irreducible polynomial
  than QR Code. This package builds its own exp/log tables and does not reuse
  the `GF256` package's tables (which use 0x11D). The `GF256` package is still
  listed as a dependency for consistency with the wider barcode stack.
- **No masking**: unlike QR Code, Data Matrix ECC200 has no masking step. The
  Utah diagonal placement naturally distributes bits without clustering.
- **Corner pixel rules**: the top-right corner `(0, C-1)` is always dark because
  the right column is initialized after the top row, and row 0 is even → dark.
  The bottom-right corner `(R-1, C-1)` is always dark because the L-finder
  bottom row is written last and overrides everything.
