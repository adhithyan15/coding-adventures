//! # `BigRational` — an exact fraction, built on [`BigInteger`]
//!
//! A `BigRational` is a number written as a fraction — a **numerator** over a
//! **denominator** — where *both* parts are arbitrary-precision [`BigInteger`]s. It is
//! the second rung (NUM-2) of the ADJ numeric substrate, and it exists to make the
//! four everyday arithmetic operations — `+ − × ÷` — **exact, always, forever**.
//!
//! ## Why a fraction, and not a float?
//!
//! A binary floating-point number (`f64`) cannot represent `1/3`. The closest it can do
//! is `0.333333333333333314829616256247390992939472198486328125`, which is *wrong* — and
//! every operation you do afterwards compounds that error. `0.1 + 0.2` famously comes out
//! as `0.30000000000000004`, not `0.3`.
//!
//! A `BigRational` never rounds. `1/3` is stored as the pair `(1, 3)`; `0.1 + 0.2` is the
//! exact fraction `3/10`. Because both parts grow without bound, no sum, difference,
//! product, or quotient of two fractions can ever overflow or lose a digit. This is the
//! *"a single percentage point can be worth hundreds of millions of dollars"* discipline
//! made concrete: precision is exact by default, and only ever made lossy **on purpose**,
//! at a boundary you can see (that lossy `f64` export is a later rung's job — NUM-5 —
//! deliberately *not* built here, so that nothing in this crate can silently round).
//!
//! ## The one canonical form
//!
//! The same number can be written many ways: `1/2`, `2/4`, `3/6`, `-1/-2`, `50/100`. If we
//! stored fractions however they arrived, `2/4 == 1/2` would be *false* and a fraction
//! could not be a hash-map key. So every `BigRational` is kept in exactly **one** canonical
//! form, re-established after every single operation:
//!
//! 1. **Lowest terms.** Numerator and denominator share no common factor — we divide both
//!    by their [greatest common divisor](BigInteger::gcd). `50/100` becomes `1/2`.
//! 2. **Sign lives in the numerator.** The denominator is always **strictly positive**;
//!    a value like `3/-4` is stored as `-3/4`. So the sign of the fraction is just the
//!    sign of its numerator — no ambiguity.
//! 3. **Zero is `0/1`.** There is one and only one representation of zero.
//! 4. **The denominator is never zero.** A zero denominator is not a number; constructing
//!    one panics (or, via [`BigRational::checked_new`], returns `None`).
//!
//! Because the form is canonical and unique, `PartialEq`/`Eq`/`Hash` can be *derived*:
//! two `BigRational`s are equal exactly when their stored `(num, den)` pairs are identical.
//!
//! ```
//! use bignum_core::BigRational;
//!
//! // 1/3 + 1/6 = 1/2 — exact, no rounding.
//! let a = BigRational::from_ints(1, 3);
//! let b = BigRational::from_ints(1, 6);
//! assert_eq!((&a + &b).to_string(), "1/2");
//!
//! // Reducible and non-canonical inputs are normalized on the way in.
//! assert_eq!(BigRational::from_ints(50, 100).to_string(), "1/2");
//! assert_eq!(BigRational::from_ints(3, -4).to_string(), "-3/4");
//!
//! // 0.1 + 0.2 is exactly 3/10 (compare that to an f64!).
//! let tenth = BigRational::from_ints(1, 10);
//! let fifth = BigRational::from_ints(2, 10);
//! assert_eq!((&tenth + &fifth).to_string(), "3/10");
//! ```

use crate::{BigInteger, PowTooLargeError};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

// ===========================================================================
//  The type
// ===========================================================================

/// An arbitrary-precision **exact** rational number: a [`BigInteger`] numerator over a
/// [`BigInteger`] denominator, always kept in the one canonical form described in the
/// [module documentation](crate::rational).
///
/// The fields are private so the canonical-form invariant can never be violated from
/// outside this module. Read them with [`numerator`](Self::numerator) and
/// [`denominator`](Self::denominator).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BigRational {
    /// The numerator. Carries the sign of the whole fraction.
    num: BigInteger,
    /// The denominator. **Always strictly positive**, and coprime with `num`.
    den: BigInteger,
}

