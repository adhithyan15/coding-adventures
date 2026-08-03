//! # `BigDouble` — an arbitrary-precision binary float that knows its own accuracy
//!
//! `BigInteger`, `BigRational`, and `BigDecimal` are all **exact**. But some numbers are not
//! exactly any fraction: `√2`, `ln 2`, `e`, `π`. For those, exactness is impossible, so the
//! honest thing is not to *pretend* — it is to compute to a **stated precision** with a
//! **stated rounding mode**, and to *carry* how many bits are trustworthy. That is `BigDouble`
//! (NUM-4): a binary floating-point number, like an `f64`, but with a mantissa and an exponent
//! of *unbounded* size — so instead of 53 bits it can carry hundreds or thousands, as many as
//! you ask for.
//!
//! ## How a `BigDouble` is stored
//!
//! A value is `mantissa × 2^exponent`, exactly the shape of an IEEE-754 `f64` (`mantissa` a
//! [`BigInteger`], `exponent` an `i64`), plus a **precision** `prec` — the number of
//! significant bits the mantissa is kept to. The invariant: a non-zero mantissa is normalized
//! to *exactly* `prec` bits (`bit_len(|mantissa|) == prec`); zero is `mantissa = 0`.
//!
//! ```text
//!     value  =  (±) mantissa  ×  2^exponent          bit_len(|mantissa|) = prec
//! ```
//!
//! ## Correct rounding, and why it can be trusted
//!
//! Every operation that cannot be exact (`√`, or a sum/product/quotient whose true result needs
//! more than `prec` bits) computes the exact result conceptually, then rounds it to `prec` bits
//! under the mode you chose — using **guard and sticky** information so the rounding is the same
//! one IEEE-754 hardware would make. In fact the test suite proves exactly that: at `prec = 53`
//! with round-half-even, `BigDouble` addition, subtraction, multiplication, division, and square
//! root reproduce `f64` **bit for bit** across tens of thousands of random operands. Beyond 53
//! bits it keeps going where the hardware cannot.
//!
//! Because every binary fraction is a terminating decimal (`x·2^-k = x·5^k·10^-k`), a
//! `BigDouble` converts **exactly** to a [`BigDecimal`](crate::BigDecimal) — reusing that rung
//! rather than reinventing decimal rendering.
//!
//! ```
//! use bignum_core::{BigDouble, RoundingMode};
//!
//! // √2 to 60 significant bits, then read it back as decimal.
//! let root2 = BigDouble::from_i64(2).sqrt(60, RoundingMode::HalfEven);
//! assert!(root2.to_decimal().unwrap().to_string().starts_with("1.41421356237309"));
//!
//! // At 53 bits, arithmetic matches the hardware f64 exactly.
//! let a = BigDouble::from_f64(0.1);
//! let b = BigDouble::from_f64(0.2);
//! assert_eq!(a.add(&b, 53, RoundingMode::HalfEven).to_f64(), 0.1_f64 + 0.2_f64);
//! ```
//!
//! Transcendental functions (`ln`, `exp`, `sin`, …) are a separate later effort that builds on
//! this core; NUM-4 here is the representation plus correctly-rounded `+ − × ÷` and `√`.

use crate::{BigDecimal, BigInteger, BigRational, RoundingMode};
use std::cmp::Ordering;
use std::fmt;

// ===========================================================================
//  Budgets
// ===========================================================================

/// The largest working precision (in mantissa bits) a `BigDouble` may carry.
///
/// A security budget, not a real limit on usefulness: a million bits is ~300,000 decimal
/// digits, astronomically beyond any measurement. Bounding it keeps every shift and
/// multiplication a `2·prec`-bounded amount of work, so no precision request can be turned into
/// unbounded memory.
pub const MAX_PRECISION: u32 = 1_000_000;

/// The largest magnitude a `BigDouble`'s base-2 exponent may take.
///
/// A security budget on the *scale* of a value, the exponent's analogue of [`MAX_PRECISION`].
/// `2^62` lets a value be as large as `2^(2^62)` — a number with more than a *quintillion* bits
/// before the point, astronomically past anything physical. Bounding it lets every internal
/// exponent computation be carried in `i128` (see [`fit_exp`]) with no risk of silently wrapping
/// `i64`: `exp ± prec`, `exp + exp` (multiply), and `exp − exp` (divide) of two in-range operands
/// all stay far inside `i128`. An operation whose true exponent would leave this band is reported
/// as an explicit "exponent out of range" panic — never a silent wrong answer — because this type
/// deliberately has no infinity.
pub const MAX_EXPONENT: i64 = 1 << 62;

/// The largest `|exponent|` for which [`BigDouble::to_decimal`] will *materialize* the exact
/// decimal, independent of (and far below) [`MAX_EXPONENT`].
///
/// This is the crucial distinction: a `BigDouble` can *hold* `2^(2^62)` in a handful of bytes
/// (`mant = 1`, huge `exp`), and arithmetic on it stays `O(prec)`. But *rendering* that as a
/// decimal would need `~|exp|` bits (`2^exp` above the point, or `5^|exp|` below) — so `to_decimal`
/// refuses (returns `None`) once `|exp|` passes this budget, rather than turning a cheap value into
/// unbounded memory. `8_000_000` bits/digits is already megabytes of output; anything past it is a
/// display request no one can consume. `to_f64`/`Display`, which route through `to_decimal`, then
/// fall back to saturation / a raw `mantissa·2^exp` rendering.
const MAX_DECIMAL_EXPONENT: i64 = 8_000_000;

/// Extra low-order bits kept during a rounded operation so the round/sticky decision is exact.
const GUARD: u32 = 3;

/// Narrow a computed exponent (carried in `i128` so it cannot have wrapped) back into the stored
/// `i64` field, enforcing the [`MAX_EXPONENT`] budget. Panics with a clear message — rather than
/// truncating — if the value is out of range, so an overflow can never become a silent wrong
/// result.
fn fit_exp(e: i128) -> i64 {
    assert!(
        e.unsigned_abs() <= MAX_EXPONENT as u128,
        "BigDouble exponent {e} is out of range (|exponent| must be ≤ MAX_EXPONENT = {MAX_EXPONENT})"
    );
    e as i64
}

