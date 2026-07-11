//! # bignum-core — an arbitrary-precision signed integer, from scratch
//!
//! This crate implements [`BigInteger`], a signed integer with **no fixed width**.
//! Where a machine `i64` overflows past `9_223_372_036_854_775_807`, a `BigInteger`
//! keeps counting — `100!` (a 158-digit number) is represented exactly, with every
//! digit correct. It is the foundation rung (NUM-1) of the ADJ arbitrary-precision
//! numeric substrate: the layer on which exact rationals, decimals, and
//! arbitrary-precision floats are later built.
//!
//! It has **zero third-party dependencies** and **no `unsafe`** (enforced by the
//! `forbid(unsafe_code)` attribute below).
//!
//! ## How a big integer is stored
//!
//! A number has two independent parts: a **sign** and a **magnitude** (its absolute
//! value). We keep them separate — this is *sign-magnitude* representation, the way
//! you would write a number on paper (`-` then the digits), and deliberately **not**
//! two's-complement (which is what fixed-width machine integers use). Sign-magnitude
//! keeps the two concerns cleanly apart and avoids ever needing a width to "borrow
//! from".
//!
//! ```text
//!            sign              magnitude (little-endian base-2^32 limbs)
//!          ┌──────┐        ┌────────────┬────────────┬────────────┐
//!   value  │ Plus │   ×    │  limb[0]   │  limb[1]   │  limb[2]   │  ...
//!          └──────┘        └────────────┴────────────┴────────────┘
//!                            least significant          most significant
//! ```
//!
//! The magnitude is a `Vec<u32>` of **limbs**. Think of each limb as one "super-digit"
//! in base `2^32` (i.e. `4_294_967_296`). We chose base `2^32` because two limbs
//! multiply into a `u64` and three quantities add into a `u64` without overflow, so
//! the CPU's 64-bit arithmetic is exactly the scratch space we need — no `unsafe`, no
//! 128-bit tricks except where division genuinely needs them.
//!
//! Limbs are stored **little-endian**: `mag[0]` is the least-significant limb. So the
//! number `N = Σ mag[i] · (2^32)^i`. For example the value `2^32 + 7` is `[7, 1]`
//! (`7·1 + 1·2^32`).
//!
//! ## The normalization invariant (the one rule everything depends on)
//!
//! After **every** operation we enforce a strict *canonical form*:
//!
//! 1. **No trailing zero limbs.** `[7, 0]` is illegal; it must be trimmed to `[7]`.
//!    (A trailing *high* zero limb carries no information, like a leading zero in
//!    `007`.) This makes magnitude comparison as simple as "longer vector wins".
//! 2. **Zero is unique.** The value zero is *always* `sign = Zero, mag = []` — an
//!    empty magnitude. There is no `-0`, ever. `0 - 0`, `x - x`, and `parse("-0")`
//!    all produce the exact same canonical zero.
//!
//! Because the form is canonical, `#[derive(PartialEq, Eq, Hash)]` is correct:
//! two `BigInteger`s are equal *if and only if* their fields are structurally equal.
//!
//! ## Quick tour
//!
//! ```
//! use bignum_core::BigInteger;
//!
//! let a = BigInteger::from_u64(1_000_000_000);
//! let b = &a * &a;                 // 10^18, still tiny for us
//! assert_eq!(b.to_string(), "1000000000000000000");
//!
//! let big = BigInteger::from_u64(2).pow(128);
//! assert_eq!(big.to_string(), "340282366920938463463374607431768211456");
//!
//! let (q, r) = BigInteger::from_i64(-7).div_rem(&BigInteger::from_i64(2));
//! assert_eq!(q.to_string(), "-3");   // truncates toward zero, like Rust's `/`
//! assert_eq!(r.to_string(), "-1");   // remainder takes the dividend's sign
//! ```

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

// The exact-rational rung (NUM-2) lives in its own module and is built entirely on the
// `BigInteger` below. It is re-exported here so consumers write `bignum_core::BigRational`.
pub mod rational;
pub use rational::{BigRational, ParseRatioError};

// ===========================================================================
//  Sign
// ===========================================================================

/// The sign of a [`BigInteger`].
///
/// `Zero` is a *distinct* third state, not "positive zero" — this is what makes the
/// zero value canonical and unique. Whenever the magnitude becomes empty, the sign
/// must be `Zero`; whenever the sign is `Zero`, the magnitude must be empty.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Sign {
    Minus,
    Zero,
    Plus,
}

// ===========================================================================
//  The public type
// ===========================================================================

/// An arbitrary-precision signed integer.
///
/// See the [crate-level documentation](crate) for the representation and invariants.
/// The two fields are private so the normalization invariant can never be violated
/// from outside this module.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BigInteger {
    /// The sign. `Zero` iff `mag` is empty.
    sign: Sign,
    /// The magnitude: little-endian base-`2^32` limbs, with **no trailing zero limb**.
    /// Empty iff the value is zero.
    mag: Vec<u32>,
}

// ===========================================================================
//  Errors
// ===========================================================================

/// The error returned when a string cannot be parsed into a [`BigInteger`].
///
/// Parsing never panics; every rejection is one of these typed variants.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ParseBigIntError {
    /// The input was empty, or was only a sign (`"+"`, `"-"`) with no digits.
    Empty,
    /// A character was not a valid digit in the requested radix. Carries the offender.
    InvalidDigit(char),
    /// The requested radix was outside the supported `2..=36` range. Carries the value.
    InvalidRadix(u32),
}

impl fmt::Display for ParseBigIntError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseBigIntError::Empty => f.write_str("cannot parse an empty integer string"),
            ParseBigIntError::InvalidDigit(c) => {
                write!(f, "invalid digit {c:?} for the requested radix")
            }
            ParseBigIntError::InvalidRadix(r) => {
                write!(f, "invalid radix {r}: must be in 2..=36")
            }
        }
    }
}

impl std::error::Error for ParseBigIntError {}

/// The error returned by [`BigInteger::try_pow`] when the projected result would
/// exceed the caller's size ceiling.
///
/// Exponentiation grows the result LINEARLY in the exponent: `bit_len(baseᵉ) ≈
/// bit_len(base) · e`. A `u32` exponent sourced from untrusted input (a document, a
/// model) can therefore ask for a multi-gigabit number from a tiny expression
/// (`2 ^ 4000000000`), exhausting memory before a single wrong digit is produced.
/// `try_pow` refuses up front — in O(1), before any allocation — when the projected
/// bit length crosses `max_bits`, turning a resource-exhaustion DoS into this clean,
/// typed error. Carries both the projected size and the ceiling that rejected it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PowTooLargeError {
    /// The (upper-bound) bit length the result would have had: `bit_len(base) · exp`.
    pub projected_bits: u64,
    /// The ceiling the caller supplied, which `projected_bits` exceeded.
    pub max_bits: u64,
}

impl fmt::Display for PowTooLargeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "pow result would be ~{} bits, exceeding the {}-bit ceiling",
            self.projected_bits, self.max_bits
        )
    }
}

impl std::error::Error for PowTooLargeError {}

// ===========================================================================
//  Magnitude primitives — pure functions on normalized little-endian limb slices
// ===========================================================================
//
// These helpers know nothing about sign. They operate on `&[u32]` magnitudes that
// are already normalized (no trailing zero limb) and return normalized results.
// Keeping them as free functions makes the sign-handling code above read like the
// grade-school rules: "add the magnitudes, keep the common sign", and so on.

/// Trim trailing zero limbs so the magnitude is canonical.
///
/// `[7, 0, 0]` → `[7]`; `[0]` → `[]` (the canonical zero magnitude).
fn normalize(mag: &mut Vec<u32>) {
    while matches!(mag.last(), Some(&0)) {
        mag.pop();
    }
}

/// Compare two normalized magnitudes.
///
/// Because there are no trailing zeros, a longer limb vector is unambiguously the
/// larger magnitude; only when the lengths tie do we compare limbs, and then from the
/// **most** significant down (the first limb that differs decides it).
fn mag_cmp(a: &[u32], b: &[u32]) -> Ordering {
    if a.len() != b.len() {
        return a.len().cmp(&b.len());
    }
    for i in (0..a.len()).rev() {
        match a[i].cmp(&b[i]) {
            Ordering::Equal => continue,
            non_eq => return non_eq,
        }
    }
    Ordering::Equal
}

