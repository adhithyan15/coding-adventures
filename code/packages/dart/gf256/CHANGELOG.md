# Changelog — coding_adventures_gf256

## 0.1.0 — 2026-04-24

### Added

- Initial release: GF(2^8) field arithmetic using the Reed-Solomon primitive
  polynomial `p(x) = x^8 + x^4 + x^3 + x^2 + 1 = 0x11D`.
- `gfAdd` / `gfSubtract` — XOR (identical operations in characteristic 2).
- `gfMultiply` — log/antilog table lookup, O(1).
- `gfDivide` — quotient using log tables; throws `ArgumentError` for b = 0.
- `gfPower` — exponentiation using the log table; throws for negative exp.
- `gfInverse` — multiplicative inverse via `alog[255 - log[a]]`; throws for a = 0.
- `gfZero()` / `gfOne()` — additive and multiplicative identities.
- `alog` / `log` — exported lookup tables (lazily initialized on first use).
- 42 unit tests covering all operations, edge cases, algebraic properties,
  and the standard GF(256) spot check (0x53 × 0x8C = 0x01 for RS polynomial).
