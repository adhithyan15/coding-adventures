# Changelog

All notable changes to the `bignum-core` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.8.0] - 2026-08-02

### Added

- **`BigDouble::from_rational(r: &BigRational, prec: u32, mode: RoundingMode) -> BigDouble`** (NUM-7a)
  — the promotion primitive from the exact `BigRational` world into the approximate `BigDouble`
  world: the counterpart to `BigRational::to_f64`, generalized from a fixed 53-bit `f64` target to a
  caller-requested precision. Both parts enter exactly (each at its own bit length, capped at
  `MAX_PRECISION`), and the quotient is taken at `prec` bits so the result is the correctly rounded
  `BigDouble` for any rational of practical size. Introduced so the logic engine can wire a
  per-`KnowledgeBase`-configurable `Real`/`BigDouble` audit companion (ADJ-NUMERIC-SUBSTRATE §8).

## [0.7.0] - 2026-07-14

### Added

- **`BigDecimal::from_rational_exact(r: &BigRational) -> Option<BigDecimal>`** — the inverse of
  `to_rational`, and the rendering half of the ADJ exact-numbers arc (NX-4). A reduced fraction
  `p/q` has a **finite** decimal expansion iff `q`'s only prime factors are `2` and `5` (the primes
  of base 10); the method strips those factors, and if anything else remains (`3`, `7`, `11`, …) the
  expansion repeats and it returns `None` (so the caller falls back to a labeled-lossy `f64`). When
  it terminates, it rebalances to `mantissa / 10^scale` and hands the parts to `from_parts`, so
  `1/4 → 0.25`, `7/20 → 0.35`, and a doubled 39-digit π
  (`6283185307179586476925286766559005768394 / 10^39`) renders to all 39 fractional digits, while
  `1/3` and `2/7` return `None`. Sign rides on the mantissa; zero and integers are the scale-0 case.
  Round-trips exactly against `to_rational`. Introduced so the logic engine / CLI can print an exact
  **computed** result with every digit instead of the ~16-significant-digit f64 export.

## [0.6.0] - 2026-07-14

### Added

- **`BigDecimal::to_rational(&self) -> BigRational`** — the **exact** counterpart of the lossy
  `to_f64()`. A `BigDecimal` is `mantissa × 10^(-scale)`, always a ratio of two integers, so this
  converts with **zero loss and no `f64` hop**: `scale > 0` yields `mantissa / 10^scale`
  (`2.54 → 127/50`, `0.3048 → 381/1250`), and `scale ≤ 0` yields the whole number
  `mantissa × 10^|scale|` (`100 → 100/1`). The result is reduced to lowest terms by
  `BigRational::new`, and a 39-digit π converts to `3141592653589793238462643383279502884197 / 10^39`
  exactly. Introduced for ADJ exact-numbers NX-3 (exact compute ingestion): the logic engine now
  ingests a stored `Number::Exact` decimal into its `ExactRational` sidecar through this method, so
  arithmetic on a high-precision constant stays exact. Unit-tested across fractional, integer,
  zero, negative, and 39-digit-π inputs.

## [0.5.0] - 2026-07-14

### Added

