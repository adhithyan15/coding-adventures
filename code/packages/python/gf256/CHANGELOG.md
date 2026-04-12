# Changelog — coding-adventures-gf256

## [0.2.0] — 2026-04-11

### Added

- `GF256Field(polynomial)` class — parameterizable field factory that accepts any
  primitive polynomial. Uses Russian peasant (shift-and-XOR) multiplication —
  works correctly for any irreducible polynomial regardless of which element is a
  primitive generator. (Log/antilog tables with g=2 fail for `0x11B` because x is
  not a primitive element of the AES field; AES uses g=0x03 per FIPS 197 §4.1.)
  - `GF256Field(0x11B)` creates the AES GF(2^8) field.
  - `GF256Field(0x11D)` matches the module-level functions (Reed-Solomon).
  - Methods: `multiply`, `divide`, `power`, `inverse`, `add`, `subtract`.
  - `polynomial` attribute stores the polynomial used for construction.
- Tests for `GF256Field`: AES field sanity check (`0x53 × 0xCA = 1`), FIPS 197
  Appendix B (`0x57 × 0x83 = 0xC1`), RS backward-compat, commutativity, error cases.

## [0.1.0] — 2026-04-03

### Added

- Initial implementation of GF(2^8) with primitive polynomial 0x11D.
- Module-level LOG (256-entry tuple) and ALOG (255-entry tuple) tables.
- `add(a, b)` / `subtract(a, b)` — XOR (identical in characteristic 2).
- `multiply(a, b)` — via LOG/ALOG tables; zero special-case handled.
- `divide(a, b)` — via LOG/ALOG; raises ValueError for b=0.
- `power(base, exp)` — exponentiation via LOG/ALOG; 0^0=1, 0^n=0.
- `inverse(a)` — multiplicative inverse; raises ValueError for a=0.
- `zero()` / `one()` — field identity elements.
- Exports LOG, ALOG, PRIMITIVE_POLYNOMIAL, ZERO, ONE constants.
- Comprehensive test suite with full table consistency checks and spot checks.
