# bignum-core

A zero-dependency, `unsafe`-free **arbitrary-precision numeric core**: an unbounded signed integer (`BigInteger`, **NUM-1**) and, built on it, an exact never-rounding rational (`BigRational`, **NUM-2**) and an exact base-10 decimal (`BigDecimal`, **NUM-3**).

Where a machine `i64` overflows past `9_223_372_036_854_775_807`, a `BigInteger` just keeps going. `100!` (a 158-digit number) is stored exactly, every digit correct. A `BigRational` then makes the four everyday operations — `+ − × ÷` — **exact forever**: `1/3` is `1/3`, and `0.1 + 0.2` is exactly `3/10`, not `0.30000000000000004`.

## Why it exists

Decision-critical arithmetic cannot silently lose precision — a single rounded percentage point can be worth hundreds of millions of dollars. The ADJ numeric substrate makes **exactness the default**, and every exact type (`BigRational`, `BigDecimal`, `BigDouble`) is built on top of an unbounded integer. That integer is this crate; so is the first exact type built on it.

## Representation

A number is a **sign** plus a **magnitude**:

```
         sign              magnitude (little-endian base-2^32 limbs)
       ┌──────┐        ┌──────────┬──────────┬──────────┐
value  │ Plus │   ×    │  limb[0] │  limb[1] │  limb[2] │  ...
       └──────┘        └──────────┴──────────┴──────────┘
                         least significant     most significant
```

- **Sign-magnitude**, not two's-complement — the sign is a separate `Minus`/`Zero`/`Plus` tag, so no fixed width is ever needed.
- The magnitude is a `Vec<u32>` of **limbs** in base `2^32` (each limb is one "super-digit"), stored **little-endian** (`mag[0]` is least significant). Base `2^32` is chosen so two limbs multiply into a `u64` without overflow.
- **Strict normalization invariant**, enforced after every operation:
  1. No trailing zero limbs (`[7, 0]` is illegal → `[7]`).
  2. Zero is unique: always `sign = Zero, mag = []`. There is **never** a `-0`.

Because the form is canonical, `PartialEq`/`Eq`/`Hash` are derived and correct.

## Layer position

```
                                   BigDouble          (later NUM rung)
                                       ▲
                     ┌──────────────┐  │  ┌──────────────┐
                     │ BigRational  │  │  │  BigDecimal  │  ← this crate (NUM-2, NUM-3)
                     └──────┬───────┘  │  └──────┬───────┘
                            └──────────┴─────────┘
                                       ▼
                              ┌────────────────┐
                              │   BigInteger   │   ← this crate (NUM-1)
                              │    zero deps   │
                              └────────────────┘
```

## Usage

```rust
use bignum_core::BigInteger;

// Construction
let a = BigInteger::from_u64(1_000_000_000);
let b = BigInteger::from_i128(-42);

// Arithmetic — inherent methods or operators (owned and borrowed forms)
let sum  = a.add(&BigInteger::one());
let prod = &a * &a;                 // 10^18
let neg  = -&a;

// Truncating division: quotient toward zero, remainder takes the dividend's sign
let (q, r) = BigInteger::from_i64(-7).div_rem(&BigInteger::from_i64(2));
assert_eq!(q.to_string(), "-3");
assert_eq!(r.to_string(), "-1");

// Powers (exponentiation by squaring) and gcd (Euclid)
let big = BigInteger::from_u64(2).pow(128);
assert_eq!(big.to_string(), "340282366920938463463374607431768211456");
let g = BigInteger::from_u64(48).gcd(&BigInteger::from_u64(36)); // 12

// Parse / format in any radix 2..=36
use std::str::FromStr;
let n = BigInteger::from_str("340282366920938463463374607431768211456").unwrap();
assert_eq!(n.to_str_radix(16), "100000000000000000000000000000000");
assert_eq!(BigInteger::parse_radix("ff", 16).unwrap(), BigInteger::from_u64(255));
```

## API reference

### Constructors

| Method | Description |
|--------|-------------|
| `zero()` / `one()` | Canonical `0` / `1` |
| `from_i64` / `from_u64` | From a 64-bit integer |
| `from_i128` / `from_u128` | From a 128-bit integer |
| `From<i64/u64/i128/u128>` | Same, via `.into()` |
| `parse_radix(s, radix)` | Parse in radix `2..=36`; typed error, never panics |
| `FromStr` | Base-10 parse |

