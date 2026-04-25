# Changelog

All notable changes to `coding_adventures_micro_qr` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-24

### Added

- Initial release: full Micro QR Code encoder conforming to ISO/IEC 18004:2015 Annex E.

#### Encoding

- `encode()` — auto-selects the smallest M1–M4 symbol and encoding mode.
- `encodeAt()` — encodes to a user-specified version and ECC level.
- `encodeAndLayout()` — convenience: encode + barcode-2d layout in one call.
- `layoutGrid()` — wraps barcode-2d's `layout()` with Micro QR defaults
  (2-module quiet zone instead of QR's 4-module zone).

#### Types

- `MicroQRVersion` enum: `m1`, `m2`, `m3`, `m4`.
- `MicroQREccLevel` enum: `detection`, `l`, `m`, `q`.
- `MicroQRError` base class; subtypes: `InputTooLong`, `UnsupportedMode`,
  `ECCNotAvailable`, `InvalidCharacter`.

#### Encoding modes

- Numeric mode (all symbols): groups of 3 digits → 10 bits, pair → 7 bits, single → 4 bits.
- Alphanumeric mode (M2–M4): 45-character set, pairs → 11 bits, single → 6 bits.
- Byte mode (M3–M4): raw UTF-8 bytes, each byte → 8 bits.
- Kanji mode not implemented (future extension for M4).

#### Symbol structure

- Finder pattern: 7×7 at top-left (rows 0–6, cols 0–6).
- L-shaped separator: row 7 cols 0–7, col 7 rows 0–7.
- Timing patterns: row 0 and col 0 from position 8 to symbol edge (alternating dark/light).
- No alignment patterns (Micro QR is too small for distortion correction).
- Format information: 15-bit word in L-shape at row 8 / col 8 (single copy only).

#### Error correction

- Reed-Solomon over GF(256)/0x11D with b=0 convention (identical to regular QR).
- Single-block RS (no interleaving); generator polynomials for 2, 5, 6, 8, 10, 14 ECC bytes.
- Format information: 15-bit BCH code, XOR-masked with 0x4445 (not QR's 0x5412).

#### Masking

- Four mask patterns (subset of regular QR's eight).
- Penalty scoring: all four rules from ISO 18004 (runs, 2×2 blocks, finder-like sequences,
  dark proportion).
- Mask selection: lowest-penalty mask wins; ties broken by lower mask index.

#### Tests

- 17 test groups covering dimensions, auto-selection, structural modules, format info,
  capacity boundaries, ECC constraints, encoding modes, error handling, determinism,
  cross-language corpus, layout functions, and structural invariants.

### Dependencies

- `coding_adventures_barcode_2d` (path: `../barcode-2d`) — `ModuleGrid`, `layout()`.
- `coding_adventures_gf256` (path: `../gf256`) — `gfMultiply()` for RS encoding.
