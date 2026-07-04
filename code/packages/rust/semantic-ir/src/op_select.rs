//! Type-directed operation selection (SIR21 §"Type-directed operation selection").
//!
//! This is "semantic neutrality made mechanical". A binary arithmetic node
//! (`+`, `-`, `*`) does not carry a pre-baked target opcode; it carries its
//! operands, and **each backend resolves the concrete operation from the
//! operands' static types**. When the types are known and agree, the backend
//! specialises (a native `i32` add that wraps, a bignum add that grows, a float
//! add that promotes). When an operand is `Dynamic` — as *every* operand is in
//! the current fully-dynamic pipeline — it falls back to runtime dispatch
//! (`_sir_plus`/`_sir_times`/…), which is **exactly today's behaviour**.
//!
//! ```text
//! (Int i32{wrap}, Int i32{wrap})  ⟶  Int(i32{wrap})   native 32-bit, wraps
//! (Int Arbitrary, Int Arbitrary)  ⟶  Int(Arbitrary)   bignum, grows
//! (Float, Int)                    ⟶  Float            promote to float
//! (Str, Str) / (Dynamic, …)       ⟶  RuntimeDispatch  the runtime decides
//! ```
//!
//! This module is a **pure decision function**: it reads the carried types and
//! returns a lowering choice. It performs no inference (SIR carries types, it
//! does not synthesise them — SIR10), mutates nothing, and changes no
//! behaviour: an untyped program resolves to `RuntimeDispatch` on every node, so
//! the emitters keep doing precisely what they do now. It is the shared rule the
//! per-backend sized-integer lowering (T4–T8) will consult so they all agree on
//! *when* to specialise.
//!
//! SIR21 milestone **T3c-1**. Scoped to the numeric resolution — the `+`/`-`/`*`
//! keystone. String concatenation on `+` and comparison operators keep their own
//! handling and are folded in by a later slice.

use crate::types::{IntSpec, SirType};

/// How a binary numeric op should be lowered, resolved from its operand types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericLowering {
    /// Both operands are the **same** concrete integer type: lower to that
    /// type's native operation with its declared overflow behaviour. An
    /// `Arbitrary`-width spec here is the bignum path (grows, never overflows).
    Int(IntSpec),
    /// At least one operand is a `Float` and the other is numeric: promote to
    /// float arithmetic.
    Float,
    /// An operand is `Dynamic`/absent, the operands are non-numeric, or two
    /// integers of *different* specs meet (SIR does not silently pick a
    /// promotion — that would be inference). Fall back to the runtime helper
    /// (`_sir_plus`/`_sir_minus`/`_sir_times`) — today's behaviour, unchanged.
    RuntimeDispatch,
}

/// The numeric classification of an operand type, or `None` if it is not a
/// number (including `Dynamic`, which is *unknown*, not *numeric*).
enum Numeric {
    Int(IntSpec),
    Float,
}

fn as_numeric(t: Option<&SirType>) -> Option<Numeric> {
    match t? {
        SirType::Int(spec) => Some(Numeric::Int(*spec)),
        SirType::Float => Some(Numeric::Float),
        _ => None,
    }
}

/// Resolve the lowering for a binary numeric op from its operand types.
///
/// `lhs`/`rhs` are the *carried* types of the operands (`None` = untyped =
/// `Dynamic`). The result is a pure function of the two — no inference, no
/// mutation.
///
/// # Examples
///
/// ```
/// use semantic_ir::op_select::{resolve_numeric, NumericLowering};
/// use semantic_ir::{IntSpec, IntWidth, Overflow, SirType};
///
/// let i32 = SirType::int(IntWidth::W32, true, Overflow::Wrap);
/// // Two matching i32s specialise to a native i32 op.
/// assert_eq!(
///     resolve_numeric(Some(&i32), Some(&i32)),
///     NumericLowering::Int(IntSpec::sized(IntWidth::W32, true, Overflow::Wrap)),
/// );
/// // An untyped operand falls back to runtime dispatch — today's behaviour.
/// assert_eq!(resolve_numeric(Some(&i32), None), NumericLowering::RuntimeDispatch);
/// ```
pub fn resolve_numeric(lhs: Option<&SirType>, rhs: Option<&SirType>) -> NumericLowering {
    match (as_numeric(lhs), as_numeric(rhs)) {
        // Same concrete integer type on both sides → specialise to it.
        (Some(Numeric::Int(a)), Some(Numeric::Int(b))) if a == b => NumericLowering::Int(a),
        // A float meets another number → promote to float.
        (Some(Numeric::Float), Some(_)) | (Some(_), Some(Numeric::Float)) => NumericLowering::Float,
        // Dynamic/absent, non-numeric, or mismatched integer widths → dispatch.
        _ => NumericLowering::RuntimeDispatch,
    }
}

