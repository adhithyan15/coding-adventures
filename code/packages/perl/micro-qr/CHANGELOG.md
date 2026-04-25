# Changelog — CodingAdventures::MicroQR

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of the Micro QR Code encoder (ISO/IEC 18004:2015 Annex E).
- `encode($input, $version, $ecc)` — encodes a string to a `ModuleGrid` hashref.
  Auto-selects the smallest fitting symbol (M1–M4) and encoding mode when
  `$version` and `$ecc` are omitted.
- `encode_at($input, $version, $ecc)` — wrapper that requires both `$version`
  and `$ecc` to be specified explicitly.
- `layout_grid($grid, $config)` — converts a `ModuleGrid` to a `PaintScene`
  via `CodingAdventures::Barcode2D::layout()`, defaulting to the 2-module
  Micro QR quiet zone.
- All four symbol sizes: M1 (11×11), M2 (13×13), M3 (15×15), M4 (17×17).
- All available ECC levels: Detection (M1 only), L, M, Q.
- Three encoding modes: numeric, alphanumeric (45-char set), byte (raw bytes).
- Reed-Solomon ECC using GF(256)/0x11D with b=0 convention, single block
  (no interleaving), via `CodingAdventures::GF256::multiply`.
- Hard-coded RS generator polynomials for the six ECC codeword counts used by
  Micro QR (2, 5, 6, 8, 10, 14).
- Pre-computed 32-entry format information table (symbol_indicator × mask_pattern)
  XOR-masked with 0x4445 (Micro QR specific, not 0x5412).
- 4 mask patterns (vs. regular QR's 8); lowest 4-rule penalty wins.
- Penalty evaluation includes all four ISO 18004 rules:
  runs, 2×2 blocks, finder-like sequences, dark-ratio deviation.
- M1 special handling: 2.5-codeword data capacity (20 bits), half-codeword in
  upper nibble of third byte.
- Finder pattern (7×7, top-left only), L-shaped separator, timing at row 0 /
  col 0 (not row 6 / col 6 as in regular QR).
- Single-copy format information strip (row 8 cols 1-8, col 8 rows 1-7).
- 58 unit tests covering: dimensions, auto-selection, finder/separator/timing
  structure, format info, ECC constraints, modes, capacity bounds, error
  conditions, determinism, and cross-language corpus.
- `BUILD` file for the repository build tool.
- `Makefile.PL` for CPAN-style distribution.
- `cpanfile` for CPAN dependency declaration.
- `README.md` with usage examples, encoding pipeline, and architecture notes.
