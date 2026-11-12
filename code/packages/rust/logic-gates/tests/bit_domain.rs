//! # The bit domain: what every public operation does with a value that is not 0 or 1.
//!
//! A hardware wire carries 0 or 1.  This crate models a wire as a `u8`, and a
//! `u8` can hold 254 other values.  The contract (LG01, "Bit representation")
//! is that every operation REJECTS such a value, and that rejection is a panic
//! in EVERY build profile: debug, release, with or without overflow checks.
//! Nothing is masked (`a & 1`) and nothing is passed through.
//!
//! Why a test of its own?  Before finding SEC-007 the crate's own guard,
//! `validate_bit`, was a `debug_assert!`.  A debug build panicked; a release
//! build skipped the check, and whether the call then panicked depended on
//! which gate happened to sit underneath:
//!
//! ```text
//!   and/or/xor/not   the transistor model rejects the value  -> panic (by luck)
//!   tri_state        no gate underneath                      -> Some(2), garbage out
//!   priority_encoder compares with `== 1`                    -> a 2 is read as 0
//!   encoder          sums the inputs in a u8                 -> 257 ones wrap to 1
//! ```
//!
//! The Warrant harness (`tests/warrant_evidence.rs`) checks the same rejection
//! rows, but it runs in the debug profile; this file is also run with
//! `cargo test --release` (see BUILD) so the release profile is covered by
//! name, not by hope.