// ===========================================================================
//  The type
// ===========================================================================

/// An arbitrary-precision binary floating-point number: `mantissa × 2^exponent`, carrying the
/// number of significant bits (`prec`) it is trustworthy to. See the
/// [module documentation](crate::float).
///
/// Fields are private so the "mantissa is exactly `prec` bits" invariant cannot be broken.
#[derive(Clone)]
pub struct BigDouble {
    /// Signed significand; `bit_len(|mant|) == prec` unless the value is zero.
    mant: BigInteger,
    /// Base-2 exponent: the value is `mant × 2^exp`.
    exp: i64,
    /// Working precision: the number of significant bits kept in `mant`.
    prec: u32,
}

// ===========================================================================
//  Bit-shift helpers on BigInteger (base-2, via multiply/divide by 2^k)
// ===========================================================================

/// `2^k` as a [`BigInteger`]. `k` is bounded by the precision/exponent budgets to fit `u32`.
fn pow2(k: u64) -> BigInteger {
    BigInteger::from_i64(2).pow(u32::try_from(k).expect("bit shift exceeds the supported range"))
}

/// Left-shift a magnitude by `k` bits: `mag << k == mag · 2^k` (exact).
fn shl(mag: &BigInteger, k: u64) -> BigInteger {
    if k == 0 {
        mag.clone()
    } else {
        mag * &pow2(k)
    }
}

/// Right-shift a **non-negative** magnitude by `k` bits, returning the shifted value and whether
/// any set bits fell off (the sticky flag). `mag >> k == floor(mag / 2^k)`.
fn shr_sticky(mag: &BigInteger, k: u64) -> (BigInteger, bool) {
    if k == 0 {
        return (mag.clone(), false);
    }
    if k >= mag.bit_len() {
        // Everything shifts out; the result is 0 and every non-zero bit is sticky.
        return (BigInteger::zero(), !mag.is_zero());
    }
    let (q, r) = mag.div_rem(&pow2(k));
    (q, !r.is_zero())
}

// ===========================================================================
//  Rounding a magnitude to a precision
// ===========================================================================

/// Round `sign · mag · 2^exp` (with `mag ≥ 0`, plus an extra `sticky` flag for bits already
/// dropped *below* `exp`) to exactly `prec` significant bits under `mode`, returning the signed
/// mantissa and its exponent. This is the one place rounding happens.
fn round_magnitude(
    negative: bool,
    mag: BigInteger,
    exp: i128,
    sticky: bool,
    prec: u32,
    mode: RoundingMode,
) -> (BigInteger, i64) {
    if mag.is_zero() {
        // Nothing above the round position. A directed mode may still nudge a pure-sticky value
        // to the smallest representable magnitude, but that requires a non-zero mantissa to
        // point at; with no bits at all the only sound answer is zero.
        return (BigInteger::zero(), 0);
    }
    let bl = mag.bit_len();
    let prec64 = prec as u64;

    if bl <= prec64 {
        // Fewer significant bits than requested: the value is exact at this precision. Pad the
        // mantissa up to exactly `prec` bits (shifting the exponent down to match). Any `sticky`
        // sits strictly below the last bit — less than half a ULP — so it only matters to a
        // directed mode, which then rounds one ULP away from zero.
        let pad = prec64 - bl;
        let mut m = shl(&mag, pad);
        let mut e = exp - pad as i128;
        if sticky && directed_rounds_away(mode, negative) {
            m = &m + &BigInteger::one();
            // A carry here (m became 2^prec) renormalizes on the next op; at pad≥0 it cannot
            // exceed prec+1 bits, and callers re-round, so leave it — but keep the invariant by
            // shifting back if it overflowed.
            if m.bit_len() > prec64 {
                m = shr_sticky(&m, 1).0;
                e += 1;
            }
        }
        return (apply_sign(m, negative), fit_exp(e));
    }

    // More bits than requested: drop the low `drop` bits, rounding.
    let drop = bl - prec64;
    let (q, rem) = mag.div_rem(&pow2(drop));
    // `sticky` here is the *external* remnant — bits already dropped below this field. It is kept
    // separate from `rem` so the exact-halfway test can tell "exactly ½ ULP" (round to even)
    // apart from "½ ULP plus a little more" (always round up).
    let round_up = decide_round_up(&rem, drop, sticky, &q, mode, negative);

    let mut q = if round_up { &q + &BigInteger::one() } else { q };
    let mut e = exp + drop as i128;
    if q.bit_len() > prec64 {
        // Rounding carried into a new top bit (q became 2^prec); halve and bump the exponent.
        q = shr_sticky(&q, 1).0;
        e += 1;
    }
    (apply_sign(q, negative), fit_exp(e))
}

/// Would `mode` round a value with something below the last kept bit *away from zero*?
fn directed_rounds_away(mode: RoundingMode, negative: bool) -> bool {
    match mode {
        RoundingMode::Up => true,
        RoundingMode::Ceiling => !negative,
        RoundingMode::Floor => negative,
        _ => false, // Down and the Half* modes leave a sub-half remnant alone.
    }
}

/// Decide whether the truncated quotient `q` should be incremented, given the dropped field `rem`
/// (the low `drop` bits), the `external_sticky` bits already dropped *below* that field, `q`, and
/// the mode. Mirrors the decimal rounding logic, but the half-way point is `2^(drop-1)`.
fn decide_round_up(
    rem: &BigInteger,
    drop: u64,
    external_sticky: bool,
    q: &BigInteger,
    mode: RoundingMode,
    negative: bool,
) -> bool {
    let anything_dropped = external_sticky || !rem.is_zero();
    if !anything_dropped {
        return false; // exact — nothing to round
    }
    match mode {
        RoundingMode::Down => false,
        RoundingMode::Up => true,
        RoundingMode::Floor => negative,
        RoundingMode::Ceiling => !negative,
        RoundingMode::HalfUp | RoundingMode::HalfDown | RoundingMode::HalfEven => {
            let half = pow2(drop - 1);
            match rem.cmp(&half) {
                Ordering::Less => false,
                Ordering::Greater => true,
                // Exactly half of the dropped field. If any bit sits still lower, the true value
                // is past half → round up; otherwise it is a genuine tie, broken by the mode.
                Ordering::Equal if external_sticky => true,
                Ordering::Equal => match mode {
                    RoundingMode::HalfUp => true,
                    RoundingMode::HalfDown => false,
                    _ => q_is_odd(q), // HalfEven
                },
            }
        }
    }
}

