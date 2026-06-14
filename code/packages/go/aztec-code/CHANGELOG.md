# Changelog — aztec-code (Go)

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-05-06

### Added

- `Encode(data string, opts *Options) (barcode2d.ModuleGrid, error)` — encode a
  UTF-8 string as an Aztec Code symbol grid. Auto-selects the smallest symbol
  (compact 1–4 layers, then full 1–32 layers) at ≥23% ECC.
- `EncodeBytes(input []byte, opts *Options) (barcode2d.ModuleGrid, error)` — same
  as `Encode` but accepts arbitrary `[]byte` for raw binary data.
- `EncodeToScene(data string, opts *Options, cfg barcode2d.Barcode2DLayoutConfig) (paintinstructions.PaintScene, error)` —
  convenience wrapper that encodes and converts to a pixel-resolved `PaintScene`
  using the `barcode2d.Layout` pipeline.
- `Options.MinEccPercent` — configures the minimum error-correction percentage
  (default 23, range 10–90).
- `InputTooLongError` — returned when input exceeds 32-layer full Aztec capacity.
- GF(16) arithmetic (log/antilog tables, `gf16Mul`, `gf16RsEncode`) for mode
  message Reed-Solomon encoding.
- GF(256)/0x12D arithmetic (`gf256Mul`, `gf256RsEncode`) for 8-bit codeword
  Reed-Solomon encoding (same polynomial as Data Matrix ECC200).
- Bit stuffing: `stuffBits` inserts a complement bit after every 4 consecutive
  identical bits.
- Mode message encoding: `encodeModeMessage` — compact (28 bits = 7 nibbles)
  and full (40 bits = 10 nibbles).
- Bullseye finder pattern placement using Chebyshev-distance rule.
- Orientation mark placement (4 corners of mode message ring, always dark).
- Reference grid placement for full symbols (alternating dark/light lines at
  every 16 modules from center).
- Clockwise layer spiral data placement, inside → outside.
- Full test suite with ≥90% coverage.

### Notes on v0.1.0 simplifications

- **Byte-mode only**: all input encoded via Binary-Shift escape from Upper mode.
  Multi-mode optimization (Digit/Upper/Lower/Mixed/Punct) is v0.2.0.
- **8-bit codewords**: GF(256)/0x12D RS for all data. GF(16)/GF(32) RS for
  4-bit/5-bit codeword modes is v0.2.0.
- **Auto-select only**: no force-compact option (v0.2.0).
