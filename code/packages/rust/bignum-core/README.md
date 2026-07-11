# bignum-core

An arbitrary-precision **signed integer** (`BigInteger`) built entirely from scratch, with **zero third-party dependencies** and **no `unsafe`**.

Where a machine `i64` overflows past `9_223_372_036_854_775_807`, a `BigInteger` just keeps going. `100!` (a 158-digit number) is stored exactly, every digit correct. It is the foundation rung (**NUM-1**) of the ADJ arbitrary-precision numeric substrate.

## Why it exists

Decision-critical arithmetic cannot silently lose precision — a single rounded percentage point can be worth hundreds of millions of dollars. The ADJ numeric substrate makes **exactness the default**, and every exact type (`BigRational`, `BigDecimal`, `BigDouble`) is built on top of an unbounded integer. That integer is this crate.

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
        BigRational   BigDecimal   BigDouble      (later NUM rungs)
             └─────────────┼─────────────┘
                           ▼
                   ┌───────────────┐
                   │  BigInteger   │   ← this crate (NUM-1)
                   │  zero deps    │
                   └───────────────┘
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

## The interesting algorithm: long division

Multiplication is grade-school. Division is **Knuth's Algorithm D** (TAOCP Vol. 2, §4.3.1) generalized to base `2^32`: normalize the divisor so its top limb has its high bit set (making each quotient-limb estimate at most 1 too big), estimate each quotient limb from the top two limbs of the running dividend, multiply-and-subtract, and — the famous part — **add the divisor back once** when the estimate overshoots. The source explains every step inline.

## Testing

```bash
cargo test -p bignum-core -- --nocapture
```

The suite is **differential**: it checks `BigInteger` against `i128` for every operation on both a hand-picked boundary table (values straddling the 32/64/96-bit limb edges) and tens of thousands of deterministic LCG-generated pairs (no RNG crate, fully reproducible). Beyond `i128` it pins `50!`, `100!`, `2^128`, `10^50`, consecutive-Fibonacci coprimality, radix round-trips, and the `a == q·b + r` division identity at arbitrary width.

## Zero dependencies

This is a standalone foundation package. Its `Cargo.toml` has an empty `[dependencies]`, and `#![forbid(unsafe_code)]` guarantees no `unsafe`.
