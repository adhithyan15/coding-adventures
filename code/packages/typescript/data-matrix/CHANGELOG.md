# Changelog — @coding-adventures/data-matrix

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of Data Matrix ECC200 encoder (ISO/IEC 16022:2006).
- GF(256) arithmetic over primitive polynomial 0x12D (Data Matrix field,
  distinct from QR Code's 0x11D).  Log/antilog tables pre-computed at module
  load time.
- ASCII encoding mode with greedy digit-pair packing: two consecutive digits
  encoded as one codeword (130 + value), halving the codeword budget for
  all-numeric input.  Extended ASCII (128-255) supported via UPPER_SHIFT.
- Scrambled pad codewords (ISO/IEC 16022 §5.2.3) to prevent degenerate
  placement patterns in unused symbol capacity.
- Reed-Solomon ECC using b=1 generator polynomial convention, over GF(256)/0x12D.
  Generator polynomials for all required ECC lengths computed at module load
  time and cached.
- Multi-block RS with round-robin interleaving: data codewords and ECC
  codewords each interleaved across all blocks for burst-error resilience.
- Symbol size selection: 24 square sizes (10×10 through 144×144) and 6
  rectangular sizes (8×18 through 16×48).  Smallest fitting symbol chosen
  automatically.  `InputTooLongError` raised if input exceeds 144×144 capacity.
- L-finder + timing clock border: left column and bottom row all dark (L-bar),
  top row and right column alternating dark/light (clock).  Outer border
  precedence: L-finder writes last to override alignment border and timing
  values at corner intersections.
- Alignment borders for multi-region symbols (32×32 and larger): solid-dark
  bar adjacent to alternating dark/light bar, placed between data regions.
- Utah diagonal placement algorithm: diagonal codeword traversal with all
  four corner patterns (ISO/IEC 16022 Annex F).  Fill pattern for residual
  modules.  No masking step (Data Matrix never masks).
- Logical-to-physical coordinate mapping for multi-region symbols.
- Public API: `encode()`, `encodeAndLayout()`, `renderSvg()`, `explain()`.
- `DataMatrixOptions` type with `shape` and `mode` fields.
- 69 unit and integration tests covering GF arithmetic, ASCII encoding,
  pad codewords, RS encoding, Utah placement, symbol border structure,
  multi-region alignment borders, the full pipeline, and the cross-language
  test corpus.  Coverage: 93.72% statements, 95.65% branches, 96% functions.
