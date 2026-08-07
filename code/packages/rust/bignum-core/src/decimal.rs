//! # `BigDecimal` — an exact base-10 number, built on [`BigInteger`]
//!
//! A `BigDecimal` is a number written the way money is: a string of decimal digits with a
//! decimal point somewhere in it. Internally it is an arbitrary-precision integer **mantissa**
//! and an integer **scale** — the value is `mantissa × 10^(-scale)`. So `1.23` is
//! `(mantissa 123, scale 2)`, `100` is `(1, -2)`, and `0.001` is `(1, 3)`. It is the third
//! rung (NUM-3) of the ADJ numeric substrate.
//!
//! ## Why base 10, when we already have [`BigRational`](crate::BigRational)?
//!
//! `BigRational` is exact for *every* fraction, but it does not know that money is counted in
//! tenths and hundredths. A binary `f64` cannot even hold `0.10` exactly. `BigDecimal` works
//! in the same base humans (and tax codes, and drug-dosing charts) do, so `0.1 + 0.2` is
//! exactly `0.3`, `$100.00 − $0.01` is exactly `$99.99`, and a value rounds to the cent the
//! way an accountant expects — with a **rounding mode you state**, not one a float picks for
//! you. Addition, subtraction, and multiplication are always exact; division is the one
//! operation that must round (you cannot write `1/3` as a terminating decimal), so it is done
//! [to a scale you choose, with a mode you choose](BigDecimal::div_round).
//!
//! ## The one canonical form
//!
//! `1.20` and `1.2` are the same number, and `100` and `1×10²` are the same number. To make
//! equality and hashing trustworthy, every `BigDecimal` is kept in one canonical form:
//! **trailing zeros are stripped from the mantissa** (each one dropped lowers the scale by
//! one, since `mant0 × 10^-s == (mant0/10) × 10^-(s-1)`), and zero is always `(0, 0)`. So
//! `1.20` is stored as `(12, 1)`, `100` as `(1, -2)`, `0` as `(0, 0)`. Because the form is
//! unique, `Clone`/`PartialEq`/`Eq`/`Hash` are derived and value-correct. (Presenting a value
//! at a *fixed* number of places — `"$1.20"` — is a formatting choice applied at the boundary,
//! not a property of the stored number; that lives in NUM-6.)
//!
//! ```
//! use bignum_core::{BigDecimal, RoundingMode};
//! use std::str::FromStr;
//!
//! // 0.1 + 0.2 is exactly 0.3 (which a binary float cannot manage).
//! let sum = &BigDecimal::from_str("0.1").unwrap() + &BigDecimal::from_str("0.2").unwrap();
//! assert_eq!(sum.to_string(), "0.3");
//!
//! // Money stays exact through +, -, *.
//! let change = &BigDecimal::from_str("100.00").unwrap() - &BigDecimal::from_str("0.01").unwrap();
//! assert_eq!(change.to_string(), "99.99");
//!
//! // Division rounds — to the scale and mode you name.
//! let third = BigDecimal::from_str("10").unwrap()
//!     .div_round(&BigDecimal::from_str("3").unwrap(), 4, RoundingMode::HalfEven);
//! assert_eq!(third.to_string(), "3.3333");
//! ```

use crate::{BigInteger, BigRational};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

// ===========================================================================
//  Rounding modes
// ===========================================================================

/// How to round when a value cannot be represented exactly at the requested scale.
///
/// The `Half*` modes decide only the exact-halfway case (`…5`); away from the halfway point
/// they all round to the nearest representable value.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RoundingMode {
    /// Toward zero — drop the extra digits (truncate). `2.5 → 2`, `-2.5 → -2`.
    Down,
    /// Away from zero — round up in magnitude if anything is dropped. `2.1 → 3`, `-2.1 → -3`.
    Up,
    /// Toward negative infinity. `2.5 → 2`, `-2.5 → -3`.
    Floor,
    /// Toward positive infinity. `2.5 → 3`, `-2.5 → -2`.
    Ceiling,
    /// Nearest; ties round away from zero. `2.5 → 3`, `-2.5 → -3`. (The schoolbook rule.)
    HalfUp,
    /// Nearest; ties round toward zero. `2.5 → 2`, `-2.5 → -2`.
    HalfDown,
    /// Nearest; ties round to the even neighbor — "banker's rounding". `2.5 → 2`, `1.5 → 2`,
    /// `1.25 → 1.2`. Removes the upward bias of `HalfUp` over many roundings.
    HalfEven,
}

// ===========================================================================
//  The type
// ===========================================================================

/// An arbitrary-precision **exact base-10** number: a [`BigInteger`] mantissa scaled by a
/// power of ten. The value is `mantissa × 10^(-scale)`, always held in the canonical form
/// described in the [module documentation](crate::decimal) (mantissa carries no trailing
/// zero; zero is `(0, 0)`).
///
/// Fields are private so the canonical-form invariant cannot be broken from outside. Read
/// them with [`mantissa`](Self::mantissa) and [`scale`](Self::scale).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BigDecimal {
    /// The unscaled integer value. Carries the sign; never ends in a `0` digit unless it *is*
    /// zero (in which case `scale` is `0`).
    mant: BigInteger,
    /// The base-10 scale: the value is `mant × 10^(-scale)`. Positive = digits after the
    /// point; negative = trailing zeros before an implied point; may be any `i64`.
    scale: i64,
}

// ===========================================================================
//  Errors
// ===========================================================================

/// The error returned when a string cannot be parsed as a [`BigDecimal`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ParseDecimalError {
    /// The input was empty, or had no digits where digits were required.
    Empty,
    /// The input contained a character that is not part of a decimal literal.
    InvalidDigit,
    /// The input had more than one `.`, or more than one `e`/`E`.
    MalformedShape,
    /// The exponent did not fit in the supported range.
    ExponentOverflow,
}

impl fmt::Display for ParseDecimalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            ParseDecimalError::Empty => "empty decimal literal",
            ParseDecimalError::InvalidDigit => "invalid character in decimal literal",
            ParseDecimalError::MalformedShape => "malformed decimal literal (stray '.' or 'e')",
            ParseDecimalError::ExponentOverflow => "exponent out of range",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for ParseDecimalError {}

// ===========================================================================
//  Small integer helpers
// ===========================================================================

/// The largest scale magnitude accepted from **untrusted input** — the strict budget
/// [`FromStr`](std::str::FromStr) enforces (rejecting with `ExponentOverflow`).
///
/// This is a **security budget**, not a precision limit: a value is `mantissa × 10^(-scale)`,
/// and operations that line two decimals up to a common scale (or render one) must materialize
/// `10^(scale difference)`. If a parser accepted an unbounded scale, a tiny input like
/// `"1e-2000000000"` (a handful of bytes) could force a `BigInteger` of billions of digits —
/// gigabytes — during a later `+`, `cmp`, or `Display`, none of which can return an error.
/// Bounding the parsed scale keeps every such materialization on parsed values well under a
/// megabyte. A million places on either side of the point is astronomically more than money,
/// tax, or dosing ever needs; callers with genuinely enormous *integers* should use
/// [`BigInteger`] directly.
pub const MAX_SCALE: i64 = 1_000_000;

/// The hard ceiling every constructor (hence every arithmetic result) enforces. It is
/// deliberately **wider** than [`MAX_SCALE`] so that `+ − ×` of two parse-budget operands —
/// whose result scale can reach `2·MAX_SCALE` (a product) or `MAX_SCALE + 1` (an additive
/// carry) — never trips it, i.e. ordinary arithmetic on validated inputs never panics. It
/// still bounds any materialization to a few megabytes; only a pathological explicit
/// [`from_parts`](BigDecimal::from_parts) or a long chain of scale-growing operations can
/// reach it, at which point [`from_parts`](BigDecimal::from_parts) panics (the same "unbounded
/// on purpose, caller's responsibility" contract as [`BigInteger::pow`]).
const INTERNAL_SCALE_LIMIT: i64 = 8_000_000;

