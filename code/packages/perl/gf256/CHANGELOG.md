# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] — 2026-04-11

### Added

- `CodingAdventures::GF256::Field` class — parameterizable field factory that accepts
  any primitive polynomial and builds independent LOG/ALOG tables.
  - `CodingAdventures::GF256::Field->new(0x11B)` creates the AES GF(2^8) field.
  - `CodingAdventures::GF256::Field->new(0x11D)` matches module-level functions.
  - Methods: `multiply`, `divide`, `power`, `inverse`, `add`, `subtract`.
  - `polynomial` accessor stores the polynomial used for construction.
- Tests: AES sanity check (`0x53 × 0x8C = 1`), FIPS 197 Appendix B
  (`0x57 × 0x83 = 0xC1`), RS backward-compat, commutativity, error cases.

## [0.1.0] - 2026-04-03

### Added

- Initial implementation of `CodingAdventures::GF256`
- Functions: `add`, `subtract`, `multiply`, `divide`, `power`, `inverse`
- LOG and ALOG lookup tables built at module load time using the primitive
  polynomial x^8 + x^4 + x^3 + x + 1 (0x11D)
- Generator g = 2 (the polynomial x) used for table construction
- Knuth-style literate comments explaining finite fields, GF(2^8), and
  the LOG/ALOG optimization for O(1) multiplication
- Test suite with 40+ tests using Test2::V0, including field axiom checks
  and verification against AES known-good values (0x53 * 0xCA = 1)