// ===========================================================================
//  Errors
// ===========================================================================

/// The error returned when a string cannot be parsed as a [`BigRational`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ParseRatioError {
    /// The input was empty, or a side of the `/` was empty (`""`, `"/3"`, `"5/"`).
    Empty,
    /// The numerator or denominator was not a valid base-10 integer.
    InvalidInteger,
    /// The input had more than one `/` (e.g. `"1/2/3"`).
    TooManySlashes,
    /// The denominator parsed to zero (e.g. `"5/0"`).
    ZeroDenominator,
}

impl fmt::Display for ParseRatioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            ParseRatioError::Empty => "empty numerator or denominator",
            ParseRatioError::InvalidInteger => "numerator or denominator is not an integer",
            ParseRatioError::TooManySlashes => "more than one '/' in rational literal",
            ParseRatioError::ZeroDenominator => "denominator is zero",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for ParseRatioError {}

// ===========================================================================
//  Construction & normalization
// ===========================================================================

impl BigRational {
    /// The fraction `0/1`.
    pub fn zero() -> Self {
        BigRational {
            num: BigInteger::zero(),
            den: BigInteger::one(),
        }
    }

    /// The fraction `1/1`.
    pub fn one() -> Self {
        BigRational {
            num: BigInteger::one(),
            den: BigInteger::one(),
        }
    }

    /// Build a fraction from a numerator and denominator, reducing it to canonical form.
    ///
    /// # Panics
    /// Panics if `den` is zero — a fraction with a zero denominator is not a number. Use
    /// [`checked_new`](Self::checked_new) for the non-panicking form.
    pub fn new(num: BigInteger, den: BigInteger) -> Self {
        Self::checked_new(num, den).expect("BigRational denominator must be non-zero")
    }

    /// Build a fraction from a numerator and denominator, or `None` if `den` is zero.
    ///
    /// This performs the full canonicalization: it moves any sign onto the numerator so the
    /// denominator is positive, divides both parts by their gcd so the fraction is in lowest
    /// terms, and collapses every representation of zero to `0/1`.
    pub fn checked_new(num: BigInteger, den: BigInteger) -> Option<Self> {
        if den.is_zero() {
            return None;
        }
        // Step 1 — force the denominator positive, carrying the sign to the numerator.
        // (`-a / -b` and `a / -b` both need fixing; `a / b` with `b > 0` is untouched.)
        let (num, den) = if den.is_negative() {
            (-num, -den)
        } else {
            (num, den)
        };
        // Step 2 — reduce to lowest terms. `gcd` is always non-negative, and `gcd(0, d) == d`,
        // so a zero numerator divides through to the canonical `0/1` automatically. `den > 0`
        // here, so dividing by the (positive) gcd leaves it positive.
        let g = num.gcd(&den);
        // gcd is at least 1 for a non-zero denominator, so this division is always exact.
        let num = &num / &g;
        let den = &den / &g;
        Some(BigRational { num, den })
    }

    /// Build a fraction directly from two primitive integers, e.g. `from_ints(22, 7)`.
    ///
    /// # Panics
    /// Panics if `den == 0`.
    pub fn from_ints(num: i64, den: i64) -> Self {
        Self::new(BigInteger::from_i64(num), BigInteger::from_i64(den))
    }

    /// Promote a whole [`BigInteger`] to the fraction `n/1`.
    pub fn from_integer(n: BigInteger) -> Self {
        BigRational {
            num: n,
            den: BigInteger::one(),
        }
    }

    /// The numerator (carries the fraction's sign).
    pub fn numerator(&self) -> &BigInteger {
        &self.num
    }

    /// The denominator (always strictly positive).
    pub fn denominator(&self) -> &BigInteger {
        &self.den
    }
}