/// `10^n` as a [`BigInteger`]. `n` is a `u32` because `10^(2^32)` is already an
/// astronomically large number; a scale difference that would exceed it is not a real value.
/// In practice, the [`MAX_SCALE`] budget keeps every `n` reached here below `2 · MAX_SCALE`.
fn ten_pow(n: u32) -> BigInteger {
    BigInteger::from_i64(10).pow(n)
}

/// Narrow a non-negative scale difference to the `u32` that [`ten_pow`] needs. The argument is
/// `i128` so the difference of two `i64` scales can never overflow before it gets here.
///
/// For any value that came through the [`MAX_SCALE`] budget, `diff` is at most `2·MAX_SCALE`,
/// far inside `u32`. The cap below is a backstop for a caller who supplies an extreme *explicit*
/// target scale (to `div_round`/`round_to_scale`), the same class as [`BigInteger::pow`]'s
/// unbounded exponent.
///
/// # Panics
/// Panics if `diff` is negative (a caller bug) or larger than `u32::MAX`.
fn scale_diff_to_u32(diff: i128) -> u32 {
    assert!(diff >= 0, "internal error: negative scale difference");
    u32::try_from(diff).expect("scale difference too large to materialize (over ~4 billion digits)")
}

// ===========================================================================
//  Construction & normalization
// ===========================================================================

impl BigDecimal {
    /// The value `0`.
    pub fn zero() -> Self {
        BigDecimal {
            mant: BigInteger::zero(),
            scale: 0,
        }
    }

    /// The value `1`.
    pub fn one() -> Self {
        BigDecimal {
            mant: BigInteger::one(),
            scale: 0,
        }
    }

    /// Build `mantissa × 10^(-scale)` and reduce it to canonical form.
    ///
    /// # Panics
    /// Panics only if the canonical scale magnitude would exceed the internal
    /// `INTERNAL_SCALE_LIMIT` — a ceiling wide enough that `+ − ×` of any two
    /// [`MAX_SCALE`]-budget values never reaches it, so ordinary arithmetic on validated inputs
    /// never panics. Reaching it takes a pathological explicit scale or a long chain of
    /// scale-growing operations (the same unbounded-on-purpose contract as [`BigInteger::pow`]).
    /// Use [`checked_from_parts`](Self::checked_from_parts) for the non-panicking form.
    pub fn from_parts(mant: BigInteger, scale: i64) -> Self {
        Self::checked_from_parts(mant, scale)
            .expect("BigDecimal scale magnitude exceeds the internal ceiling")
    }

    /// Build `mantissa × 10^(-scale)`, reduce it to canonical form, and return `None` if the
    /// canonical scale magnitude would exceed the internal ceiling that bounds materialization
    /// (see [`from_parts`](Self::from_parts) and [`MAX_SCALE`]).
    pub fn checked_from_parts(mant: BigInteger, scale: i64) -> Option<Self> {
        let d = (BigDecimal { mant, scale }).normalized();
        if d.scale.unsigned_abs() > INTERNAL_SCALE_LIMIT as u64 {
            None
        } else {
            Some(d)
        }
    }

    /// Promote a whole [`BigInteger`] to a decimal (scale `0`).
    pub fn from_integer(n: BigInteger) -> Self {
        BigDecimal::from_parts(n, 0)
    }

    /// Build from a primitive integer, e.g. `from_i64(42)`.
    pub fn from_i64(n: i64) -> Self {
        BigDecimal::from_parts(BigInteger::from_i64(n), 0)
    }

    /// The unscaled integer mantissa (carries the sign).
    pub fn mantissa(&self) -> &BigInteger {
        &self.mant
    }

    /// The base-10 scale: the value is `mantissa × 10^(-scale)`.
    pub fn scale(&self) -> i64 {
        self.scale
    }

    /// Reduce to canonical form: strip every trailing zero digit from the mantissa (lowering
    /// the scale by one each time, which preserves the value), and pin zero to `(0, 0)`.
    fn normalized(mut self) -> Self {
        if self.mant.is_zero() {
            return BigDecimal::zero();
        }
        let ten = BigInteger::from_i64(10);
        loop {
            let (q, r) = self.mant.div_rem(&ten);
            if !r.is_zero() {
                break;
            }
            // The last digit was 0: mant·10^-s == (mant/10)·10^-(s-1), so drop it and adjust.
            // `saturating_sub` keeps a pathological near-`i64::MIN` parsed scale from underflowing
            // (which would panic under overflow checks) — a scale anywhere near `i64::MIN` is
            // astronomically past every ceiling, so the caller's budget check rejects it anyway.
            self.mant = q;
            self.scale = self.scale.saturating_sub(1);
        }
        self
    }
}

// ===========================================================================
//  Predicates & sign
// ===========================================================================

impl BigDecimal {
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

    /// How many **significant digits** the value carries — the count of digits in the mantissa
    /// once trailing zeros are removed. Because a `BigDecimal` is always kept in canonical form
    /// (the [module documentation](crate::decimal) explains: the mantissa never ends in a `0`
    /// unless the value *is* zero), this is simply the number of base-10 digits in the mantissa,
    /// and `0` for the value zero. The scale (where the decimal point sits) is irrelevant — only
    /// the run of meaningful digits counts.
    ///
    /// | value    | canonical `(mant, scale)` | significant digits |
    /// |----------|---------------------------|--------------------|
    /// | `0`      | `(0, 0)`                  | `0`                |
    /// | `100`    | `(1, -2)`                 | `1`                |
    /// | `20.180` | `(2018, 2)`               | `4`                |
    /// | `-3.14`  | `(-314, 2)`               | `3`                |
    /// | π (39dp) | `(3141…197, 39)`          | `40`               |
    ///
    /// This is the measure the exact-numbers renderer (ADJ NX-2) uses to decide whether a value
    /// still fits an `f64`'s ~17-digit budget (render the `f64` canonical form, preserving
    /// existing output byte-for-byte) or exceeds it (render every exact digit instead).
    pub fn significant_digits(&self) -> usize {
        if self.mant.is_zero() {
            return 0;
        }
        // The mantissa carries no trailing zero in canonical form, so its digit count *is* the
        // significant-digit count. `abs()` drops a leading `-` so the sign never inflates the
        // length.
        self.mant.abs().to_string().len()
    }

    /// The absolute value `|self|`.
    pub fn abs(&self) -> BigDecimal {
        BigDecimal {
            mant: self.mant.abs(),
            scale: self.scale,
        }
    }
}

// ===========================================================================
//  Scale alignment (shared by +, -, and comparison)
// ===========================================================================

impl BigDecimal {
    /// Return the two mantissas re-expressed at a common scale `max(self.scale, other.scale)`,
    /// together with that scale. Re-expressing to a *larger* scale multiplies the mantissa by
    /// a power of ten and is exact (it only appends zeros); we never round here.
    fn aligned_mantissas(&self, other: &BigDecimal) -> (BigInteger, BigInteger, i64) {
        let target = self.scale.max(other.scale);
        // `i128` differences cannot overflow (both operands are `i64`); the `MAX_SCALE` budget
        // keeps each difference at most `2·MAX_SCALE`, so no oversized power is materialized.
        let a = &self.mant * &ten_pow(scale_diff_to_u32(target as i128 - self.scale as i128));
        let b = &other.mant * &ten_pow(scale_diff_to_u32(target as i128 - other.scale as i128));
        (a, b, target)
    }
}

// ===========================================================================
//  Lossy export
// ===========================================================================

