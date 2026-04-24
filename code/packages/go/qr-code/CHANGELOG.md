# Changelog — qr-code (Go)

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of the QR Code encoder, ISO/IEC 18004:2015 compliant.
- `Encode(data string, opts EncodeOptions) (barcode2d.ModuleGrid, error)` — the
  main entry point. Encodes any UTF-8 string into a QR Code module grid.
- `EncodeToScene(data string, opts EncodeOptions, cfg barcode2d.Barcode2DLayoutConfig)` —
  convenience function that calls `Encode` and then `barcode2d.Layout` in one step.
- All 4 error correction levels: L, M, Q, H.
- Encoding modes: Numeric (digits only), Alphanumeric (45-char QR alphanum set),
  Byte (raw UTF-8). Mode is auto-selected for maximum compactness.
- Version selection: 1–40, picks the smallest version that fits the input at the
  chosen ECC level. Forced version also supported via `EncodeOptions.Version`.
- Reed-Solomon ECC with the b=0 root convention (g(x) = ∏(x + α^i) for i=0..n-1),
  distinct from MA02's b=1. Generator polynomials are cached after first build.
- Data interleaving across RS blocks for burst-error resilience.
- Module placement via two-column zigzag scan (bottom-right to top-left).
- All 8 mask patterns evaluated; lowest-penalty mask selected (ISO 18004 §7.8.3).
- Format information (15-bit BCH-protected) written at both copy locations.
- Version information (18-bit BCH-protected) for versions 7–40.
- Structural elements: finder patterns, separators, timing strips, alignment
  patterns, dark module.
- `InputTooLongError` returned when input exceeds version-40 capacity.
- `InvalidInputError` returned when input contains characters invalid for the mode.
- 94.4% test coverage across 47 unit and integration tests.
- Knuth-style literate comments throughout explaining every algorithm.