/// Add two magnitudes. Result is normalized.
///
/// This is the paper algorithm: line the limbs up, add column by column, and carry
/// the overflow. We use a `u64` accumulator so that `limb + limb + carry` (each at
/// most `2^32 - 1`) can never overflow (the max is well under `2^33`).
fn mag_add(a: &[u32], b: &[u32]) -> Vec<u32> {
    // Iterate over the longer operand; treat the shorter one as zero-extended.
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut result = Vec::with_capacity(long.len() + 1);
    let mut carry: u64 = 0;
    for (i, &hi) in long.iter().enumerate() {
        let mut sum = hi as u64 + carry;
        if let Some(&lo) = short.get(i) {
            sum += lo as u64;
        }
        result.push(sum as u32); // low 32 bits stay in this column
        carry = sum >> 32; // high bits carry to the next column
    }
    if carry != 0 {
        result.push(carry as u32);
    }
    // The top limb is either the (nonzero) carry or `long`'s (nonzero) top limb, so
    // the result is already normalized — but we do not rely on that assumption
    // anywhere fragile, and every caller re-checks via `from_parts`.
    result
}

/// Subtract magnitudes, computing `a - b`. **Requires `a >= b`** (caller guarantees).
///
/// The paper "borrow" method: subtract column by column, and when a column would go
/// negative, borrow `2^32` from the next column up. Because `a >= b`, the final borrow
/// is always zero. We normalize because subtraction can create trailing zeros
/// (e.g. `[0, 1] - [0, 1] = [0, 0]` → `[]`).
fn mag_sub(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut result = Vec::with_capacity(a.len());
    let mut borrow: i64 = 0;
    for (i, &hi) in a.iter().enumerate() {
        let lo = b.get(i).copied().unwrap_or(0) as i64;
        let mut diff = hi as i64 - lo - borrow;
        if diff < 0 {
            diff += 1i64 << 32;
            borrow = 1;
        } else {
            borrow = 0;
        }
        result.push(diff as u32);
    }
    normalize(&mut result);
    result
}

/// Multiply two magnitudes with the schoolbook `O(n·m)` algorithm. Result normalized.
///
/// Each limb of `a` is multiplied by all of `b`, shifted into place by `i`, and
/// accumulated. The running `carry` plus the product of two limbs plus the slot's
/// current value all fit in a `u64`:
/// `(2^32-1) + (2^32-1)·(2^32-1) + (2^32-1) = 2^64 - 2^32 < 2^64`. No overflow.
fn mag_mul(a: &[u32], b: &[u32]) -> Vec<u32> {
    if a.is_empty() || b.is_empty() {
        return Vec::new(); // anything times zero is zero
    }
    // A product of an m-limb and n-limb number needs at most m+n limbs.
    let mut result = vec![0u32; a.len() + b.len()];
    for (i, &ai_u32) in a.iter().enumerate() {
        let ai = ai_u32 as u64;
        let mut carry: u64 = 0;
        for (j, &bj) in b.iter().enumerate() {
            let idx = i + j;
            let cur = result[idx] as u64 + ai * bj as u64 + carry;
            result[idx] = cur as u32;
            carry = cur >> 32;
        }
        // The limb just above this row has only ever received carries, so it is still
        // free to take the final carry from this row.
        result[i + b.len()] = (result[i + b.len()] as u64 + carry) as u32;
    }
    normalize(&mut result);
    result
}

/// Multiply a magnitude by a single small factor (`< 2^32`). Used by the parser.
fn mag_mul_small(mag: &[u32], factor: u32) -> Vec<u32> {
    if factor == 0 || mag.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::with_capacity(mag.len() + 1);
    let mut carry: u64 = 0;
    let f = factor as u64;
    for &limb in mag {
        let cur = limb as u64 * f + carry;
        result.push(cur as u32);
        carry = cur >> 32;
    }
    if carry != 0 {
        result.push(carry as u32);
    }
    result
}

/// Add a single small addend (`< 2^32`) to a magnitude in place-style. Used by the parser.
fn mag_add_small(mag: &[u32], addend: u32) -> Vec<u32> {
    let mut result = mag.to_vec();
    let mut carry = addend as u64;
    let mut i = 0;
    while carry != 0 {
        if i < result.len() {
            let cur = result[i] as u64 + carry;
            result[i] = cur as u32;
            carry = cur >> 32;
            i += 1;
        } else {
            result.push(carry as u32);
            carry = 0;
        }
    }
    result
}

/// Divide a magnitude by a single small divisor (`< 2^32`), returning
/// `(quotient_magnitude, remainder)`. This is "long division" with a one-digit
/// divisor: sweep from the most-significant limb down, carrying the running remainder.
fn mag_divmod_small(mag: &[u32], divisor: u32) -> (Vec<u32>, u32) {
    let d = divisor as u64;
    let mut q = vec![0u32; mag.len()];
    let mut rem: u64 = 0;
    for i in (0..mag.len()).rev() {
        // `cur` is the two-limb number "remainder so far, then this limb".
        let cur = (rem << 32) | mag[i] as u64;
        q[i] = (cur / d) as u32;
        rem = cur % d;
    }
    normalize(&mut q);
    (q, rem as u32)
}

/// Shift a magnitude **left** by `bits` (0..32). May grow by one limb. Used to
/// "normalize the divisor" for Knuth's Algorithm D (see [`mag_divmod`]).
fn shl_small(a: &[u32], bits: u32) -> Vec<u32> {
    if bits == 0 {
        return a.to_vec();
    }
    let mut result = Vec::with_capacity(a.len() + 1);
    let mut carry: u32 = 0;
    for &limb in a {
        let v = ((limb as u64) << bits) | carry as u64;
        result.push(v as u32);
        carry = (v >> 32) as u32;
    }
    if carry != 0 {
        result.push(carry);
    }
    result
}

/// Shift a magnitude **right** by `bits` (0..32). Result normalized. Used to undo the
/// divisor-normalization shift when recovering the remainder in [`mag_divmod`].
fn shr_small(a: &[u32], bits: u32) -> Vec<u32> {
    if bits == 0 {
        let mut r = a.to_vec();
        normalize(&mut r);
        return r;
    }
    let mut result = vec![0u32; a.len()];
    let mut carry: u32 = 0; // bits that fell off the bottom of the limb above
    for i in (0..a.len()).rev() {
        let cur = a[i];
        result[i] = (cur >> bits) | carry;
        carry = cur << (32 - bits);
    }
    normalize(&mut result);
    result
}

