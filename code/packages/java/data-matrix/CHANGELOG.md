# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-26

### Added

- Initial implementation of Data Matrix ECC200 encoder (ISO/IEC 16022:2006).

#### Core encoding pipeline

- **ASCII encoding** with digit-pair optimization:
  - Two consecutive ASCII digits → one codeword (e.g., "12" → 142)
  - Single ASCII chars 0–127 → `ASCII_value + 1`
  - Extended ASCII 128–255 → UPPER_SHIFT (235) then shifted value
- **Symbol size selection**: automatically picks the smallest symbol that fits
  the encoded codeword count; supports all 24 square and 6 rectangular sizes
- **Pad codewords**: first pad is literal 129; subsequent pads scrambled using
  the `149 × k mod 253` formula to prevent degenerate placement patterns
- **Reed-Solomon ECC** over GF(256)/0x12D (Data Matrix's field polynomial,
  distinct from QR's 0x11D), using the b=1 generator root convention:
  `g(x) = (x+α)(x+α^2)···(x+α^n_ecc)`
- **Multi-block interleaving**: for large symbols, data and ECC are split
  across multiple RS blocks and interleaved to distribute burst errors
- **Grid initialization**: L-shaped finder (left column + bottom row, all dark),
  timing clock (top row + right column, alternating), and alignment borders for
  multi-region symbols (32×32 and larger)
- **Utah diagonal placement algorithm**: places 8 codeword bits at each step
  using the diagonal "Utah" shape; handles all four corner patterns for
  boundary wrapping; fills unused positions with `(r+c)%2==1` fill pattern
- **Logical-to-physical coordinate mapping**: correctly accounts for outer
  border (+1) and alignment borders (+2 per region boundary)

#### Symbol sizes supported

- 24 square sizes: 10×10 through 144×144
- 6 rectangular sizes: 8×18, 8×32, 12×26, 12×36, 16×36, 16×48
- Shape preference: `SQUARE` (default), `RECTANGULAR`, or `ANY`

#### Testing

- 61 unit and integration tests with JUnit 5
- GF(256)/0x12D field arithmetic verification (exp/log tables, gfMul)
- ASCII encoding edge cases (digit pairs, extended ASCII, odd lengths)
- Pad codeword scrambling formula verification
- RS ECC encoding verified against TypeScript reference implementation
- Symbol border pattern tests (L-finder, timing, corner invariants)
- Alignment border tests for 32×32 (2×2 region symbol)
- Full pipeline tests for cross-language corpus inputs:
  - "A" → 10×10
  - "1234" → 10×10 (digit pairs)
  - "Hello World" → 16×16
  - "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789" → 24×24
  - "https://coding-adventures.dev" → 22×22
- Determinism verification (encoding same input twice → identical grids)
- Error handling (InputTooLongException, null options defaults)

#### Implementation notes

- `layout.buildDirectory = file("gradle-build")` to avoid case-insensitive
  filesystem collision between Gradle's `build/` output and repo's `BUILD` file
- Generator polynomials computed dynamically using `buildGenerator(nEcc)` with
  a static cache — avoids embedding large constant arrays while remaining fast
- All GF tables and generators pre-built in a `static {}` initializer for
  zero-latency first call
- No external dependencies beyond `barcode-2d` and `gf256` (both in-monorepo)