fn q_is_odd(q: &BigInteger) -> bool {
    !(q % &BigInteger::from_i64(2)).is_zero()
}

fn apply_sign(mag: BigInteger, negative: bool) -> BigInteger {
    if negative {
        -mag
    } else {
        mag
    }
}

fn check_prec(prec: u32) -> u32 {
    assert!(prec >= 1, "BigDouble precision must be at least 1 bit");
    assert!(
        prec <= MAX_PRECISION,
        "BigDouble precision exceeds MAX_PRECISION"
    );
    prec
}

// ===========================================================================
//  Construction
// ===========================================================================

impl BigDouble {
    /// The value `0`, carried at `prec` bits.
    pub fn zero(prec: u32) -> Self {
        BigDouble {
            mant: BigInteger::zero(),
            exp: 0,
            prec: check_prec(prec),
        }
    }

    /// The value `1`, carried at `prec` bits.
    pub fn one(prec: u32) -> Self {
        BigDouble::from_bigint(BigInteger::one(), prec, RoundingMode::HalfEven)
    }

    /// Build `mantissa × 2^exponent`, rounded to `prec` significant bits under `mode`.
    pub fn from_parts(mant: BigInteger, exp: i64, prec: u32, mode: RoundingMode) -> Self {
        let prec = check_prec(prec);
        let negative = mant.is_negative();
        let (m, e) = round_magnitude(negative, mant.abs(), exp as i128, false, prec, mode);
        BigDouble {
            mant: m,
            exp: e,
            prec,
        }
    }

    /// Promote an exact integer to a `BigDouble` at `prec` bits (rounding only if the integer has
    /// more than `prec` significant bits).
    pub fn from_bigint(n: BigInteger, prec: u32, mode: RoundingMode) -> Self {
        BigDouble::from_parts(n, 0, prec, mode)
    }

    /// Build from a primitive integer at `prec = 64` bits (enough to hold any `i64` exactly).
    pub fn from_i64(n: i64) -> Self {
        BigDouble::from_bigint(BigInteger::from_i64(n), 64, RoundingMode::HalfEven)
    }

    /// Convert an `f64` **exactly** (an `f64` *is* a `mantissa × 2^exp` binary float). The result
    /// carries 53 bits — the significand width of a `double`.
    ///
    /// # Panics
    /// Panics on a non-finite input (NaN or infinity); this type has no NaN/Inf.
    pub fn from_f64(x: f64) -> Self {
        assert!(x.is_finite(), "BigDouble::from_f64 requires a finite value");
        if x == 0.0 {
            return BigDouble::zero(53);
        }
        let bits = x.to_bits();
        let negative = (bits >> 63) != 0;
        let biased = ((bits >> 52) & 0x7ff) as i64;
        let frac = bits & 0x000f_ffff_ffff_ffff;
        let (mant_u, exp) = if biased == 0 {
            (frac, -1074) // subnormal: frac × 2^-1074
        } else {
            ((1u64 << 52) | frac, biased - 1075) // normal: (1.frac) × 2^(e-1023), scaled by 2^-52
        };
        let mant = apply_sign(BigInteger::from_u64(mant_u), negative);
        BigDouble::from_parts(mant, exp, 53, RoundingMode::HalfEven)
    }

    /// Re-round this value to a different precision under `mode`.
    pub fn with_precision(&self, prec: u32, mode: RoundingMode) -> BigDouble {
        BigDouble::from_parts(self.mant.clone(), self.exp, prec, mode)
    }

    /// Promote an exact rational to a `BigDouble`, correctly rounded to `prec` bits under `mode`
    /// — the promotion primitive from the exact `BigRational` world into the approximate
    /// `BigDouble` world (NUM-7). It is the same shape as [`BigRational::to_f64`], generalized
    /// from a fixed 64-bit division target to a caller-requested precision: both parts enter
    /// exactly (each at its own bit length, capped at [`MAX_PRECISION`]), and the quotient is
    /// taken at `prec` bits so the result is the correctly rounded `BigDouble` for any rational
    /// of practical size.
    pub fn from_rational(r: &BigRational, prec: u32, mode: RoundingMode) -> BigDouble {
        let prec = check_prec(prec);
        if r.numerator().is_zero() {
            return BigDouble::zero(prec);
        }
        let np = u32::try_from(r.numerator().bit_len())
            .unwrap_or(MAX_PRECISION)
            .clamp(1, MAX_PRECISION);
        let dp = u32::try_from(r.denominator().bit_len())
            .unwrap_or(MAX_PRECISION)
            .clamp(1, MAX_PRECISION);
        let n = BigDouble::from_bigint(r.numerator().clone(), np, mode);
        let d = BigDouble::from_bigint(r.denominator().clone(), dp, mode);
        n.div(&d, prec, mode)
    }

    /// The signed significand.
    pub fn mantissa(&self) -> &BigInteger {
        &self.mant
    }

    /// The base-2 exponent (`value = mantissa × 2^exponent`).
    pub fn exponent(&self) -> i64 {
        self.exp
    }

    /// The working precision, in significant bits.
    pub fn precision(&self) -> u32 {
        self.prec
    }
}

// ===========================================================================
//  Predicates & sign
// ===========================================================================

impl BigDouble {
    /// Is this exactly zero?
    pub fn is_zero(&self) -> bool {
        self.mant.is_zero()
    }