/// Divide magnitudes: `u_in / v_in`, returning `(quotient, remainder)`, both normalized.
///
/// `v_in` must be normalized, non-empty, and non-zero. This is the heart of the crate:
/// **Knuth's Algorithm D** (TAOCP Vol. 2, §4.3.1), the classical long-division method
/// generalized from base 10 to base `2^32`.
///
/// ### Why long division on limbs is subtle
///
/// On paper you eyeball each quotient digit. In base `2^32` we cannot eyeball; we must
/// *estimate* each quotient limb `qhat` from the top two limbs of the running dividend
/// and the top limb of the divisor. Knuth proves that if the divisor's top limb has its
/// high bit set (is `>= base/2`), this estimate is either exactly right or too big by at
/// most **one** — never too big by two. So:
///
/// 1. **D1 (normalize):** left-shift both `u` and `v` by the divisor's leading-zero
///    count so `v`'s top limb has its high bit set. This does not change the quotient
///    (we shift the remainder back at the end).
/// 2. **D3 (estimate):** compute `qhat = (top two limbs of u) / (top limb of v)` and
///    refine it with a second check against the next limb, so it is at most 1 too big.
/// 3. **D4 (multiply & subtract):** subtract `qhat · v` from the current window of `u`.
/// 4. **D5/D6 (correct):** if that subtraction went negative, `qhat` was 1 too big —
///    decrement it and **add `v` back** once. This is the famous correction step.
fn mag_divmod(u_in: &[u32], v_in: &[u32]) -> (Vec<u32>, Vec<u32>) {
    debug_assert!(!v_in.is_empty(), "divisor magnitude must be non-empty");

    // If the dividend is smaller than the divisor, the quotient is 0 and the whole
    // dividend is the remainder.
    if mag_cmp(u_in, v_in) == Ordering::Less {
        let mut r = u_in.to_vec();
        normalize(&mut r);
        return (Vec::new(), r);
    }

    let n = v_in.len();

    // ---- Fast path: single-limb divisor is just `mag_divmod_small`. ----
    if n == 1 {
        let (q, rem) = mag_divmod_small(u_in, v_in[0]);
        let mut r = vec![rem];
        normalize(&mut r);
        return (q, r);
    }

    // ---- General case: Knuth Algorithm D, n >= 2. ----
    let base = 1u64 << 32;

    // D1: normalize so the divisor's top limb has its high bit set.
    let shift = v_in[n - 1].leading_zeros();
    let v = shl_small(v_in, shift); // stays length n (see shl_small reasoning)
    let mut u = shl_small(u_in, shift);
    // The working dividend needs exactly m+n+1 limbs (one extra guard limb on top).
    u.resize(u_in.len() + 1, 0);
    let m = u_in.len() - n;

    let mut q = vec![0u32; m + 1];

    // D2: loop over quotient limbs from most significant (j = m) down to 0.
    for j in (0..=m).rev() {
        // D3: estimate this quotient limb from the top two limbs of the window.
        let dividend = ((u[j + n] as u64) << 32) | u[j + n - 1] as u64;
        let mut qhat = dividend / v[n - 1] as u64;
        let mut rhat = dividend % v[n - 1] as u64;
        // Refine: while qhat is provably too big, drop it by one. At most two passes.
        loop {
            if qhat >= base || qhat * v[n - 2] as u64 > (rhat << 32) + u[j + n - 2] as u64 {
                qhat -= 1;
                rhat += v[n - 1] as u64;
                if rhat < base {
                    continue; // rhat still fits a limb — the refinement stays meaningful
                }
            }
            break;
        }

        // D4: multiply-and-subtract  u[j..=j+n] -= qhat * v.
        // `k` is a signed running borrow-plus-carry (Hacker's Delight formulation).
        let mut k: i64 = 0;
        for i in 0..n {
            let p = qhat * v[i] as u64; // fits u64: both factors < 2^32
            let t = u[j + i] as i64 - k - (p & 0xFFFF_FFFF) as i64;
            u[j + i] = t as u32; // wrap keeps the low 32 bits, exactly as intended
            k = (p >> 32) as i64 - (t >> 32);
        }
        let t = u[j + n] as i64 - k;
        u[j + n] = t as u32;

        if t < 0 {
            // D5/D6: qhat was one too large. Fix the quotient limb and add v back once.
            qhat -= 1;
            let mut carry: u64 = 0;
            for i in 0..n {
                let sum = u[j + i] as u64 + v[i] as u64 + carry;
                u[j + i] = sum as u32;
                carry = sum >> 32;
            }
            // This final carry cancels the earlier over-borrow; its overflow is discarded.
            u[j + n] = (u[j + n] as u64 + carry) as u32;
        }

        q[j] = qhat as u32;
    }

    normalize(&mut q);
    // D8: the remainder sits in the low n limbs of u, still shifted left by `shift`.
    let rem = shr_small(&u[0..n], shift);
    (q, rem)
}

// ===========================================================================
//  BigInteger — construction
// ===========================================================================

impl BigInteger {
    /// The value `0` in canonical form (`Zero` sign, empty magnitude).
    ///
    /// ```
    /// # use bignum_core::BigInteger;
    /// assert_eq!(BigInteger::zero().to_string(), "0");
    /// assert!(BigInteger::zero().is_zero());
    /// ```
    pub fn zero() -> Self {
        BigInteger {
            sign: Sign::Zero,
            mag: Vec::new(),
        }
    }

    /// The value `1`.
    pub fn one() -> Self {
        BigInteger::from_u64(1)
    }

    /// Build a `BigInteger` from a sign and an *already-normalized* magnitude.
    ///
    /// If the magnitude is empty, the result is canonical zero regardless of the sign
    /// passed in — this is the single funnel through which every arithmetic result
    /// flows, guaranteeing "no `-0`".
    fn from_parts(sign: Sign, mag: Vec<u32>) -> BigInteger {
        if mag.is_empty() {
            BigInteger::zero()
        } else {
            BigInteger { sign, mag }
        }
    }

    /// Construct from an unsigned 128-bit integer (covers `u8`..`u128` via `into`).
    pub fn from_u128(value: u128) -> Self {
        if value == 0 {
            return BigInteger::zero();
        }
        let mut mag = Vec::new();
        let mut x = value;
        while x != 0 {
            mag.push(x as u32);
            x >>= 32;
        }
        BigInteger {
            sign: Sign::Plus,
            mag,
        }
    }

    /// Construct from a signed 128-bit integer.
    ///
    /// Uses `unsigned_abs` so that even `i128::MIN` (whose negation overflows `i128`)
    /// is handled without panicking.
    pub fn from_i128(value: i128) -> Self {
        if value == 0 {
            return BigInteger::zero();
        }
        let sign = if value < 0 { Sign::Minus } else { Sign::Plus };
        let mut mag = Vec::new();
        let mut x = value.unsigned_abs();
        while x != 0 {
            mag.push(x as u32);
            x >>= 32;
        }
        BigInteger { sign, mag }
    }

    /// Construct from an unsigned 64-bit integer.
    pub fn from_u64(value: u64) -> Self {
        BigInteger::from_u128(value as u128)
    }

    /// Construct from a signed 64-bit integer (handles `i64::MIN` correctly).
    pub fn from_i64(value: i64) -> Self {
        BigInteger::from_i128(value as i128)
    }
}

// ===========================================================================
//  BigInteger — queries
// ===========================================================================

impl BigInteger {
    /// Is this exactly zero?
    pub fn is_zero(&self) -> bool {
        self.sign == Sign::Zero
    }

    /// Is this strictly negative?
    pub fn is_negative(&self) -> bool {
        self.sign == Sign::Minus
    }

    /// Is this strictly positive?
    pub fn is_positive(&self) -> bool {
        self.sign == Sign::Plus
    }

    /// The sign as an `i32`: `-1`, `0`, or `+1` (matching `i64::signum`'s convention).
    pub fn signum(&self) -> i32 {
        match self.sign {
            Sign::Minus => -1,
            Sign::Zero => 0,
            Sign::Plus => 1,
        }
    }

    /// The number of base-`2^32` limbs in the magnitude (`0` for zero).
    pub fn num_limbs(&self) -> usize {
        self.mag.len()
    }

    /// The number of bits in the magnitude — i.e. `floor(log2(|self|)) + 1`, and `0`
    /// for zero. `bit_len(255) == 8`, `bit_len(256) == 9`.
    pub fn bit_len(&self) -> u64 {
        match self.mag.last() {
            None => 0,
            Some(&top) => {
                // All lower limbs contribute a full 32 bits; the top limb contributes
                // however many bits it actually uses.
                (self.mag.len() as u64 - 1) * 32 + (32 - top.leading_zeros() as u64)
            }
        }
    }
}

// ===========================================================================
//  BigInteger — sign transforms (used by both the public API and the ops traits)
// ===========================================================================

impl BigInteger {
    /// Return `-self`. Zero negates to itself (never `-0`).
    fn negated(&self) -> BigInteger {
        match self.sign {
            Sign::Zero => BigInteger::zero(),
            Sign::Plus => BigInteger {
                sign: Sign::Minus,
                mag: self.mag.clone(),
            },
            Sign::Minus => BigInteger {
                sign: Sign::Plus,
                mag: self.mag.clone(),
            },
        }
    }

    /// Return `|self|` (the absolute value), always non-negative.
    ///
    /// ```
    /// # use bignum_core::BigInteger;
    /// assert_eq!(BigInteger::from_i64(-42).abs(), BigInteger::from_i64(42));
    /// ```
    pub fn abs(&self) -> BigInteger {
        match self.sign {
            Sign::Minus => BigInteger {
                sign: Sign::Plus,
                mag: self.mag.clone(),
            },
            _ => self.clone(),
        }
    }
}

// ===========================================================================
//  BigInteger — core arithmetic (private engines)
// ===========================================================================
//
// The real logic lives in these private methods with distinctive names. Both the
// public inherent methods (`add`, `sub`, ...) and the `std::ops` trait impls delegate
// here, so there is never any ambiguity about which "add" is meant.

