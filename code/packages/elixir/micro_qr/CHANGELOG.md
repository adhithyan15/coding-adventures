# Changelog — coding_adventures_micro_qr (Elixir)

All notable changes to this package are documented here.

---

## [0.1.0] — 2026-04-24

### Added

- `CodingAdventures.MicroQR` module — ISO/IEC 18004:2015 Annex E compliant
  Micro QR Code encoder for Elixir.

- `encode/3` — encodes a string to a `ModuleGrid`. Auto-selects the smallest
  symbol (M1..M4) and encoding mode (numeric > alphanumeric > byte) that fits
  the input. Optional `version` and `ecc` atoms override the selection.
  Returns `{:ok, %ModuleGrid{}}` or `{:error, reason}`.

- `encode!/3` — bang variant of `encode/3`. Raises `RuntimeError` on failure.

- `layout_grid/2` — converts a `ModuleGrid` to a `PaintScene` using the
  `barcode_2d` layout function with a 2-module quiet zone (Micro QR minimum,
  half of standard QR's 4-module requirement).

- `encode_and_layout/4` — convenience wrapper combining `encode/3` and
  `layout_grid/2` in one step.

- `version/0` — returns the package version string `"0.1.0"`.

- Full symbol configuration table for all 8 valid (version, ECC) combinations:
  M1/detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q. Each entry carries
  data/ECC codeword counts, capacities, terminator widths, mode indicator
  widths, and character count field widths.

- Reed-Solomon encoder over GF(256)/0x11D with b=0 convention. Generator
  polynomials for all required ECC counts: {2, 5, 6, 8, 10, 14}.

- Pre-computed format information table (32 entries — 8 symbols × 4 masks),
  XOR-masked with the Micro QR constant `0x4445`.

- Grid construction pipeline:
  - 7×7 finder pattern at top-left corner.
  - L-shaped separator (light modules at row 7 cols 0–7 and col 7 rows 0–7).
  - Timing pattern extensions along row 0 and col 0 (positions 8 onward).
  - Format information reservation and final write (15 bits, single copy).

- Two-column zigzag data placement from bottom-right corner, skipping reserved
  modules. No col-6 skip needed (Micro QR timing is at col 0, already reserved).

- All 4 Micro QR mask patterns with penalty evaluation (QR Code 4-rule scoring).
  Best mask is selected by lowest penalty.

- Special M1 half-codeword handling: last data codeword contributes only its
  upper 4 bits to the bit stream (20-bit total data capacity).

- 58 ExUnit tests with 96.76% coverage, covering:
  - Grid dimensions for all 4 versions.
  - Auto-version selection for all mode types.
  - Explicit numeric, alphanumeric, and byte encoding in M2–M4.
  - All 8 ECC combinations and invalid ECC requests.
  - Structural module placement (finder, separator, timing).
  - `encode!` bang variant and `RuntimeError` on failure.
  - `layout_grid` quiet zone and instruction count.
  - `encode_and_layout` convenience wrapper.
  - Determinism and different-input-different-grid assertions.
  - Cross-language reference corpus (6 test vectors).