    /// Is this strictly negative?
    pub fn is_negative(&self) -> bool {
        self.mant.is_negative()
    }

    /// Is this strictly positive?
    pub fn is_positive(&self) -> bool {
        self.mant.is_positive()
    }

    /// `-1`, `0`, or `+1` according to the sign.
    pub fn signum(&self) -> i32 {
        self.mant.signum()
    }

    /// The absolute value, at the same precision.
    pub fn abs(&self) -> BigDouble {
        BigDouble {
            mant: self.mant.abs(),
            exp: self.exp,
            prec: self.prec,
        }
    }

    /// The negation, at the same precision.
    pub fn neg(&self) -> BigDouble {
        BigDouble {
            mant: -&self.mant,
            exp: self.exp,
            prec: self.prec,
        }
    }

    /// The position of the most significant bit as a signed binary exponent: for a non-zero
    /// value, `floor(log2(|self|))`. Used to compare and align magnitudes without materializing
    /// anything. Returns `i64::MIN as i128` for zero. Carried in `i128` so `exp + bit_len` cannot
    /// overflow even for an exponent at the `MAX_EXPONENT` ceiling.
    fn msb_position(&self) -> i128 {
        if self.mant.is_zero() {
            i64::MIN as i128
        } else {
            self.exp as i128 + self.mant.bit_len() as i128 - 1
        }
    }
}

// ===========================================================================
//  Arithmetic
// ===========================================================================

impl BigDouble {
    /// Correctly-rounded sum, to `prec` bits under `mode`.
    ///
    /// The two values are aligned to a common low exponent, but never below `prec + GUARD` bits
    /// under the result's most-significant bit — anything further down cannot change the rounded
    /// answer beyond a sticky bit, so it is folded into one rather than materialized. This makes
    /// `10^large + 10^small`-style sums cost `O(prec)`, not `O(exponent gap)`.
    pub fn add(&self, other: &BigDouble, prec: u32, mode: RoundingMode) -> BigDouble {
        let prec = check_prec(prec);
        if self.is_zero() {
            return other.with_precision(prec, mode);
        }
        if other.is_zero() {
            return self.with_precision(prec, mode);
        }
        // A window low enough to hold both operands' contributions to `prec + GUARD` bits.
        // Carried in `i128` (via `msb_position`) so the exponent math cannot overflow `i64`.
        let top = self.msb_position().max(other.msb_position()) + 1;
        let keep_exp = top - (prec as i128 + GUARD as i128 + 1);

        let (ca, sa) = self.contribution_at(keep_exp);
        let (cb, sb) = other.contribution_at(keep_exp);
        let summed = &ca + &cb;

        let negative = summed.is_negative();
        let mut mag = summed.abs();

        // Fold each operand's *lost* low-order fraction into a single sticky bit — but mind its
        // sign. The larger-magnitude operand never loses bits (its least-significant bit sits at
        // least `GUARD+1` places above `keep_exp`), so at most the *smaller* operand is sticky,
        // and a smaller operand losing bits means the magnitudes are far apart — so `mag` is
        // nowhere near zero and there is no catastrophic cancellation to worry about.
        //
        // A lost fraction `f ∈ (0, 1)` ULP carries its own operand's sign:
        //   • same sign as the result  → it *adds*:      |result| = |summed| + f   (ordinary sticky)
        //   • opposite sign            → it *subtracts*:  |result| = |summed| − f   (a borrow)
        // The borrow is rewritten additively as `(|summed| − 1) + (1 − f)`: drop one ULP and keep
        // a sticky, since `1 − f ∈ (0, 1)` is still a positive fraction below `keep_exp`. Without
        // this, an opposite-sign lost fraction would round the magnitude the wrong way.
        let mut sticky = false;
        for (lost, operand_negative) in
            [(sa, self.mant.is_negative()), (sb, other.mant.is_negative())]
        {
            if !lost {
                continue;
            }
            if operand_negative == negative || mag.is_zero() {
                sticky = true; // additive
            } else {
                mag = &mag - &BigInteger::one(); // borrow: |summed| − 1, then …
                sticky = true; // … + (1 − f), still a positive remnant
            }
        }
        round_to_bigdouble(negative, mag, keep_exp, sticky, prec, mode)
    }

    /// Correctly-rounded difference.
    pub fn sub(&self, other: &BigDouble, prec: u32, mode: RoundingMode) -> BigDouble {
        self.add(&other.neg(), prec, mode)
    }

    /// This value's mantissa expressed at exponent `keep_exp` (left-shifted if `exp ≥ keep_exp`,
    /// else right-shifted with a sticky flag for the bits that fall off).
    ///
    /// The shift distances are computed in `i128` (`keep_exp` is `i128`). On the left-shift branch
    /// the distance is at most `prec + GUARD` (this operand's least-significant bit never sits far
    /// above `keep_exp`), so it fits `u64` comfortably. On the right-shift branch the distance can
    /// be the full exponent gap — but it stays within `2·MAX_EXPONENT < u64::MAX`, and
    /// [`shr_sticky`] short-circuits (without allocating) once it exceeds the mantissa's bit length.
    fn contribution_at(&self, keep_exp: i128) -> (BigInteger, bool) {
        let negative = self.mant.is_negative();
        let mag = self.mant.abs();
        let exp = self.exp as i128;
        if exp >= keep_exp {
            (apply_sign(shl(&mag, (exp - keep_exp) as u64), negative), false)
        } else {
            let (shifted, sticky) = shr_sticky(&mag, (keep_exp - exp) as u64);
            (apply_sign(shifted, negative), sticky)
        }
    }

    /// Correctly-rounded product. The mantissa product is exact; only the final round to `prec`
    /// bits can lose anything.
    pub fn mul(&self, other: &BigDouble, prec: u32, mode: RoundingMode) -> BigDouble {
        let prec = check_prec(prec);
        if self.is_zero() || other.is_zero() {
            return BigDouble::zero(prec);
        }
        let negative = self.mant.is_negative() != other.mant.is_negative();
        let mag = &self.mant.abs() * &other.mant.abs();
        let exp = self.exp as i128 + other.exp as i128; // i128: two i64 exponents can't overflow it
        round_to_bigdouble(negative, mag, exp, false, prec, mode)
    }

