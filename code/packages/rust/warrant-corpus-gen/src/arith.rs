//! # The single wrapping rule and its bounds (VER04r section 3)
//!
//! > Every `+`, `-`, `*` and unary `-` in the model IR on a fixed-width type wraps modulo
//! > `2^w`, in two's complement for signed types.  This holds in the generator, in the
//! > oracle and in every lane, in every profile level.
//!
//! This module is the *arithmetic* of that rule, as small total functions the S3b evaluator
//! will call node by node.  They are here, with their tests, in S3a because the rule is
//! easier to read on its own than inside an evaluator:
//!
//! ```text
//!   type    operation       exact value   result        wrapped?
//!   ------  --------------  ------------  ------------  --------
//!   U8      255 + 1         256           0             yes
//!   U8      0 - 1           -1            255           yes
//!   U8      16 * 16         256           0             yes
//!   U64     MAX * MAX       2^128-2^65+1  1             yes   (needs more than i128)
//!   I8      127 + 1         128           -128          yes
//!   I8      -(-128)         128           -128          yes
//!   I64     MIN - 1         -2^63-1       MAX           yes
//!   Nat/Int 3 - 5           -2            -2 (Int)      n/a: exact, never wraps
//! ```
//!
//! `wrapped` is the bit the oracle counts as `wraps` (VER04r 3.3 rule 1): the evaluation
//! whose wrapped result **differs from the exact result**.  The count itself is the
//! evaluator's (S3b); the function that says whether one evaluation wrapped is here.
//!
//! ## Exact intermediates
//!
//! `Nat` and `Int` are exact but the evaluator is not unbounded: a value outside
//! `-(2^127-1)..=2^127-1` is a generation error (`W0701`), so every value fits an `i128` and
//! a product is computed with checked arithmetic (two in-range values can need 254 bits).
//!
//! ## The v0 guard, rule 2
//!
//! For a v0 document, `wraps > 0` is a generation error: the frozen Rust harness evaluates
//! `U8` as `Nat` (WD-013) and would disagree with every lane on that case.

use coding_adventures_warrant_grammar::fixed;
use coding_adventures_warrant_grammar::fixed::FixedTy;
use coding_adventures_warrant_grammar::parser::Version;

use crate::error::{code, GenError};
use crate::ir::Arith;

/// `MAX_IR_VALUE` (VER04r 4.2 rule 6): no `Bytes` or list value an evaluator produces may
/// exceed this many bytes or elements, checked **before** the allocation.
pub const MAX_IR_VALUE: u64 = 65_536;

/// The largest magnitude of an exact `Nat`/`Int` intermediate: `2^127 - 1` (VER04r 3.2).
pub const EXACT_BOUND: i128 = i128::MAX;

/// Is `v` inside `-(2^127-1)..=2^127-1`?
///
/// # Errors
///
/// `W0701` otherwise (`i128::MIN`, which is `-2^127`, is outside the window).
pub fn check_exact(v: i128) -> Result<i128, GenError> {
    if v == i128::MIN {
        return Err(GenError::new(
            code::W0701,
            "",
            "an exact Nat/Int intermediate left the 2^127 window (VER04r 3.2)",
        ));
    }
    Ok(v)
}

/// Exact `a op b` on `Nat`/`Int` values.
///
/// # Errors
///
/// `W0701` if the result (or the machine product) leaves the 2^127 window.
pub fn exact_op(op: Arith, a: i128, b: i128) -> Result<i128, GenError> {
    let r = match op {
        Arith::Add => a.checked_add(b),
        Arith::Sub => a.checked_sub(b),
        Arith::Mul => a.checked_mul(b),
    };
    match r {
        Some(v) => check_exact(v),
        None => Err(GenError::new(
            code::W0701,
            "",
            "an exact Nat/Int intermediate left the 2^127 window (VER04r 3.2)",
        )),
    }
}

/// Exact `-a` on an `Int`.
///
/// # Errors
///
/// `W0701` if the result leaves the window (only for `a == i128::MIN`, which is not a legal
/// intermediate to begin with).
pub fn exact_neg(a: i128) -> Result<i128, GenError> {
    match a.checked_neg() {
        Some(v) => check_exact(v),
        None => Err(GenError::new(
            code::W0701,
            "",
            "an exact Nat/Int intermediate left the 2^127 window (VER04r 3.2)",
        )),
    }
}