impl BigInteger {
    /// Signed addition. Combines the grade-school magnitude rules with sign logic:
    /// like signs add magnitudes and keep the sign; unlike signs subtract the smaller
    /// magnitude from the larger and take the larger's sign (equal magnitudes cancel).
    fn combine_add(&self, other: &BigInteger) -> BigInteger {
        use Sign::{Minus, Plus, Zero};
        match (self.sign, other.sign) {
            (Zero, _) => other.clone(),
            (_, Zero) => self.clone(),
            (Plus, Plus) => BigInteger::from_parts(Plus, mag_add(&self.mag, &other.mag)),
            (Minus, Minus) => BigInteger::from_parts(Minus, mag_add(&self.mag, &other.mag)),
            // Opposite signs: this is really a subtraction of magnitudes.
            (Plus, Minus) | (Minus, Plus) => match mag_cmp(&self.mag, &other.mag) {
                Ordering::Equal => BigInteger::zero(),
                Ordering::Greater => {
                    BigInteger::from_parts(self.sign, mag_sub(&self.mag, &other.mag))
                }
                Ordering::Less => {
                    BigInteger::from_parts(other.sign, mag_sub(&other.mag, &self.mag))
                }
            },
        }
    }

    /// Signed subtraction, defined as `self + (-other)`.
    fn combine_sub(&self, other: &BigInteger) -> BigInteger {
        self.combine_add(&other.negated())
    }

    /// Signed multiplication. Magnitudes multiply; the sign follows the rule of signs
    /// (same signs → positive, different → negative), with zero absorbing everything.
    fn multiply(&self, other: &BigInteger) -> BigInteger {
        if self.is_zero() || other.is_zero() {
            return BigInteger::zero();
        }
        let sign = if self.sign == other.sign {
            Sign::Plus
        } else {
            Sign::Minus
        };
        BigInteger::from_parts(sign, mag_mul(&self.mag, &other.mag))
    }

    /// Truncating division, returning `(quotient, remainder)`.
    ///
    /// Semantics match Rust's built-in `/` and `%` on integers exactly:
    /// the quotient **truncates toward zero**, and the remainder takes the **dividend's
    /// sign**, so that the identity `dividend == quotient · divisor + remainder` always
    /// holds with `|remainder| < |divisor|`.
    ///
    /// # Panics
    /// Panics if `other` is zero, mirroring integer division-by-zero in Rust.
    fn divide_rem(&self, other: &BigInteger) -> (BigInteger, BigInteger) {
        if other.is_zero() {
            panic!("BigInteger division by zero");
        }
        if self.is_zero() {
            return (BigInteger::zero(), BigInteger::zero());
        }
        let (qmag, rmag) = mag_divmod(&self.mag, &other.mag);
        // Rule of signs for the quotient; the remainder always carries the dividend's sign.
        let qsign = if self.sign == other.sign {
            Sign::Plus
        } else {
            Sign::Minus
        };
        let quotient = BigInteger::from_parts(qsign, qmag);
        let remainder = BigInteger::from_parts(self.sign, rmag);
        (quotient, remainder)
    }
}

// ===========================================================================
//  BigInteger — public arithmetic API (inherent methods)
// ===========================================================================

impl BigInteger {
    /// `self + other`.
    pub fn add(&self, other: &BigInteger) -> BigInteger {
        self.combine_add(other)
    }

    /// `self - other`.
    pub fn sub(&self, other: &BigInteger) -> BigInteger {
        self.combine_sub(other)
    }

    /// `self * other`.
    pub fn mul(&self, other: &BigInteger) -> BigInteger {
        self.multiply(other)
    }

    /// Truncating division returning `(quotient, remainder)`. See [`Self::divide_rem`]
    /// for the exact semantics.
    ///
    /// # Panics
    /// Panics if `other` is zero.
    pub fn div_rem(&self, other: &BigInteger) -> (BigInteger, BigInteger) {
        self.divide_rem(other)
    }

    /// The truncating quotient `self / other`.
    ///
    /// # Panics
    /// Panics if `other` is zero.
    pub fn div(&self, other: &BigInteger) -> BigInteger {
        self.divide_rem(other).0
    }

    /// The remainder `self % other` (takes the sign of `self`).
    ///
    /// # Panics
    /// Panics if `other` is zero.
    pub fn rem(&self, other: &BigInteger) -> BigInteger {
        self.divide_rem(other).1
    }

    /// Raise to a non-negative integer power via **exponentiation by squaring**.
    ///
    /// Instead of `exp` multiplications, this needs only `O(log exp)` of them: it walks
    /// the bits of the exponent, squaring a running base each step and folding it into
    /// the result whenever the current bit is set. By convention `x.pow(0) == 1` for
    /// every `x`, including zero.
    ///
    /// ```
    /// # use bignum_core::BigInteger;
    /// assert_eq!(BigInteger::from_u64(3).pow(4), BigInteger::from_u64(81));
    /// assert_eq!(BigInteger::from_u64(10).pow(0), BigInteger::one());
    /// ```
    ///
    /// ## Unbounded result — do not call with an untrusted exponent
    ///
    /// The result grows LINEARLY in `exp`: `bit_len(baseᵉ) ≈ bit_len(base) · exp`. A
    /// large `exp` (still only a small `u32`) therefore asks for an arbitrarily large
    /// number — `2.pow(4_000_000_000)` targets ~4 gigabits and exhausts memory. When
    /// `exp` may come from untrusted input (a document, a model), use
    /// [`try_pow`](Self::try_pow), which refuses an oversized result up front instead
    /// of OOMing.
    pub fn pow(&self, exp: u32) -> BigInteger {
        let mut result = BigInteger::one();
        let mut base = self.clone();
        let mut e = exp;
        while e > 0 {
            if e & 1 == 1 {
                result = result.multiply(&base);
            }
            e >>= 1;
            if e > 0 {
                base = base.multiply(&base);
            }
        }
        result
    }

    /// [`pow`](Self::pow) with a size guard — the safe form for an exponent that may
    /// come from untrusted input.
    ///
    /// Because `bit_len(baseᵉ) ≤ bit_len(base) · exp`, the result's size is known in
    /// O(1) *before* any work is done. If that upper bound exceeds `max_bits`, this
    /// returns [`PowTooLargeError`] immediately — no allocation, no squaring — so a
    /// hostile `2 ^ 4_000_000_000` is a clean typed error, not an out-of-memory abort.
    /// Otherwise it computes `self.pow(exp)`. Exponentiation-by-squaring's largest
    /// intermediate is the result itself, so bounding the final size bounds every
    /// step. A zero/one/-one base or `exp == 0` never grows and always succeeds.
    ///
    /// ```
    /// # use bignum_core::BigInteger;
    /// // Fits: 3^40 is ~64 bits, well under the ceiling.
    /// assert!(BigInteger::from_u64(3).try_pow(40, 4096).is_ok());
    /// // Refused up front: 2^1_000_000 would be ~1e6 bits, over the 4096-bit cap.
    /// assert!(BigInteger::from_u64(2).try_pow(1_000_000, 4096).is_err());
    /// ```
    pub fn try_pow(&self, exp: u32, max_bits: u64) -> Result<BigInteger, PowTooLargeError> {
        // Bases that never grow: 0, ±1, or exp 0 → the result is 0 or ±1 (≤ 1 bit).
        let projected = if exp == 0 || self.is_zero() || self.bit_len() <= 1 {
            1
        } else {
            // Upper bound on the result's bit length. `saturating_mul` keeps a huge
            // exponent from wrapping the projection (which would defeat the guard).
            self.bit_len().saturating_mul(exp as u64)
        };
        if projected > max_bits {
            return Err(PowTooLargeError {
                projected_bits: projected,
                max_bits,
            });
        }
        Ok(self.pow(exp))
    }

    /// The greatest common divisor of `self` and `other`, always **non-negative**,
    /// computed by the **Euclidean algorithm**: repeatedly replace `(a, b)` with
    /// `(b, a mod b)` until `b` is zero; the surviving `a` is the gcd. Signs are
    /// stripped up front, so `gcd(-12, 18) == 6`. `gcd(0, 0) == 0`.
    ///
    /// ```
    /// # use bignum_core::BigInteger;
    /// let g = BigInteger::from_u64(48).gcd(&BigInteger::from_u64(36));
    /// assert_eq!(g, BigInteger::from_u64(12));
    /// ```
    pub fn gcd(&self, other: &BigInteger) -> BigInteger {
        let mut a = self.abs();
        let mut b = other.abs();
        while !b.is_zero() {
            // `a` and `b` are non-negative here, so the remainder is non-negative too.
            let r = a.divide_rem(&b).1;
            a = b;
            b = r;
        }
        a
    }
}

// ===========================================================================
//  BigInteger — parsing and formatting
// ===========================================================================