impl BigDecimal {
    /// A **lossy** narrowing to the nearest `f64` — the *labeled lossy boundary* for interop and
    /// display. `BigDecimal` is exact base-10; `f64` is binary, so a value like `0.1` cannot be
    /// represented and this deliberately rounds. A caller that needs the exact money/percent
    /// value keeps the `BigDecimal`.
    ///
    /// It goes through the value's own plain-decimal string and Rust's **correctly-rounded**
    /// float parser, so the result is the true nearest `f64` (including in the subnormal range),
    /// and out-of-range magnitudes saturate to `±∞` / `0` exactly as the parser does.
    pub fn to_f64(&self) -> f64 {
        self.to_string().parse::<f64>().unwrap_or(f64::NAN)
    }
}

// ===========================================================================
//  Exact export to BigRational
// ===========================================================================

impl BigDecimal {
    /// The **exact** value as a [`BigRational`] — no rounding, no `f64` hop.
    ///
    /// A [`BigDecimal`] is `mantissa × 10^(-scale)`, so it is *always* a ratio of two integers
    /// and converts to a fraction with **zero loss**. This is the exact counterpart of the
    /// lossy [`to_f64`](Self::to_f64): where `to_f64` narrows to the nearest binary float,
    /// `to_rational` hands back the true value that a rational engine can keep computing on
    /// exactly (see ADJ-EXACT-NUMBERS NX-3 — exact compute ingestion).
    ///
    /// The scale sign picks the shape of the fraction:
    ///
    /// | value | mantissa | scale | `to_rational()` |
    /// |---|---|---|---|
    /// | `2.54`   | `254`  | `2`  | `254 / 100` → `127/50`  (scale > 0 ⇒ denominator `10^scale`) |
    /// | `0.3048` | `3048` | `4`  | `3048 / 10000` → `381/1250` |
    /// | `100`    | `1`    | `-2` | `1 × 10^2` → `100/1`  (scale ≤ 0 ⇒ whole number `mant · 10^|scale|`) |
    /// | `0`      | `0`    | `0`  | `0/1` |
    ///
    /// [`BigRational::new`] reduces to lowest terms and moves the sign onto the numerator, so the
    /// result is canonical. The mantissa already carries the sign, so no separate sign handling
    /// is needed here.
    pub fn to_rational(&self) -> BigRational {
        if self.scale > 0 {
            // Fractional digits: the value is `mantissa / 10^scale`.
            let denom = ten_pow(scale_diff_to_u32(self.scale as i128));
            BigRational::new(self.mant.clone(), denom)
        } else {
            // scale <= 0: the value is a whole number `mantissa × 10^(-scale)`.
            // (`scale == 0` folds in here too, since `10^0 == 1`.)
            let factor = ten_pow(scale_diff_to_u32(-(self.scale as i128)));
            BigRational::from_integer(&self.mant * &factor)
        }
    }

    /// The **exact** base-10 expansion of a rational, when a finite one exists — the inverse
    /// direction of [`to_rational`](Self::to_rational), and the rendering half of the
    /// ADJ-EXACT-NUMBERS arc (NX-4).
    ///
    /// ## When does a fraction have a *finite* decimal?
    ///
    /// Write the rational in lowest terms as `p / q` (a [`BigRational`] is always kept reduced
    /// with a positive denominator). Its base-10 expansion terminates **iff** `q`'s only prime
    /// factors are the primes of the base, `2` and `5`:
    ///
    /// | fraction | reduced `q` | factors of `q` | expansion |
    /// |---|---|---|---|
    /// | `1/4`   | `4`    | `2²`      | `0.25`  — terminates |
    /// | `3/8`   | `8`    | `2³`      | `0.375` — terminates |
    /// | `7/20`  | `20`   | `2²·5`    | `0.35`  — terminates |
    /// | `1/3`   | `3`    | `3`       | `0.333…` — **repeats**, no finite decimal |
    /// | `2/7`   | `7`    | `7`       | `0.285714…` — **repeats** |
    ///
    /// So we strip every factor of `2` and `5` out of `q`, counting how many of each; if anything
    /// but `1` is left over, the expansion repeats and we return `None` (the caller then falls
    /// back to a labeled-lossy `f64` — a repeating value simply has no exact `BigDecimal`).
    ///
    /// ## Building the mantissa when it *does* terminate
    ///
    /// After stripping we know `q = 2^a · 5^b`. We want a scale `s` and mantissa `m` with
    /// `p / q = m / 10^s`. Choosing `s = max(a, b)` makes `10^s / q = 2^(s−a) · 5^(s−b)` a whole
    /// number, so `m = p · 2^(s−a) · 5^(s−b)` is exact — we scale numerator and denominator by the
    /// same factor, which cannot change the value. Handing `(m, s)` to [`from_parts`](Self::from_parts)
    /// (which strips trailing zeros into canonical form) gives the terminating decimal, sign and all.
    ///
    /// ```
    /// # use bignum_core::{BigDecimal, BigRational, BigInteger};
    /// let quarter = BigRational::from_ints(1, 4);
    /// assert_eq!(BigDecimal::from_rational_exact(&quarter).unwrap().to_string(), "0.25");
    /// let third = BigRational::from_ints(1, 3);
    /// assert!(BigDecimal::from_rational_exact(&third).is_none()); // repeats — no finite decimal
    /// ```
    pub fn from_rational_exact(r: &BigRational) -> Option<BigDecimal> {
        let two = BigInteger::from_i64(2);
        let five = BigInteger::from_i64(5);

        // Strip factors of 2, then of 5, from the (positive) denominator, tallying each.
        let mut den = r.denominator().clone();
        let mut twos: i64 = 0;
        let mut fives: i64 = 0;
        loop {
            let (q, rem) = den.div_rem(&two);
            if rem.is_zero() {
                den = q;
                twos += 1;
            } else {
                break;
            }
        }
        loop {
            let (q, rem) = den.div_rem(&five);
            if rem.is_zero() {
                den = q;
                fives += 1;
            } else {
                break;
            }
        }
        // Any leftover prime factor (3, 7, 11, …) ⇒ the expansion repeats ⇒ no finite decimal.
        if den != BigInteger::one() {
            return None;
        }

        // Rebalance so the denominator is exactly 10^scale, scaling the numerator to match.
        let scale = twos.max(fives);
        let mut mant = r.numerator().clone();
        if scale > twos {
            mant = &mant * &two.pow((scale - twos) as u32);
        }
        if scale > fives {
            mant = &mant * &five.pow((scale - fives) as u32);
        }
        // `checked_from_parts` keeps this a **total** function: a value whose exact scale would
        // exceed `BigDecimal`'s internal scale budget (an astronomically fine fraction like
        // `1 / 2^8_000_001`) yields `None` — the same "no representable exact decimal" signal a
        // repeating expansion gives — rather than panicking. In every real caller the rational is
        // already size-bounded well under the budget, so this branch is defence-in-depth.
        BigDecimal::checked_from_parts(mant, scale)
    }
}

// ===========================================================================
//  Exact arithmetic (+, -, *) and rounding division
// ===========================================================================

impl BigDecimal {
    /// Exact sum. Align to the finer scale, add the mantissas, renormalize.
    pub fn add(&self, other: &BigDecimal) -> BigDecimal {
        let (a, b, scale) = self.aligned_mantissas(other);
        BigDecimal::from_parts(&a + &b, scale)
    }

    /// Exact difference.
    pub fn sub(&self, other: &BigDecimal) -> BigDecimal {
        let (a, b, scale) = self.aligned_mantissas(other);
        BigDecimal::from_parts(&a - &b, scale)
    }

    /// Exact product. `(m1·10^-s1)·(m2·10^-s2) = (m1·m2)·10^-(s1+s2)`, so multiply the
    /// mantissas and add the scales.
    pub fn mul(&self, other: &BigDecimal) -> BigDecimal {
        let scale = self
            .scale
            .checked_add(other.scale)
            .expect("scale overflow in multiplication");
        BigDecimal::from_parts(&self.mant * &other.mant, scale)
    }

