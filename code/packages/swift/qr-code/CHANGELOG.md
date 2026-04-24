# Changelog — qr-code (Swift)

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-24

### Added

- **`QrCode.encode(_:level:)`** — encodes any UTF-8 string into a
  `Barcode2D.ModuleGrid`. Returns a `(4V+17) × (4V+17)` boolean grid,
  `true` = dark module.

- **`ErrorCorrectionLevel`** — public enum with cases `.low`, `.medium`,
  `.quartile`, `.high`. Defaults to `.medium`.

- **`QrCodeError`** — public error enum:
  - `.inputTooLong` — thrown when the input exceeds version-40 capacity.
  - `.encodingError` — thrown on internal precondition violations.

- **Encoding modes**: automatic selection of numeric, alphanumeric, or byte
  mode based on the input character set.

- **Version selection**: selects the minimum version (1-40) that fits the input
  at the chosen ECC level.

- **Reed-Solomon ECC**: embedded LFSR-based RS encoder using the b=0 root
  convention required by ISO/IEC 18004. Does NOT wrap the repo-level
  `ReedSolomon` package (which uses b=1).

- **Full module placement**: finder patterns, separators, timing strips,
  alignment patterns, format information, version information (v7+), and the
  always-dark module.

- **Data placement**: two-column zigzag scan from the bottom-right corner,
  skipping column 6 (vertical timing strip) and all reserved modules.

- **Mask selection**: all 8 mask patterns evaluated; the one with the lowest
  4-rule penalty score (ISO 18004 §7.8.3) is selected.

- **`Tables.swift`**: ISO lookup tables (ECC_CODEWORDS_PER_BLOCK, NUM_BLOCKS,
  ALIGNMENT_POSITIONS) embedded as constants.

- **42 unit tests** covering geometry helpers, mode selection, bit writer, RS
  generator, RS encoder, version selection, full encode, format/version info
  BCH, block processing, mask application, and structural properties.

- **`.gitignore`** — excludes `.build/` and `.swiftpm/` to prevent CI failures
  on Windows (deeply nested paths) and repo bloat.
