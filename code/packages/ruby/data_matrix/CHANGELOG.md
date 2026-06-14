# Changelog — coding_adventures_data_matrix

All notable changes to this package will be documented in this file.
This project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-04-26

### Added

- **Core encoder** (`lib/coding_adventures/data_matrix.rb`):
  - `encode(data, opts = {})` — encodes a string to a `ModuleGrid`.
    Supports `size: [rows, cols]` to force a specific symbol and
    `shape: :square | :rectangle | :any` to control shape selection.
  - `encode_and_layout(data, opts = {})` — convenience wrapper returning
    `{ grid:, scene: nil }` (scene support planned for v0.2.0).
  - `grid_to_string(grid)` — debug/test rendering as '0'/'1' multiline string.
  - Full ISO/IEC 16022:2006 ECC200 pipeline:
    - ASCII encoding with digit-pair compaction (two digits → one codeword).
    - ECC200 scrambled-pad codewords.
    - GF(256)/0x12D Reed-Solomon encoding, b=1 convention.
    - Per-block interleaving for burst-error resilience.
    - L-finder (left column + bottom row all dark) initialization.
    - Timing clock borders (top row + right column alternating).
    - Alignment borders for multi-region symbols.
    - Utah diagonal placement algorithm with four corner special patterns.
    - ISO §10 fill rule for unvisited modules.
  - All 30 ECC200 symbol sizes: 24 square (10×10 … 144×144) and
    6 rectangular (8×18 … 16×48).

- **Error hierarchy** (`lib/coding_adventures/data_matrix/errors.rb`):
  - `DataMatrixError < StandardError` — base class.
  - `InputTooLongError < DataMatrixError` — input exceeds 144×144 capacity.
  - `InvalidSizeError < DataMatrixError` — forced size does not match ECC200.

- **GF(256)/0x12D arithmetic**:
  - Precomputed `GF_EXP` and `GF_LOG` tables (256 entries each) built at
    module load time.
  - `gf_mul(a, b)` — O(1) multiply via log/antilog tables.
  - `build_generator(n_ecc)` — RS generator polynomial (b=1 convention),
    cached in `GEN_CACHE`.

- **Test suite** (`spec/data_matrix_spec.rb`):
  - 70+ examples covering VERSION, error hierarchy, ModuleGrid struct,
    symbol size tables, GF arithmetic, RS encoding, ASCII encoding,
    padding, symbol selection, wrap rules, Utah placement internals,
    `encode`, `encode_and_layout`, and `grid_to_string`.
  - L-finder validation (bottom row all dark, left column all dark).
  - Timing clock validation (top row alternating, right column alternating).
  - Determinism, error-on-overflow, shape selection, explicit size option.

- **Package files**: `Gemfile`, `Rakefile`, `BUILD`, `README.md`,
  `CHANGELOG.md`, `coding_adventures_data_matrix.gemspec`.

### Notes

- `encode_and_layout` returns `scene: nil` in v0.1.0 because this gem has
  no dependency on `coding_adventures_paint_instructions`. Full scene support
  will be added in v0.2.0 when the paint-instructions integration is complete.

[0.1.0]: https://github.com/adhithyan15/coding-adventures/tree/main/code/packages/ruby/data_matrix