    /// Correctly-rounded quotient, to `prec` bits under `mode`.
    ///
    /// # Panics
    /// Panics if `other` is zero. Use [`checked_div`](Self::checked_div) for the non-panicking
    /// form.
    pub fn div(&self, other: &BigDouble, prec: u32, mode: RoundingMode) -> BigDouble {
        self.checked_div(other, prec, mode)
            .expect("division by zero")
    }

    /// Correctly-rounded quotient, or `None` if `other` is zero.
    pub fn checked_div(&self, other: &BigDouble, prec: u32, mode: RoundingMode) -> Option<BigDouble> {
        let prec = check_prec(prec);
        if other.is_zero() {
            return None;
        }
        if self.is_zero() {
            return Some(BigDouble::zero(prec));
        }
        let negative = self.mant.is_negative() != other.mant.is_negative();
        let na = self.mant.abs();
        let nb = other.mant.abs();
        // Shift the numerator so the quotient carries at least prec + GUARD + 1 bits, then a
        // single integer division gives the quotient and a remainder that is exactly the sticky
        // information the rounding needs.
        let want = prec as i64 + GUARD as i64 + 1;
        let shift = (want - (na.bit_len() as i64 - nb.bit_len() as i64)).max(0) as u64;
        let (q, r) = shl(&na, shift).div_rem(&nb);
        let sticky = !r.is_zero();
        let exp = self.exp as i128 - other.exp as i128 - shift as i128; // i128: no i64 overflow
        Some(round_to_bigdouble(negative, q, exp, sticky, prec, mode))
    }

    /// The correctly-rounded square root, to `prec` bits under `mode`.
    ///
    /// The radicand is scaled so its integer square root already carries `prec + GUARD` bits; the
    /// integer-sqrt remainder is the sticky information. The exponent is made even first so it
    /// halves cleanly.
    ///
    /// # Panics
    /// Panics on a negative value (this type has no imaginary results).
    pub fn sqrt(&self, prec: u32, mode: RoundingMode) -> BigDouble {
        let prec = check_prec(prec);
        assert!(!self.is_negative(), "sqrt of a negative BigDouble");
        if self.is_zero() {
            return BigDouble::zero(prec);
        }
        // Make the exponent even so √(m·2^e) = √m · 2^(e/2) with an integer e/2.
        let (mut m, mut e) = (self.mant.clone(), self.exp);
        if e.rem_euclid(2) != 0 {
            m = shl(&m, 1); // m·2, e-1 keeps the value and makes e even
            e -= 1;
        }
        // Scale the radicand up (by an even power of two) so isqrt yields ~prec+GUARD bits.
        let want_radicand_bits = 2 * (prec as i64 + GUARD as i64 + 1);
        let mut s = want_radicand_bits - m.bit_len() as i64;
        if s < 0 {
            s = 0;
        }
        if s % 2 != 0 {
            s += 1; // keep the shift even so the halved exponent stays an integer
        }
        let radicand = shl(&m, s as u64);
        let root = isqrt(&radicand);
        let root_squared = &root * &root;
        let sticky = root_squared != radicand;
        let result_exp = e as i128 / 2 - s as i128 / 2;
        round_to_bigdouble(false, root, result_exp, sticky, prec, mode)
    }
}

/// Round a signed magnitude to a `BigDouble` of the given precision (thin wrapper over
/// [`round_magnitude`] that packages the result).
fn round_to_bigdouble(
    negative: bool,
    mag: BigInteger,
    exp: i128,
    sticky: bool,
    prec: u32,
    mode: RoundingMode,
) -> BigDouble {
    let (mant, exp) = round_magnitude(negative, mag, exp, sticky, prec, mode);
    BigDouble { mant, exp, prec }
}

/// Floor of the integer square root of a non-negative `BigInteger`, by Newton's method
/// converging from above (each step `x ← (x + n/x)/2` until it stops decreasing).
fn isqrt(n: &BigInteger) -> BigInteger {
    if n.is_zero() {
        return BigInteger::zero();
    }
    let one = BigInteger::one();
    if n <= &one {
        return one;
    }
    let two = BigInteger::from_i64(2);
    // Start above √n: 2^ceil(bit_len/2) ≥ √n.
    let mut x = pow2(n.bit_len().div_ceil(2));
    loop {
        // y = (x + n/x) / 2  — always ≤ x once we're at/above the root.
        let y = &(&x + &(n / &x)) / &two;
        if y >= x {
            break;
        }
        x = y;
    }
    x
}

// ===========================================================================
//  Comparison  (by real value, independent of the stored precision)
// ===========================================================================

impl BigDouble {
    /// Compare by real value. Two `BigDouble`s are equal when they denote the same real number,
    /// regardless of the precision each happens to carry.
    fn value_cmp(&self, other: &Self) -> Ordering {
        // Signs first.
        let sa = self.signum();
        let sb = other.signum();
        if sa != sb {
            return sa.cmp(&sb);
        }
        if sa == 0 {
            return Ordering::Equal; // both zero
        }
        // Same non-zero sign: compare magnitudes, then flip if both negative.
        let mag = self.cmp_magnitude(other);
        if sa < 0 {
            mag.reverse()
        } else {
            mag
        }
    }

    /// Compare `|self|` and `|other|` (both non-zero). A cheap most-significant-bit check settles
    /// values of clearly different scale; only when their leading bits line up do we align and
    /// compare mantissas, and that alignment is bounded by the mantissa widths.
    fn cmp_magnitude(&self, other: &Self) -> Ordering {
        let pa = self.msb_position();
        let pb = other.msb_position();
        if pa != pb {
            return pa.cmp(&pb);
        }
        // Same leading-bit position: compare at a common exponent.
        let e = self.exp.min(other.exp);
        let a = shl(&self.mant.abs(), (self.exp - e) as u64);
        let b = shl(&other.mant.abs(), (other.exp - e) as u64);
        a.cmp(&b)
    }
}

