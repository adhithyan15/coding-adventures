//! Integer-reflection const-intrinsics (SIR21 §"Min / max / limits are derived").
//!
//! A program sometimes needs to *read* an integer type's limits — C's
//! `INT_MAX`, Rust's `i32::MAX`, the width in bits. SIR21's rule is that these
//! are **not** stored on the type; they are a pure function of the
//! `(width, signed)` spec (see [`crate::IntSpec`]). This module is the canonical
//! evaluator for those three reflection queries.
//!
//! They are **const-intrinsics**: pure, total, and target-independent. A backend
//! emits the *literal* result (`2147483647`), never a runtime call — the value
//! is the same on every target, so there is nothing to dispatch. They ride the
//! [SIR10 `Intrinsic`](crate) boundary under the stable names `int.max`,
//! `int.min`, `int.width`, and this module maps each name to its value.
//!
//! ```text
//! int.max(i32)   ⟶  2147483647      (2³¹ − 1)
//! int.min(u8)    ⟶  0
//! int.width(i32) ⟶  32
//! int.max(arbitrary) ⟶  «none»      (Ruby's Integer has no MAX — unbounded)
//! ```
//!
//! This is SIR21 milestone **T3a**. It is behaviour-preserving: no frontend
//! emits these intrinsics yet, so nothing changes until a frontend opts in; the
//! module establishes the one canonical meaning every backend will const-fold to.

use crate::types::IntSpec;

/// One of the three pure integer-reflection queries.
///
/// | variant | reads          | of an `i32`      | of `arbitrary` |
/// |---------|----------------|------------------|----------------|
/// | `Max`   | largest value  | `2147483647`     | none (unbounded) |
/// | `Min`   | smallest value | `-2147483648`    | none (unbounded) |
/// | `Width` | bits           | `32`             | none (grows)   |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntConst {
    Max,
    Min,
    Width,
}

impl IntConst {
    /// Every reflection query, in declaration order.
    pub const ALL: &'static [IntConst] = &[IntConst::Max, IntConst::Min, IntConst::Width];

    /// The canonical intrinsic name on the SIR boundary.
    pub fn name(self) -> &'static str {
        match self {
            IntConst::Max => "int.max",
            IntConst::Min => "int.min",
            IntConst::Width => "int.width",
        }
    }

    /// Parse a canonical name back to an [`IntConst`]; `None` for anything else.
    pub fn from_name(name: &str) -> Option<IntConst> {
        IntConst::ALL.iter().copied().find(|c| c.name() == name)
    }

    /// Evaluate this reflection query against `spec`, const-folding to its
    /// integer value — or `None` when the query is meaningless for the type.
    ///
    /// An `Arbitrary`-width integer (the Ruby/Python integer) has no `Max`,
    /// `Min`, or fixed `Width`: it grows without bound, so reflecting on its
    /// limits yields `None`. Every fixed width yields a definite value.
    pub fn eval(self, spec: IntSpec) -> Option<i128> {
        match self {
            IntConst::Max => spec.max(),
            IntConst::Min => spec.min(),
            IntConst::Width => spec.width.bits().map(|bits| bits as i128),
        }
    }
}

/// Convenience: evaluate the const-intrinsic named `name` against `spec`.
///
/// Returns `None` if `name` is not a known const-intrinsic **or** if the query
/// is meaningless for the type (an `Arbitrary` width). Callers that need to
/// distinguish "unknown name" from "no value" should use
/// [`IntConst::from_name`] then [`IntConst::eval`].
pub fn eval_named(name: &str, spec: IntSpec) -> Option<i128> {
    IntConst::from_name(name).and_then(|c| c.eval(spec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{IntWidth, Overflow};

    fn i(width: IntWidth) -> IntSpec {
        IntSpec::sized(width, true, Overflow::Wrap)
    }
    fn u(width: IntWidth) -> IntSpec {
        IntSpec::sized(width, false, Overflow::Wrap)
    }

    // ── The canonical spec examples ───────────────────────────────────

    #[test]
    fn i32_max_min_width() {
        let i32 = i(IntWidth::W32);
        assert_eq!(IntConst::Max.eval(i32), Some(2_147_483_647));
        assert_eq!(IntConst::Min.eval(i32), Some(-2_147_483_648));
        assert_eq!(IntConst::Width.eval(i32), Some(32));
    }

    #[test]
    fn u8_bounds() {
        let u8 = u(IntWidth::W8);
        assert_eq!(IntConst::Max.eval(u8), Some(255));
        assert_eq!(IntConst::Min.eval(u8), Some(0));
        assert_eq!(IntConst::Width.eval(u8), Some(8));
    }

    #[test]
    fn i64_bounds_match_native() {
        let i64 = i(IntWidth::W64);
        assert_eq!(IntConst::Max.eval(i64), Some(i64::MAX as i128));
        assert_eq!(IntConst::Min.eval(i64), Some(i64::MIN as i128));
        assert_eq!(IntConst::Width.eval(i64), Some(64));
    }

    #[test]
    fn arbitrary_has_no_limits() {
        // Ruby's Integer has no MAX/MIN and no fixed width.
        let arb = IntSpec::arbitrary();
        assert_eq!(IntConst::Max.eval(arb), None);
        assert_eq!(IntConst::Min.eval(arb), None);
        assert_eq!(IntConst::Width.eval(arb), None);
    }

    // ── Name round-trip ───────────────────────────────────────────────

    #[test]
    fn names_round_trip() {
        for &c in IntConst::ALL {
            assert_eq!(IntConst::from_name(c.name()), Some(c));
        }
        assert_eq!(IntConst::Max.name(), "int.max");
        assert_eq!(IntConst::Min.name(), "int.min");
        assert_eq!(IntConst::Width.name(), "int.width");
    }

    #[test]
    fn unknown_name_is_none() {
        assert_eq!(IntConst::from_name("int.bogus"), None);
        assert_eq!(IntConst::from_name("max"), None);
    }

    #[test]
    fn all_names_unique() {
        let mut seen = std::collections::HashSet::new();
        for &c in IntConst::ALL {
            assert!(seen.insert(c.name()), "duplicate name {}", c.name());
        }
        assert_eq!(IntConst::ALL.len(), 3);
    }

    // ── The convenience wrapper ───────────────────────────────────────

    #[test]
    fn eval_named_dispatches_and_folds() {
        assert_eq!(eval_named("int.max", i(IntWidth::W32)), Some(2_147_483_647));
        assert_eq!(eval_named("int.width", u(IntWidth::W16)), Some(16));
        // Unknown name → None.
        assert_eq!(eval_named("int.nope", i(IntWidth::W32)), None);
        // Known name, but meaningless for arbitrary → None.
        assert_eq!(eval_named("int.max", IntSpec::arbitrary()), None);
    }

    #[test]
    fn w128_width_is_folded_but_bounds_are_the_i128_report() {
        // Width is always exact; the i128-saturated bounds come straight from
        // IntSpec (documented there) and must not panic.
        let i128w = i(IntWidth::W128);
        assert_eq!(IntConst::Width.eval(i128w), Some(128));
        assert_eq!(IntConst::Max.eval(i128w), Some(i128::MAX));
        assert_eq!(IntConst::Min.eval(i128w), Some(i128::MIN));
    }
}
