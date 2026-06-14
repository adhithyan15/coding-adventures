# Changelog — coding_adventures_aztec_code

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-26

### Added

- Initial implementation of the Aztec Code encoder (ISO/IEC 24778:2008).
- `CodingAdventures::AztecCode.encode(data, min_ecc_percent:)` — encode a
  String or byte Array to a `ModuleGrid`.  Auto-selects the smallest Compact
  (1–4 layers) or Full (1–32 layers) symbol that fits at the requested ECC.
- `CodingAdventures::AztecCode.layout(grid, config)` — convert a `ModuleGrid`
  to a `PaintScene` via `barcode_2d` (defaults to a 2-module quiet zone).
- `CodingAdventures::AztecCode.encode_and_layout(data, …)` — convenience
  helper combining encode + layout.
- Compact symbol sizes 15×15, 19×19, 23×23, 27×27 (1–4 layers).
- Full symbol sizes 19×19 through 143×143 (1–32 layers).
- Central bullseye finder pattern (5-radius compact / 7-radius full).
- Reference grid for Full symbols (modules at every offset that is a multiple
  of 16 from the centre).
- 4 dark orientation marks at the bullseye+1 ring corners.
- Mode-message ring with GF(16) Reed-Solomon protection (5 ECC nibbles in
  Compact, 6 ECC nibbles in Full; primitive polynomial x^4 + x + 1 = 0x13).
- Data placement via clockwise layer spiral, two bits per step.
- GF(256)/0x12D Reed-Solomon over 8-bit codewords with b=1 root convention
  (same primitive polynomial as Data Matrix ECC200, NOT the QR/0x11D one).
- Aztec bit-stuffing: insert one complement bit after every run of 4
  identical bits; rescues an all-zero last codeword as 0xFF.
- Binary-Shift encoding from the Upper-mode start state (5-bit length when
  ≤ 31 bytes, 5+11-bit length otherwise).
- `AztecError` (base) + `InputTooLong` error classes.
- Comprehensive RSpec test suite covering version constants, GF(16)/GF(256)
  arithmetic, generator polynomials, bit stuffing, padding, mode message,
  symbol selection, geometry, structural patterns, end-to-end encode, and
  PaintScene layout — targeting ≥ 90 % line coverage via SimpleCov.

### Known limitations (planned for v0.2.0)

- Byte-mode only (Binary-Shift).  Multi-mode optimisation
  (Upper/Lower/Mixed/Punct/Digit state machine) not yet implemented.
- 8-bit codewords only.  GF(16)/GF(32) RS for 4-bit/5-bit codewords (which
  enable maximum-density very small symbols) not yet implemented.
- No `force_compact:` or explicit-layer override; auto-selection only.
