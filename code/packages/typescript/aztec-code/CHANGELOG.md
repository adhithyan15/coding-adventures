# Changelog — @coding-adventures/aztec-code

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-24

### Added

- Initial release of `@coding-adventures/aztec-code`.
- `encode(data, options?)` — encodes a string or `Uint8Array` into a `ModuleGrid`.
- `encodeAndLayout(data, options?, config?)` — convenience wrapper that also calls `barcode-2d`'s `layout()`.
- `renderSvg(data, options?, config?)` — convenience wrapper that produces an SVG string.
- `explain(data, options?)` — returns an `AnnotatedModuleGrid` (annotations stubbed in v0.1.0).
- `AztecError` and `InputTooLongError` error classes.
- Full ISO/IEC 24778:2008 compliant bullseye finder pattern (compact: 11×11, full: 15×15).
- Correct orientation marks (4 dark corners of the mode message ring).
- GF(16) Reed-Solomon for mode message (compact: (7,2) code; full: (10,4) code).
- GF(256)/0x12D Reed-Solomon for 8-bit data codewords (same polynomial as Data Matrix).
- Bit stuffing algorithm (insert complement after every 4 consecutive identical bits).
- Data layer clockwise spiral placement (innermost layer outward).
- Reference grid support for full symbols (alternating dark/light at 16-module intervals).
- Auto-selection of compact (1–4 layers) vs full (1–32 layers) based on input size.
- 68 unit tests, 100% line coverage, 93% branch coverage.

### Implementation notes

- v0.1.0 uses byte mode only (Binary-Shift from Upper mode for all input).
  Multi-mode optimization (Digit, Upper, Lower, Mixed, Punct) is planned for v0.2.0.
- GF(256) RS uses polynomial 0x12D (Aztec/Data Matrix), implemented inline since the
  repo's `@coding-adventures/gf256` package uses 0x11D (QR Code polynomial).
- Capacity tables are embedded as lookup arrays derived from ISO/IEC 24778:2008 Table 1.
