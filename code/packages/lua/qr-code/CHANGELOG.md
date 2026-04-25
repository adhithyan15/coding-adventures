# Changelog — coding-adventures-qr-code (Lua)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-24

### Added

- Initial release of the Lua QR Code encoder (ISO/IEC 18004:2015).
- `encode(data, level)` — encode any UTF-8 string to a `ModuleGrid`.
  - Supports all four ECC levels: L (~7%), M (~15%), Q (~25%), H (~30%).
  - Automatic mode selection: numeric, alphanumeric, or byte.
  - Version selection 1–40 (smallest version that fits at the chosen ECC level).
  - Full structural module placement: finders, separators, timing strips,
    alignment patterns (v2+), format information, version information (v7+),
    dark module.
  - 8-pattern mask evaluation with 4-rule ISO penalty scoring; lowest-penalty
    mask is selected.
  - RS ECC computed via `coding-adventures-gf256` (b=0 convention, poly 0x11D).
  - Interleaved block encoding as specified by ISO 18004 Table 9.
- Returns a `ModuleGrid` table compatible with `coding-adventures-barcode-2d`'s
  `layout()` function for pixel-level rendering.
- Structured error returns (no exceptions): `{kind, message}` tables for
  `InputTooLongError` and `QRCodeError`.
- Comprehensive spec suite (`spec/qr_code_spec.lua`) covering:
  - API contract (VERSION, encode return shape, default ECC level)
  - Mode selection (numeric / alphanumeric / byte)
  - Version selection (version 1 at short inputs, higher versions for longer data)
  - Grid geometry (size formula 4v+17)
  - Finder pattern structure (outer ring, inner light ring, dark core)
  - Timing strip alternation (row 7 and col 7 in 1-indexed Lua)
  - Dark module placement
  - All four ECC levels
  - Edge cases (empty string, single character, exact capacity boundaries)
  - Determinism (same input → same output)
  - Large inputs (200-char numeric, 500-char alphanumeric)
  - Error handling (invalid ECC level, input exceeding v40 capacity)

### Implementation notes

- Lua 5.4+ bitwise operators used throughout (`~` for XOR, `<<`/`>>` for shifts).
- All grid coordinates are 1-indexed in module tables (Lua convention).
  Formulas from the ISO standard and the TypeScript reference use 0-indexed
  coordinates; these are converted with `+ 1` at grid access points.
- The GF(256) ALOG table is built locally (the `gf256` module does not export
  its internal tables). The `gf256.multiply()` function is used for RS ECC.
- Format information bit ordering follows the correction documented in
  `lessons.md` (2026-04-23): MSB-first across row 8 cols 0–5, then specific
  single bits at (8,7), (8,8), (7,8), then LSB-first down col 8 rows 0–5.