- **`BigDecimal::significant_digits(&self) -> usize`** — counts the meaningful digits a value
  carries (the mantissa's digit count once trailing zeros are removed; `0` for zero). Because a
  `BigDecimal` is always canonical (no trailing-zero mantissa), this is simply the digit length of
  the mantissa's magnitude — `20.180 → 4`, `100 → 1`, `0 → 0`, π-to-39 → `39`. Introduced for the
  ADJ exact-numbers renderer (NX-2): it decides whether an exact value still fits an `f64`'s
  ~17-digit budget (render the `f64` canonical form, preserving existing output byte-for-byte) or
  exceeds it (render every exact digit). Tested across zero, trailing/leading zeros, the 17-vs-18
  digit f64 boundary, negatives, and scientific-notation inputs.

## [0.4.1] - 2026-07-11

### Added

- **`BigRational::to_f64()`** and **`BigDecimal::to_f64()`** — the **labeled lossy `f64`
  export** the ADJ engine needs for NUM-5 (arbitrary precision is the default; `f64` is a
  boundary export a consumer explicitly asks for). `BigRational::to_f64` divides the exact
  `num / den` as a `BigDouble` (numerator and denominator entering it exactly, capped at
  `MAX_PRECISION`) at `f64` width plus guard bits, so the result is the correctly-rounded
  nearest `f64` for any rational of practical size and saturates cleanly for extreme
  magnitudes (`10^400 → ∞`, `10^-400 → 0`); `BigDecimal::to_f64` routes through the value's
  plain-decimal string and Rust's correctly-rounded float parser. Tested: exact/dyadic
  round-trips, a 5,000-case differential of `to_f64` against hardware `n as f64 / d as f64`,
  and extreme-magnitude saturation without panic.

## [0.4.0] - 2026-07-11

### Added

- **`BigDouble`** (NUM-4): an arbitrary-precision **binary floating-point** number — a
  `BigInteger` mantissa and an `i64` base-2 exponent (`value = mantissa × 2^exponent`),
  carrying a working precision `prec` (the mantissa is normalized to exactly `prec`
  significant bits). This is the rung for numbers that are *not* any exact fraction —
  `√2`, and later `ln`, `exp`, `π` — where the honest thing is to compute to a **stated
  precision** under a **stated rounding mode** and carry how many bits are trustworthy,
  rather than pretend to exactness.
- Correctly-rounded `add`/`sub`/`mul`/`div` (+ `checked_div`) and `sqrt`, each to a
  requested precision and `RoundingMode` (all seven modes, reusing NUM-3's enum). Rounding
  uses **guard + sticky** information, so alignment costs `O(prec)`, not `O(exponent gap)`
  — a `10^large + 10^small`-style sum does not blow up. Addition handles the
  effective-subtraction (opposite-sign) case with a sign-aware sticky/borrow, so
  cancellation rounds the correct direction.
- `with_precision(prec, mode)` (re-round to a new precision), `from_f64` (**exact** —
  every `f64` is a dyadic rational), `from_bigint`/`from_i64`/`zero`/`one`/`from_parts`,
  and accessors `mantissa`/`exponent`/`precision`.
- `is_zero`/`is_negative`/`is_positive`/`signum`/`abs`/`neg`, total ordering **by value**
  (independent of stored precision, so `3` at 64 bits equals `3` at 200 bits), and
  `to_decimal()` — an **exact** conversion to `BigDecimal` (every binary fraction
  terminates in base 10, `x·2^-k = x·5^k·10^-k`), plus a lossy `to_f64()` that narrows
  through the exact decimal and Rust's correctly-rounded float parser.
- Security budgets so no untrusted input can be turned into unbounded memory or a silent
  wrong answer: `MAX_PRECISION` (1,000,000 bits) bounds every precision-driven shift/multiply;
  a public `MAX_EXPONENT` (`2^62`) bounds the stored base-2 exponent, and all
  exponent-*combining* arithmetic (`exp ± prec`, `exp + exp`, `exp − exp`) is carried in
  `i128` so it can never wrap `i64` — an out-of-range result is an explicit
  "exponent out of range" panic, never a silent truncation. `to_decimal` (which must
  *materialize* `~|exp|` digits) has its own smaller budget and returns `None` past it, so
  `to_f64`/`Display` fall back to saturation instead of exhausting memory.
- Tests: the headline **IEEE-754 differential** — at `prec = 53` with round-half-even,
  `+ − × ÷` and `√` reproduce hardware `f64` **bit for bit** across tens of thousands of
  random operands (from `from_f64` being exact); high-precision `√2`/`√3`/`√10` pinned to
  Python's `decimal` at 200 bits; the full rounding truth table on a binary tie across all
  seven modes; exact `to_decimal` (incl. the long tail of `f64` `0.1`); ordering/sign
  edges; and precision-budget/negative-`sqrt`/div-by-zero panics.
- Literate programming throughout; zero third-party dependencies; `#![forbid(unsafe_code)]`.
- The `BigDouble` module is declared at the end of `lib.rs` so it never textually collides
  with the other numeric-rung module declarations.
- **Deliberately deferred:** transcendental functions (`ln`, `exp`, `sin`, …) build on this
  core and are a separate later effort (NUM-4b); the engine adopting Big numbers by default
  is NUM-5.

## [0.3.0] - 2026-07-11

### Added

- **`BigDecimal`** (NUM-3): an arbitrary-precision **exact base-10** number — a `BigInteger`
  mantissa and an `i64` scale, with value `mantissa × 10^(-scale)`. Works in the base money,
  tax, and dosing are counted in, so `0.1 + 0.2` is exactly `0.3` and `100.00 − 0.01` is
  exactly `99.99` (neither of which a binary `f64` can represent). Held in one canonical form
  — trailing zeros stripped from the mantissa, zero pinned to `(0, 0)` — so `Clone`/`Eq`/`Hash`
  are derived and value-correct (`1.20 == 1.2`, `100 == 1e2`).
- **`RoundingMode`**: `Down`, `Up`, `Floor`, `Ceiling`, `HalfUp`, `HalfDown`, `HalfEven`
  (banker's). Division is the one base-10 operation that need not terminate, so it is done to
  a scale and mode you choose.
- Exact `add`/`sub`/`mul` (inherent + `std::ops`, owned & borrowed), `Neg`, `abs`, and
  `pow(u32)` (all exact); `div_round(other, target_scale, mode)` and `checked_div_round`;
  `round_to_scale(target_scale, mode)`.
- Total ordering by scale-aligned mantissa comparison, `is_zero`/`is_negative`/`is_positive`/
  `signum`, `mantissa`/`scale` accessors, `from_parts`/`from_integer`/`from_i64`/`zero`/`one`
  and `From<BigInteger/i64/u64/i128/u128>`.
- Parsing (`FromStr`) of plain (`"123.45"`, `"-0.001"`) and scientific (`"1.5e-3"`,
  `"6.022E23"`) notation with a typed `ParseDecimalError`; plain-decimal `Display` (never
  scientific) and a readable `Debug`.
- Tests: exact-arithmetic identities (incl. `0.1 + 0.2 == 0.3`), the full rounding truth
  table pinned against Python's `decimal` (`2.5`/`-2.5` across all seven modes, `1.25`→`1.2`
  and `1.35`→`1.4` under half-even), `div_round` pins, a 40,000-case differential of
  `+ − ×`/ordering and a 20,000-case differential of `div_round` (all seven modes) against an
  in-test `i128` decimal oracle, plus parse/round-trip and zero/canonical edges.
- Literate programming throughout; zero third-party dependencies; `#![forbid(unsafe_code)]`.
- The `BigDecimal` module is declared at the end of `lib.rs` so it never textually collides
  with the other numeric-rung module declarations.

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
