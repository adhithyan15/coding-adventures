# Changelog — gf256

All notable changes to this package are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## 0.2.0 — 2026-04-11

### Added

- `Field` struct and `Field::new(primitive_poly: u16)` constructor — parameterizable
  field factory that accepts any primitive polynomial. Uses Russian peasant
  (shift-and-XOR) multiplication; no log/antilog tables stored.
  - `Field::new(0x11B)` creates the AES GF(2^8) field.
  - `Field::new(0x11D)` matches the module-level functions (Reed-Solomon).
  - Methods: `multiply`, `divide`, `power`, `inverse`, `add`, `subtract`.
  - `primitive_polynomial: u16` field stores the polynomial used for construction.
- Integration tests for `Field`: AES sanity check (`0x53 × 0xCA = 1`), FIPS 197
  Appendix B (`0x57 × 0x83 = 0xC1`), RS backward-compat, `#[should_panic]` tests
  for divide-by-zero and inverse-of-zero.

## 0.1.0 — Initial release

### Added

- `ZERO: u8 = 0` — additive identity constant
- `ONE: u8 = 1` — multiplicative identity constant
- `PRIMITIVE_POLYNOMIAL: u16 = 0x11D` — the Reed-Solomon primitive polynomial x^8+x^4+x^3+x^2+1
- `add(a, b)` — GF(256) addition (bitwise XOR)
- `subtract(a, b)` — GF(256) subtraction (identical to add in characteristic 2)
- `multiply(a, b)` — GF(256) multiplication via precomputed log/antilog tables
- `divide(a, b)` — GF(256) division; panics on zero divisor
- `power(base, exp)` — GF(256) exponentiation via log table
- `inverse(a)` — multiplicative inverse; panics on zero input
- Log/antilog table construction via `build_tables()`, initialized lazily with `std::sync::OnceLock`
- `ALOG[255] = 1` to support `inverse(1)` and `power(a, 255) = 1` correctly
- Knuth-style literate comments throughout `src/lib.rs`
- 55+ integration tests in `tests/gf256_test.rs` including standard test vectors and exhaustive field-axiom checks
