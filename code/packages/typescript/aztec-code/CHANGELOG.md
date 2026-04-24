# Changelog — @coding-adventures/aztec-code

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of the Aztec Code encoder (ISO/IEC 24778:2008).
- Compact Aztec support: 1–4 layers, 15×15 to 27×27 symbols.
- Full Aztec support: 1–32 layers, 19×19 to 143×143 symbols.
- Bullseye finder pattern using Chebyshev-distance ring rule.
- Orientation marks: 4 always-dark corners of the mode message ring.
- Mode message encoding:
  - Compact: 28-bit (7 nibbles, GF(16)/0x13 RS, 2 data + 5 ECC nibbles).
  - Full: 40-bit (10 nibbles, GF(16)/0x13 RS, 4 data + 6 ECC nibbles).
- GF(16)/0x13 Reed-Solomon for mode message (inline implementation).
- GF(256)/0x12D Reed-Solomon for 8-bit data codewords (b=1 convention,
  same polynomial as Data Matrix ECC200, implemented inline).
- Bit stuffing: after 4 consecutive identical bits, insert complement bit.
- Clockwise spiral data placement (2-module-wide bands, outer-before-inner).
- Reference grid for full symbols: center row/col + ±16n lines.
- Auto-selection of compact vs full based on data length and ECC percentage.
- Default ECC: 23% of total codewords.
- `encode(input, options?)` — returns a `ModuleGrid`.
- `encodeAndLayout(...)` — returns a `PaintScene` via `barcode-2d`.
- `renderSvg(...)` — returns an SVG string via `paint-vm-svg`.
- `explain(...)` — returns an `AnnotatedModuleGrid` (v0.1.0: null annotations).
- Comprehensive test suite with >80% coverage:
  - GF(16) RS correctness via mode message.
  - Bit stuffing edge cases.
  - Bullseye Chebyshev pattern verification.
  - Orientation mark placement.
  - Grid dimension and formula verification.
  - Cross-language test corpus (A, Hello World, https://example.com, etc.).

### v0.1.0 simplifications (noted for v0.2.0)

- Byte mode only (via Binary-Shift from Upper mode). Multi-mode encoding
  (Digit/Upper/Lower/Mixed/Punct) is planned for v0.2.0.
- `AztecOptions.compact` and `AztecOptions.minEccPercent` are implemented;
  multi-mode encoding options are deferred.