/// Fixed-width `a op b`: the wrapped result and whether it **differs from the exact
/// result** (one `wraps` increment).  Operands must be in range for `ty`.
pub fn fixed_op(op: Arith, ty: FixedTy, a: i128, b: i128) -> (i128, bool) {
    let (wrapped, symbol) = match op {
        Arith::Add => (fixed::add(ty, a, b), '+'),
        Arith::Sub => (fixed::sub(ty, a, b), '-'),
        Arith::Mul => (fixed::mul(ty, a, b), '*'),
    };
    // `checked` is `None` exactly when the exact result leaves the type's range, which
    // includes an exact result too big for an i128 (a `U64 * U64` can be 128 bits).
    (wrapped, fixed::checked(symbol, ty, a, b).is_none())
}

/// Fixed-width unary minus: the wrapped result and whether it differs from the exact `-a`.
/// For a signed type only the minimum wraps (`-(-128i8) = -128i8`); the checker refuses
/// unary minus on an unsigned type, so for an unsigned `a != 0` this reports a wrap too
/// (`-1u8` is `255`, not `-1`).
pub fn fixed_neg(ty: FixedTy, a: i128) -> (i128, bool) {
    let wrapped = fixed::neg(ty, a);
    (
        wrapped,
        a.checked_neg().is_none_or(|exact| wrapped != exact),
    )
}

/// Does the length `len` of a `Bytes` or list value respect [`MAX_IR_VALUE`]?  Call before
/// allocating the value.
///
/// # Errors
///
/// `W0701` (the generator's form of `ir-overflow`, VER04r 4.2 rule 6) past the limit.
pub fn check_value_len(len: u64) -> Result<(), GenError> {
    if len > MAX_IR_VALUE {
        return Err(GenError::size_error(
            "a Bytes or list value exceeds MAX_IR_VALUE (VER04r 4.2 rule 6)",
        ));
    }
    Ok(())
}

