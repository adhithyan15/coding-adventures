//! # The integer reference oracle (SIR21 §P2)
//!
//! A conformance harness is only trustworthy if the value it compares each
//! backend against is computed **independently of every backend** — otherwise
//! two backends that share a bug agree with each other and the bug hides. This
//! module is that independent authority for *integer arithmetic*: a small, pure,
//! audited model of what a typed integer operation is *supposed* to produce,
//! derived straight from the [`IntSpec`] semantics defined in SIR21 T1a.
//!
//! It runs no toolchain and touches no backend. Given the operands, the
//! operation, and the integer's `(width, signed, overflow)` spec, it returns the
//! one observable outcome the SIR21 faithfulness contract prescribes. Every
//! backend is later measured against *this*, never against another backend.
//!
//! ## The rule, in one sentence
//!
//! Compute the *mathematically exact* result, then apply the type's policy:
//!
//! | width       | in range?      | out of range → depends on `Overflow`                    |
//! |-------------|----------------|---------------------------------------------------------|
//! | `Arbitrary` | always (grows) | never overflows                                         |
//! | fixed *n*   | `min..=max`    | `Wrap`→mod 2ⁿ · `Saturate`→clamp · `Trap`→raise · `Checked`→none · `Undefined`→backend's choice |
//!
//! The min/max/modulus are **not** stored — they are pure functions of
//! `(width, signed)` (see [`semantic_ir::IntSpec`]). This oracle just applies
//! them.
//!
//! ## Worked truth (the constants this module is unit-tested against)
//!
//! ```text
//! INT32_MAX + 1  (i32, wrap)      ⟶  INT32_MIN      (2147483647 + 1 = -2147483648)
//! 0u32 - 1       (u32, wrap)      ⟶  4294967295     (borrow wraps to the top)
//! 255 + 1        (u8,  wrap)      ⟶  0
//! 127 + 1        (i8,  wrap)      ⟶  -128
//! 127 + 100      (i8,  saturate)  ⟶  127            (clamped, not wrapped)
//! 127 + 1        (i8,  trap)      ⟶  «raise»        (a real program would raise)
//! 127 + 1        (i8,  checked)   ⟶  «none»         (produces Optional::None)
//! 10^12 * 10^12  (arbitrary)      ⟶  10^24          (grows; never overflows)
//! ```
//!
//! ## Current range limit (honest, not silent)
//!
//! The oracle computes in `i128`, which faithfully covers every operation on
//! widths ≤ 64 (a product of two `i64` values is at most 2¹²⁶, well inside
//! `i128`). Two cases exceed that and are reported as [`Outcome::BeyondOracle`]
//! rather than guessed at: (a) a `W128` fixed-width reduction (its `2¹²⁸`
//! modulus does not fit `i128`), and (b) an `Arbitrary`-precision result larger
//! than `i128::MAX` (true bignum). Both are follow-up work — a real bignum
//! backing the oracle — and until then the oracle *says so* instead of returning
//! a wrong number. This mirrors T1a's decision to special-case the `i128`
//! corners instead of panicking.

use semantic_ir::{IntSpec, IntWidth, Overflow};

/// A binary integer operation whose overflow behaviour the spec pins.
///
/// Kept to the three operations whose overflow semantics actually differ by
/// type (`+`, `-`, `*`); comparison and bitwise ops are a later addition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntOp {
    Add,
    Sub,
    Mul,
}

impl IntOp {
    /// Every operation the oracle knows, in declaration order.
    ///
    /// This is the enumeration the **coverage gate** (SIR21 §P5) iterates: it
    /// asserts that for each op here there is a passing conformance case on
    /// every backend that accepts it, so an op the frontend can emit but a
    /// backend never implemented cannot hide. **Adding a variant to [`IntOp`]
    /// requires adding it here** — the gate then forces a case for it.
    pub const ALL: &'static [IntOp] = &[IntOp::Add, IntOp::Sub, IntOp::Mul];

