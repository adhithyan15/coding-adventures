# Changelog

All notable changes to the `bignum-core` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-07-11

### Added

- Core `BigInteger` type: an arbitrary-precision signed integer in sign-magnitude form
  (`Minus`/`Zero`/`Plus` sign + little-endian base-`2^32` `Vec<u32>` limbs), with a
  strict normalization invariant (no trailing zero limbs; a unique, canonical zero and
  never `-0`) enforced after every operation.
- Constructors: `zero()`, `one()`, `from_i64`, `from_u64`, `from_i128`, `from_u128`,
  and `From<i64/u64/i128/u128>` conversions.
- Parsing: `parse_radix(s, radix)` for radix `2..=36` (optional leading `+`/`-`,
  case-insensitive digits) with a typed `ParseBigIntError` (`Empty`, `InvalidDigit`,
  `InvalidRadix`) — never panics; and `FromStr` for base 10.
- Formatting: `to_str_radix(radix)` for radix `2..=36`, `Display` (base 10), and a
  readable `Debug` (`BigInteger(-123)`).
- Arithmetic as both inherent methods and `std::ops` traits (owned and borrowed forms):
  `add`, `sub`, `mul` (schoolbook `O(n·m)`), `div_rem`/`div`/`rem` (truncating toward
  zero via Knuth's Algorithm D on the limb vectors; remainder takes the dividend's
  sign, matching Rust's `/` and `%`; panics on a zero divisor), `Neg`, and `abs`.
- `pow(exp)` via exponentiation by squaring; `gcd(&other)` via the Euclidean algorithm
  (always non-negative).
- Comparison and hashing: `Ord`/`PartialOrd`/`Eq`/`PartialEq`/`Hash` (by sign then
  magnitude), plus `is_zero`, `is_negative`, `is_positive`, `signum`, `num_limbs`,
  `bit_len`.
- Extensive test suite: differential checks against `i128` over a hand-picked
  limb-boundary table and tens of thousands of deterministic LCG-generated operand
  pairs (no RNG crate — fully reproducible); known-big pins (`50!`, `100!`, `2^128`,
  `10^50`); consecutive-Fibonacci coprimality; radix round-trips across several bases;
  the `a == q·b + r`, `|r| < |b|` division identity including Algorithm-D
  correction-step cases; zero-normalization and parse edge cases; and division-by-zero
  panics.
- Literate programming throughout: the representation, the normalization invariant, and
  every algorithm (schoolbook multiply, long division / Knuth Algorithm D with its
  multiply-subtract and add-back correction, exponentiation by squaring, Euclid) are
  explained inline with diagrams and worked reasoning.
- Zero third-party dependencies; `#![forbid(unsafe_code)]`.
