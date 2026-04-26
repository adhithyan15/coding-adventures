# Changelog — micro-qr (Python)

All notable changes to this package will be documented in this file.

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of Micro QR Code encoder (ISO/IEC 18004:2015 Annex E).
- `encode(input_str, version=None, ecc=None)` — auto-selects smallest M1–M4
  symbol; accepts optional version/ECC overrides.
- `encode_at(input_str, version, ecc)` — encode at a specific symbol version
  and ECC level.
- `layout_grid(grid, config=None)` — convert `ModuleGrid` to `PaintScene` via
  `barcode-2d`, defaulting to 2-module quiet zone.
- `encode_and_layout(input_str, ecc=None, config=None)` — convenience wrapper
  combining encode + layout in one call.
- `compute_format_word(symbol_indicator, mask_pattern)` — utility to recompute
  BCH-protected 15-bit format words from first principles (tests + docs).
- `grid_to_string(grid)` — render `ModuleGrid` as `"0"`/`"1"` string for
  debugging and cross-language snapshot comparison.
- Full ECC support: M1/Detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q.
- Three encoding modes: numeric (max 35 chars in M4-L), alphanumeric (max 21),
  byte/UTF-8 (max 15).
- Reed-Solomon ECC using `gf256` package: GF(256)/0x11D, b=0 convention,
  generator polynomials for all Micro QR block sizes (2, 5, 6, 8, 10, 14).
- Mask evaluation: all 4 Micro QR mask patterns scored via the 4-rule QR
  penalty function; lowest-penalty mask selected.
- Format information: 15-bit BCH-protected word, XOR-masked with 0x4445.
- Pre-computed `_FORMAT_TABLE` for fast format word lookup (all 32 combinations).
- Literate inline documentation throughout the source.
- `MicroQRVersion` and `MicroQREccLevel` classes with string constants.
- Error hierarchy: `MicroQRError`, `InputTooLongError`, `ECCNotAvailableError`,
  `UnsupportedModeError`, `InvalidCharacterError`.
- Comprehensive test suite: 50+ tests covering dimensions, structural modules,
  auto-selection, capacity boundaries, error handling, determinism,
  cross-language corpus, format information, and RS ECC.
- `pyproject.toml` with `hatchling` build backend, `ruff`, `mypy`, `pytest-cov`
  (≥90% coverage required).