impl BigInteger {
    /// Parse a string in the given `radix` (`2..=36`) into a `BigInteger`.
    ///
    /// Accepts an optional leading `+` or `-`. Digits are `0-9` then `a-z`/`A-Z`
    /// (case-insensitive) up to the radix. Returns a typed [`ParseBigIntError`] — never
    /// panics — on an empty string, a sign with no digits, an invalid digit, or an
    /// out-of-range radix. `"0"`, `"-0"`, and `"+000"` all parse to canonical zero.
    ///
    /// ```
    /// # use bignum_core::BigInteger;
    /// assert_eq!(BigInteger::parse_radix("ff", 16).unwrap(), BigInteger::from_u64(255));
    /// assert_eq!(BigInteger::parse_radix("-1010", 2).unwrap(), BigInteger::from_i64(-10));
    /// assert!(BigInteger::parse_radix("", 10).is_err());
    /// ```
    pub fn parse_radix(s: &str, radix: u32) -> Result<BigInteger, ParseBigIntError> {
        if !(2..=36).contains(&radix) {
            return Err(ParseBigIntError::InvalidRadix(radix));
        }
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            return Err(ParseBigIntError::Empty);
        }

        // Optional leading sign.
        let mut start = 0;
        let mut negative = false;
        match bytes[0] {
            b'+' => start = 1,
            b'-' => {
                negative = true;
                start = 1;
            }
            _ => {}
        }
        if start >= bytes.len() {
            // A lone "+" or "-" has no digits.
            return Err(ParseBigIntError::Empty);
        }

        // Horner accumulation: mag = mag * radix + digit, one character at a time.
        let mut mag: Vec<u32> = Vec::new();
        for &byte in &bytes[start..] {
            let c = byte as char;
            let digit = c.to_digit(radix).ok_or(ParseBigIntError::InvalidDigit(c))?;
            mag = mag_mul_small(&mag, radix);
            mag = mag_add_small(&mag, digit);
        }
        normalize(&mut mag);

        if mag.is_empty() {
            // All digits were zero — canonical zero, sign discarded ("-0" is 0).
            return Ok(BigInteger::zero());
        }
        let sign = if negative { Sign::Minus } else { Sign::Plus };
        Ok(BigInteger { sign, mag })
    }

    /// Render the value in the given `radix` (`2..=36`), lowercase, with a leading `-`
    /// for negatives. Zero renders as `"0"`.
    ///
    /// The algorithm is repeated division: divide the magnitude by the radix, collect
    /// each remainder as a digit (least-significant first), then reverse.
    ///
    /// # Panics
    /// Panics if `radix` is outside `2..=36`.
    ///
    /// ```
    /// # use bignum_core::BigInteger;
    /// assert_eq!(BigInteger::from_u64(255).to_str_radix(16), "ff");
    /// assert_eq!(BigInteger::from_i64(-42).to_str_radix(2), "-101010");
    /// ```
    pub fn to_str_radix(&self, radix: u32) -> String {
        assert!(
            (2..=36).contains(&radix),
            "radix must be in 2..=36, got {radix}"
        );
        if self.is_zero() {
            return "0".to_string();
        }
        let mut digits: Vec<char> = Vec::new();
        let mut mag = self.mag.clone();
        while !mag.is_empty() {
            let (q, rem) = mag_divmod_small(&mag, radix);
            // `from_digit` maps 0..36 to '0'..'9','a'..'z'; rem < radix so it is Some.
            digits.push(std::char::from_digit(rem, radix).expect("remainder < radix"));
            mag = q;
        }
        let mut s = String::with_capacity(digits.len() + 1);
        if self.sign == Sign::Minus {
            s.push('-');
        }
        for &c in digits.iter().rev() {
            s.push(c);
        }
        s
    }
}

// ===========================================================================
//  Trait implementations
// ===========================================================================

impl fmt::Display for BigInteger {
    /// Base-10 rendering (the everyday form).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_str_radix(10))
    }
}

impl fmt::Debug for BigInteger {
    /// A readable debug form, e.g. `BigInteger(-123)`, rather than raw limbs.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BigInteger({self})")
    }
}

impl FromStr for BigInteger {
    type Err = ParseBigIntError;
    /// Parse a base-10 string (see [`BigInteger::parse_radix`]).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        BigInteger::parse_radix(s, 10)
    }
}

impl Ord for BigInteger {
    fn cmp(&self, other: &Self) -> Ordering {
        use Sign::{Minus, Plus, Zero};
        match (self.sign, other.sign) {
            (Zero, Zero) => Ordering::Equal,
            // Anything on the left that is "more negative" than the right is Less.
            (Zero, Plus) | (Minus, Zero) | (Minus, Plus) => Ordering::Less,
            (Zero, Minus) | (Plus, Zero) | (Plus, Minus) => Ordering::Greater,
            (Plus, Plus) => mag_cmp(&self.mag, &other.mag),
            // Both negative: the one with the *larger* magnitude is the *smaller* value.
            (Minus, Minus) => mag_cmp(&other.mag, &self.mag),
        }
    }
}

impl PartialOrd for BigInteger {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// --- From conversions for ergonomics (all lossless) ---

impl From<i64> for BigInteger {
    fn from(v: i64) -> Self {
        BigInteger::from_i64(v)
    }
}
impl From<u64> for BigInteger {
    fn from(v: u64) -> Self {
        BigInteger::from_u64(v)
    }
}
impl From<i128> for BigInteger {
    fn from(v: i128) -> Self {
        BigInteger::from_i128(v)
    }
}
impl From<u128> for BigInteger {
    fn from(v: u128) -> Self {
        BigInteger::from_u128(v)
    }
}

// --- Operator overloading. Each op has an owned form (`a + b`) and a borrowed form
//     (`&a + &b`, which avoids cloning large operands). All delegate to the private
//     engines above. ---

macro_rules! impl_binop {
    ($trait:ident, $method:ident, $engine:ident) => {
        impl std::ops::$trait for BigInteger {
            type Output = BigInteger;
            fn $method(self, rhs: BigInteger) -> BigInteger {
                self.$engine(&rhs)
            }
        }
        impl std::ops::$trait<&BigInteger> for &BigInteger {
            type Output = BigInteger;
            fn $method(self, rhs: &BigInteger) -> BigInteger {
                self.$engine(rhs)
            }
        }
    };
}

impl_binop!(Add, add, combine_add);
impl_binop!(Sub, sub, combine_sub);
impl_binop!(Mul, mul, multiply);

// Div and Rem return only one half of `divide_rem`, so they get bespoke impls.
impl std::ops::Div for BigInteger {
    type Output = BigInteger;
    fn div(self, rhs: BigInteger) -> BigInteger {
        self.divide_rem(&rhs).0
    }
}
impl std::ops::Div<&BigInteger> for &BigInteger {
    type Output = BigInteger;
    fn div(self, rhs: &BigInteger) -> BigInteger {
        self.divide_rem(rhs).0
    }
}
impl std::ops::Rem for BigInteger {
    type Output = BigInteger;
    fn rem(self, rhs: BigInteger) -> BigInteger {
        self.divide_rem(&rhs).1
    }
}
impl std::ops::Rem<&BigInteger> for &BigInteger {
    type Output = BigInteger;
    fn rem(self, rhs: &BigInteger) -> BigInteger {
        self.divide_rem(rhs).1
    }
}

impl std::ops::Neg for BigInteger {
    type Output = BigInteger;
    fn neg(self) -> BigInteger {
        self.negated()
    }
}
impl std::ops::Neg for &BigInteger {
    type Output = BigInteger;
    fn neg(self) -> BigInteger {
        self.negated()
    }
}

// ===========================================================================
//  Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    //  A tiny deterministic pseudo-random generator.
    //
    //  Tests must be reproducible and must NOT use `rand`/time (unavailable and
    //  non-deterministic). This is a plain linear-congruential generator (the classic
    //  Numerical-Recipes / PCG multiplier and increment) with an output mix so
    //  consecutive draws are well spread. Same seed → same stream, forever.
    // -----------------------------------------------------------------------
    struct Lcg {
        state: u64,
    }
    impl Lcg {
        fn new(seed: u64) -> Self {
            Lcg { state: seed }
        }
        fn next_u64(&mut self) -> u64 {
            self.state = self
                .state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let x = self.state;
            // xorshift-multiply output mixing (splitmix64-style finalizer)
            let x = (x ^ (x >> 31)).wrapping_mul(0x2545_F491_4F6C_DD1D);
            x ^ (x >> 29)
        }
        /// A signed value with magnitude in `[0, 2^bits)` and a random sign.
        /// With `bits <= 60`, products of two draws stay well inside `i128`.
        fn next_i128(&mut self, bits: u32) -> i128 {
            let hi = self.next_u64() as u128;
            let lo = self.next_u64() as u128;
            let mask = (1u128 << bits) - 1;
            let mag = (((hi << 64) | lo) & mask) as i128;
            if self.next_u64() & 1 == 1 {
                -mag
            } else {
                mag
            }
        }
    }