use logic_gates::combinational::{
    decoder, demux, encoder, mux2, mux4, mux_n, priority_encoder, tri_state,
};
use logic_gates::gates::{
    and_gate, and_n, nand_and, nand_gate, nand_not, nand_or, nand_xor, nor_gate, not_gate, or_gate,
    or_n, xnor_gate, xor_gate, xor_n,
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
const GARBAGE: [u8; 3] = [2, 17, 255];

#[test]
fn every_scalar_gate_rejects_a_non_bit_in_either_position() {
    for g in GARBAGE {
        assert!(panics(|| not_gate(g)), "not_gate({g})");
        assert!(panics(|| nand_not(g)), "nand_not({g})");
        let two_input: [(&str, fn(u8, u8) -> u8); 9] = [
            ("and_gate", and_gate),
            ("or_gate", or_gate),
            ("xor_gate", xor_gate),
            ("nand_gate", nand_gate),
            ("nor_gate", nor_gate),
            ("xnor_gate", xnor_gate),
            ("nand_and", nand_and),
            ("nand_or", nand_or),
            ("nand_xor", nand_xor),
        ];
        for (name, f) in two_input {
            for good in [0u8, 1] {
                assert!(panics(|| f(g, good)), "{name}({g}, {good})");
                assert!(panics(|| f(good, g)), "{name}({good}, {g})");
            }
            assert!(panics(|| f(g, g)), "{name}({g}, {g})");
        }
    }
}

#[test]
fn the_n_input_gates_reject_a_non_bit_and_a_short_input() {
    for g in GARBAGE {
        for f in [and_n, or_n, xor_n] {
            assert!(panics(|| f(&[g, 1])));
            assert!(panics(|| f(&[1, g])));
            assert!(panics(|| f(&[1, 0, g, 1])));
        }
    }
    for f in [and_n, or_n, xor_n] {
        assert!(panics(|| f(&[])));
        assert!(panics(|| f(&[1])));
    }
}

#[test]
fn mux2_and_tri_state_reject_a_non_bit_in_every_position() {
    for g in GARBAGE {
        assert!(panics(|| mux2(g, 0, 0)), "mux2 d0");
        assert!(panics(|| mux2(0, g, 1)), "mux2 d1");
        assert!(panics(|| mux2(0, 1, g)), "mux2 sel");
        // tri_state has no gate underneath it: this is the finding.
        assert!(panics(|| tri_state(g, 1)), "tri_state data, enabled");
        assert!(panics(|| tri_state(g, 0)), "tri_state data, disabled");
        assert!(panics(|| tri_state(1, g)), "tri_state enable");
    }
}

#[test]
fn mux4_mux_n_and_demux_reject_a_non_bit_and_a_bad_shape() {
    for g in GARBAGE {
        assert!(panics(|| mux4(g, 0, 0, 0, &[0, 0])));
        assert!(panics(|| mux4(0, 0, 0, g, &[1, 1])));
        assert!(panics(|| mux4(0, 0, 0, 0, &[g, 0])));
        assert!(panics(|| mux4(0, 0, 0, 0, &[0, g])));
        assert!(panics(|| mux_n(&[0, g, 0, 0], &[0, 0])));
        assert!(panics(|| mux_n(&[0, 0, 0, 0], &[0, g])));
        assert!(panics(|| demux(g, &[0, 0], 4)));
        assert!(panics(|| demux(1, &[g, 0], 4)));
    }
    assert!(panics(|| mux4(0, 0, 0, 0, &[0])), "sel too short");
    assert!(panics(|| mux4(0, 0, 0, 0, &[0, 0, 0])), "sel too long");
    assert!(panics(|| mux_n(&[0], &[])), "one input");
    assert!(panics(|| mux_n(&[0, 0, 0], &[0, 0])), "three inputs");
    assert!(panics(|| mux_n(&[0, 0, 0, 0], &[0])), "sel too short");
    assert!(panics(|| demux(1, &[0], 4)), "sel does not match n_outputs");
    assert!(panics(|| demux(1, &[0, 0, 0], 6)), "n_outputs not a power of 2");
}

#[test]
fn decoder_and_priority_encoder_reject_a_non_bit() {
    for g in GARBAGE {
        assert!(panics(|| decoder(&[g])));
        assert!(panics(|| decoder(&[0, g, 1])));
        // priority_encoder compares each input with `== 1`; before the fix a 2
        // was silently read as "inactive" in a release build.
        assert!(panics(|| priority_encoder(&[g, 0])), "priority_encoder first");
        assert!(panics(|| priority_encoder(&[0, 0, 1, g])), "priority_encoder last");
        assert!(panics(|| priority_encoder(&[0, g, 0, 0])), "priority_encoder middle");
    }
    assert!(panics(|| decoder(&[])), "decoder needs an input");
    assert!(panics(|| priority_encoder(&[1])), "one input");
    assert!(panics(|| priority_encoder(&[0, 0, 0])), "three inputs");
}

#[test]
fn encoder_rejects_a_non_bit_a_bad_shape_and_a_non_one_hot_input() {
    for g in GARBAGE {
        assert!(panics(|| encoder(&[g, 0])));
        assert!(panics(|| encoder(&[0, 1, 0, g])));
    }
    assert!(panics(|| encoder(&[1])), "one input");
    assert!(panics(|| encoder(&[1, 0, 0])), "three inputs");
    assert!(panics(|| encoder(&[0, 0, 0, 0])), "no bit active");
    assert!(panics(|| encoder(&[1, 1, 0, 0])), "two bits active");
}

/// The wrap-around case of the finding.  The encoder counted active inputs in
/// a `u8`, so 256 + 1 = 257 active inputs counted as 1 and passed the
/// "exactly one active" check in a release build (in a debug build the sum
/// itself panics, which is why this needs the release profile to show).
#[test]
fn encoder_counts_active_inputs_without_wrapping() {
    let mut wide = vec![0u8; 512];
    for slot in wide.iter_mut().take(257) {
        *slot = 1;
    }
    assert!(panics(|| encoder(&wide)), "257 active inputs are not one-hot");
    let mut wide = vec![0u8; 512];
    for slot in wide.iter_mut().take(256) {
        *slot = 1;
    }
    assert!(panics(|| encoder(&wide)), "256 active inputs are not one-hot");
}
