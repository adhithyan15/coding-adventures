# Changelog — aztec-code (Rust)

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of the Aztec Code encoder (ISO/IEC 24778:2008).
- Compact Aztec support: 1–4 layers, 15×15 to 27×27 symbols.
- Full Aztec support: 1–32 layers, 19×19 to 143×143 symbols.
- Bullseye finder pattern:
  - d ≤ 1: solid 3×3 dark inner core.
  - d ≥ 2: alternating light (even) and dark (odd) rings.
  - All bullseye modules marked as reserved.
- Orientation marks: 4 always-dark corners of the mode message ring.
- Mode message encoding:
  - Compact: 28-bit (7 nibbles, GF(16)/0x13 RS, 2 data + 5 ECC nibbles).
  - Full: 40-bit (10 nibbles, GF(16)/0x13 RS, 4 data + 6 ECC nibbles).
- GF(16)/0x13 Reed-Solomon: inline implementation, `build_gf16_generator`,
  `gf16_rs_encode`.
- GF(256)/0x12D Reed-Solomon: inline with precomputed log/antilog tables
  via `std::sync::OnceLock`, b=1 convention (same as Data Matrix ECC200).
- Bit stuffing: after 4 consecutive identical bits, insert complement bit.
- Clockwise spiral data placement (2-module-wide bands, outer-before-inner).
- Reference grid for full symbols: centre row/col + ±16n lines; skips
  already-reserved modules.
- Auto-selection of compact vs full based on data length and ECC percentage.
- Default ECC: 23% of total codewords.
- `encode(input: &[u8], options: Option<&AztecOptions>) → Result<ModuleGrid, AztecError>`
- `encode_str(input: &str, ...) → Result<ModuleGrid, AztecError>` — convenience wrapper.
- `encode_and_layout(...)` → delegates to `barcode_2d::layout()`.
- `AztecOptions { min_ecc_percent, compact }` — optional configuration.
- 40 unit tests with comprehensive coverage:
  - GF(16) log/antilog tables and period-15 property.
  - GF(256)/0x12D table correctness.
  - Mode message bit lengths and determinism.
  - Bit stuffing edge cases (runs of 4, 8; alternating; empty).
  - Symbol sizing and compact/full selection.
  - Bullseye Chebyshev-distance verification for all ring radii.
  - Orientation marks always dark.
  - Integration tests: "A", "Hello World", URLs, raw binary, digit-heavy.
  - Error propagation: InputTooLong, compact-forced-error.

### v0.1.0 simplifications (noted for v0.2.0)

- Byte mode only (via Binary-Shift from Upper mode). Multi-mode optimization
  (Digit/Upper/Lower/Mixed/Punct) is planned for v0.2.0.
- GF(256)/0x12D is implemented inline because the shared `gf256` crate uses
  polynomial 0x11D (QR Code), which is incompatible with Aztec Code / Data Matrix.