/// How *any* binary operator lowers, resolved from its operand types. This is
/// the complete, op-aware companion to [`resolve_numeric`]: it folds in the two
/// cases a bare numeric resolver can't see — `+` on strings is concatenation,
/// and the comparison operators are their own category (they yield `Bool`, not
/// a number).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryLowering {
    /// Typed integer arithmetic on the given spec (both operands that int type).
    IntArith(IntSpec),
    /// Float arithmetic (a float met another number — promote).
    FloatArith,
    /// String concatenation — `+` where both operands are `Str`.
    StrConcat,
    /// A comparison whose operands share a single concrete comparable type
    /// (both the same int spec, both `Float`, both `Str`, or both `Bool`) — the
    /// backend can emit a native typed comparison.
    TypedCompare,
    /// Any operand `Dynamic`/absent, mismatched, or otherwise not statically
    /// resolvable — fall back to the runtime helper (`_sir_plus`/`_sir_lt`/…),
    /// exactly today's behaviour.
    RuntimeDispatch,
}

/// The set of binary operators a frontend emits as `BuiltinCall(op, [a, b])`.
/// Anything outside this set resolves to [`BinaryLowering::RuntimeDispatch`].
///
/// | category   | operators                       |
/// |------------|---------------------------------|
/// | arithmetic | `+` `-` `*`                     |
/// | comparison | `<` `>` `<=` `>=` `==` `!=`      |
///
/// (`/` is deliberately absent — division is split into `div_floor`/`div_trunc`
/// with its own rounding semantics; see SIR21 §E3 and `sir-conformance`.)
pub fn resolve_binary(op: &str, lhs: Option<&SirType>, rhs: Option<&SirType>) -> BinaryLowering {
    match op {
        // `+` is polymorphic: string concat when both are strings, else numeric.
        "+" => match (lhs, rhs) {
            (Some(SirType::Str), Some(SirType::Str)) => BinaryLowering::StrConcat,
            _ => from_numeric(resolve_numeric(lhs, rhs)),
        },
        "-" | "*" => from_numeric(resolve_numeric(lhs, rhs)),
        "<" | ">" | "<=" | ">=" | "==" | "!=" => resolve_compare(lhs, rhs),
        // Unknown / not-yet-modelled operator: let the runtime decide.
        _ => BinaryLowering::RuntimeDispatch,
    }
}

/// Lift a [`NumericLowering`] into the richer [`BinaryLowering`].
fn from_numeric(n: NumericLowering) -> BinaryLowering {
    match n {
        NumericLowering::Int(spec) => BinaryLowering::IntArith(spec),
        NumericLowering::Float => BinaryLowering::FloatArith,
        NumericLowering::RuntimeDispatch => BinaryLowering::RuntimeDispatch,
    }
}

