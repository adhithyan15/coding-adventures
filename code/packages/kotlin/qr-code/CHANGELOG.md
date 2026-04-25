# Changelog — qr-code (Kotlin)

All notable changes to this package are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of the QR Code encoder for Kotlin.
- Full ISO/IEC 18004:2015 compliant encoding pipeline:
  - Mode selection: Numeric, Alphanumeric, Byte (UTF-8).
  - Version selection: versions 1–40, automatically picks minimum version.
  - Bit stream assembly: mode indicator, character count, data, terminator, padding (0xEC/0x11).
  - Reed-Solomon ECC: GF(256) b=0 convention, polynomial 0x11D, using gf256 (MA01) package.
  - Block structure and interleaving per ISO Table 9.
  - Grid construction: finder patterns, separators, timing strips, alignment patterns (v2+), dark module.
  - Format information: 15-bit BCH(15,5) protected word with ISO XOR masking (0x5412). Correct MSB-first bit ordering per lessons.md.
  - Version information: 18-bit BCH protected word for versions 7–40.
  - Mask evaluation: all 8 mask patterns with full 4-rule penalty scoring.
  - Zigzag data placement: two-column snake from bottom-right, skipping column 6.
- `encode(input, ecc, minVersion)` — encodes a string to a `ModuleGrid`.
- `encodeAndLayout(input, ecc, config)` — encodes and produces a `PaintScene` via barcode-2d.
- Comprehensive test suite (30+ test cases):
  - All four ECC levels.
  - Forced versions 1–5.
  - Finder patterns at all three corners.
  - Finder pattern corner darkness.
  - Dark module position.
  - Timing strip correctness.
  - Format info BCH validity for all ECC levels.
  - Format info copy 1 / copy 2 consistency.
  - Standard test corpus (5 canonical inputs).
  - Determinism.
  - UTF-8 multi-byte characters.
  - Error handling (InputTooLong).
  - `encodeAndLayout` integration.
- `BUILD` file with serialisation lock (prevents Gradle daemon conflicts in parallel CI).
- `BUILD_windows` for Windows CI runners.
- `required_capabilities.json`: `["kotlin", "java"]`.

### Implementation notes

- The `BUILD` file redirects Gradle output to `gradle-build/` to avoid the
  `BUILD` filename collision on case-insensitive filesystems (macOS/Windows).
- Format info bit ordering follows the MSB-first rule from lessons.md:
  f14→f9 across row 8 cols 0–5, f0→f5 ascending down col 8 rows 0–5 for copy 1.
- RS encoder uses the b=0 convention (roots α^0, α^1, …, α^{n-1}) distinct
  from MA02 reed-solomon (which uses b=1).
- GF(256) arithmetic delegates entirely to the gf256 (MA01) package.
