# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-07-11

### Added

- **BigDecimal** (`bignum_decimal.h` / `bignum_decimal.c`) — the exact base-10
  `decimal` rung of the Rust crate, built on `BigInteger`: `mantissa × 10^(-scale)`
  in canonical form (mantissa carries no trailing zero; zero is `(0, 0)`).
- Construction (`dec_zero` / `one` / `from_i64` / `from_integer` / `from_parts` /
  `clone`), accessors (`dec_mantissa` / `dec_scale`), predicates & sign
  (`is_zero` / `is_negative` / `is_positive` / `signum` / `abs` / `neg`).
- Exact `dec_add` / `dec_sub` / `dec_mul` / `dec_pow`; rounding `dec_div_round`
  and `dec_round_to_scale` over seven `DecRoundingMode`s; `dec_cmp` total order.
- `dec_parse` (plain and scientific notation, typed `DecParseStatus`),
  `dec_to_string` (plain notation, never scientific), and the lossy `dec_to_f64`
  (through `strtod` — no `<math.h>`).
- Security: `dec_parse` enforces the strict `DEC_MAX_SCALE` (10^6) budget on the
  canonical scale, bounding any later power-of-ten materialization; scale
  bookkeeping uses hand-written checked i64 arithmetic (no `__int128`); Rust
  panics (scale past the ceiling, divide by zero) are returned as `DecStatus`.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): canonical form, display,
  parsing, exact arithmetic, `pow`, the full rounding truth table, rounding
  division, ordering, `to_f64`, and the MAX_SCALE amplification-payload rejection.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the `BigInteger` core of the Rust `bignum-core`
  crate: sign-magnitude arbitrary-precision integers, little-endian base-2^32
  limbs, 32-bit limbs with a 64-bit accumulator (no 128-bit integers).
- Construction (`bigint_zero` / `one` / `from_i64` / `from_u64` / `clone`),
  queries (`is_zero` / `signum` / `num_limbs` / `bit_len` / `cmp`), and sign
  transforms (`abs` / `neg`).
- Arithmetic: `add` / `sub` (column methods), `mul` (schoolbook), `div_rem` /
  `div` / `rem` (Knuth Algorithm D, truncating toward zero), `pow`
  (exponentiation by squaring), `try_pow` (O(1) size guard), `gcd` (Euclid).
- Radix 2–36 `parse_radix` (typed `BigIntStatus` errors, never crashes) and
  `to_str_radix` / `to_string`.
- malloc-owned handles freed with `bigint_free`; overflow-guarded via checked
  `calloc` in multiply and a `BigIntStatus` throughout.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) cross-checked against
  Python's arbitrary-precision integers (factorials, 2^128, 7^99, div_rem, gcd,
  base-16/36), matching the Rust crate's oracle tests.