    fn big(v: i128) -> BigInteger {
        BigInteger::from_i128(v)
    }

    /// A hand-picked table of "interesting" values: zero, ±1, powers of two around the
    /// 32/64/96-bit limb boundaries (where carries and limb counts change), and a few
    /// odd primes. Every differential test sweeps all ordered pairs from this table.
    fn interesting_values() -> Vec<i128> {
        let mut v = vec![
            0i128,
            1,
            -1,
            2,
            -2,
            10,
            -10,
            255,
            256,
            65535,
            65536,
            i64::MAX as i128,
            i64::MIN as i128,
            u64::MAX as i128,
            1_000_003,       // a prime
            999_999_937,     // a prime near 10^9
            2_147_483_647,   // 2^31 - 1 (Mersenne prime)
            4_294_967_295,   // 2^32 - 1  (top of one limb)
            4_294_967_296,   // 2^32      (first two-limb value)
            4_294_967_297,   // 2^32 + 1
        ];
        // Powers of two straddling each limb boundary.
        for &p in &[31u32, 32, 33, 63, 64, 65, 95, 96, 97, 120] {
            let val = 1i128 << p;
            v.push(val);
            v.push(val - 1);
            v.push(val + 1);
            v.push(-val);
        }
        v
    }

    // ==== Construction & round-trip ====

    #[test]
    fn zero_is_canonical() {
        let z = BigInteger::zero();
        assert!(z.is_zero());
        assert_eq!(z.signum(), 0);
        assert_eq!(z.num_limbs(), 0);
        assert_eq!(z.bit_len(), 0);
        assert_eq!(z.to_string(), "0");
        // Every route to zero yields the *same* canonical value.
        assert_eq!(z, big(0));
        assert_eq!(z, BigInteger::from_i64(5).sub(&BigInteger::from_i64(5)));
        assert_eq!(z, BigInteger::parse_radix("-0", 10).unwrap());
        assert_eq!(z, BigInteger::parse_radix("+000", 10).unwrap());
        assert_eq!(z, -BigInteger::zero());
    }

    #[test]
    fn constructors_agree() {
        assert_eq!(BigInteger::from_u64(255).to_string(), "255");
        assert_eq!(BigInteger::from_i64(-255).to_string(), "-255");
        assert_eq!(BigInteger::from_u128(u128::MAX).to_string(), u128::MAX.to_string());
        assert_eq!(BigInteger::from_i128(i128::MIN).to_string(), i128::MIN.to_string());
        assert_eq!(BigInteger::from_i128(i128::MAX).to_string(), i128::MAX.to_string());
        assert_eq!(BigInteger::from_i64(i64::MIN).to_string(), i64::MIN.to_string());
        assert_eq!(BigInteger::one().to_string(), "1");
        // From<..> conversions.
        assert_eq!(BigInteger::from(-7i64), big(-7));
        assert_eq!(BigInteger::from(7u64), big(7));
        assert_eq!(BigInteger::from(-7i128), big(-7));
        assert_eq!(BigInteger::from(7u128), big(7));
    }

    #[test]
    fn queries_report_correctly() {
        let n = BigInteger::from_i64(-42);
        assert!(n.is_negative());
        assert!(!n.is_positive());
        assert_eq!(n.signum(), -1);
        assert_eq!(n.abs().to_string(), "42");

        let p = BigInteger::from_u64(42);
        assert!(p.is_positive());
        assert_eq!(p.signum(), 1);

        // bit_len boundaries.
        assert_eq!(BigInteger::from_u64(255).bit_len(), 8);
        assert_eq!(BigInteger::from_u64(256).bit_len(), 9);
        assert_eq!(BigInteger::from_u64(1).bit_len(), 1);
        assert_eq!(BigInteger::from_u64(u64::MAX).bit_len(), 64);
        assert_eq!(BigInteger::from_u64(2).pow(96).bit_len(), 97);
        assert_eq!(BigInteger::from_u64(2).pow(96).num_limbs(), 4);
    }

    // ==== Differential vs i128: the hand table ====

    #[test]
    fn differential_table_add_sub_mul_cmp() {
        let vals = interesting_values();
        for &a in &vals {
            for &b in &vals {
                let (ba, bb) = (big(a), big(b));

                // Comparison must always match.
                assert_eq!(ba.cmp(&bb), a.cmp(&b), "cmp {a} ? {b}");

                if let Some(s) = a.checked_add(b) {
                    assert_eq!(ba.add(&bb), big(s), "{a} + {b}");
                }
                if let Some(s) = a.checked_sub(b) {
                    assert_eq!(ba.sub(&bb), big(s), "{a} - {b}");
                }
                if let Some(p) = a.checked_mul(b) {
                    assert_eq!(ba.mul(&bb), big(p), "{a} * {b}");
                }
            }
        }
    }

    #[test]
    fn differential_table_div_rem() {
        let vals = interesting_values();
        for &a in &vals {
            for &b in &vals {
                if b == 0 {
                    continue;
                }
                // Skip the single i128 case that itself overflows (MIN / -1).
                if a == i128::MIN && b == -1 {
                    continue;
                }
                let (ba, bb) = (big(a), big(b));
                let (q, r) = ba.div_rem(&bb);
                assert_eq!(q, big(a / b), "{a} / {b} quotient");
                assert_eq!(r, big(a % b), "{a} % {b} remainder");
                assert_eq!(ba.rem(&bb), big(a % b), "{a} rem {b}");

                // The defining identity: a == q*b + r, with |r| < |b|.
                let recon = &(&q * &bb) + &r;
                assert_eq!(recon, ba, "reconstruct {a} from q,b,r");
                assert!(r.abs() < bb.abs(), "|r| < |b| for {a} / {b}");
            }
        }
    }

    // ==== Differential vs i128: LCG-generated breadth ====

    #[test]
    fn differential_random_arithmetic() {
        let mut rng = Lcg::new(0x00C0_FFEE_1234_5678);
        for _ in 0..20_000 {
            // 60-bit magnitudes → products < 2^120, safely inside i128.
            let a = rng.next_i128(60);
            let b = rng.next_i128(60);
            let (ba, bb) = (big(a), big(b));

            assert_eq!(ba.cmp(&bb), a.cmp(&b));
            assert_eq!(ba.add(&bb), big(a + b));
            assert_eq!(ba.sub(&bb), big(a - b));
            assert_eq!(ba.mul(&bb), big(a * b));

            if b != 0 {
                let (q, r) = ba.div_rem(&bb);
                assert_eq!(q, big(a / b), "rng {a} / {b}");
                assert_eq!(r, big(a % b), "rng {a} % {b}");
                let recon = &(&q * &bb) + &r;
                assert_eq!(recon, ba);
                assert!(r.abs() < bb.abs());
            }
        }
    }

    #[test]
    fn differential_random_small_divisor() {
        // Bias toward small divisors to exercise the single-limb division fast path
        // alongside large multi-limb dividends.
        let mut rng = Lcg::new(0xABCD_EF01);
        for _ in 0..10_000 {
            let a = rng.next_i128(100);
            let b = rng.next_i128(20);
            if b == 0 || (a == i128::MIN && b == -1) {
                continue;
            }
            let (ba, bb) = (big(a), big(b));
            let (q, r) = ba.div_rem(&bb);
            assert_eq!(q, big(a / b), "small-div {a} / {b}");
            assert_eq!(r, big(a % b), "small-div {a} % {b}");
        }
    }

    // ==== Known-big values beyond i128 ====

    fn factorial(n: u64) -> BigInteger {
        let mut acc = BigInteger::one();
        for i in 2..=n {
            acc = &acc * &BigInteger::from_u64(i);
        }
        acc
    }

    #[test]
    fn factorials_match_known_decimals() {
        // 50! and 100! — famous exact values, far beyond any machine integer.
        // This cross-checks big multiplication AND decimal formatting at once.
        assert_eq!(
            factorial(50).to_string(),
            "30414093201713378043612608166064768844377641568960512000000000000"
        );
        assert_eq!(
            factorial(100).to_string(),
            "9332621544394415268169923885626670049071596826438162146859296389\
5217599993229915608941463976156518286253697920827223758251185210916864\
000000000000000000000000"
        );
    }

