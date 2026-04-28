# Changelog — data-matrix (Go)

All notable changes to this package are documented here.
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-04-24

### Added

- **Full Data Matrix ECC200 encoder** conforming to ISO/IEC 16022:2006.

- **GF(256)/0x12D field arithmetic** — exp and log tables for the Data Matrix
  primitive polynomial (different from QR Code's 0x11D). Pre-built at init time.
  `GFMul` uses the log/antilog trick for O(1) field multiplication.

- **24 square symbol sizes** — 10×10 through 144×144, covering all primary
  square sizes from ISO/IEC 16022:2006 Table 7.

- **6 rectangular symbol sizes** — 8×18 through 16×48, for width-constrained
  print areas.

- **ASCII encoding** with digit-pair optimization — two consecutive ASCII digits
  are packed into one codeword (value 130–229), halving the codeword budget for
  numeric strings. Standard ASCII characters encode as (value + 1). Extended
  ASCII (128–255) uses the UPPER_SHIFT (235) prefix mechanism.

- **Scrambled pad codewords** (ISO/IEC 16022:2006 §5.2.3) — unused symbol
  capacity is filled with pseudo-random values derived from `149 × k mod 253`
  to prevent degenerate placement patterns.

- **Reed-Solomon ECC** — inline LFSR shift-register encoder over GF(256)/0x12D
  with b=1 root convention (roots α¹…αⁿ). Generator polynomials built at init
  time for all required ECC lengths (5, 7, 10, 11, 12, 14, 17, 18, 20, 21,
  24, 28, 36, 42, 48, 56, 62, 68). Verified by syndrome-zero check.

- **Multi-block interleaving** — data and ECC codewords from multiple RS blocks
  are interleaved round-robin for burst-error resilience.

- **L-shaped finder + timing border** — left column and bottom row all dark
  (L-finder); top row and right column alternating dark/light (timing clock).
  Alignment borders (2 modules wide, all-dark + alternating) placed between
  data regions for multi-region symbols.

- **Utah diagonal placement algorithm** — places all codeword bits onto the
  logical data matrix using the diagonal zigzag with four special corner
  patterns for boundary wrap. ISO §5 right-and-bottom fill rule applied to
  any residual unset modules.

- **Logical-to-physical coordinate mapping** — translates logical data matrix
  coordinates (accounting for multi-region layout with 2-module alignment
  borders and 1-module outer border) to physical symbol coordinates.

- **`Encode(input []byte, opts Options) (barcode2d.ModuleGrid, error)`** —
  primary encoding entry point.

- **`EncodeString(input string, opts Options) (barcode2d.ModuleGrid, error)`** —
  convenience wrapper for UTF-8 strings.

- **`EncodeToScene(input []byte, opts Options, cfg barcode2d.Barcode2DLayoutConfig) (paintinstructions.PaintScene, error)`** —
  encodes + lays out to a pixel-resolved PaintScene. Defaults quiet zone to
  1 module (narrower than QR's 4-module quiet zone; L-finder is self-delimiting).

- **`InputTooLongError`** — structured error with `EncodedCW` and `MaxCW` fields,
  compatible with `errors.As` and `errors.Is`.

- **`SymbolShape` options** — `SymbolShapeSquare` (default), `SymbolShapeRectangular`,
  `SymbolShapeAny`.

- **56 tests** with 95.4% statement coverage, covering:
  - GF(256)/0x12D tables (exp, log, mul, commutativity, distributivity, field order)
  - ASCII encoding (single chars, digit pairs, odd-length runs, extended ASCII)
  - Pad codewords (ISO worked example: "A" → [66, 129, 70])
  - Symbol selection (all boundary conditions, shape filtering, error cases)
  - RS encoding (generator polynomial degrees, roots verification, ISO example)
  - Block interleaving (single-block, two-block round-robin)
  - Grid initialization (finder pattern, timing, corner overrides, alignment borders)
  - Utah placement (determinism, grid dimensions, multi-region)
  - Logical-to-physical mapping (single and multi-region)
  - Full pipeline (10×10 through 144×144, rectangular, error propagation)
  - `EncodeToScene` integration

### Implementation notes

- RS encoding is inlined rather than delegating to the `reed-solomon` (MA02)
  package, to keep the dependency count minimal. MA02 could be used directly
  (it supports b=1 with configurable polynomial) but the inline approach avoids
  adding a transitive dependency.

- The `gf256` package uses 0x11D (QR Code polynomial) and is NOT used for
  Data Matrix field arithmetic. The separate `dmGFExp`/`dmGFLog` tables in this
  package use 0x12D exclusively.

- Generator polynomials are computed on first use and cached. All polynomials
  needed by the 30 standard symbol sizes are pre-built at package `init` time
  to avoid first-encode latency.
