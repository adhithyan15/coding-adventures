# Changelog

All notable changes to this package will be documented in this file.

## [0.3.0] - 2026-07-11

### Added

- **`ca::BigRational`** (`bignum_rational.hpp`, header-only) — the exact fraction
  `rational` rung of the Rust crate, built on `BigInteger`: a numerator over a
  denominator in canonical form (lowest terms, positive denominator, `0/1`
  zero), with value semantics.
- Construction (`zero` / `one` / `from_i64` / `from_u64` / `from_integer` /
  `from_ints` / `make` / `checked_make`), accessors (`numerator` /
  `denominator`), predicates & sign (incl. `abs` / `recip` / `checked_recip`).
- Exact `add` / `sub` / `mul` / `div` (and `+ - * /` / unary `-` operators);
  integer `pow` (negative exponent → reciprocal) and DoS-safe `try_pow`; `cmp`
  and the six comparison operators via cross-multiplication.
- `parse` / `try_parse` (`"num/den"` or a bare integer; `ca::ParseRatioError`
  with a `kind`), `to_string`, and the lossy `to_f64` (through the exact
  `BigDecimal` division — no `<cmath>`; saturates to ±inf / 0 out of range).
- Every fallible op has both a throwing and a non-throwing (`std::optional` /
  exception) form; the i32 exponent magnitude is taken without INT32_MIN UB.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): canonical form, sign
  placement, exact arithmetic (including big operands pinned against Python's
  `fractions.Fraction`), ordering, reciprocal, powers, parsing, and `to_f64`.

## [0.2.0] - 2026-07-11

### Added

- **`ca::BigDecimal`** (`bignum_decimal.hpp`, header-only) — the exact base-10
  `decimal` rung of the Rust crate, built on `BigInteger`: `mantissa × 10^(-scale)`
  in canonical form, with value semantics.
- Construction (`zero` / `one` / `from_i64` / `from_integer` / `from_parts` /
  `checked_from_parts`), accessors (`mantissa` / `scale`), predicates & sign.
- Exact `add` / `sub` / `mul` / `pow` (and `+ - *` / unary `-` operators);
  rounding `div_round` / `checked_div_round` / `round_to_scale` over seven
  `ca::RoundingMode`s; `cmp` and the six comparison operators.
- `parse` / `try_parse` (plain and scientific; `ca::ParseDecimalError` with a
  `kind`), `to_string` (plain notation), and the lossy `to_f64` (through
  `std::strtod` — no `<cmath>`).
- Every fallible op has both a throwing and a non-throwing (`std::optional`)
  form. `parse` enforces the strict `MAX_SCALE` (10^6) budget on the canonical
  scale; scale bookkeeping uses hand-written checked i64 arithmetic (no `i128`).
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): canonical form, display,
  parsing, exact arithmetic, `pow`, the full rounding truth table, rounding
  division, ordering, `to_f64`, and MAX_SCALE amplification-payload rejection.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the `BigInteger` core of the Rust
  `bignum-core` crate, in namespace `ca`: sign-magnitude arbitrary-precision
  integers over `std::vector<std::uint32_t>` limbs (64-bit accumulator, no
  128-bit integers).
- Value type with operator overloads (`+ - * / % -` and the six comparisons);
  `abs` / `neg` / `pow` / `try_pow` / `gcd`.
- Arithmetic: column add/sub, schoolbook multiply, Knuth Algorithm D `div_rem`
  (truncating toward zero); `to_str_radix` / `to_string` and `parse_radix`
  (radix 2–36).
- Errors as exceptions: `std::domain_error` (divide by zero),
  `ca::ParseBigIntError` (parse), `ca::PowTooLargeError` (try_pow guard).
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) cross-checked against
  Python's arbitrary-precision integers, matching the Rust crate's oracle tests.