// ===========================================================================
//  Predicates
// ===========================================================================

impl BigRational {
    /// Is this exactly zero (`0/1`)?
    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    /// Is this a whole number — i.e. is the denominator `1`?
    pub fn is_integer(&self) -> bool {
        self.den == BigInteger::one()
    }

    /// Is this strictly less than zero?
    pub fn is_negative(&self) -> bool {
        self.num.is_negative()
    }

    /// Is this strictly greater than zero?
    pub fn is_positive(&self) -> bool {
        self.num.is_positive()
    }

    /// `-1`, `0`, or `+1` according to the sign (the sign of the numerator, since the
    /// denominator is always positive).
    pub fn signum(&self) -> i32 {
        self.num.signum()
    }
}

// ===========================================================================
//  Sign & reciprocal
// ===========================================================================

impl BigRational {
    /// The absolute value `|self|`.
    pub fn abs(&self) -> BigRational {
        BigRational {
            num: self.num.abs(),
            den: self.den.clone(),
        }
    }

    /// The reciprocal `1/self` (`a/b` ↦ `b/a`).
    ///
    /// # Panics
    /// Panics if `self` is zero. Use [`checked_recip`](Self::checked_recip) for the
    /// non-panicking form.
    pub fn recip(&self) -> BigRational {
        self.checked_recip()
            .expect("cannot take the reciprocal of zero")
    }

    /// The reciprocal `1/self`, or `None` if `self` is zero.
    ///
    /// Swapping numerator and denominator can put the sign on the denominator (the
    /// reciprocal of `-3/4` is `4/-3`), so this routes through [`new`](Self::new) to restore
    /// the canonical form (`-4/3`). The two parts are already coprime, so no reduction
    /// happens — only the sign is normalized.
    pub fn checked_recip(&self) -> Option<BigRational> {
        if self.is_zero() {
            return None;
        }
        Some(BigRational::new(self.den.clone(), self.num.clone()))
    }
}

// ===========================================================================
//  Arithmetic
// ===========================================================================

impl BigRational {
    /// Exact sum. `a/b + c/d = (a·d + c·b) / (b·d)`, then reduced.
    ///
    /// We form the sum over the common denominator `b·d` (not the least common multiple —
    /// canonicalization reduces the result either way, and skipping the lcm keeps the code
    /// obviously correct). The final [`new`](Self::checked_new) restores lowest terms.
    pub fn add(&self, other: &BigRational) -> BigRational {
        let num = &(&self.num * &other.den) + &(&other.num * &self.den);
        let den = &self.den * &other.den;
        BigRational::new(num, den)
    }

    /// Exact difference. `a/b - c/d = (a·d - c·b) / (b·d)`, then reduced.
    pub fn sub(&self, other: &BigRational) -> BigRational {
        let num = &(&self.num * &other.den) - &(&other.num * &self.den);
        let den = &self.den * &other.den;
        BigRational::new(num, den)
    }

    /// Exact product. `a/b · c/d = (a·c) / (b·d)`, then reduced.
    pub fn mul(&self, other: &BigRational) -> BigRational {
        let num = &self.num * &other.num;
        let den = &self.den * &other.den;
        BigRational::new(num, den)
    }

    /// Exact quotient. `a/b ÷ c/d = (a·d) / (b·c)`, then reduced.
    ///
    /// # Panics
    /// Panics if `other` is zero (division by zero). Use [`checked_div`](Self::checked_div)
    /// for the non-panicking form.
    pub fn div(&self, other: &BigRational) -> BigRational {
        self.checked_div(other).expect("division by zero")
    }

    /// Exact quotient, or `None` if `other` is zero.
    pub fn checked_div(&self, other: &BigRational) -> Option<BigRational> {
        if other.is_zero() {
            return None;
        }
        let num = &self.num * &other.den;
        let den = &self.den * &other.num;
        // `den` here can be negative (if `other` was negative); `new` fixes the sign.
        BigRational::checked_new(num, den)
    }