### Arithmetic

| Method | Operator | Description |
|--------|----------|-------------|
| `add` | `+` | Addition |
| `sub` | `-` | Subtraction |
| `mul` | `*` | Multiplication (schoolbook `O(n·m)`) |
| `div_rem` | | `(quotient, remainder)`, truncating toward zero |
| `div` | `/` | Quotient |
| `rem` | `%` | Remainder (sign of dividend) |
| `neg` / `abs` | unary `-` | Negation / absolute value |
| `pow(exp)` | | Exponentiation by squaring |
| `gcd(&other)` | | Euclid's algorithm, always non-negative |

Division panics on a zero divisor, mirroring Rust's built-in integer `/` and `%`.

### Queries & comparison

| Method | Description |
|--------|-------------|
| `is_zero` / `is_negative` / `is_positive` | Sign predicates |
| `signum() -> i32` | `-1`, `0`, or `+1` |
| `num_limbs()` / `bit_len()` | Magnitude size (limbs / bits) |
| `Ord`, `PartialOrd`, `Eq`, `PartialEq`, `Hash` | By sign then magnitude |

### Formatting

| Method | Description |
|--------|-------------|
| `to_str_radix(radix)` | Render in radix `2..=36` (lowercase) |
| `Display` | Base-10 (e.g. `-123`) |
| `Debug` | `BigInteger(-123)` |

## BigRational — exact fractions (NUM-2)

`BigRational` is a numerator over a denominator, both `BigInteger`s, so `+ − × ÷` never round.

```rust
use bignum_core::BigRational;
use std::str::FromStr;

// 1/3 + 1/6 = 1/2 — exact.
let half = &BigRational::from_ints(1, 3) + &BigRational::from_ints(1, 6);
assert_eq!(half.to_string(), "1/2");

// The float trap, avoided: 0.1 + 0.2 is exactly 3/10.
let sum = &BigRational::from_ints(1, 10) + &BigRational::from_ints(2, 10);
assert_eq!(sum.to_string(), "3/10");

// Canonical on the way in: reduced, sign on the numerator, zero is 0/1.
assert_eq!(BigRational::from_ints(50, 100).to_string(), "1/2");
assert_eq!(BigRational::from_ints(3, -4).to_string(), "-3/4");

// Reciprocal, negative powers, exact comparison.
assert_eq!(BigRational::from_ints(2, 3).pow(-3).to_string(), "27/8");
assert!(BigRational::from_ints(22, 7) > BigRational::from_ints(355, 113));

// Parse "num/den" or a bare integer; whole numbers print without a slash.
assert_eq!(BigRational::from_str("6/3").unwrap().to_string(), "2");
```

Every `BigRational` is held in one **canonical form**, re-established after each operation:
lowest terms (divide both parts by their gcd), sign carried by the numerator (denominator
always strictly positive), zero always `0/1`, and a zero denominator forbidden. Because that
form is unique, `Eq`/`Hash` are derived and value-correct — `2/4` and `1/2` are equal and
hash alike.

| Area | API |
|------|-----|
| Construct | `zero`, `one`, `new`/`checked_new`, `from_ints`, `from_integer`, `From<BigInteger/i64/u64/i128/u128>` |
| Arithmetic | `add`/`+`, `sub`/`-`, `mul`/`*`, `div`/`/` (+ `checked_div`), unary `-`, `abs`, `recip`/`checked_recip` |
| Powers | `pow(i32)` (negative ⇒ reciprocal), `try_pow(exp, max_bits)` (DoS-safe) |
| Query | `is_zero`, `is_integer`, `is_negative`, `is_positive`, `signum`, `numerator`, `denominator`, `Ord`/`Eq`/`Hash` |
| I/O | `FromStr` (`"num/den"` or `"num"`) with typed `ParseRatioError`; `Display`/`Debug` |

Exact by default, lossy only on purpose: the `f64` boundary export is deliberately **not**
here — it belongs to a later rung (NUM-5) — so nothing in this crate can silently round.

## The interesting algorithm: long division

