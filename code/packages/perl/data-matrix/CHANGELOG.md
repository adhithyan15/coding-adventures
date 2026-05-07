# Changelog

All notable changes to `CodingAdventures::DataMatrix` are documented here.

## [0.1.0] — 2026-05-06

### Added

- Initial release: ISO/IEC 16022:2006 Data Matrix ECC200 encoder.
- GF(256) arithmetic with primitive polynomial 0x12D (same as Aztec Code;
  different from QR Code's 0x11D). Pre-built exp/log lookup tables.
- RS generator polynomial construction and caching (b=1 convention:
  roots α^1..α^n). Matches the MA02 reed-solomon convention exactly.
- RS encoding via LFSR polynomial-division algorithm. All 24 square and
  6 rectangular symbol sizes covered.
- All 24 square symbol sizes: 10×10 through 144×144.
- All 6 rectangular symbol sizes: 8×18 through 16×48.
- ASCII encoding with digit-pair optimization (two consecutive digits
  packed into a single codeword, 130+d1*10+d2).
- Extended ASCII (UPPER_SHIFT: codeword 235 + char-127) for bytes 128–255.
- Pad codeword scrambling (ISO/IEC 16022:2006 §5.2.3): first pad = 129,
  subsequent pads = scrambled via 149×k mod 253.
- RS block splitting and round-robin interleaving for multi-block symbols
  (44×44 upward). The first `dataCW mod numBlocks` blocks get one extra
  data codeword (ISO interleaving convention).
- Grid initialization: L-shaped finder (left column + bottom row all dark),
  timing clock (top row + right column alternating starting dark), and
  2-module-wide alignment borders for multi-region symbols.
- Utah diagonal codeword placement algorithm with all four corner special
  cases (corner patterns 1–4) and the five boundary-wrap rules from
  ISO/IEC 16022:2006, Annex F.
- Residual fill: unset logical grid positions filled with (r+c) mod 2 == 1.
- Logical-to-physical coordinate mapping for multi-region symbols.
- Public API: `encode_data_matrix($data, \%options)` and `encode` alias.
  Options: `shape => 'square' | 'rectangular' | 'any'` (default `square`).
- Returns a `ModuleGrid` hashref compatible with `CodingAdventures::Barcode2D`.
- Croaks with `InputTooLong: ...` when data exceeds 144×144 capacity.
- Test suite with 40 subtests covering GF arithmetic, ASCII encoding, padding,
  symbol selection, RS encoding, border invariants, alignment borders,
  Utah placement, determinism, rectangle support, binary/UTF-8 input,
  error paths, and the `encode()` alias.