    /// A short, stable tag for assertion messages and coverage keys.
    pub fn tag(self) -> &'static str {
        match self {
            IntOp::Add => "add",
            IntOp::Sub => "sub",
            IntOp::Mul => "mul",
        }
    }

    /// The exact (unbounded) result of the op, or `None` if it overflows the
    /// oracle's own `i128` working range (see [`Outcome::BeyondOracle`]).
    fn exact(self, lhs: i128, rhs: i128) -> Option<i128> {
        match self {
            IntOp::Add => lhs.checked_add(rhs),
            IntOp::Sub => lhs.checked_sub(rhs),
            IntOp::Mul => lhs.checked_mul(rhs),
        }
    }
}

/// The observable outcome the reference semantics prescribe for one integer op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// A definite integer result the backend must reproduce.
    Value(i128),
    /// `Overflow::Trap`: a faithful backend *raises* (Rust panic, Ruby
    /// `RangeError`, …). The harness asserts the program fails, not a value.
    Trapped,
    /// `Overflow::Checked`: the op yields "no value" (`Optional::None`/`nil`).
    NoValue,
    /// `Overflow::Undefined`: UB — the backend MAY choose and MUST record its
    /// choice, so the oracle deliberately asserts *nothing* about the value.
    Unspecified,
    /// The exact result (or a `W128` modulus) exceeds the oracle's `i128`
    /// working range. A documented, honest limit — not a wrong answer.
    BeyondOracle,
}

/// Reduce a mathematically-exact result `exact` to the observable outcome for
/// an integer of spec `spec`.
///
/// This is the heart of the oracle and is deliberately independent of any
/// operation — feed it the true result and it applies width + overflow policy.
pub fn reduce(exact: i128, spec: IntSpec) -> Outcome {
    // Arbitrary precision never overflows: the value *is* the result (as long
    // as it fits the oracle's i128 range; the caller guarantees `exact` was
    // computed with checked ops, so we only get here in range).
    if spec.width == IntWidth::Arbitrary {
        return Outcome::Value(exact);
    }

    let bits = spec.width.bits().expect("non-Arbitrary width has a bit count");
    if bits > 64 {
        // A 128-bit modulus (2¹²⁸) does not fit i128; defer to the bignum
        // follow-up rather than compute a wrong wrap.
        return Outcome::BeyondOracle;
    }

    // Safe: min/max exist for any fixed width, and both fit i128.
    let min = spec.min().expect("fixed width has a min");
    let max = spec.max().expect("fixed width has a max");
    if exact >= min && exact <= max {
        // In range: the result is itself regardless of overflow mode.
        return Outcome::Value(exact);
    }

    match spec.overflow {
        Overflow::Wrap => Outcome::Value(wrap_to(exact, bits, spec.signed)),
        Overflow::Saturate => Outcome::Value(exact.clamp(min, max)),
        Overflow::Trap => Outcome::Trapped,
        Overflow::Checked => Outcome::NoValue,
        Overflow::Undefined => Outcome::Unspecified,
        // A fixed width tagged `Arbitrary` overflow is a malformed spec (only
        // `IntWidth::Arbitrary` may use it); treat the value as-is rather than
        // inventing a wrap the type never asked for.
        Overflow::Arbitrary => Outcome::Value(exact),
    }
}

/// Modular reduction of `exact` into a `bits`-wide integer, re-centred for
/// signedness. `bits` must be ≤ 64 so the `2^bits` modulus fits `i128`.
///
/// - unsigned: `exact mod 2ⁿ` in `0 ..= 2ⁿ−1`
/// - signed:   the same, then values `≥ 2ⁿ⁻¹` are shifted down by `2ⁿ` so the
///   range is `−2ⁿ⁻¹ ..= 2ⁿ⁻¹−1` (two's-complement wrap).
fn wrap_to(exact: i128, bits: u32, signed: bool) -> i128 {
    debug_assert!(bits <= 64, "wrap_to only valid for widths that fit i128 math");
    let modulus = 1i128 << bits; // bits ≤ 64 ⇒ ≤ 2^64, fits i128
    // rem_euclid gives a non-negative representative in 0..modulus even for a
    // negative `exact` (e.g. -1 mod 2^32 = 4294967295 — the `0u32 - 1` case).
    let m = exact.rem_euclid(modulus);
    if signed && m >= (modulus >> 1) {
        m - modulus
    } else {
        m
    }
}