    /// Raise to a non-negative integer power (exact). `(m·10^-s)^e = m^e · 10^-(s·e)`.
    ///
    /// # Panics
    /// Panics if `s·e` overflows `i64`. Like [`BigInteger::pow`], this is unbounded in the size
    /// of the result — a large exponent on a many-digit base can exhaust memory.
    pub fn pow(&self, exp: u32) -> BigDecimal {
        let scale = self
            .scale
            .checked_mul(exp as i64)
            .expect("scale overflow in pow");
        BigDecimal::from_parts(self.mant.pow(exp), scale)
    }

    /// Divide, rounding the result to exactly `target_scale` decimal places with `mode`.
    ///
    /// Division is the one base-10 operation that need not terminate (`10/3`), so — unlike
    /// `+ − ×` — it always rounds, and you say to how many places and how. The result has
    /// scale `target_scale` before canonical trailing-zero stripping.
    ///
    /// # Panics
    /// Panics if `other` is zero. Use [`checked_div_round`](Self::checked_div_round) for the
    /// non-panicking form.
    pub fn div_round(&self, other: &BigDecimal, target_scale: i64, mode: RoundingMode) -> BigDecimal {
        self.checked_div_round(other, target_scale, mode)
            .expect("division by zero")
    }

    /// Divide with rounding, or `None` if `other` is zero.
    ///
    /// We want `R` such that `R·10^-target_scale ≈ self/other`. Writing `self = m1·10^-s1` and
    /// `other = m2·10^-s2`, that is `R = round( m1 · 10^(s2 - s1 + target_scale) / m2 )`. The
    /// exponent `e = s2 - s1 + target_scale` is applied to whichever side keeps both operands
    /// integers, then a single [`round_div`] does the rounding.
    pub fn checked_div_round(
        &self,
        other: &BigDecimal,
        target_scale: i64,
        mode: RoundingMode,
    ) -> Option<BigDecimal> {
        if other.is_zero() {
            return None;
        }
        let e = target_scale
            .checked_add(other.scale)
            .and_then(|x| x.checked_sub(self.scale))
            .expect("scale overflow in division");
        let rounded = if e >= 0 {
            let num = &self.mant * &ten_pow(scale_diff_to_u32(e as i128));
            round_div(&num, &other.mant, mode)
        } else {
            // `-(e as i128)` avoids the `i64::MIN` negation overflow that `-e` would hit.
            let den = &other.mant * &ten_pow(scale_diff_to_u32(-(e as i128)));
            round_div(&self.mant, &den, mode)
        };
        Some(BigDecimal::from_parts(rounded, target_scale))
    }

    /// Round to `target_scale` decimal places with `mode`. Increasing the scale is exact and
    /// changes nothing (the extra places are zeros, which canonicalization strips again);
    /// decreasing it drops digits and rounds.
    pub fn round_to_scale(&self, target_scale: i64, mode: RoundingMode) -> BigDecimal {
        if target_scale >= self.scale {
            // Already exactly representable at this (or a coarser-mantissa) scale.
            return self.clone();
        }
        let drop = scale_diff_to_u32(self.scale as i128 - target_scale as i128);
        let rounded = round_div(&self.mant, &ten_pow(drop), mode);
        BigDecimal::from_parts(rounded, target_scale)
    }
}

/// Round the exact quotient `n / d` (with `d != 0`) to the nearest integer under `mode`.
///
/// Everything is decided from the truncating quotient `q` and remainder `r` of the
/// magnitudes: `|n| = q·|d| + r` with `0 ≤ r < |d|`. If `r == 0` the quotient is exact.
/// Otherwise the fractional part is `r/|d|`, and we compare `2r` with `|d|` to place it below,
/// at, or above the halfway mark. The result's sign is the sign of `n·d`.
fn round_div(n: &BigInteger, d: &BigInteger, mode: RoundingMode) -> BigInteger {
    debug_assert!(!d.is_zero());
    let sign = n.signum() * d.signum(); // -1, 0, or +1
    if sign == 0 {
        return BigInteger::zero(); // n == 0
    }
    let na = n.abs();
    let da = d.abs();
    let (q, r) = na.div_rem(&da);
    if r.is_zero() {
        return apply_sign(q, sign);
    }
    let two_r = &r * &BigInteger::from_i64(2);
    let half_cmp = two_r.cmp(&da); // r vs d/2
    let q_is_odd = !(&q % &BigInteger::from_i64(2)).is_zero();
    let round_away = match mode {
        RoundingMode::Down => false,
        RoundingMode::Up => true,
        RoundingMode::Floor => sign < 0,   // toward -inf: negatives round away
        RoundingMode::Ceiling => sign > 0, // toward +inf: positives round away
        RoundingMode::HalfUp => half_cmp != Ordering::Less,
        RoundingMode::HalfDown => half_cmp == Ordering::Greater,
        RoundingMode::HalfEven => {
            half_cmp == Ordering::Greater || (half_cmp == Ordering::Equal && q_is_odd)
        }
    };
    let magnitude = if round_away {
        &q + &BigInteger::one()
    } else {
        q
    };
    apply_sign(magnitude, sign)
}

/// Attach `sign` (`-1`/`+1`) to a non-negative magnitude.
fn apply_sign(magnitude: BigInteger, sign: i32) -> BigInteger {
    if sign < 0 {
        -magnitude
    } else {
        magnitude
    }
}

// ===========================================================================
//  Ordering
// ===========================================================================

impl Ord for BigDecimal {
    /// Compare by re-expressing both mantissas at a common scale (exact) and comparing them.
    fn cmp(&self, other: &Self) -> Ordering {
        let (a, b, _) = self.aligned_mantissas(other);
        a.cmp(&b)
    }
}

impl PartialOrd for BigDecimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ===========================================================================
//  Formatting
// ===========================================================================

impl fmt::Display for BigDecimal {
    /// Renders in plain decimal notation (never scientific): `100`, `1.23`, `0.001`, `-0.5`,
    /// `0`. The canonical `(mantissa, scale)` is expanded by placing the decimal point `scale`
    /// digits from the right of the mantissa's digits (padding with zeros as needed).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mant.is_zero() {
            return f.write_str("0");
        }
        let neg = self.mant.is_negative();
        let digits = self.mant.abs().to_string(); // base-10 digits, no sign
        let out = if self.scale <= 0 {
            // Whole number with |scale| trailing zeros appended.
            let zeros = self.scale.unsigned_abs() as usize; // scale <= 0 here; avoids i64::MIN negation
            format!("{digits}{}", "0".repeat(zeros))
        } else {
            let s = self.scale as usize;
            let len = digits.len();
            if len > s {
                // Point sits inside the digit string.
                let (int_part, frac_part) = digits.split_at(len - s);
                format!("{int_part}.{frac_part}")
            } else {
                // Value is < 1: "0." then enough leading zeros to place the digits.
                let leading = "0".repeat(s - len);
                format!("0.{leading}{digits}")
            }
        };
        if neg {
            write!(f, "-{out}")
        } else {
            f.write_str(&out)
        }
    }
}

impl fmt::Debug for BigDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BigDecimal({} × 10^{})", self.mant, -self.scale)
    }
}

// ===========================================================================
//  Parsing
// ===========================================================================

impl FromStr for BigDecimal {
    type Err = ParseDecimalError;

