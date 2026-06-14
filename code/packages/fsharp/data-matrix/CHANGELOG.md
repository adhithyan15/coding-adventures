# Changelog — CodingAdventures.DataMatrix (F#)

All notable changes to this package are documented here.

## [0.1.0] — 2026-05-06

### Added

- Initial release of the F# Data Matrix ECC200 encoder.
- ISO/IEC 16022:2006 compliant encoder supporting all 24 square ECC200 symbol
  sizes (10×10 through 144×144) and 6 rectangular sizes (8×18 through 16×48).
- Reed-Solomon error correction over GF(256) with primitive polynomial `0x12D`
  and b=1 generator roots convention (α^1 … α^n), matching the MA02
  reed-solomon package convention.
- Multi-block RS interleaving: data round-robin then ECC round-robin.
- ASCII encoding mode: single ASCII characters (byte+1), digit-pair compression
  (two consecutive digits → one codeword), and extended ASCII via UPPER_SHIFT.
- Scrambled pad codewords (ISO/IEC 16022:2006 §5.2.3) fill unused capacity.
- Grid initialisation: L-shaped finder (solid-dark left column and bottom row)
  plus alternating timing clock on the top row and right column.
- Alignment borders for multi-region symbols (32×32 and larger): all-dark
  row/column followed by alternating row/column.
- Utah diagonal codeword placement algorithm with all four corner patterns and
  residual fill per ISO/IEC 16022:2006 Annex F.
- Output is a `ModuleGrid` from `CodingAdventures.Barcode2D` — compatible with
  `Barcode2D.layout` for pixel-level rendering.
- Comprehensive unit tests covering: GF(256) table correctness, multiplication,
  ASCII encoding rules, pad scrambling, symbol selection, RS ECC length and
  range, grid border geometry, full encode pipeline for multiple inputs,
  determinism, byte-array entry point, InputTooLong error path, digit-pair
  packing savings, multi-region alignment borders, and Utah grid dimensions.

### Limitations (deferred to v0.2.0)

- ASCII encoding mode only — C40, Text, X12, EDIFACT, and Base256 modes are
  reserved for v0.2.0.
- Square-symbol-first selection only — no explicit rectangular-preference
  option in v0.1.0.