    #[test]
    fn powers_beyond_i128() {
        assert_eq!(
            BigInteger::from_u64(2).pow(128).to_string(),
            "340282366920938463463374607431768211456"
        );
        assert_eq!(
            BigInteger::from_u64(10).pow(50).to_string(),
            "100000000000000000000000000000000000000000000000000"
        );
        // (-2)^7 = -128, (-2)^8 = 256: sign of a power follows parity of the exponent.
        assert_eq!(BigInteger::from_i64(-2).pow(7), big(-128));
        assert_eq!(BigInteger::from_i64(-2).pow(8), big(256));
        // Anything to the 0th power is 1, including zero.
        assert_eq!(BigInteger::zero().pow(0), BigInteger::one());
        assert_eq!(BigInteger::zero().pow(5), BigInteger::zero());
    }

    #[test]
    fn parse_huge_then_reformat_roundtrips() {
        let s = "9332621544394415268169923885626670049071596826438162146859296389\
5217599993229915608941463976156518286253697920827223758251185210916864\
000000000000000000000000";
        let parsed = BigInteger::from_str(s).unwrap();
        assert_eq!(parsed.to_string(), s);
        assert_eq!(parsed, factorial(100));

        let neg = format!("-{s}");
        let parsed_neg = BigInteger::from_str(&neg).unwrap();
        assert_eq!(parsed_neg.to_string(), neg);
        assert_eq!(parsed_neg, -&parsed);
    }

    #[test]
    fn consecutive_fibonaccis_are_coprime() {
        // A classic number-theory fact: gcd(F(n), F(n+1)) == 1 for all n. Exercises gcd
        // on genuinely large multi-limb operands.
        let mut a = BigInteger::zero();
        let mut b = BigInteger::one();
        for _ in 0..300 {
            let c = &a + &b;
            a = b;
            b = c;
        }
        assert!(b.bit_len() > 128, "F(n) should be huge by now");
        assert_eq!(a.gcd(&b), BigInteger::one());

        // A gcd with a known non-trivial answer, and sign-insensitivity.
        assert_eq!(
            BigInteger::from_u64(48).gcd(&BigInteger::from_i64(-36)),
            BigInteger::from_u64(12)
        );
        assert_eq!(
            BigInteger::from_i64(-48).gcd(&BigInteger::from_i64(-36)),
            BigInteger::from_u64(12)
        );
        assert_eq!(BigInteger::zero().gcd(&BigInteger::zero()), BigInteger::zero());
        assert_eq!(BigInteger::zero().gcd(&big(7)), big(7));
    }

    // ==== Radix round-tripping ====

    #[test]
    fn radix_roundtrip_all_bases() {
        let mut rng = Lcg::new(0x5EED_5EED);
        for _ in 0..2_000 {
            let v = rng.next_i128(110);
            let n = big(v);
            for radix in [2u32, 3, 8, 10, 16, 36] {
                let s = n.to_str_radix(radix);
                let back = BigInteger::parse_radix(&s, radix).unwrap();
                assert_eq!(back, n, "roundtrip {v} in base {radix} via {s:?}");
            }
        }
    }

    #[test]
    fn radix_known_renderings() {
        assert_eq!(BigInteger::from_u64(255).to_str_radix(16), "ff");
        assert_eq!(BigInteger::from_u64(255).to_str_radix(2), "11111111");
        assert_eq!(BigInteger::from_i64(-42).to_str_radix(2), "-101010");
        assert_eq!(BigInteger::from_u64(35).to_str_radix(36), "z");
        assert_eq!(BigInteger::zero().to_str_radix(16), "0");
        // Uppercase and lowercase digits both parse.
        assert_eq!(BigInteger::parse_radix("FF", 16).unwrap(), big(255));
        assert_eq!(BigInteger::parse_radix("ff", 16).unwrap(), big(255));
        assert_eq!(BigInteger::parse_radix("+7B", 16).unwrap(), big(123));
    }

    // ==== Parsing errors (never panics) ====

    #[test]
    fn parse_rejects_bad_input() {
        assert_eq!(BigInteger::from_str(""), Err(ParseBigIntError::Empty));
        assert_eq!(BigInteger::from_str("-"), Err(ParseBigIntError::Empty));
        assert_eq!(BigInteger::from_str("+"), Err(ParseBigIntError::Empty));
        assert_eq!(
            BigInteger::from_str("12x3"),
            Err(ParseBigIntError::InvalidDigit('x'))
        );
        assert_eq!(
            BigInteger::from_str("1 2"),
            Err(ParseBigIntError::InvalidDigit(' '))
        );
        // '2' is not a valid binary digit.
        assert_eq!(
            BigInteger::parse_radix("102", 2),
            Err(ParseBigIntError::InvalidDigit('2'))
        );
        assert_eq!(
            BigInteger::parse_radix("10", 1),
            Err(ParseBigIntError::InvalidRadix(1))
        );
        assert_eq!(
            BigInteger::parse_radix("10", 37),
            Err(ParseBigIntError::InvalidRadix(37))
        );
        // The error implements Display and Error.
        let e: &dyn std::error::Error = &ParseBigIntError::InvalidDigit('q');
        assert!(e.to_string().contains('q'));
        assert!(!ParseBigIntError::Empty.to_string().is_empty());
        assert!(ParseBigIntError::InvalidRadix(99).to_string().contains("99"));
    }

    // ==== Edge cases in arithmetic ====

    #[test]
    fn operations_with_zero() {
        let z = BigInteger::zero();
        let n = big(12345);
        assert_eq!(n.add(&z), n);
        assert_eq!(z.add(&n), n);
        assert_eq!(n.sub(&z), n);
        assert_eq!(z.sub(&n), -&n);
        assert_eq!(n.mul(&z), z);
        assert_eq!(z.mul(&n), z);
        assert_eq!(z.div_rem(&n), (BigInteger::zero(), BigInteger::zero()));
        // x - x is canonical zero.
        assert_eq!((&n - &n), z);
    }

    #[test]
    fn negatives_in_every_op() {
        let a = big(-123456789);
        let b = big(987654321);
        assert_eq!(&a + &b, big(-123456789 + 987654321));
        assert_eq!(&a - &b, big(-123456789 - 987654321));
        assert_eq!(&a * &b, big(-123456789i128 * 987654321));
        assert_eq!(&a / &b, big(-123456789 / 987654321));
        assert_eq!(&a % &b, big(-123456789)); // |a| < |b|, so a % b == a
        assert_eq!(-&a, big(123456789));
        assert_eq!(a.abs(), big(123456789));
        // Owned-form operators too.
        assert_eq!(big(-5) + big(3), big(-2));
        assert_eq!(big(-5) - big(3), big(-8));
        assert_eq!(big(-5) * big(3), big(-15));
        assert_eq!(big(-17) / big(5), big(-3));
        assert_eq!(big(-17) % big(5), big(-2));
        assert_eq!(-big(9), big(-9));
    }

    #[test]
    fn single_and_multi_limb_boundaries() {
        // Adding across the one-limb boundary produces a second limb.
        let max_limb = big((u32::MAX) as i128);
        assert_eq!(&max_limb + &big(1), big(1i128 << 32));
        assert_eq!((&max_limb + &big(1)).num_limbs(), 2);
        // Subtracting back collapses to one limb (normalization removes the top zero).
        assert_eq!(&big(1i128 << 32) - &big(1), max_limb);
        assert_eq!((&big(1i128 << 32) - &big(1)).num_limbs(), 1);
        // Multiplying two one-limb values that just barely need two limbs.
        assert_eq!(&max_limb * &max_limb, big((u32::MAX as i128).pow(2)));
    }

