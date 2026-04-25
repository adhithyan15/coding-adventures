# Changelog — data-matrix (Rust)

## 0.1.0 — 2026-04-24

Initial release.

### Added

- `encode(input: &[u8], options: DataMatrixOptions) -> Result<ModuleGrid, DataMatrixError>` — Full Data Matrix ECC200 encoding pipeline:
  - ASCII encoding: single bytes as `ASCII+1`, paired digits packed into one codeword (130 + d1×10 + d2), extended ASCII via UPPER_SHIFT (235)
  - Symbol selection: smallest square or rectangular symbol (from all 30 ISO/IEC 16022:2006 sizes) whose data capacity fits the encoded codeword count
  - Scrambled-pad fill: 129 followed by position-scrambled pad codewords (ISO §5.2.3)
  - Reed-Solomon ECC over GF(256)/0x12D with b=1 root convention (α^1..α^n); block splitting with round-robin data + ECC interleaving
  - Grid initialisation: L-finder (left column + bottom row), timing borders (top row + right column alternating), alignment borders for multi-region symbols
  - Utah diagonal codeword placement with four corner patterns (ISO/IEC 16022:2006 Annex F); no masking applied

- `encode_str(input: &str, options: DataMatrixOptions) -> Result<ModuleGrid, DataMatrixError>` — convenience UTF-8 wrapper around `encode()`

- `encode_and_layout(input: &[u8], options: DataMatrixOptions, config: Option<Barcode2DLayoutConfig>) -> Result<PaintScene, DataMatrixError>` — encode + pixel geometry via `barcode-2d`'s `layout()`

- `DataMatrixError` enum — `InputTooLong(String)`. Implements `Display`, `Error`.

- `SymbolShape` enum — `Square` (default), `Rectangular`, `Any`.

- `DataMatrixOptions` struct — `shape: SymbolShape`.

- GF(256)/0x12D field tables lazily initialised via `std::sync::OnceLock` (generator g = 2, primitive polynomial 0x12D = x⁸+x⁵+x⁴+x²+x+1).

- All 24 square symbol sizes and 6 rectangular symbol sizes from ISO/IEC 16022:2006 Table 7.

- `VERSION` constant — `"0.1.0"`.

- 40 unit tests covering GF field arithmetic, ASCII encoding, pad codewords, Reed-Solomon systematic check, symbol selection, Utah placement dimensions, L-finder/timing borders, and cross-language corpus cases matching the TypeScript reference encoder.

### Dependencies

- `barcode-2d` — `ModuleGrid` type, `layout()` pixel geometry
- `paint-instructions` — `PaintScene` type (for `encode_and_layout` return type)
