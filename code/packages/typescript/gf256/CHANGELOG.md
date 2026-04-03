# Changelog — @coding-adventures/gf256

## [0.1.0] — 2026-04-03

### Added

- Initial implementation of GF(2^8) arithmetic with primitive polynomial 0x11D.
- Module-level LOG (256 entries) and ALOG (255 entries) table construction
  using generator g=2 with bit-shift and XOR reduction.
- `add(a, b)` / `subtract(a, b)` — XOR (identical in characteristic 2).
- `multiply(a, b)` — via LOG/ALOG tables; O(1) with zero special-case.
- `divide(a, b)` — via LOG/ALOG tables; throws on division by zero.
- `power(base, exp)` — exponentiation via LOG/ALOG; handles 0^0=1 and 0^n=0.
- `inverse(a)` — multiplicative inverse via ALOG[255-LOG[a]]; throws for a=0.
- `zero()` / `one()` — field identity elements.
- Exports `LOG`, `ALOG`, `PRIMITIVE_POLYNOMIAL`, `ZERO`, `ONE` constants.
- Comprehensive test suite including:
  - Full table consistency checks (ALOG[LOG[x]] = x for all x in 1..255)
  - Known spot check: 0x53 × 0xCA = 0x01
  - Generator order: g^255 = 1
  - All-x inverse check: x × inverse(x) = 1 for all x in 1..255
