# Changelog

All notable changes to `coding_adventures_data_matrix` are documented here.

## [0.1.0] - 2026-04-24

### Added

- Initial release of the Elixir Data Matrix ECC200 encoder.
- GF(256) arithmetic over primitive polynomial 0x12D (Data Matrix field),
  distinct from QR Code's 0x11D polynomial.
- ASCII encoding with digit-pair optimization: two consecutive ASCII digits
  are packed into one codeword (range 130–229), cutting codeword count by
  50% for purely numeric input.
- Scrambled pad codewords (ISO/IEC 16022 §5.2.3) to fill unused symbol
  capacity without creating degenerate placement patterns.
- Reed-Solomon ECC generation with b=1 convention (α^1..α^n roots), exactly
  matching the Data Matrix / ISO/IEC 16022 specification.
- Multi-block RS interleaving: data and ECC codewords from all blocks are
  interleaved round-robin in the final codeword stream.
- Symbol size selection: 24 square sizes (10×10 to 144×144) and 6 rectangular
  sizes (8×18 to 16×48). Selects the smallest symbol whose data capacity
  fits the encoded codeword count.
- L-finder + timing border initialization: solid-dark left column and bottom
  row (finder), alternating top row and right column (timing).
- Alignment border initialization for multi-region symbols (each 2 modules
  wide: one all-dark bar + one alternating bar).
- Utah diagonal placement algorithm: the 8-bit codeword placement scheme
  named for the US state's outline shape. Includes all four corner patterns
  and the residual fill rule (dark at (r+c) mod 2 == 1).
- Logical-to-physical coordinate mapping for multi-region symbols.
- `encode/2` public API returning `{:ok, %{rows, cols, modules}}` grid.
- `encode!/2` raising variant.
- `render_ascii/2` for text-mode symbol visualization (debugging).
- 60+ unit and integration tests covering GF arithmetic, ASCII encoding,
  pad scrambling, RS systematic property, symbol selection, border patterns,
  Utah placement, multi-region symbols, and full encode pipeline.

### Notes

- This is the Elixir port of the reference Rust implementation at
  `code/packages/rust/data-matrix/`.
- No masking is applied — Data Matrix ECC200 does not use masking.
- C40, Text, X12, EDIFACT, and Base256 modes are planned for v0.2.0.
