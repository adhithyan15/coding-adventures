# Changelog — data-matrix (Kotlin)

All notable changes to this package follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-06

### Added

- Initial release: complete Data Matrix ECC 200 encoder in idiomatic Kotlin.
- `DataMatrix.encode(data: String, shape: SymbolShape): Array<BooleanArray>` — primary public API.
- Self-contained GF(256)/0x12D field with precomputed exp/log tables (512 bytes, loaded once at class init).
- Reed-Solomon LFSR encoder over GF(256)/0x12D with b=1 roots (α^1 … α^n) as required by ISO/IEC 16022.
- Generator polynomials precomputed and cached for all ECC block sizes that appear in the symbol table.
- All 24 square ECC 200 symbol sizes (10×10 … 144×144) with correct data region, block, and ECC counts.
- All 6 rectangular ECC 200 symbol sizes (8×18 … 16×48).
- Symbol auto-selection: smallest symbol whose data capacity fits the encoded codeword count.
- `SymbolShape` enum: `SQUARE` (default), `RECTANGULAR`, `ANY`.
- ASCII mode encoder with digit-pair optimization (two consecutive digits → one codeword).
- UPPER_SHIFT for extended ASCII characters (128–255).
- Scrambled pad codewords (ISO §5.2.3) for unused symbol capacity.
- Physical grid initialization: L-finder (left column + bottom row all dark) + timing clock (top row + right column alternating) + alignment borders for multi-region symbols.
- Utah diagonal placement algorithm (ISO Annex F) with all four corner patterns and boundary wrapping.
- Logical → physical coordinate mapping for multi-region symbols.
- ISO right-and-bottom fill rule for residual unvisited modules.
- `InputTooLongException` with encoded codeword count and max capacity (1558).
- Comprehensive test suite (≥ 90% coverage): GF arithmetic, ASCII encoding, pad codewords, symbol selection, border invariants, multi-region alignment borders, Utah placement, integration tests, cross-language verification vectors, and error handling.

### Implementation notes

- The encoder is entirely self-contained — no external gf256 or reed-solomon Kotlin packages are consumed. All GF(256)/0x12D tables and RS logic are inlined in `DataMatrix.kt`, consistent with the Go and TypeScript ports.
- GF tables and generator polynomial cache are initialized at Kotlin object/top-level init time (class load), so first-encode latency is near-zero.
- Grid uses `Array<BooleanArray>` (not `Array<Array<Boolean>>`) for minimal heap allocation.
- All internal helpers (`encodeAscii`, `padCodewords`, `selectSymbol`, `computeInterleaved`, `initGrid`, `utahPlacement`) are marked `internal` so the test module can access them as white-box tests without exposing them as public API.
