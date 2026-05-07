# Changelog - CodingAdventures.DataMatrix.CSharp

## [0.1.0] - 2026-05-06

### Added

- Initial native C# Data Matrix ECC 200 encoder (ISO/IEC 16022:2006).
- ASCII encoding with digit-pair compression (two decimal digits per codeword),
  UPPER_SHIFT for extended ASCII (bytes > 127), and scrambled pad codewords
  using the ISO §5.2.3 randomisation formula.
- Self-contained GF(256) arithmetic over primitive polynomial 0x12D (Data
  Matrix's field; distinct from QR Code's 0x11D). Log/exp table construction
  in the static constructor; generator polynomial cache keyed by ECC length.
- Reed-Solomon encoding with b=1 root convention (roots α¹…αⁿ), using an
  LFSR-style division loop over the precomputed generator polynomial.
- Block interleaving: data codewords round-robin across blocks, then ECC
  codewords round-robin, matching ISO §5.2.6.
- All 30 symbol sizes: 24 square (10×10 through 144×144) and 6 rectangular
  (8×18 through 16×48).
- Utah diagonal placement algorithm with four corner patterns for boundary
  handling. No masking step required.
- Grid initialisation: L-shaped finder bar, alternating timing clock borders
  on top and right edges, and alignment borders for multi-region symbols
  (32×32 and above).
- Logical-to-physical coordinate mapping for multi-region symbols with
  correct per-region offset computation.
- `DataMatrixSymbolShape` enum (Square, Rectangular, Any) and `DataMatrixOptions`
  record for shape selection.
- `DataMatrixInputTooLongException` with `EncodedCW` and `MaxCW` properties.
- 66 unit tests covering GF arithmetic, ASCII encoding, pad codewords,
  RS block encoding, symbol selection, grid border structure, Utah placement
  integration, determinism, binary payloads, rectangular symbols, and module
  grid properties.