impl PartialEq for BigDouble {
    fn eq(&self, other: &Self) -> bool {
        self.value_cmp(other) == Ordering::Equal
    }
}
impl Eq for BigDouble {}
impl PartialOrd for BigDouble {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for BigDouble {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value_cmp(other)
    }
}

// ===========================================================================
//  Conversions out
// ===========================================================================

impl BigDouble {
    /// The exact value as a [`BigDecimal`], or `None` if the exponent is so extreme that the exact
    /// decimal would exceed `BigDecimal`'s scale budget. Exact because every binary fraction
    /// terminates in base 10: `mant · 2^-k = mant · 5^k · 10^-k`.
    pub fn to_decimal(&self) -> Option<BigDecimal> {
        // Materializing the exact decimal costs `O(|exp|)` bits, so refuse *before* allocating if
        // that would exceed the budget. This gates every path below (both `shl`/`pow2` for a
        // positive exponent and `5^k` for a negative one) — nothing large is built once we return
        // here, so a pathological exponent cannot become an out-of-memory or a panic.
        if self.exp.unsigned_abs() > MAX_DECIMAL_EXPONENT as u64 {
            return None;
        }
        if self.exp >= 0 {
            let scaled = shl(&self.mant.abs(), self.exp as u64);
            Some(BigDecimal::from_integer(apply_sign(scaled, self.mant.is_negative())))
        } else {
            let k = (-self.exp) as u64;
            let five_k = BigInteger::from_i64(5).pow(u32::try_from(k).ok()?);
            let num = apply_sign(&self.mant.abs() * &five_k, self.mant.is_negative());
            // A negative `exp` becomes a decimal scale of `k`; reject if beyond BigDecimal's guard.
            BigDecimal::checked_from_parts(num, i64::try_from(k).ok()?)
        }
    }

    /// A **lossy** narrowing to `f64` (nearest, ties to even), for interop and quick reads.
    /// Values outside `f64`'s range become `±∞` or `0.0`.
    ///
    /// The narrowing goes through the *exact* decimal string (when the exponent is in range) and
    /// Rust's correctly-rounded float parser, so the result is the true nearest `f64` — including
    /// in the subnormal range, where a naive `mantissa · 2^exp` multiply would underflow the
    /// intermediate power of two and lose bits.
    pub fn to_f64(&self) -> f64 {
        if self.is_zero() {
            return 0.0;
        }
        if let Some(dec) = self.to_decimal() {
            if let Ok(x) = dec.to_string().parse::<f64>() {
                return x;
            }
        }
        // Exponent beyond BigDecimal's range (astronomically large or small): saturate.
        let sign = if self.mant.is_negative() { -1.0 } else { 1.0 };
        if self.exp > 0 {
            sign * f64::INFINITY
        } else {
            sign * 0.0
        }
    }
}

// ===========================================================================
//  Formatting
// ===========================================================================

impl fmt::Display for BigDouble {
    /// Renders the exact decimal value when the exponent is within range (via [`to_decimal`]),
    /// otherwise a `mantissa·2^exp` binary form for extreme exponents.
    ///
    /// [`to_decimal`]: BigDouble::to_decimal
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_decimal() {
            Some(d) => write!(f, "{d}"),
            None => write!(f, "{}p{}", self.mant, self.exp),
        }
    }
}

impl fmt::Debug for BigDouble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BigDouble({} × 2^{}, {} bits)", self.mant, self.exp, self.prec)
    }
}

