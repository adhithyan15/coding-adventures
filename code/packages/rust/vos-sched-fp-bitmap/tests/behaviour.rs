//! Hand-written behaviour tests (the generated evidence suite is separate and needs the
//! `ghost` feature).  Everything here goes through the public trait, as the spine does.
//!
//! A state is only ever made by `init` and the operations (its field is private), so the
//! tests build a state with a set of `insert` calls and compare against a plain
//! `[bool; 8]` reference model written the obvious way.

use vos_sched_contract::{Pick, Sched, Upd};
use vos_sched_fp_bitmap::{Bitmap, FpBitmap};

type S = FpBitmap;

/// The state whose ready set is exactly the bits of `mask`.
fn state_of(mask: u8) -> Bitmap {
    let mut s = S::init();
    for t in 0..8u8 {
        if mask >> t & 1 == 1 {
            let (u, next) = S::insert(s, t);
            assert_eq!(u, Upd::Changed);
            s = next;
        }
    }
    s
}

#[test]
fn n_is_eight() {
    assert_eq!(S::N, 8);
}

#[test]
fn init_is_the_empty_set() {
    assert_eq!(S::init(), state_of(0));
    assert_eq!(S::dispatch(S::init()), (Pick::Idle, S::init()));
}

#[test]
fn insert_matches_the_reference_for_every_state_and_id() {
    for mask in 0..=255u8 {
        for t in 0..=255u8 {
            let (u, s2) = S::insert(state_of(mask), t);
            if t >= 8 {
                assert_eq!((u, s2), (Upd::OutOfRange, state_of(mask)));
            } else if mask >> t & 1 == 1 {
                assert_eq!((u, s2), (Upd::Unchanged, state_of(mask)));
            } else {
                assert_eq!((u, s2), (Upd::Changed, state_of(mask | 1 << t)));
            }
        }
    }
}

#[test]
fn remove_matches_the_reference_for_every_state_and_id() {
    for mask in 0..=255u8 {
        for t in 0..=255u8 {
            let (u, s2) = S::remove(state_of(mask), t);
            if t >= 8 {
                assert_eq!((u, s2), (Upd::OutOfRange, state_of(mask)));
            } else if mask >> t & 1 == 0 {
                assert_eq!((u, s2), (Upd::Unchanged, state_of(mask)));
            } else {
                assert_eq!((u, s2), (Upd::Changed, state_of(mask & !(1 << t))));
            }
        }
    }
}

#[test]
fn dispatch_picks_the_highest_ready_thread_and_removes_only_it() {
    for mask in 0..=255u8 {
        let (p, s2) = S::dispatch(state_of(mask));
        if mask == 0 {
            assert_eq!((p, s2), (Pick::Idle, state_of(0)));
        } else {
            // The reference: scan from 7 down, the obvious way.
            let top = (0..8u8).rev().find(|t| mask >> t & 1 == 1).unwrap();
            assert_eq!((p, s2), (Pick::Run(top), state_of(mask & !(1 << top))));
        }
    }
}

#[test]
fn draining_yields_the_threads_in_descending_priority() {
    let mut s = state_of(0b1011_0101);
    let mut order = Vec::new();
    loop {
        let (p, next) = S::dispatch(s);
        s = next;
        match p {
            Pick::Idle => break,
            Pick::Run(t) => order.push(t),
        }
    }
    assert_eq!(order, vec![7, 5, 4, 2, 0]);
    assert_eq!(s, S::init());
}

#[test]
fn insert_then_remove_restores_the_state() {
    for mask in 0..=255u8 {
        for t in 0..8u8 {
            if mask >> t & 1 == 0 {
                let (_, s1) = S::insert(state_of(mask), t);
                assert_eq!(S::remove(s1, t), (Upd::Changed, state_of(mask)));
            }
        }
    }
}

#[cfg(feature = "ghost")]
mod ghost_face {
    use super::*;
    use vos_sched_fp_bitmap::ghost::{AbsW, CfgN, Image};
    use vos_sched_fp_bitmap::*;

    #[test]
    fn the_wrappers_forward_to_the_production_operations() {
        assert_eq!(vos_ghost_init(), Image { mask: 0 });
        assert_eq!(vos_ghost_cfg_n(), CfgN { v: 8 });
        for mask in 0..=255u8 {
            let img = Image { mask };
            assert_eq!(vos_ghost_alpha_w(img), AbsW { w: mask });
            for t in [0u8, 3, 7, 8, 255] {
                let (u, s2) = S::insert(state_of(mask), t);
                assert_eq!(vos_ghost_insert_out(img, t), u);
                assert_eq!(vos_ghost_alpha_w(vos_ghost_insert_img(img, t)).w, alpha(s2));
                let (u, s2) = S::remove(state_of(mask), t);
                assert_eq!(vos_ghost_remove_out(img, t), u);
                assert_eq!(vos_ghost_alpha_w(vos_ghost_remove_img(img, t)).w, alpha(s2));
            }
            let (p, s2) = S::dispatch(state_of(mask));
            assert_eq!(vos_ghost_dispatch_out(img), p);
            assert_eq!(vos_ghost_alpha_w(vos_ghost_dispatch_img(img)).w, alpha(s2));
        }
    }

    /// The mask of a state, read through the image of the same set.
    fn alpha(s: Bitmap) -> u8 {
        // A state is equal to the one built from its mask: find that mask.
        (0..=255u8).find(|m| state_of(*m) == s).unwrap()
    }
}