    /// Raise to an **integer** power. A negative exponent takes the reciprocal:
    /// `(a/b)^-n = (b/a)^n`. `x^0` is `1` for every `x` (including zero).
    ///
    /// Because the base is already in lowest terms, so is every power of it, so raising
    /// numerator and denominator separately needs no extra reduction (only a sign fix-up in
    /// the negative-exponent case, handled by [`new`](Self::new)).
    ///
    /// # Panics
    /// Panics on a negative exponent of zero (that is `1/0`). Beware: like
    /// [`BigInteger::pow`], this is **unbounded** — a large exponent can exhaust memory.
    /// For an untrusted exponent use [`try_pow`](Self::try_pow).
    pub fn pow(&self, exp: i32) -> BigRational {
        if exp == 0 {
            return BigRational::one();
        }
        let n = exp.unsigned_abs();
        let num_pow = self.num.pow(n);
        let den_pow = self.den.pow(n);
        if exp > 0 {
            // num^n / den^n: coprime because num,den are; den^n > 0. Already canonical.
            BigRational {
                num: num_pow,
                den: den_pow,
            }
        } else {
            // Reciprocal: den^n / num^n. `num^n` may be negative or (if self==0) zero —
            // `new` restores the sign and panics on the `1/0` case, as documented.
            BigRational::new(den_pow, num_pow)
        }
    }

    /// The DoS-safe form of [`pow`](Self::pow): raise to an integer power, but refuse up
    /// front (before allocating) if either the numerator or denominator of the result would
    /// exceed `max_bits` bits. Because a power's bit length is at most `bit_len(base)·|exp|`,
    /// [`BigInteger::try_pow`] can reject an oversized result in `O(1)`, so an untrusted
    /// exponent cannot trigger an out-of-memory abort.
    ///
    /// Returns [`PowTooLargeError`] if the projected size is too large.
    ///
    /// # Panics
    /// Panics on a negative exponent of zero (`1/0`), the same as [`pow`](Self::pow); that
    /// is a domain error, not a size error.
    pub fn try_pow(&self, exp: i32, max_bits: u64) -> Result<BigRational, PowTooLargeError> {
        if exp == 0 {
            return Ok(BigRational::one());
        }
        let n = exp.unsigned_abs();
        let num_pow = self.num.try_pow(n, max_bits)?;
        let den_pow = self.den.try_pow(n, max_bits)?;
        Ok(if exp > 0 {
            BigRational {
                num: num_pow,
                den: den_pow,
            }
        } else {
            BigRational::new(den_pow, num_pow)
        })
    }
}

// ===========================================================================
//  Ordering
// ===========================================================================

// Two fractions `a/b` and `c/d` (both with **positive** denominators, guaranteed by our
// canonical form) compare the same way their cross-products do: `a/b < c/d` exactly when
// `a·d < c·b`. Positive denominators are what make this valid — multiplying an inequality by
// a positive number preserves its direction, so no sign-flip bookkeeping is needed.
impl Ord for BigRational {
    fn cmp(&self, other: &Self) -> Ordering {
        let left = &self.num * &other.den;
        let right = &other.num * &self.den;
        left.cmp(&right)
    }
}

impl PartialOrd for BigRational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ===========================================================================
//  Formatting & parsing
// ===========================================================================

impl fmt::Display for BigRational {
    /// Renders as `numerator/denominator`, or just `numerator` when the value is a whole
    /// number (denominator `1`). Examples: `-3/4`, `1/2`, `5`, `0`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_integer() {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

impl fmt::Debug for BigRational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BigRational({}/{})", self.num, self.den)
    }
}

impl FromStr for BigRational {
    type Err = ParseRatioError;

