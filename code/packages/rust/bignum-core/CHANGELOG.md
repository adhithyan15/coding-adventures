# Changelog

All notable changes to the `bignum-core` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-07-11

### Added

- **`BigRational`** (NUM-2): an arbitrary-precision **exact** rational number — a
  `BigInteger` numerator over a `BigInteger` denominator — that never rounds. It is kept in
  one canonical form, re-established after every operation: reduced to lowest terms (both
  parts divided by their gcd), the sign carried entirely by the numerator (denominator
  **always strictly positive**), zero collapsed to `0/1`, and a zero denominator forbidden.
  Because the form is unique, `Clone`/`PartialEq`/`Eq`/`Hash` are derived and value-correct
  (so `2/4 == 1/2` and both hash to the same bucket).
- Constructors: `zero()`, `one()`, `new(num, den)` (panics on a zero denominator),
  `checked_new` (returns `None` instead), `from_ints(i64, i64)`, `from_integer(BigInteger)`,
  and `From<BigInteger/i64/u64/i128/u128>`.
- Exact arithmetic as inherent methods and `std::ops` traits (owned and borrowed forms):
  `add`, `sub`, `mul`, `div` (+ `checked_div`) — all reduced back to lowest terms — plus
  unary `Neg`, `abs`, and `recip`/`checked_recip` (reciprocal). `div`/`recip` panic on a
  zero divisor/zero value; the `checked_*` forms return `None`.
- `pow(exp: i32)` — integer powers, with a **negative** exponent taking the reciprocal
  (`(a/b)^-n = (b/a)^n`) and `x^0 = 1`; and `try_pow(exp, max_bits)`, the DoS-safe form that
  refuses (in O(1), before allocating) if the numerator or denominator of the result would
  exceed `max_bits` — reusing `BigInteger::try_pow` so an untrusted exponent cannot OOM.
- Total ordering (`Ord`/`PartialOrd`) by cross-multiplication (`a/b < c/d` iff `a·d < c·b`,
  valid because denominators are canonically positive), plus `is_zero`, `is_integer`,
  `is_negative`, `is_positive`, `signum`, and `numerator`/`denominator` accessors.
- Parsing (`FromStr`) of `"num/den"` or a bare integer `"num"` with a typed
  `ParseRatioError` (`Empty`, `InvalidInteger`, `TooManySlashes`, `ZeroDenominator`), and
  `Display`/`Debug` that render `num/den` (or just `num` for whole numbers).
- Tests: exact identities (incl. the float-famous `0.1 + 0.2 == 3/10`), big cases pinned
  against Python's `fractions.Fraction` (sums, products, quotients, and `x^3`/`x^-2` beyond
  `i128`), a 40,000-case deterministic differential check of `+ - * /` and ordering against
  an in-test `i128` fraction oracle, canonicalization/sign/zero edge cases, the `try_pow`
  DoS guard, and reciprocal/division-by-zero panics.
- Literate programming throughout: the canonical-form invariant and each operation
  (including *why* cross-multiplication needs positive denominators) are explained inline.
- **Deliberately deferred:** the lossy `f64` boundary export lives in NUM-5, not here, so
  that nothing in this crate can silently round.

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
- `try_pow(exp, max_bits)` — the DoS-safe form of `pow`: because a result's bit length
  is `≤ bit_len(base) · exp`, it refuses an oversized result up front (in O(1), before
  any allocation) with a typed `PowTooLargeError`, so an untrusted exponent cannot
  trigger an out-of-memory abort. `pow` documents that it must not be called with an
  untrusted exponent.
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