    /// Parses plain (`"123.45"`, `"-0.001"`, `"42"`) and scientific (`"1.5e-3"`, `"6.022E23"`)
    /// decimal notation. The mantissa is the integer and fractional digits concatenated; the
    /// scale is `(number of fractional digits) − (exponent)`. Whitespace is not trimmed.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseDecimalError::Empty);
        }
        // --- optional leading sign -------------------------------------------------
        let bytes = s.as_bytes();
        let mut i = 0usize;
        let mut negative = false;
        if bytes[0] == b'+' || bytes[0] == b'-' {
            negative = bytes[0] == b'-';
            i = 1;
        }
        // --- split off an optional exponent ('e'/'E') ------------------------------
        let mantissa_str = &s[i..];
        let (digits_part, exp_part) = match mantissa_str.split_once(['e', 'E']) {
            Some((d, e)) => (d, Some(e)),
            None => (mantissa_str, None),
        };
        // --- integer and fractional digit groups -----------------------------------
        let mut dot_split = digits_part.split('.');
        let int_digits = dot_split.next().unwrap_or("");
        let frac_digits = dot_split.next().unwrap_or("");
        if dot_split.next().is_some() {
            return Err(ParseDecimalError::MalformedShape); // two dots
        }
        if int_digits.is_empty() && frac_digits.is_empty() {
            return Err(ParseDecimalError::Empty);
        }
        if !int_digits.bytes().all(|b| b.is_ascii_digit())
            || !frac_digits.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(ParseDecimalError::InvalidDigit);
        }
        // --- the exponent, if any --------------------------------------------------
        let exp: i64 = match exp_part {
            None => 0,
            Some(e) => e.parse::<i64>().map_err(|_| {
                if e.bytes().all(|b| b.is_ascii_digit() || b == b'+' || b == b'-') && !e.is_empty() {
                    ParseDecimalError::ExponentOverflow
                } else {
                    ParseDecimalError::InvalidDigit
                }
            })?,
        };
        // --- assemble the mantissa integer and the scale ---------------------------
        let mut all_digits = String::with_capacity(int_digits.len() + frac_digits.len() + 1);
        if negative {
            all_digits.push('-');
        }
        all_digits.push_str(int_digits);
        all_digits.push_str(frac_digits);
        // A group like ".5" or "5." leaves one side empty; guard the all-zero-length mantissa.
        let mant = if all_digits.is_empty() || all_digits == "-" {
            BigInteger::zero()
        } else {
            BigInteger::from_str(&all_digits).map_err(|_| ParseDecimalError::InvalidDigit)?
        };
        // fractional digits push the point right (scale up); the exponent pushes it left.
        let scale = (frac_digits.len() as i64)
            .checked_sub(exp)
            .ok_or(ParseDecimalError::ExponentOverflow)?;
        // Canonicalize first (this can *change* the scale: trailing-zero stripping lowers it,
        // which *grows* `|scale|` when the scale is negative — e.g. "100e999999" normalizes to
        // scale -1000001), then enforce the strict MAX_SCALE budget on the *stored* scale. This
        // is the untrusted-input boundary: no parsed value may exceed MAX_SCALE, so a later
        // `+`/`cmp`/`Display` cannot be forced to materialize a huge power of ten, and (because
        // parse outputs really are ≤ MAX_SCALE) arithmetic stays comfortably under the internal
        // ceiling. `checked_from_parts` returns `None` rather than panicking on an extreme input.
        let d = BigDecimal::checked_from_parts(mant, scale).ok_or(ParseDecimalError::ExponentOverflow)?;
        if d.scale().unsigned_abs() > MAX_SCALE as u64 {
            return Err(ParseDecimalError::ExponentOverflow);
        }
        Ok(d)
    }
}

// ===========================================================================
//  Conversions & operator overloads
// ===========================================================================

impl From<BigInteger> for BigDecimal {
    fn from(n: BigInteger) -> Self {
        BigDecimal::from_integer(n)
    }
}

macro_rules! impl_from_primitive {
    ($t:ty, $ctor:ident) => {
        impl From<$t> for BigDecimal {
            fn from(v: $t) -> Self {
                BigDecimal::from_parts(BigInteger::$ctor(v), 0)
            }
        }
    };
}
impl_from_primitive!(i64, from_i64);
impl_from_primitive!(u64, from_u64);
impl_from_primitive!(i128, from_i128);
impl_from_primitive!(u128, from_u128);

// The inherent methods are called through the fully-qualified `BigDecimal::$inherent(a, b)`
// form so the `Div`-style name collision that bites `std::ops` never arises.
macro_rules! impl_binop {
    ($trait:ident, $method:ident, $inherent:ident) => {
        impl std::ops::$trait for BigDecimal {
            type Output = BigDecimal;
            fn $method(self, rhs: BigDecimal) -> BigDecimal {
                BigDecimal::$inherent(&self, &rhs)
            }
        }
        impl std::ops::$trait<&BigDecimal> for &BigDecimal {
            type Output = BigDecimal;
            fn $method(self, rhs: &BigDecimal) -> BigDecimal {
                BigDecimal::$inherent(self, rhs)
            }
        }
    };
}
impl_binop!(Add, add, add);
impl_binop!(Sub, sub, sub);
impl_binop!(Mul, mul, mul);

impl std::ops::Neg for BigDecimal {
    type Output = BigDecimal;
    fn neg(self) -> BigDecimal {
        BigDecimal {
            mant: -self.mant,
            scale: self.scale,
        }
    }
}

impl std::ops::Neg for &BigDecimal {
    type Output = BigDecimal;
    fn neg(self) -> BigDecimal {
        BigDecimal {
            mant: -&self.mant,
            scale: self.scale,
        }
    }
}