    /// Parses `"num/den"` or a bare integer `"num"` (base 10). A bare integer `n` becomes
    /// `n/1`. Whitespace is not trimmed — `"1 / 2"` is rejected.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('/');
        let num_str = parts.next().ok_or(ParseRatioError::Empty)?;
        let den_str = parts.next();
        if parts.next().is_some() {
            return Err(ParseRatioError::TooManySlashes);
        }
        if num_str.is_empty() {
            return Err(ParseRatioError::Empty);
        }
        let num = BigInteger::from_str(num_str).map_err(|_| ParseRatioError::InvalidInteger)?;
        match den_str {
            None => Ok(BigRational::from_integer(num)),
            Some(den_str) => {
                if den_str.is_empty() {
                    return Err(ParseRatioError::Empty);
                }
                let den =
                    BigInteger::from_str(den_str).map_err(|_| ParseRatioError::InvalidInteger)?;
                BigRational::checked_new(num, den).ok_or(ParseRatioError::ZeroDenominator)
            }
        }
    }
}

// ===========================================================================
//  Conversions from primitives & operator overloads
// ===========================================================================

impl From<BigInteger> for BigRational {
    fn from(n: BigInteger) -> Self {
        BigRational::from_integer(n)
    }
}

macro_rules! impl_from_primitive {
    ($t:ty, $ctor:ident) => {
        impl From<$t> for BigRational {
            fn from(v: $t) -> Self {
                BigRational::from_integer(BigInteger::$ctor(v))
            }
        }
    };
}
impl_from_primitive!(i64, from_i64);
impl_from_primitive!(u64, from_u64);
impl_from_primitive!(i128, from_i128);
impl_from_primitive!(u128, from_u128);

// Owned and borrowed forms of the four operators, all delegating to the inherent methods so
// the canonicalization lives in exactly one place.
// The `$inherent` method is called through the fully-qualified associated-function form
// `BigRational::$inherent(a, b)` rather than `a.$inherent(b)`. This matters for `Div`: the
// inherent method and the `std::ops::Div` trait method share the name `div`, and a bare
// `self.div(..)` inside the trait impl could resolve to the trait method (infinite
// recursion). The `Type::method` path unambiguously selects the inherent method.
macro_rules! impl_binop {
    ($trait:ident, $method:ident, $inherent:ident) => {
        impl std::ops::$trait for BigRational {
            type Output = BigRational;
            fn $method(self, rhs: BigRational) -> BigRational {
                BigRational::$inherent(&self, &rhs)
            }
        }
        impl std::ops::$trait<&BigRational> for &BigRational {
            type Output = BigRational;
            fn $method(self, rhs: &BigRational) -> BigRational {
                BigRational::$inherent(self, rhs)
            }
        }
    };
}
impl_binop!(Add, add, add);
impl_binop!(Sub, sub, sub);
impl_binop!(Mul, mul, mul);
impl_binop!(Div, div, div);

impl std::ops::Neg for BigRational {
    type Output = BigRational;
    fn neg(self) -> BigRational {
        BigRational {
            num: -self.num,
            den: self.den,
        }
    }
}

