# Changelog

## 0.1.0 — 2026-05-06

### Added

- Initial implementation of Micro QR Code encoder (ISO/IEC 18004:2015 Annex E).
- M1–M4 symbol versions (11×11 through 17×17 modules).
- Three encoding modes: numeric, alphanumeric, byte.
- Reed-Solomon ECC (GF(256)/0x11D, b=0 convention, single block per symbol).
- Embedded RS generator polynomials for ECC codeword counts 2, 5, 6, 8, 10, 14.
- Four mask patterns (Micro QR subset of regular QR's 8) with penalty evaluation.
- Four penalty rules (adjacent runs, 2×2 blocks, finder-like sequences, dark proportion).
- Pre-computed format information table (all 32 values: 8 symbol+ECC × 4 masks).
- Two-column zigzag data placement, scanning from bottom-right corner.
- Automatic version and ECC selection (smallest symbol that fits the input).
- Forced version/ECC via `options.version` and `options.ecc` parameters.
- `M.encode(input, options?)` public API → `{rows, cols, modules, module_shape, version, ecc}`.
- `M.VERSION` = `"0.1.0"`.
- 68 passing unit tests (busted) covering all encoding modes, symbol sizes, structural
  patterns, cross-language corpus inputs, boundary conditions, and error handling.
- Self-contained: inline GF(256)/0x11D arithmetic, no external Lua package dependencies.
