//! # The bit domain and the shape domain of the adders and the ALU.
//!
//! A hardware wire carries 0 or 1.  This crate models a wire as a `u8`, and a
//! `u8` can hold 254 other values; an operand is a slice of such wires, and a
//! slice can have any length.  The contract (AR01, "Domain") is that every
//! operation REJECTS a value that is not a bit and an operand of a shape it
//! cannot add, and that rejection is a panic in EVERY build profile: debug,
//! release, with or without overflow checks.  Nothing is masked (`a & 1`) and
//! nothing is passed through.
//!
//! The adders and the ALU do not validate bits themselves: every bit passes
//! through a `logic-gates` gate (`xor_gate`, `and_gate`, `or_gate`, `not_gate`),
//! and the gates reject a non-bit (finding SEC-007 of `logic-gates`, whose guard
//! used to be a `debug_assert!` and compiled away in a release build).  So this
//! crate's rejection is INHERITED, and a test that runs only in the debug
//! profile would pass even if that guard regressed.  This file is therefore also
//! run with `cargo test --release` (see BUILD), and it states, operation by
//! operation and position by position, what must happen.
//!
//! The shape rules the crate itself enforces (`assert!`, not `debug_assert!`):
//! the two operands have the same length, and that length is at least 1.  There
//! is no width-driven allocation: the output is exactly as long as the input,
//! so the caller's length is not an amplification factor (AR01, "Width").
//!
//! The one operand that is NOT checked is `b` of `AluOp::Not`: it is ignored
//! entirely (any content, any length), which is the documented behaviour and
//! is pinned here so it cannot drift.

use arithmetic::{
    alu, full_adder, half_adder, ripple_carry_adder, ripple_carry_adder_with_carry, AluOp,
};
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Once;

thread_local! {
    /// `true` while `panics` is running a call that is EXPECTED to panic.
    static EXPECTING: Cell<bool> = const { Cell::new(false) };
}

/// Install, once, a panic hook that stays silent while a panic is expected and
/// otherwise defers to the previous hook, so a genuine assertion failure in
/// this file still prints its message.
fn quiet() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !EXPECTING.with(Cell::get) {
                previous(info);
            }
        }));
    });
}

/// `true` when `f` panics.  A call that returns normally (with any value at
/// all) is the bug this file exists to catch.
fn panics<R>(f: impl FnOnce() -> R) -> bool {
    quiet();
    EXPECTING.with(|e| e.set(true));
    let panicked = catch_unwind(AssertUnwindSafe(f)).is_err();
    EXPECTING.with(|e| e.set(false));
    panicked
}

/// Values that are not bits: the smallest, a mid value, and the largest.
const NON_BITS: [u8; 4] = [2, 3, 128, 255];

/// The six operations, for the loops that attack all of them.
const ALL_OPS: [AluOp; 6] = [
    AluOp::Add,
    AluOp::Sub,
    AluOp::And,
    AluOp::Or,
    AluOp::Xor,
    AluOp::Not,
];

/// The five operations that read `b`.
const BINARY_OPS: [AluOp; 5] = [AluOp::Add, AluOp::Sub, AluOp::And, AluOp::Or, AluOp::Xor];

/// An 8-bit operand of `fill` with `junk` written at `position`.
fn with_junk(fill: u8, junk: u8, position: usize) -> Vec<u8> {
    let mut v = vec![fill; 8];
    v[position] = junk;
    v
}

// ---------------------------------------------------------------------------
// Single-bit adders
// ---------------------------------------------------------------------------

#[test]
fn half_adder_rejects_a_non_bit_in_either_input() {
    for &junk in &NON_BITS {
        for bit in 0..=1u8 {
            assert!(panics(|| half_adder(junk, bit)), "half_adder({junk}, {bit})");
            assert!(panics(|| half_adder(bit, junk)), "half_adder({bit}, {junk})");
        }
    }
}