impl std::ops::Neg for &BigRational {
    type Output = BigRational;
    fn neg(self) -> BigRational {
        BigRational {
            num: -&self.num,
            den: self.den.clone(),
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

    fn r(n: i64, d: i64) -> BigRational {
        BigRational::from_ints(n, d)
    }

    // ---- canonical form -------------------------------------------------

    #[test]
    fn reduces_to_lowest_terms() {
        assert_eq!(r(50, 100).to_string(), "1/2");
        assert_eq!(r(462, 1071).to_string(), "22/51"); // Python oracle: gcd 21
        assert_eq!(r(6, 3).to_string(), "2"); // integer collapses to "n"
    }

    #[test]
    fn sign_lives_in_numerator() {
        assert_eq!(r(3, -4).to_string(), "-3/4");
        assert_eq!(r(-3, -4).to_string(), "3/4");
        assert_eq!(r(-3, 4).to_string(), "-3/4");
    }

    #[test]
    fn zero_is_canonical_regardless_of_denominator() {
        assert_eq!(r(0, 5), r(0, 999));
        assert_eq!(r(0, 5).to_string(), "0");
        assert!(r(0, 5).is_zero());
        assert_eq!(r(0, -7).denominator().to_string(), "1");
    }

    #[test]
    fn equality_and_hash_follow_value_not_spelling() {
        assert_eq!(r(2, 4), r(1, 2));
        assert_eq!(r(10, 20), r(1, 2));
        let mut m: HashMap<BigRational, &str> = HashMap::new();
        m.insert(r(2, 4), "half");
        // A different spelling of the same value must hit the same bucket.
        assert_eq!(m.get(&r(1, 2)), Some(&"half"));
        assert_eq!(m.get(&r(1, 3)), None);
    }

    #[test]
    #[should_panic(expected = "non-zero")]
    fn zero_denominator_panics() {
        let _ = BigRational::new(BigInteger::from_i64(1), BigInteger::zero());
    }

    #[test]
    fn checked_new_rejects_zero_denominator() {
        assert!(BigRational::checked_new(BigInteger::one(), BigInteger::zero()).is_none());
        assert!(BigRational::checked_new(BigInteger::one(), BigInteger::from_i64(2)).is_some());
    }

    // ---- small exact arithmetic (hand + Python oracle) ------------------

    #[test]
    fn exact_small_arithmetic() {
        assert_eq!((&r(1, 3) + &r(1, 6)).to_string(), "1/2");
        assert_eq!((&r(2, 7) * &r(14, 3)).to_string(), "4/3");
        assert_eq!((&r(355, 113) - &r(22, 7)).to_string(), "-1/791");
        assert_eq!((&r(1, 2) / &r(3, 4)).to_string(), "2/3");
        // The float-famous case: 0.1 + 0.2 is exactly 3/10.
        assert_eq!((&r(1, 10) + &r(2, 10)).to_string(), "3/10");
    }

    #[test]
    fn owned_and_borrowed_operators_agree() {
        assert_eq!(r(1, 3) + r(1, 6), &r(1, 3) + &r(1, 6));
        assert_eq!(r(2, 7) * r(14, 3), &r(2, 7) * &r(14, 3));
        assert_eq!(r(1, 2) / r(3, 4), &r(1, 2) / &r(3, 4));
        assert_eq!(-r(3, 4), -&r(3, 4));
    }

    // ---- big exact cases, pinned against Python's fractions.Fraction ----

    #[test]
    fn matches_python_fraction_oracle_on_big_operands() {
        let a = BigRational::from_str("1000000000000000000000000000001/100000000000000000000").unwrap();
        let b = BigRational::from_str("6366805760909027985741435139224001/847288609443").unwrap();

        assert_eq!(
            (&a + &b).to_string(),
            "636680576091750087183586513922400100000000847288609443/84728860944300000000000000000000"
        );
        assert_eq!(
            (&a - &b).to_string(),
            "-636680576090055509964700513922400099999999152711390557/84728860944300000000000000000000"
        );
        assert_eq!(
            (&a * &b).to_string(),
            "6366805760909027985741435139230367805760909027985741435139224001/84728860944300000000000000000000"
        );
        assert_eq!(
            (&a / &b).to_string(),
            "847288609443000000000000000000847288609443/636680576090902798574143513922400100000000000000000000"
        );
        assert_eq!(
            a.pow(3).to_string(),
            "1000000000000000000000000000003000000000000000000000000000003000000000000000000000000000001/1000000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(
            b.pow(-2).to_string(),
            "717897987691852588770249/40536215597144386832065866109016673800875222251012083746192454448001"
        );
    }

    // ---- ordering -------------------------------------------------------

    #[test]
    fn ordering_by_cross_multiplication() {
        assert!(r(22, 7) > r(355, 113)); // 3.142857… > 3.14159…
        assert!(r(-1, 3) > r(-1, 2)); // -0.333… > -0.5
        assert_eq!(r(2, 4).cmp(&r(1, 2)), Ordering::Equal);

        let mut xs = [r(3, 2), r(-1, 2), r(0, 1), r(22, 7), r(1, 3)];
        xs.sort();
        let shown: Vec<String> = xs.iter().map(|x| x.to_string()).collect();
        assert_eq!(shown, vec!["-1/2", "0", "1/3", "3/2", "22/7"]);
    }

    // ---- sign, reciprocal, predicates -----------------------------------

    #[test]
    fn sign_reciprocal_and_predicates() {
        assert_eq!(r(-3, 4).abs().to_string(), "3/4");
        assert_eq!(r(-3, 4).signum(), -1);
        assert_eq!(r(0, 1).signum(), 0);
        assert_eq!(r(3, 4).signum(), 1);
        assert!(r(-3, 4).is_negative());
        assert!(r(3, 4).is_positive());
        assert!(r(6, 3).is_integer());
        assert!(!r(1, 2).is_integer());

        assert_eq!(r(-3, 4).recip().to_string(), "-4/3");
        assert_eq!(r(7, 1).recip().to_string(), "1/7");
        assert!(r(0, 1).checked_recip().is_none());
    }

    #[test]
    #[should_panic(expected = "reciprocal of zero")]
    fn reciprocal_of_zero_panics() {
        let _ = r(0, 1).recip();
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn division_by_zero_panics() {
        let _ = &r(1, 2) / &r(0, 1);
    }

    // ---- pow, including negative exponents & the DoS guard ---------------

    #[test]
    fn pow_positive_negative_zero() {
        assert_eq!(r(2, 3).pow(0).to_string(), "1");
        assert_eq!(r(2, 3).pow(3).to_string(), "8/27");
        assert_eq!(r(2, 3).pow(-3).to_string(), "27/8");
        assert_eq!(r(-2, 3).pow(2).to_string(), "4/9");
        assert_eq!(r(-2, 3).pow(3).to_string(), "-8/27");
        assert_eq!(r(0, 1).pow(5).to_string(), "0");
    }

    #[test]
    #[should_panic]
    fn negative_power_of_zero_panics() {
        let _ = r(0, 1).pow(-2);
    }

    #[test]
    fn try_pow_guards_oversized_results() {
        // 2^10 = 1024, comfortably under a 64-bit cap.
        assert_eq!(r(2, 1).try_pow(10, 64).unwrap().to_string(), "1024");
        // 10/3 raised to a huge power projects to millions of bits -> refused up front,
        // before any allocation, so an untrusted exponent cannot OOM us.
        assert!(r(10, 3).try_pow(1_000_000, 4096).is_err());
        // A negative exponent is guarded the same way (numerator/denominator swap).
        assert!(r(10, 3).try_pow(-1_000_000, 4096).is_err());
    }

    // ---- parsing & formatting round-trips --------------------------------

    #[test]
    fn parse_and_display_round_trip() {
        for s in ["22/7", "-3/4", "5", "0", "-8/27", "1/1000000000000000000000"] {
            let parsed = BigRational::from_str(s).unwrap();
            // Re-parsing the display must give an equal value.
            assert_eq!(BigRational::from_str(&parsed.to_string()).unwrap(), parsed);
        }
        // A bare integer becomes n/1 and displays without a slash.
        assert_eq!(BigRational::from_str("42").unwrap().to_string(), "42");
        // Non-canonical input is normalized on parse.
        assert_eq!(BigRational::from_str("50/100").unwrap().to_string(), "1/2");
        assert_eq!(BigRational::from_str("3/-4").unwrap().to_string(), "-3/4");
    }

    #[test]
    fn parse_errors_are_typed() {
        assert_eq!(BigRational::from_str(""), Err(ParseRatioError::Empty));
        assert_eq!(BigRational::from_str("/3"), Err(ParseRatioError::Empty));
        assert_eq!(BigRational::from_str("5/"), Err(ParseRatioError::Empty));
        assert_eq!(BigRational::from_str("1/2/3"), Err(ParseRatioError::TooManySlashes));
        assert_eq!(BigRational::from_str("5/0"), Err(ParseRatioError::ZeroDenominator));
        assert_eq!(BigRational::from_str("x/2"), Err(ParseRatioError::InvalidInteger));
        assert_eq!(BigRational::from_str("1/y"), Err(ParseRatioError::InvalidInteger));
    }

    #[test]
    fn conversions_from_primitives_and_bigint() {
        assert_eq!(BigRational::from(5_i64).to_string(), "5");
        assert_eq!(BigRational::from(5_u64).to_string(), "5");
        assert_eq!(BigRational::from(-5_i128).to_string(), "-5");
        assert_eq!(BigRational::from(BigInteger::from_i64(9)).to_string(), "9");
        assert_eq!(BigRational::one().to_string(), "1");
        assert_eq!(BigRational::zero().to_string(), "0");
    }

    // ---- differential: thousands of cases vs an i128 fraction oracle ----

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            // Numerical Recipes LCG constants — deterministic, no RNG crate.
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            self.0
        }
        // A small signed value in [-range, range].
        fn small(&mut self, range: i64) -> i64 {
            (self.next() % (2 * range as u64 + 1)) as i64 - range
        }
    }

    fn gcd_i128(mut a: i128, mut b: i128) -> i128 {
        a = a.abs();
        b = b.abs();
        while b != 0 {
            let t = a % b;
            a = b;
            b = t;
        }
        a
    }

    /// Reduce an i128 fraction to the same canonical form BigRational uses.
    fn norm_i128(mut n: i128, mut d: i128) -> (i128, i128) {
        assert!(d != 0);
        if d < 0 {
            n = -n;
            d = -d;
        }
        let g = gcd_i128(n, d);
        if g == 0 {
            (0, 1)
        } else {
            (n / g, d / g)
        }
    }

    #[test]
    fn differential_against_i128_oracle() {
        let mut rng = Lcg(0x9E3779B97F4A7C15);
        // Operands small enough that every i128 cross-product stays well inside i128.
        let range = 30_000i64;
        for _ in 0..40_000 {
            let an = rng.small(range);
            let mut ad = rng.small(range);
            let cn = rng.small(range);
            let mut cd = rng.small(range);
            if ad == 0 {
                ad = 1;
            }
            if cd == 0 {
                cd = 1;
            }
            let a = r(an, ad);
            let c = r(cn, cd);
            // Normalize the oracle operands to the same canonical (positive-denominator,
            // lowest-terms) form BigRational uses. This is what makes the cross-multiplication
            // comparison below valid — `a·d < c·b` only tracks `a/b < c/d` when b, d > 0.
            let (an, ad) = norm_i128(an as i128, ad as i128);
            let (cn, cd) = norm_i128(cn as i128, cd as i128);

            // add
            let (en, ed) = norm_i128(an * cd + cn * ad, ad * cd);
            assert_eq!(&a + &c, r(en as i64, ed as i64), "add {an}/{ad} + {cn}/{cd}");
            // sub
            let (en, ed) = norm_i128(an * cd - cn * ad, ad * cd);
            assert_eq!(&a - &c, r(en as i64, ed as i64), "sub {an}/{ad} - {cn}/{cd}");
            // mul
            let (en, ed) = norm_i128(an * cn, ad * cd);
            assert_eq!(&a * &c, r(en as i64, ed as i64), "mul {an}/{ad} * {cn}/{cd}");
            // div (skip when c == 0)
            if cn != 0 {
                let (en, ed) = norm_i128(an * cd, ad * cn);
                assert_eq!(&a / &c, r(en as i64, ed as i64), "div {an}/{ad} / {cn}/{cd}");
            }
            // ordering must agree with the cross-product sign
            let expected = (an * cd).cmp(&(cn * ad));
            assert_eq!(a.cmp(&c), expected, "cmp {an}/{ad} vs {cn}/{cd}");
        }
    }
}