/// Evaluate one binary integer operation under `spec` and return the observable
/// outcome. Computes the exact result with checked `i128` math (so the oracle's
/// own range limit surfaces as [`Outcome::BeyondOracle`] rather than a silent
/// wrap), then [`reduce`]s it.
pub fn eval(op: IntOp, lhs: i128, rhs: i128, spec: IntSpec) -> Outcome {
    match op.exact(lhs, rhs) {
        Some(exact) => reduce(exact, spec),
        None => Outcome::BeyondOracle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Constructors for the specs used across the tests.
    fn i(width: IntWidth, ovf: Overflow) -> IntSpec {
        IntSpec::sized(width, true, ovf)
    }
    fn u(width: IntWidth, ovf: Overflow) -> IntSpec {
        IntSpec::sized(width, false, ovf)
    }

    // ── The canonical constants from the spec (§P2) ───────────────────

    #[test]
    fn int32_max_plus_one_wraps_to_int32_min() {
        // INT32_MAX + 1 == INT32_MIN
        let out = eval(IntOp::Add, i32::MAX as i128, 1, i(IntWidth::W32, Overflow::Wrap));
        assert_eq!(out, Outcome::Value(i32::MIN as i128));
    }

    #[test]
    fn zero_u32_minus_one_wraps_to_max() {
        // 0u32 - 1 == 4294967295
        let out = eval(IntOp::Sub, 0, 1, u(IntWidth::W32, Overflow::Wrap));
        assert_eq!(out, Outcome::Value(4_294_967_295));
    }

    #[test]
    fn u8_and_i8_wrap_edges() {
        assert_eq!(
            eval(IntOp::Add, 255, 1, u(IntWidth::W8, Overflow::Wrap)),
            Outcome::Value(0)
        );
        assert_eq!(
            eval(IntOp::Add, 127, 1, i(IntWidth::W8, Overflow::Wrap)),
            Outcome::Value(-128)
        );
    }

    // ── Overflow modes ────────────────────────────────────────────────

    #[test]
    fn saturate_clamps_both_ends() {
        assert_eq!(
            eval(IntOp::Add, 127, 100, i(IntWidth::W8, Overflow::Saturate)),
            Outcome::Value(127)
        );
        assert_eq!(
            eval(IntOp::Sub, 0, 5, u(IntWidth::W8, Overflow::Saturate)),
            Outcome::Value(0)
        );
        assert_eq!(
            eval(IntOp::Sub, -120, 20, i(IntWidth::W8, Overflow::Saturate)),
            Outcome::Value(-128)
        );
    }

    #[test]
    fn trap_reports_raise() {
        assert_eq!(
            eval(IntOp::Add, 127, 1, i(IntWidth::W8, Overflow::Trap)),
            Outcome::Trapped
        );
        // In range under trap is a plain value.
        assert_eq!(
            eval(IntOp::Add, 100, 20, i(IntWidth::W8, Overflow::Trap)),
            Outcome::Value(120)
        );
    }

    #[test]
    fn checked_reports_no_value() {
        assert_eq!(
            eval(IntOp::Add, 127, 1, i(IntWidth::W8, Overflow::Checked)),
            Outcome::NoValue
        );
    }

    #[test]
    fn undefined_is_unspecified() {
        assert_eq!(
            eval(IntOp::Add, i32::MAX as i128, 1, i(IntWidth::W32, Overflow::Undefined)),
            Outcome::Unspecified
        );
        // But an in-range op under UB is still a definite value.
        assert_eq!(
            eval(IntOp::Add, 2, 2, i(IntWidth::W32, Overflow::Undefined)),
            Outcome::Value(4)
        );
    }

    // ── Arbitrary precision (the dynamic-language integer) ────────────

    #[test]
    fn arbitrary_grows_within_oracle_range() {
        // 10^12 * 10^12 == 10^24 (fits i128, ~1.2e24 < 1.7e38)
        let ten12 = 1_000_000_000_000i128;
        assert_eq!(
            eval(IntOp::Mul, ten12, ten12, IntSpec::arbitrary()),
            Outcome::Value(1_000_000_000_000_000_000_000_000)
        );
    }

    #[test]
    fn arbitrary_beyond_i128_is_reported_not_wrong() {
        // i128::MAX * 2 exceeds i128 — the oracle says BeyondOracle rather than
        // silently wrapping.
        assert_eq!(
            eval(IntOp::Mul, i128::MAX, 2, IntSpec::arbitrary()),
            Outcome::BeyondOracle
        );
    }

    // ── In-range passthrough ──────────────────────────────────────────

    #[test]
    fn in_range_is_untouched_for_every_mode() {
        for ovf in [
            Overflow::Wrap,
            Overflow::Saturate,
            Overflow::Trap,
            Overflow::Checked,
            Overflow::Undefined,
        ] {
            assert_eq!(
                eval(IntOp::Add, 5, 3, i(IntWidth::W32, ovf)),
                Outcome::Value(8),
                "in-range add should pass through under {ovf:?}"
            );
        }
    }

    // ── The honest W128 limit ─────────────────────────────────────────

    #[test]
    fn w128_reduction_is_beyond_oracle() {
        // A 128-bit modulus doesn't fit i128; the oracle declines rather than
        // computes a wrong wrap.
        assert_eq!(
            reduce(i128::MAX, i(IntWidth::W128, Overflow::Wrap)),
            Outcome::BeyondOracle
        );
    }

    // ── reduce() directly, decoupled from any op ──────────────────────

    #[test]
    fn reduce_applies_policy_to_an_exact_value() {
        // 4294967296 (2^32) reduced as i32 wrap → 0.
        assert_eq!(
            reduce(1i128 << 32, i(IntWidth::W32, Overflow::Wrap)),
            Outcome::Value(0)
        );
        // Same exact value, arbitrary precision → itself.
        assert_eq!(reduce(1i128 << 32, IntSpec::arbitrary()), Outcome::Value(1i128 << 32));
    }

    #[test]
    fn wrap_to_is_two_complement() {
        // -1 in an 8-bit unsigned field is 255; in a signed field it stays -1.
        assert_eq!(wrap_to(-1, 8, false), 255);
        assert_eq!(wrap_to(-1, 8, true), -1);
        // 256 wraps to 0 in 8 bits either way.
        assert_eq!(wrap_to(256, 8, false), 0);
        assert_eq!(wrap_to(256, 8, true), 0);
    }

    #[test]
    fn all_ops_is_exhaustive_and_tagged() {
        // Exhaustive match: adding a variant to `IntOp` makes this fail to
        // compile until the author handles it — a compile-time nudge to keep
        // `ALL` (and the coverage gate) complete.
        fn recognised(op: IntOp) -> bool {
            match op {
                IntOp::Add | IntOp::Sub | IntOp::Mul => true,
            }
        }
        assert!(IntOp::ALL.iter().copied().all(recognised));
        assert_eq!(IntOp::ALL.len(), 3, "IntOp::ALL must list every variant");

        // Tags are unique and non-empty (they key the coverage matrix).
        let mut seen = std::collections::HashSet::new();
        for op in IntOp::ALL {
            assert!(!op.tag().is_empty());
            assert!(seen.insert(op.tag()), "duplicate op tag {}", op.tag());
        }
    }
}
