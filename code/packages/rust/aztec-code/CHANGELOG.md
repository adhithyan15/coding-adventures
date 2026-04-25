# Changelog — aztec-code (Rust)

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-24

### Added

- Initial release of the `aztec-code` Rust crate.
- `encode(data: &[u8], options: Option<AztecOptions>) -> Result<ModuleGrid, AztecError>` — encodes raw bytes into a `ModuleGrid`.
- `encode_str(s: &str, options: Option<AztecOptions>) -> Result<ModuleGrid, AztecError>` — UTF-8 convenience wrapper over `encode`.
- `encode_and_layout(data: &[u8], options: Option<AztecOptions>, config: Option<Barcode2DLayoutConfig>) -> Result<PaintScene, AztecError>` — encode then layout in one call.
- `AztecError` and `AztecError::InputTooLong` error variants.
- `AztecOptions` struct with `min_ecc_percent` field (default 23).
- Full ISO/IEC 24778:2008 compliant bullseye finder pattern (compact: 11×11, full: 15×15).
- Correct orientation marks (4 dark corners of the mode message ring).
- GF(16)/0x13 Reed-Solomon for mode message (compact: (7,2) code; full: (10,4) code).
- GF(256)/0x12D Reed-Solomon for 8-bit data codewords (same polynomial as Data Matrix).
- Bit stuffing algorithm (insert complement after every 4 consecutive identical bits).
- Data layer clockwise spiral placement (innermost layer outward).
- Reference grid support for full symbols (alternating dark/light at 16-module intervals).
- Auto-selection of compact (1–4 layers) vs full (1–32 layers) based on input size.
- 80+ unit tests verifying bullseye structure, orientation marks, symbol sizes, error handling.

### Implementation notes

- v0.1.0 uses byte mode only (Binary-Shift from Upper mode for all input).
  Multi-mode optimization (Digit, Upper, Lower, Mixed, Punct) is planned for v0.2.0.
- GF(256) RS tables use `std::sync::OnceLock` for one-time lazy initialization.
- GF(256) uses polynomial 0x12D (Aztec/Data Matrix), NOT 0x11D (QR Code).
- Capacity tables are embedded as lookup arrays derived from ISO/IEC 24778:2008 Table 1.