/// The v0 divergence guard, rule 2 (VER04r 3.3): in a v0 document `wraps > 0` is an error.
/// A v1 document may wrap everywhere and `wraps` is informational (rule 5).
///
/// # Errors
///
/// `W0701: U8 arithmetic wraps; v0 harness evaluates U8 as Nat`.
pub fn check_v0_wraps(version: Version, wraps: u64) -> Result<(), GenError> {
    if version == Version::V0 && wraps > 0 {
        return Err(GenError::new(
            code::W0701,
            "",
            "U8 arithmetic wraps; v0 harness evaluates U8 as Nat (VER04r 3.3 rule 2)",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const U8: FixedTy = FixedTy::U8;
    const U64: FixedTy = FixedTy::U64;
    const I8: FixedTy = FixedTy::I8;
    const I64: FixedTy = FixedTy::I64;

    // Every expectation below is computed by hand from the table in the module docs.

    #[test]
    fn unsigned_wraps_modulo_two_to_the_width() {
        assert_eq!(fixed_op(Arith::Add, U8, 255, 1), (0, true));
        assert_eq!(fixed_op(Arith::Sub, U8, 0, 1), (255, true));
        assert_eq!(fixed_op(Arith::Mul, U8, 16, 16), (0, true));
        assert_eq!(fixed_op(Arith::Mul, U8, 15, 17), (255, false));
    }

    #[test]
    fn in_range_results_do_not_count_as_wraps() {
        assert_eq!(fixed_op(Arith::Add, U8, 100, 55), (155, false));
        assert_eq!(fixed_op(Arith::Sub, U8, 9, 9), (0, false));
        assert_eq!(fixed_op(Arith::Add, U8, 128, 127), (255, false));
        assert_eq!(fixed_op(Arith::Add, I8, -128, 127), (-1, false));
    }

    #[test]
    fn u64_product_needs_more_than_an_i128_and_still_wraps_correctly() {
        let max = u64::MAX as i128;
        // (2^64 - 1)^2 = 2^128 - 2^65 + 1 = 1 (mod 2^64); the exact value overflows i128.
        assert_eq!(fixed_op(Arith::Mul, U64, max, max), (1, true));
        assert_eq!(fixed_op(Arith::Add, U64, max, 1), (0, true));
        assert_eq!(fixed_op(Arith::Sub, U64, 0, 1), (max, true));
        assert_eq!(
            fixed_op(Arith::Mul, U64, 1 << 32, 1 << 31),
            (1i128 << 63, false)
        );
        assert_eq!(fixed_op(Arith::Mul, U64, 1 << 32, 1 << 32), (0, true));
    }

    #[test]
    fn signed_wraps_into_the_twos_complement_range() {
        assert_eq!(fixed_op(Arith::Add, I8, 127, 1), (-128, true));
        assert_eq!(fixed_op(Arith::Sub, I8, -128, 1), (127, true));
        assert_eq!(fixed_op(Arith::Mul, I8, -128, -1), (-128, true));
        assert_eq!(
            fixed_op(Arith::Sub, I64, i64::MIN as i128, 1),
            (i64::MAX as i128, true)
        );
        assert_eq!(fixed_op(Arith::Mul, I8, -2, 3), (-6, false));
    }

    #[test]
    fn unary_minus() {
        assert_eq!(fixed_neg(I8, -128), (-128, true));
        assert_eq!(fixed_neg(I8, 5), (-5, false));
        assert_eq!(fixed_neg(I8, 0), (0, false));
        assert_eq!(fixed_neg(I64, i64::MIN as i128), (i64::MIN as i128, true));
        // an unsigned negation (never produced by the checked lowering) still reports
        // the truth: -1u8 is 255
        assert_eq!(fixed_neg(U8, 1), (255, true));
        assert_eq!(fixed_neg(U8, 0), (0, false));
        // an out-of-range operand at i128::MIN must not overflow the comparison
        assert!(fixed_neg(I8, i128::MIN).1);
    }

    #[test]
    fn nat_and_int_are_exact_and_nat_minus_nat_goes_negative() {
        assert_eq!(exact_op(Arith::Sub, 3, 5).unwrap(), -2);
        assert_eq!(exact_op(Arith::Add, 3, 5).unwrap(), 8);
        assert_eq!(
            exact_op(Arith::Mul, 1 << 62, 1 << 62).unwrap(),
            1i128 << 124
        );
        assert_eq!(exact_neg(7).unwrap(), -7);
        assert_eq!(exact_neg(-7).unwrap(), 7);
    }

    #[test]
    fn the_2_to_127_window_is_enforced_with_checked_arithmetic() {
        let big = (1i128 << 126) + 1;
        // 2^127 + 2 > 2^127 - 1
        assert_eq!(
            exact_op(Arith::Add, big, big).unwrap_err().code,
            code::W0701
        );
        // the product of two in-range values can need 254 bits: no wrap, no panic
        assert_eq!(
            exact_op(Arith::Mul, i128::MAX, i128::MAX).unwrap_err().code,
            code::W0701
        );
        assert_eq!(
            exact_op(Arith::Mul, 1 << 64, 1 << 64).unwrap_err().code,
            code::W0701
        );
        // the largest legal value is 2^127 - 1; one past it is refused
        assert_eq!(exact_op(Arith::Add, i128::MAX - 1, 1).unwrap(), i128::MAX);
        assert_eq!(
            exact_op(Arith::Add, i128::MAX, 1).unwrap_err().code,
            code::W0701
        );
        // -(2^127) is outside the window even though an i128 holds it
        assert_eq!(
            exact_op(Arith::Sub, -i128::MAX, 1).unwrap_err().code,
            code::W0701
        );
        assert_eq!(check_exact(i128::MIN).unwrap_err().code, code::W0701);
        // the integer window is not a size error (VER04r 8.4: only the two Bytes limits are)
        for e in [
            check_exact(i128::MIN).unwrap_err(),
            exact_op(Arith::Mul, i128::MAX, 2).unwrap_err(),
            exact_neg(i128::MIN).unwrap_err(),
        ] {
            assert!(!e.size, "{e}");
        }
        assert_eq!(check_exact(-i128::MAX).unwrap(), -i128::MAX);
        assert_eq!(exact_neg(i128::MIN).unwrap_err().code, code::W0701);
    }

    #[test]
    fn max_ir_value_is_checked_before_allocation() {
        assert!(check_value_len(0).is_ok());
        assert!(check_value_len(MAX_IR_VALUE).is_ok());
        assert_eq!(
            check_value_len(MAX_IR_VALUE + 1).unwrap_err().code,
            code::W0701
        );
        assert_eq!(check_value_len(u64::MAX).unwrap_err().code, code::W0701);
        assert!(check_value_len(MAX_IR_VALUE + 1).unwrap_err().size);
    }

    #[test]
    fn the_v0_guard_refuses_wraps_only_in_a_v0_document() {
        assert!(check_v0_wraps(Version::V0, 0).is_ok());
        assert!(check_v0_wraps(Version::V1, 0).is_ok());
        assert!(check_v0_wraps(Version::V1, 99).is_ok());
        let e = check_v0_wraps(Version::V0, 1).unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert!(e.message.contains("v0 harness evaluates U8 as Nat"));
    }
}
