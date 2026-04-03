# Changelog — go/gf256

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