Multiplication is grade-school. Division is **Knuth's Algorithm D** (TAOCP Vol. 2, §4.3.1) generalized to base `2^32`: normalize the divisor so its top limb has its high bit set (making each quotient-limb estimate at most 1 too big), estimate each quotient limb from the top two limbs of the running dividend, multiply-and-subtract, and — the famous part — **add the divisor back once** when the estimate overshoots. The source explains every step inline.

## Testing

```bash
cargo test -p bignum-core -- --nocapture
```

The suite is **differential**: it checks `BigInteger` against `i128` for every operation on both a hand-picked boundary table (values straddling the 32/64/96-bit limb edges) and tens of thousands of deterministic LCG-generated pairs (no RNG crate, fully reproducible). Beyond `i128` it pins `50!`, `100!`, `2^128`, `10^50`, consecutive-Fibonacci coprimality, radix round-trips, and the `a == q·b + r` division identity at arbitrary width.

`BigRational` is tested the same way: a 40,000-case differential run of `+ − × ÷` and ordering against an in-test `i128` fraction oracle, big cases pinned against Python's `fractions.Fraction` (sums, products, quotients, and powers beyond `i128`), the float-famous `0.1 + 0.2 == 3/10`, canonicalization/sign/zero edge cases, the `try_pow` DoS guard, and the reciprocal/division-by-zero panics.

`BigDecimal` adds a 40,000-case differential of `+ − ×`/ordering and a 20,000-case differential of `div_round` (all seven rounding modes) against an in-test `i128` decimal oracle, plus the rounding truth table pinned against Python's `decimal`.

## BigDecimal — exact base-10 (NUM-3)

`BigDecimal` is a `BigInteger` **mantissa** and an `i64` **scale**: the value is `mantissa × 10^(-scale)`. It works in the base money, tax, and dosing use, so `+ − ×` are always exact and division rounds to a scale and mode you name.

```rust
use bignum_core::{BigDecimal, RoundingMode};
use std::str::FromStr;

// 0.1 + 0.2 is exactly 0.3 (a binary f64 cannot hold either operand).
let sum = &BigDecimal::from_str("0.1").unwrap() + &BigDecimal::from_str("0.2").unwrap();
assert_eq!(sum.to_string(), "0.3");

// Money stays exact through +, -, *.
let change = &BigDecimal::from_str("100.00").unwrap() - &BigDecimal::from_str("0.01").unwrap();
assert_eq!(change.to_string(), "99.99");

// Division rounds — to the scale and mode you state.
let third = BigDecimal::from_str("10").unwrap()
    .div_round(&BigDecimal::from_str("3").unwrap(), 4, RoundingMode::HalfEven);
assert_eq!(third.to_string(), "3.3333");

// Banker's rounding breaks ties to even: 2.5 → 2, 1.25 → 1.2.
assert_eq!(BigDecimal::from_str("2.5").unwrap().round_to_scale(0, RoundingMode::HalfEven).to_string(), "2");
```

Every value is canonical — trailing zeros stripped, zero pinned to `(0, 0)` — so `Eq`/`Hash` are derived and value-correct (`1.20 == 1.2`, `100 == 1e2`). Presenting a value at a *fixed* number of places (`"$1.20"`) is a boundary-formatting concern (NUM-6), not a property of the stored number.

| Area | API |
|------|-----|
| Construct | `zero`, `one`, `from_parts`, `from_integer`, `from_i64`, `From<BigInteger/i64/u64/i128/u128>` |
| Exact arithmetic | `add`/`+`, `sub`/`-`, `mul`/`*`, unary `-`, `abs`, `pow(u32)` |
| Rounding | `RoundingMode` (`Down`/`Up`/`Floor`/`Ceiling`/`HalfUp`/`HalfDown`/`HalfEven`), `div_round`/`checked_div_round`, `round_to_scale` |
| Query | `is_zero`, `is_negative`, `is_positive`, `signum`, `mantissa`, `scale`, `Ord`/`Eq`/`Hash` |
| I/O | `FromStr` (plain + scientific) with typed `ParseDecimalError`; plain-decimal `Display`/`Debug` |

## Zero dependencies

This is a standalone foundation package. Its `Cargo.toml` has an empty `[dependencies]`, and `#![forbid(unsafe_code)]` guarantees no `unsafe`.