// ===========================================================================
//  Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use RoundingMode::HalfEven;

    // ---- from_f64 / to_f64 are an exact round trip ----------------------

    #[test]
    fn f64_round_trips_exactly() {
        for x in [0.0, 1.0, -1.0, 0.5, 0.1, 0.2, 3.625, -2.5, 1e300, 1e-300, 12345.678] {
            assert_eq!(BigDouble::from_f64(x).to_f64(), x, "round-trip {x}");
        }
    }

    // ---- the headline: matches IEEE-754 f64 bit-for-bit at 53 bits ------

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            self.0
        }
        /// A random *normal* f64 in a moderate magnitude range (exponent within ±60 of 1),
        /// with a full 52-bit random fraction, so op results usually stay normal.
        fn normal_f64(&mut self) -> f64 {
            let sign = self.next() & 1;
            let exp = 1023u64.wrapping_add((self.next() % 121).wrapping_sub(60)) & 0x7ff;
            let exp = exp.clamp(1, 2046);
            let frac = self.next() & 0x000f_ffff_ffff_ffff;
            f64::from_bits((sign << 63) | (exp << 52) | frac)
        }
    }

    fn bd(x: f64) -> BigDouble {
        BigDouble::from_f64(x)
    }

    #[test]
    fn matches_f64_hardware_bit_for_bit() {
        let mut rng = Lcg(0x00C0_FFEE_1234_5678);
        let (mut n_add, mut n_mul, mut n_div, mut n_sqrt) = (0, 0, 0, 0);
        for _ in 0..40_000 {
            let a = rng.normal_f64();
            let b = rng.normal_f64();

            // Only assert when the hardware result is itself a normal number — BigDouble has no
            // subnormals or infinities, so those are out of its model, not disagreements.
            let s = a + b;
            if s.is_normal() {
                assert_eq!(bd(a).add(&bd(b), 53, HalfEven).to_f64(), s, "add {a} + {b}");
                n_add += 1;
            }
            let d = a - b;
            if d.is_normal() {
                assert_eq!(bd(a).sub(&bd(b), 53, HalfEven).to_f64(), d, "sub {a} - {b}");
            }
            let p = a * b;
            if p.is_normal() {
                assert_eq!(bd(a).mul(&bd(b), 53, HalfEven).to_f64(), p, "mul {a} * {b}");
                n_mul += 1;
            }
            if b != 0.0 {
                let q = a / b;
                if q.is_normal() {
                    assert_eq!(bd(a).div(&bd(b), 53, HalfEven).to_f64(), q, "div {a} / {b}");
                    n_div += 1;
                }
            }
            let r = a.abs();
            let rs = r.sqrt();
            if rs.is_normal() {
                assert_eq!(bd(r).sqrt(53, HalfEven).to_f64(), rs, "sqrt {r}");
                n_sqrt += 1;
            }
        }
        // Sanity: the filters didn't reject essentially everything.
        assert!(n_add > 30_000 && n_mul > 20_000 && n_div > 20_000 && n_sqrt > 30_000);
    }

    // ---- precision beyond f64: sqrt pinned to Python's decimal ----------

    #[test]
    fn sqrt_high_precision_matches_python() {
        // 200 bits ≈ 60 correct decimal digits; check a comfortable 40-digit prefix.
        let r2 = BigDouble::from_i64(2).sqrt(200, HalfEven).to_decimal().unwrap().to_string();
        assert!(r2.starts_with("1.41421356237309504880168872420969807856"), "got {r2}");
        let r3 = BigDouble::from_i64(3).sqrt(200, HalfEven).to_decimal().unwrap().to_string();
        assert!(r3.starts_with("1.73205080756887729352744634150587236694"), "got {r3}");
        let r10 = BigDouble::from_i64(10).sqrt(200, HalfEven).to_decimal().unwrap().to_string();
        assert!(r10.starts_with("3.16227766016837933199889354443271853371"), "got {r10}");
    }

    #[test]
    fn sqrt_of_perfect_squares_is_exact() {
        assert_eq!(BigDouble::from_i64(4).sqrt(53, HalfEven).to_decimal().unwrap().to_string(), "2");
        assert_eq!(BigDouble::from_i64(144).sqrt(53, HalfEven).to_decimal().unwrap().to_string(), "12");
        // (10^15)^2 = 10^30 → √ is exactly 10^15.
        let big = BigDouble::from_bigint(BigInteger::from_i64(10).pow(30), 120, HalfEven);
        assert_eq!(big.sqrt(120, HalfEven).to_decimal().unwrap().to_string(), "1000000000000000");
    }

    // ---- exact decimal conversion ---------------------------------------

    #[test]
    fn to_decimal_is_exact() {
        assert_eq!(bd(0.5).to_decimal().unwrap().to_string(), "0.5");
        assert_eq!(bd(0.25).to_decimal().unwrap().to_string(), "0.25");
        assert_eq!(bd(3.0).to_decimal().unwrap().to_string(), "3");
        // 0.1 is not exactly 0.1 as an f64 — the exact binary value has a long decimal tail.
        assert_eq!(
            bd(0.1).to_decimal().unwrap().to_string(),
            "0.1000000000000000055511151231257827021181583404541015625"
        );
    }

    // ---- ordering, sign, precision --------------------------------------

    #[test]
    fn ordering_by_value_ignores_precision() {
        assert!(bd(0.1) < bd(0.2));
        assert!(bd(-1.0) < bd(0.0));
        assert!(bd(1e100) > bd(1e-100));
        // Same value at different precisions compares Equal.
        let a = BigDouble::from_i64(3);
        let b = a.with_precision(200, HalfEven);
        assert_eq!(a.cmp(&b), Ordering::Equal);
        assert_eq!(a, b);
        let mut xs = [bd(3.5), bd(-0.5), bd(0.0), bd(100.0), bd(0.001)];
        xs.sort();
        let shown: Vec<f64> = xs.iter().map(|x| x.to_f64()).collect();
        assert_eq!(shown, vec![-0.5, 0.0, 0.001, 3.5, 100.0]);
    }

    #[test]
    fn sign_and_precision_queries() {
        assert_eq!(bd(-2.5).signum(), -1);
        assert_eq!(bd(0.0).signum(), 0);
        assert_eq!(bd(2.5).signum(), 1);
        assert_eq!(bd(-2.5).abs().to_f64(), 2.5);
        assert_eq!(bd(2.5).neg().to_f64(), -2.5);
        assert!(bd(-1.0).is_negative());
        assert!(BigDouble::zero(53).is_zero());
        assert_eq!(BigDouble::from_i64(2).sqrt(120, HalfEven).precision(), 120);
        // A value carrying more bits than it needs still knows its precision.
        assert_eq!(BigDouble::from_i64(1).precision(), 64);
    }

    // ---- panics & checked forms -----------------------------------------

    #[test]
    #[should_panic(expected = "division by zero")]
    fn div_by_zero_panics() {
        let _ = bd(1.0).div(&BigDouble::zero(53), 53, HalfEven);
    }

    #[test]
    fn checked_div_handles_zero() {
        assert!(bd(1.0).checked_div(&BigDouble::zero(53), 53, HalfEven).is_none());
        assert!(bd(1.0).checked_div(&bd(2.0), 53, HalfEven).is_some());
    }

    #[test]
    #[should_panic(expected = "sqrt of a negative")]
    fn sqrt_of_negative_panics() {
        let _ = bd(-1.0).sqrt(53, HalfEven);
    }

    #[test]
    #[should_panic(expected = "finite")]
    fn from_f64_rejects_infinity() {
        let _ = BigDouble::from_f64(f64::INFINITY);
    }

    // ---- exponent / decimal-expansion budgets (DoS guards) --------------
    // (`BigInteger` and `MAX_EXPONENT` are in scope via `use super::*`.)

    fn parts(mant: i64, exp: i64, prec: u32) -> BigDouble {
        BigDouble::from_parts(BigInteger::from_i64(mant), exp, prec, HalfEven)
    }

    #[test]
    fn to_decimal_refuses_extreme_exponents_without_oom() {
        // A value whose exact decimal would need ~2·10⁹ bits: `to_decimal` must return `None`
        // *before* allocating anything (both the positive `2^exp` and negative `5^k` paths).
        assert!(parts(1, 2_000_000_000, 53).to_decimal().is_none());
        assert!(parts(1, -2_000_000_000, 53).to_decimal().is_none());
        // Just inside the budget still materializes (a few-thousand-digit decimal is fine).
        assert!(parts(1, 1000, 53).to_decimal().is_some());
        assert!(parts(1, -1000, 53).to_decimal().is_some());
    }

    #[test]
    fn to_f64_saturates_on_extreme_exponents() {
        // With `to_decimal` refusing, the documented saturate-to-±∞/0 path actually holds — no OOM.
        assert_eq!(parts(1, 2_000_000_000, 53).to_f64(), f64::INFINITY);
        assert_eq!(parts(-1, 2_000_000_000, 53).to_f64(), f64::NEG_INFINITY);
        assert_eq!(parts(1, -2_000_000_000, 53).to_f64(), 0.0);
    }

    #[test]
    fn arithmetic_survives_large_but_in_range_exponents() {
        // Exponents far larger than any f64 can hold, but within MAX_EXPONENT — arithmetic is still
        // O(prec) and does not overflow the i64 exponent field.
        let big = parts(3, 1_000_000_000_000, 80);
        let small = parts(5, -1_000_000_000_000, 80);
        assert!(big.mul(&big, 80, HalfEven).is_positive());
        assert!(big.add(&small, 80, HalfEven).is_positive());
        assert!(big.div(&small, 80, HalfEven).is_positive());
        // The sum of two vastly-separated magnitudes rounds back to the larger — cheaply.
        assert_eq!(big.add(&small, 80, HalfEven).exponent(), big.exponent());
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn from_parts_rejects_out_of_range_exponent() {
        // A raw exponent past MAX_EXPONENT is a clear, explicit panic — never a silent i64 wrap.
        let _ = parts(1, i64::MAX, 53);
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn multiply_overflowing_the_exponent_band_panics_clearly() {
        // Two values at the exponent ceiling multiply to 2·MAX_EXPONENT, out of the storable band:
        // an explicit "out of range" panic, not a silently-wrong result.
        let ceil = parts(1, MAX_EXPONENT, 53);
        let _ = ceil.mul(&ceil, 53, HalfEven);
    }

    // ---- rounding modes on a known tie ----------------------------------

    #[test]
    fn rounding_modes_break_a_binary_tie() {
        use RoundingMode::*;
        // 3 in 1 bit of precision is exactly between 2 (=10b) and 4 (=100b): mantissa 11b, drop
        // the low bit → 1.1b, a tie. Round to 1 significant bit under each mode.
        let three = BigDouble::from_i64(3);
        assert_eq!(three.with_precision(1, HalfUp).to_f64(), 4.0);
        assert_eq!(three.with_precision(1, HalfDown).to_f64(), 2.0);
        assert_eq!(three.with_precision(1, HalfEven).to_f64(), 4.0); // 4=100b has even low bit
        assert_eq!(three.with_precision(1, Down).to_f64(), 2.0);
        assert_eq!(three.with_precision(1, Up).to_f64(), 4.0);
        assert_eq!(three.with_precision(1, Floor).to_f64(), 2.0);
        assert_eq!(three.with_precision(1, Ceiling).to_f64(), 4.0);
        // 5 = 101b at 1 bit: low field 01b < half(10b) → rounds down to 4 under half-even.
        assert_eq!(BigDouble::from_i64(5).with_precision(1, HalfEven).to_f64(), 4.0);
        // negative tie: -3 → -4 under Floor, -2 under Ceiling.
        assert_eq!(BigDouble::from_i64(-3).with_precision(1, Floor).to_f64(), -4.0);
        assert_eq!(BigDouble::from_i64(-3).with_precision(1, Ceiling).to_f64(), -2.0);
    }

    // ---- from_rational: the exact-rational → BigDouble promotion (NUM-7a) ----

    #[test]
    fn from_rational_of_an_integer_is_exact() {
        let r = BigRational::from_ints(3, 1);
        assert_eq!(BigDouble::from_rational(&r, 64, HalfEven).to_decimal().unwrap().to_string(), "3");
    }

    #[test]
    fn from_rational_of_zero_is_zero() {
        let r = BigRational::from_ints(0, 5);
        assert!(BigDouble::from_rational(&r, 64, HalfEven).is_zero());
    }

    #[test]
    fn from_rational_is_negative_for_a_negative_rational() {
        let r = BigRational::from_ints(-3, 4);
        assert!(BigDouble::from_rational(&r, 64, HalfEven).is_negative());
    }

    #[test]
    fn from_rational_matches_to_f64_at_53_bits() {
        // Differential: at f64 width, from_rational should agree with BigRational's own,
        // independently-implemented to_f64 (rational.rs) for a spread of rationals.
        for (n, d) in [(1, 3), (22, 7), (1, 1_000_000), (-5, 8), (7, 2), (1, 2)] {
            let r = BigRational::from_ints(n, d);
            let via_from_rational = BigDouble::from_rational(&r, 53, HalfEven).to_f64();
            assert_eq!(via_from_rational, r.to_f64(), "{n}/{d}");
        }
    }

    #[test]
    fn from_rational_one_third_at_high_precision_matches_known_digits() {
        let third = BigRational::from_ints(1, 3);
        let rendered = BigDouble::from_rational(&third, 200, HalfEven).to_decimal().unwrap().to_string();
        assert!(rendered.starts_with("0.3333333333333333333"), "got {rendered}");
    }

    #[test]
    fn from_rational_handles_a_numerator_far_larger_than_the_requested_precision() {
        // The numerator's own bit length (thousands of bits, from repeated squaring) vastly
        // exceeds the requested working `prec` (64) — `np`/`dp` are derived from the operands'
        // own bit lengths, not `prec`, and must not panic or silently misbehave when they differ
        // this much.
        let big = BigInteger::from_i64(2).pow(4000);
        let r = BigRational::new(big, BigInteger::from_i64(3));
        let result = BigDouble::from_rational(&r, 64, HalfEven);
        assert!(!result.is_zero());
        assert!(!result.is_negative());
    }
}