/// Resolve a comparison. A native typed comparison is emitted only when both
/// operands are the *same* concrete comparable type; anything mixed or `Dynamic`
/// dispatches (no inference, no silent numeric promotion in comparisons).
fn resolve_compare(lhs: Option<&SirType>, rhs: Option<&SirType>) -> BinaryLowering {
    match (lhs, rhs) {
        (Some(SirType::Int(a)), Some(SirType::Int(b))) if a == b => BinaryLowering::TypedCompare,
        (Some(SirType::Float), Some(SirType::Float)) => BinaryLowering::TypedCompare,
        (Some(SirType::Str), Some(SirType::Str)) => BinaryLowering::TypedCompare,
        (Some(SirType::Bool), Some(SirType::Bool)) => BinaryLowering::TypedCompare,
        _ => BinaryLowering::RuntimeDispatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{IntWidth, Overflow};

    fn int(w: IntWidth, signed: bool, o: Overflow) -> SirType {
        SirType::int(w, signed, o)
    }

    // ── The spec table, row by row ────────────────────────────────────

    #[test]
    fn matching_i32_specialises() {
        let i32 = int(IntWidth::W32, true, Overflow::Wrap);
        assert_eq!(
            resolve_numeric(Some(&i32), Some(&i32)),
            NumericLowering::Int(IntSpec::sized(IntWidth::W32, true, Overflow::Wrap))
        );
    }

    #[test]
    fn matching_arbitrary_is_the_bignum_path() {
        let big = SirType::int_default(); // arbitrary precision
        assert_eq!(
            resolve_numeric(Some(&big), Some(&big)),
            NumericLowering::Int(IntSpec::arbitrary())
        );
    }

    #[test]
    fn float_promotes_against_any_number() {
        let f = SirType::Float;
        let i32 = int(IntWidth::W32, true, Overflow::Wrap);
        assert_eq!(resolve_numeric(Some(&f), Some(&f)), NumericLowering::Float);
        assert_eq!(resolve_numeric(Some(&f), Some(&i32)), NumericLowering::Float);
        assert_eq!(resolve_numeric(Some(&i32), Some(&f)), NumericLowering::Float);
    }

    #[test]
    fn dynamic_operand_dispatches() {
        let i32 = int(IntWidth::W32, true, Overflow::Wrap);
        // None == Dynamic (untyped) — today's every-operand case.
        assert_eq!(resolve_numeric(None, None), NumericLowering::RuntimeDispatch);
        assert_eq!(resolve_numeric(Some(&i32), None), NumericLowering::RuntimeDispatch);
        assert_eq!(resolve_numeric(None, Some(&i32)), NumericLowering::RuntimeDispatch);
        // Explicit Dynamic behaves the same as absent.
        assert_eq!(
            resolve_numeric(Some(&SirType::Dynamic), Some(&i32)),
            NumericLowering::RuntimeDispatch
        );
        // Float against Dynamic is NOT a promotion — Dynamic isn't "numeric".
        assert_eq!(
            resolve_numeric(Some(&SirType::Float), Some(&SirType::Dynamic)),
            NumericLowering::RuntimeDispatch
        );
    }

    #[test]
    fn mismatched_int_widths_dispatch_no_inference() {
        // SIR does not silently promote i8 + i32 — that would be inference.
        let i8 = int(IntWidth::W8, true, Overflow::Wrap);
        let i32 = int(IntWidth::W32, true, Overflow::Wrap);
        assert_eq!(resolve_numeric(Some(&i8), Some(&i32)), NumericLowering::RuntimeDispatch);
    }

    #[test]
    fn different_signedness_or_overflow_dispatches() {
        let u32 = int(IntWidth::W32, false, Overflow::Wrap);
        let i32 = int(IntWidth::W32, true, Overflow::Wrap);
        assert_eq!(resolve_numeric(Some(&u32), Some(&i32)), NumericLowering::RuntimeDispatch);

        let i32_wrap = int(IntWidth::W32, true, Overflow::Wrap);
        let i32_trap = int(IntWidth::W32, true, Overflow::Trap);
        // Same width+signedness but different overflow mode is NOT the same
        // type — the overflow behaviour is part of the semantics.
        assert_eq!(
            resolve_numeric(Some(&i32_wrap), Some(&i32_trap)),
            NumericLowering::RuntimeDispatch
        );
    }

    #[test]
    fn non_numeric_types_dispatch() {
        let s = SirType::Str;
        let i32 = int(IntWidth::W32, true, Overflow::Wrap);
        // (Str, Str) is string concat's job, handled elsewhere → dispatch here.
        assert_eq!(resolve_numeric(Some(&s), Some(&s)), NumericLowering::RuntimeDispatch);
        assert_eq!(resolve_numeric(Some(&s), Some(&i32)), NumericLowering::RuntimeDispatch);
        assert_eq!(
            resolve_numeric(Some(&SirType::Bool), Some(&SirType::Bool)),
            NumericLowering::RuntimeDispatch
        );
    }

    #[test]
    fn matching_i64_and_sized_widths() {
        for w in [IntWidth::W8, IntWidth::W16, IntWidth::W64, IntWidth::W128] {
            let t = int(w, true, Overflow::Wrap);
            assert_eq!(
                resolve_numeric(Some(&t), Some(&t)),
                NumericLowering::Int(IntSpec::sized(w, true, Overflow::Wrap)),
                "matching {w:?} should specialise"
            );
        }
    }

    // ── resolve_binary: arithmetic + concat + comparison (T3c-2) ──────

    #[test]
    fn binary_arithmetic_matches_the_numeric_rule() {
        let i32 = int(IntWidth::W32, true, Overflow::Wrap);
        let spec = IntSpec::sized(IntWidth::W32, true, Overflow::Wrap);
        for op in ["+", "-", "*"] {
            assert_eq!(resolve_binary(op, Some(&i32), Some(&i32)), BinaryLowering::IntArith(spec));
            assert_eq!(
                resolve_binary(op, Some(&SirType::Float), Some(&i32)),
                BinaryLowering::FloatArith
            );
            assert_eq!(
                resolve_binary(op, Some(&i32), None),
                BinaryLowering::RuntimeDispatch
            );
        }
    }

    #[test]
    fn plus_on_strings_is_concat_but_minus_times_are_not() {
        let s = SirType::Str;
        assert_eq!(resolve_binary("+", Some(&s), Some(&s)), BinaryLowering::StrConcat);
        // `-`/`*` on strings are not numeric and not concat → dispatch.
        assert_eq!(resolve_binary("-", Some(&s), Some(&s)), BinaryLowering::RuntimeDispatch);
        assert_eq!(resolve_binary("*", Some(&s), Some(&s)), BinaryLowering::RuntimeDispatch);
        // `+` with only one string (or string+int) is not concat → dispatch.
        let i32 = int(IntWidth::W32, true, Overflow::Wrap);
        assert_eq!(resolve_binary("+", Some(&s), Some(&i32)), BinaryLowering::RuntimeDispatch);
        assert_eq!(resolve_binary("+", Some(&s), None), BinaryLowering::RuntimeDispatch);
    }

    #[test]
    fn comparisons_specialise_on_matching_comparable_types() {
        let i32 = int(IntWidth::W32, true, Overflow::Wrap);
        for op in ["<", ">", "<=", ">=", "==", "!="] {
            assert_eq!(resolve_binary(op, Some(&i32), Some(&i32)), BinaryLowering::TypedCompare);
            assert_eq!(
                resolve_binary(op, Some(&SirType::Float), Some(&SirType::Float)),
                BinaryLowering::TypedCompare
            );
            assert_eq!(
                resolve_binary(op, Some(&SirType::Str), Some(&SirType::Str)),
                BinaryLowering::TypedCompare
            );
            assert_eq!(
                resolve_binary(op, Some(&SirType::Bool), Some(&SirType::Bool)),
                BinaryLowering::TypedCompare
            );
        }
    }

    #[test]
    fn comparisons_dispatch_on_mismatch_or_dynamic() {
        let i8 = int(IntWidth::W8, true, Overflow::Wrap);
        let i32 = int(IntWidth::W32, true, Overflow::Wrap);
        // Mismatched int widths, mixed int/float, and Dynamic all dispatch —
        // no silent promotion in a comparison.
        assert_eq!(resolve_binary("<", Some(&i8), Some(&i32)), BinaryLowering::RuntimeDispatch);
        assert_eq!(
            resolve_binary("<", Some(&i32), Some(&SirType::Float)),
            BinaryLowering::RuntimeDispatch
        );
        assert_eq!(resolve_binary("==", Some(&i32), None), BinaryLowering::RuntimeDispatch);
        assert_eq!(resolve_binary("==", None, None), BinaryLowering::RuntimeDispatch);
    }

    #[test]
    fn unknown_and_division_operators_dispatch() {
        let i32 = int(IntWidth::W32, true, Overflow::Wrap);
        // `/` is intentionally not modelled here (division is split — §E3).
        assert_eq!(resolve_binary("/", Some(&i32), Some(&i32)), BinaryLowering::RuntimeDispatch);
        // A genuinely unknown operator dispatches too.
        assert_eq!(resolve_binary("<=>", Some(&i32), Some(&i32)), BinaryLowering::RuntimeDispatch);
    }
}
