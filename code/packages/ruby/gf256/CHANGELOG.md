# Changelog — coding_adventures_gf256 (Ruby)

## [0.1.0] — 2026-04-03

### Added

- Initial implementation of GF(2^8) with primitive polynomial 0x11D.
- Module-level LOG (256-element frozen array) and ALOG (255-element frozen array).
- `add` / `subtract` — XOR (identical in characteristic 2).
- `multiply` — via LOG/ALOG tables; zero special-case handled.
- `divide` — via LOG/ALOG; raises ArgumentError for b=0.
- `power` — exponentiation via LOG/ALOG; 0^0=1, 0^n=0.
- `inverse` — multiplicative inverse; raises ArgumentError for a=0.
- `zero` / `one` — field identity elements.
- `log_table` / `alog_table` — accessor methods for testing and downstream use.
- Minitest test suite with full table consistency, spot checks, and all-x tests.
