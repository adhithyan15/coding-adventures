# Changelog

## 0.1.0 (2026-05-06)

### Added

- Initial release: ISO/IEC 15438:2015 PDF417 stacked linear barcode encoder.
- `encodePDF417 :: String -> Either PDF417Error ModuleGrid` — main public API.
- `encodePDF417With :: String -> PDF417Options -> Either PDF417Error ModuleGrid` — with options.
- Three compaction modes:
  - **Byte compaction** (codeword 924 latch): 6 bytes → 5 base-900 codewords; remainder 1:1.
  - **Text compaction** (codeword 900 latch): UC/LC/ML/PL sub-modes, 2 chars per codeword.
  - **Numeric compaction** (codeword 902 latch): 44-digit chunks → ≤15 codewords (~2.93 digits/cw).
  - **Auto-compaction**: selects numeric for all-digit, text for ASCII-safe, byte otherwise.
- GF(929) arithmetic over the prime field ℤ/929ℤ:
  - `gfAdd`, `gfSub`, `gfMul` with precomputed log/antilog tables.
  - `powMod` for fast modular exponentiation.
- Reed-Solomon ECC over GF(929), b=3 convention (roots α^3..α^{k+2}):
  - `buildGenerator`: builds the generator polynomial for ECC level 0–8.
  - `rsEncode`: shift-register encoder; no interleaving (unlike QR Code).
  - `autoEccLevel`: recommends minimum level based on data codeword count.
- Symbol layout:
  - `chooseDimensions`: selects roughly-square (cols, rows) with c = ceil(sqrt(total/3)).
  - `computeLRI`, `computeRRI`: row indicator codewords encoding R, C, ECC level.
  - Three cluster tables (929 × 3 = 2787 packed patterns, sourced from the TypeScript reference implementation, MIT-licensed).
  - Start pattern: `[8,1,1,1,1,1,1,3]` → 17 modules `11111111010101000`.
  - Stop pattern: `[7,1,1,3,1,1,1,2,1]` → 18 modules `111111101000101001`.
  - `rasterize`: converts codeword sequence to `ModuleGrid`.
- `PDF417Options`: `eccLevel`, `columns`, `rowHeight` configuration.
- `PDF417Error`: `InputTooLong`, `InvalidECCLevel`, `InvalidDimensions`.
- Thorough hspec test suite (≥90% coverage).