    #[test]
    fn division_correction_step_paths() {
        // Cases engineered to make the quotient-digit estimate too big by one, forcing
        // Algorithm D's add-back correction. We verify by reconstruction rather than by
        // hard-coding the (huge) expected quotients.
        let cases = [
            (BigInteger::from_u64(2).pow(128), BigInteger::from_u64(3).pow(40)),
            (factorial(40), factorial(20)),
            (
                BigInteger::from_str("340282366920938463463374607431768211455").unwrap(),
                BigInteger::from_str("18446744073709551616").unwrap(), // 2^64
            ),
            (
                BigInteger::from_str("100000000000000000000000000000001").unwrap(),
                BigInteger::from_str("99999999999999999999").unwrap(),
            ),
        ];
        for (a, b) in cases {
            let (q, r) = a.div_rem(&b);
            // a == q*b + r and 0 <= r < b (a and b are positive here).
            assert_eq!(&(&q * &b) + &r, a);
            assert!(r < b);
            assert!(!r.is_negative());
        }
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn division_by_zero_panics() {
        let _ = big(5).div_rem(&BigInteger::zero());
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn rem_by_zero_panics() {
        let _ = big(5).rem(&BigInteger::zero());
    }

    // ==== Ordering, equality, hashing ====

    #[test]
    fn ordering_across_signs() {
        let mut xs = [
            big(-1_000_000_000_000i128),
            big(-1),
            big(0),
            big(1),
            big(1i128 << 40),
            big(1i128 << 80),
        ];
        // Sorting with our Ord must match the numeric order.
        xs.sort();
        let strs: Vec<String> = xs.iter().map(|x| x.to_string()).collect();
        assert_eq!(
            strs,
            vec![
                "-1000000000000",
                "-1",
                "0",
                "1",
                "1099511627776",
                "1208925819614629174706176",
            ]
        );
        assert!(big(-5) < big(-4));
        assert!(big(-4) < big(0));
        assert!(big(0) < big(1));
        assert_eq!(big(7), big(7));
        assert_ne!(big(7), big(-7));
    }

    #[test]
    fn hash_and_eq_are_consistent() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(big(42));
        set.insert(BigInteger::from_i64(42)); // same value, different construction path
        set.insert(&big(6) * &big(7)); // 42 again
        set.insert(big(-42));
        assert_eq!(set.len(), 2); // {42, -42}
        assert!(set.contains(&big(42)));
        assert!(set.contains(&big(-42)));
    }

    #[test]
    fn debug_and_display_forms() {
        assert_eq!(format!("{}", big(-123)), "-123");
        assert_eq!(format!("{:?}", big(-123)), "BigInteger(-123)");
        assert_eq!(format!("{}", BigInteger::zero()), "0");
    }

    #[test]
    #[should_panic(expected = "radix must be in 2..=36")]
    fn to_str_radix_bad_radix_panics() {
        let _ = big(10).to_str_radix(40);
    }

    // ==== Algebraic identities over the LCG stream (self-consistency, no oracle) ====

    #[test]
    fn algebraic_identities_hold() {
        let mut rng = Lcg::new(0x1357_9BDF);
        for _ in 0..3_000 {
            // Use wide (beyond-i128) operands: identities must hold at any size.
            let a = make_wide(&mut rng);
            let b = make_wide(&mut rng);
            let c = make_wide(&mut rng);

            // Commutativity.
            assert_eq!(&a + &b, &b + &a);
            assert_eq!(&a * &b, &b * &a);
            // Associativity.
            assert_eq!(&(&a + &b) + &c, &a + &(&b + &c));
            assert_eq!(&(&a * &b) * &c, &a * &(&b * &c));
            // Distributivity.
            assert_eq!(&a * &(&b + &c), &(&a * &b) + &(&a * &c));
            // Additive inverse and double negation.
            assert_eq!(&a + &(-&a), BigInteger::zero());
            assert_eq!(-&(-&a), a.clone());
            // Subtraction is add-of-negation.
            assert_eq!(&a - &b, &a + &(-&b));

            if !b.is_zero() {
                // Division identity at arbitrary precision.
                let (q, r) = a.div_rem(&b);
                assert_eq!(&(&q * &b) + &r, a);
                assert!(r.abs() < b.abs());
                // Remainder shares the dividend's sign (or is zero).
                if !r.is_zero() {
                    assert_eq!(r.is_negative(), a.is_negative());
                }
            }
        }
    }

    /// Build a "wide" BigInteger (typically several limbs) from the LCG by stitching
    /// several 64-bit draws together and picking a sign — deliberately larger than i128.
    fn make_wide(rng: &mut Lcg) -> BigInteger {
        let limbs = (rng.next_u64() % 5) + 1; // 1..=5 limbs of raw material
        let mut acc = BigInteger::zero();
        let base = BigInteger::from_u64(1u64 << 32);
        for _ in 0..limbs {
            // acc = acc * 2^32 + next_draw  — shift up a limb and drop in fresh bits.
            acc = &(&acc * &base) + &BigInteger::from_u64(rng.next_u64());
        }
        if rng.next_u64() & 1 == 1 {
            -&acc
        } else {
            acc
        }
    }

    /// INDEPENDENT-ORACLE differential. The other tests check the crate against `i128`
    /// (bounded) and against ITSELF (the reconstruction identity uses the crate's own
    /// `mul` to validate its own `div`). This test pins a handful of hard, deliberately
    /// beyond-`i128` results to values computed OUT OF BAND by Python's arbitrary-
    /// precision integers (the canonical oracle) — so a systematic bug shared by the
    /// crate's own mul+div (which reconstruction could not catch) is still detected.
    /// The expected strings are Python `int` outputs; do not "fix" them to match code.
    #[test]
    fn matches_python_arbitrary_precision_oracle() {
        let a: BigInteger =
            "123456789012345678901234567890123456789".parse().unwrap();
        let b: BigInteger = "98765432109876543210987654321".parse().unwrap();

        // a * b  (Python: a*b)
        assert_eq!(
            (&a * &b).to_string(),
            "12193263113702179522618503273374485596336229233322374638011112635269"
        );
        // a / b and a % b, truncating toward zero (Python trunc-div; positives here).
        let (q, r) = a.div_rem(&b);
        assert_eq!(q.to_string(), "1249999988");
        assert_eq!(r.to_string(), "60185185206018518520725308641");
        // Negative dividend: truncation toward zero, remainder takes dividend's sign.
        let (nq, nr) = (-&a).div_rem(&b);
        assert_eq!(nq.to_string(), "-1249999988");
        assert_eq!(nr.to_string(), "-60185185206018518520725308641");
        // gcd (Python math.gcd(a, b))
        assert_eq!(a.gcd(&b).to_string(), "9");
        // pow beyond i128 (Python 7**99)
        assert_eq!(
            BigInteger::from_u64(7).pow(99).to_string(),
            "462068072803536855906378252728602401551029028414946485847699333055955922805275437143"
        );
        // Radix round-trips against Python's base conversions.
        assert_eq!(b.to_str_radix(16), "13f20d9c2fff89d38e1c70cb1");
        assert_eq!(b.to_str_radix(36), "9kpsz865lt7jkxk0gq9");
        assert_eq!(
            BigInteger::parse_radix("13f20d9c2fff89d38e1c70cb1", 16).unwrap(),
            b
        );
        // 2^200 in base 36 (Python).
        assert_eq!(
            BigInteger::from_u64(2).pow(200).to_str_radix(36),
            "bnklg118comha6gqury14067gur54n8won6guf4"
        );
    }

    /// The `try_pow` size guard: a hostile exponent is refused up front (no OOM), a
    /// legitimate one computes the same value as `pow`, and non-growing bases always
    /// pass regardless of the exponent.
    #[test]
    fn try_pow_guards_oversized_results() {
        let two = BigInteger::from_u64(2);
        // Refused BEFORE any allocation: 2^4e9 would be ~4 gigabits.
        let err = two.try_pow(4_000_000_000, 1 << 20).unwrap_err();
        assert_eq!(err.max_bits, 1 << 20);
        assert!(err.projected_bits > err.max_bits);
        // A modest exponent within the ceiling succeeds and equals plain `pow`.
        assert_eq!(two.try_pow(200, 4096).unwrap(), two.pow(200));
        // Exactly at the boundary: 2^100 is 101 bits (bit_len(2)=2 → projected 200).
        assert!(BigInteger::from_u64(2).try_pow(100, 200).is_ok());
        assert!(BigInteger::from_u64(2).try_pow(100, 199).is_err());
        // Non-growing bases pass at any exponent — 1, -1, 0 never blow up.
        assert_eq!(BigInteger::one().try_pow(u32::MAX, 1).unwrap(), BigInteger::one());
        assert_eq!(
            (-&BigInteger::one()).try_pow(u32::MAX, 1).unwrap(),
            -&BigInteger::one()
        );
        assert_eq!(BigInteger::zero().try_pow(u32::MAX, 1).unwrap(), BigInteger::zero());
        // exp == 0 is 1 for any base, always within any non-zero ceiling.
        assert_eq!(BigInteger::from_u64(999).try_pow(0, 1).unwrap(), BigInteger::one());
    }
}
