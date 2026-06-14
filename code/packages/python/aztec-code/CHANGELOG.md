# Changelog — aztec-code (Python)

All notable changes to this package will be documented in this file.

## [0.1.0] — 2026-04-26

### Added

- Initial implementation of the Aztec Code encoder (ISO/IEC 24778:2008).
- `encode(data, options=None)` — encodes a string or bytes payload into a
  `ModuleGrid`, auto-selecting the smallest compact (1–4 layers) or full
  (1–32 layers) symbol that satisfies the requested ECC level.
- `encode_and_layout(data, options=None, config=None)` — convenience wrapper
  combining `encode` and `barcode_2d.layout` to return a `PaintScene`.
- `layout_grid(grid, config=None)` — thin re-export of `barcode_2d.layout`
  for callers that already have a `ModuleGrid`.
- `explain(data, options=None)` — returns an `AnnotatedModuleGrid` (per-module
  annotations are stubbed in v0.1.0; populated in v0.2.0).
- `AztecOptions(min_ecc_percent=23)` — frozen dataclass for encoder options.
- Error hierarchy: `AztecError`, `InputTooLongError`.
- Byte-mode (Binary-Shift from Upper) encoding path with short-form length
  (1–31 bytes, 5-bit count) and long-form length (32+ bytes, 5+11 bits).
- GF(256)/0x12D Reed-Solomon encoder for 8-bit data codewords (the same
  primitive polynomial Data Matrix uses, distinct from QR's 0x11D).
- GF(16)/0x13 Reed-Solomon encoder for the mode message.
- Bit-stuffing rule: insert a complement bit after every run of 4 identical
  bits in the data+ECC stream.
- All-zero last-codeword avoidance (substitute 0xFF) per §7.3.1.1.
- Bullseye finder pattern, orientation-mark corners, and reference grid
  drawing helpers.
- Clockwise layer-spiral data placement (mode-ring leftovers first, then
  layer-by-layer outward).
- `pyproject.toml` with `hatchling` build backend, `ruff`, `mypy`, and
  `pytest-cov` (≥90% coverage required).
- Comprehensive test suite mirroring the TypeScript reference: symbol size
  selection (compact 1–4 + full), bullseye structure (compact + full),
  orientation marks, ECC level option, determinism, Unicode handling, large
  payloads up to 500 bytes, and error cases.
- Literate inline documentation throughout the source.

### Notes

This v0.1.0 mirrors the TypeScript `@coding-adventures/aztec-code` v0.1.0
implementation feature-for-feature. Future work tracked for v0.2.0:

1. Multi-mode (Digit/Upper/Lower/Mixed/Punct) encoding optimisation.
2. GF(16) and GF(32) RS for 4-bit/5-bit codewords on smaller symbols.
3. `force_compact` / `force_layers` options.
4. Populated `explain()` annotations.
