# Changelog — coding_adventures_qr_code

All notable changes to this package are documented here.
This project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-04-24

### Added

- **`QrCode.encode(data, level:, version:, mode:)`** — full QR Code encoder
  returning a `CodingAdventures::Barcode2D::ModuleGrid`.  Supports all four ECC
  levels (`:L`, `:M`, `:Q`, `:H`) and all 40 versions.

- **Encoding modes**: Numeric (3 digits → 10 bits), Alphanumeric (45-char set,
  pairs → 11 bits), and Byte (raw UTF-8).  Automatic mode selection picks the
  most compact mode that covers the entire input.

- **Version selection**: automatically selects the smallest version (1–40) whose
  data codeword capacity is sufficient for the encoded input at the chosen ECC
  level.  Version can also be forced explicitly.

- **Reed-Solomon ECC**: pure-Ruby LFSR implementation over GF(256) using the
  b=0 convention: `g(x) = ∏(x + αⁱ)` for `i = 0..n−1`.  Generator polynomials
  for all 13 distinct ECC codeword counts used in ISO 18004:2015 Table 9 are
  pre-built at module load time.

- **Data masking**: all 8 ISO 18004 mask patterns are evaluated; the pattern
  with the lowest 4-rule penalty score is selected.  Format information is
  written after mask selection.

- **Full module placement**: finder patterns, separators, timing strips,
  alignment patterns (version 2+), format information (15-bit BCH(15,5), two
  copies), version information (18-bit BCH(18,6), v7+), and the always-dark
  module at `(4V+9, 8)`.

- **`QrCode.encode_to_scene(data, ...)`** — encodes and converts the grid to a
  `PaintScene` via `CodingAdventures::Barcode2D.layout()`.

- **Error types**: `QrCode::QrCodeError` (base), `QrCode::InputTooLongError`
  (raised when input exceeds QR v40 capacity).

- **Lessons applied**:
  - Format information is written MSB-first (`f14` at row 8 col 0) per the
    lessons.md fix from 2026-04-23.  This is the most common source of
    unscannable QR codes; the implementation was verified correct before commit.

### Notes

- Kanji encoding (mode `1000`) is deferred to v0.2.0.
- Mixed-mode (segmented) encoding is deferred to v0.2.0.
- ECI mode is deferred to v0.2.0.
