# Changelog — micro-qr (Java)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-24

### Added

- `MicroQR.encode(String)` — convenience overload, auto-selects symbol and ECC.
- `MicroQR.encode(String, MicroQRVersion, EccLevel)` — full control over version
  and ECC level.
- `MicroQR.MicroQRVersion` enum: `M1`, `M2`, `M3`, `M4`.
- `MicroQR.EccLevel` enum: `DETECTION`, `L`, `M`, `Q`.
- `MicroQR.MicroQRException` base exception with four subclasses:
  - `InputTooLongException`
  - `ECCNotAvailableException`
  - `UnsupportedModeException`
  - `InvalidCharacterException`
- Three encoding modes:
  - **Numeric** (M1–M4): digits 0–9; groups of 3 → 10 bits.
  - **Alphanumeric** (M2–M4): 45-char QR set; pairs → 11 bits.
  - **Byte** (M3–M4): raw UTF-8 bytes; each byte → 8 bits.
- Reed-Solomon ECC (GF(256)/0x11D, b=0 convention, single block).
  - Generator polynomials for n = 2, 5, 6, 8, 10, 14 codewords.
- Full structural module placement:
  - 7×7 finder pattern (top-left corner, rows 0–6 cols 0–6).
  - L-shaped separator (row 7 cols 0–7 and col 7 rows 0–7).
  - Timing patterns at row 0 (cols 8+) and col 0 (rows 8+).
  - 15-module format information strip (row 8 and col 8).
- Two-column zigzag data placement from bottom-right.
- Four mask patterns with full penalty scoring (rules 1–4).
- Pre-computed 32-entry format information table (XOR mask 0x4445).
- JUnit 5 test suite with >90% coverage.
- `BUILD` script with shared lock (`jvm-barcode.lock`) to prevent parallel
  Gradle invocations from corrupting shared composite-build outputs.
- `build.gradle.kts` with `layout.buildDirectory = file("gradle-build")` to
  avoid `BUILD` / `build/` name collision on case-insensitive filesystems.
- `settings.gradle.kts` with composite builds for `gf256`, `barcode-2d`, and
  `paint-instructions`.
