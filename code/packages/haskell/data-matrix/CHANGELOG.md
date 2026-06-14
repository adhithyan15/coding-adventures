# Changelog — data-matrix

## 0.1.0 — 2026-04-27

### Added

- `CodingAdventures.DataMatrix` module implementing ISO/IEC 16022:2006 Data Matrix ECC200 encoder
- Support for all 24 square symbols (10×10 to 144×144) from Table 7
- Support for all 6 rectangular symbols (8×18 to 16×48) from Table 7
- ASCII encoding with digit-pair compaction (two consecutive digits → one codeword using `130 + d1*10 + d2`)
- Extended ASCII (128–255) via UPPER_SHIFT (codeword 235) mechanism
- Scrambled-pad codeword sequence for unused symbol capacity
- GF(256)/0x12D Reed-Solomon encoder with b=1 convention (roots α¹…α^n)
- Multi-block interleaving (data round-robin then ECC round-robin)
- L-shaped finder pattern placement (left column + bottom row, all dark)
- Alternating timing border (top row and right column, dark at even positions)
- Alignment borders for multi-region symbols (every 2 rows/cols between data regions)
- Utah diagonal placement algorithm with all 4 corner patterns
- Boundary wrap rules from ISO/IEC 16022:2006 Annex F
- Logical-to-physical coordinate mapping for multi-region symbols
- Fill pattern for residual unvisited modules: `(r+c) mod 2 == 1`
- `SymbolShape` type: `Square`, `Rectangular`, `AnyShape`
- `DataMatrixOptions` record with `dmShape` field
- `defaultOptions` with square-only selection
- `DataMatrixError` type with `InputTooLong` and `InvalidSymbolSize` variants
- `encode`, `encodeAt`, `encodeAndLayout` public API functions
- Integration with `barcode-2d`'s `ModuleGrid`, `emptyGrid`, `setModule`, `layout`
- Comprehensive hspec test suite with 30+ test cases covering all public API paths
- Literate Haskell code with Knuth-style inline comments, worked examples, and algorithmic notes
