# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.2.0] — 2026-04-11

### Added

- `GF256Field` struct — parameterizable field factory that accepts any primitive polynomial.
  Uses Russian peasant (shift-and-XOR) multiplication; no log/antilog tables stored.
  Initialization is O(1).
  - `GF256Field(polynomial: 0x11B)` creates the AES GF(2^8) field.
  - `GF256Field(polynomial: 0x11D)` matches the module-level `GF256` enum functions.
  - Methods: `multiply(_:_:)`, `divide(_:_:)`, `power(_:_:)`, `inverse(_:)`,
    `add(_:_:)`, `subtract(_:_:)`.
  - `polynomial: UInt16` property stores the polynomial used for construction.
- Tests: AES sanity check (`0x53 × 0xCA = 1`), FIPS 197 Appendix B
  (`0x57 × 0x83 = 0xC1`), RS backward-compat, commutativity, edge cases.

## [0.1.0] - 2026-04-03

### Added

- `GF256.zero` — The additive identity (UInt8 = 0).
- `GF256.one` — The multiplicative identity (UInt8 = 1).
- `GF256.primitivePoly` — The primitive polynomial 0x11D used for modular
  reduction (x^8 + x^4 + x^3 + x^2 + 1).
- `GF256.ALOG` — Read-only antilogarithm table: `ALOG[i]` = 2^i in GF(256).
  256 entries; `ALOG[255]` = 1 to support the full cyclic group of order 255.
- `GF256.LOG` — Read-only logarithm table: `LOG[x]` = i such that 2^i = x.
  `LOG[0]` is unused; valid for x in 1..255.
- `GF256.add(_:_:)` — XOR-based addition. O(1).
- `GF256.subtract(_:_:)` — Equal to addition in GF(2^8). O(1).
- `GF256.multiply(_:_:)` — Table-based multiplication using LOG/ALOG.
  O(1): two table lookups and one addition.
- `GF256.divide(_:_:)` — Table-based division; `precondition` guards against
  divide-by-zero.
- `GF256.power(_:_:)` — Exponentiation using logarithm tables; handles
  0^0 = 1, 0^n = 0, and Fermat's little theorem (a^255 = 1).
- `GF256.inverse(_:)` — Multiplicative inverse via `ALOG[255 - LOG[a]]`;
  `precondition` guards against inversion of zero.
- LOG/ALOG tables built once at module initialization via `buildTables()`.
  Uses `UInt16` for intermediate values during table construction to safely
  handle the 256-threshold overflow before XOR reduction.
- Comprehensive test suite with 40+ test cases covering all operations,
  table correctness, field axioms (distributivity, associativity,
  commutativity), Fermat's little theorem, and edge cases.
- Literate programming style with extensive inline documentation explaining
  GF(2^8) theory, the role of the primitive polynomial, and why XOR is
  addition in characteristic-2 fields.

### Design Notes

- All public functions are members of `public enum GF256` (namespace enum)
  to avoid Swift operator conflicts.
- Elements are typed as `UInt8` — naturally bounded to 0..255 with no
  overflow possible in the public API.
- `UInt16` is used internally only in `buildTables()` to detect the overflow
  during the repeated-doubling construction of ALOG.