// ===========================================================================
//  Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::str::FromStr;

    fn d(s: &str) -> BigDecimal {
        BigDecimal::from_str(s).unwrap()
    }

    // ---- lossy f64 export -----------------------------------------------

    #[test]
    fn to_f64_is_the_correctly_rounded_nearest() {
        // Exactly-representable decimals round-trip; others match parsing the literal.
        assert_eq!(d("0.5").to_f64(), 0.5);
        assert_eq!(d("100.00").to_f64(), 100.0);
        assert_eq!(d("-2.25").to_f64(), -2.25);
        assert_eq!(d("0").to_f64(), 0.0);
        // 0.1 has no exact f64; the narrowing is the same nearest f64 the literal parses to.
        assert_eq!(d("0.1").to_f64(), 0.1);
        assert_eq!(d("0.3").to_f64(), 0.3);
        assert_eq!(d("123.456").to_f64(), 123.456);
        assert_eq!(d("6.02214076e23").to_f64(), 6.02214076e23);
        // Out-of-range magnitudes saturate exactly as the parser does.
        let huge = "1".to_string() + &"0".repeat(400); // 10^400
        assert_eq!(d(&huge).to_f64(), f64::INFINITY);
    }

    // ---- exact BigRational export --------------------------------------

    #[test]
    fn to_rational_is_exact_for_fractional_decimals() {
        // 2.54 = 254/100 = 127/50 in lowest terms.
        let r = d("2.54").to_rational();
        assert_eq!(r.numerator().to_string(), "127");
        assert_eq!(r.denominator().to_string(), "50");
        // 0.3048 = 3048/10000 = 381/1250.
        let r = d("0.3048").to_rational();
        assert_eq!(r.numerator().to_string(), "381");
        assert_eq!(r.denominator().to_string(), "1250");
    }

    #[test]
    fn to_rational_handles_integers_and_zero_and_signs() {
        // Whole numbers land on denominator 1 (scale <= 0 branch).
        assert_eq!(d("100").to_rational(), BigRational::from_ints(100, 1));
        assert_eq!(d("7").to_rational(), BigRational::from_ints(7, 1));
        // 12300 canonicalizes to mantissa 123, scale -2, then rehydrates exactly.
        assert_eq!(d("12300").to_rational(), BigRational::from_ints(12300, 1));
        // Zero is the canonical 0/1.
        assert_eq!(d("0").to_rational(), BigRational::zero());
        // Sign rides on the mantissa; new() keeps the denominator positive.
        let r = d("-2.5").to_rational();
        assert_eq!(r.numerator().to_string(), "-5");
        assert_eq!(r.denominator().to_string(), "2");
        assert!(r.is_negative());
    }

    #[test]
    fn to_rational_is_exact_for_39_digit_pi() {
        // The stdlib ships pi to 39 digits. Its exact rational value is
        // 3141592653589793238462643383279502884197 / 10^39 (already coprime, so it
        // does not reduce). No f64 hop, so all 39 digits survive.
        let pi = d("3.141592653589793238462643383279502884197");
        let r = pi.to_rational();
        assert_eq!(
            r.numerator().to_string(),
            "3141592653589793238462643383279502884197"
        );
        assert_eq!(
            r.denominator().to_string(),
            "1000000000000000000000000000000000000000" // 10^39
        );
        // Round-trip sanity: numerator has 40 digits, denominator has 40 (1 + 39 zeros).
        assert_eq!(r.numerator().to_string().len(), 40);
        assert_eq!(r.denominator().to_string().len(), 40);
    }

    // ---- canonical form & display --------------------------------------

    #[test]
    fn canonical_strips_trailing_zeros() {
        assert_eq!(d("1.230").to_string(), "1.23");
        assert_eq!(d("1.230").scale(), 2);
        // 100 canonicalizes to mantissa 1, scale -2, but still displays as "100".
        let hundred = d("100");
        assert_eq!(hundred.mantissa().to_string(), "1");
        assert_eq!(hundred.scale(), -2);
        assert_eq!(hundred.to_string(), "100");
        assert_eq!(d("12300").to_string(), "12300");
    }

    #[test]
    fn significant_digits_counts_meaningful_mantissa_digits() {
        // Zero has no significant digits.
        assert_eq!(d("0").significant_digits(), 0);
        assert_eq!(d("0.000").significant_digits(), 0);
        // Trailing zeros are not significant (canonical form already strips them).
        assert_eq!(d("20.180").significant_digits(), 4); // 2018
        assert_eq!(d("100").significant_digits(), 1); // mantissa 1
        assert_eq!(d("1000").significant_digits(), 1);
        assert_eq!(d("12300").significant_digits(), 3); // 123
        // Leading zeros before the first non-zero digit are not significant either.
        assert_eq!(d("0.001").significant_digits(), 1);
        assert_eq!(d("0.0123").significant_digits(), 3);
        // Ordinary values.
        assert_eq!(d("1.23").significant_digits(), 3);
        assert_eq!(d("-3.14").significant_digits(), 3); // sign does not count
        assert_eq!(d("42").significant_digits(), 2);
        // The f64 fit/overflow boundary the renderer keys on: 17 digits vs 18.
        assert_eq!(d("1.2345678901234567").significant_digits(), 17);
        assert_eq!(d("1.23456789012345678").significant_digits(), 18);
        // π to 39 decimal places — the motivating case (every digit counts). The literal has one
        // integer digit plus 39 fractional digits, so 40 significant digits in all.
        let pi = "3.141592653589793238462643383279502884197";
        assert_eq!(d(pi).significant_digits(), 40);
        // Scientific notation is normalized first, so only real digits count.
        assert_eq!(d("6.02214076e23").significant_digits(), 9);
        assert_eq!(d("1e3").significant_digits(), 1); // 1000 -> mantissa 1
    }

    #[test]
    fn display_places_the_point_correctly() {
        assert_eq!(d("123.45").to_string(), "123.45");
        assert_eq!(d("0.001").to_string(), "0.001");
        assert_eq!(d("0.0123").to_string(), "0.0123");
        assert_eq!(d("-0.5").to_string(), "-0.5");
        assert_eq!(d("0").to_string(), "0");
        assert_eq!(d("-0").to_string(), "0"); // negative zero collapses
        assert_eq!(BigDecimal::from_i64(42).to_string(), "42");
    }

    #[test]
    fn zero_is_canonical() {
        assert_eq!(d("0.00"), d("0"));
        assert_eq!(d("0e5"), d("0"));
        assert_eq!(d("-0.0"), BigDecimal::zero());
        assert!(d("0.000").is_zero());
        assert_eq!(d("0").scale(), 0);
    }

    // ---- equality & hash ------------------------------------------------

    #[test]
    fn equality_and_hash_follow_value() {
        assert_eq!(d("1.20"), d("1.2"));
        assert_eq!(d("100"), d("1e2"));
        let mut m: HashMap<BigDecimal, &str> = HashMap::new();
        m.insert(d("1.20"), "buck-twenty");
        assert_eq!(m.get(&d("1.2")), Some(&"buck-twenty"));
        assert_eq!(m.get(&d("1.3")), None);
    }

    // ---- parsing --------------------------------------------------------

    #[test]
    fn parse_plain_and_scientific() {
        assert_eq!(d("1.5e-3").to_string(), "0.0015");
        assert_eq!(d("6.022E23").to_string(), "602200000000000000000000");
        assert_eq!(d("1e3").to_string(), "1000");
        assert_eq!(d("-0.001").to_string(), "-0.001");
        assert_eq!(d("+42").to_string(), "42");
        assert_eq!(d(".5").to_string(), "0.5");
        assert_eq!(d("5.").to_string(), "5");
    }

    #[test]
    fn parse_round_trips() {
        for s in ["0.1", "0.2", "99.99", "-3.14159", "1000", "0.0015", "0"] {
            let parsed = d(s);
            assert_eq!(BigDecimal::from_str(&parsed.to_string()).unwrap(), parsed);
        }
    }

    #[test]
    fn parse_errors_are_typed() {
        assert_eq!(BigDecimal::from_str(""), Err(ParseDecimalError::Empty));
        assert_eq!(BigDecimal::from_str("1.2.3"), Err(ParseDecimalError::MalformedShape));
        assert_eq!(BigDecimal::from_str("1x2"), Err(ParseDecimalError::InvalidDigit));
        assert_eq!(BigDecimal::from_str("."), Err(ParseDecimalError::Empty));
        assert_eq!(BigDecimal::from_str("1e"), Err(ParseDecimalError::InvalidDigit));
        assert_eq!(BigDecimal::from_str("abc"), Err(ParseDecimalError::InvalidDigit));
    }

    // ---- exact +, -, * (pinned against Python's decimal) ----------------

    #[test]
    fn exact_add_sub_mul() {
        assert_eq!((&d("0.1") + &d("0.2")).to_string(), "0.3"); // the float trap
        assert_eq!((&d("1.23") + &d("4.5")).to_string(), "5.73");
        assert_eq!((&d("100") - &d("0.01")).to_string(), "99.99");
        assert_eq!((&d("1.5") * &d("1.5")).to_string(), "2.25");
        assert_eq!((&d("2.5") * &d("4")).to_string(), "10");
        assert_eq!((&d("12345.678") * &d("1000")).to_string(), "12345678");
        assert_eq!((&d("-1.5") * &d("0.2")).to_string(), "-0.3");
    }

    #[test]
    fn owned_and_borrowed_operators_agree() {
        assert_eq!(d("0.1") + d("0.2"), &d("0.1") + &d("0.2"));
        assert_eq!(d("1.5") * d("1.5"), &d("1.5") * &d("1.5"));
        assert_eq!(-d("1.25"), -&d("1.25"));
    }

    #[test]
    fn pow_is_exact() {
        assert_eq!(d("1.1").pow(2).to_string(), "1.21");
        assert_eq!(d("2").pow(10).to_string(), "1024");
        assert_eq!(d("0.5").pow(3).to_string(), "0.125");
        assert_eq!(d("10").pow(0).to_string(), "1");
    }

    // ---- rounding modes (full Python truth table) -----------------------

    #[test]
    fn rounding_modes_on_halves() {
        use RoundingMode::*;
        // (value, mode) -> integer result, from Python's decimal.quantize.
        let cases = [
            ("2.5", HalfUp, "3"),
            ("2.5", HalfEven, "2"),
            ("2.5", HalfDown, "2"),
            ("2.5", Down, "2"),
            ("2.5", Up, "3"),
            ("2.5", Floor, "2"),
            ("2.5", Ceiling, "3"),
            ("-2.5", HalfUp, "-3"),
            ("-2.5", HalfEven, "-2"),
            ("-2.5", HalfDown, "-2"),
            ("-2.5", Down, "-2"),
            ("-2.5", Up, "-3"),
            ("-2.5", Floor, "-3"),
            ("-2.5", Ceiling, "-2"),
        ];
        for (val, mode, want) in cases {
            assert_eq!(d(val).round_to_scale(0, mode).to_string(), want, "{val} {mode:?}");
        }
    }

    #[test]
    fn rounding_to_one_place() {
        use RoundingMode::*;
        assert_eq!(d("1.25").round_to_scale(1, HalfUp).to_string(), "1.3");
        assert_eq!(d("1.25").round_to_scale(1, HalfEven).to_string(), "1.2"); // 2 is even
        assert_eq!(d("1.35").round_to_scale(1, HalfEven).to_string(), "1.4"); // 4 is even
        assert_eq!(d("1.35").round_to_scale(1, HalfUp).to_string(), "1.4");
    }

    #[test]
    fn round_to_larger_scale_is_a_noop() {
        // Increasing precision cannot change an exact value.
        assert_eq!(d("1.5").round_to_scale(5, RoundingMode::HalfUp), d("1.5"));
        assert_eq!(d("100").round_to_scale(3, RoundingMode::Down), d("100"));
    }

    // ---- rounding division (pinned against Python) ----------------------

    #[test]
    fn div_round_matches_python() {
        use RoundingMode::*;
        assert_eq!(d("10").div_round(&d("3"), 4, HalfEven).to_string(), "3.3333");
        assert_eq!(d("2").div_round(&d("3"), 2, HalfUp).to_string(), "0.67");
        assert_eq!(d("1").div_round(&d("8"), 3, HalfEven).to_string(), "0.125");
        assert_eq!(d("100").div_round(&d("7"), 6, Down).to_string(), "14.285714");
        assert_eq!(d("-10").div_round(&d("3"), 2, Floor).to_string(), "-3.34");
        assert_eq!(d("1").div_round(&d("3"), 0, HalfUp).to_string(), "0");
        // Exact divisions land exactly regardless of mode.
        assert_eq!(d("1").div_round(&d("4"), 10, HalfEven).to_string(), "0.25");
    }

    #[test]
    fn checked_div_round_handles_zero() {
        assert!(d("1").checked_div_round(&d("0"), 2, RoundingMode::HalfUp).is_none());
        assert!(d("1").checked_div_round(&d("3"), 2, RoundingMode::HalfUp).is_some());
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn div_round_by_zero_panics() {
        let _ = d("1").div_round(&d("0"), 2, RoundingMode::HalfUp);
    }

    // ---- security: the MAX_SCALE budget bounds materialization ----------

    #[test]
    fn from_str_rejects_scale_amplification_payloads() {
        // A few-byte string must NOT be storable with a billions-of-digits scale, or a later
        // `+`/`cmp`/`Display` could be forced to materialize a multi-gigabyte power of ten.
        assert_eq!(
            BigDecimal::from_str("1e-2000000000"),
            Err(ParseDecimalError::ExponentOverflow)
        );
        assert_eq!(
            BigDecimal::from_str("1e2000000000"),
            Err(ParseDecimalError::ExponentOverflow)
        );
        // An exponent that doesn't even fit i64 is also rejected, not wrapped.
        assert_eq!(
            BigDecimal::from_str("1e99999999999999999999"),
            Err(ParseDecimalError::ExponentOverflow)
        );
        // Subtle: the parsed scale (-999999) is within budget, but trailing-zero normalization
        // drives the *stored* scale to -1000001 (over budget). The check is on the canonical
        // scale, so this is rejected too — otherwise a value past MAX_SCALE could sneak in.
        assert_eq!(
            BigDecimal::from_str("100e999999"),
            Err(ParseDecimalError::ExponentOverflow)
        );
        // A parsed scale exactly at the budget with no leak is still fine.
        assert!(BigDecimal::from_str("1e-1000000").is_ok());
        // Pathological: a near-i64::MIN parsed scale with trailing zeros must return an error,
        // NOT panic on an i64 underflow inside normalization (saturating_sub guards it).
        assert_eq!(
            BigDecimal::from_str("100e9223372036854775807"),
            Err(ParseDecimalError::ExponentOverflow)
        );
        assert_eq!(
            BigDecimal::from_str("1000e9223372036854775806"),
            Err(ParseDecimalError::ExponentOverflow)
        );
    }

    #[test]
    fn checked_from_parts_enforces_the_internal_ceiling() {
        use super::INTERNAL_SCALE_LIMIT;
        // Construction (and every arithmetic result) is bounded by the wider internal ceiling,
        // not the strict parse budget — so results a bit past MAX_SCALE still exist.
        assert!(BigDecimal::checked_from_parts(BigInteger::one(), MAX_SCALE + 1).is_some());
        assert!(BigDecimal::checked_from_parts(BigInteger::one(), INTERNAL_SCALE_LIMIT).is_some());
        assert!(BigDecimal::checked_from_parts(BigInteger::one(), -INTERNAL_SCALE_LIMIT).is_some());
        assert!(BigDecimal::checked_from_parts(BigInteger::one(), INTERNAL_SCALE_LIMIT + 1).is_none());
        assert!(BigDecimal::checked_from_parts(BigInteger::one(), i64::MIN).is_none());
    }

    #[test]
    #[should_panic(expected = "internal ceiling")]
    fn from_parts_panics_past_the_internal_ceiling() {
        use super::INTERNAL_SCALE_LIMIT;
        let _ = BigDecimal::from_parts(BigInteger::one(), INTERNAL_SCALE_LIMIT + 1);
    }

    #[test]
    fn arithmetic_on_parse_budget_operands_never_panics() {
        // Regression: a `*` or `+` whose result scale exceeds the *parse* budget must NOT panic
        // — both operands parse cleanly (in budget), and the wider internal ceiling absorbs the
        // combined scale. (This is the case a naive construction-time cap got wrong.)
        let a = BigDecimal::from_str("1e-600000").unwrap(); // scale 600000, in budget
        let prod = &a * &a; // result scale 1_200_000 > MAX_SCALE, but < INTERNAL_SCALE_LIMIT
        assert_eq!(prod.scale(), 1_200_000);
        assert!(prod.is_positive());
        let big = BigDecimal::from_str("5e1000000").unwrap(); // scale -1_000_000 (at budget)
        let sum = &big + &big; // additive carry pushes canonical scale to -1_000_001
        assert_eq!(sum.scale(), -1_000_001);
        assert!(sum.is_positive());
    }

    #[test]
    fn wide_scale_gaps_align_without_overflow() {
        // A wide but in-budget scale gap: alignment/cmp/`+`/Display must all work and must not
        // overflow i64 (scale differences are computed in i128). We use 10^4 rather than the
        // full MAX_SCALE only so the O(n^2) base-10 rendering stays fast in the test — the code
        // path is identical, and in-budget scales are nowhere near the i64 limits.
        let gap = 10_000i64;
        let big = BigDecimal::from_parts(BigInteger::one(), -gap); // 10^gap
        let tiny = BigDecimal::from_parts(BigInteger::one(), gap); // 10^-gap
        assert!(tiny < big);
        assert!((&big + &tiny).is_positive());
        assert_eq!(big.to_string().len(), (gap as usize) + 1); // "1" + gap zeros
    }

    // ---- ordering, sign -------------------------------------------------

    #[test]
    fn ordering_and_sign() {
        assert!(d("0.1") < d("0.2"));
        assert!(d("100") > d("99.99"));
        assert!(d("-0.5") < d("0"));
        assert_eq!(d("1.20").cmp(&d("1.2")), Ordering::Equal);

        let mut xs = [d("1.5"), d("-0.5"), d("0"), d("100"), d("0.001")];
        xs.sort();
        let shown: Vec<String> = xs.iter().map(|x| x.to_string()).collect();
        assert_eq!(shown, vec!["-0.5", "0", "0.001", "1.5", "100"]);

        assert_eq!(d("-3.14").signum(), -1);
        assert_eq!(d("0").signum(), 0);
        assert_eq!(d("-3.14").abs().to_string(), "3.14");
        assert!(d("-1").is_negative());
        assert!(d("1").is_positive());
    }

    #[test]
    fn conversions_from_primitives() {
        assert_eq!(BigDecimal::from(250_i64).to_string(), "250");
        assert_eq!(BigDecimal::from(7_u64).to_string(), "7");
        assert_eq!(BigDecimal::from(-9_i128).to_string(), "-9");
        assert_eq!(BigDecimal::from(BigInteger::from_i64(42)).to_string(), "42");
    }

    // ---- differential vs an i128 decimal oracle -------------------------

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            self.0
        }
        fn mant(&mut self) -> i64 {
            (self.next() % 200_001) as i64 - 100_000 // [-100000, 100000]
        }
        fn scale(&mut self) -> i64 {
            (self.next() % 7) as i64 // [0, 6]
        }
    }

    fn ten_pow_i128(n: i64) -> i128 {
        let mut r = 1i128;
        for _ in 0..n {
            r *= 10;
        }
        r
    }

    /// Strip trailing zeros from an i128 (mantissa, scale) into the same canonical form
    /// BigDecimal uses, then build the canonical BigDecimal for comparison.
    fn oracle(mut m: i128, mut s: i64) -> BigDecimal {
        if m == 0 {
            return BigDecimal::zero();
        }
        while m % 10 == 0 {
            m /= 10;
            s -= 1;
        }
        BigDecimal::from_parts(BigInteger::from_i128(m), s)
    }

    #[test]
    fn differential_against_i128_decimal_oracle() {
        let mut rng = Lcg(0xD1CE_5EED_1234_5678);
        for _ in 0..40_000 {
            let (m1, s1) = (rng.mant() as i128, rng.scale());
            let (m2, s2) = (rng.mant() as i128, rng.scale());
            let a = BigDecimal::from_parts(BigInteger::from_i128(m1), s1);
            let b = BigDecimal::from_parts(BigInteger::from_i128(m2), s2);

            // Align to the finer scale (max), staying well inside i128.
            let s = s1.max(s2);
            let a_al = m1 * ten_pow_i128(s - s1);
            let b_al = m2 * ten_pow_i128(s - s2);

            assert_eq!(&a + &b, oracle(a_al + b_al, s), "add {m1}e-{s1} + {m2}e-{s2}");
            assert_eq!(&a - &b, oracle(a_al - b_al, s), "sub {m1}e-{s1} - {m2}e-{s2}");
            assert_eq!(&a * &b, oracle(m1 * m2, s1 + s2), "mul {m1}e-{s1} * {m2}e-{s2}");
            assert_eq!(a.cmp(&b), a_al.cmp(&b_al), "cmp {m1}e-{s1} vs {m2}e-{s2}");
        }
    }

    // ---- differential: rounding division vs exact i128 rounding ---------

    #[test]
    fn differential_div_round_vs_i128() {
        use RoundingMode::*;
        let mut rng = Lcg(0x0BAD_F00D_CAFE_BABE);
        let modes = [Down, Up, Floor, Ceiling, HalfUp, HalfDown, HalfEven];
        for _ in 0..20_000 {
            let n = rng.mant() as i128;
            let mut dd = rng.mant() as i128;
            if dd == 0 {
                dd = 1;
            }
            let target = (rng.next() % 5) as i64; // 0..4 places
            let mode = modes[(rng.next() % modes.len() as u64) as usize];

            // value = n/dd, both scale 0. R = round(n * 10^target / dd).
            let num = n * ten_pow_i128(target);
            let expect_r = round_div_i128(num, dd, mode);
            let got = BigDecimal::from_i64(n as i64)
                .div_round(&BigDecimal::from_i64(dd as i64), target, mode);
            let want = oracle(expect_r, target);
            assert_eq!(got, want, "{n}/{dd} @s{target} {mode:?}");
        }
    }

    // ---- from_rational_exact: terminating vs repeating expansions ---------

    #[test]
    fn from_rational_exact_renders_terminating_decimals() {
        // Powers-of-2-and-5 denominators all terminate; the string is fully exact.
        let cases = [
            (1i64, 4i64, "0.25"),
            (3, 8, "0.375"),
            (7, 20, "0.35"),
            (1, 2, "0.5"),
            (1, 1, "1"),      // an integer is the scale-0 case
            (0, 1, "0"),      // zero
            (5463, 20, "273.15"), // the temperature-conversion offset
            (1013248623, 10000, "101324.8623"),
            (-3, 8, "-0.375"), // sign rides on the mantissa
        ];
        for (n, d, want) in cases {
            let r = BigRational::from_ints(n, d);
            let got = BigDecimal::from_rational_exact(&r)
                .unwrap_or_else(|| panic!("{n}/{d} should terminate"));
            assert_eq!(got.to_string(), want, "{n}/{d}");
        }
    }

    #[test]
    fn from_rational_exact_rejects_repeating_decimals() {
        // Any leftover prime factor (3, 7, 11, …) means the expansion repeats forever,
        // so no finite BigDecimal can hold it — the caller must fall back to a lossy f64.
        for (n, d) in [(1i64, 3i64), (2, 7), (5, 6), (1, 11), (22, 7)] {
            let r = BigRational::from_ints(n, d);
            assert!(
                BigDecimal::from_rational_exact(&r).is_none(),
                "{n}/{d} repeats and must return None"
            );
        }
    }

    #[test]
    fn from_rational_exact_round_trips_high_precision_pi_times_two() {
        // The exact-numbers headline: pi to 39 digits, doubled, stays exact end to end.
        // pi*2 = 6283185307179586476925286766559005768394 / 10^39 — a terminating decimal.
        let pi2 = BigRational::new(
            BigInteger::parse_radix("6283185307179586476925286766559005768394", 10).unwrap(),
            ten_pow(39),
        );
        let got = BigDecimal::from_rational_exact(&pi2).expect("pi*2 terminates");
        assert_eq!(
            got.to_string(),
            "6.283185307179586476925286766559005768394",
            "all 39 fractional digits survive, not the ~16 an f64 carries"
        );
    }

    #[test]
    fn from_rational_exact_inverts_to_rational() {
        // Round-trip: a terminating BigDecimal → rational → back is byte-identical.
        for s in ["0.25", "273.15", "101324.8623", "-0.375", "1000", "0"] {
            let d = BigDecimal::from_str(s).unwrap();
            let back = BigDecimal::from_rational_exact(&d.to_rational())
                .expect("a decimal's rational always terminates");
            assert_eq!(back, d, "round-trip {s}");
        }
    }

    /// Reference rounding of n/d to an integer, mirroring `round_div` but on i128.
    fn round_div_i128(n: i128, d: i128, mode: RoundingMode) -> i128 {
        use RoundingMode::*;
        let sign = n.signum() * d.signum();
        if sign == 0 {
            return 0;
        }
        let (na, da) = (n.unsigned_abs(), d.unsigned_abs());
        let q = na / da;
        let r = na % da;
        if r == 0 {
            return sign * q as i128;
        }
        let two_r = r * 2;
        let round_away = match mode {
            Down => false,
            Up => true,
            Floor => sign < 0,
            Ceiling => sign > 0,
            HalfUp => two_r >= da,
            HalfDown => two_r > da,
            HalfEven => two_r > da || (two_r == da && q % 2 == 1),
        };
        let mag = if round_away { q + 1 } else { q };
        sign * mag as i128
    }
}