#[test]
fn full_adder_rejects_a_non_bit_in_any_of_its_three_inputs() {
    for &junk in &NON_BITS {
        for x in 0..=1u8 {
            for y in 0..=1u8 {
                assert!(panics(|| full_adder(junk, x, y)), "a = {junk}");
                assert!(panics(|| full_adder(x, junk, y)), "b = {junk}");
                assert!(panics(|| full_adder(x, y, junk)), "carry_in = {junk}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Ripple-carry adders
// ---------------------------------------------------------------------------

#[test]
fn ripple_carry_adder_rejects_a_non_bit_at_every_position_of_either_operand() {
    let zeros = vec![0u8; 8];
    for &junk in &NON_BITS {
        for position in 0..8 {
            let bad = with_junk(0, junk, position);
            assert!(panics(|| ripple_carry_adder(&bad, &zeros)), "a[{position}] = {junk}");
            assert!(panics(|| ripple_carry_adder(&zeros, &bad)), "b[{position}] = {junk}");
            assert!(
                panics(|| ripple_carry_adder_with_carry(&bad, &zeros, 0)),
                "with_carry a[{position}] = {junk}"
            );
            assert!(
                panics(|| ripple_carry_adder_with_carry(&zeros, &bad, 1)),
                "with_carry b[{position}] = {junk}"
            );
        }
    }
}

#[test]
fn ripple_carry_adder_with_carry_rejects_a_non_bit_carry_in() {
    for &junk in &NON_BITS {
        assert!(panics(|| ripple_carry_adder_with_carry(&[0, 1], &[1, 0], junk)));
        assert!(panics(|| ripple_carry_adder_with_carry(&[0], &[0], junk)));
    }
}

#[test]
fn ripple_carry_adder_rejects_unequal_lengths_and_empty_operands() {
    assert!(panics(|| ripple_carry_adder(&[1, 0], &[1])));
    assert!(panics(|| ripple_carry_adder(&[1], &[1, 0])));
    assert!(panics(|| ripple_carry_adder(&[], &[])));
    assert!(panics(|| ripple_carry_adder(&[], &[1])));
    assert!(panics(|| ripple_carry_adder_with_carry(&[1, 0], &[1], 0)));
    assert!(panics(|| ripple_carry_adder_with_carry(&[], &[], 0)));
}

// ---------------------------------------------------------------------------
// ALU
// ---------------------------------------------------------------------------

#[test]
fn alu_rejects_a_non_bit_at_every_position_of_a_for_every_operation() {
    let zeros = vec![0u8; 8];
    for &junk in &NON_BITS {
        for position in 0..8 {
            let bad = with_junk(0, junk, position);
            for op in ALL_OPS {
                assert!(panics(|| alu(&bad, &zeros, op)), "a[{position}] = {junk}, {op:?}");
            }
        }
    }
}

#[test]
fn alu_rejects_a_non_bit_at_every_position_of_b_for_the_five_binary_operations() {
    let zeros = vec![0u8; 8];
    for &junk in &NON_BITS {
        for position in 0..8 {
            let bad = with_junk(0, junk, position);
            for op in BINARY_OPS {
                assert!(panics(|| alu(&zeros, &bad, op)), "b[{position}] = {junk}, {op:?}");
            }
        }
    }
}

#[test]
fn alu_rejects_unequal_lengths_for_the_five_binary_operations() {
    for op in BINARY_OPS {
        assert!(panics(|| alu(&[1, 0, 1], &[1, 0], op)), "{op:?}: b shorter");
        assert!(panics(|| alu(&[1, 0], &[1, 0, 1], op)), "{op:?}: b longer");
        assert!(panics(|| alu(&[1], &[], op)), "{op:?}: b empty");
    }
}

#[test]
fn alu_rejects_an_empty_a_for_every_operation() {
    for op in ALL_OPS {
        assert!(panics(|| alu(&[], &[], op)), "{op:?}: both empty");
        assert!(panics(|| alu(&[], &[1, 0], op)), "{op:?}: a empty");
    }
}

#[test]
fn alu_not_ignores_b_whatever_it_holds() {
    // `Not` is unary: `b` is never read, so a hostile `b` (any length, any
    // content, even empty) must change nothing.  This is the documented
    // behaviour (AR01), pinned so that it cannot start rejecting or reading `b`
    // without the contract changing with it.
    let a = [1u8, 0, 1, 1];
    let expected = alu(&a, &[0, 0, 0, 0], AluOp::Not);
    for b in [vec![], vec![255], vec![2, 3, 128, 255], vec![1; 64]] {
        assert_eq!(alu(&a, &b, AluOp::Not), expected, "b = {b:?}");
    }
}

#[test]
fn a_long_operand_costs_time_linear_in_its_length_and_allocates_its_own_size() {
    // No width-driven amplification: a 100,000-bit addition returns a
    // 100,000-bit sum, and nothing is sized from the caller's width except by
    // the caller's own slice.
    let a = vec![1u8; 100_000];
    let b = vec![0u8; 100_000];
    let r = ripple_carry_adder_with_carry(&a, &b, 1);
    assert_eq!(r.sum.len(), 100_000);
    assert_eq!(r.carry_out, 1, "all ones plus the carry-in carries out");
    assert!(r.sum.iter().all(|&bit| bit == 0));
}
