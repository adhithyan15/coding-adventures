//! # How wide may a decoder be?  (finding SEC-008)
//!
//! A decoder with `n` inputs has `2^n` outputs, so the caller's `n` is an
//! amplification factor: three bits in, eight lines out; sixty bits in, more
//! lines than there is memory.  `decoder` (and `demux`, which is built on it)
//! used to size its output as `1usize << n` with no bound on `n`:
//!
//! ```text
//!   n = 17..=40   a 128 KiB .. 1 TiB allocation and 2^n loop turns: a hang or an abort
//!   n = 64        debug: `1 << 64` overflows and panics
//!                 release: the shift WRAPS to `1 << 0 = 1`; the call returns ONE output
//!                 line for sixty-four inputs, silently
//! ```
//!
//! The contract (LG01) is a width cap, [`MAX_DECODER_WIDTH`] = 16 inputs, so at
//! most 65,536 outputs, rejected by a panic in every profile.  Sixteen is far
//! above anything a real design here uses (the Intel 4004 decodes four bits)
//! and keeps the cap small enough to check exhaustively at the boundary.

use logic_gates::combinational::{decoder, demux, MAX_DECODER_WIDTH};
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Once;

thread_local! {
    static EXPECTING: Cell<bool> = const { Cell::new(false) };
}

fn panics<R>(f: impl FnOnce() -> R) -> bool {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !EXPECTING.with(Cell::get) {
                previous(info);
            }
        }));
    });
    EXPECTING.with(|e| e.set(true));
    let panicked = catch_unwind(AssertUnwindSafe(f)).is_err();
    EXPECTING.with(|e| e.set(false));
    panicked
}

#[test]
fn the_cap_is_sixteen_inputs() {
    assert_eq!(MAX_DECODER_WIDTH, 16);
}

#[test]
fn a_decoder_at_the_cap_still_works() {
    let out = decoder(&[1; MAX_DECODER_WIDTH]);
    assert_eq!(out.len(), 1 << MAX_DECODER_WIDTH);
    assert_eq!(out.iter().map(|&b| usize::from(b)).sum::<usize>(), 1);
    assert_eq!(out[out.len() - 1], 1, "all ones selects the last line");
}

#[test]
fn a_decoder_one_wider_than_the_cap_is_rejected() {
    assert!(panics(|| decoder(&[0; MAX_DECODER_WIDTH + 1])));
}

#[test]
fn a_decoder_with_sixty_four_inputs_is_rejected_not_wrapped() {
    // Release builds used to return a one-element vector here.
    assert!(panics(|| decoder(&[0; 64])));
}

#[test]
fn a_demux_with_more_than_the_cap_of_select_lines_is_rejected() {
    // 2^17 outputs: n_outputs and sel agree, so only the cap can stop it.
    assert!(panics(|| demux(1, &[0; MAX_DECODER_WIDTH + 1], 1 << (MAX_DECODER_WIDTH + 1))));
}
