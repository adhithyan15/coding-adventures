# Changelog — java/qr-code

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-24

### Added

- Initial release of the Java QR Code encoder.
- `QRCode.encode(String, EccLevel)` — full ISO/IEC 18004:2015 pipeline:
  - Automatic mode selection: Numeric, Alphanumeric, Byte (UTF-8)
  - Version selection: smallest v1–40 that fits the input at the ECC level
  - GF(256) Reed-Solomon ECC (b=0 convention, primitive poly 0x11D)
  - Two-group block structure (short + long blocks per ISO §7.5)
  - Round-robin interleaving of data and ECC codewords
  - Full grid construction: finder patterns × 3, separators, timing strips,
    alignment patterns (all versions), format info (BCH, two copies),
    version info (Golay code, v7+), dark module
  - Zigzag (two-column snake) codeword placement
  - Eight mask patterns evaluated; lowest-penalty mask selected
  - Format information MSB-first per ISO §7.9
- `QRCode.encodeAndLayout(String, EccLevel, Barcode2DLayoutConfig)` — encodes
  and renders to a `PaintScene` via `barcode-2d`'s layout engine
- `QRCode.EccLevel` enum: L, M, Q, H
- `QRCode.QRCodeException` checked exception for invalid inputs
- Composite build integration with `gf256`, `polynomial`, `reed-solomon`,
  `paint-instructions`, `barcode-2d` via `settings.gradle.kts`
- 30+ unit tests covering all encoding modes, ECC levels, grid structure,
  format information validity (BCH), determinism, error paths, and integration
