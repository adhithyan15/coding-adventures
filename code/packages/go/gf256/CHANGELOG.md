# Changelog — go/gf256

## [0.2.0] — 2026-04-11

### Added

- `Field` struct and `NewField(primitivePoly int) *Field` constructor — parameterizable
  field factory that accepts any primitive polynomial. Uses Russian peasant
  (shift-and-XOR) multiplication; no log/antilog tables stored.
  - `NewField(0x11B)` creates the AES GF(2^8) field.
  - `NewField(0x11D)` matches the module-level functions (Reed-Solomon).
  - Methods: `Multiply`, `Divide`, `Power`, `Inverse`, `Add`, `Subtract`.
  - `PrimitivePoly` field stores the polynomial used for construction.
- Tests for `Field`: AES sanity check (`0x53 × 0xCA = 1`), FIPS 197 Appendix B
  (`0x57 × 0x83 = 0xC1`), RS backward-compat, commutativity, panic cases.

## [0.1.0] — 2026-04-03

### Added

- Initial implementation of GF(2^8) with primitive polynomial 0x11D.
- `init()` builds LOG (256-byte array) and ALOG (256-int array) tables.
- `Add` / `Subtract` — XOR (identical in characteristic 2).
- `Multiply` — via LOG/ALOG tables; zero special-case handled.
- `Divide` — via LOG/ALOG; panics on division by zero.
- `Power` — exponentiation; handles 0^0=1, 0^n=0.
- `Inverse` — multiplicative inverse; panics for a=0.
- `Zero` / `One` — field identity elements.
- `LOG()` / `ALOG()` — table accessors for testing and downstream use.
- Comprehensive test suite: table consistency, spot checks, all-x inverse.
